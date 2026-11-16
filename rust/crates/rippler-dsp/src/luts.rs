//! The lookup tables the partials need.
//!
//! The C++ keeps `Partial::a1LUT` and `Partial::sinLUT` as mutable statics,
//! re-initialised whenever the sample rate changes. Rust has no safely
//! shareable mutable statics, so the tables are owned here and built once per
//! sample rate instead.

use crate::lut::LookupTable;

/// The number of samples in each lookup table.
const LUT_SIZE: usize = 4096;

/// The lowest frequency the `a1` table covers.
const A1_FREQ_MIN: f64 = 1.0;

/// The highest frequency the `a1` table covers.
const A1_FREQ_MAX: f64 = 20000.0;

/// The two lookup tables the partials read from.
#[derive(Clone, Debug)]
pub struct Luts {
    /// Maps a frequency onto `-2 * cos(omega)`, for retuning a partial.
    pub a1: LookupTable,
    /// Maps a phase in `[0, 1]` onto a sine, for the oscillator exciters.
    pub sin: LookupTable,
}

impl Luts {
    /// Builds both tables for the given sample rate.
    pub fn new(sample_rate: f64) -> Self {
        let a1 = LookupTable::new(
            |f| {
                let omega = std::f64::consts::TAU * f / sample_rate;
                -2.0 * omega.cos()
            },
            A1_FREQ_MIN,
            A1_FREQ_MAX,
            LUT_SIZE,
        );

        let sin = LookupTable::new(
            |norm| (std::f64::consts::TAU * norm).sin(),
            0.0,
            1.0,
            LUT_SIZE,
        );

        Self { a1, sin }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_a1_table_maps_a_frequency_onto_minus_two_cos_omega() {
        let luts = Luts::new(48_000.0);
        // A quarter of the sample rate is a quarter turn of the phase.
        let expected = -2.0 * (std::f64::consts::FRAC_PI_2).cos();
        assert!((luts.a1.get(12_000.0) - expected).abs() < 1e-9);
    }

    #[test]
    fn the_sin_table_maps_a_phase_onto_a_sine() {
        let luts = Luts::new(48_000.0);
        assert!(luts.sin.get(0.0).abs() < 1e-12);
        assert!((luts.sin.get(0.25) - 1.0).abs() < 1e-6);
        assert!(luts.sin.get(0.5).abs() < 1e-12);
    }

    #[test]
    fn the_a1_table_spans_the_documented_range() {
        let luts = Luts::new(48_000.0);
        assert_eq!(luts.a1.min(), 1.0);
        assert_eq!(luts.a1.max(), 20000.0);
        assert_eq!(luts.a1.len(), 4096);
    }
}
