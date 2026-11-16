# Rust Parameters and Compatibility — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Define RipplerX's seventy-seven parameters with their legacy identifiers, reproduce JUCE's parameter ranges exactly, and build the reader that makes old patches loadable.

**Architecture:** A new `rippler-params` crate holding the parameter definitions, the range mapping, the legacy XML reader and the patch converter. The parameter identifiers are the legacy ones, verbatim, so a saved patch from the JUCE build means the same thing in the Rust build.

**Tech Stack:** Rust 2024. Adds `nice-plug-core` for the parameter types and `quick-xml` for the legacy reader.

**Design doc:** `docs/superpowers/specs/2026-11-16-rust-nice-plug-migration-design.md`

---

## Position in the roadmap

| # | Plan | Status |
|---|---|---|
| 1 | Rust workspace and DSP primitives | Done |
| 2 | Modal core | Done |
| 3 | Excitation and voice | Done |
| 4 | **This plan — parameters and compatibility** | Current |
| 5 | Plugin shell | Next |
| 6 | Editor | |

## What already exists

`rippler-dsp` holds sixteen modules, all verified **bit-identical** to their C++ originals. Two of its types matter here:

- `rippler_dsp::models::ModalModel` — the twelve acoustic models, with discriminants 0–11 matching the plugin's `a_model` and `b_model` choice order.
- `rippler_dsp::mallet::MalletType` — the twenty-six mallet types, with discriminants 0–25 matching the `mallet_type` choice order. It has `from_index(u8) -> Option<MalletType>` and `MalletType::ALL`.

---

## Findings that shape this plan

### 1. The legacy state stores plain values, not normalised ones

JUCE's parameter adapter writes `unnormalisedValue` into the `ValueTree`
(`juce_AudioProcessorValueTreeState.cpp`, `flushToTree`). So a saved float
parameter holds its **plain** value, a choice parameter holds its **index**, and
a boolean holds `0.0` or `1.0`.

This matters: the legacy reader does **not** apply the skew. It reads the plain
value and sets it directly.

The skew still matters for the *feel* — the host's generic editor and the
automation curve — and that is preserved by the range mapping below.

### 2. The skew mapping is exact

JUCE's `NormalisableRange` with a positive skew `s` normalises as `p^(1/(1+s))`.
nice-plug's `FloatRange::Skewed` normalises as `p^factor`. So `factor = 1/(1+s)`
reproduces JUCE's curve bit for bit.

Every skewed parameter in RipplerX has a **positive** skew — `0.2`, `0.3`, `0.5`
and `1.0` — so all of them map exactly. The unskewed ones map to
`FloatRange::Linear`.

JUCE's `interval` maps to nice-plug's `with_step_size`.

### 3. The legacy state omits parameters and values

Twenty-two of the seventy-seven parameters appear in **no** factory preset —
they were added to the plugin after the presets were made. A reader must leave
those at their defaults.

Four parameters (`a_coarse`, `a_fine`, `b_coarse`, `b_fine`) appear in some
presets **without** a `value` attribute. JUCE's `replaceState` reads a missing
property as `var()`, which converts to `0.0f`. The reader must do the same
rather than treating it as "keep the default", because the two differ for a
parameter whose default is not zero.

### 4. The `a_cut` and `b_cut` parameters need the historical migration

In the presets, `a_cut` and `b_cut` hold values in the **old** hertz scale —
20, 58, 144, 372, 1662 and so on. The current plugin's range is `-1.0` to
`1.0`, where the value is a position on a logarithmic scale from 20 Hz to
20 kHz.

The JUCE build migrates on load:

```cpp
if (val >= 20.0f) {
    val = std::log(val / 20.0f) / std::log(20000.0f / 20.0f);
}
```

Every value in every factory preset needs this migration. The reader must
reproduce it exactly.

### 5. The `Enum` trait and the orphan rule

nice-plug's `EnumParam<T>` needs `T` to implement
`nice_plug_core::params::enums::Enum`, and that trait needs `variants()`,
`ids()`, `to_index()` and `from_index()`.

`ModalModel` and `MalletType` live in `rippler-dsp`, so `rippler-params` cannot
implement a foreign trait for them — Rust's orphan rule forbids it.

**Decision:** `rippler-dsp` gains an **optional** dependency on `nice-plug-core`
behind a `params` feature, and implements `Enum` for its two enums there.
`rippler-params` enables `rippler-dsp/params`. The DSP crate stays free of any
framework in its default build, and the parameter layer gets named choice
parameters in the host.

`PartialCount`, `NoiseFilterMode` and `Coupling` are new types that exist only
for the parameter layer, so they live in `rippler-params` and implement `Enum`
there, where the orphan rule allows it.

### 6. The user sample uses JUCE's base64, not standard base64

Already established: JUCE's `MemoryBlock::toBase64Encoding` writes the byte
count, a dot, then the payload, using an alphabet whose index 0 is `.`, and
packs the bits LSB-first. `rippler-dsp`'s `Sampler::load_encoded` already
reproduces it, and the legacy reader reuses it.

---

## The seventy-seven parameters

Grouped as they will be nested. The identifiers are the legacy ones, verbatim.

| Group | Parameters |
|---|---|
| Mallet (7) | `mallet_type`, `mallet_pitch`, `mallet_filter`, `mallet_mix`, `mallet_res`, `mallet_stiff`, `mallet_ktrack` |
| Noise (13) | `noise_osc`, `noise_mix`, `noise_res`, `noise_filter_mode`, `noise_filter_freq`, `noise_filter_q`, `noise_att`, `noise_dec`, `noise_sus`, `noise_rel`, `noise_att_ten`, `noise_dec_ten`, `noise_rel_ten` |
| Resonator A (14) | `a_on`, `a_model`, `a_partials`, `a_decay`, `a_damp`, `a_tone`, `a_hit`, `a_rel`, `a_inharm`, `a_ratio`, `a_cut`, `a_radius`, `a_coarse`, `a_fine` |
| Resonator B (14) | `b_on`, `b_model`, `b_partials`, `b_decay`, `b_damp`, `b_tone`, `b_hit`, `b_rel`, `b_inharm`, `b_ratio`, `b_cut`, `b_radius`, `b_coarse`, `b_fine` |
| Velocity (21) | `vel_mallet_mix`, `vel_mallet_res`, `vel_mallet_stiff`, `vel_noise_mix`, `vel_noise_res`, `vel_noise_freq`, `vel_noise_att`, `vel_noise_dec`, `vel_noise_sus`, `vel_noise_rel`, `vel_noise_q`, `vel_a_decay`, `vel_a_hit`, `vel_a_inharm`, `vel_a_damp`, `vel_a_tone`, `vel_b_decay`, `vel_b_hit`, `vel_b_inharm`, `vel_b_damp`, `vel_b_tone` |
| Top level (8) | `couple`, `ab_mix`, `ab_split`, `gain`, `bend_range`, `stereoizer`, `reuse_voices`, `fadeout_repeats` |

### Ranges

Taken from `src/PluginProcessor.cpp`. `skew` is JUCE's `NormalisableRange` skew.

| Parameter | Type | Range | Default | Skew | Step |
|---|---|---|---|---|---|
| `mallet_type` | choice | 26 | Impulse | | |
| `mallet_pitch` | float | -24 … 24 | 0 | | |
| `mallet_filter` | float | -1 … 1 | 0 | | |
| `mallet_mix` | float | 0 … 1 | 0 | | |
| `mallet_res` | float | 0 … 1 | 0.8 | | |
| `mallet_stiff` | float | 100 … 5000 | 600 | 0.3 | 1 |
| `mallet_ktrack` | float | 0 … 1 | 0 | | |
| `noise_osc` | float | 0 … 1 | 0 | 1.0 | 0.0001 |
| `noise_mix` | float | 0 … 1 | 0 | 0.3 | 0.0001 |
| `noise_res` | float | 0 … 1 | 0 | 0.3 | 0.0001 |
| `noise_filter_mode` | choice | 3 | HP (2) | | |
| `noise_filter_freq` | float | 20 … 20000 | 20 | 0.3 | 1 |
| `noise_filter_q` | float | 0.707 … 4 | 0.707 | | |
| `noise_att` | float | 1 … 20000 | 1 | 0.3 | 1 |
| `noise_dec` | float | 1 … 20000 | 500 | 0.3 | 1 |
| `noise_sus` | float | 0 … 1 | 0 | | |
| `noise_rel` | float | 1 … 20000 | 500 | 0.3 | 1 |
| `noise_att_ten` | float | -1 … 1 | 0.4 | | |
| `noise_dec_ten` | float | -1 … 1 | 0.4 | | |
| `noise_rel_ten` | float | -1 … 1 | 0.4 | | |
| `a_on` / `b_on` | bool | | true / false | | |
| `a_model` / `b_model` | choice | 12 | String | | |
| `a_partials` / `b_partials` | choice | 7 | 32 partials | | |
| `a_decay` / `b_decay` | float | 0.01 … 100 | 1 | 0.2 | 0.01 |
| `a_damp` / `b_damp` | float | -1 … 1 | 0 | | |
| `a_tone` / `b_tone` | float | -1 … 1 | 0 | | |
| `a_hit` / `b_hit` | float | 0.02 … 0.5 | 0.26 | | |
| `a_rel` / `b_rel` | float | 0.001 … 1 | 1 | | |
| `a_inharm` / `b_inharm` | float | 0.0001 … 1 | 0.0001 | 0.3 | 0.0001 |
| `a_ratio` / `b_ratio` | float | 0.1 … 10 | 1 | 0.3 | 0.01 |
| `a_cut` / `b_cut` | float | -1 … 1 | 0 | | |
| `a_radius` / `b_radius` | float | 0 … 1 | 0.5 | | |
| `a_coarse` / `b_coarse` | float | -48 … 48 | 0 | | |
| `a_fine` / `b_fine` | float | -99 … 99 | 0 | | |
| `vel_*` (21) | float | -1 … 1 | 0 | | |
| `couple` | choice | 2 | A+B | | |
| `ab_mix` | float | 0 … 1 | 0.5 | | |
| `ab_split` | float | 0.01 … 1 | 0.01 | 0.5 | 0.001 |
| `gain` | float | -24 … 24 | 0 | | |
| `bend_range` | int | 1 … 24 | 2 | | |
| `stereoizer` | bool | | true | | |
| `reuse_voices` | bool | | false | | |
| `fadeout_repeats` | bool | | false | | |

---

## Conventions

- Use `.powf(n)`, never `.powi(n)`.
- The crate forbids `unsafe` and denies `missing_docs`.
- Every parameter keeps its legacy identifier as its `#[id]`, so a state file
  written by the Rust build means the same thing to the JUCE build.

---

### Task 1: The range mapping

**Files:**
- Create: `rust/crates/rippler-params/Cargo.toml`
- Create: `rust/crates/rippler-params/src/lib.rs`
- Create: `rust/crates/rippler-params/src/ranges.rs`

- [ ] **Step 1: Create the crate**

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
nice-plug-core = { version = "0.4", default-features = false }
quick-xml = "0.38"
rippler-dsp = { workspace = true, features = ["params"] }
```

Create `rust/crates/rippler-params/src/lib.rs`:

```rust
//! Parameter definitions and legacy state compatibility for RipplerX.
//!
//! Everything that has to stay compatible with the JUCE implementation lives
//! here: parameter identifiers, value ranges, and the legacy XML reader.

pub mod enums;
pub mod legacy;
pub mod params;
pub mod patch;
pub mod ranges;
```

- [ ] **Step 2: Add the `params` feature to `rippler-dsp`**

Add to `rust/crates/rippler-dsp/Cargo.toml`:

```toml
[dependencies]
nice-plug-core = { version = "0.4", default-features = false, optional = true }

[features]
default = []
params = ["dep:nice-plug-core"]
```

The feature lets `rippler-dsp` implement nice-plug's `Enum` trait for
`ModalModel` and `MalletType`, which `rippler-params` cannot do because of
Rust's orphan rule. With the feature off, the crate has no framework
dependency at all.

- [ ] **Step 3: Write the range helpers**

Create `rust/crates/rippler-params/src/ranges.rs`:

```rust
//! Reproducing JUCE's parameter ranges.
//!
//! JUCE's `NormalisableRange` with a positive skew `s` normalises a value as
//! `p^(1/(1+s))`, and nice-plug's `FloatRange::Skewed` normalises as
//! `p^factor`, so `factor = 1/(1+s)` reproduces JUCE's curve exactly.

use nice_plug_core::params::range::FloatRange;

/// Builds the range for a JUCE parameter with no skew.
pub fn linear(min: f32, max: f32) -> FloatRange {
    FloatRange::Linear { min, max }
}

/// Builds the range for a JUCE parameter with a positive skew.
///
/// `skew` is JUCE's `NormalisableRange` skew. A skew of `0.3` gives a factor of
/// `1 / 1.3`.
pub fn skewed(min: f32, max: f32, skew: f32) -> FloatRange {
    FloatRange::Skewed {
        min,
        max,
        factor: 1.0 / (1.0 + skew),
    }
}
```

- [ ] **Step 4: Write the tests**

Add a `#[cfg(test)] mod tests` block asserting:

- `the_skew_factor_reproduces_juce_s_curve` — that `skewed(0.0, 1.0, 0.3).normalize(0.5)` equals `0.5^(1/1.3)` to within `1e-12`, and the same for skews `0.2`, `0.5` and `1.0`.
- `a_skew_of_one_is_a_square_root` — that `skewed(0.0, 1.0, 1.0).normalize(0.25)` is `0.5`.
- `a_linear_range_is_a_plain_proportion` — that `linear(20.0, 20000.0).normalize(20.0)` is `0.0` and `normalize(20000.0)` is `1.0`.

- [ ] **Step 5: Run the tests**

Run: `cd rust && cargo test -p rippler-params ranges`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/rippler-params rust/crates/rippler-dsp/Cargo.toml rust/Cargo.lock
git commit -m "feat(params): add the parameters crate and the range mapping"
```

---

### Task 2: The parameter enums

**Files:**
- Create: `rust/crates/rippler-params/src/enums.rs`
- Modify: `rust/crates/rippler-dsp/src/models.rs` (implement `Enum` for `ModalModel`)
- Modify: `rust/crates/rippler-dsp/src/mallet.rs` (implement `Enum` for `MalletType`)

- [ ] **Step 1: Implement `Enum` for the DSP enums**

In `rippler-dsp`, behind `#[cfg(feature = "params")]`, implement
`nice_plug_core::params::enums::Enum` for `ModalModel` and `MalletType`.

- `variants()` returns the human-readable names, in the legacy order.
  For `ModalModel`: "String", "Beam", "Squared", "Membrane", "Plate",
  "Drumhead", "Marimba", "Open Tube", "Closed Tube", "Marimba 2", "Bell",
  "Djembe".
  For `MalletType`: "Impulse", ten "Reserved", "User File", then the fourteen
  sample names — "Click 1" to "Click 6", "Blip", "Blop", "Metal 1", "Metal 2",
  "Wood", "Strike", "Perc 1", "Perc 2".
- `ids()` returns stable identifiers in the same order. Use lower-case,
  hyphen-free identifiers such as `"string"`, `"open_tube"`, `"click_1"`.
  **The order must match `variants()` exactly.**
- `to_index(self)` returns the variant's index.
- `from_index(index)` returns the variant at that index, or the first variant
  when the index is out of range.

The `ModalModel` and `MalletType` types already have `#[repr(u8)]` discriminants
matching the legacy choice order, so `to_index` is the discriminant.

- [ ] **Step 2: Write the parameter-layer enums**

Create `rust/crates/rippler-params/src/enums.rs` with three new enums, each
implementing `Enum`:

- `PartialCount` — the seven choices, in the legacy order: "4", "8", "16",
  "32", "64", "1", "2". The order is deliberately not sorted, because the
  legacy state stores the index.
- `NoiseFilterMode` — "LP", "BP", "HP".
- `Coupling` — "A+B", "A>B".

Each needs a doc comment explaining that the variant order matches the legacy
choice index, and stable identifiers.

- [ ] **Step 3: Write the tests**

Add tests asserting:

- `the_enum_indices_match_the_legacy_choice_order` — for each of the five enums, that the variant at index *n* is the one the legacy state means by *n*. Cross-check against the choice lists in `src/PluginProcessor.cpp`.
- `the_partial_count_order_is_not_sorted` — that `PartialCount` runs 4, 8, 16, 32, 64, 1, 2.
- `the_stable_ids_match_the_variants_in_order` — that `ids()` and `variants()` have the same length and are in the same order, for every enum.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-params enums`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-params/src/enums.rs rust/crates/rippler-dsp/src/models.rs rust/crates/rippler-dsp/src/mallet.rs
git commit -m "feat(params): define the parameter enums with their legacy order"
```

---

### Task 3: The parameter set

**Files:**
- Create: `rust/crates/rippler-params/src/params.rs`

- [ ] **Step 1: Write the parameter structs**

Create `rust/crates/rippler-params/src/params.rs`.

Define nested parameter structs, one per group, each deriving `Params` and each
field carrying its legacy `#[id]`:

- `MalletParams` — the seven mallet parameters.
- `NoiseParams` — the thirteen noise parameters.
- `ResonatorParams` — the fourteen parameters for one resonator. **The same
  struct serves both sides**, with the identifiers differing only in their
  `a_`/`b_` prefix.

  That means the `a_` and `b_` sets cannot both come from one `#[nested]`
  struct, because the identifiers are baked into the struct's fields. Either
  write two structs, `ResonatorAParams` and `ResonatorBParams`, with the same
  shape and different identifiers, or write the fields out at the top level.

  **Write two structs.** The duplication is mechanical, and it keeps the
  identifiers exact.
- `VelocityParams` — the twenty-one velocity parameters.
- `RipplerParams` — the top level, with `#[nested]` for the four groups plus
  its own eight parameters.

  **Do not use `#[nested(id_prefix = …)]`.** The prefix would rewrite the
  identifiers and break the compatibility this whole crate exists for. Plain
  `#[nested]` keeps the inner identifiers as they are.

Each parameter's range and default come from the table above. Use
`ranges::linear` and `ranges::skewed`, and `with_step_size` for the stepped
ones.

The `mallet_type` and `a_model`/`b_model` parameters are `EnumParam`s over
`MalletType` and `ModalModel`. The `a_partials`/`b_partials` parameters are
`EnumParam`s over `PartialCount`.

- [ ] **Step 2: Write the tests**

Add a `#[cfg(test)] mod tests` block asserting:

- `there_are_seventy_seven_parameters` — that `RipplerParams::param_map()` returns seventy-seven entries, counting the nested ones.
- `every_parameter_id_is_the_legacy_one` — that the set of identifiers equals the set declared in `src/PluginProcessor.cpp`, extracted from that file at test time or compared against a literal list.
- `the_parameter_ids_are_unique` — that no identifier appears twice.
- `the_defaults_match_the_juce_build` — that each parameter's default equals the one in `src/PluginProcessor.cpp`.
- `the_ranges_match_the_juce_build` — that each parameter's normalised endpoints match the JUCE range, which for a skewed parameter means checking the factor.

The last three need the JUCE values as data. Write them as literal lists in the
test module, with a comment pointing at `src/PluginProcessor.cpp` as the source
of truth, and check them by eye once.

- [ ] **Step 3: Run the tests**

Run: `cd rust && cargo test -p rippler-params params`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-params/src/params.rs
git commit -m "feat(params): define the parameter set with its legacy identifiers"
```

---

### Task 4: The legacy reader

**Files:**
- Create: `rust/crates/rippler-params/src/legacy.rs`

- [ ] **Step 1: Write the reader**

Create `rust/crates/rippler-params/src/legacy.rs`.

It reads a JUCE `ValueTree` XML of the form:

```xml
<PARAMETERS>
  <PARAM id="a_decay" value="10.77999973297119"/>
  <PARAM id="a_coarse"/>
</PARAMETERS>
```

and produces a set of parameter values plus any extra state.

The public shape should be something like:

```rust
/// The values read from a legacy patch.
pub struct LegacyPatch {
    /// The parameter values, by legacy identifier.
    pub values: BTreeMap<String, f64>,
    /// The base64-encoded user sample, if the patch carried one.
    pub user_sample: Option<String>,
    /// The program the patch was on, if it named one.
    pub current_program: Option<i32>,
}
```

and

```rust
/// Reads a legacy JUCE patch.
///
/// Returns `None` if the XML is not a `PARAMETERS` tree.
pub fn read_legacy_patch(xml: &str) -> Option<LegacyPatch>;
```

The rules, all established from the JUCE source and the factory presets:

- A `<PARAM>` element without a `value` attribute yields **0.0**, because
  JUCE's `replaceState` reads a missing property as `var()` and converts it to
  `0.0f`. This is *not* the same as "keep the default", and the difference
  matters for any parameter whose default is not zero.
- A parameter that is absent entirely is simply not in the map, and the
  parameter keeps its default.
- An identifier the plugin does not know is ignored.
- `a_cut` and `b_cut` need the historical migration: a value of `20.0` or
  more is in the old hertz scale, and becomes
  `ln(value / 20) / ln(20000 / 20)`. A value below `20.0` is already in the
  current scale and is left alone.
- The `userSample` property on the root element, if present, is kept as the
  base64 string. **It is JUCE's base64, not standard base64.**
- The `currentProgram` property on the root element, if present, is kept.

Use `quick-xml` to parse. It handles the entities and the attributes; do not
write an XML parser by hand.

- [ ] **Step 2: Write the conversion into parameter values**

Add a function that turns a `LegacyPatch` into the plain values the parameter
struct wants:

```rust
/// Converts a legacy patch into plain parameter values.
///
/// Applies the `a_cut` and `b_cut` migration, and snaps each value to its
/// parameter's step size the way JUCE's `snapToLegalValue` does.
pub fn to_parameter_values(patch: &LegacyPatch) -> BTreeMap<String, f64>;
```

Snapping matters: JUCE's `AudioParameterFloat::setValue` snaps to the legal
value, so a stored `10.77999973297119` with a step of `0.01` becomes `10.78`.
A value with no step size is left exactly as it is.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block asserting:

- `a_legacy_patch_reads_its_parameters` — that a small hand-written XML yields the expected values.
- `a_parameter_without_a_value_reads_as_zero` — that `<PARAM id="a_coarse"/>` yields `0.0`.
- `an_absent_parameter_is_simply_missing` — that the map has no entry for it.
- `an_unknown_identifier_is_ignored` — that an XML with a bogus identifier still parses, and the bogus one is not in the map.
- `the_cut_parameters_are_migrated_from_hertz` — that `a_cut` of `372.0` becomes `ln(372/20)/ln(1000)`, and that `20.0` becomes `0.0`.
- `a_cut_below_twenty_is_left_alone` — that `-0.5` stays `-0.5`.
- `the_user_sample_is_kept` — that the `userSample` attribute is captured.
- `the_current_program_is_kept`.
- `values_are_snapped_to_their_step` — that `10.77999973297119` with a `0.01` step becomes `10.78`, and that a value with no step is unchanged.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-params legacy`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-params/src/legacy.rs
git commit -m "feat(params): read legacy JUCE patches"
```

---

### Task 5: The patch format and the converter

**Files:**
- Create: `rust/crates/rippler-params/src/patch.rs`
- Create: `rust/crates/rippler-params/src/bin/rippler-import.rs`

- [ ] **Step 1: Write the patch format**

Create `rust/crates/rippler-params/src/patch.rs`.

A `.ripx` file is a JSON document:

```json
{
  "version": 1,
  "params": {
    "a_decay": 10.78,
    "a_model": 1,
    "mallet_type": 0
  },
  "userSample": "<JUCE base64>",
  "currentProgram": 3
}
```

The `params` object holds **plain** values, keyed by the legacy identifier, and
the choice parameters hold their **index**, exactly as the legacy state does.
That keeps one conversion, not two.

- [ ] **Step 2: Write the patch reader and writer**

Add:

```rust
/// Reads a `.ripx` patch.
pub fn read_patch(json: &str) -> Result<Patch, PatchError>;

/// Writes a `.ripx` patch.
pub fn write_patch(patch: &Patch) -> String;
```

with a `Patch` struct holding the same fields as the JSON above.

- [ ] **Step 3: Write the converter**

Create `rust/crates/rippler-params/src/bin/rippler-import.rs`, a small command
line tool:

```
rippler-import <legacy.xml> [-o <output.ripx>]
rippler-import --presets <directory> [-o <output-directory>]
```

It reads a legacy XML file, converts it and writes a `.ripx` file. With
`--presets`, it converts every `.xml` in a directory — which is how the twenty-eight
factory presets in `resources/presets/` are converted.

Without `-o`, it writes to standard output.

- [ ] **Step 4: Write the tests**

Add a `#[cfg(test)] mod tests` block asserting:

- `a_patch_round_trips` — that writing and reading a patch gives the same values.
- `a_legacy_patch_converts_to_a_patch` — that the converter produces the expected values, including the migrated `a_cut`.
- `the_converter_handles_every_factory_preset` — that all twenty-eight presets in `resources/presets/` convert without error, and that the resulting `a_cut` and `b_cut` values are all within `-1.0 … 1.0`.

That last test is the real check on the migration: every preset in the repository
holds `a_cut` and `b_cut` in the old hertz scale, so if the migration is right,
every converted value lands in the current parameter's range.

- [ ] **Step 5: Run the tests**

Run: `cd rust && cargo test -p rippler-params`
Expected: all tests pass.

- [ ] **Step 6: Run the converter over the factory presets**

Run: `cd rust && cargo run -p rippler-params --bin rippler-import -- --presets ../resources/presets -o /tmp/converted`
Expected: twenty-eight `.ripx` files written.

- [ ] **Step 7: Commit**

```bash
git add rust/crates/rippler-params/src/patch.rs rust/crates/rippler-params/src/bin/rippler-import.rs
git commit -m "feat(params): add the patch format and the legacy converter"
```

---

### Task 6: Completion gates

- [ ] **Step 1: Run the whole test suite**

Run: `cd rust && cargo test`
Expected: every test passes.

- [ ] **Step 2: Check for warnings**

Run: `cd rust && cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: Check formatting**

Run: `cd rust && cargo fmt --check`
Expected: no diff.

- [ ] **Step 4: Verify the JUCE tree is untouched**

```bash
git diff --stat <commit-before-this-plan>..HEAD -- src/ libs/ resources/ CMakeLists.txt .github/
```

Expected: no output.

## Notes for the next plan

- `rippler-params` depends on `nice-plug-core` and `quick-xml`. Plan 5 adds
  `nice-plug` and `nice-plug-egui` to `rippler-plugin`.
- `rippler-dsp` has an optional `params` feature that pulls in `nice-plug-core`
  so its enums can implement the `Enum` trait. Without the feature the crate
  has no framework dependency.
- The parameter identifiers are the legacy ones, verbatim. Plan 5's editor and
  the patch exporter both use them.
- The plugin's own state is a nice-plug `PluginState`, which is JSON. The
  `.ripx` patch format is a separate, simpler document that the editor's
  export and import actions use.
- The editor's "Import legacy patch" action reads a JUCE XML through
  `legacy::read_legacy_patch` and applies the values through the editor's
  parameter setter, exactly as `PluginProcessor::setCurrentProgram` does.
