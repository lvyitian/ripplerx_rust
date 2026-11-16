# Rust Modal Core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port RipplerX's modal synthesis core to Rust — the twelve acoustic model tables, the partial, the resonator bank and the tube waveguide.

**Architecture:** Four new modules in `rippler-dsp`, plus a `Luts` owner for the two lookup tables the C++ keeps as mutable statics. Every module is a port of its C++ counterpart. The model tables become immutable data plus pure recompute functions, which removes the data race the C++ has between the message thread and the audio thread.

**Tech Stack:** Rust 2024, no new dependencies. `cargo test` for the analytic tests, plus a differential harness that compiles the C++ originals and compares output sample-for-sample.

**Design doc:** `docs/superpowers/specs/2026-11-16-rust-nice-plug-migration-design.md`
**Previous plan:** `docs/superpowers/plans/2026-11-16-rust-workspace-and-dsp-primitives.md` — read its "Decisions carried into Plan 2" section before starting. Two questions were settled there and are binding.

---

## Position in the roadmap

| # | Plan | Status |
|---|---|---|
| 1 | Rust workspace and DSP primitives | Done |
| 2 | **This plan — modal core** | Current |
| 3 | Excitation and voice | Next |
| 4 | Parameters and compatibility | |
| 5 | Plugin shell | |
| 6 | Editor | |

## What already exists

`rippler-dsp` currently holds `lut.rs`, `rng.rs`, `filter.rs`, `envelope.rs`, `comb.rs`, `limiter.rs` and `utils.rs`. Each was verified **bit-identical** to its C++ original by a differential harness. The crate lints `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`, so every public item needs a doc comment.

`Filter` provides `low_pass`, `band_pass`, `high_pass`, `copy_from` and `process`. `LookupTable` provides `new`, `get` and `cubic`.

## Reference material

- `src/dsp/Models.{h,cpp}` — the twelve ratio tables, the gain tables, and the three recompute functions
- `src/dsp/Partial.{h,cpp}` — one bandpass partial
- `src/dsp/Resonator.{h,cpp}` — the 64-partial bank
- `src/dsp/Waveguide.{h,cpp}` — the tube waveguide
- `src/Globals.h` — `MAX_PARTIALS` is 64

Do not modify any of these.

---

## Findings that shape this plan

### 1. The two tube tables are swapped relative to the enum

`ModalModels` in `src/dsp/Models.h` lists `OpenTube` at index 7 and `ClosedTube` at index 8. The `aModels` array in `src/dsp/Models.cpp` fills index 7 with the **closed** tube harmonics (`1.0, 2.0, 3.0, … 64.0`) and index 8 with the **open** tube harmonics (`1, 3, 5, … 127`).

So `aModels[OpenTube]` holds the closed tube's table and `aModels[ClosedTube]` holds the open tube's. Every other model lines up.

The effect is limited. For the tube models the resonator only reads `model[0]`, which is `1.0` in both tables, so the waveguide itself is unaffected. The difference shows up in `Voice::calcFrequencyShifts`, which walks all 64 entries when the resonators are coupled in series — so serial coupling on a tube model shifts by a different amount than the model name suggests.

**Decision: reproduce the swap exactly**, with a comment at both the enum and the table explaining it. The port's standard is bit-identical output, and changing this would alter the sound for tube models with serial coupling. It is recorded here so it can be changed deliberately later, separately from the port.

### 2. `Partial` carries its own biquad

`Partial::process` divides by `a0` at the end of every sample; `Filter` folds `1/a0` into its coefficients at construction. Pre-dividing would change the rounding, so `partial.rs` keeps the C++'s operation order and does not use `Filter`.

### 3. The lookup tables are owned, not static

`Partial::a1LUT` and `Partial::sinLUT` are mutable statics in the C++. `Luts` owns both tables, built once per sample rate. The voice bank borrows them.

### 4. Model tables are data, not mutable state

`Models::recalcBeam` and friends mutate a table in place, on the message thread, while the audio thread reads it through `Voice::updateResonators`. That is a genuine data race.

In Rust the tables are immutable. `beam(ratio)`, `membrane(ratio)` and `plate(ratio)` are pure functions that return a new table, and the plugin replaces the affected table on the audio thread when the ratio parameter changes. The result is the same sound with no race.

---

## Conventions

- Internal arithmetic is `f64`, matching the C++.
- **Use `.powf(n)` for every C++ `pow(x, n)` call, never `.powi(n)`.** `powi` multiplies repeatedly and can differ from libm's `pow` in the last bit, which would break the bit-exactness the harness checks.
- The C++ clamps with `fmax(lo, fmin(hi, x))`, which returns the non-NaN operand. Rust's `.clamp(lo, hi)` propagates a NaN. They agree for every value the plugin can produce, but if the differential harness reports a difference, this is the first thing to check.
- Every module carries a doc comment naming the C++ file it ports.
- Tests live in a `#[cfg(test)] mod tests` block at the bottom of the module file.
- The crate forbids `unsafe` and denies `missing_docs`, so every public item needs a doc comment.

### Extracting the numeric tables

`Models.cpp` contains 64 + 12×64 + 3×64 = 320 literal doubles. **Do not retype them.** Extract them mechanically from `src/dsp/Models.cpp` with a script that parses the C++ initialiser lists, and emit Rust `const` arrays. Then verify that every extracted value matches the C++ exactly, and that the array lengths are right.

A transcription error in one of these numbers would silently detune one partial of one model, which no amount of listening would reliably catch.

---

### Task 1: The modal models

**Files:**
- Create: `rust/crates/rippler-dsp/src/models.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod models;`)

- [ ] **Step 1: Declare the module**

Add `pub mod models;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/models.rs`. Its structure:

```rust
//! The acoustic models: frequency ratio tables and gain tables.
//!
//! Port of `src/dsp/Models.{h,cpp}`.
//!
//! The twelve models are described by a table of 64 frequency ratios each,
//! plus a table of 64 gains for the models that were measured rather than
//! derived.

/// The number of partials in every modal model.
pub const MAX_PARTIALS: usize = 64;

/// The frequency ratios of one modal model, one entry per partial.
pub type Ratios = [f64; MAX_PARTIALS];

/// The acoustic models a resonator can be tuned to.
///
/// The discriminants match the `a_model` and `b_model` parameters' choice
/// order, so a variant's discriminant is the value the plugin stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ModalModel {
    /// A stretched string. The partials sit at whole multiples of the base.
    String = 0,
    /// A free-free beam.
    Beam = 1,
    /// A string squared, for inharmonic strings.
    Squared = 2,
    /// A circular membrane.
    Membrane = 3,
    /// A rectangular plate.
    Plate = 4,
    /// A circular drumhead.
    Drumhead = 5,
    /// A marimba bar, measured from a Chromaphone sample.
    Marimba = 6,
    /// An open tube. See the note on the table order below.
    OpenTube = 7,
    /// A closed tube. See the note on the table order below.
    ClosedTube = 8,
    /// A marimba bar, from the Faust modal model library.
    Marimba2 = 9,
    /// A Russian bell, from the Faust modal model library.
    Bell = 10,
    /// A djembe, modelled as a series of partials 200 Hz apart.
    Djembe = 11,
}
```

Then, in the same file:

- `const BFREE: [f64; MAX_PARTIALS]` — the 64 values from the `bFree` initialiser, extracted mechanically.
- `const MODEL_RATIOS: [Ratios; 12]` — the twelve tables, extracted mechanically.
- `const MODEL_GAINS: [Ratios; 4]` — the four gain rows, extracted mechanically. Note that the C++ declares four rows but initialises three, so the fourth is all zeros in the C++ too; reproduce that.
- `const fn model_index(model: ModalModel) -> usize` — the discriminant as an index.

Add these functions:

```rust
/// The frequency ratios of a model, as shipped.
pub fn ratios(model: ModalModel) -> &'static Ratios {
    &MODEL_RATIOS[model_index(model)]
}

/// The gains of a model, as shipped.
///
/// Only the two models that were measured have gains of their own; every
/// other model uses a flat gain table.
pub fn gains(model: ModalModel) -> &'static Ratios {
    match model {
        ModalModel::Marimba2 => &MODEL_GAINS[1],
        ModalModel::Bell => &MODEL_GAINS[2],
        _ => &MODEL_GAINS[0],
    }
}

/// Divides a table by its first entry, turning frequencies into ratios.
fn normalise(table: &mut Ratios) {
    let f0 = table[0];
    for value in table.iter_mut() {
        *value /= f0;
    }
}
```

And the three recompute functions, which are pure in Rust where they were mutating in the C++:

```rust
/// Recomputes the beam model's ratios for a given ratio parameter.
pub fn beam(ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    let mut i = 0;
    for m in 1..=8 {
        for _n in 1..=8 {
            table[i] = ((m as f64).powf(4.0) + (ratio * BFREE[i]).powf(4.0)).sqrt();
            i += 1;
        }
    }
    normalise(&mut table);
    table
}

/// Recomputes the membrane model's ratios for a given ratio parameter.
pub fn membrane(ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    let mut i = 0;
    for m in 1..=8 {
        for n in 1..=8 {
            table[i] = ((m as f64).powf(2.0) + (ratio * n as f64).powf(2.0)).sqrt();
            i += 1;
        }
    }
    normalise(&mut table);
    table
}

/// Recomputes the plate model's ratios for a given ratio parameter.
pub fn plate(ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    let mut i = 0;
    for m in 1..=8 {
        for n in 1..=8 {
            table[i] = (m as f64).powf(2.0) + (ratio * n as f64).powf(2.0);
            i += 1;
        }
    }
    normalise(&mut table);
    table
}

/// Computes the djembe model's ratios for a given fundamental frequency.
pub fn djembe(freq: f64, ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    for (i, value) in table.iter_mut().enumerate() {
        *value = 1.0 + (200.0 * i as f64 * ratio) / freq;
    }
    table
}
```

Note that `beam` uses `BFREE[i]`, indexed by the flat loop counter, while `membrane` and `plate` use `n`, the inner loop variable. That asymmetry is in the C++ and must be preserved.

Add a comment on `MODEL_RATIOS` recording that index 7 holds the closed tube's harmonics and index 8 the open tube's, that this is swapped relative to the enum, that the effect is limited to the serial coupling frequency shift, and that it is reproduced deliberately.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `the_model_discriminants_match_the_choice_order` — asserting each variant's discriminant is 0 through 11 in the enum's order, since the plugin stores the choice index.
- `every_model_has_a_full_table` — asserting `ratios(model)` has 64 entries and that `ratios(model)[0] == 1.0` for every model.
- `the_shipped_tables_match_the_cpp_values` — asserting a handful of exact values from each table, taken from `src/dsp/Models.cpp`, including the first and last entry of each. Every one of these values must be copied from the C++ file, not from memory.
- `the_beam_recalculation_uses_the_free_beam_table` — asserting `beam(ratio)[0] == 1.0` (it is normalised) and that `beam(ratio)` differs from `ratios(ModalModel::Beam)` for a ratio other than the shipped one.
- `the_membrane_and_plate_recalculations_differ_in_shape` — asserting that `membrane(0.78)` and `plate(0.78)` differ, since the membrane takes a square root and the plate does not.
- `the_djembe_ratios_are_spaced_by_the_ratio` — asserting `djembe(200.0, 1.0)[0] == 1.0` and that consecutive entries differ by `200.0 * ratio / freq`.
- `only_two_models_have_gains_of_their_own` — asserting `gains(ModalModel::Marimba2)` differs from `gains(ModalModel::String)` and that `gains(ModalModel::Bell)` differs from both, while `gains(ModalModel::String)` is all ones.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp models`
Expected: all tests pass.

- [ ] **Step 5: Verify the extracted tables against the C++**

Write a check that parses `src/dsp/Models.cpp` and compares every one of the 64 + 12×64 + 3×64 values against the Rust constants, and reports any difference. This is the check that catches a transcription error, so do not skip it.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/rippler-dsp/src/models.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the modal models and their ratio tables"
```

---

### Task 2: The lookup table owner

**Files:**
- Create: `rust/crates/rippler-dsp/src/luts.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod luts;`)

The C++ keeps `Partial::a1LUT` and `Partial::sinLUT` as mutable statics, re-initialised whenever the sample rate changes. Rust has no safely shareable mutable statics, and `LookupTable` owns a 4096-entry `Vec`, so cloning one per partial would be a 32 KB allocation on the audio path.

`Luts` owns both tables and is built once per sample rate.

- [ ] **Step 1: Declare the module**

Add `pub mod luts;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/luts.rs`:

```rust
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
```

- [ ] **Step 3: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp luts`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-dsp/src/luts.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): own the lookup tables the partials read from"
```

---

### Task 3: The partial

**Files:**
- Create: `rust/crates/rippler-dsp/src/partial.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod partial;`)

Port of `src/dsp/Partial.{h,cpp}`.

- [ ] **Step 1: Declare the module**

Add `pub mod partial;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/partial.rs`. It holds a partial's parameters, its biquad coefficients and its four-sample history.

The biquad is the C++'s own, not `Filter`'s:

```rust
/// A single mode of a resonator.
///
/// Port of `src/dsp/Partial.{h,cpp}`. This is a second-order bandpass with
/// its own coefficients, rather than a `Filter`, because the C++ divides by
/// `a0` at the end of every sample and folding that normalisation into the
/// coefficients would change the rounding.
#[derive(Clone, Debug)]
pub struct Partial {
    /// The partial number, counting from one.
    pub k: usize,
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
```

With these methods:

- `Partial::new(k: usize) -> Self`, initialising every field to the C++'s default: `srate = 0.0`, `decay = 0.0`, `damp = 0.0`, `tone = 0.0`, `hit = 0.0`, `rel = 0.0`, `inharm = 0.0`, the five velocity fields `0.0`, `f_k = 1000.0`, `out_of_range = false`, `base_f_k = 1000.0`, `b0 = 0.0`, `b2 = 0.0`, `a0 = 1.0`, `a1 = 0.0`, `a2 = 0.0`, and the four history values `0.0`.

- `Partial::update(&mut self, freq: f64, ratio: f64, ratio_max: f64, vel: f64, pitch_bend: f64, is_release: bool)`, ported from `Partial::update` with the C++'s exact expression order:

```rust
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
        let mut inharm_k = (self.inharm.ln() + vel * self.vel_inharm * -0.0001f64.ln()).exp()
            - 0.0001;
        inharm_k = inharm_k.clamp(0.0, 1.0);
        inharm_k = (1.0 + inharm_k * (ratio - 1.0) * (ratio - 1.0)).sqrt();

        self.f_k = freq * ratio * inharm_k;
        self.base_f_k = self.f_k;
        self.f_k *= pitch_bend;

        // Normalise the velocity contribution on a logarithmic scale.
        let mut decay_k = (self.decay.ln() + vel * self.vel_decay * (100.0f64.ln() - 0.01f64.ln()))
            .exp()
            .clamp(0.01, 100.0);
        if is_release {
            decay_k *= self.rel;
        }

        if self.f_k >= 20000.0 || self.f_k < 1.0 || decay_k == 0.0 {
            self.out_of_range = true;
        }

        let f_max = (20000.0f64).min(freq * ratio_max * inharm_k);
        let omega = std::f64::consts::TAU * self.f_k / self.srate;
        let alpha = std::f64::consts::TAU / self.srate;

        let damp_base = (self.damp + self.vel_damp * 2.0 * vel).clamp(-1.0, 1.0);
        let damp_k = if damp_base <= 0.0 {
            (freq / self.f_k).powf(damp_base * 2.0)
        } else {
            (f_max / self.f_k).powf(damp_base * 2.0)
        };

        decay_k /= damp_k;

        let tone_base = (self.tone + self.vel_tone * 2.0 * vel).clamp(-1.0, 1.0);
        let tone_gain = if tone_base <= 0.0 {
            (self.f_k / freq).powf(tone_base * 12.0 / 6.0)
        } else {
            (self.f_k / f_max).powf(tone_base * 12.0 / 6.0)
        };

        let mut amp_k = (std::f64::consts::PI
            * self.k as f64
            * (self.hit + self.vel_hit * vel / 2.0).clamp(0.02, 0.5))
            .sin()
            .abs();
        amp_k *= 35.0;

        // Bandpass filter coefficients.
        self.b0 = alpha * tone_gain * amp_k;
        self.b2 = -alpha * tone_gain * amp_k;
        self.a0 = if decay_k != 0.0 { 1.0 + alpha / decay_k } else { 0.0 };
        self.a1 = -2.0 * omega.cos();
        self.a2 = if decay_k != 0.0 { 1.0 - alpha / decay_k } else { 0.0 };
    }
```

- `Partial::process(&mut self, input: f64) -> f64`, ported with the C++'s order:

```rust
    pub fn process(&mut self, input: f64) -> f64 {
        if self.out_of_range {
            return 0.0;
        }
        let output =
            ((self.b0 * input + self.b2 * self.x2) - (self.a1 * self.y1 + self.a2 * self.y2))
                / self.a0;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
```

- `Partial::clear(&mut self)` — zeroes the four history values.
- `Partial::apply_gain(&mut self, gain: f64)` — multiplies `b0` and `b2` by `gain`.
- `Partial::apply_pitch_bend(&mut self, bend: f64, luts: &Luts)`:

```rust
    pub fn apply_pitch_bend(&mut self, bend: f64, luts: &Luts) {
        self.out_of_range = false;
        self.f_k = self.base_f_k * bend;
        if self.f_k < 1.0 || self.f_k > 20000.0 {
            self.out_of_range = true;
            return;
        }
        self.a1 = luts.a1.get(self.f_k);
    }
```

Note that the C++ reads `a1LUT(f_k)`, which is the *linear* lookup, not the cubic one. `LookupTable::get` is the linear one.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `a_new_partial_has_the_cpp_defaults` — asserting the initial values listed above.
- `the_partial_is_out_of_range_below_one_hertz` — asserting `out_of_range` after an update with a low frequency.
- `the_partial_is_out_of_range_above_twenty_kilohertz` — the same at the top end.
- `an_out_of_range_partial_produces_no_output` — asserting `process` returns `0.0`.
- `the_inharmonicity_raises_the_partial_frequency` — asserting that with a non-zero inharmonicity the tuned frequency is above `freq * ratio`, and that it follows `sqrt(1 + inharm_k * (ratio - 1)^2)`.
- `the_strike_position_nulls_the_partial` — asserting that a hit position at a half period drives the amplitude towards zero.
- `applying_a_gain_scales_the_output` — asserting that a gain of 2 doubles the output of a partial, compared against the same partial without the gain.
- `a_pitch_bend_retunes_the_partial` — asserting that after `apply_pitch_bend` the partial's frequency is the base frequency times the bend.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp partial`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-dsp/src/partial.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the bandpass partial"
```

---

### Task 4: The tube waveguide

**Files:**
- Create: `rust/crates/rippler-dsp/src/waveguide.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod waveguide;`)

Port of `src/dsp/Waveguide.{h,cpp}`.

- [ ] **Step 1: Declare the module**

Add `pub mod waveguide;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/waveguide.rs`:

```rust
//! The delay line behind the tube models.
//!
//! Port of `src/dsp/Waveguide.{h,cpp}`.

/// The length of the tube, in samples.
///
/// The C++ uses 20000, which allows for 10 Hz at a 200 kHz sample rate.
const TUBE_LEN: usize = 20000;

/// A waveguide delay line, used by the open and closed tube models.
#[derive(Clone, Debug)]
pub struct Waveguide {
    /// The tube's base frequency.
    pub base_freq: f64,
    /// Whether the tube is closed, which halves its length and inverts it.
    pub is_closed: bool,
    /// The sample rate.
    pub srate: f64,
    /// The decay time.
    pub decay: f64,
    /// The tube radius, which controls the damping.
    pub radius: f64,
    /// The release multiplier.
    pub rel: f64,
    /// How much velocity shifts the decay.
    pub vel_decay: f64,
    /// The tube's current frequency.
    pub f_k: f64,

    read_ptr_frac: f64,
    write_ptr: usize,
    tube_decay: f64,
    tube: Box<[f64]>,
    y: f64,
    y1: f64,
}

impl Waveguide {
    /// Creates a waveguide with an empty tube.
    pub fn new() -> Self {
        Self {
            base_freq: 1000.0,
            is_closed: false,
            srate: 0.0,
            decay: 0.0,
            radius: 0.0,
            rel: 0.0,
            vel_decay: 0.0,
            f_k: 0.0,
            read_ptr_frac: 0.0,
            write_ptr: 0,
            tube_decay: 0.0,
            tube: Box::new([0.0; TUBE_LEN]),
            y: 0.0,
            y1: 0.0,
        }
    }
```

with `update`, `apply_pitch_bend`, `process` and `clear` ported from the C++.

**Two porting hazards to handle:**

The C++ computes `read_ptr_frac = write_ptr - tlen` and adds `tube_len` once if it is negative. If `tlen` exceeds `tube_len` — which happens when the frequency is low enough that `srate / f_k` is longer than the tube — the result is still negative, and the C++ then reads `tube[(int)read_ptr_frac]` with a negative index, which is undefined behaviour.

In Rust, wrap the index instead of reading out of bounds. Use `rem_euclid` so a negative position maps to the right place in the tube. Note this as a deliberate difference from the C++'s undefined behaviour, and say so in a comment.

The `i1` index is `(i0 + 1) % tube_len` in the C++, which is the same as wrapping.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `a_new_waveguide_has_the_cpp_defaults` — asserting the initial values.
- `an_impulse_returns_after_the_delay` — asserting that a single impulse comes back after `srate / f_k` samples, with the right sign for a closed tube.
- `a_closed_tube_inverts_the_returned_signal` — asserting the sign difference between an open and a closed tube.
- `the_radius_damps_the_high_frequencies` — asserting that a larger radius gives a smaller output for a high-frequency input.
- `the_waveguide_survives_a_frequency_below_the_tube_length` — asserting that a `f_k` low enough to make `tlen` exceed the tube length still produces finite output rather than panicking. This is the case the C++ gets wrong.
- `clearing_resets_the_tube` — asserting the output goes to silence after `clear`.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp waveguide`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-dsp/src/waveguide.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the tube waveguide"
```

---

### Task 5: The resonator

**Files:**
- Create: `rust/crates/rippler-dsp/src/resonator.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod resonator;`)

Port of `src/dsp/Resonator.{h,cpp}`.

The C++'s `Resonator::setParams` takes sixteen arguments, which is too many to port as-is. Introduce a settings struct:

```rust
/// The parameters that configure a resonator.
#[derive(Clone, Copy, Debug)]
pub struct ResonatorSettings {
    /// Whether the resonator is sounding.
    pub on: bool,
    /// The model the resonator plays.
    pub model: ModalModel,
    /// How many partials the resonator uses.
    pub partials: usize,
    /// The base decay time in seconds.
    pub decay: f64,
    /// The damping, from -1 to 1.
    pub damp: f64,
    /// The tone, from -1 to 1.
    pub tone: f64,
    /// The strike position.
    pub hit: f64,
    /// The release multiplier.
    pub rel: f64,
    /// The inharmonicity.
    pub inharm: f64,
    /// The cutoff, from -1 to 1.
    pub cut: f64,
    /// The tube radius.
    pub radius: f64,
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
}
```

- [ ] **Step 1: Declare the module**

Add `pub mod resonator;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/resonator.rs` with:

- `pub struct Resonator` holding `partials: Vec<Partial>` (64 of them, built with `Partial::new(i + 1)`), a `Waveguide`, a `Filter`, and the C++'s public state: `silence`, `active`, `srate`, `on`, `nmodel`, `npartials`, `decay`, `radius`, `cut`.

- `Resonator::new() -> Self`, building the 64 partials.

- `Resonator::set_params(&mut self, srate: f64, settings: &ResonatorSettings)`, ported from `Resonator::setParams`. The cutoff mapping is:

```rust
        // Map 1..0 to 20..20000, with an inverse scale for a negative value.
        let freq = 20.0 * (20000.0f64 / 20.0).powf(if settings.cut < 0.0 { 1.0 + settings.cut } else { settings.cut });
        if settings.cut < 0.0 {
            self.filter = Filter::low_pass(srate, freq, 0.707);
        } else {
            self.filter = Filter::high_pass(srate, freq, 0.707);
        }
```

Then the per-partial parameter assignment and the waveguide settings, ported exactly.

- `Resonator::update(&mut self, freq: f64, vel: f64, is_release: bool, pitch_bend: f64, model: &Ratios, model_gain: &Ratios)`, ported from `Resonator::update`. The C++ takes the two 64-arrays by value; take them by reference instead, which is the same data.

- `Resonator::apply_pitch_bend(&mut self, bend: f64, luts: &Luts)`, ported.

- `Resonator::activate(&mut self)`, `Resonator::clear(&mut self)`, and `Resonator::process(&mut self, input: f64) -> f64`, ported.

Note the silence counter in `process`: the C++ compares an `int` counter against a `double` sample rate, so compare `(self.silence as f64) >= self.srate`.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `a_new_resonator_has_sixty_four_partials` — asserting the count and that the partials are numbered from one.
- `the_tube_models_use_the_waveguide` — asserting that a resonator playing an open tube produces output from the waveguide rather than from the partials.
- `the_partial_count_limits_the_output` — asserting that a resonator with four partials produces a different output from one with sixty-four.
- `an_inactive_resonator_produces_no_output` — asserting silence before `activate`.
- `the_resonator_deactivates_after_a_second_of_silence` — asserting that `active` goes false after roughly `srate` samples of silence.
- `the_cutoff_selects_a_low_pass_or_a_high_pass` — asserting that a negative cutoff gives a low-pass response and a positive one a high-pass response, by driving a low-frequency and a high-frequency signal and comparing the outputs.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp resonator`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-dsp/src/resonator.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the resonator bank"
```

---

### Task 6: Completion gates

- [ ] **Step 1: Run the whole test suite**

Run: `cd rust && cargo test`
Expected: every test passes.

- [ ] **Step 2: Check for warnings**

Run: `cd rust && cargo clippy --all-targets -- -D warnings`
Expected: no warnings. Fix them rather than suppressing them.

- [ ] **Step 3: Check formatting**

Run: `cd rust && cargo fmt --check`
Expected: no diff.

- [ ] **Step 4: Build the differential harness for the modal core**

The harness at `F:\rippler-harness\` already compiles the C++ primitives with g++ and compares them against the Rust modules. Extend it to cover the modal core:

- Compile `src/dsp/Models.cpp`, `src/dsp/Partial.cpp`, `src/dsp/Resonator.cpp` and `src/dsp/Waveguide.cpp` together with `src/dsp/Filter.cpp` and `src/dsp/Utils.h`, substituting `2.0 * M_PI` for `juce::MathConstants<double>::twoPi` as the existing harness already does, and including `src/Globals.h` for `MAX_PARTIALS`.
- Drive the C++ `Models::recalcBeam`, `recalcMembrane`, `recalcPlate` and `calcDjembe` and the Rust `beam`, `membrane`, `plate` and `djembe` with the same inputs and compare all 64 entries of each.
- Drive the C++ `Partial::update` and `process` and the Rust equivalents with the same parameter sets and inputs, and compare every output sample.
- Drive the C++ `Waveguide` and the Rust `Waveguide` the same way.
- Drive the C++ `Resonator` and the Rust `Resonator` the same way, including the tube models.

Report the **maximum absolute difference** for each. The expectation is exactly `0.0`, as it was for the six primitives in Plan 1.

If a comparison is not exactly zero, do not adjust the Rust to match. Find the difference, and report it.

- [ ] **Step 5: Commit any formatting changes**

```bash
git add rust
git commit -m "style: format the Rust workspace"
```

- [ ] **Step 6: Verify the JUCE tree is untouched**

```bash
git diff --stat <commit-before-this-plan>..HEAD -- src/ libs/ resources/ CMakeLists.txt .github/
```

Expected: no output.

## Notes for the next plan

- `rippler-dsp` still has no dependencies. Plan 3 adds `symphonia` for audio decoding, Plan 4 adds `nice-plug-core` to `rippler-params`.
- `Models::recalcBeam` and friends are now pure functions. The plugin holds the current table for each resonator and replaces it on the audio thread when the ratio parameter changes, which removes the C++'s data race. Plan 3's voice code, which reads the tables, should take them as an argument rather than holding a reference.
- `Voice::updateResonators` copies the model's ratio table and gain table before use, because it applies a pitch factor and a coupling shift to the copy. Plan 3 must keep that copy semantics: the tables are shared, and the per-voice values are derived.
- `Voice::calcFrequencyShifts` is O(64 × 64) per update. It is not on the per-sample path, so this is not urgent, but it is worth knowing.
