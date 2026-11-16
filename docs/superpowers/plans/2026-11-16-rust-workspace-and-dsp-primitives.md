# Rust Workspace and DSP Primitives — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the Cargo workspace for the Rust port and port RipplerX's seven DSP primitive modules (lookup table, RNG, biquad, envelope, comb stereoizer, limiter) to tested Rust.

**Architecture:** Three crates under `rust/` — `rippler-dsp` (pure DSP, no framework dependencies), `rippler-params` (compatibility surface), `rippler-plugin` (nice-plug shell). This plan builds the workspace skeleton and completes `rippler-dsp`'s primitive layer; every module is a direct port of its C++ counterpart with analytic tests asserting against theory.

**Tech Stack:** Rust 2024 edition (MSRV 1.88), Cargo workspace, `cargo test`. No third-party dependencies in `rippler-dsp` yet.

**Design doc:** `docs/superpowers/specs/2026-11-16-rust-nice-plug-migration-design.md`

---

## Plan roadmap

The design doc spans six separable subsystems. Each gets its own plan; each plan ends with working, testable software.

| # | Plan | Scope | Depends on |
|---|---|---|---|
| 1 | **This plan** — Rust workspace and DSP primitives | Workspace, `lut`, `rng`, `filter`, `envelope`, `comb`, `limiter` | — |
| 2 | Modal core | `models`, `partial`, `resonator`, `waveguide` | 1 |
| 3 | Excitation and voice | `mallet`, `sampler`, `noise`, `voice` | 1, 2 |
| 4 | Parameters and compatibility | `rippler-params`: params, ranges, enums, legacy XML reader, `rippler-import` CLI | 1, 3 |
| 5 | Plugin shell | `rippler-plugin`: nice-plug `Plugin`, MIDI, voice allocation, VST3/CLAP export, bundling, CI | 4 |
| 6 | Editor | egui editor: knob, tension, meter, pitch, keyboard, menus, themes | 5 |

Plan 5 also carries the two verification items from the design doc: recovering the existing VST3 class ID from a reference JUCE build, and confirming the sidechain input bus is exposed in Reaper and a CLAP host.

## Reference material

The C++ source being ported lives in `src/dsp/` at the repository root and stays untouched throughout this plan. Consult it when a step says "compare against the C++".

- `src/dsp/Utils.h` — `Utils` (freq/dB helpers) and `LookupTable`
- `src/dsp/Filter.{h,cpp}` — RBJ biquad, direct form I
- `src/dsp/Envelope.{h,cpp}` — ADSR with tension (Envlib / zenvelib port)
- `src/dsp/Comb.h` — comb-filter stereoizer
- `src/dsp/Limiter.h` — brickwall limiter (FairlyChildish, by Thomas Scott Stillwell)
- `src/dsp/Noise.cpp` — the only caller of `std::rand()`, mapped in Task 3

## Conventions

- Internal arithmetic is `f64`, matching the C++.
- Every module carries a doc comment stating which C++ file it ports.
- Tests live in a `#[cfg(test)] mod tests` block at the bottom of the module file.
- Module-private helpers are plain `fn`, not `pub`, unless a later plan needs them.
- Commits use conventional prefixes (`feat:`, `test:`, `chore:`).

---

### Task 1: Cargo workspace and crate skeletons

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/rust-toolchain.toml`
- Create: `rust/crates/rippler-dsp/Cargo.toml`
- Create: `rust/crates/rippler-dsp/src/lib.rs`
- Create: `rust/crates/rippler-params/Cargo.toml`
- Create: `rust/crates/rippler-params/src/lib.rs`
- Create: `rust/crates/rippler-plugin/Cargo.toml`
- Create: `rust/crates/rippler-plugin/src/lib.rs`
- Modify: `.gitignore`

- [ ] **Step 1: Create the workspace manifest**

Create `rust/Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = [
    "crates/rippler-dsp",
    "crates/rippler-params",
    "crates/rippler-plugin",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88"
license = "GPL-3.0-or-later"
repository = "https://github.com/tiagolr/ripplerx"
```

The `[workspace.dependencies]` section is added when the first cross-crate
dependency appears. Cargo warns about declared but unused workspace
dependencies, so adding it here would break the clean build this plan ends on.

- [ ] **Step 2: Create `rippler-dsp`**

Create `rust/crates/rippler-dsp/Cargo.toml`:

```toml
[package]
name = "rippler-dsp"
description = "Pure DSP core for the RipplerX physical modelling synthesiser"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
```

Create `rust/crates/rippler-dsp/src/lib.rs`:

```rust
//! Pure DSP core for the RipplerX physical modelling synthesiser.
//!
//! This crate has no dependency on any plugin framework. Everything here is
//! portable, testable in isolation, and free of allocation on the audio path.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod comb;
pub mod envelope;
pub mod filter;
pub mod limiter;
pub mod lut;
pub mod rng;
pub mod utils;
```

The module declarations are added one at a time by the tasks that create them,
so the crate always compiles. The crate lints came out of the final review:
`forbid(unsafe_code)` holds the line on the audio path, and `missing_docs`
keeps every public item documented.

- [ ] **Step 3: Create `rippler-params`**

Create `rust/crates/rippler-params/Cargo.toml`:

```toml
[package]
name = "rippler-params"
description = "Parameter definitions and legacy state compatibility for RipplerX"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
```

The crates start with no cross-crate dependencies. Rust warns about declared but
unused dependencies, so `rippler-params` gains `rippler-dsp` in the plan that
needs it, and `rippler-plugin` gains both.

Create `rust/crates/rippler-params/src/lib.rs`:

```rust
//! Parameter definitions and legacy state compatibility for RipplerX.
//!
//! Everything that has to stay compatible with the JUCE implementation lives
//! here: parameter identifiers, value ranges, and the legacy XML reader.
```

- [ ] **Step 4: Create `rippler-plugin`**

Create `rust/crates/rippler-plugin/Cargo.toml`:

```toml
[package]
name = "rippler-plugin"
description = "The RipplerX audio plugin"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[lib]
crate-type = ["cdylib", "lib"]

[dependencies]
```

`crate-type` includes both `cdylib` (what the host loads) and `lib` (so the crate can be unit tested and linked by the CLI).

Create `rust/crates/rippler-plugin/src/lib.rs`:

```rust
//! The RipplerX audio plugin.
```

- [ ] **Step 5: Pin the toolchain**

Create `rust/rust-toolchain.toml`:

```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy"]
```

The local toolchain is nightly, and nice-plug documents that some of its
functionality depends on optional nightly features. Pinning the channel keeps
every contributor and the CI build on the same compiler.

- [ ] **Step 6: Ignore build output**

Append to `.gitignore`:

```
# Rust
rust/target/
```

The file currently ends without a trailing newline. Add one first, so the
appended block starts on its own line.

- [ ] **Step 7: Verify the workspace builds**

Run: `cd rust && cargo build`
Expected: compiles all three crates with no errors.

Run: `cd rust && cargo test`
Expected: `running 0 tests` for each crate, no failures.

- [ ] **Step 8: Commit**

```bash
git add rust .gitignore
git commit -m "chore: scaffold the Rust workspace for the RipplerX port"
```

---

### Task 2: Lookup table

**Files:**
- Create: `rust/crates/rippler-dsp/src/lut.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod lut;`)

Port of `LookupTable` in `src/dsp/Utils.h`. The C++ version is a re-implementation of JUCE's `LookupTableTransform` with cubic interpolation added.

- [ ] **Step 1: Declare the module**

Add `pub mod lut;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `rust/crates/rippler-dsp/src/lut.rs`:

```rust
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
```

- [ ] **Step 3: Run the test to verify it passes**

The implementation and the tests are written together here because the port is
mechanical; the test is the specification of the behaviour being preserved.

Run: `cd rust && cargo test -p rippler-dsp lut`
Expected: 15 tests pass.

- [ ] **Step 4: Verify the port against the C++**

Compare `get` and `cubic` against `LookupTable::operator()` and
`LookupTable::cubic()` in `src/dsp/Utils.h`. The Rust version is a literal
transliteration and should produce identical results.

Two details are worth checking explicitly, because both are easy to get wrong:

- The C++ computes the index as `int i = (int)index`, which truncates towards
  zero, while Rust floors. These agree because the input is clamped to
  `[min, max]` and `offset = -min * scaler` is the exact negation of
  `min * scaler`, so the index is never negative.
- The C++ clamps `i0` to `max(0, i - 1)` and `i2`/`i3` to `min(size - 1, ...)`.
  Together with `i <= size - 1` that keeps every index in range, so the C++
  does not actually read out of bounds. The Rust version adds an explicit
  `i.min(len - 1)` guard, which is a no-op on the reachable domain and only
  makes the code memory-safe on inputs the C++ would treat as undefined.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-dsp/src/lut.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the lookup table with linear and cubic interpolation"
```

---

### Task 3: Per-voice random number generator

**Files:**
- Create: `rust/crates/rippler-dsp/src/rng.rs`

The C++ noise generator calls the global `std::rand()`, which is shared between
all voices, unseeded, and not reproducible. This task introduces a deterministic
per-voice generator. The mapping the C++ uses is in `src/dsp/Noise.cpp`:

```cpp
double sample = (std::rand() / (double)RAND_MAX) * 2.0 - 1.0;
```

- [ ] **Step 1: Declare the module**

Add `pub mod rng;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `rust/crates/rippler-dsp/src/rng.rs`:

```rust
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
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cd rust && cargo test -p rippler-dsp rng`
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-dsp/src/rng.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): add a deterministic per-voice random number generator"
```

---

### Task 4: Biquad filter

**Files:**
- Create: `rust/crates/rippler-dsp/src/filter.rs`

Port of `src/dsp/Filter.{h,cpp}`. The C++ mutates a filter in place with
`lp`/`bp`/`hp`; in Rust these become constructors, because a filter's
coefficients and its history are set at different times (the noise generator
copies coefficients from one filter to another with `Filter::copy`).

The C++ `copy` copies only the coefficients, not the history. `copy_from` below
does the same, which is why the noise generator keeps two filters with separate
histories but identical coefficients.

- [ ] **Step 1: Declare the module**

Add `pub mod filter;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `rust/crates/rippler-dsp/src/filter.rs`:

```rust
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
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cd rust && cargo test -p rippler-dsp filter`
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-dsp/src/filter.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the RBJ biquad filters"
```

---

### Task 5: Envelope with tension

**Files:**
- Create: `rust/crates/rippler-dsp/src/envelope.rs`

Port of `src/dsp/Envelope.{h,cpp}`, which is itself a port of Envlib (by
dwelx) based on zenvelib.

**Important:** the envelope's `sustain` argument is in **decibels**, not a linear
0..1 level. The C++ computes `sus = pow(10, min(s, 0) / 20)`, and its only
caller (`Noise::initEnvelope` in `src/dsp/Noise.cpp`) converts a linear level to
decibels first with `susToDb(val) = val * 60.0 - 60.0`. The Rust port keeps the
decibel convention and names the argument `sustain_db` so the unit is visible.

The stage constants in the C++ are bit flags (`0` off, `1` attack, `2` decay,
`4` sustain, `8` release). In Rust they become a plain enum.

- [ ] **Step 1: Declare the module**

Add `pub mod envelope;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `rust/crates/rippler-dsp/src/envelope.rs`:

```rust
//! An ADSR envelope with tension shaping.
//!
//! Port of `src/dsp/Envelope.{h,cpp}`, which is a port of Envlib (by dwelx)
//! based on zenvelib.v1.
//!
//! The sustain level is given to [`Envelope::init`] in decibels and stored as
//! a linear gain, matching the C++.

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
    /// The sustain level as a linear gain.
    ///
    /// [`Envelope::init`] takes the sustain level in decibels and converts it
    /// into this linear gain.
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
    initialised: bool,
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

        self.initialised = true;
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
    ///
    /// An envelope that has not been configured by [`Envelope::init`] has no
    /// attack rate, and starting it would divide by a zero rate and jump
    /// straight to full scale. The attack is therefore ignored and the
    /// envelope stays off until it has been initialised.
    pub fn attack(&mut self, scale: f64) {
        if !self.initialised {
            return;
        }

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
    fn attacking_an_uninitialised_envelope_does_nothing() {
        // Without an `init` call there is no attack rate, and starting the
        // attack would divide by a zero rate and jump straight to full
        // scale, which is a loud click on the audio thread.
        let mut env = Envelope::new();
        env.attack(1.0);

        assert_eq!(env.state, Stage::Off);
        assert_eq!(env.env, 0.0);

        // The envelope stays off until it is initialised.
        for _ in 0..1000 {
            assert_eq!(env.process(), Stage::Off);
        }
        assert_eq!(env.env, 0.0);
    }

    #[test]
    fn an_initialised_envelope_still_attacks() {
        let mut env = Envelope::new();
        env.init(SRATE, 5.0, 50.0, -30.0, 100.0, 0.0, 0.0, 0.0);
        env.attack(1.0);

        assert_eq!(env.state, Stage::Attack);

        let mut previous = env.env;
        for _ in 0..100 {
            env.process();
            assert!(env.env > previous, "the attack did not rise");
            previous = env.env;
        }
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

    /// Returns the level after each of the first `steps` samples of an
    /// attack, for an envelope built with the given tensions.
    fn attack_curve(tension_a: f64, tension_d: f64, tension_r: f64, steps: usize) -> Vec<f64> {
        let mut env = Envelope::new();
        env.init(
            SRATE, 5.0, 50.0, -30.0, 100.0, tension_a, tension_d, tension_r,
        );
        env.attack(1.0);

        let mut curve = Vec::with_capacity(steps);
        for _ in 0..steps {
            env.process();
            curve.push(env.env);
        }
        curve
    }

    /// The number of samples the attack takes to reach half of the scale.
    fn samples_to_half(curve: &[f64]) -> usize {
        curve
            .iter()
            .position(|level| *level >= 0.5)
            .expect("the attack never reached half of the scale")
    }

    #[test]
    fn the_slow_start_shape_differs_from_the_fast_start_shape() {
        // A tension argument of zero normalises to 100, which is greater than
        // one, so every stage takes the slow-start branch. A negative tension
        // for the attack (and a positive one for the decay and the release,
        // which are negated on the way in) normalises to a value below one,
        // so those take the fast-start branch.
        let slow = attack_curve(0.0, 0.0, 0.0, 400);
        let fast = attack_curve(-0.5, 0.5, 0.5, 400);

        // The slow-start curve starts from a lower level, and accelerates
        // towards the target instead of jumping at it.
        assert!(
            slow[0] < fast[0],
            "slow started at {} and fast at {}",
            slow[0],
            fast[0]
        );

        // The fast-start curve reaches half of the scale much sooner.
        assert!(
            samples_to_half(&fast) < samples_to_half(&slow),
            "fast took {} samples and slow took {}",
            samples_to_half(&fast),
            samples_to_half(&slow)
        );

        // The fast-start attack is a decaying inverse exponential, so its
        // steps shrink.
        for index in 2..100 {
            assert!(
                fast[index] - fast[index - 1] < fast[index - 1] - fast[index - 2],
                "the fast-start attack did not decelerate at {index}"
            );
        }

        // The slow-start attack is a rising exponential, so its steps grow.
        for index in 2..100 {
            assert!(
                slow[index] - slow[index - 1] >= slow[index - 1] - slow[index - 2],
                "the slow-start attack did not accelerate at {index}"
            );
        }
    }

    #[test]
    fn the_slow_start_envelope_runs_through_every_stage() {
        let mut env = Envelope::new();
        env.init(SRATE, 5.0, 50.0, -30.0, 100.0, 0.0, 0.0, 0.0);
        env.attack(1.0);

        let mut stage = Stage::Attack;
        for _ in 0..10_000 {
            stage = env.process();
            if stage != Stage::Attack {
                break;
            }
        }
        assert_eq!(stage, Stage::Decay);

        for _ in 0..100_000 {
            stage = env.process();
            if stage != Stage::Decay {
                break;
            }
        }
        assert_eq!(stage, Stage::Sustain);
        assert!((env.env - 0.031_622_776_601_683_8).abs() < 1e-3);

        env.release();
        assert_eq!(env.state, Stage::Release);

        for _ in 0..100_000 {
            stage = env.process();
            if stage != Stage::Release {
                break;
            }
        }
        assert_eq!(stage, Stage::Off);
        assert_eq!(env.env, 0.0);
    }
}
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cd rust && cargo test -p rippler-dsp envelope`
Expected: 10 tests pass.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-dsp/src/envelope.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the ADSR envelope with tension shaping"
```

---

### Task 6: Comb-filter stereoizer

**Files:**
- Create: `rust/crates/rippler-dsp/src/comb.rs`

Port of `src/dsp/Comb.h`.

The C++ computes the delay length as `(int)(20 * srate / 1000)`, which is zero
for a sample rate below 50 Hz and would make the modulo operation divide by
zero. The port clamps the length to at least one sample.

- [ ] **Step 1: Declare the module**

Add `pub mod comb;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `rust/crates/rippler-dsp/src/comb.rs`:

```rust
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
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cd rust && cargo test -p rippler-dsp comb`
Expected: 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-dsp/src/comb.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the comb-filter stereoizer"
```

---

### Task 7: Limiter

**Files:**
- Create: `rust/crates/rippler-dsp/src/limiter.rs`

Port of `src/dsp/Limiter.h`, itself a port of FairlyChildish by Thomas Scott
Stillwell.

**Known quirk, preserved deliberately.** The C++ `init` computes `rmscoef`
from `rmstime` *before* assigning `rmstime`:

```cpp
rmscoef = exp(-1.0 / (rmstime * srate));
rmstime = rms_win / 1000000.0;
```

`rmstime` is therefore always zero, `rmscoef` is always zero, and the RMS
average tracks the instantaneous value rather than a smoothed window. The port
reproduces this ordering exactly, with a comment, so the sound is unchanged.
Fixing it is a separate, deliberate change.

- [ ] **Step 1: Declare the module**

Add `pub mod limiter;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `rust/crates/rippler-dsp/src/limiter.rs`:

```rust
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

    /// Creates a limiter configured with the defaults the plugin uses.
    ///
    /// The limiter is initialised with a threshold of 0 dB, a bias of 70%, an
    /// RMS window of 100 ms and no makeup gain, so it is usable straight away
    /// and cannot produce `NaN`. Call [`Limiter::init`] to reconfigure it.
    ///
    /// This is the same output as the C++ default-constructed limiter
    /// followed by a single `init` call with those parameters: `init` reads
    /// the RMS window before assigning it, so the RMS coefficient is zero on
    /// that first call, and it is zero here as well. The limiter therefore
    /// behaves identically to the original from the very first sample.
    pub fn new(srate: f64) -> Self {
        let mut limiter = Self {
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
        };

        limiter.init(srate, 0.0, 70.0, 100.0, 0.0);

        // `init` reads `rmstime` before assigning it, so the RMS coefficient
        // of this first configuration is zero. Clearing `rmstime` keeps that
        // "first configuration" property for any later `init` call, which
        // makes `new` followed by `init` behave exactly like the C++'s
        // default-constructed limiter followed by a single `init`.
        limiter.rmstime = 0.0;

        limiter
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

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 48_000.0;

    fn limiter() -> Limiter {
        Limiter::new(SRATE)
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
        let mut limiter = Limiter::new(SRATE);
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
    fn a_new_limiter_is_already_initialised() {
        // The point of the old `Default` test was that the two constructors
        // must not disagree. `Default` is gone, so the equivalent guarantee
        // is that `new` alone leaves the limiter configured and working.
        let mut loud = Limiter::new(SRATE);

        // A limiter with a zero `capsc` would leave the signal completely
        // alone, so a loud signal must still be attenuated here.
        assert!(loud.process(2.0, 2.0).0 < 2.0);

        // A fresh limiter and one that has been reconfigured with the same
        // parameters must behave identically.
        let mut fresh = Limiter::new(SRATE);
        let mut reconfigured = Limiter::new(SRATE);
        reconfigured.init(SRATE, 0.0, 70.0, 100.0, 0.0);

        for n in 0..1000 {
            let input = (n as f64 * 0.01).sin() * 2.0;
            let (left, right) = fresh.process(input, -input);
            let (reference_left, reference_right) = reconfigured.process(input, -input);

            assert_eq!(left, reference_left, "left diverged at {n}");
            assert_eq!(right, reference_right, "right diverged at {n}");
        }
    }

    #[test]
    fn a_new_limiter_does_not_produce_nan() {
        // The C++ leaves the ratio and the makeup gain at zero until `init`
        // is called, which makes its output `NaN`. The Rust constructor
        // initialises the limiter, so this cannot happen.
        let mut limiter = Limiter::new(SRATE);

        let (left, right) = limiter.process(0.5, -0.5);
        assert!(left.is_finite(), "left was {left}");
        assert!(right.is_finite(), "right was {right}");
        assert!((left - 0.5).abs() < 1e-12, "left was {left}");
        assert!((right + 0.5).abs() < 1e-12, "right was {right}");

        for n in 0..1000 {
            let input = (n as f64 * 0.01).sin() * 2.0;
            let (left, right) = limiter.process(input, -input);
            assert!(left.is_finite(), "left was {left}");
            assert!(right.is_finite(), "right was {right}");
        }
    }

    #[test]
    fn the_rms_window_has_no_effect_on_the_first_init() {
        // The original computes `rmscoef` from `rmstime` before assigning it,
        // so on a first `init` the coefficient is zero and the RMS window has
        // no effect at all. A refactor that fixes the ordering would change
        // the sound, so the quirk is pinned here.
        let mut short_window = Limiter::new(SRATE);
        let mut long_window = Limiter::new(SRATE);

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
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cd rust && cargo test -p rippler-dsp limiter`
Expected: 8 tests pass.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-dsp/src/limiter.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the brickwall limiter"
```

---

## Plan 1 completion

- [ ] **Step 1: Run the whole test suite**

Run: `cd rust && cargo test`
Expected: every test in the workspace passes.

- [ ] **Step 3: Check for warnings**

Run: `cd rust && cargo clippy --all-targets -- -D warnings`
Expected: no warnings. Fix anything it reports rather than suppressing it.

- [ ] **Step 4: Check formatting**

Run: `cd rust && cargo fmt --check`
Expected: no diff. If there is one, run `cargo fmt` and re-run the tests.

- [ ] **Step 5: Commit any formatting changes**

```bash
git add rust
git commit -m "style: format the Rust workspace"
```

- [ ] **Step 6: Verify the JUCE tree is untouched**

The JUCE tree is the reference implementation the remaining plans port from, so
it has to be intact. This plan only adds files under `rust/`, so the check is
that nothing outside `rust/` changed:

```bash
git diff --stat <commit-before-this-plan>..HEAD -- src/ libs/ resources/ CMakeLists.txt .github/
```

Expected: no output. Every change is inside `rust/`.

If an MSVC toolchain is installed, also run the existing CMake build to confirm
the plugin still compiles:

```bash
cmake -G "Visual Studio 17 2022" -DCMAKE_BUILD_TYPE=Release -S . -B ./build
cmake --build ./build --config Release
```

JUCE rejects MinGW outright, so this needs the MSVC C++ toolchain. On a machine
without it, the `git diff` check above is the verification, and the limitation
should be recorded rather than worked around.

## Hardening pass

A final whole-crate review, after the per-task reviews, produced a set of changes
that are recorded here rather than as separate plan tasks:

- The crate gained `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`.
- `LookupTable` computes its last index with `saturating_sub(1)`, so a future
  change that allows an empty table cannot underflow and panic. Its `is_empty`
  method, which could never return true, was removed in favour of
  `#[allow(clippy::len_without_is_empty)]`.
- `Limiter::new` now takes the sample rate and initialises the limiter with the
  defaults the plugin uses, so an uninitialised limiter cannot produce NaN. The
  `Default` implementation was removed, since it had no sensible sample-rate-free
  default and was the thing that made the trap reachable. `new(srate)` clears
  `rmstime` after initialising, so a later `init` still sees the "first
  configuration" the C++ sees and the two stay bit-identical.
- `Envelope` tracks whether it has been initialised, and `attack` on an
  uninitialised envelope leaves it silent instead of producing a full-scale click.
- `utils.rs` ports the `Utils` conversion helpers from `src/dsp/Utils.h`:
  `gainTodB`, `normalToFreq`, `freqToNormal` and their `f32` variants.

Create `rust/crates/rippler-dsp/src/utils.rs`:

```rust
//! Small conversion helpers.
//!
//! Port of the `Utils` class in `src/dsp/Utils.h`.

/// Conversion helpers between normalised values, frequencies and decibels.
///
/// Port of the `Utils` class in `src/dsp/Utils.h`. The helpers are stateless,
/// so they are associated functions on an otherwise empty struct.
pub struct Utils;

impl Utils {
    /// `log(20000 / 20)`, the span of the normalised frequency range.
    pub const LOG_MAX_OVER_MIN_FREQ: f64 = 6.907_755_278_982_137;

    /// Maps a normalised frequency onto a frequency in hertz.
    ///
    /// This is the C++ `Utils::normalToFreq`. A normalised value of zero is
    /// 20 Hz and one is 20 kHz.
    pub fn normal_to_freq(norm: f64) -> f64 {
        20.0 * (norm * Self::LOG_MAX_OVER_MIN_FREQ).exp()
    }

    /// Maps a frequency in hertz onto its normalised position.
    ///
    /// This is the C++ `Utils::freqToNormal`, the inverse of
    /// [`Utils::normal_to_freq`].
    pub fn freq_to_normal(norm: f64) -> f64 {
        (norm / 20.0).ln() / Self::LOG_MAX_OVER_MIN_FREQ
    }

    /// Converts a linear gain into decibels.
    ///
    /// This is the C++ `Utils::gainTodB`. A gain of zero maps to `-60` rather
    /// than to negative infinity.
    pub fn gain_to_db(gain: f64) -> f64 {
        if gain == 0.0 {
            -60.0
        } else {
            20.0 * gain.log10()
        }
    }

    /// Maps a normalised value onto a frequency between `min` and `max`.
    ///
    /// This is the C++ `Utils::normalToFreqf`, which works in single
    /// precision.
    pub fn normal_to_freqf(min: f32, max: f32, norm: f32) -> f32 {
        min * (norm * (max / min).ln()).exp()
    }

    /// Maps a frequency between `min` and `max` onto its normalised position.
    ///
    /// This is the C++ `Utils::freqToNormalf`, the inverse of
    /// [`Utils::normal_to_freqf`].
    pub fn freq_to_normalf(min: f32, max: f32, norm: f32) -> f32 {
        (norm / min).ln() / (max / min).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_constant_is_the_span_of_the_frequency_range() {
        assert_eq!(Utils::LOG_MAX_OVER_MIN_FREQ, (20_000f64 / 20.0).ln());
        assert_eq!(Utils::LOG_MAX_OVER_MIN_FREQ, 1000f64.ln());
    }

    #[test]
    fn a_normalised_frequency_maps_onto_the_audio_range() {
        assert!((Utils::normal_to_freq(0.0) - 20.0).abs() < 1e-12);
        assert!((Utils::normal_to_freq(1.0) - 20_000.0).abs() < 1e-9);
        assert!((Utils::normal_to_freq(0.5) - 632.455_532_033_675_9).abs() < 1e-9);
    }

    #[test]
    fn a_frequency_maps_onto_its_normalised_position() {
        assert!((Utils::freq_to_normal(20.0) - 0.0).abs() < 1e-12);
        assert!((Utils::freq_to_normal(20_000.0) - 1.0).abs() < 1e-12);
        assert!((Utils::freq_to_normal(1000.0) - 0.566_323_334_778_672_9).abs() < 1e-12);
    }

    #[test]
    fn the_normalised_frequency_helpers_invert_each_other() {
        for norm in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
            let frequency = Utils::normal_to_freq(norm);
            let round_trip = Utils::freq_to_normal(frequency);
            assert!(
                (round_trip - norm).abs() < 1e-12,
                "{norm} round tripped to {round_trip}"
            );
        }

        for frequency in [20.0, 100.0, 440.0, 1000.0, 10_000.0, 20_000.0] {
            let norm = Utils::freq_to_normal(frequency);
            let round_trip = Utils::normal_to_freq(norm);
            assert!(
                (round_trip - frequency).abs() < 1e-9,
                "{frequency} round tripped to {round_trip}"
            );
        }
    }

    #[test]
    fn a_gain_maps_onto_decibels() {
        assert_eq!(Utils::gain_to_db(1.0), 0.0);
        assert!((Utils::gain_to_db(0.5) + 6.020_599_913_279_624).abs() < 1e-12);
        assert!((Utils::gain_to_db(2.0) - 6.020_599_913_279_625).abs() < 1e-12);
        assert!((Utils::gain_to_db(0.1) + 20.0).abs() < 1e-12);
    }

    #[test]
    fn a_gain_of_zero_maps_onto_minus_sixty_decibels() {
        // Not negative infinity, which is what a plain logarithm would give.
        assert_eq!(Utils::gain_to_db(0.0), -60.0);
        assert!(Utils::gain_to_db(0.0).is_finite());
    }

    #[test]
    fn a_normalised_value_maps_onto_a_bounded_frequency() {
        assert!((Utils::normal_to_freqf(20.0, 20_000.0, 0.0) - 20.0).abs() < 1e-3);
        assert!((Utils::normal_to_freqf(20.0, 20_000.0, 1.0) - 20_000.0).abs() < 1.0);
        assert!((Utils::normal_to_freqf(100.0, 1000.0, 0.5) - 316.227_78).abs() < 1e-3);
    }

    #[test]
    fn a_bounded_frequency_maps_onto_its_normalised_position() {
        assert!((Utils::freq_to_normalf(20.0, 20_000.0, 20.0) - 0.0).abs() < 1e-6);
        assert!((Utils::freq_to_normalf(20.0, 20_000.0, 20_000.0) - 1.0).abs() < 1e-6);
        assert!((Utils::freq_to_normalf(100.0, 1000.0, 316.227_78) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn the_bounded_frequency_helpers_invert_each_other() {
        for norm in [0.0f32, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
            let frequency = Utils::normal_to_freqf(20.0, 20_000.0, norm);
            let round_trip = Utils::freq_to_normalf(20.0, 20_000.0, frequency);
            assert!(
                (round_trip - norm).abs() < 1e-5,
                "{norm} round tripped to {round_trip}"
            );
        }
    }
}
```

Add `pub mod utils;` to `rust/crates/rippler-dsp/src/lib.rs`.

## Decisions carried into Plan 2 (the modal core)

A final whole-crate review raised two questions that have to be answered before
the modal core is written, because getting them wrong would mean rewriting it.
Both are settled here.

### 1. `Partial` gets its own biquad, not `Filter`

`Partial::process` in `src/dsp/Partial.cpp` computes:

```cpp
auto output = ((b0 * input + b2 * x2) - (a1 * y1 + a2 * y2)) / a0;
```

with `b0 = alpha * tone_gain * amp_k`, `b2 = -b0`, `a0 = 1 + alpha / decay_k`,
`a1 = -2 * cos(omega)` and `a2 = 1 - alpha / decay_k`.

`Filter::process` in `src/dsp/Filter.cpp` computes a direct form I section:

```cpp
y0 = b0*x0 + b1*x1 + b2*x2 - a1*y1 - a2*y2;
```

The two are not the same structure. `Filter` folds the `1/a0` normalisation into
`a1` and `a2` at construction time, has a `b1` term the partial does not use,
and its coefficients are immutable apart from `copy_from`. The partial divides
by `a0` at the end of every sample, and `applyGain` mutates `b0` and `b2` in
place after construction.

The partial could be expressed through `Filter` by pre-dividing every
coefficient by `a0`, but that changes the floating-point rounding: the C++
divides once at the end, the folded form divides four times at construction.
The whole point of the differential harness is that the ports are bit-identical
to the C++ rather than merely close, so the division has to stay where the C++
puts it.

**Decision:** `partial.rs` carries its own small biquad with the C++'s exact
operation order. `Filter` stays as it is, for the places that want a general
RBJ response.

### 2. The two lookup tables are owned by the plugin and borrowed

`Partial::a1LUT` and `Partial::sinLUT` are function-local statics in the C++,
re-initialised whenever the sample rate changes. Rust has no mutable statics
that can be handed out safely, and `LookupTable` owns a 4096-entry `Vec`, so
cloning it per partial would be a large allocation on the audio path.

**Decision:** a `Luts` owner struct holding both tables is built once when the
plugin is activated, and the voice bank borrows it. Because the tables are
immutable after construction, a sample-rate change replaces the whole voice
bank rather than mutating the tables in place — which is also what the design
document's "values owned by the plugin, passed by reference" says.

`Partial` itself should stay free of lifetimes: `apply_pitch_bend` needs the
`a1` table at runtime, so the resonator threads the borrowed tables through as
an argument rather than storing them in each partial.

### 3. Smaller things Plan 2 should know

- `Filter::copy_from` copies coefficients only and leaves the history alone,
  matching the C++ `Filter::copy`. Anything that copies a filter must use it
  rather than assigning a freshly constructed filter, which would reset the
  state and diverge from the C++.
- `Utils::gainTodB` and the frequency conversions are now ported in `utils.rs`
  and are needed for the resonator's cutoff mapping.

## Notes for the next plan

- `rippler-dsp` has no dependencies yet. Plan 3 adds `symphonia` for audio
  decoding, Plan 4 adds `nice-plug-core` to `rippler-params`.
- The `Filter::magnitude_at` helper exists for the tests. Plan 3's noise filter
  tests can reuse it.
- `Envelope::init` takes its sustain level in decibels. Plan 3's noise module
  converts the plugin's linear `noise_sus` parameter with the same
  `susToDb(val) = val * 60 - 60` mapping the C++ uses.
- Cargo names the plugin's shared library `librippler_plugin.so` from the
  package name `rippler-plugin`. Plan 5 bundles the plugin, and the bundle must
  ship the conventional name `RipplerX.so` / `RipplerX.vst3` / `RipplerX.clap`,
  so the bundler configuration has to set the artifact name explicitly.
- The C++ `LookupTable` has a default constructor that leaves it empty, and
  calling it before `init` is undefined behaviour. Its only users are
  `Partial::a1LUT` and `Partial::sinLUT`, both initialised in
  `Partial::initA1LUT` from `prepareToPlay`. The Rust `LookupTable` has no such
  state, so the modal core plan must own the two tables explicitly and make
  sure they are initialised before any partial reads them.
- The C++ re-initialises both tables whenever the sample rate changes, which
  reallocates their backing storage. The Rust table is immutable after `new`,
  so the modal core plan replaces the whole table on a sample-rate change
  rather than mutating it in place.
- A `LookupTable` with `NaN` input is undefined behaviour in the C++ and
  degenerate in Rust, where `clamp` propagates the `NaN` and the index
  saturates to zero. Unreachable in practice, since the callers feed it
  computed frequencies.
