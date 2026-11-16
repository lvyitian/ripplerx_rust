//! The mallet generator.
//!
//! Port of `src/dsp/Mallet.{h,cpp}`.

use crate::filter::Filter;
use crate::sampler::Sampler;

/// The mallet that excites a resonator.
///
/// The discriminants match the `mallet_type` parameter's choice order, so a
/// variant's discriminant is the value the plugin stores. The ten reserved
/// slots must be kept: dropping them would shift every later index and
/// break every existing patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MalletType {
    /// A unit impulse through a band-pass filter.
    Impulse = 0,
    /// Reserved by the JUCE implementation.
    Reserved1 = 1,
    /// Reserved by the JUCE implementation.
    Reserved2 = 2,
    /// Reserved by the JUCE implementation.
    Reserved3 = 3,
    /// Reserved by the JUCE implementation.
    Reserved4 = 4,
    /// Reserved by the JUCE implementation.
    Reserved5 = 5,
    /// Reserved by the JUCE implementation.
    Reserved6 = 6,
    /// Reserved by the JUCE implementation.
    Reserved7 = 7,
    /// Reserved by the JUCE implementation.
    Reserved8 = 8,
    /// Reserved by the JUCE implementation.
    Reserved9 = 9,
    /// Reserved by the JUCE implementation.
    Reserved10 = 10,
    /// A user-supplied sample.
    UserFile = 11,
    /// The bundled `click1.flac` sample.
    Sample1 = 12,
    /// The bundled `click2.flac` sample.
    Sample2 = 13,
    /// The bundled `click3.flac` sample.
    Sample3 = 14,
    /// The bundled `click4.flac` sample.
    Sample4 = 15,
    /// The bundled `click5.flac` sample.
    Sample5 = 16,
    /// The bundled `click6.flac` sample.
    Sample6 = 17,
    /// The bundled `blip.flac` sample.
    Sample7 = 18,
    /// The bundled `blop.flac` sample.
    Sample8 = 19,
    /// The bundled `metal1.flac` sample.
    Sample9 = 20,
    /// The bundled `metal2.flac` sample.
    Sample10 = 21,
    /// The bundled `wood.flac` sample.
    Sample11 = 22,
    /// The bundled `strike.flac` sample.
    Sample12 = 23,
    /// The bundled `perc1.flac` sample.
    Sample13 = 24,
    /// The bundled `perc2.flac` sample.
    Sample14 = 25,
}

/// The mallet that excites a resonator.
///
/// Port of `src/dsp/Mallet.{h,cpp}`.
///
/// The C++ holds a reference to the [`Sampler`], which the plugin swaps
/// under the audio thread. Here the sampler is passed in instead, so the
/// mallet borrows the waveform rather than owning it.
#[derive(Clone, Debug)]
pub struct Mallet {
    /// The sample rate the mallet runs at.
    pub srate: f64,

    /// The impulse's amplitude.
    pub impulse: f64,
    /// The number of samples left in the impulse's countdown.
    pub countdown: i32,
    /// The impulse's decay factor.
    pub env: f64,
    /// The band-pass filter the impulse runs through.
    pub impulse_filter: Filter,

    /// The key tracking, from zero to one.
    pub ktrack: f64,
    /// The factor the key tracking scales the playback speed by.
    pub keytrack_factor: f64,
    /// The position in the sample the mallet is playing.
    pub playback: f64,
    /// The speed the sample is played back at.
    pub playback_speed: f64,
    /// Whether the sample filter is bypassed.
    pub disable_filter: bool,
    /// The filter the sample runs through.
    pub sample_filter: Filter,

    /// The mallet this one is playing.
    type_: MalletType,
}

impl Mallet {
    /// Creates a mallet with the C++'s default values.
    pub fn new() -> Self {
        Self {
            srate: 44100.0,
            impulse: 0.0,
            countdown: 0,
            env: 0.0,
            impulse_filter: Filter::default(),
            ktrack: 0.0,
            keytrack_factor: 1.0,
            playback: f64::INFINITY,
            playback_speed: 1.0,
            disable_filter: false,
            sample_filter: Filter::default(),
            type_: MalletType::Impulse,
        }
    }

    /// Starts the mallet.
    ///
    /// The impulse mallet runs a unit impulse through a band-pass filter for
    /// a hundred milliseconds. Every other mallet plays the sampler's
    /// waveform, at a speed the sampler's sample rate and the key tracking
    /// decide.
    pub fn trigger(
        &mut self,
        mallet_type: MalletType,
        srate: f64,
        freq: f64,
        note: i32,
        ktrack: f64,
        sampler: &Sampler,
    ) {
        self.type_ = mallet_type;
        self.srate = srate;
        self.ktrack = ktrack;

        if self.type_ == MalletType::Impulse {
            // The C++ sets the filter's coefficients in place, which leaves
            // the filter's history alone, so the coefficients are copied
            // across rather than the whole filter being replaced.
            self.impulse_filter
                .copy_from(&Filter::band_pass(srate, freq, 0.707));
            self.countdown = (srate / 10.0) as i32; // countdown (100ms)
            self.impulse = 1.0;
            self.env = (-100.0 / srate).exp();
        } else {
            self.keytrack_factor = 2.0f64.powf(((note - 60) as f64 / 12.0) * ktrack);
            self.playback_speed = sampler.wavesrate / srate;
            self.playback = 0.0;
        }
    }

    /// Silences the mallet.
    pub fn clear(&mut self) {
        self.countdown = 0;
        self.impulse = 0.0;
        self.playback = f64::INFINITY;
        self.impulse_filter.clear(0.0);
        self.sample_filter.clear(0.0);
    }

    /// Produces the next sample of the mallet.
    pub fn process(&mut self, sampler: &Sampler) -> f64 {
        let mut sample = 0.0;

        if self.type_ == MalletType::Impulse && self.countdown > 0 {
            sample = self.impulse_filter.process(self.impulse) * 2.0;
            self.countdown -= 1;
            self.impulse *= self.env;
        } else if (self.type_ as u8) >= (MalletType::UserFile as u8)
            && self.playback < sampler.waveform.len() as f64
        {
            sample = sampler.wave_cubic(self.playback);
            self.playback += self.playback_speed * sampler.pitchfactor * self.keytrack_factor;

            if !self.disable_filter {
                sample = self.sample_filter.process(sample);
            }
        }

        sample
    }

    /// Sets the filter the sample runs through.
    ///
    /// A negative normalisation gives a low-pass filter and a positive one a
    /// high-pass filter. A normalisation of zero bypasses the filter.
    pub fn set_filter(&mut self, norm: f64) {
        // Map 1..0 to 20..20000, with an inverse scale for a negative norm.
        let freq = 20.0 * (20000.0f64 / 20.0).powf(if norm < 0.0 { 1.0 + norm } else { norm });
        self.disable_filter = norm == 0.0;
        if !self.disable_filter {
            if norm < 0.0 {
                self.sample_filter
                    .copy_from(&Filter::low_pass(self.srate, freq, 0.707));
            } else {
                self.sample_filter
                    .copy_from(&Filter::high_pass(self.srate, freq, 0.707));
            }
        }
    }
}

impl Default for Mallet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 44100.0;

    /// A sampler holding a wave of the given frequency.
    ///
    /// The wave starts at one, so the first sample the mallet reads is not
    /// silent.
    fn sine_sampler(freq: f64) -> Sampler {
        let mut sampler = Sampler::new();
        sampler.waveform = (0..4096)
            .map(|index| {
                let t = index as f64 / SRATE;
                (std::f64::consts::TAU * freq * t).cos()
            })
            .collect();
        sampler
    }

    #[test]
    fn the_mallet_type_discriminants_match_the_choice_order() {
        assert_eq!(MalletType::Impulse as u8, 0);

        for index in 1..=10 {
            let reserved = match index {
                1 => MalletType::Reserved1,
                2 => MalletType::Reserved2,
                3 => MalletType::Reserved3,
                4 => MalletType::Reserved4,
                5 => MalletType::Reserved5,
                6 => MalletType::Reserved6,
                7 => MalletType::Reserved7,
                8 => MalletType::Reserved8,
                9 => MalletType::Reserved9,
                _ => MalletType::Reserved10,
            };
            assert_eq!(reserved as u8, index);
        }

        assert_eq!(MalletType::UserFile as u8, 11);

        assert_eq!(MalletType::Sample1 as u8, 12);
        assert_eq!(MalletType::Sample2 as u8, 13);
        assert_eq!(MalletType::Sample3 as u8, 14);
        assert_eq!(MalletType::Sample4 as u8, 15);
        assert_eq!(MalletType::Sample5 as u8, 16);
        assert_eq!(MalletType::Sample6 as u8, 17);
        assert_eq!(MalletType::Sample7 as u8, 18);
        assert_eq!(MalletType::Sample8 as u8, 19);
        assert_eq!(MalletType::Sample9 as u8, 20);
        assert_eq!(MalletType::Sample10 as u8, 21);
        assert_eq!(MalletType::Sample11 as u8, 22);
        assert_eq!(MalletType::Sample12 as u8, 23);
        assert_eq!(MalletType::Sample13 as u8, 24);
        assert_eq!(MalletType::Sample14 as u8, 25);
    }

    #[test]
    fn a_new_mallet_has_the_cpp_defaults() {
        let mallet = Mallet::new();

        assert_eq!(mallet.srate, 44100.0);
        assert_eq!(mallet.impulse, 0.0);
        assert_eq!(mallet.countdown, 0);
        assert_eq!(mallet.env, 0.0);
        assert_eq!(mallet.ktrack, 0.0);
        assert_eq!(mallet.keytrack_factor, 1.0);
        assert_eq!(mallet.playback, f64::INFINITY);
        assert_eq!(mallet.playback_speed, 1.0);
        assert!(!mallet.disable_filter);
    }

    #[test]
    fn the_impulse_mallet_produces_a_decaying_burst() {
        let sampler = Sampler::new();
        let mut mallet = Mallet::new();
        mallet.trigger(MalletType::Impulse, SRATE, 1000.0, 60, 0.0, &sampler);

        assert_eq!(mallet.countdown, (SRATE / 10.0) as i32);
        assert_eq!(mallet.impulse, 1.0);
        assert_eq!(mallet.env, (-100.0 / SRATE).exp());

        let mut early = 0.0f64;
        let mut late = 0.0f64;

        for index in 0..(SRATE as usize / 10) {
            let sample = mallet.process(&sampler);
            if index < 100 {
                early = early.max(sample.abs());
            }
            if index > 3000 {
                late = late.max(sample.abs());
            }
        }

        assert!(early > 0.0, "the impulse was silent");
        assert!(
            late < early * 0.5,
            "the burst did not decay: {early} against {late}"
        );
    }

    #[test]
    fn the_impulse_mallet_stops_after_the_countdown() {
        let sampler = Sampler::new();
        let mut mallet = Mallet::new();
        mallet.trigger(MalletType::Impulse, SRATE, 1000.0, 60, 0.0, &sampler);

        for _ in 0..(SRATE as usize / 10) {
            mallet.process(&sampler);
        }

        assert_eq!(mallet.countdown, 0);
        for _ in 0..100 {
            assert_eq!(mallet.process(&sampler), 0.0);
        }
    }

    #[test]
    fn the_reserved_mallet_types_produce_silence() {
        let sampler = sine_sampler(1000.0);
        let mut mallet = Mallet::new();
        mallet.trigger(MalletType::Reserved1, SRATE, 1000.0, 60, 0.0, &sampler);

        for _ in 0..100 {
            assert_eq!(mallet.process(&sampler), 0.0);
        }
    }

    #[test]
    fn the_sample_mallet_reads_the_sampler() {
        let sampler = sine_sampler(1000.0);
        let mut mallet = Mallet::new();
        mallet.trigger(MalletType::Sample1, SRATE, 1000.0, 60, 0.0, &sampler);
        // The plugin bypasses the sample filter with a normalisation of
        // zero, and a filter that was never set has no coefficients at all.
        mallet.set_filter(0.0);

        assert_eq!(mallet.playback, 0.0);
        assert_eq!(mallet.playback_speed, 1.0);

        let first = mallet.process(&sampler);
        assert_ne!(first, 0.0, "the sample mallet was silent");
        assert!(
            mallet.playback > 0.0,
            "the playback position did not advance"
        );
    }

    #[test]
    fn the_key_tracking_scales_the_playback_speed() {
        let sampler = sine_sampler(1000.0);
        let mut mallet = Mallet::new();

        mallet.trigger(MalletType::Sample1, SRATE, 1000.0, 72, 1.0, &sampler);
        assert!(
            (mallet.keytrack_factor - 2.0).abs() < 1e-12,
            "the key tracking factor was {}",
            mallet.keytrack_factor
        );

        mallet.trigger(MalletType::Sample1, SRATE, 1000.0, 48, 1.0, &sampler);
        assert!(
            (mallet.keytrack_factor - 0.5).abs() < 1e-12,
            "the key tracking factor was {}",
            mallet.keytrack_factor
        );

        // Without key tracking the factor stays at one.
        mallet.trigger(MalletType::Sample1, SRATE, 1000.0, 72, 0.0, &sampler);
        assert_eq!(mallet.keytrack_factor, 1.0);
    }

    #[test]
    fn the_filter_normalisation_selects_a_low_pass_or_a_high_pass() {
        /// The root mean square of the mallet's output.
        fn response(norm: f64, freq: f64) -> f64 {
            let sampler = sine_sampler(freq);
            let mut mallet = Mallet::new();
            mallet.trigger(MalletType::Sample1, SRATE, 1000.0, 60, 0.0, &sampler);
            mallet.set_filter(norm);

            let mut sum = 0.0;
            for _ in 0..2000 {
                let sample = mallet.process(&sampler);
                sum += sample * sample;
            }
            (sum / 2000.0).sqrt()
        }

        let low_pass_low = response(-0.5, 100.0);
        let low_pass_high = response(-0.5, 10_000.0);
        assert!(
            low_pass_low > low_pass_high * 10.0,
            "the low pass kept the high frequencies: {low_pass_low} against {low_pass_high}"
        );

        let high_pass_low = response(0.5, 100.0);
        let high_pass_high = response(0.5, 10_000.0);
        assert!(
            high_pass_high > high_pass_low * 10.0,
            "the high pass kept the low frequencies: {high_pass_high} against {high_pass_low}"
        );
    }

    #[test]
    fn a_zero_normalisation_disables_the_filter() {
        let sampler = sine_sampler(1000.0);
        let mut mallet = Mallet::new();
        mallet.trigger(MalletType::Sample1, SRATE, 1000.0, 60, 0.0, &sampler);
        mallet.set_filter(0.0);

        assert!(mallet.disable_filter);

        // The waveform is played back untouched.
        for index in 0..100 {
            let expected = sampler.wave_cubic(index as f64);
            assert!((mallet.process(&sampler) - expected).abs() < 1e-15);
        }
    }
}
