//! Reproducing JUCE's parameter ranges.
//!
//! JUCE's `NormalisableRange` with a positive skew `s` normalises a value as
//! `p^(1/(1+s))`, and nice-plug's `FloatRange::Skewed` normalises as
//! `p^factor`, so `factor = 1/(1+s)` reproduces JUCE's curve exactly.

use nice_plug_core::params::range::FloatRange;

/// Builds the range for a JUCE parameter with no skew.
pub fn linear(min: f32, max: f32) -> FloatRange {
    FloatRange::Linear { min, max }
}

/// Builds the range for a JUCE parameter with a positive skew.
///
/// `skew` is JUCE's `NormalisableRange` skew. A skew of `0.3` gives a factor of
/// `1 / 1.3`.
pub fn skewed(min: f32, max: f32, skew: f32) -> FloatRange {
    FloatRange::Skewed {
        min,
        max,
        factor: 1.0 / (1.0 + skew),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The normalised value JUCE's skew gives for a plain value.
    fn juce_skewed_normalise(plain: f32, skew: f32) -> f32 {
        plain.powf(1.0 / (1.0 + skew))
    }

    #[test]
    fn the_skew_factor_reproduces_juce_s_curve() {
        for skew in [0.2, 0.3, 0.5, 1.0] {
            let normalised = skewed(0.0, 1.0, skew).normalize(0.5);
            let expected = juce_skewed_normalise(0.5, skew);
            assert!(
                (normalised - expected).abs() < 1e-12,
                "skew {skew}: {normalised} is not {expected}"
            );
        }
    }

    #[test]
    fn a_skew_of_one_is_a_square_root() {
        assert_eq!(skewed(0.0, 1.0, 1.0).normalize(0.25), 0.5);
    }

    #[test]
    fn a_linear_range_is_a_plain_proportion() {
        let range = linear(20.0, 20000.0);
        assert_eq!(range.normalize(20.0), 0.0);
        assert_eq!(range.normalize(20000.0), 1.0);
    }
}
