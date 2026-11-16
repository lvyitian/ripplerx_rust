//! Small conversion helpers.
//!
//! Port of the `Utils` class in `src/dsp/Utils.h`.

/// Conversion helpers between normalised values, frequencies and decibels.
///
/// Port of the `Utils` class in `src/dsp/Utils.h`. The helpers are stateless,
/// so they are associated functions on an otherwise empty struct.
pub struct Utils;

/// Clamps `x` into `[lo, hi]` the way the C++'s `fmax(lo, fmin(hi, x))` does.
///
/// The C++'s `fmin` and `fmax` return the non-NaN operand, so a NaN is
/// clamped to the high bound here. Rust's `clamp` propagates the NaN instead,
/// which is not what the C++ does.
///
/// This is the nesting the C++ uses for the envelope's attack, decay, sustain
/// and release, and for the partial's inharmonicity, decay and amplitude.
pub fn clamp_cpp(lo: f64, hi: f64, x: f64) -> f64 {
    lo.max(hi.min(x))
}

/// Clamps `x` into `[lo, hi]` the way the C++'s `fmin(hi, fmax(lo, x))` does.
///
/// The C++'s `fmin` and `fmax` return the non-NaN operand, so a NaN is
/// clamped to the low bound here.
///
/// This is the nesting the C++ uses for the noise's filter frequency and
/// resonance, and for the partial's damping and tone.
pub fn clamp_cpp_inverted(lo: f64, hi: f64, x: f64) -> f64 {
    hi.min(lo.max(x))
}

impl Utils {
    /// `log(20000 / 20)`, the span of the normalised frequency range.
    pub const LOG_MAX_OVER_MIN_FREQ: f64 = 6.907_755_278_982_137;

    /// Maps a normalised frequency onto a frequency in hertz.
    ///
    /// This is the C++ `Utils::normalToFreq`. A normalised value of zero is
    /// 20 Hz and one is 20 kHz.
    pub fn normal_to_freq(norm: f64) -> f64 {
        20.0 * (norm * Self::LOG_MAX_OVER_MIN_FREQ).exp()
    }

    /// Maps a frequency in hertz onto its normalised position.
    ///
    /// This is the C++ `Utils::freqToNormal`, the inverse of
    /// [`Utils::normal_to_freq`].
    pub fn freq_to_normal(norm: f64) -> f64 {
        (norm / 20.0).ln() / Self::LOG_MAX_OVER_MIN_FREQ
    }

    /// Converts a linear gain into decibels.
    ///
    /// This is the C++ `Utils::gainTodB`. A gain of zero maps to `-60` rather
    /// than to negative infinity.
    pub fn gain_to_db(gain: f64) -> f64 {
        if gain == 0.0 {
            -60.0
        } else {
            20.0 * gain.log10()
        }
    }

    /// Maps a normalised value onto a frequency between `min` and `max`.
    ///
    /// This is the C++ `Utils::normalToFreqf`, which works in single
    /// precision.
    pub fn normal_to_freqf(min: f32, max: f32, norm: f32) -> f32 {
        min * (norm * (max / min).ln()).exp()
    }

    /// Maps a frequency between `min` and `max` onto its normalised position.
    ///
    /// This is the C++ `Utils::freqToNormalf`, the inverse of
    /// [`Utils::normal_to_freqf`].
    pub fn freq_to_normalf(min: f32, max: f32, norm: f32) -> f32 {
        (norm / min).ln() / (max / min).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_constant_is_the_span_of_the_frequency_range() {
        assert_eq!(Utils::LOG_MAX_OVER_MIN_FREQ, (20_000f64 / 20.0).ln());
        assert_eq!(Utils::LOG_MAX_OVER_MIN_FREQ, 1000f64.ln());
    }

    #[test]
    fn a_normalised_frequency_maps_onto_the_audio_range() {
        assert!((Utils::normal_to_freq(0.0) - 20.0).abs() < 1e-12);
        assert!((Utils::normal_to_freq(1.0) - 20_000.0).abs() < 1e-9);
        assert!((Utils::normal_to_freq(0.5) - 632.455_532_033_675_9).abs() < 1e-9);
    }

    #[test]
    fn a_frequency_maps_onto_its_normalised_position() {
        assert!((Utils::freq_to_normal(20.0) - 0.0).abs() < 1e-12);
        assert!((Utils::freq_to_normal(20_000.0) - 1.0).abs() < 1e-12);
        assert!((Utils::freq_to_normal(1000.0) - 0.566_323_334_778_672_9).abs() < 1e-12);
    }

    #[test]
    fn the_normalised_frequency_helpers_invert_each_other() {
        for norm in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
            let frequency = Utils::normal_to_freq(norm);
            let round_trip = Utils::freq_to_normal(frequency);
            assert!(
                (round_trip - norm).abs() < 1e-12,
                "{norm} round tripped to {round_trip}"
            );
        }

        for frequency in [20.0, 100.0, 440.0, 1000.0, 10_000.0, 20_000.0] {
            let norm = Utils::freq_to_normal(frequency);
            let round_trip = Utils::normal_to_freq(norm);
            assert!(
                (round_trip - frequency).abs() < 1e-9,
                "{frequency} round tripped to {round_trip}"
            );
        }
    }

    #[test]
    fn a_gain_maps_onto_decibels() {
        assert_eq!(Utils::gain_to_db(1.0), 0.0);
        assert!((Utils::gain_to_db(0.5) + 6.020_599_913_279_624).abs() < 1e-12);
        assert!((Utils::gain_to_db(2.0) - 6.020_599_913_279_625).abs() < 1e-12);
        assert!((Utils::gain_to_db(0.1) + 20.0).abs() < 1e-12);
    }

    #[test]
    fn a_gain_of_zero_maps_onto_minus_sixty_decibels() {
        // Not negative infinity, which is what a plain logarithm would give.
        assert_eq!(Utils::gain_to_db(0.0), -60.0);
        assert!(Utils::gain_to_db(0.0).is_finite());
    }

    #[test]
    fn a_normalised_value_maps_onto_a_bounded_frequency() {
        assert!((Utils::normal_to_freqf(20.0, 20_000.0, 0.0) - 20.0).abs() < 1e-3);
        assert!((Utils::normal_to_freqf(20.0, 20_000.0, 1.0) - 20_000.0).abs() < 1.0);
        assert!((Utils::normal_to_freqf(100.0, 1000.0, 0.5) - 316.227_78).abs() < 1e-3);
    }

    #[test]
    fn a_bounded_frequency_maps_onto_its_normalised_position() {
        assert!((Utils::freq_to_normalf(20.0, 20_000.0, 20.0) - 0.0).abs() < 1e-6);
        assert!((Utils::freq_to_normalf(20.0, 20_000.0, 20_000.0) - 1.0).abs() < 1e-6);
        assert!((Utils::freq_to_normalf(100.0, 1000.0, 316.227_78) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn the_bounded_frequency_helpers_invert_each_other() {
        for norm in [0.0f32, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
            let frequency = Utils::normal_to_freqf(20.0, 20_000.0, norm);
            let round_trip = Utils::freq_to_normalf(20.0, 20_000.0, frequency);
            assert!(
                (round_trip - norm).abs() < 1e-5,
                "{norm} round tripped to {round_trip}"
            );
        }
    }

    #[test]
    fn the_cpp_clamp_clamps_a_nan_to_the_high_bound() {
        // The C++'s `fmax(lo, fmin(hi, x))` returns the non-NaN operand, so
        // a NaN ends up at the high bound.
        assert_eq!(clamp_cpp(0.0, 1.0, f64::NAN), 1.0);
        assert_eq!(clamp_cpp(1.0, 20000.0, f64::NAN), 20000.0);
    }

    #[test]
    fn the_inverted_cpp_clamp_clamps_a_nan_to_the_low_bound() {
        // The C++'s `fmin(hi, fmax(lo, x))` returns the non-NaN operand too,
        // but the `fmax` is the inner call, so a NaN ends up at the low bound.
        assert_eq!(clamp_cpp_inverted(0.0, 1.0, f64::NAN), 0.0);
        assert_eq!(clamp_cpp_inverted(20.0, 20000.0, f64::NAN), 20.0);
    }

    #[test]
    fn the_cpp_clamps_agree_with_the_rust_clamp() {
        for value in [-2.0, -0.5, 0.0, 0.25, 0.5, 1.0, 2.0] {
            assert_eq!(clamp_cpp(0.0, 1.0, value), value.clamp(0.0, 1.0));
            assert_eq!(clamp_cpp_inverted(0.0, 1.0, value), value.clamp(0.0, 1.0));
        }

        assert_eq!(clamp_cpp(0.0, 1.0, -1.0), 0.0);
        assert_eq!(clamp_cpp(0.0, 1.0, 2.0), 1.0);
        assert_eq!(clamp_cpp_inverted(0.0, 1.0, -1.0), 0.0);
        assert_eq!(clamp_cpp_inverted(0.0, 1.0, 2.0), 1.0);
    }
}
