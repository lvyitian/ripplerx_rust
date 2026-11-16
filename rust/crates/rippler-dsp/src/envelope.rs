//! An ADSR envelope with tension shaping.
//!
//! Port of `src/dsp/Envelope.{h,cpp}`, which is a port of Envlib (by dwelx)
//! based on zenvelib.v1.
//!
//! The sustain level is expressed in decibels, matching the C++.

/// The stage an envelope is currently in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    /// Silent, and not running.
    #[default]
    Off,
    /// Rising towards the peak.
    Attack,
    /// Falling towards the sustain level.
    Decay,
    /// Holding at the sustain level.
    Sustain,
    /// Falling towards silence.
    Release,
}

/// An ADSR envelope with per-stage tension.
#[derive(Clone, Copy, Debug, Default)]
pub struct Envelope {
    /// Attack time in samples.
    pub att: f64,
    /// Decay time in samples.
    pub dec: f64,
    /// Sustain level as a linear gain.
    pub sus: f64,
    /// Release time in samples.
    pub rel: f64,
    /// The peak level the envelope scales to.
    pub scale: f64,
    /// The current output level.
    pub env: f64,
    /// The current stage.
    pub state: Stage,

    ab: f64,
    ac: f64,
    db: f64,
    dc: f64,
    rb: f64,
    rc: f64,
    ta: f64,
    td: f64,
    tr: f64,
}

/// Maps a tension in `[-1, 1]` onto the internal range `[0.001..1, 100]`.
fn normalize_tension(t: f64) -> f64 {
    let t = t + 1.0;
    if t == 1.0 {
        100.0
    } else if t > 1.0 {
        3.001 - t
    } else {
        0.001 + t
    }
}

/// Computes the `(b, c)` coefficients of one exponential segment.
fn calc_coefs(
    target_b1: f64,
    target_b2: f64,
    target_c: f64,
    rate: f64,
    tension: f64,
    mult: f64,
) -> (f64, f64) {
    if tension > 1.0 {
        // Slow-start shape.
        let t = (tension - 1.0).powi(3);
        let c = (((target_c + t) / t).ln() / rate).exp();
        let b = (target_b1 - mult * t) * (1.0 - c);
        (b, c)
    } else {
        // Fast-start shape (inverse exponential).
        let t = tension.powi(3);
        let c = (-((target_c + t) / t).ln() / rate).exp();
        let b = (target_b2 + mult * t) * (1.0 - c);
        (b, c)
    }
}

impl Envelope {
    /// Creates an idle envelope.
    pub fn new() -> Self {
        Self {
            state: Stage::Off,
            ..Default::default()
        }
    }

    /// Configures the envelope's times and tensions.
    ///
    /// `a`, `d` and `r` are times in milliseconds. `sustain_db` is the
    /// sustain level in decibels, where `-60` is silent and `0` is full scale.
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        &mut self,
        srate: f64,
        a: f64,
        d: f64,
        sustain_db: f64,
        r: f64,
        tension_a: f64,
        tension_d: f64,
        tension_r: f64,
    ) {
        self.att = a.max(1.0) * 0.001 * srate;
        self.dec = d.max(1.0) * 0.001 * srate;
        self.sus = 10f64.powf(sustain_db.min(0.0) / 20.0);
        self.rel = r.max(1.0) * 0.001 * srate;

        self.ta = normalize_tension(tension_a);
        self.td = normalize_tension(-tension_d);
        self.tr = normalize_tension(-tension_r);
    }

    /// Recomputes the attack and decay coefficients for the current scale.
    fn recalc_coefs(&mut self) {
        let (b, c) = calc_coefs(0.0, self.scale, self.scale, self.att, self.ta, 1.0);
        self.ab = b;
        self.ac = c;

        let (b2, c2) = calc_coefs(
            1.0,
            self.sus * self.scale,
            (1.0 - self.sus) * self.scale,
            self.dec,
            self.td,
            -1.0,
        );
        self.db = b2;
        self.dc = c2;
    }

    /// Silences the envelope and stops it.
    pub fn reset(&mut self) {
        self.state = Stage::Off;
        self.env = 0.0;
    }

    /// Starts the attack stage towards `scale`.
    pub fn attack(&mut self, scale: f64) {
        self.scale = scale;
        self.recalc_coefs();
        self.state = Stage::Attack;
    }

    /// Starts the decay stage.
    pub fn decay(&mut self) {
        self.env = self.scale;
        self.state = Stage::Decay;
    }

    /// Starts the sustain stage.
    pub fn sustain(&mut self) {
        self.env = self.scale * self.sus;
        self.state = Stage::Sustain;
    }

    /// Starts the release stage from the current level.
    pub fn release(&mut self) {
        let from = self.env.max(self.sus) * self.scale;
        let (b, c) = calc_coefs(from, 0.0, from, self.rel, self.tr, -1.0);
        self.rb = b;
        self.rc = c;
        self.state = Stage::Release;
    }

    /// Advances the envelope by one sample and returns its current stage.
    pub fn process(&mut self) -> Stage {
        match self.state {
            Stage::Off => {}
            Stage::Attack => {
                self.env = self.ab + self.env * self.ac;
                if self.env >= self.scale {
                    self.decay();
                }
            }
            Stage::Decay => {
                self.env = self.db + self.env * self.dc;
                if self.env <= self.sus * self.scale {
                    self.sustain();
                }
            }
            Stage::Release => {
                self.env = self.rb + self.env * self.rc;
                if self.env <= 0.0 {
                    self.reset();
                }
            }
            Stage::Sustain => {}
        }
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 48_000.0;

    #[test]
    fn tension_is_normalised_onto_the_internal_range() {
        assert_eq!(normalize_tension(-1.0), 0.001);
        assert_eq!(normalize_tension(0.0), 100.0);
        assert_eq!(normalize_tension(1.0), 1.001);
        assert!((normalize_tension(0.4) - 1.601).abs() < 1e-12);
    }

    #[test]
    fn the_sustain_level_is_expressed_in_decibels() {
        let mut env = Envelope::new();
        env.init(SRATE, 1.0, 10.0, -30.0, 10.0, 0.4, 0.4, 0.4);
        assert!((env.sus - 0.031_622_776_601_683_8).abs() < 1e-15);
    }

    #[test]
    fn the_envelope_reaches_the_sustain_level() {
        let mut env = Envelope::new();
        env.init(SRATE, 1.0, 10.0, -30.0, 10.0, 0.4, 0.4, 0.4);
        env.attack(1.0);

        for _ in 0..10_000 {
            env.process();
        }

        assert_eq!(env.state, Stage::Sustain);
        assert!((env.env - 0.031_622_776_601_683_8).abs() < 1e-9);
    }

    #[test]
    fn the_attack_stage_ends_before_the_decay_stage() {
        let mut env = Envelope::new();
        env.init(SRATE, 1.0, 10.0, -30.0, 10.0, 0.4, 0.4, 0.4);
        env.attack(1.0);

        let mut stage = Stage::Attack;
        for _ in 0..10_000 {
            stage = env.process();
            if stage != Stage::Attack {
                break;
            }
        }

        assert_eq!(stage, Stage::Decay);
    }

    #[test]
    fn releasing_returns_the_envelope_to_silence() {
        let mut env = Envelope::new();
        env.init(SRATE, 1.0, 10.0, -30.0, 10.0, 0.4, 0.4, 0.4);
        env.attack(1.0);
        for _ in 0..10_000 {
            env.process();
        }
        assert_eq!(env.state, Stage::Sustain);

        env.release();
        assert_eq!(env.state, Stage::Release);

        for _ in 0..10_000 {
            env.process();
        }
        assert_eq!(env.state, Stage::Off);
        assert_eq!(env.env, 0.0);
    }

    #[test]
    fn resetting_silences_the_envelope() {
        let mut env = Envelope::new();
        env.init(SRATE, 1.0, 10.0, -30.0, 10.0, 0.4, 0.4, 0.4);
        env.attack(1.0);
        for _ in 0..100 {
            env.process();
        }

        env.reset();
        assert_eq!(env.state, Stage::Off);
        assert_eq!(env.env, 0.0);
    }

    #[test]
    fn the_attack_rises_monotonically() {
        let mut env = Envelope::new();
        env.init(SRATE, 5.0, 10.0, -30.0, 10.0, 0.4, 0.4, 0.4);
        env.attack(1.0);

        let mut previous = env.env;
        for _ in 0..100 {
            env.process();
            assert!(env.env >= previous, "the attack dipped");
            previous = env.env;
        }
    }

    #[test]
    fn the_lowest_sustain_level_is_minus_sixty_decibels() {
        let mut env = Envelope::new();
        env.init(SRATE, 1.0, 10.0, -60.0, 10.0, 0.4, 0.4, 0.4);
        env.attack(1.0);
        for _ in 0..10_000 {
            env.process();
        }
        assert_eq!(env.state, Stage::Sustain);
        // The C++ maps a linear level of zero onto -60 dB, which is 0.001 of
        // full scale. The sustain level never reaches true silence.
        assert!((env.env - 0.001).abs() < 1e-9, "sustain was {}", env.env);
    }
}
