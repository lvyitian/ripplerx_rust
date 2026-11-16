//! The resonator bank: a set of partials, or a waveguide for the tubes.
//!
//! Port of `src/dsp/Resonator.{h,cpp}`.

use crate::filter::Filter;
use crate::luts::Luts;
use crate::models::{MAX_PARTIALS, ModalModel, Ratios};
use crate::partial::Partial;
use crate::waveguide::Waveguide;

/// The parameters that configure a resonator.
#[derive(Clone, Copy, Debug)]
pub struct ResonatorSettings {
    /// Whether the resonator is sounding.
    pub on: bool,
    /// The model the resonator plays.
    pub model: ModalModel,
    /// How many partials the resonator uses.
    pub partials: usize,
    /// The base decay time in seconds.
    pub decay: f64,
    /// The damping, from -1 to 1.
    pub damp: f64,
    /// The tone, from -1 to 1.
    pub tone: f64,
    /// The strike position.
    pub hit: f64,
    /// The release multiplier.
    pub rel: f64,
    /// The inharmonicity.
    pub inharm: f64,
    /// The cutoff, from -1 to 1.
    pub cut: f64,
    /// The tube radius.
    pub radius: f64,
    /// How much velocity shifts the decay.
    pub vel_decay: f64,
    /// How much velocity shifts the strike position.
    pub vel_hit: f64,
    /// How much velocity shifts the inharmonicity.
    pub vel_inharm: f64,
    /// How much velocity shifts the damping.
    pub vel_damp: f64,
    /// How much velocity shifts the tone.
    pub vel_tone: f64,
}

/// A resonator: either a bank of partials or a tube waveguide.
#[derive(Clone, Debug)]
pub struct Resonator {
    partials: Vec<Partial>,
    waveguide: Waveguide,
    filter: Filter,

    /// Counter of samples of silence.
    pub silence: i64,
    /// Returns to false if samples of silence run for a bit.
    pub active: bool,
    /// The sample rate.
    pub srate: f64,
    /// Whether the resonator is switched on.
    pub on: bool,
    /// The model the resonator plays.
    pub nmodel: ModalModel,
    /// How many partials the resonator uses.
    pub npartials: usize,
    /// The base decay time in seconds.
    pub decay: f64,
    /// The tube radius.
    pub radius: f64,
    /// The cutoff, from -1 to 1.
    pub cut: f64,
}

impl Resonator {
    /// Creates a resonator with sixty-four partials.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut partials = Vec::with_capacity(MAX_PARTIALS);
        for i in 0..MAX_PARTIALS {
            partials.push(Partial::new(i + 1));
        }

        Self {
            partials,
            waveguide: Waveguide::new(),
            filter: Filter::default(),
            silence: 0,
            active: false,
            srate: 0.0,
            on: false,
            nmodel: ModalModel::String,
            npartials: 0,
            decay: 0.0,
            radius: 0.0,
            cut: 0.0,
        }
    }

    /// Sets the resonator's parameters.
    pub fn set_params(&mut self, srate: f64, settings: &ResonatorSettings) {
        self.on = settings.on;
        self.nmodel = settings.model;
        self.npartials = settings.partials;
        self.decay = settings.decay;
        self.radius = settings.radius;
        self.srate = srate;
        self.cut = settings.cut;

        // Map 1..0 to 20..20000, with an inverse scale for a negative value.
        let freq = 20.0
            * (20000.0f64 / 20.0).powf(if settings.cut < 0.0 {
                1.0 + settings.cut
            } else {
                settings.cut
            });
        // The C++ sets the filter's coefficients in place, which leaves the
        // filter's history alone, so the coefficients are copied across
        // rather than the whole filter being replaced.
        if settings.cut < 0.0 {
            self.filter.copy_from(&Filter::low_pass(srate, freq, 0.707));
        } else {
            self.filter
                .copy_from(&Filter::high_pass(srate, freq, 0.707));
        }

        for partial in self.partials.iter_mut() {
            partial.damp = settings.damp;
            partial.decay = settings.decay;
            partial.hit = settings.hit;
            partial.inharm = settings.inharm;
            partial.rel = settings.rel;
            partial.tone = settings.tone;
            partial.vel_decay = settings.vel_decay;
            partial.vel_hit = settings.vel_hit;
            partial.vel_inharm = settings.vel_inharm;
            partial.vel_damp = settings.vel_damp;
            partial.vel_tone = settings.vel_tone;
            partial.srate = srate;
        }

        self.waveguide.decay = settings.decay;
        self.waveguide.radius = settings.radius;
        self.waveguide.is_closed = settings.model == ModalModel::ClosedTube;
        self.waveguide.srate = srate;
        self.waveguide.vel_decay = settings.vel_decay;
        self.waveguide.rel = settings.rel;
    }

    /// Retunes the resonator's partials, or its waveguide for the tubes.
    pub fn update(
        &mut self,
        freq: f64,
        vel: f64,
        is_release: bool,
        pitch_bend: f64,
        model: &Ratios,
        model_gain: &Ratios,
    ) {
        if self.nmodel == ModalModel::OpenTube || self.nmodel == ModalModel::ClosedTube {
            self.waveguide
                .update(model[0] * freq, vel, pitch_bend, is_release);
        } else {
            for partial in self.partials.iter_mut() {
                let idx = partial.k - 1; // clears warning when accessing model[k-1] directly
                partial.update(
                    freq,
                    model[idx],
                    model[model.len() - 1],
                    vel,
                    pitch_bend,
                    is_release,
                );
                partial.apply_gain(model_gain[idx]);
            }
        }
    }

    /// Retunes the resonator for a pitch bend.
    pub fn apply_pitch_bend(&mut self, bend: f64, luts: &Luts) {
        if self.active {
            if self.nmodel == ModalModel::OpenTube || self.nmodel == ModalModel::ClosedTube {
                self.waveguide.apply_pitch_bend(bend);
            } else {
                for partial in self.partials.iter_mut().take(self.npartials) {
                    partial.apply_pitch_bend(bend, luts);
                }
            }
        }
    }

    /// Starts the resonator sounding.
    pub fn activate(&mut self) {
        self.active = true;
        self.silence = 0;
    }

    /// Clears the resonator's history.
    pub fn clear(&mut self) {
        for partial in self.partials.iter_mut() {
            partial.clear();
        }
        self.waveguide.clear();
        self.filter.clear(0.0);
    }

    /// Processes one sample and returns the output.
    pub fn process(&mut self, input: f64) -> f64 {
        let mut out = 0.0;

        // use active and silence to turn off strings process if not in use
        if self.active {
            if self.nmodel == ModalModel::OpenTube || self.nmodel == ModalModel::ClosedTube {
                out += self.waveguide.process(input); // waveguide process
            } else {
                for partial in self.partials.iter_mut().take(self.npartials) {
                    out += partial.process(input);
                }
            }
        }

        if out.abs() + input.abs() > 0.00001 {
            self.silence = 0;
        } else {
            self.silence += 1;
        }

        // The C++ compares an int counter against a double sample rate.
        if (self.silence as f64) >= self.srate {
            self.active = false;
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{gains, ratios};

    const SRATE: f64 = 48_000.0;

    fn settings(model: ModalModel, partials: usize) -> ResonatorSettings {
        ResonatorSettings {
            on: true,
            model,
            partials,
            decay: 1.0,
            damp: 0.0,
            tone: 0.0,
            hit: 0.1,
            rel: 1.0,
            inharm: 0.0,
            cut: 0.0,
            radius: 0.5,
            vel_decay: 0.0,
            vel_hit: 0.0,
            vel_inharm: 0.0,
            vel_damp: 0.0,
            vel_tone: 0.0,
        }
    }

    #[test]
    fn a_new_resonator_has_sixty_four_partials() {
        let resonator = Resonator::new();

        assert_eq!(resonator.partials.len(), MAX_PARTIALS);
        for (i, partial) in resonator.partials.iter().enumerate() {
            assert_eq!(partial.k, i + 1);
        }

        assert_eq!(resonator.silence, 0);
        assert!(!resonator.active);
        assert_eq!(resonator.srate, 0.0);
        assert!(!resonator.on);
        assert_eq!(resonator.nmodel, ModalModel::String);
        assert_eq!(resonator.npartials, 0);
        assert_eq!(resonator.decay, 0.0);
        assert_eq!(resonator.radius, 0.0);
        assert_eq!(resonator.cut, 0.0);
    }

    #[test]
    fn the_tube_models_use_the_waveguide() {
        let mut resonator = Resonator::new();
        resonator.set_params(SRATE, &settings(ModalModel::OpenTube, 64));
        resonator.update(
            200.0,
            0.5,
            false,
            1.0,
            ratios(ModalModel::OpenTube),
            gains(ModalModel::OpenTube),
        );
        resonator.activate();

        // The waveguide was tuned and the partials were left alone.
        assert_eq!(resonator.waveguide.f_k, 200.0);
        assert!(
            resonator
                .partials
                .iter()
                .all(|partial| partial.f_k == 1000.0)
        );

        // The output comes from the waveguide, which rings at the tube's
        // frequency. The partials were never retuned, so they cannot be the
        // source of the sound.
        let mut loudest: f64 = 0.0;
        for n in 0..4800 {
            let t = n as f64 / SRATE;
            let excitation = (std::f64::consts::TAU * 200.0 * t).sin();
            loudest = loudest.max(resonator.process(excitation).abs());
        }
        assert!(loudest > 0.0, "the tube did not sound");
        assert!(
            resonator
                .partials
                .iter()
                .all(|partial| partial.f_k == 1000.0)
        );

        // A closed tube is closed.
        let mut closed = Resonator::new();
        closed.set_params(SRATE, &settings(ModalModel::ClosedTube, 64));
        assert!(closed.waveguide.is_closed);
    }

    #[test]
    fn the_partial_count_limits_the_output() {
        let model = ratios(ModalModel::String);
        let gain = gains(ModalModel::String);

        let mut few = Resonator::new();
        few.set_params(SRATE, &settings(ModalModel::String, 4));
        few.update(200.0, 0.5, false, 1.0, model, gain);
        few.activate();

        let mut many = Resonator::new();
        many.set_params(SRATE, &settings(ModalModel::String, 64));
        many.update(200.0, 0.5, false, 1.0, model, gain);
        many.activate();

        let mut few_output = 0.0;
        let mut many_output = 0.0;
        for _ in 0..100 {
            few_output = few.process(1.0);
            many_output = many.process(1.0);
        }

        assert!(few_output != 0.0);
        assert!(many_output != 0.0);
        assert!(
            (few_output - many_output).abs() > 1e-9,
            "the two resonators produced the same output"
        );
    }

    #[test]
    fn an_inactive_resonator_produces_no_output() {
        let mut resonator = Resonator::new();
        resonator.set_params(SRATE, &settings(ModalModel::String, 64));
        resonator.update(
            200.0,
            0.5,
            false,
            1.0,
            ratios(ModalModel::String),
            gains(ModalModel::String),
        );

        assert!(!resonator.active);
        assert_eq!(resonator.process(1.0), 0.0);
    }

    #[test]
    fn the_resonator_deactivates_after_a_second_of_silence() {
        let mut resonator = Resonator::new();
        resonator.set_params(SRATE, &settings(ModalModel::String, 1));

        // A sub-audio frequency puts the only partial out of range, so the
        // resonator is silent.
        resonator.update(
            0.5,
            0.5,
            false,
            1.0,
            ratios(ModalModel::String),
            gains(ModalModel::String),
        );
        resonator.activate();
        assert!(resonator.active);

        for _ in 0..SRATE as usize - 1 {
            assert_eq!(resonator.process(0.0), 0.0);
            assert!(resonator.active, "the resonator gave up too early");
        }

        resonator.process(0.0);
        assert!(!resonator.active, "the resonator stayed active");
    }

    #[test]
    fn the_cutoff_selects_a_low_pass_or_a_high_pass() {
        fn response(cut: f64, freq: f64) -> f64 {
            let mut resonator = Resonator::new();
            let mut params = settings(ModalModel::String, 64);
            params.cut = cut;
            resonator.set_params(SRATE, &params);
            assert_eq!(resonator.cut, cut);

            // 2048 samples to let the transient decay, then 4800 samples,
            // which is exactly 100 periods of a 10 kHz sine at 48 kHz.
            let mut sum = 0.0;
            for n in 0..6848 {
                let t = n as f64 / SRATE;
                let output = resonator
                    .filter
                    .process((std::f64::consts::TAU * freq * t).sin());
                if n >= 2048 {
                    sum += output * output;
                }
            }
            (sum / 4800.0).sqrt()
        }

        let low_cut = -0.5;
        let high_cut = 0.5;

        let low_pass_low = response(low_cut, 100.0);
        let low_pass_high = response(low_cut, 10_000.0);
        assert!(
            low_pass_low > low_pass_high * 10.0,
            "the low pass kept the high frequencies: {low_pass_low} against {low_pass_high}"
        );

        let high_pass_low = response(high_cut, 100.0);
        let high_pass_high = response(high_cut, 10_000.0);
        assert!(
            high_pass_high > high_pass_low * 10.0,
            "the high pass kept the low frequencies: {high_pass_high} against {high_pass_low}"
        );
    }
}
