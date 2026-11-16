//! Reproducing JUCE's parameter ranges.
//!
//! JUCE's `NormalisableRange` normalises a value with the proportion of the
//! value's place in the range, and then raises that proportion to the power of
//! the range's skew:
//!
//! ```cpp
//! auto proportion = clampTo0To1 ((v - start) / (end - start));
//!
//! if (exactlyEqual (skew, static_cast<ValueType> (1)))
//!     return proportion;
//!
//! if (! symmetricSkew)
//!     return std::pow (proportion, skew);
//! ```
//!
//! The skew is therefore the exponent of the curve itself, and a skew of
//! exactly `1.0` is not a skew at all but a linear range. nice-plug's
//! `FloatRange::Skewed` normalises as `proportion^factor`, so JUCE's skew is
//! nice-plug's factor.
//!
//! The four-argument `NormalisableRange` constructor leaves `symmetricSkew`
//! alone, so the code above is the one the JUCE build takes.

use nice_plug_core::params::range::FloatRange;

/// Builds the range for a JUCE parameter with no skew.
///
/// This is the range of a `NormalisableRange` whose skew is `1.0`, or of one
/// that was declared without a skew at all.
pub fn linear(min: f32, max: f32) -> FloatRange {
    FloatRange::Linear { min, max }
}

/// Builds the range for a JUCE parameter with a genuine skew.
///
/// `skew` is JUCE's `NormalisableRange` skew, which is the exponent of the
/// curve: a value in the middle of the range normalises to `0.5^skew`. A skew
/// of exactly `1.0` is a linear range rather than a skew, so it belongs to
/// [`linear`].
pub fn skewed(min: f32, max: f32, skew: f32) -> FloatRange {
    assert!(skew != 1.0, "a skew of one is a linear range, not a skew");
    FloatRange::Skewed {
        min,
        max,
        factor: skew,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JUCE's normalisation, straight from `NormalisableRange::convertTo0to1`.
    fn juce_normalise(value: f32, min: f32, max: f32, skew: f32) -> f32 {
        let proportion = ((value - min) / (max - min)).clamp(0.0, 1.0);
        if skew == 1.0 {
            proportion
        } else {
            proportion.powf(skew)
        }
    }

    #[test]
    fn a_skew_is_the_exponent_of_the_curve() {
        for skew in [0.2, 0.3, 0.5] {
            let range = skewed(0.0, 1.0, skew);
            let expected = 0.5f32.powf(skew);
            assert!(
                (range.normalize(0.5) - expected).abs() < 1e-6,
                "skew {skew}: {} is not {expected}",
                range.normalize(0.5)
            );
        }
    }

    #[test]
    fn a_skew_is_not_the_factor_of_a_curve() {
        // The curve JUCE draws is `proportion^0.3`, not `proportion^(1/1.3)`.
        let range = skewed(0.0, 1.0, 0.3);
        let wrong = 0.5f32.powf(1.0 / 1.3);
        assert!((range.normalize(0.5) - wrong).abs() > 1e-2);
    }

    #[test]
    fn a_skewed_range_matches_juce_over_its_whole_range() {
        let range = skewed(20.0, 20000.0, 0.3);
        for value in [20.0, 100.0, 1000.0, 10000.0, 20000.0] {
            let expected = juce_normalise(value, 20.0, 20000.0, 0.3);
            assert!(
                (range.normalize(value) - expected).abs() < 1e-6,
                "{value} normalises to {}, not {expected}",
                range.normalize(value)
            );
        }
    }

    #[test]
    fn a_linear_range_is_a_plain_proportion() {
        let range = linear(20.0, 20000.0);
        assert_eq!(range.normalize(20.0), 0.0);
        assert_eq!(range.normalize(20000.0), 1.0);
        assert!((range.normalize(20.0 + 0.1 * 19980.0) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn a_skew_of_one_is_not_a_skew() {
        assert!(linear(0.0, 1.0).normalize(0.5) == 0.5);
    }

    #[test]
    #[should_panic(expected = "a skew of one is a linear range")]
    fn a_skew_of_one_is_refused() {
        let _ = skewed(0.0, 1.0, 1.0);
    }

    #[test]
    fn a_range_round_trips() {
        for skew in [0.2, 0.3, 0.5] {
            let range = skewed(0.01, 100.0, skew);
            for value in [0.01, 0.1, 1.0, 10.0, 50.0, 100.0] {
                let normalised = range.normalize(value);
                let back = range.unnormalize(normalised);
                assert!(
                    (back - value).abs() < value * 1e-4,
                    "skew {skew}: {value} came back as {back}"
                );
            }
        }

        let range = linear(-48.0, 48.0);
        for value in [-48.0, -1.0, 0.0, 1.0, 48.0] {
            let back = range.unnormalize(range.normalize(value));
            assert!((back - value).abs() < 1e-4, "{value} came back as {back}");
        }
    }
}
