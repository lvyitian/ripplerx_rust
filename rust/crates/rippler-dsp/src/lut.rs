//! Lookup table with linear and cubic interpolation.
//!
//! Port of `LookupTable` in `src/dsp/Utils.h`.

/// A table of precomputed values sampled evenly across `[min, max]`.
///
/// A `NaN` input is degenerate in Rust: `clamp` propagates the `NaN` and the
/// index saturates to zero, so the result is `NaN` where the C++ would be
/// undefined. The callers only ever pass computed frequencies, so this is
/// documented rather than guarded.
#[derive(Clone, Debug)]
pub struct LookupTable {
    values: Vec<f64>,
    min_value: f64,
    max_value: f64,
    scaler: f64,
    offset: f64,
}

#[allow(clippy::len_without_is_empty)]
impl LookupTable {
    /// Builds a table by sampling `f` at `size` evenly spaced points across
    /// `[min, max]`.
    ///
    /// # Panics
    ///
    /// Panics if `max <= min` or `size < 2`.
    pub fn new<F: Fn(f64) -> f64>(f: F, min: f64, max: f64, size: usize) -> Self {
        assert!(max > min, "max must be greater than min");
        assert!(size >= 2, "size must be at least 2");

        let mut values = Vec::with_capacity(size);
        for i in 0..size {
            let x = i as f64 / (size - 1) as f64;
            let mapped_x = (min + x * (max - min)).clamp(min, max);
            values.push(f(mapped_x));
        }

        let scaler = (size - 1) as f64 / (max - min);
        let offset = -min * scaler;

        Self {
            values,
            min_value: min,
            max_value: max,
            scaler,
            offset,
        }
    }

    /// Linear interpolation. `input` is clamped to the table's range.
    pub fn get(&self, input: f64) -> f64 {
        let input = input.clamp(self.min_value, self.max_value);
        let normalized_index = input * self.scaler + self.offset;
        let index = normalized_index.floor() as usize;
        let last = self.values.len().saturating_sub(1);

        if index >= last {
            return self.values[last];
        }

        let frac = normalized_index - index as f64;
        self.values[index] + frac * (self.values[index + 1] - self.values[index])
    }

    /// Cubic interpolation over the four samples surrounding `input`.
    ///
    /// This is a literal transliteration of the C++ `LookupTable::cubic`: the
    /// same clamped index, the same `i0`/`i2`/`i3` clamping and the same cubic
    /// coefficients. The index clamping is a no-op on the reachable domain,
    /// since the C++'s `i1 = i <= size - 1`, `i0 = max(0, i - 1)` and
    /// `i2`/`i3 = min(size - 1, ...)` never yield an index outside the
    /// vector. The extra `i.min(len - 1)` guard only removes a potential
    /// out-of-bounds read for inputs the C++ would treat as undefined.
    pub fn cubic(&self, input: f64) -> f64 {
        let input = input.clamp(self.min_value, self.max_value);
        let index = input * self.scaler + self.offset;
        let last = self.values.len().saturating_sub(1);
        let i = (index.floor() as usize).min(last);
        let t = index - i as f64;

        let i0 = i.saturating_sub(1);
        let i1 = i;
        let i2 = (i + 1).min(last);
        let i3 = (i + 2).min(last);

        let y0 = self.values[i0];
        let y1 = self.values[i1];
        let y2 = self.values[i2];
        let y3 = self.values[i3];

        let a0 = y3 - y2 - y0 + y1;
        let a1 = y0 - y1 - a0;
        let a2 = y2 - y0;
        let a3 = y1;

        a0 * t * t * t + a1 * t * t + a2 * t + a3
    }

    /// The lower bound of the table's range.
    pub fn min(&self) -> f64 {
        self.min_value
    }

    /// The upper bound of the table's range.
    pub fn max(&self) -> f64 {
        self.max_value
    }

    /// The number of samples in the table.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// The raw sample values.
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_table_interpolates_linearly() {
        let lut = LookupTable::new(|x| x, 0.0, 1.0, 5);
        assert_eq!(lut.get(0.0), 0.0);
        assert_eq!(lut.get(1.0), 1.0);
        assert!((lut.get(0.5) - 0.5).abs() < 1e-12);
        assert!((lut.get(0.125) - 0.125).abs() < 1e-12);
        assert!((lut.get(0.375) - 0.375).abs() < 1e-12);
    }

    #[test]
    fn out_of_range_input_is_clamped() {
        let lut = LookupTable::new(|x| x, 0.0, 1.0, 5);
        assert_eq!(lut.get(-1.0), 0.0);
        assert_eq!(lut.get(2.0), 1.0);
    }

    #[test]
    fn cubic_passes_through_the_stored_samples() {
        let lut = LookupTable::new(|x| x, 0.0, 1.0, 5);
        for (i, value) in lut.values().iter().enumerate() {
            let x = i as f64 / 4.0;
            assert!(
                (lut.cubic(x) - value).abs() < 1e-12,
                "sample {i}: cubic did not pass through the stored value"
            );
        }
    }

    #[test]
    fn cubic_of_a_linear_table_matches_the_interior_midpoints() {
        let lut = LookupTable::new(|x| x, 0.0, 1.0, 5);
        // Interior segments have four evenly spaced samples available, and the
        // cubic reproduces the linear value at the segment midpoint. The first
        // segment is excluded because its left neighbour is clamped.
        assert!((lut.cubic(0.375) - 0.375).abs() < 1e-12);
        assert!((lut.cubic(0.625) - 0.625).abs() < 1e-12);
    }

    #[test]
    fn the_range_is_mapped_onto_the_sample_positions() {
        let lut = LookupTable::new(|x| x * 2.0, 20.0, 20000.0, 64);
        assert_eq!(lut.min(), 20.0);
        assert_eq!(lut.max(), 20000.0);
        assert_eq!(lut.len(), 64);
        assert!((lut.get(20.0) - 40.0).abs() < 1e-9);
        assert!((lut.get(20000.0) - 40000.0).abs() < 1e-9);
    }

    #[test]
    fn a_non_linear_function_is_sampled_at_evenly_spaced_points() {
        let lut = LookupTable::new(|x| x * x, 0.0, 1.0, 3);
        assert_eq!(lut.values(), &[0.0, 0.25, 1.0]);
    }

    #[test]
    fn the_minimum_size_table_of_two_samples_works() {
        let lut = LookupTable::new(|x| x, 0.0, 1.0, 2);
        assert_eq!(lut.len(), 2);
        assert_eq!(lut.values(), &[0.0, 1.0]);

        // The upper end takes the `index >= len - 1` early return in `get`.
        assert_eq!(lut.get(1.0), 1.0);
        assert!((lut.get(0.5) - 0.5).abs() < 1e-12);
        assert!((lut.get(0.25) - 0.25).abs() < 1e-12);
        assert_eq!(lut.get(-1.0), 0.0);
        assert_eq!(lut.get(2.0), 1.0);
    }

    #[test]
    fn the_minimum_size_table_of_two_samples_interpolates_cubically() {
        let lut = LookupTable::new(|x| x, 0.0, 1.0, 2);

        // `i2` and `i3` are clamped to the last sample for every input.
        assert!((lut.cubic(0.0) - 0.0).abs() < 1e-12);
        assert!((lut.cubic(1.0) - 1.0).abs() < 1e-12);
        assert!((lut.cubic(0.5) - 0.5).abs() < 1e-12);
        assert!((lut.cubic(-1.0) - 0.0).abs() < 1e-12);
        assert!((lut.cubic(2.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_negative_min_flips_the_sign_of_the_offset() {
        let lut = LookupTable::new(|x| x, -10.0, 10.0, 5);
        assert_eq!(lut.min(), -10.0);
        assert_eq!(lut.max(), 10.0);
        assert_eq!(lut.values(), &[-10.0, -5.0, 0.0, 5.0, 10.0]);

        // `offset = -min * scaler` is positive here, so the index starts at
        // zero at `min` rather than at the negative end.
        assert!((lut.get(-10.0) - -10.0).abs() < 1e-12);
        assert!((lut.get(0.0) - 0.0).abs() < 1e-12);
        assert!((lut.get(5.0) - 5.0).abs() < 1e-12);
        assert!((lut.get(10.0) - 10.0).abs() < 1e-12);
        assert!((lut.get(-100.0) - -10.0).abs() < 1e-12);
        assert!((lut.get(100.0) - 10.0).abs() < 1e-12);
    }

    #[test]
    fn a_negative_min_interpolates_cubically() {
        let lut = LookupTable::new(|x| x, -10.0, 10.0, 5);
        assert!((lut.cubic(-10.0) - -10.0).abs() < 1e-12);
        assert!((lut.cubic(10.0) - 10.0).abs() < 1e-12);
        assert!((lut.cubic(0.0) - 0.0).abs() < 1e-12);
        assert!((lut.cubic(-100.0) - -10.0).abs() < 1e-12);
        assert!((lut.cubic(100.0) - 10.0).abs() < 1e-12);
    }

    #[test]
    fn cubic_at_the_endpoints_matches_the_stored_samples() {
        let lut = LookupTable::new(|x| x * x, 0.0, 1.0, 5);
        assert_eq!(lut.values(), &[0.0, 0.0625, 0.25, 0.5625, 1.0]);
        assert!((lut.cubic(0.0) - 0.0).abs() < 1e-12);
        assert!((lut.cubic(1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn cubic_outside_the_range_is_clamped_to_the_endpoints() {
        let lut = LookupTable::new(|x| x * x, 0.0, 1.0, 5);
        assert!((lut.cubic(-1.0) - lut.cubic(0.0)).abs() < 1e-12);
        assert!((lut.cubic(2.0) - lut.cubic(1.0)).abs() < 1e-12);
        assert!((lut.cubic(-1.0) - 0.0).abs() < 1e-12);
        assert!((lut.cubic(2.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    #[should_panic(expected = "max must be greater than min")]
    fn an_empty_range_panics() {
        let _ = LookupTable::new(|x| x, 1.0, 1.0, 5);
    }

    #[test]
    #[should_panic(expected = "max must be greater than min")]
    fn an_inverted_range_panics() {
        let _ = LookupTable::new(|x| x, 2.0, 1.0, 5);
    }

    #[test]
    #[should_panic(expected = "size must be at least 2")]
    fn a_table_of_one_sample_panics() {
        let _ = LookupTable::new(|x| x, 0.0, 1.0, 1);
    }
}
