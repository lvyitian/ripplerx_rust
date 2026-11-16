//! A single mode of a resonator.
//!
//! Port of `src/dsp/Partial.{h,cpp}`.

use crate::luts::Luts;

/// Clamps `x` into `[lo, hi]` the way the C++'s `fmax(lo, fmin(hi, x))` does.
///
/// The C++'s `fmin` and `fmax` return the non-NaN operand, so a NaN is
/// clamped to the high bound here. Rust's `clamp` propagates the NaN instead,
/// which is not what the C++ does.
fn clamp_cpp(lo: f64, hi: f64, x: f64) -> f64 {
    lo.max(hi.min(x))
}

/// Clamps `x` into `[lo, hi]` the way the C++'s `fmin(hi, fmax(lo, x))` does.
///
/// The C++ uses this nesting for the damping and the tone, where the `fmax`
/// is the inner call, so a NaN is clamped to the low bound here.
fn clamp_cpp_inverted(lo: f64, hi: f64, x: f64) -> f64 {
    hi.min(lo.max(x))
}

/// A single mode of a resonator.
///
/// Port of `src/dsp/Partial.{h,cpp}`. This is a second-order bandpass with
/// its own coefficients, rather than a `Filter`, because the C++ divides by
/// `a0` at the end of every sample and folding that normalisation into the
/// coefficients would change the rounding.
#[derive(Clone, Debug)]
pub struct Partial {
    /// The partial number, counting from one.
    k: usize,
    /// The sample rate the partial is tuned for.
    pub srate: f64,

    /// The base decay time in seconds.
    pub decay: f64,
    /// The damping, from -1 to 1.
    pub damp: f64,
    /// The tone, from -1 to 1.
    pub tone: f64,
    /// The strike position along the resonator.
    pub hit: f64,
    /// The release multiplier.
    pub rel: f64,
    /// The inharmonicity.
    pub inharm: f64,

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

    /// The partial's current frequency.
    pub f_k: f64,
    /// Whether the partial has been tuned out of the audible range.
    pub out_of_range: bool,

    base_f_k: f64,
    b0: f64,
    b2: f64,
    a0: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Partial {
    /// The partial's number, counting from one.
    ///
    /// The number is fixed when the partial is created, so it is read-only.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Creates a partial with the C++'s default values.
    pub fn new(k: usize) -> Self {
        Self {
            k,
            srate: 0.0,
            decay: 0.0,
            damp: 0.0,
            tone: 0.0,
            hit: 0.0,
            rel: 0.0,
            inharm: 0.0,
            vel_decay: 0.0,
            vel_hit: 0.0,
            vel_inharm: 0.0,
            vel_damp: 0.0,
            vel_tone: 0.0,
            f_k: 1000.0,
            out_of_range: false,
            base_f_k: 1000.0,
            b0: 0.0,
            b2: 0.0,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Retunes the partial and recomputes its filter coefficients.
    pub fn update(
        &mut self,
        freq: f64,
        ratio: f64,
        ratio_max: f64,
        vel: f64,
        pitch_bend: f64,
        is_release: bool,
    ) {
        self.out_of_range = false;

        // Normalise the velocity contribution on a logarithmic scale.
        let mut inharm_k =
            (self.inharm.ln() + vel * self.vel_inharm * -0.0001f64.ln()).exp() - 0.0001;
        inharm_k = clamp_cpp(0.0, 1.0, inharm_k);
        inharm_k = (1.0 + inharm_k * (ratio - 1.0) * (ratio - 1.0)).sqrt();

        self.f_k = freq * ratio * inharm_k;
        self.base_f_k = self.f_k;
        self.f_k *= pitch_bend;

        // Normalise the velocity contribution on a logarithmic scale.
        let mut decay_k =
            (self.decay.ln() + vel * self.vel_decay * (100.0f64.ln() - 0.01f64.ln())).exp();
        decay_k = clamp_cpp(0.01, 100.0, decay_k);
        if is_release {
            decay_k *= self.rel;
        }

        if self.f_k >= 20000.0 || self.f_k < 1.0 || decay_k == 0.0 {
            self.out_of_range = true;
        }

        let f_max = (20000.0f64).min(freq * ratio_max * inharm_k);
        let omega = std::f64::consts::TAU * self.f_k / self.srate;
        let alpha = std::f64::consts::TAU / self.srate;

        let damp_base = clamp_cpp_inverted(-1.0, 1.0, self.damp + self.vel_damp * 2.0 * vel);
        let damp_k = if damp_base <= 0.0 {
            (freq / self.f_k).powf(damp_base * 2.0)
        } else {
            (f_max / self.f_k).powf(damp_base * 2.0)
        };

        decay_k /= damp_k;

        let tone_base = clamp_cpp_inverted(-1.0, 1.0, self.tone + self.vel_tone * 2.0 * vel);
        let tone_gain = if tone_base <= 0.0 {
            (self.f_k / freq).powf(tone_base * 12.0 / 6.0)
        } else {
            (self.f_k / f_max).powf(tone_base * 12.0 / 6.0)
        };

        let mut amp_k = (std::f64::consts::PI
            * self.k as f64
            * clamp_cpp(0.02, 0.5, self.hit + self.vel_hit * vel / 2.0))
        .sin()
        .abs();
        amp_k *= 35.0;

        // Bandpass filter coefficients.
        self.b0 = alpha * tone_gain * amp_k;
        self.b2 = -alpha * tone_gain * amp_k;
        self.a0 = if decay_k != 0.0 {
            1.0 + alpha / decay_k
        } else {
            0.0
        };
        self.a1 = -2.0 * omega.cos();
        self.a2 = if decay_k != 0.0 {
            1.0 - alpha / decay_k
        } else {
            0.0
        };
    }

    /// Processes one sample and returns the output.
    pub fn process(&mut self, input: f64) -> f64 {
        if self.out_of_range {
            return 0.0;
        }
        let output = ((self.b0 * input + self.b2 * self.x2)
            - (self.a1 * self.y1 + self.a2 * self.y2))
            / self.a0;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    /// Clears the partial's history.
    pub fn clear(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    /// Scales the partial's output by `gain`.
    ///
    /// The coefficients are scaled in place, which is only correct because
    /// [`Partial::update`] resets `b0` and `b2` before they are scaled
    /// again. Calling this twice without an update in between squares the
    /// gain.
    pub fn apply_gain(&mut self, gain: f64) {
        self.b0 *= gain;
        self.b2 *= gain;
    }

    /// Retunes the partial for a pitch bend, using the `a1` lookup table.
    pub fn apply_pitch_bend(&mut self, bend: f64, luts: &Luts) {
        self.out_of_range = false;
        self.f_k = self.base_f_k * bend;
        if self.f_k < 1.0 || self.f_k > 20000.0 {
            self.out_of_range = true;
            return;
        }
        self.a1 = luts.a1.get(self.f_k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 48_000.0;

    fn tuned(k: usize) -> Partial {
        let mut partial = Partial::new(k);
        partial.srate = SRATE;
        partial.decay = 1.0;
        partial
    }

    #[test]
    fn a_new_partial_has_the_cpp_defaults() {
        let partial = Partial::new(3);

        assert_eq!(partial.k(), 3);
        assert_eq!(partial.srate, 0.0);
        assert_eq!(partial.decay, 0.0);
        assert_eq!(partial.damp, 0.0);
        assert_eq!(partial.tone, 0.0);
        assert_eq!(partial.hit, 0.0);
        assert_eq!(partial.rel, 0.0);
        assert_eq!(partial.inharm, 0.0);
        assert_eq!(partial.vel_decay, 0.0);
        assert_eq!(partial.vel_hit, 0.0);
        assert_eq!(partial.vel_inharm, 0.0);
        assert_eq!(partial.vel_damp, 0.0);
        assert_eq!(partial.vel_tone, 0.0);
        assert_eq!(partial.f_k, 1000.0);
        assert!(!partial.out_of_range);

        assert_eq!(partial.base_f_k, 1000.0);
        assert_eq!(partial.b0, 0.0);
        assert_eq!(partial.b2, 0.0);
        assert_eq!(partial.a0, 1.0);
        assert_eq!(partial.a1, 0.0);
        assert_eq!(partial.a2, 0.0);
        assert_eq!(partial.x1, 0.0);
        assert_eq!(partial.x2, 0.0);
        assert_eq!(partial.y1, 0.0);
        assert_eq!(partial.y2, 0.0);
    }

    #[test]
    fn the_partial_is_out_of_range_below_one_hertz() {
        let mut partial = tuned(1);
        partial.update(0.5, 1.0, 64.0, 0.5, 1.0, false);
        assert!(partial.out_of_range);
    }

    #[test]
    fn the_partial_is_out_of_range_above_twenty_kilohertz() {
        let mut partial = tuned(1);
        partial.update(25_000.0, 1.0, 64.0, 0.5, 1.0, false);
        assert!(partial.out_of_range);
    }

    #[test]
    fn an_out_of_range_partial_produces_no_output() {
        let mut partial = tuned(1);
        partial.update(0.5, 1.0, 64.0, 0.5, 1.0, false);
        assert!(partial.out_of_range);
        assert_eq!(partial.process(1.0), 0.0);
    }

    #[test]
    fn the_inharmonicity_raises_the_partial_frequency() {
        // Without any inharmonicity the partial sits at the plain frequency
        // ratio.
        let mut plain = tuned(2);
        plain.inharm = 0.0;
        plain.update(200.0, 2.0, 64.0, 0.5, 1.0, false);
        assert!(
            (plain.f_k - 200.0 * 2.0).abs() < 1e-9,
            "f_k was {}",
            plain.f_k
        );

        // Raising the inharmonicity raises the partial above that frequency.
        let mut previous = plain.f_k;
        for inharm in [0.001, 0.01, 0.1, 0.5, 1.0] {
            let mut partial = tuned(2);
            partial.inharm = inharm;
            partial.update(200.0, 2.0, 64.0, 0.5, 1.0, false);

            assert!(
                partial.f_k > plain.f_k,
                "inharmonicity {inharm} did not raise the frequency"
            );
            assert!(
                partial.f_k > previous,
                "inharmonicity {inharm} did not raise the frequency further"
            );
            previous = partial.f_k;
        }
    }

    #[test]
    fn the_strike_position_nulls_the_partial() {
        // The amplitude is `|sin(pi * k * hit)|`, so a hit of a half period
        // nulls the even partials.
        let mut partial = tuned(2);
        partial.hit = 0.5;
        partial.update(200.0, 1.0, 64.0, 0.5, 1.0, false);

        // `sin(pi)` is not exactly zero, so the null is a very small number
        // rather than a hard zero.
        assert!(partial.b0.abs() < 1e-12, "b0 was {}", partial.b0);
        assert!(partial.process(1.0).abs() < 1e-12);
    }

    #[test]
    fn applying_a_gain_scales_the_output() {
        let mut plain = tuned(1);
        plain.update(200.0, 1.0, 64.0, 0.5, 1.0, false);

        let mut scaled = tuned(1);
        scaled.update(200.0, 1.0, 64.0, 0.5, 1.0, false);
        scaled.apply_gain(2.0);

        let plain_output = plain.process(1.0);
        let scaled_output = scaled.process(1.0);

        assert!(plain_output != 0.0);
        assert!(
            (scaled_output - 2.0 * plain_output).abs() < 1e-12 * plain_output.abs(),
            "plain {plain_output}, scaled {scaled_output}"
        );
    }

    #[test]
    fn a_pitch_bend_retunes_the_partial() {
        let luts = Luts::new(SRATE);

        let mut partial = tuned(1);
        partial.update(200.0, 1.0, 64.0, 0.5, 1.0, false);

        let base = partial.f_k;
        partial.apply_pitch_bend(1.5, &luts);

        assert!((partial.f_k - base * 1.5).abs() < 1e-9);
        assert!(!partial.out_of_range);
    }

    #[test]
    fn a_pitch_bend_out_of_range_silences_the_partial() {
        let luts = Luts::new(SRATE);

        let mut partial = tuned(1);
        partial.update(200.0, 1.0, 64.0, 0.5, 1.0, false);
        assert!(!partial.out_of_range);

        // A bend of two hundred takes the partial well above the top of the
        // audible range.
        partial.apply_pitch_bend(200.0, &luts);
        assert!(partial.out_of_range);
        assert_eq!(partial.process(1.0), 0.0);

        // And a bend of a thousandth takes it below the bottom.
        let mut low = tuned(1);
        low.update(200.0, 1.0, 64.0, 0.5, 1.0, false);
        low.apply_pitch_bend(0.001, &luts);
        assert!(low.out_of_range);
        assert_eq!(low.process(1.0), 0.0);
    }

    #[test]
    fn the_cpp_clamp_clamps_a_nan_to_the_high_bound() {
        // The C++'s `fmin` and `fmax` return the non-NaN operand, so a NaN
        // is clamped to the high bound. Rust's `clamp` propagates it instead.
        assert_eq!(clamp_cpp(0.0, 1.0, f64::NAN), 1.0);
        assert_eq!(clamp_cpp(0.01, 100.0, f64::NAN), 100.0);
        assert_eq!(clamp_cpp(0.02, 0.5, f64::NAN), 0.5);

        // The other nesting, which the C++ uses for the damping and the tone,
        // clamps a NaN to the low bound.
        assert_eq!(clamp_cpp_inverted(-1.0, 1.0, f64::NAN), -1.0);
    }

    #[test]
    fn the_cpp_clamp_agrees_with_the_rust_clamp() {
        for value in [-2.0, -0.5, 0.0, 0.25, 0.5, 1.0, 2.0] {
            assert_eq!(clamp_cpp(0.0, 1.0, value), value.clamp(0.0, 1.0));
            assert_eq!(clamp_cpp_inverted(-1.0, 1.0, value), value.clamp(-1.0, 1.0));
        }

        assert_eq!(clamp_cpp(0.0, 1.0, -1.0), 0.0);
        assert_eq!(clamp_cpp(0.0, 1.0, 2.0), 1.0);
        assert_eq!(clamp_cpp_inverted(-1.0, 1.0, -2.0), -1.0);
        assert_eq!(clamp_cpp_inverted(-1.0, 1.0, 2.0), 1.0);
    }
}
