//! The voice: a mallet, a noise generator and two resonators.
//!
//! Port of `src/dsp/Voice.{h,cpp}`.

use crate::luts::Luts;
use crate::mallet::{Mallet, MalletType};
use crate::models::{MAX_PARTIALS, ModalModel, ModelTables, Ratios, djembe, gains};
use crate::noise::Noise;
use crate::resonator::Resonator;
use crate::sampler::Sampler;

/// The number of samples a repeated note takes to fade out.
///
/// The C++'s `globals::REPEAT_NOTE_FADE_MS`.
const REPEAT_NOTE_FADE_MS: f64 = 1.0;

/// Produces the oscillator exciters of a resonator.
///
/// This is a free function so that the resonator and the phase array can be
/// borrowed at the same time.
fn oscillators(
    resonator: &Resonator,
    phases: &mut [f64; MAX_PARTIALS],
    srate: f64,
    luts: &Luts,
) -> f64 {
    let mut final_value = 0.0;
    if !resonator.on {
        return final_value;
    }

    let is_tube = resonator.nmodel.is_tube();

    if is_tube {
        phases[0] += resonator.waveguide.f_k / srate;
        if phases[0] > 1.0 {
            phases[0] -= 1.0;
        }
        final_value += if resonator.nmodel == ModalModel::OpenTube {
            // A saw wave produces the same harmonics as an open tube.
            phases[0] * -2.0 + 1.0
        } else if phases[0] < 0.5 {
            // A square wave produces the same harmonics as a closed tube.
            -1.0
        } else {
            1.0
        };
    } else {
        for (index, partial) in resonator.partials.iter().enumerate() {
            if !partial.out_of_range {
                phases[index] += partial.f_k / srate;
                if phases[index] > 1.0 {
                    phases[index] -= 1.0;
                }
                final_value += luts.sin.get(phases[index]);
            }
        }
    }

    // The gain normalisation was set by ear.
    final_value *= if is_tube { 0.125 } else { 0.004 };

    final_value
}

/// One note of the synthesiser.
#[derive(Clone, Debug)]
pub struct Voice {
    /// The note the voice plays.
    pub note: i32,
    /// The note's frequency.
    pub freq: f64,
    /// The note's velocity.
    pub vel: f64,
    /// Whether the note has been released.
    pub is_release: bool,
    /// Whether the note is held down.
    pub is_pressed: bool,
    /// Whether the two resonators are coupled.
    pub couple: bool,
    /// The mallet's key tracking.
    pub mallet_ktrack: f64,
    /// The coupling's strength.
    pub split: f64,
    /// The sample rate.
    pub srate: f64,
    /// The first resonator's model ratio.
    pub a_ratio: f64,
    /// The second resonator's model ratio.
    pub b_ratio: f64,
    /// The timestamp the note was pressed at.
    pub pressed_ts: u64,
    /// The timestamp the note was released at.
    pub release_ts: u64,
    /// Whether the voice is fading out after a repeated note.
    pub is_fading: bool,
    /// The number of samples the fade takes.
    pub fade_total_samples: i32,
    /// The number of samples the fade has left.
    pub fade_samples: i32,
    /// The mallet the voice plays with.
    pub mallet_type: MalletType,
    /// The mallet's centre frequency.
    pub mallet_freq: f64,
    /// The frequency of the note that is being triggered.
    pub new_freq: f64,
    /// The velocity of the note that is being triggered.
    pub new_vel: f64,
    /// The note that is being triggered.
    pub new_note: i32,
    /// The first resonator's pitch factor.
    pub a_pitch_factor: f64,
    /// The second resonator's pitch factor.
    pub b_pitch_factor: f64,
    /// The pitch bend.
    pub pitch_bend: f64,

    /// The voice's mallet.
    pub mallet: Mallet,
    /// The voice's noise generator.
    pub noise: Noise,
    /// The first resonator.
    pub res_a: Resonator,
    /// The second resonator.
    pub res_b: Resonator,

    a_phases: [f64; MAX_PARTIALS],
    b_phases: [f64; MAX_PARTIALS],
}

impl Voice {
    /// Creates a voice with the C++'s default values.
    pub fn new() -> Self {
        Self {
            note: 0,
            freq: 0.0,
            vel: 0.0,
            is_release: false,
            is_pressed: false,
            couple: false,
            mallet_ktrack: 0.0,
            split: 0.0,
            srate: 44100.0,
            a_ratio: 1.0,
            b_ratio: 1.0,
            pressed_ts: 0,
            release_ts: 0,
            is_fading: false,
            fade_total_samples: 0,
            fade_samples: 0,
            mallet_type: MalletType::Impulse,
            mallet_freq: 0.0,
            new_freq: 0.0,
            new_vel: 0.0,
            new_note: 0,
            a_pitch_factor: 1.0,
            b_pitch_factor: 1.0,
            pitch_bend: 1.0,
            mallet: Mallet::new(),
            noise: Noise::new(),
            res_a: Resonator::new(),
            res_b: Resonator::new(),
            a_phases: [0.0; MAX_PARTIALS],
            b_phases: [0.0; MAX_PARTIALS],
        }
    }

    /// Triggers the voice.
    ///
    /// The frequency of the note is given rather than computed, because the
    /// C++ asks MTS-ESP for it and this crate stays free of FFI.
    #[allow(clippy::too_many_arguments)]
    pub fn trigger(
        &mut self,
        timestamp: u64,
        srate: f64,
        note: i32,
        vel: f64,
        mallet_type: MalletType,
        mallet_freq: f64,
        mallet_ktrack: f64,
        skip_fadeout: bool,
        freq: f64,
        models: &ModelTables,
        sampler: &Sampler,
    ) {
        self.srate = srate;
        self.mallet_type = mallet_type;
        self.mallet_freq = mallet_freq;

        self.new_vel = vel;
        self.new_note = note;
        self.new_freq = freq;
        self.mallet_ktrack = mallet_ktrack;

        self.is_release = false;
        self.is_pressed = true;
        self.pressed_ts = timestamp;

        // The active voice fades out before the note is retriggered.
        if skip_fadeout {
            self.trigger_start(false, models, sampler);
        } else if (self.res_a.on && self.res_a.active) || (self.res_b.on && self.res_b.active) {
            self.is_fading = true;
            self.fade_total_samples = (REPEAT_NOTE_FADE_MS * 0.001 * srate) as i32;
            self.fade_samples = self.fade_total_samples;
            self.update_resonators(models);
        } else {
            self.trigger_start(true, models, sampler);
        }
    }

    /// Starts the voice's mallet and noise and activates its resonators.
    pub fn trigger_start(&mut self, reset: bool, models: &ModelTables, sampler: &Sampler) {
        if reset {
            self.res_a.clear();
            self.res_b.clear();
        }
        self.note = self.new_note;
        self.vel = self.new_vel;
        self.freq = self.new_freq;

        self.mallet.trigger(
            self.mallet_type,
            self.srate,
            self.mallet_freq,
            self.note,
            self.mallet_ktrack,
            sampler,
        );
        self.noise.attack(self.vel);
        if self.res_a.on {
            self.res_a.activate();
        }
        if self.res_b.on {
            self.res_b.activate();
        }
        self.update_resonators(models);
    }

    /// Advances the fade out of a repeated note.
    ///
    /// Returns the gain the voice should be played at.
    pub fn fade_out(&mut self, models: &ModelTables, sampler: &Sampler) -> f64 {
        self.fade_samples -= 1;
        if self.fade_samples <= 0 {
            self.is_fading = false;
            self.trigger_start(true, models, sampler);
        }

        if self.is_fading {
            self.fade_samples as f64 / self.fade_total_samples as f64
        } else {
            1.0
        }
    }

    /// Releases the voice.
    pub fn release(&mut self, timestamp: u64, models: &ModelTables) {
        self.is_release = true;
        self.is_pressed = false;
        self.release_ts = timestamp;
        self.noise.release();
        self.update_resonators(models);
    }

    /// Silences the voice.
    pub fn clear(&mut self) {
        self.mallet.clear();
        self.noise.clear();
        self.res_a.clear();
        self.res_b.clear();
    }

    /// Sets the pitch of the two resonators.
    pub fn set_pitch(
        &mut self,
        a_coarse: f64,
        b_coarse: f64,
        a_fine: f64,
        b_fine: f64,
        pitch_bend: f64,
    ) {
        self.a_pitch_factor = 2.0f64.powf((a_coarse + a_fine / 100.0) / 12.0);
        self.b_pitch_factor = 2.0f64.powf((b_coarse + b_fine / 100.0) / 12.0);
        self.pitch_bend = pitch_bend;
    }

    /// Sets the model ratios of the two resonators.
    pub fn set_ratio(&mut self, a_ratio: f64, b_ratio: f64) {
        self.a_ratio = a_ratio;
        self.b_ratio = b_ratio;
    }

    /// Sets the coupling of the two resonators.
    pub fn set_coupling(&mut self, couple: bool, split: f64) {
        self.couple = couple;
        self.split = split;
    }

    /// Scales every ratio of a model by a factor.
    pub fn apply_pitch(model: &mut Ratios, factor: f64) {
        for ratio in model.iter_mut() {
            *ratio *= factor;
        }
    }

    /// Retunes the resonators for a pitch bend.
    pub fn apply_pitch_bend(&mut self, bend: f64, luts: &Luts) {
        if bend != self.pitch_bend {
            self.pitch_bend = bend;
            if self.res_a.on {
                self.res_a.apply_pitch_bend(bend, luts);
            }
            if self.res_b.on {
                self.res_b.apply_pitch_bend(bend, luts);
            }
        }
    }

    /// Produces the next sample of the oscillator exciters.
    pub fn process_oscillators(&mut self, is_a: bool, luts: &Luts) -> f64 {
        if is_a {
            oscillators(&self.res_a, &mut self.a_phases, self.srate, luts)
        } else {
            oscillators(&self.res_b, &mut self.b_phases, self.srate, luts)
        }
    }

    /// The frequency shift a pair of coupled partials gets.
    pub fn freq_shift(&self, fa: f64, fb: f64) -> f64 {
        let avg = (fa + fb) / 2.0;
        // A pseudo random offset, so that the coupled frequencies are not in
        // perfect sync.
        let k = self.split + (avg.cos() / 5.0);
        let w = avg + (((fa - fb) / 2.0).powf(2.0) + (k / 2.5).powf(2.0)).sqrt();
        (fa.max(fb) - w).abs()
    }

    /// The shifts a pair of models gets when the resonators are coupled in
    /// series.
    ///
    /// This walks every pair of partials, so it costs 4,096 square roots and
    /// cosines. It runs on the audio thread at note-on.
    pub fn calc_frequency_shifts(&self, a_model: &Ratios, b_model: &Ratios) -> (Ratios, Ratios) {
        let mut a_shifts = *a_model;
        let mut b_shifts = *b_model;

        for i in 0..MAX_PARTIALS {
            let fa = a_model[i];
            for &fb in b_model.iter() {
                if (fa - fb).abs() <= 4.0 {
                    let shift = self.freq_shift(fa * self.freq, fb * self.freq) / self.freq;
                    a_shifts[i] += if fa > fb { shift } else { -shift };
                    b_shifts[i] += if fa > fb { -shift } else { shift };
                }
            }
        }

        (a_shifts, b_shifts)
    }

    /// Retunes the two resonators.
    ///
    /// The models are copied out of the tables, so the pitch factor and the
    /// coupling's frequency shift never touch the tables themselves.
    pub fn update_resonators(&mut self, models: &ModelTables) {
        let mut a_model = *models.a(self.res_a.nmodel);
        let mut b_model = *models.b(self.res_b.nmodel);
        let a_gain = gains(self.res_a.nmodel);
        let b_gain = gains(self.res_b.nmodel);

        // The djembe model is computed for the voice's own frequency.
        if self.res_a.nmodel == ModalModel::Djembe {
            a_model = djembe(self.freq, self.a_ratio);
        }
        if self.res_b.nmodel == ModalModel::Djembe {
            b_model = djembe(self.freq, self.b_ratio);
        }

        if self.a_pitch_factor != 1.0 {
            Self::apply_pitch(&mut a_model, self.a_pitch_factor);
        }
        if self.b_pitch_factor != 1.0 {
            Self::apply_pitch(&mut b_model, self.b_pitch_factor);
        }

        // If the coupling mode is serial, a frequency splitting is applied.
        if self.couple && self.res_a.on && self.res_b.on {
            let (a_shifts, b_shifts) = self.calc_frequency_shifts(&a_model, &b_model);
            a_model = a_shifts;
            b_model = b_shifts;
        }

        if self.res_a.on {
            self.res_a.update(
                self.freq,
                self.vel,
                self.is_release,
                self.pitch_bend,
                &a_model,
                a_gain,
            );
        }
        if self.res_b.on {
            self.res_b.update(
                self.freq,
                self.vel,
                self.is_release,
                self.pitch_bend,
                &b_model,
                b_gain,
            );
        }
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::Stage;
    use crate::models::ratios;
    use crate::noise::NoiseSettings;
    use crate::resonator::ResonatorSettings;

    const SRATE: f64 = 44100.0;

    /// The resonator settings of a sounding resonator.
    fn resonator_settings(model: ModalModel) -> ResonatorSettings {
        ResonatorSettings {
            on: true,
            model,
            partials: MAX_PARTIALS,
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

    /// The noise settings the plugin's defaults produce.
    fn noise_settings() -> NoiseSettings {
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

    /// A voice with both resonators switched on.
    fn sounding_voice(model: ModalModel) -> Voice {
        let mut voice = Voice::new();
        voice.res_a.set_params(SRATE, &resonator_settings(model));
        voice
            .res_b
            .set_params(SRATE, &resonator_settings(ModalModel::String));
        voice.noise.init(SRATE, &noise_settings());
        voice
    }

    #[test]
    fn a_new_voice_has_the_cpp_defaults() {
        let voice = Voice::new();

        assert_eq!(voice.note, 0);
        assert_eq!(voice.freq, 0.0);
        assert_eq!(voice.vel, 0.0);
        assert!(!voice.is_release);
        assert!(!voice.is_pressed);
        assert!(!voice.couple);
        assert_eq!(voice.mallet_ktrack, 0.0);
        assert_eq!(voice.split, 0.0);
        assert_eq!(voice.srate, 44100.0);
        assert_eq!(voice.a_ratio, 1.0);
        assert_eq!(voice.b_ratio, 1.0);
        assert_eq!(voice.pressed_ts, 0);
        assert_eq!(voice.release_ts, 0);
        assert!(!voice.is_fading);
        assert_eq!(voice.fade_total_samples, 0);
        assert_eq!(voice.fade_samples, 0);
        assert_eq!(voice.mallet_type, MalletType::Impulse);
        assert_eq!(voice.mallet_freq, 0.0);
        assert_eq!(voice.new_freq, 0.0);
        assert_eq!(voice.new_vel, 0.0);
        assert_eq!(voice.new_note, 0);
        assert_eq!(voice.a_pitch_factor, 1.0);
        assert_eq!(voice.b_pitch_factor, 1.0);
        assert_eq!(voice.pitch_bend, 1.0);
    }

    #[test]
    fn triggering_a_voice_starts_it() {
        let models = ModelTables::new();
        let sampler = Sampler::new();
        let mut voice = sounding_voice(ModalModel::String);

        voice.trigger(
            1,
            SRATE,
            69,
            0.8,
            MalletType::Impulse,
            1000.0,
            0.0,
            false,
            440.0,
            &models,
            &sampler,
        );

        assert!(voice.is_pressed);
        assert!(!voice.is_release);
        assert_eq!(voice.pressed_ts, 1);
        assert_eq!(voice.note, 69);
        assert_eq!(voice.vel, 0.8);
        assert_eq!(voice.freq, 440.0);

        assert!(voice.res_a.active);
        assert!(voice.res_b.active);

        // The mallet is counting down and the noise's envelope is running.
        assert!(voice.mallet.countdown > 0);
        assert_eq!(voice.noise.env.state, Stage::Attack);
    }

    #[test]
    fn releasing_a_voice_marks_it_released() {
        let models = ModelTables::new();
        let sampler = Sampler::new();
        let mut voice = sounding_voice(ModalModel::String);

        voice.trigger(
            1,
            SRATE,
            69,
            0.8,
            MalletType::Impulse,
            1000.0,
            0.0,
            false,
            440.0,
            &models,
            &sampler,
        );
        voice.release(2, &models);

        assert!(voice.is_release);
        assert!(!voice.is_pressed);
        assert_eq!(voice.release_ts, 2);
        assert_eq!(voice.noise.env.state, Stage::Release);
    }

    #[test]
    fn the_pitch_factor_follows_the_coarse_and_fine_pitch() {
        let mut voice = Voice::new();

        voice.set_pitch(12.0, -12.0, 50.0, -50.0, 1.0);
        assert!(
            (voice.a_pitch_factor - 2.0f64.powf((12.0 + 0.5) / 12.0)).abs() < 1e-15,
            "the first pitch factor was {}",
            voice.a_pitch_factor
        );
        assert!(
            (voice.b_pitch_factor - 2.0f64.powf((-12.0 - 0.5) / 12.0)).abs() < 1e-15,
            "the second pitch factor was {}",
            voice.b_pitch_factor
        );

        voice.set_pitch(0.0, 0.0, 0.0, 0.0, 1.5);
        assert_eq!(voice.a_pitch_factor, 1.0);
        assert_eq!(voice.b_pitch_factor, 1.0);
        assert_eq!(voice.pitch_bend, 1.5);
    }

    #[test]
    fn the_frequency_shift_is_symmetric() {
        let mut voice = Voice::new();
        voice.freq = 440.0;

        let model = *ratios(ModalModel::String);
        let (a_shifts, b_shifts) = voice.calc_frequency_shifts(&model, &model);

        // The two tables are mirror images of each other: the partials of the
        // two resonators are pushed apart by the same amount.
        for index in 0..MAX_PARTIALS {
            let sum = a_shifts[index] + b_shifts[index];
            assert!(
                (sum - 2.0 * model[index]).abs() < 1e-9,
                "the shifts of entry {index} are not symmetric: {sum}"
            );
        }

        // The shift itself is the pseudo random offset of the coupling, and
        // it is small next to the ratios it shifts.
        for index in 0..MAX_PARTIALS {
            let shift = (a_shifts[index] - model[index]).abs();
            assert!(shift < 0.01, "the shift of entry {index} was {shift}");
        }
    }

    #[test]
    fn coupled_resonators_shift_apart() {
        let mut voice = Voice::new();
        voice.freq = 440.0;

        let string = *ratios(ModalModel::String);
        let beam = *ratios(ModalModel::Beam);

        let (a_shifts, b_shifts) = voice.calc_frequency_shifts(&string, &beam);

        assert_ne!(a_shifts, string);
        assert_ne!(b_shifts, beam);
        assert_ne!(a_shifts, b_shifts);
    }

    #[test]
    fn the_pitch_factor_scales_a_copy_of_the_model() {
        let mut model = *ratios(ModalModel::String);
        let original = model;

        Voice::apply_pitch(&mut model, 2.0);

        for index in 0..MAX_PARTIALS {
            assert_eq!(model[index], original[index] * 2.0);
        }

        // The shared table is not touched.
        assert_eq!(ratios(ModalModel::String), &original);
    }

    #[test]
    fn the_djembe_model_is_computed_per_voice() {
        let models = ModelTables::new();
        let sampler = Sampler::new();
        let mut voice = sounding_voice(ModalModel::Djembe);

        voice.trigger(
            1,
            SRATE,
            60,
            0.8,
            MalletType::Impulse,
            1000.0,
            0.0,
            false,
            200.0,
            &models,
            &sampler,
        );

        // The djembe's partials sit 200 hertz apart, so the second one is
        // twice the note's frequency.
        assert_eq!(voice.res_a.partials[1].f_k, 400.0);

        // The table of the model is not used for the djembe.
        assert_eq!(models.a(ModalModel::Djembe)[1], 3.857_142_857_142_857);
    }

    #[test]
    fn a_repeated_note_fades_the_old_voice_out() {
        let models = ModelTables::new();
        let sampler = Sampler::new();
        let mut voice = sounding_voice(ModalModel::String);

        voice.trigger(
            1,
            SRATE,
            69,
            0.8,
            MalletType::Impulse,
            1000.0,
            0.0,
            false,
            440.0,
            &models,
            &sampler,
        );
        assert!(voice.res_a.active);

        // The same note again fades the voice out instead of retriggering it.
        voice.trigger(
            2,
            SRATE,
            69,
            0.8,
            MalletType::Impulse,
            1000.0,
            0.0,
            false,
            440.0,
            &models,
            &sampler,
        );

        assert!(voice.is_fading);
        assert_eq!(voice.fade_total_samples, (0.001 * SRATE) as i32);
        assert_eq!(voice.fade_samples, voice.fade_total_samples);

        // The fade runs down to nothing and then starts the voice again.
        let mut gain = 1.0;
        for _ in 0..voice.fade_total_samples {
            gain = voice.fade_out(&models, &sampler);
        }
        assert!(!voice.is_fading);
        assert!(gain <= 1.0);
    }
}
