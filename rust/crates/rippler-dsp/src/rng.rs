//! Deterministic pseudo-random numbers for the noise generator.
//!
//! The JUCE implementation calls the global `std::rand()` from
//! `Noise::process`, which shares one sequence across every voice and cannot
//! be reproduced. Each voice gets its own generator instead, seeded from its
//! index, so a given session always sounds the same.

/// A xorshift64* generator.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

/// The seed substituted for a zero seed, which xorshift cannot escape from.
const DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// `2^53`, the denominator used to turn 53 random bits into a unit interval.
const TWO_POW_53: f64 = 9_007_199_254_740_992.0;

impl Rng {
    /// Creates a generator with the given seed. A seed of zero is replaced
    /// with a fixed non-zero constant.
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { DEFAULT_SEED } else { seed },
        }
    }

    /// Returns the next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Returns a value in `[0, 1)`.
    pub fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / TWO_POW_53
    }

    /// Returns a value in `[-1, 1)`.
    ///
    /// This is the mapping the JUCE implementation used:
    /// `(rand() / RAND_MAX) * 2.0 - 1.0`.
    pub fn next_bipolar(&mut self) -> f64 {
        self.next_unit() * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_produces_the_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_produce_different_sequences() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let differs = (0..100).any(|_| a.next_u64() != b.next_u64());
        assert!(differs, "two different seeds produced identical output");
    }

    #[test]
    fn a_zero_seed_does_not_lock_the_generator_up() {
        let mut rng = Rng::new(0);
        let first = rng.next_u64();
        let second = rng.next_u64();
        assert_ne!(first, second);
        assert_ne!(first, 0);
    }

    #[test]
    fn unit_samples_stay_in_range() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let value = rng.next_unit();
            assert!((0.0..1.0).contains(&value), "out of range: {value}");
        }
    }

    #[test]
    fn bipolar_samples_stay_in_range() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let value = rng.next_bipolar();
            assert!((-1.0..1.0).contains(&value), "out of range: {value}");
        }
    }

    #[test]
    fn bipolar_samples_are_centred_on_zero() {
        let mut rng = Rng::new(12345);
        let count = 100_000;
        let sum: f64 = (0..count).map(|_| rng.next_bipolar()).sum();
        let mean = sum / count as f64;
        assert!(mean.abs() < 0.02, "mean was {mean}");
    }

    #[test]
    fn a_known_seed_produces_the_expected_values() {
        // Golden values for seed 42. They were obtained by re-implementing the
        // xorshift64* step independently of this crate, from the algorithm
        // documented in the module header:
        //
        //     x ^= x >> 12;
        //     x ^= x << 25;
        //     x ^= x >> 27;
        //     output = x * 0x2545F4914F6CDD1D
        //
        // with the state starting at the seed, and the unit value being
        // `(output >> 11) / 2^53`. Pinning the exact numbers means a future
        // refactor of the generator cannot silently change the sound.
        let mut rng = Rng::new(42);

        let expected_u64 = [
            6_255_019_084_209_693_600,
            14_430_073_426_741_505_498,
            14_575_455_857_230_217_846,
            17_414_512_882_241_728_735,
        ];
        for (index, expected) in expected_u64.iter().enumerate() {
            assert_eq!(rng.next_u64(), *expected, "wrong value at {index}");
        }

        let mut rng = Rng::new(42);
        let expected_unit = [
            0.339_085_264_001_921_96,
            0.782_255_847_919_924_3,
            0.790_137_045_268_778_6,
            0.944_042_634_985_164_3,
        ];
        for (index, expected) in expected_unit.iter().enumerate() {
            assert_eq!(rng.next_unit(), *expected, "wrong value at {index}");
        }

        let mut rng = Rng::new(42);
        let expected_bipolar = [
            -0.321_829_471_996_156_1,
            0.564_511_695_839_848_6,
            0.580_274_090_537_557_2,
            0.888_085_269_970_328_6,
        ];
        for (index, expected) in expected_bipolar.iter().enumerate() {
            assert_eq!(rng.next_bipolar(), *expected, "wrong value at {index}");
        }
    }
}
