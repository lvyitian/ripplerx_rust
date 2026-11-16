//! Second-order IIR filters in direct form I.
//!
//! Port of `src/dsp/Filter.{h,cpp}`, which implements the RBJ cookbook
//! biquads.

/// A second-order IIR filter in direct form I.
#[derive(Clone, Copy, Debug, Default)]
pub struct Filter {
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

impl Filter {
    /// The denominator coefficients shared by every response.
    fn denominator(srate: f64, freq: f64, q: f64) -> (f64, f64) {
        let w0 = std::f64::consts::TAU * (freq / srate).min(0.49);
        let alpha = w0.sin() / (2.0 * q);
        let scale = 1.0 / (1.0 + alpha);
        (w0.cos() * -2.0 * scale, (1.0 - alpha) * scale)
    }

    /// A low-pass filter.
    pub fn low_pass(srate: f64, freq: f64, q: f64) -> Self {
        let (a1, a2) = Self::denominator(srate, freq, q);
        let b0 = (1.0 + a1 + a2) * 0.25;
        Self {
            a1,
            a2,
            b0,
            b1: b0 * 2.0,
            b2: b0,
            ..Default::default()
        }
    }

    /// A band-pass filter with unity gain at the centre frequency.
    pub fn band_pass(srate: f64, freq: f64, q: f64) -> Self {
        let (a1, a2) = Self::denominator(srate, freq, q);
        let b0 = (1.0 - a2) * 0.5 * q;
        Self {
            a1,
            a2,
            b0,
            b1: 0.0,
            b2: -b0,
            ..Default::default()
        }
    }

    /// A high-pass filter.
    pub fn high_pass(srate: f64, freq: f64, q: f64) -> Self {
        let (a1, a2) = Self::denominator(srate, freq, q);
        let b0 = (1.0 - a1 + a2) * 0.25;
        Self {
            a1,
            a2,
            b0,
            b1: b0 * -2.0,
            b2: b0,
            ..Default::default()
        }
    }

    /// Resets the filter's history, ramping in from `input`.
    pub fn clear(&mut self, input: f64) {
        self.x0 = input;
        self.x1 = input;
        self.y0 = input / (1.0 + self.a1 + self.a2) * (self.b0 + self.b1 + self.b2);
        self.y1 = self.y0;
    }

    /// Copies the coefficients of `src`, leaving this filter's history alone.
    pub fn copy_from(&mut self, src: &Filter) {
        self.a1 = src.a1;
        self.a2 = src.a2;
        self.b0 = src.b0;
        self.b1 = src.b1;
        self.b2 = src.b2;
    }

    /// Processes one sample and returns the output.
    pub fn process(&mut self, sample: f64) -> f64 {
        let x2 = self.x1;
        self.x1 = self.x0;
        self.x0 = sample;

        let y2 = self.y1;
        self.y1 = self.y0;
        self.y0 =
            self.b0 * self.x0 + self.b1 * self.x1 + self.b2 * x2 - self.a1 * self.y1 - self.a2 * y2;

        self.y0
    }

    /// The magnitude of this filter's response at `freq`, computed directly
    /// from its coefficients.
    ///
    /// This exists for the tests within this crate, which check the
    /// difference equation against theory.
    #[allow(dead_code)]
    pub(crate) fn magnitude_at(&self, srate: f64, freq: f64) -> f64 {
        let w = std::f64::consts::TAU * freq / srate;
        let (sin_w, cos_w) = w.sin_cos();
        let (sin_2w, cos_2w) = (2.0 * w).sin_cos();

        let num_re = self.b0 + self.b1 * cos_w + self.b2 * cos_2w;
        let num_im = -(self.b1 * sin_w + self.b2 * sin_2w);
        let den_re = 1.0 + self.a1 * cos_w + self.a2 * cos_2w;
        let den_im = -(self.a1 * sin_w + self.a2 * sin_2w);

        num_re.hypot(num_im) / den_re.hypot(den_im)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 48_000.0;

    fn rms(samples: &[f64]) -> f64 {
        (samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64).sqrt()
    }

    #[test]
    fn low_pass_has_unit_gain_at_dc() {
        let mut filter = Filter::low_pass(SRATE, 1000.0, 0.707);
        let mut last = 0.0;
        for _ in 0..500 {
            last = filter.process(1.0);
        }
        assert!((last - 1.0).abs() < 1e-9, "dc gain was {last}");
    }

    #[test]
    fn high_pass_rejects_dc() {
        let mut filter = Filter::high_pass(SRATE, 1000.0, 0.707);
        let mut last = 0.0;
        for _ in 0..500 {
            last = filter.process(1.0);
        }
        assert!(last.abs() < 1e-12, "dc gain was {last}");
    }

    #[test]
    fn band_pass_rejects_dc() {
        let mut filter = Filter::band_pass(SRATE, 1000.0, 0.707);
        let mut last = 0.0;
        for _ in 0..500 {
            last = filter.process(1.0);
        }
        assert!(last.abs() < 1e-12, "dc gain was {last}");
    }

    #[test]
    fn the_response_matches_the_magnitude_from_the_coefficients() {
        for filter in [
            Filter::low_pass(SRATE, 1000.0, 0.707),
            Filter::band_pass(SRATE, 1000.0, 0.707),
            Filter::high_pass(SRATE, 1000.0, 0.707),
        ] {
            let expected = filter.magnitude_at(SRATE, 1000.0);
            let mut filter = filter;

            // 2048 samples to let the transient decay, then 2016 samples,
            // which is exactly 42 periods of a 1 kHz sine at 48 kHz.
            let mut output = Vec::with_capacity(4064);
            for n in 0..4064 {
                let t = n as f64 / SRATE;
                output.push(filter.process((std::f64::consts::TAU * 1000.0 * t).sin()));
            }

            let measured = rms(&output[2048..]);
            let expected_rms = expected / std::f64::consts::SQRT_2;
            assert!(
                (measured - expected_rms).abs() < 1e-9 * expected_rms.max(1e-9),
                "measured {measured}, expected {expected_rms}"
            );
        }
    }

    #[test]
    fn a_low_pass_attenuates_high_frequencies() {
        let mut filter = Filter::low_pass(SRATE, 1000.0, 0.707);

        // 2048 samples to let the transient decay, then 4800 samples, which is
        // exactly 1000 periods of a 10 kHz sine at 48 kHz.
        let mut output = Vec::with_capacity(6848);
        for n in 0..6848 {
            let t = n as f64 / SRATE;
            output.push(filter.process((std::f64::consts::TAU * 10_000.0 * t).sin()));
        }
        let measured = rms(&output[2048..]);

        let expected = filter.magnitude_at(SRATE, 10_000.0) / std::f64::consts::SQRT_2;
        assert!(
            (measured - expected).abs() < 1e-9 * expected.max(1e-9),
            "measured {measured}, expected {expected}"
        );
    }

    #[test]
    fn copying_coefficients_leaves_the_history_alone() {
        let mut source = Filter::low_pass(SRATE, 1000.0, 0.707);
        source.process(1.0);

        let mut target = Filter::low_pass(SRATE, 1000.0, 0.707);
        target.process(0.5);
        target.copy_from(&source);

        // Same coefficients, so the same response magnitude.
        assert!(
            (source.magnitude_at(SRATE, 500.0) - target.magnitude_at(SRATE, 500.0)).abs() < 1e-12
        );
    }

    #[test]
    fn the_filter_stays_finite_under_a_loud_signal() {
        let mut filter = Filter::band_pass(SRATE, 1000.0, 0.707);
        for n in 0..10_000 {
            let t = n as f64 / SRATE;
            let value = filter.process((std::f64::consts::TAU * 997.0 * t).sin());
            assert!(value.is_finite(), "filter produced {value}");
            assert!(value.abs() < 10.0, "filter produced {value}");
        }
    }
}
