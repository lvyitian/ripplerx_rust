//! The noise generator.
//!
//! Port of `src/dsp/Noise.{h,cpp}`.

use crate::envelope::{Envelope, Stage};
use crate::filter::Filter;
use crate::rng::Rng;
use crate::utils::{clamp_cpp, clamp_cpp_inverted};

/// The parameters that configure the noise generator.
#[derive(Clone, Copy, Debug)]
pub struct NoiseSettings {
    /// The filter mode: 0 low-pass, 1 band-pass, 2 high-pass.
    pub filter_mode: u8,
    /// The filter's centre frequency.
    pub freq: f64,
    /// The filter's resonance.
    pub q: f64,
    /// The attack time in milliseconds.
    pub attack: f64,
    /// The decay time in milliseconds.
    pub decay: f64,
    /// The sustain level, from 0 to 1.
    pub sustain: f64,
    /// The release time in milliseconds.
    pub release: f64,
    /// How much velocity shifts the filter frequency.
    pub vel_freq: f64,
    /// How much velocity shifts the filter resonance.
    pub vel_q: f64,
    /// The attack tension.
    pub att_ten: f64,
    /// The decay tension.
    pub dec_ten: f64,
    /// The release tension.
    pub rel_ten: f64,
    /// How much velocity shifts the attack.
    pub vel_att: f64,
    /// How much velocity shifts the decay.
    pub vel_dec: f64,
    /// How much velocity shifts the sustain.
    pub vel_sus: f64,
    /// How much velocity shifts the release.
    pub vel_rel: f64,
}

/// A noise generator with a filter and an envelope.
#[derive(Clone, Debug)]
pub struct Noise {
    /// The attack time in milliseconds.
    pub att: f64,
    /// The decay time in milliseconds.
    pub dec: f64,
    /// The sustain level, from 0 to 1.
    pub sus: f64,
    /// The release time in milliseconds.
    pub rel: f64,
    /// The attack tension.
    pub att_ten: f64,
    /// The decay tension.
    pub dec_ten: f64,
    /// The release tension.
    pub rel_ten: f64,
    /// How much velocity shifts the attack.
    pub vel_att: f64,
    /// How much velocity shifts the decay.
    pub vel_dec: f64,
    /// How much velocity shifts the sustain.
    pub vel_sus: f64,
    /// How much velocity shifts the release.
    pub vel_rel: f64,
    /// How much velocity shifts the filter frequency.
    pub vel_freq: f64,
    /// How much velocity shifts the filter resonance.
    pub vel_q: f64,
    /// The sample rate.
    pub srate: f64,
    /// The velocity of the last attack.
    pub vel: f64,
    /// The filter's resonance.
    pub q: f64,
    /// Whether the filter is in use.
    pub filter_active: bool,
    /// The noise's envelope.
    pub env: Envelope,

    filter: Filter,
    /// A duplicate of the filter, used on the oscillator exciters' signal.
    osc_filter: Filter,
    fmode: u8,
    freq: f64,
}

impl Noise {
    /// Creates a noise generator with the C++'s default values.
    pub fn new() -> Self {
        Self {
            att: 0.0,
            dec: 0.0,
            sus: -60.0,
            rel: 0.0,
            att_ten: 0.0,
            dec_ten: 0.0,
            rel_ten: 0.0,
            vel_att: 0.0,
            vel_dec: 0.0,
            vel_sus: 0.0,
            vel_rel: 0.0,
            vel_freq: 0.0,
            vel_q: 0.0,
            srate: 44100.0,
            vel: 0.0,
            q: 0.707,
            filter_active: false,
            env: Envelope::new(),
            filter: Filter::default(),
            osc_filter: Filter::default(),
            fmode: 0,
            freq: 0.0,
        }
    }

    /// Configures the noise generator.
    pub fn init(&mut self, srate: f64, settings: &NoiseSettings) {
        self.srate = srate;
        self.fmode = settings.filter_mode;
        self.freq = settings.freq;
        self.q = settings.q;
        self.vel_freq = settings.vel_freq;
        self.vel_q = settings.vel_q;
        self.init_filter();

        self.att = settings.attack;
        self.dec = settings.decay;
        self.sus = settings.sustain;
        self.rel = settings.release;

        self.att_ten = settings.att_ten;
        self.dec_ten = settings.dec_ten;
        self.rel_ten = settings.rel_ten;

        self.vel_att = settings.vel_att;
        self.vel_dec = settings.vel_dec;
        self.vel_sus = settings.vel_sus;
        self.vel_rel = settings.vel_rel;

        self.init_envelope();
    }

    /// Starts the noise with a velocity.
    pub fn attack(&mut self, vel: f64) {
        self.vel = vel;
        self.init_filter();
        self.init_envelope();
        self.env.attack(1.0);
    }

    /// Recomputes the filter for the current velocity.
    fn init_filter(&mut self) {
        // The C++ clamps the frequency and the resonance with
        // `fmin(hi, fmax(lo, x))`.
        let f = clamp_cpp_inverted(
            20.0,
            20000.0,
            (self.freq.ln() + self.vel * self.vel_freq * (20000.0f64.ln() - 20.0f64.ln())).exp(),
        );
        let res = clamp_cpp_inverted(0.707, 4.0, self.q + self.vel * self.vel_q * (4.0 - 0.707));

        self.filter_active =
            self.fmode == 1 || (self.fmode == 0 && f < 20000.0) || (self.fmode == 2 && f > 20.0);

        match self.fmode {
            0 => {
                // The C++ sets the filter's coefficients in place, which
                // leaves the filter's history alone, so the coefficients are
                // copied across rather than the whole filter being replaced.
                self.filter.copy_from(&Filter::low_pass(self.srate, f, res));
            }
            1 => {
                self.filter
                    .copy_from(&Filter::band_pass(self.srate, f, res));
            }
            2 => {
                self.filter
                    .copy_from(&Filter::high_pass(self.srate, f, res));
            }
            _ => {
                // The C++ throws for an unknown mode. The mode comes from a
                // three way choice, so it cannot be invalid in practice, and
                // here the filter is simply left alone.
                debug_assert!(false, "unknown noise filter mode");
                self.filter_active = false;
            }
        }

        self.osc_filter.copy_from(&self.filter);
    }

    /// Recomputes the envelope for the current velocity.
    fn init_envelope(&mut self) {
        // The C++ clamps the envelope's times and its sustain with
        // `fmax(lo, fmin(hi, x))`.
        let _att = clamp_cpp(
            1.0,
            20000.0,
            (self.att.ln() + self.vel * self.vel_att * (20000.0f64.ln() - 1.0f64.ln())).exp(),
        );
        let _dec = clamp_cpp(
            1.0,
            20000.0,
            (self.dec.ln() + self.vel * self.vel_dec * (20000.0f64.ln() - 1.0f64.ln())).exp(),
        );
        let _sus = clamp_cpp(0.0, 1.0, self.sus + self.vel * self.vel_sus);
        let _rel = clamp_cpp(
            1.0,
            20000.0,
            (self.rel.ln() + self.vel * self.vel_rel * (20000.0f64.ln() - 1.0f64.ln())).exp(),
        );

        // The sustain level is given to the envelope in decibels.
        let sustain_db = _sus * 60.0 - 60.0;
        self.env.init(
            self.srate,
            _att,
            _dec,
            sustain_db,
            _rel,
            self.att_ten,
            self.dec_ten,
            self.rel_ten,
        );
    }

    /// Starts the envelope's release stage.
    pub fn release(&mut self) {
        self.env.release();
    }

    /// Silences the noise generator.
    pub fn clear(&mut self) {
        self.env.reset();
        self.filter.clear(0.0);
        self.osc_filter.clear(0.0);
    }

    /// Produces the next sample of the noise.
    pub fn process(&mut self, rng: &mut Rng) -> f64 {
        if self.env.state == Stage::Off {
            return 0.0;
        }

        self.env.process();
        let mut sample = rng.next_bipolar();
        if self.filter_active {
            sample = self.filter.process(sample);
        }

        if self.env.state == Stage::Off {
            // The envelope has finished, so the filter is cleared to avoid
            // a pop.
            self.filter.clear(0.0);
        }

        sample * self.env.env
    }

    /// Runs an input through the same filter and envelope as this noise.
    pub fn process_osc(&mut self, input: f64) -> f64 {
        if self.env.state == Stage::Off {
            return 0.0;
        }

        let sample = if self.filter_active {
            self.osc_filter.process(input)
        } else {
            input
        };

        sample * self.env.env
    }
}

impl Default for Noise {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 44100.0;

    /// The settings the plugin's defaults produce.
    fn settings() -> NoiseSettings {
        NoiseSettings {
            filter_mode: 2,
            freq: 20.0,
            q: 0.707,
            attack: 1.0,
            decay: 500.0,
            sustain: 0.0,
            release: 500.0,
            vel_freq: 0.0,
            vel_q: 0.0,
            att_ten: 0.4,
            dec_ten: 0.4,
            rel_ten: 0.4,
            vel_att: 0.0,
            vel_dec: 0.0,
            vel_sus: 0.0,
            vel_rel: 0.0,
        }
    }

    #[test]
    fn a_new_noise_generator_has_the_cpp_defaults() {
        let noise = Noise::new();

        assert_eq!(noise.att, 0.0);
        assert_eq!(noise.dec, 0.0);
        assert_eq!(noise.sus, -60.0);
        assert_eq!(noise.rel, 0.0);
        assert_eq!(noise.att_ten, 0.0);
        assert_eq!(noise.dec_ten, 0.0);
        assert_eq!(noise.rel_ten, 0.0);
        assert_eq!(noise.vel_att, 0.0);
        assert_eq!(noise.vel_dec, 0.0);
        assert_eq!(noise.vel_sus, 0.0);
        assert_eq!(noise.vel_rel, 0.0);
        assert_eq!(noise.vel_freq, 0.0);
        assert_eq!(noise.vel_q, 0.0);
        assert_eq!(noise.srate, 44100.0);
        assert_eq!(noise.vel, 0.0);
        assert_eq!(noise.q, 0.707);
        assert!(!noise.filter_active);
        assert_eq!(noise.env.state, Stage::Off);
    }

    #[test]
    fn the_noise_is_silent_before_the_attack() {
        let mut noise = Noise::new();
        noise.init(SRATE, &settings());
        let mut rng = Rng::new(1);

        for _ in 0..100 {
            assert_eq!(noise.process(&mut rng), 0.0);
        }
    }

    #[test]
    fn the_noise_bursts_and_then_falls_silent() {
        let mut noise = Noise::new();
        noise.init(SRATE, &settings());
        noise.attack(1.0);

        let mut rng = Rng::new(1);
        let mut loudest = 0.0f64;
        for _ in 0..1000 {
            loudest = loudest.max(noise.process(&mut rng).abs());
        }
        assert!(loudest > 0.0, "the noise was silent");

        // The release runs the envelope down to silence.
        noise.release();
        for _ in 0..100_000 {
            noise.process(&mut rng);
        }
        assert_eq!(noise.env.state, Stage::Off);
        assert_eq!(noise.process(&mut rng), 0.0);
    }

    #[test]
    fn the_noise_uses_the_supplied_generator() {
        let mut first = Noise::new();
        first.init(SRATE, &settings());
        first.attack(1.0);

        let mut second = Noise::new();
        second.init(SRATE, &settings());
        second.attack(1.0);

        let mut third = Noise::new();
        third.init(SRATE, &settings());
        third.attack(1.0);

        let mut rng_a = Rng::new(42);
        let mut rng_b = Rng::new(42);
        let mut rng_c = Rng::new(43);

        let mut differs = false;
        for _ in 0..1000 {
            let a = first.process(&mut rng_a);
            let b = second.process(&mut rng_b);
            let c = third.process(&mut rng_c);
            assert_eq!(a, b, "the same seed gave different output");
            if a != c {
                differs = true;
            }
        }
        assert!(differs, "different seeds gave the same output");
    }

    #[test]
    fn the_filter_is_active_for_the_three_modes() {
        /// Whether the filter is active for a mode, a frequency and a
        /// velocity.
        fn active(mode: u8, freq: f64, vel: f64, vel_freq: f64) -> bool {
            let mut settings = settings();
            settings.filter_mode = mode;
            settings.freq = freq;
            settings.vel_freq = vel_freq;

            let mut noise = Noise::new();
            noise.init(SRATE, &settings);
            noise.attack(vel);
            noise.filter_active
        }

        // A band-pass filter is always active.
        assert!(active(1, 20.0, 0.0, 0.0));
        assert!(active(1, 20_000.0, 0.0, 0.0));

        // A low-pass filter is inactive once the frequency reaches the top
        // of the range.
        assert!(active(0, 1000.0, 0.0, 0.0));
        assert!(!active(0, 20_000.0, 1.0, 1.0));

        // A high-pass filter is inactive once the frequency reaches the
        // bottom of the range.
        assert!(active(2, 1000.0, 0.0, 0.0));
        assert!(!active(2, 20.0, 1.0, -1.0));
    }

    #[test]
    fn an_unknown_filter_mode_does_not_panic() {
        let mut settings = settings();
        settings.filter_mode = 7;

        let mut noise = Noise::new();

        // The C++ throws for an unknown mode. The debug build asserts that
        // the mode is one of the three the C++ knows about, so the assertion
        // is caught here, and the release build simply ignores the mode.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            noise.init(SRATE, &settings);
        }));
        std::panic::set_hook(hook);

        if cfg!(debug_assertions) {
            assert!(result.is_err(), "the debug build did not assert");
        } else {
            assert!(result.is_ok());
            assert!(!noise.filter_active);
        }
    }

    #[test]
    fn the_oscillator_path_shares_the_filter() {
        let mut settings = settings();
        settings.filter_mode = 1;
        settings.freq = 1000.0;

        let mut noise = Noise::new();
        noise.init(SRATE, &settings);
        noise.attack(1.0);

        // The oscillator filter has the same coefficients as the noise's own
        // filter, so a signal that the filter passes through comes out at
        // the same level as the filtered noise.
        let mut rng = Rng::new(7);
        let mut oscillator = Noise::new();
        oscillator.init(SRATE, &settings);
        oscillator.attack(1.0);

        for _ in 0..100 {
            let noise_sample = noise.process(&mut rng);
            let osc_sample = oscillator.process_osc(noise_sample);
            // The envelope is the same in both, and the filters have the same
            // response, so the levels are of the same order.
            assert!(
                osc_sample.abs() < noise_sample.abs() * 10.0 + 1e-9,
                "the oscillator path was {osc_sample} against {noise_sample}"
            );
        }
    }

    /// Builds a noise generator with the given settings.
    fn noise_with(settings: NoiseSettings) -> Noise {
        let mut noise = Noise::new();
        noise.init(SRATE, &settings);
        noise.attack(0.5);
        noise
    }

    #[test]
    fn a_nan_filter_frequency_clamps_to_the_low_bound() {
        // The C++ clamps the filter frequency with `fmin(hi, fmax(lo, x))`,
        // and its `fmax` returns the non-NaN operand, so a NaN ends up at
        // the low bound of twenty hertz.
        let mut nan_settings = settings();
        nan_settings.filter_mode = 0;
        nan_settings.freq = f64::NAN;

        let mut low_settings = settings();
        low_settings.filter_mode = 0;
        low_settings.freq = 20.0;

        let mut nan_noise = noise_with(nan_settings);
        let mut low_noise = noise_with(low_settings);

        // A low-pass filter is active while its frequency is below twenty
        // kilohertz, so the two noises behave the same way.
        assert!(nan_noise.filter_active);
        assert!(low_noise.filter_active);

        let mut rng_a = Rng::new(11);
        let mut rng_b = Rng::new(11);
        for _ in 0..1000 {
            assert_eq!(nan_noise.process(&mut rng_a), low_noise.process(&mut rng_b));
        }
    }

    #[test]
    fn a_nan_resonance_clamps_to_the_low_bound() {
        // The C++ clamps the resonance with `fmin(hi, fmax(lo, x))` as well,
        // so a NaN ends up at the low bound of 0.707.
        let mut nan_settings = settings();
        nan_settings.filter_mode = 0;
        nan_settings.freq = 1000.0;
        nan_settings.q = f64::NAN;

        let mut low_settings = nan_settings;
        low_settings.q = 0.707;

        let mut high_settings = nan_settings;
        high_settings.q = 4.0;

        let mut nan_noise = noise_with(nan_settings);
        let mut low_noise = noise_with(low_settings);
        let mut high_noise = noise_with(high_settings);

        let mut rng_a = Rng::new(13);
        let mut rng_b = Rng::new(13);
        let mut rng_c = Rng::new(13);
        let mut differs = false;

        for _ in 0..1000 {
            let nan_sample = nan_noise.process(&mut rng_a);
            assert_eq!(nan_sample, low_noise.process(&mut rng_b));
            if nan_sample != high_noise.process(&mut rng_c) {
                differs = true;
            }
        }

        assert!(differs, "the resonance made no difference");
    }

    #[test]
    fn a_nan_envelope_time_clamps_to_the_high_bound() {
        // The C++ clamps the envelope's times with `fmax(lo, fmin(hi, x))`,
        // and its `fmin` returns the non-NaN operand, so a NaN ends up at the
        // high bound of twenty thousand milliseconds.
        let mut nan_settings = settings();
        nan_settings.attack = f64::NAN;
        nan_settings.decay = f64::NAN;
        nan_settings.release = f64::NAN;

        let noise = noise_with(nan_settings);

        assert_eq!(noise.env.att, 20000.0 * 0.001 * SRATE);
        assert_eq!(noise.env.dec, 20000.0 * 0.001 * SRATE);
        assert_eq!(noise.env.rel, 20000.0 * 0.001 * SRATE);
    }

    #[test]
    fn a_nan_sustain_clamps_to_the_high_bound() {
        // The C++ clamps the sustain with `fmax(lo, fmin(hi, x))` too, so a
        // NaN ends up at the high bound of one, which is zero decibels.
        let mut nan_settings = settings();
        nan_settings.sustain = f64::NAN;

        let noise = noise_with(nan_settings);

        assert_eq!(noise.env.sus, 1.0);
    }
}
