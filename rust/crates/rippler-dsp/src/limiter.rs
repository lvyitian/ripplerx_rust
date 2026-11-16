//! A brickwall limiter.
//!
//! Port of `src/dsp/Limiter.h`, which is a port of FairlyChildish by Thomas
//! Scott Stillwell.
//!
//! The limiter looks ahead using an RMS estimate, attenuates by a soft-knee
//! ratio, and applies the gain equally to both channels.

/// A stereo brickwall limiter.
#[derive(Clone, Copy, Debug)]
pub struct Limiter {
    log2db: f64,
    db2log: f64,
    rundb: f64,
    runave: f64,
    threshv: f64,
    cthresh: f64,
    cthreshv: f64,
    ratio: f64,
    bias: f64,
    makeupv: f64,
    capsc: f64,
    attime: f64,
    reltime: f64,
    atcoef: f64,
    relcoef: f64,
    rmstime: f64,
    rmscoef: f64,
}

impl Limiter {
    /// `20 / ln(10)`.
    const LOG2DB: f64 = 8.685_889_638_065_036_553_022_578_378_332_1;
    /// `ln(10) / 20`.
    const DB2LOG: f64 = 0.115_129_254_649_702_284_200_899_572_734_22;

    /// Creates an uninitialised limiter. Call [`Limiter::init`] before use.
    ///
    /// This is the same value as [`Limiter::default`]: the two conversion
    /// constants are filled in, so that `init` computes a usable `capsc`.
    pub fn new() -> Self {
        Self {
            log2db: Self::LOG2DB,
            db2log: Self::DB2LOG,
            rundb: 0.0,
            runave: 0.0,
            threshv: 0.0,
            cthresh: 0.0,
            cthreshv: 0.0,
            ratio: 0.0,
            bias: 0.0,
            makeupv: 0.0,
            capsc: 0.0,
            attime: 0.0002,
            reltime: 0.3,
            atcoef: 0.0,
            relcoef: 0.0,
            rmstime: 0.0,
            rmscoef: 0.0,
        }
    }

    /// Configures the limiter.
    ///
    /// `thresh` and `makeup` are in decibels. `bias` is a percentage.
    /// `rms_win` is the RMS window in milliseconds.
    pub fn init(&mut self, srate: f64, thresh: f64, bias: f64, rms_win: f64, makeup: f64) {
        self.threshv = (thresh * self.db2log).exp();
        self.ratio = 20.0;
        self.bias = 80.0 * bias / 100.0;
        self.cthresh = thresh - self.bias;
        self.cthreshv = (self.cthresh * self.db2log).exp();
        self.makeupv = (makeup * self.db2log).exp();
        self.capsc = self.log2db;
        self.attime = 0.0002;
        self.reltime = 0.3;
        self.atcoef = (-1.0 / (self.attime * srate)).exp();
        self.relcoef = (-1.0 / (self.reltime * srate)).exp();

        // NOTE: the original reads `rmstime` before assigning it. On a *first*
        // `init` the field is still zero, so the coefficient is zero and the
        // RMS average tracks the instantaneous value. A *second* `init` reuses
        // the now-set `rmstime`, which gives a non-zero coefficient. The
        // ordering is reproduced here so the sound does not change.
        self.rmscoef = (-1.0 / (self.rmstime * srate)).exp();
        self.rmstime = rms_win / 1_000_000.0;

        self.runave = 0.0;
    }

    /// Processes one stereo sample pair and returns the limited output.
    pub fn process(&mut self, spl0: f64, spl1: f64) -> (f64, f64) {
        let maxspl = spl0.abs().max(spl1.abs());
        let maxspl = maxspl * maxspl;

        self.runave = maxspl + self.rmscoef * (self.runave - maxspl);
        let det = self.runave.max(0.0).sqrt();
        let mut overdb = (self.capsc * (det / self.threshv).ln()).max(0.0);

        if overdb > self.rundb {
            self.rundb = overdb + self.atcoef * (self.rundb - overdb);
        } else {
            self.rundb = overdb + self.relcoef * (self.rundb - overdb);
        }
        overdb = self.rundb.max(0.0);

        let cratio = if self.bias == 0.0 {
            self.ratio
        } else {
            1.0 + (self.ratio - 1.0) * (overdb / self.bias).sqrt()
        };

        let gr = -overdb * (cratio - 1.0) / cratio;
        let grv = (gr * self.db2log).exp();

        (spl0 * grv * self.makeupv, spl1 * grv * self.makeupv)
    }
}

impl Default for Limiter {
    /// Same as [`Limiter::new`].
    ///
    /// A derived `Default` would leave the conversion constants at zero and
    /// make `init` compute a `capsc` of zero, so the limiter would do
    /// nothing. Delegating keeps the two constructors identical.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 48_000.0;

    fn limiter() -> Limiter {
        let mut limiter = Limiter::new();
        limiter.init(SRATE, 0.0, 70.0, 100.0, 0.0);
        limiter
    }

    #[test]
    fn a_signal_below_the_threshold_is_untouched() {
        let mut limiter = limiter();
        let (left, right) = limiter.process(0.5, -0.5);
        assert!((left - 0.5).abs() < 1e-12, "left was {left}");
        assert!((right + 0.5).abs() < 1e-12, "right was {right}");
    }

    #[test]
    fn a_signal_above_the_threshold_is_attenuated() {
        let mut limiter = limiter();
        let mut last = (0.0, 0.0);
        for _ in 0..10_000 {
            last = limiter.process(2.0, 2.0);
        }
        assert!(last.0 < 2.0, "the limiter did not attenuate");
        assert!(last.0 > 0.9 && last.0 < 1.2, "left was {}", last.0);
        assert!((last.0 - last.1).abs() < 1e-12, "the channels diverged");
    }

    #[test]
    fn the_limiter_never_increases_the_signal() {
        let mut limiter = limiter();
        for input in [0.1, 0.5, 0.9, 1.0, 1.5, 4.0] {
            let mut peak = 0.0f64;
            for _ in 0..1000 {
                let (left, _) = limiter.process(input, input);
                peak = peak.max(left.abs());
            }
            assert!(peak <= input + 1e-12, "input {input} came out as {peak}");
        }
    }

    #[test]
    fn silence_stays_silent() {
        let mut limiter = limiter();
        let (left, right) = limiter.process(0.0, 0.0);
        assert_eq!(left, 0.0);
        assert_eq!(right, 0.0);
    }

    #[test]
    fn makeup_gain_is_applied() {
        let mut limiter = Limiter::new();
        limiter.init(SRATE, 0.0, 70.0, 100.0, 6.0);
        let (left, _) = limiter.process(0.5, 0.5);
        let expected = 0.5 * 10f64.powf(6.0 / 20.0);
        assert!((left - expected).abs() < 1e-12, "left was {left}");
    }

    #[test]
    fn the_output_stays_finite_for_extreme_input() {
        let mut limiter = limiter();
        for _ in 0..1000 {
            let (left, right) = limiter.process(1000.0, -1000.0);
            assert!(left.is_finite() && right.is_finite());
        }
    }

    #[test]
    fn the_default_limiter_behaves_like_a_new_one() {
        let mut from_default = Limiter::default();
        let mut from_new = Limiter::new();

        from_default.init(SRATE, 0.0, 70.0, 100.0, 0.0);
        from_new.init(SRATE, 0.0, 70.0, 100.0, 0.0);

        // A limiter built with a derived `Default` would have a `capsc` of
        // zero and would leave the signal completely alone, so a loud signal
        // must still be attenuated here.
        for n in 0..1000 {
            let input = (n as f64 * 0.01).sin() * 2.0;
            let (default_left, default_right) = from_default.process(input, -input);
            let (new_left, new_right) = from_new.process(input, -input);

            assert_eq!(default_left, new_left, "left diverged at {n}");
            assert_eq!(default_right, new_right, "right diverged at {n}");
        }

        assert!(from_default.process(2.0, 2.0).0 < 2.0);
    }

    #[test]
    fn the_rms_window_has_no_effect_on_the_first_init() {
        // The original computes `rmscoef` from `rmstime` before assigning it,
        // so on a first `init` the coefficient is zero and the RMS window has
        // no effect at all. A refactor that fixes the ordering would change
        // the sound, so the quirk is pinned here.
        let mut short_window = Limiter::new();
        let mut long_window = Limiter::new();

        short_window.init(SRATE, 0.0, 70.0, 1.0, 0.0);
        long_window.init(SRATE, 0.0, 70.0, 500.0, 0.0);

        for n in 0..5000 {
            let input = (n as f64 * 0.01).sin() * 2.0;
            let (short_left, short_right) = short_window.process(input, -input);
            let (long_left, long_right) = long_window.process(input, -input);

            assert_eq!(short_left, long_left, "left diverged at {n}");
            assert_eq!(short_right, long_right, "right diverged at {n}");
        }
    }
}
