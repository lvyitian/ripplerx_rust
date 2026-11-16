//! A comb-filter stereoizer.
//!
//! Port of `src/dsp/Comb.h`. A single delayed copy of the signal is mixed
//! into the left channel with one sign and the right channel with the other,
//! which widens the stereo image.

/// A comb-filter stereoizer.
#[derive(Clone, Debug, Default)]
pub struct Comb {
    buf: Vec<f64>,
    pos: usize,
}

impl Comb {
    /// The length of the delay line, in milliseconds.
    pub const DELAY_MS: f64 = 20.0;

    /// How much of the delayed signal is mixed in.
    pub const MIX: f64 = 0.33;

    /// Creates a stereoizer with no delay line allocated yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates the delay line for the given sample rate.
    ///
    /// The delay length is clamped to at least one sample, which the C++ does
    /// not do and which would make its modulo operation divide by zero.
    pub fn init(&mut self, srate: f64) {
        self.pos = 0;
        let length = ((Self::DELAY_MS * srate / 1000.0) as usize).max(1);
        self.buf.clear();
        self.buf.resize(length, 0.0);
    }

    /// The delay line's length in samples.
    pub fn delay_len(&self) -> usize {
        self.buf.len()
    }

    /// Processes one sample and returns the left and right outputs.
    ///
    /// If [`Comb::init`] has not been called yet there is no delay line to
    /// read from. The C++ original is undefined behaviour in that state, so
    /// rather than panicking on the audio thread this passes the input
    /// through unchanged on both channels.
    pub fn process(&mut self, input: f64) -> (f64, f64) {
        if self.buf.is_empty() {
            return (input, input);
        }

        self.buf[self.pos] = input;
        self.pos = (self.pos + 1) % self.buf.len();

        let delayed = self.buf[self.pos];
        (input + delayed * Self::MIX, input - delayed * Self::MIX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_delay_line_is_twenty_milliseconds_long() {
        let mut comb = Comb::new();
        comb.init(48_000.0);
        assert_eq!(comb.delay_len(), 960);

        comb.init(44_100.0);
        assert_eq!(comb.delay_len(), 882);
    }

    #[test]
    fn the_delay_line_is_never_empty() {
        let mut comb = Comb::new();
        comb.init(1.0);
        assert_eq!(comb.delay_len(), 1);
    }

    #[test]
    fn a_steady_signal_settles_on_the_expected_mix() {
        let mut comb = Comb::new();
        comb.init(48_000.0);

        let mut last = (0.0, 0.0);
        for _ in 0..2000 {
            last = comb.process(1.0);
        }

        assert!((last.0 - 1.33).abs() < 1e-12, "left was {}", last.0);
        assert!((last.1 - 0.67).abs() < 1e-12, "right was {}", last.1);
    }

    #[test]
    fn processing_before_init_passes_the_input_through_unchanged() {
        // The C++ original has an empty delay line here and is undefined
        // behaviour; the Rust port must not panic on the audio thread.
        let mut comb = Comb::new();
        assert_eq!(comb.delay_len(), 0);

        let (left, right) = comb.process(0.5);
        assert_eq!(left, 0.5);
        assert_eq!(right, 0.5);

        let (left, right) = comb.process(-0.25);
        assert_eq!(left, -0.25);
        assert_eq!(right, -0.25);
    }

    #[test]
    fn the_first_sample_passes_through_unchanged() {
        let mut comb = Comb::new();
        comb.init(48_000.0);
        let (left, right) = comb.process(0.5);
        assert_eq!(left, 0.5);
        assert_eq!(right, 0.5);
    }

    #[test]
    fn an_impulse_is_echoed_after_the_delay() {
        let mut comb = Comb::new();
        comb.init(48_000.0);

        comb.process(1.0);
        // The delay line is written at the current position and read from the
        // next one, so the impulse returns on the 960th call.
        for _ in 0..958 {
            let (left, right) = comb.process(0.0);
            assert!(
                left.abs() < 1e-12 && right.abs() < 1e-12,
                "echo arrived early"
            );
        }

        let (left, right) = comb.process(0.0);
        assert!((left - 0.33).abs() < 1e-12, "left was {left}");
        assert!((right + 0.33).abs() < 1e-12, "right was {right}");
    }
}
