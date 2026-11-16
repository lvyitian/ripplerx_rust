# Rust Excitation and Voice — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port RipplerX's excitation sources and the voice that binds them together — the sampler, the mallet, the noise generator and the voice.

**Architecture:** Four new modules in `rippler-dsp`. The sampler is owned by the plugin and borrowed by the mallets, which removes the C++'s second data race without introducing a lock. The voice takes the model tables as an argument rather than holding a shared reference.

**Tech Stack:** Rust 2024. One new dependency, `symphonia`, for decoding WAV, AIFF and FLAC.

**Design doc:** `docs/superpowers/specs/2026-11-16-rust-nice-plug-migration-design.md`
**Previous plans:** `2026-11-16-rust-workspace-and-dsp-primitives.md`, `2026-11-16-rust-modal-core.md`

---

## Position in the roadmap

| # | Plan | Status |
|---|---|---|
| 1 | Rust workspace and DSP primitives | Done |
| 2 | Modal core | Done |
| 3 | **This plan — excitation and voice** | Current |
| 4 | Parameters and compatibility | Next |
| 5 | Plugin shell | |
| 6 | Editor | |

## What already exists

`rippler-dsp` holds `lut.rs`, `rng.rs`, `filter.rs`, `envelope.rs`, `comb.rs`, `limiter.rs`, `utils.rs`, `models.rs`, `luts.rs`, `partial.rs`, `waveguide.rs` and `resonator.rs`. All twelve were verified **bit-identical** to their C++ originals by a differential harness.

The APIs this plan builds on:

- `Filter::low_pass`, `band_pass`, `high_pass`, `copy_from`, `process`
- `Envelope` with `init(srate, a, d, sustain_db, r, tension_a, tension_d, tension_r)`, `attack(scale)`, `release()`, `process() -> Stage`, `reset()`
- `Rng::new(seed)`, `next_bipolar()`
- `Luts::new(sample_rate)`, with `.a1` and `.sin`
- `models::{ModalModel, Ratios, MAX_PARTIALS, ratios, gains, beam, membrane, plate, djembe}`
- `Resonator` with `set_params`, `update`, `apply_pitch_bend`, `activate`, `clear`, `process`, and public `partials`, `waveguide` and `filter`

## Reference material

- `src/dsp/Sampler.{h,cpp}` — the sample loader and the interpolators
- `src/dsp/Mallet.{h,cpp}` — the mallet generator and the `MalletType` enum
- `src/dsp/Noise.{h,cpp}` — the noise generator
- `src/dsp/Voice.{h,cpp}` — the voice
- `resources/samples/*.flac` — the fourteen bundled mallet samples
- `src/PluginProcessor.cpp` — the caller that ties the voices together

Do not modify any of these.

---

## Findings that shape this plan

### 1. The C++ swaps the sampler under the audio thread

`PluginProcessor::onSlider` calls `malletSampler->loadInternalSample(mallet_type)` on the message thread, which clears and refills `sampler.waveform` — a `std::vector<double>` — while the audio thread reads it from `Mallet::process`. That is a data race, and it is the second one in this codebase after the model tables.

**Decision: the plugin owns the `Sampler`, and the mallets borrow it.**

The voice's `process` path becomes `voice.mallet.process(&sampler)`, where `sampler` is a field of the plugin and `voice` comes from a different field of the same plugin. Rust's borrow checker allows that, because they are disjoint fields.

The consequence is that the sample is loaded on the audio thread, when the mallet type changes. That is a rare one-off allocation rather than a per-sample one, and it is the price of removing the race. Document it.

Do **not** reach for `Arc`, `Mutex` or `RwLock` here. A lock on the audio path is worse than a rare allocation.

### 2. `MalletType` has twenty-six entries and the index matters

The plugin's `mallet_type` parameter is an `AudioParameterChoice` with twenty-six entries: `Impulse`, ten `Reserved` slots, `User File`, then fourteen named samples. The plugin stores the **choice index**, so the enum's discriminants must match that order exactly:

| Index | Variant | Bundled sample |
|---|---|---|
| 0 | `Impulse` | — |
| 1–10 | `Reserved1` … `Reserved10` | — |
| 11 | `UserFile` | — |
| 12 | `Sample1` | `click1.flac` |
| 13 | `Sample2` | `click2.flac` |
| 14 | `Sample3` | `click3.flac` |
| 15 | `Sample4` | `click4.flac` |
| 16 | `Sample5` | `click5.flac` |
| 17 | `Sample6` | `click6.flac` |
| 18 | `Sample7` | `blip.flac` |
| 19 | `Sample8` | `blop.flac` |
| 20 | `Sample9` | `metal1.flac` |
| 21 | `Sample10` | `metal2.flac` |
| 22 | `Sample11` | `wood.flac` |
| 23 | `Sample12` | `strike.flac` |
| 24 | `Sample13` | `perc1.flac` |
| 25 | `Sample14` | `perc2.flac` |

The ten `Reserved` slots must be kept. Dropping them would shift every later index and break every existing patch.

The C++'s `Mallet::process` only handles `kImpulse` and `type >= kUserFile`, so the reserved slots produce silence. Keep that.

### 3. The voice reads the model tables through an argument

`Voice` holds `Models&` in the C++, and reads `models.aModels[resA.nmodel]` inside `updateResonators`. In Rust the tables are immutable, so the voice takes them as an argument. The plugin holds the current table for each model and each side, and replaces a table when the ratio parameter changes.

Add a `ModelTables` type to `models.rs` that owns `[Ratios; 12]` for each side, starts as a copy of the shipped tables, and can have its beam, membrane or plate table replaced.

### 4. Microtuning stays out of the DSP crate

`Voice::note2freq` calls into MTS-ESP, which is an FFI dependency. `rippler-dsp` must stay free of FFI, so the voice takes the frequency it should tune to rather than computing it. The plugin computes it — with `mts-client-rs` when microtuning is active, and with the equal-temperament formula otherwise.

### 5. The noise generator's filter mode cannot be invalid

`Noise::initFilter` throws a bare `const char*` for an unknown mode. In Rust, treat an unknown mode as "no filter", and assert in debug builds. The mode comes from a three-way choice parameter, so it cannot be invalid in practice.

---

## Conventions

- Internal arithmetic is `f64`, matching the C++.
- Use `.powf(n)` for every C++ `pow(x, n)` call, never `.powi(n)`.
- The C++ clamps with `fmax(lo, fmin(hi, x))` and `fmin(hi, fmax(lo, x))`, which return the non-NaN operand and differ from each other for a NaN. Rust's `f64::min` and `f64::max` already match C's `fmin` and `fmax`, so build the helpers on those. `modal_core`'s `clamp_cpp` and `clamp_cpp_inverted` are the pattern.
- Every module carries a doc comment naming the C++ file it ports.
- Tests live in a `#[cfg(test)] mod tests` block at the bottom of the module file.
- The crate forbids `unsafe` and denies `missing_docs`.

---

### Task 1: The sampler

**Files:**
- Create: `rust/crates/rippler-dsp/src/sampler.rs`
- Create: `rust/crates/rippler-dsp/src/mallet.rs` (the `MalletType` enum only)
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod sampler;` and `pub mod mallet;`)
- Modify: `rust/crates/rippler-dsp/Cargo.toml` (add `symphonia`)

The `MalletType` enum lives in `mallet.rs`, and the sampler needs it to map a
variant to its bundled sample, so the enum is created here and Task 2 adds the
`Mallet` struct to the same file.

- [ ] **Step 1: Declare the module and add the dependency**

Add `pub mod sampler;` to `rust/crates/rippler-dsp/src/lib.rs`.

Add to `rust/crates/rippler-dsp/Cargo.toml`:

```toml
[dependencies]
symphonia = { version = "0.5", default-features = false, features = ["flac", "wav", "aiff", "pcm", "mp3"] }
```

`symphonia` is the only third-party dependency this crate will have. It replaces JUCE's `AudioFormatManager`, which is not available in Rust.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/sampler.rs`.

It holds:

- `pub struct Sampler` with the C++'s public state: `waveform: Vec<f64>`, `is_user_file: bool`, `wavesrate: f64` (default 44100.0) and `pitchfactor: f64` (default 1.0).
- `impl Sampler`:
  - `new() -> Self`, with the C++'s defaults.
  - `set_pitch(&mut self, semis: f64)` — `pitchfactor = 2^(semis / 12)`.
  - `load_encoded(&mut self, encoded: &str)` — **clear** `waveform`, decode base64 into raw `f64`s, append them, then set `is_user_file = true`. This is the format the JUCE plugin stores in its session state, so it must read the same bytes. The C++ clears first, so a second load replaces the first rather than adding to it.
  - `load_internal_sample(&mut self, variant: MalletType)` — decode the bundled FLAC for that variant.
  - `load_sample(&mut self, path: &std::path::Path)` — decode a user file, falling back to `Sample1` when the file cannot be read.
  - `load_bytes(&mut self, bytes: &[u8]) -> bool` — decode a byte slice, returning whether it succeeded.
  - `wave_lerp(&self, pos: f64) -> f64` and `wave_cubic(&self, pos: f64) -> f64`, ported from the C++.

The fourteen bundled samples are embedded with `include_bytes!`. The path from `rust/crates/rippler-dsp/src/sampler.rs` to the repository root is four levels up, so:

```rust
const CLICK1: &[u8] = include_bytes!("../../../../resources/samples/click1.flac");
```

and one constant per sample, with a function mapping a `MalletType` to its bytes.

**Decoding with symphonia.** The C++'s `loadSampleFromBinary` does, in order:

1. Decode the file to `f32` samples.
2. Sum the channels and divide by the channel count, making the signal mono.
3. Truncate to `min(3 * wavesrate, num_samples)` samples.
4. Scale the whole waveform so its largest absolute value is 1.0. If the peak is zero, leave it alone.

Keep that order exactly, because the truncation happens **before** the normalisation and the peak is computed over the truncated waveform.

`symphonia`'s API: probe the format with `symphonia::default::get_probe()`, take the default track, make a decoder from its codec parameters, and decode packets into samples. The sample rate comes from the track's codec parameters. The decoded samples are `f32`, matching JUCE's `AudioBuffer<float>`.

**A subtlety in the C++ worth preserving:** `loadSampleFromBinary` catches every exception and falls back to `Sample1`, and `loadInternalSample` falls back to `Sample1` for a variant that is not one of the fourteen. Keep both fallbacks.

**Another subtlety:** `loadEncoded` does *not* set `wavesrate`, and does not normalise. It appends raw doubles to the existing waveform. Reproduce that exactly, because the JUCE session format depends on it.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `a_new_sampler_has_the_cpp_defaults` — asserting `waveform` is empty, `is_user_file` is false, `wavesrate` is 44100.0 and `pitchfactor` is 1.0.
- `setting_the_pitch_scales_by_semitones` — asserting `pitchfactor` is `2^(semis/12)`.
- `loading_an_encoded_waveform_reads_the_doubles` — asserting that a base64 blob of known doubles round-trips exactly, and that `is_user_file` becomes true.
- `loading_an_internal_sample_decodes_it` — asserting that each of the fourteen bundled samples decodes, that the waveform is not empty, that its peak is exactly 1.0, and that its length is `min(3 * wavesrate, source length)`.
- `the_bundled_samples_have_the_expected_peak_and_length` — pinning the decoded length and peak of each of the fourteen, so a change in the decoder or in the bundled files is caught. Take these values from a first run and then verify them by hand against the files.

  The peak is **not** always exactly `1.0`. The normalisation multiplies by the
  reciprocal of the peak, so the result can land one unit in the last place below
  one — `blop.flac` peaks at `0.9999999999999999`. Pin the real values.
- `the_waveform_is_mono_and_truncated_to_three_seconds` — asserting the length rule.
- `wave_lerp_interpolates_linearly_and_wraps` — asserting the wrap at the end of the waveform.
- `wave_cubic_matches_the_catmull_rom_formula` — asserting against the formula, and that it wraps.
- `loading_a_missing_file_falls_back_to_the_first_sample` — asserting the fallback.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp sampler`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-dsp/src/sampler.rs rust/crates/rippler-dsp/src/lib.rs rust/crates/rippler-dsp/Cargo.toml rust/Cargo.lock
git commit -m "feat(dsp): port the sampler with symphonia decoding"
```

---

### Task 2: The mallet

**Files:**
- Create: `rust/crates/rippler-dsp/src/mallet.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod mallet;`)

- [ ] **Step 1: Declare the module**

Add `pub mod mallet;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/mallet.rs`.

It holds the `MalletType` enum, whose discriminants must match the table in the findings above:

```rust
/// The mallet that excites a resonator.
///
/// The discriminants match the `mallet_type` parameter's choice order, so a
/// variant's discriminant is the value the plugin stores. The ten reserved
/// slots must be kept: dropping them would shift every later index and
/// break every existing patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MalletType {
    /// A unit impulse through a band-pass filter.
    Impulse = 0,
    /// Reserved by the JUCE implementation.
    Reserved1 = 1,
    /// Reserved by the JUCE implementation.
    Reserved2 = 2,
    /// Reserved by the JUCE implementation.
    Reserved3 = 3,
    /// Reserved by the JUCE implementation.
    Reserved4 = 4,
    /// Reserved by the JUCE implementation.
    Reserved5 = 5,
    /// Reserved by the JUCE implementation.
    Reserved6 = 6,
    /// Reserved by the JUCE implementation.
    Reserved7 = 7,
    /// Reserved by the JUCE implementation.
    Reserved8 = 8,
    /// Reserved by the JUCE implementation.
    Reserved9 = 9,
    /// Reserved by the JUCE implementation.
    Reserved10 = 10,
    /// A user-supplied sample.
    UserFile = 11,
    /// The bundled `click1.flac` sample.
    Sample1 = 12,
    // ... through Sample14 = 25
}
```

Give every variant its own doc comment, and add a doc comment on the type explaining the index parity.

Then `pub struct Mallet`, ported from `src/dsp/Mallet.h`:

- The C++ holds a `Sampler&`. **Do not.** The mallet receives the sampler as an argument, because the plugin owns it.
- Public state: `srate`, `impulse`, `countdown`, `env`, `impulse_filter`, `ktrack`, `keytrack_factor`, `playback`, `playback_speed`, `disable_filter`, `sample_filter`.
- Private: `type: MalletType`.

Its methods:

- `Mallet::new() -> Self`, with the C++'s defaults: `srate = 44100.0`, `impulse = 0.0`, `countdown = 0`, `env = 0.0`, `ktrack = 0.0`, `keytrack_factor = 1.0`, `playback = f64::INFINITY`, `playback_speed = 1.0`, `disable_filter = false`, `type = MalletType::Impulse`.
- `trigger(&mut self, mallet_type: MalletType, srate: f64, freq: f64, note: i32, ktrack: f64, sampler: &Sampler)`, ported from `Mallet::trigger`. The impulse branch sets up the band-pass, the countdown and the decay envelope; the sample branch computes the key-tracking factor and the playback speed from the sampler.
- `process(&mut self, sampler: &Sampler) -> f64`, ported from `Mallet::process`.
- `clear(&mut self)`, ported.
- `set_filter(&mut self, norm: f64)`, ported from `Mallet::setFilter`.

Note that `Mallet::clear` sets `playback = INFINITY`, and that `Mallet::process` checks `playback < sampler.waveform.size()` before reading, so a cleared mallet produces silence.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `the_mallet_type_discriminants_match_the_choice_order` — asserting the discriminants are 0 through 25 in order, with the reserved slots in the middle and the samples at the end.
- `a_new_mallet_has_the_cpp_defaults` — asserting the initial values, including `playback` being infinite.
- `the_impulse_mallet_produces_a_decaying_burst` — asserting that triggering produces a non-zero sample and that the burst decays over roughly the hundred milliseconds the countdown allows.
- `the_impulse_mallet_stops_after_the_countdown` — asserting it goes silent.
- `the_reserved_mallet_types_produce_silence` — asserting that a reserved type produces no output.
- `the_sample_mallet_reads_the_sampler` — asserting that a sample mallet produces output from the sampler's waveform and that the playback position advances.
- `the_key_tracking_scales_the_playback_speed` — asserting that the key-tracking factor is `2^((note - 60) / 12 * ktrack)`.
- `the_filter_normalisation_selects_a_low_pass_or_a_high_pass` — asserting that a negative normalisation gives a low-pass response and a positive one a high-pass.
- `a_zero_normalisation_disables_the_filter` — asserting that the filter is bypassed.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp mallet`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-dsp/src/mallet.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the mallet generator"
```

---

### Task 3: The noise generator

**Files:**
- Create: `rust/crates/rippler-dsp/src/noise.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod noise;`)

- [ ] **Step 1: Declare the module**

Add `pub mod noise;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Write the module**

Create `rust/crates/rippler-dsp/src/noise.rs`, ported from `src/dsp/Noise.{h,cpp}`.

The C++'s `Noise::init` takes seventeen arguments. Introduce a settings struct:

```rust
/// The parameters that configure the noise generator.
#[derive(Clone, Copy, Debug)]
pub struct NoiseSettings {
    /// The filter mode: 0 low-pass, 1 band-pass, 2 high-pass.
    pub filter_mode: u8,
    /// The filter's centre frequency.
    pub freq: f64,
    /// The filter's resonance.
    pub q: f64,
    /// The attack time in milliseconds.
    pub attack: f64,
    /// The decay time in milliseconds.
    pub decay: f64,
    /// The sustain level, from 0 to 1.
    pub sustain: f64,
    /// The release time in milliseconds.
    pub release: f64,
    /// How much velocity shifts the filter frequency.
    pub vel_freq: f64,
    /// How much velocity shifts the filter resonance.
    pub vel_q: f64,
    /// The attack tension.
    pub att_ten: f64,
    /// The decay tension.
    pub dec_ten: f64,
    /// The release tension.
    pub rel_ten: f64,
    /// How much velocity shifts the attack.
    pub vel_att: f64,
    /// How much velocity shifts the decay.
    pub vel_dec: f64,
    /// How much velocity shifts the sustain.
    pub vel_sus: f64,
    /// How much velocity shifts the release.
    pub vel_rel: f64,
}
```

`Noise` holds the C++'s public state (`att`, `dec`, `sus`, `rel`, the three tensions, the five velocity fields, `vel_freq`, `vel_q`, `srate`, `vel`, `q`, `filter_active`), an `Envelope`, the `Filter`, and the second `Filter` the C++ calls `osc_filter`, which is the one used to filter the oscillator exciters.

Its methods:

- `Noise::new() -> Self`, with the C++'s defaults: `sus = -60.0`, `srate = 44100.0`, `q = 0.707`, `filter_active = false`.
- `Noise::init(&mut self, srate: f64, settings: &NoiseSettings)`, ported.
- `Noise::attack(&mut self, vel: f64)`, ported — it recomputes the filter and the envelope for the velocity and starts the envelope's attack.
- `Noise::release(&mut self)`, `Noise::clear(&mut self)`.
- `Noise::process(&mut self, rng: &mut Rng) -> f64`, ported from `Noise::process`. The C++ calls the global `std::rand()`; use the per-voice `Rng` instead, with the same mapping: `(rand() / RAND_MAX) * 2.0 - 1.0`, which is `Rng::next_bipolar()`.
- `Noise::process_osc(&mut self, input: f64) -> f64`, ported from `Noise::processOSC`.

Two details to get right:

- The C++ calls `env.process()` **before** reading `env.env`, so the first sample already uses the post-process envelope value.
- The C++ clears the filter when the envelope finishes, to avoid a pop. Keep that.

The filter mode: the C++ throws for an unknown mode. In Rust, leave the filter unchanged and set `filter_active` to false, and assert in debug builds.

The `osc_filter` is a copy of the filter's **coefficients** with its own history, so use `copy_from`, not assignment.

- [ ] **Step 3: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `a_new_noise_generator_has_the_cpp_defaults` — asserting the initial values.
- `the_noise_is_silent_before_the_attack` — asserting `process` returns zero while the envelope is off.
- `the_noise_bursts_and_then_falls_silent` — asserting the envelope runs through its stages and the output ends at zero.
- `the_noise_uses_the_supplied_generator` — asserting that two generators with the same seed produce the same output, and that different seeds produce different output. This is what makes the noise reproducible.
- `the_filter_is_active_for_the_three_modes` — asserting `filter_active` for each mode and frequency combination the C++ checks.
- `an_unknown_filter_mode_does_not_panic` — the port uses `debug_assert!` in the unknown branch, so this test must catch the panic in a debug build while checking the release behaviour. The two requirements are otherwise contradictory.
- `the_oscillator_path_shares_the_filter` — asserting that `process_osc` filters its input with the same coefficients.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp noise`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-dsp/src/noise.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the noise generator"
```

---

### Task 4: The voice

**Files:**
- Create: `rust/crates/rippler-dsp/src/voice.rs`
- Modify: `rust/crates/rippler-dsp/src/lib.rs` (add `pub mod voice;`)
- Modify: `rust/crates/rippler-dsp/src/models.rs` (add `ModelTables`)

- [ ] **Step 1: Declare the module**

Add `pub mod voice;` to `rust/crates/rippler-dsp/src/lib.rs`.

- [ ] **Step 2: Add `ModelTables` to `models.rs`**

```rust
/// The current model tables for both resonators.
///
/// The C++ keeps a table per model per side and mutates the beam, membrane
/// and plate tables in place when the ratio parameter changes, on the message
/// thread, while the audio thread reads them. Here the tables are owned by
/// the plugin and only ever touched on the audio thread.
#[derive(Clone, Debug)]
pub struct ModelTables {
    a: [Ratios; 12],
    b: [Ratios; 12],
}
```

with:

- `ModelTables::new() -> Self`, copying the shipped tables for both sides.
- `ModelTables::a(&self, model: ModalModel) -> &Ratios` and `b(...)`.
- `ModelTables::recalc(&mut self, res_a: bool, model: ModalModel, ratio: f64)`, which recomputes the beam, membrane or plate table for the given side. For any other model, do nothing.

The shipped tables are `models::ratios(model)`, so `new` copies them.

- [ ] **Step 3: Write the module**

Create `rust/crates/rippler-dsp/src/voice.rs`, ported from `src/dsp/Voice.{h,cpp}`.

`Voice` holds the C++'s public state — `note`, `freq`, `vel`, `is_release`, `is_pressed`, `couple`, `mallet_ktrack`, `split`, `srate`, `a_ratio`, `b_ratio`, `pressed_ts`, `release_ts`, `is_fading`, `fade_total_samples`, `fade_samples`, `mallet_type`, `mallet_freq`, `new_freq`, `new_vel`, `new_note`, `a_pitch_factor`, `b_pitch_factor`, `pitch_bend` — plus a `Mallet`, a `Noise`, two `Resonator`s, and the two phase arrays.

**The voice does not own the model tables.** The C++ holds `Models&`; here `update_resonators` takes them as an argument.

**The voice does not own a sampler either.** `Mallet::process` takes it as an argument, so `Voice::process` takes it too.

Its methods, ported from the C++:

- `Voice::new() -> Self`.
- `Voice::trigger(&mut self, timestamp: u64, srate: f64, note: i32, vel: f64, mallet_type: MalletType, mallet_freq: f64, mallet_ktrack: f64, skip_fadeout: bool, freq: f64, models: &ModelTables, sampler: &Sampler)`.
  The C++ computes the note's frequency inside `trigger`, calling into MTS-ESP. **Take the frequency as an argument** instead, so that this crate stays free of FFI. The plugin computes it, with `mts-client-rs` when microtuning is active.
- `Voice::trigger_start(&mut self, reset: bool, models: &ModelTables, sampler: &Sampler)`.
- `Voice::fade_out(&mut self, models: &ModelTables, sampler: &Sampler) -> f64`. It needs both, because `fadeOut` calls `triggerStart(true)`, which calls `updateResonators` and `mallet.trigger`.
- `Voice::release(&mut self, timestamp: u64, models: &ModelTables)`.
- `Voice::clear(&mut self)`.
- `Voice::set_pitch(&mut self, a_coarse: f64, b_coarse: f64, a_fine: f64, b_fine: f64, pitch_bend: f64)`.
- `Voice::set_ratio(&mut self, a_ratio: f64, b_ratio: f64)`.
- `Voice::set_coupling(&mut self, couple: bool, split: f64)`.
- `Voice::apply_pitch(model: &mut Ratios, factor: f64)`, an associated function, as in the C++.
- `Voice::apply_pitch_bend(&mut self, bend: f64, luts: &Luts)`.
- `Voice::process_oscillators(&mut self, is_a: bool, luts: &Luts) -> f64`.
- `Voice::freq_shift(&self, fa: f64, fb: f64) -> f64`.
- `Voice::calc_frequency_shifts(&self, a_model: &Ratios, b_model: &Ratios) -> (Ratios, Ratios)`.
- `Voice::update_resonators(&mut self, models: &ModelTables)`.

Two details in `update_resonators` that matter:

- The djembe model is computed per voice from the voice's own frequency and ratio, not read from the table.
- The pitch factor and the coupling frequency shift are applied to a **copy** of the table, not to the shared one. The C++ copies the array, applies the shift and passes the copy to the resonator. Keep that.

`calc_frequency_shifts` is O(64 × 64). It runs on the audio thread at note-on. Keep it for now, but note the cost in a comment: 4,096 iterations of `sqrt` and `cos` in one callback. Plan 5 should measure it before deciding whether to precompute.

`process_oscillators` reads the sine lookup table, so it needs the `Luts`.

- [ ] **Step 4: Write the tests**

Add a `#[cfg(test)] mod tests` block with:

- `a_new_voice_has_the_cpp_defaults` — asserting the initial values.
- `triggering_a_voice_starts_it` — asserting that the voice becomes pressed, that the resonators activate and that the mallet and noise start.
- `releasing_a_voice_marks_it_released` — asserting `is_release`, that `is_pressed` goes false, and that the noise releases.
- `the_pitch_factor_follows_the_coarse_and_fine_pitch` — asserting `a_pitch_factor` is `2^((a_coarse + a_fine / 100) / 12)`.
- `the_frequency_shift_is_symmetric` — asserting that `calc_frequency_shifts` leaves the two tables as mirror images when the models are equal, so that `a[i] + b[i] == 2 * model[i]`, and that the shifts stay small.

  The shifts are **not** zero for identical models: the diagonal term contributes
  a deliberate pseudo-random offset of `|split + cos(avg) / 5| / 2.5 / freq`. Assert
  the symmetry and the magnitude, not that the shift vanishes.
- `coupled_resonators_shift_apart` — asserting that two different models produce different shift tables.
- `the_pitch_factor_scales_a_copy_of_the_model` — asserting that `apply_pitch` does not change the original table.
- `the_djembe_model_is_computed_per_voice` — asserting that a djembe resonator's ratios follow the voice's frequency.
- `a_repeated_note_fades_the_old_voice_out` — asserting the fade behaviour.

- [ ] **Step 5: Run the tests**

Run: `cd rust && cargo test -p rippler-dsp voice`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/rippler-dsp/src/voice.rs rust/crates/rippler-dsp/src/models.rs rust/crates/rippler-dsp/src/lib.rs
git commit -m "feat(dsp): port the voice"
```

---

### Task 5: Completion gates

- [ ] **Step 1: Run the whole test suite**

Run: `cd rust && cargo test`
Expected: every test passes.

- [ ] **Step 2: Check for warnings**

Run: `cd rust && cargo clippy --all-targets -- -D warnings`
Expected: no warnings. Fix them rather than suppressing them.

- [ ] **Step 3: Check formatting**

Run: `cd rust && cargo fmt --check`
Expected: no diff.

- [ ] **Step 4: Build the differential harness for the excitation and the voice**

The harness at `F:\rippler-harness\` covers the primitives and the modal core. Extend it to cover:

- `Mallet::trigger`, `process`, `clear` and `setFilter` against the Rust equivalents.
- `Noise::init`, `attack`, `process` and `processOSC` against the Rust equivalents. The noise's random source differs by design, so drive both sides with the **same** pseudo-random sequence — generate a fixed sequence of values in each language and feed it to the respective generator — so the comparison is meaningful.
- `Voice::trigger`, `release`, `updateResonators`, `calcFrequencyShifts` and `processOscillators` against the Rust equivalents.

**`Mallet` and `Noise` do not depend on JUCE**, so they can be compiled directly: `Mallet.cpp` includes `Sampler.h` for the `MalletType` enum and `JuceHeader.h` for `INFINITY` and `std::pow`, and `Noise.cpp` needs `std::rand`. Provide the small stand-ins the existing harness already uses.

**`Voice` depends on MTS-ESP** through `Voice.h`'s `#include "libMTSClient.h"`, and on `Models`, `Resonator` and `Sampler`. `MTSClient` is only used for the note-to-frequency call, which the Rust port takes as an argument. Provide a stand-in `MTSClient` header that returns the equal-temperament frequency, and compile `Voice.cpp` against it. `Voice.cpp` also uses `juce::MathConstants<double>::twoPi` and `pi`, which the harness already substitutes.

**`Sampler` cannot be compared against the C++**, because the C++ decodes with JUCE's `AudioFormatManager` and JUCE is not buildable on this machine. Verify the sampler by other means instead:

- Assert the decoded length and peak for each of the fourteen bundled samples, and check those values by hand against the files.
- Compare the interpolators, `wave_lerp` and `wave_cubic`, against the C++ directly — they only need the `waveform` vector, not the decoder. Extract the C++'s interpolation functions into the harness and feed both sides the same waveform.

Report the maximum absolute difference for each module you could compare.

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

- `rippler-dsp` now depends on `symphonia`. Plan 4 adds `nice-plug-core` to `rippler-params`, and Plan 5 adds `nice-plug` and `nice-plug-egui` to `rippler-plugin`.
- The plugin will own the `Sampler` and pass it to the voices. The voice's `process` path is `voice.mallet.process(&sampler)`.
- The plugin will own the `ModelTables` and pass them to `voice.update_resonators(&models)`. A ratio parameter change recomputes the affected table on the audio thread.
- The plugin computes the note's frequency and passes it to `voice.trigger(...)`. Plan 5 wires in `mts-client-rs` for that.
- `Voice::calc_frequency_shifts` is O(64 × 64) and runs on the audio thread at note-on. Plan 5 should measure it.
