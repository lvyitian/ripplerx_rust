# Rust Plugin Shell — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the ported DSP and parameters into a real plugin — a nice-plug `Plugin` that hosts load as VST3 and CLAP, with the same identity as the JUCE build.

**Architecture:** A third crate, `rippler-plugin`, that owns the voices, the sampler, the model tables and the parameters, and implements nice-plug's `Plugin` trait. The per-sample loop is a direct port of the JUCE `processBlock`.

**Tech Stack:** Rust 2024. Adds `nice-plug`, `nice-plug-egui` (in the editor plan) and `mts-client-rs`.

**Design doc:** `docs/superpowers/specs/2026-11-16-rust-nice-plug-migration-design.md`

---

## Position in the roadmap

| # | Plan | Status |
|---|---|---|
| 1 | Rust workspace and DSP primitives | Done |
| 2 | Modal core | Done |
| 3 | Excitation and voice | Done |
| 4 | Parameters and compatibility | Done |
| 5 | **This plan — plugin shell** | Current |
| 6 | Editor | Next |

## What already exists

- `rippler-dsp` — sixteen modules, all verified bit-identical to their C++ originals.
- `rippler-params` — the 77 parameters, the range mapping, the legacy reader and the `.ripx` format, with a generated id→parameter mapping.
- `rippler-plugin` — an empty crate with `crate-type = ["cdylib", "lib"]`.

---

## Findings that shape this plan

### 1. The VST3 class ID is platform-dependent, and both values are now known

JUCE derives a VST3 plugin's class ID from its manufacturer code, its plugin code
and the interface type, in `juce::VST3Interface::jucePluginId`. The manufacturer
and plugin codes come from the CMake options as four-character strings packed
big-endian into a `uint32`, so for RipplerX they are `0x54696C72` ("Tilr") and
`0x52697078` ("Ripx").

The two fixed words are `0xABCDEF01` and `0x9182FAEB` for the component
interface. **The byte order of those two words depends on the platform JUCE was
compiled for** — `juce_VST3Interface.h` branches on `JUCE_WINDOWS`.

Both values have been confirmed against the shipped release binaries:

| Platform | Component class ID |
|---|---|
| Windows | `01EFCDAB8291EBFA54696C7252697078` |
| macOS and Linux | `ABCDEF019182FAEB54696C7252697078` |

The Windows value appears verbatim in `ripplerx-win-v1.5.19.zip`'s
`RipplerX.vst3` binary. The Linux binary stores the same two words as native
`uint32`s, which matches the non-Windows branch of JUCE's function.

**The Rust plugin must use the class ID matching the platform it was built for**,
so that a host finds the plugin where the JUCE build left it. Use a `cfg` on the
target operating system.

The class ID must be written out as a byte array, not a string literal, because
the bytes are not ASCII.

### 2. The audio layout has a main input bus

RipplerX's "Audio In" feature is a **normal stereo input bus**, not a VST3
auxiliary bus. The JUCE build declares it with
`BusesProperties().withInput("Sidechain", stereo, true)`, and the README
documents that routing a track into the plugin works in Reaper, Ableton and
Bitwig.

So the layout is a main input of two channels and a main output of two
channels, with MIDI input. nice-plug's `aux_input_ports` is the *other* thing —
a true VST3 auxiliary bus — and is not what this feature is.

This makes the plugin an instrument with an audio input, which hosts may present
as an effect rather than an instrument. That is exactly what the JUCE build does
today, so the behaviour matches.

### 3. The per-sample loop is a direct port of `processBlock`

`src/PluginProcessor.cpp`'s `processBlockByType` is the reference. Its shape:

1. Read the parameters that vary per sample into locals.
2. Walk the queued MIDI messages, decrementing their offsets.
3. For each sample: interpolate the pitch bend, apply the MIDI events, then for
   each voice run the mallet, the noise, the two resonators, the coupling mix,
   the gain, the comb stereoizer and the limiter.

The JUCE build runs this per sample because the pitch bend glides per sample.
Keep that.

The JUCE build accumulates the MIDI messages into a queue with a per-sample
offset, because JUCE delivers MIDI at block granularity. nice-plug delivers
note events with a sample offset too, so the same queue applies.

### 4. The model tables belong to the plugin, and the ratio recalc happens on the audio thread

The C++ mutates the beam, membrane and plate tables on the message thread while
the audio thread reads them. In this port the tables are immutable and owned by
the plugin, and the plugin recomputes the affected table when it notices the
ratio parameter has changed — on the audio thread, at the top of the block.

`rippler_dsp::models::ModelTables` already provides `recalc`.

### 5. The plugin owns the sampler

The C++ swaps the sampler on the message thread while the audio thread reads it.
Here the plugin owns the `Sampler` and passes `&Sampler` into the voices, so the
swap happens on the audio thread when the mallet type changes. That is a rare
one-off allocation, and it removes the race.

### 6. Microtuning is a plugin-level concern

`rippler-dsp` is free of FFI. The plugin computes the note's frequency — with
`mts-client-rs` when MTS-ESP is active, and with the equal-temperament formula
otherwise — and passes it to `Voice::trigger`.

### 7. The `Voice` has no `process` method

Neither the C++ nor the Rust `Voice` has one. The plugin drives the chain, as
`processBlock` does. That keeps the voice's state visible to the plugin.

---

## The VST3 and CLAP identities

```rust
/// The VST3 class ID, matching the JUCE build on this platform.
#[cfg(target_os = "windows")]
const VST3_CLASS_ID: [u8; 16] = [
    0x01, 0xEF, 0xCD, 0xAB, 0x82, 0x91, 0xEB, 0xFA,
    0x54, 0x69, 0x6C, 0x72, 0x52, 0x69, 0x70, 0x78,
];

/// The VST3 class ID, matching the JUCE build on this platform.
#[cfg(not(target_os = "windows"))]
const VST3_CLASS_ID: [u8; 16] = [
    0xAB, 0xCD, 0xEF, 0x01, 0x91, 0x82, 0xFA, 0xEB,
    0x54, 0x69, 0x6C, 0x72, 0x52, 0x69, 0x70, 0x78,
];

/// The CLAP identifier, unchanged from the JUCE build.
const CLAP_ID: &str = "com.tilr.ripplerx";
```

Add a test that asserts these byte arrays, with a comment saying where they came
from and that they were verified against the released binaries.

---

## Conventions

- The audio thread must not allocate, lock, format or panic.
- Use `.powf(n)`, never `.powi(n)`.
- Every public item needs a doc comment.

---

### Task 1: The crate and the plugin type

**Files:**
- Create: `rust/crates/rippler-plugin/Cargo.toml`
- Create: `rust/crates/rippler-plugin/src/lib.rs`
- Create: `rust/crates/rippler-plugin/src/identity.rs`

- [ ] **Step 1: Fill in the manifest**

`rust/crates/rippler-plugin/Cargo.toml`:

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
nice-plug = { version = "0.4", default-features = false }
nice-plug-core = { version = "0.4", default-features = false }
rippler-dsp = { workspace = true }
rippler-params = { workspace = true }
```

`nice-plug` and `nice-plug-core` are both needed, for the same reason as in
`rippler-params`: the `Params` derive emits paths into `nice_plug`.

Add `nice-plug` to the workspace's `[workspace.dependencies]` if that is where
the other cross-crate dependencies live.

- [ ] **Step 2: Write the identity module**

Create `rust/crates/rippler-plugin/src/identity.rs` with the two class ID
arrays and the CLAP identifier, each with a doc comment explaining where the
value came from.

Add a test asserting the exact bytes of both arrays.

- [ ] **Step 3: Write the plugin skeleton**

Create `rust/crates/rippler-plugin/src/lib.rs` with the plugin type:

```rust
//! The RipplerX audio plugin.

pub mod identity;

use nice_plug::prelude::*;
use rippler_dsp::sampler::Sampler;
use rippler_dsp::voice::Voice;
use rippler_params::params::RipplerParams;
use std::sync::Arc;

/// The plugin.
pub struct RipplerX {
    /// The parameters, shared with the editor.
    params: Arc<RipplerParams>,
    /// The voices.
    voices: Vec<Voice>,
    /// The sampler, shared with the voices.
    sampler: Sampler,
    /// The current model tables.
    models: ModelTables,
    // ...
}
```

Implement `Default` and the `Plugin` trait, with:

- `NAME`, `VENDOR`, `URL`, `EMAIL` and `VERSION` matching the JUCE build. The
  JUCE build's company is "Tilr", the plugin name "RipplerX", the bundle
  identifier `com.tilr.ripplerx`, and the version from `CMakeLists.txt`.
- `AUDIO_IO_LAYOUTS` with a main input of two channels, a main output of two
  channels, and no auxiliary ports.
- `MIDI_INPUT: MidiConfig::Basic`.
- `SAMPLE_ACCURATE_AUTOMATION: true`.
- `params()` returning the `Arc<RipplerParams>`.
- `activate()` allocating the voices and the sampler, and the `Luts`.
- `process()` — filled in by Task 2.
- `deactivate()`.

`type Editor = ();` for now — the editor plan replaces it.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-plugin`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-plugin rust/Cargo.lock
git commit -m "feat(plugin): scaffold the plugin crate with its VST3 identity"
```

---

### Task 2: The processing loop

**Files:**
- Create: `rust/crates/rippler-plugin/src/process.rs`
- Modify: `rust/crates/rippler-plugin/src/lib.rs`

- [ ] **Step 1: Write the parameter snapshot**

Create `rust/crates/rippler-plugin/src/process.rs`.

The JUCE build reads every parameter once per block into locals and then works
from those. Do the same: a `BlockParams` struct holding plain `f64` and `bool`
fields, built from the parameter struct at the top of each block.

The parameters that the JUCE build reads per *sample* rather than per block are
the ones that scale with velocity: `mallet_mix`, `mallet_res`, `vel_mallet_mix`,
`vel_mallet_res`, `noise_mix`, `noise_res`, `vel_noise_mix`, `vel_noise_res`,
and the noise oscillator's amount. Read those from the parameter struct inside
the voice loop, exactly as the C++ does.

- [ ] **Step 2: Write the MIDI queue**

The JUCE build keeps a `std::vector<MIDIMsg>` with a per-sample offset, because
JUCE hands it MIDI at block granularity. nice-plug's `ProcessContext::next_event`
gives note events with their own sample offsets, so keep the same queue.

The events to handle, from `src/PluginProcessor.cpp`:

- note on and note off, with velocity.
- sustain pedal on and off. The C++ holds the released notes while the pedal is
  down and releases them when it comes up, and it removes a note from that list
  when the note is pressed again — the original's fix for a note pressed twice
  being released with the pedal.
- pitch wheel, which sets a bend *target* and glides to it over
  `BEND_GLIDE_MS` milliseconds.
- all notes off and all sound off, which clear the voices.

- [ ] **Step 3: Write the per-sample loop**

Port `processBlockByType` from `src/PluginProcessor.cpp` literally, preserving
the order of operations:

1. Interpolate the pitch bend by one sample.
2. Walk the MIDI queue, dispatching any message whose offset has reached zero
   and decrementing the offsets.
3. For the sample: sum the input channels into a mono signal.
4. For each voice: apply the pitch bend, run the fade-out, process the mallet,
   the noise, and the two resonators, accumulating the direct output, the
   resonator A output and the resonator B output.
5. Mix A and B according to the coupling mode.
6. Apply the gain, the comb stereoizer and the limiter.
7. Write the left and right samples.

Note that the JUCE build writes the output into the same buffer it read the
input from, so read the input for the sample **before** writing the output.

- [ ] **Step 4: Write the tests**

Add a `#[cfg(test)] mod tests` block asserting:

- `a_note_starts_a_voice` — that a note-on produces a non-silent output.
- `a_note_off_stops_the_voice` — that after the release time the output is silent.
- `the_sustain_pedal_holds_a_released_note` — that the note keeps sounding while the pedal is down, and stops when it comes up.
- `pressing_a_note_twice_while_the_pedal_is_down_does_not_release_it` — the original's fix.
- `all_notes_off_clears_the_voices`.
- `the_pitch_wheel_glides_rather_than_jumping` — that the bend reaches its target over the glide time.
- `the_output_is_stereo` — that the stereoizer produces different left and right samples.

These need the plugin to be drivable without a host, so give the plugin a
method that processes a buffer of audio with a given set of events, and drive it
from the tests.

- [ ] **Step 5: Run the tests**

Run: `cd rust && cargo test -p rippler-plugin`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/rippler-plugin/src/process.rs rust/crates/rippler-plugin/src/lib.rs
git commit -m "feat(plugin): port the per-sample processing loop"
```

---

### Task 3: The parameters and the voice bank

**Files:**
- Create: `rust/crates/rippler-plugin/src/params.rs`
- Modify: `rust/crates/rippler-plugin/src/lib.rs`

- [ ] **Step 1: Write the parameter application**

Create `rust/crates/rippler-plugin/src/params.rs`.

The JUCE build's `onSlider` reads every parameter and pushes it into the
voices. Port it:

- The noise generator's `init`.
- The mallet's pitch and the sampler's pitch factor.
- Each voice's pitch, ratio and coupling.
- Each resonator's parameters.
- Each voice's `update_resonators`.

Two pieces of logic need care:

- **The model change resets the ratio.** When `a_model` changes, the C++ sets
  `a_ratio` to a model-specific default — 2.0 for Beam, 1.0 for Djembe, 0.78
  otherwise — and clears the voices. The C++ writes the ratio back to the host
  parameter; the Rust plugin cannot write a host parameter from the audio
  thread, so **apply the default ratio internally** and let the editor mirror
  it into the host parameter. Document this divergence.
- **The model change clears the voices**, as does a change of the partial count
  or the mallet type.

Detect the changes by comparing against the last applied values, which is what
the C++ does with `last_a_model` and friends.

- [ ] **Step 2: Write the model table recalc**

When the ratio parameter changes, recompute the affected table with
`ModelTables::recalc` — on the audio thread, at the top of the block, before
the voices read it.

- [ ] **Step 3: Write the voice allocation**

Port `RipplerXAudioProcessor::pickVoice`:

1. If `reuse_voices` is on, a voice already playing the same note wins.
2. Otherwise, a released voice beats a pressed one.
3. Among released voices, the one released longest ago wins.
4. Among pressed voices, the one pressed longest ago wins.

The polyphony is 8 by default and up to 16, and the JUCE build stores it in a
settings file rather than a parameter.

- [ ] **Step 4: Write the tests**

Add tests asserting:

- `the_oldest_voice_is_reused` — that with all voices free, the first note takes the first voice.
- `a_released_voice_is_preferred_over_a_pressed_one`.
- `the_same_note_reuses_its_voice_when_reuse_voices_is_on`.
- `changing_the_model_resets_the_ratio` — that the ratio becomes the model default.
- `changing_the_model_clears_the_voices`.

- [ ] **Step 5: Run the tests**

Run: `cd rust && cargo test -p rippler-plugin`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/rippler-plugin/src/params.rs rust/crates/rippler-plugin/src/lib.rs
git commit -m "feat(plugin): apply the parameters and allocate the voices"
```

---

### Task 4: Microtuning

**Files:**
- Modify: `rust/crates/rippler-plugin/Cargo.toml`
- Create: `rust/crates/rippler-plugin/src/tuning.rs`

- [ ] **Step 1: Add the dependency**

Add `mts-client-rs` to `rippler-plugin`'s dependencies. Check the version that
resolves and adjust if it has moved on.

`mts-client-rs` loads the MTS-ESP library at runtime. It is optional at runtime:
if the library is not installed, the plugin must fall back to equal temperament.

- [ ] **Step 2: Write the tuning module**

Create `rust/crates/rippler-plugin/src/tuning.rs` with a function that turns a
note number into a frequency:

```rust
/// Turns a note number into a frequency in hertz.
///
/// Uses MTS-ESP when a tuning client is available, and equal temperament
/// otherwise.
pub fn note_to_frequency(note: i32, tuning: Option<&MtsClient>) -> f64;
```

The equal-temperament formula is the C++'s: `440 * 2^((note - 69) / 12)`.

The MTS-ESP path is the C++'s: `MTS_NoteToFrequency(mts, note, -1)`.

- [ ] **Step 3: Write the tests**

Add tests asserting:

- `equal_temperament_matches_the_cpp_formula` — that note 69 is 440 Hz, note 60 is 261.63 Hz, and note 81 is 880 Hz.
- `the_tuning_falls_back_when_mts_is_unavailable` — that the result is the equal-temperament one.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p rippler-plugin`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-plugin rust/Cargo.lock
git commit -m "feat(plugin): add MTS-ESP microtuning"
```

---

### Task 5: The exports

**Files:**
- Modify: `rust/crates/rippler-plugin/src/lib.rs`

- [ ] **Step 1: Implement the CLAP trait**

Implement `ClapPlugin`:

- `CLAP_ID` — `com.tilr.ripplerx`.
- `CLAP_DESCRIPTION` — a short description.
- `CLAP_MANUAL_URL` and `CLAP_SUPPORT_URL` — the project's URLs.
- `CLAP_FEATURES` — `ClapFeature::Instrument`, `ClapFeature::Stereo`, and
  `ClapFeature::Mono` if the plugin supports a mono output.

- [ ] **Step 2: Implement the VST3 trait**

Implement `Vst3Plugin`:

- `VST3_CLASS_ID` — the platform-dependent array from `identity.rs`.
- `VST3_SUBCATEGORIES` — `[Vst3SubCategory::Instrument]`.

- [ ] **Step 3: Export**

```rust
nice_export_clap!(RipplerX);
nice_export_vst3!(RipplerX);
```

- [ ] **Step 4: Build and check the bundle**

Run: `cd rust && cargo build --release`
Expected: it compiles.

Run: `cargo install cargo-nice-plug` if it is not installed, then
`cd rust && cargo nice-plug bundle --package rippler-plugin --release` — check
`cargo nice-plug --help` for the exact invocation.

Then inspect the bundle: the shared library must be named `RipplerX` inside a
`RipplerX.vst3` and a `RipplerX.clap`, and the VST3 bundle must contain a
`moduleinfo.json` whose class ID matches the platform's value.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-plugin/src/lib.rs
git commit -m "feat(plugin): export the VST3 and CLAP plugins"
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

- [ ] **Step 4: Release build**

Run: `cd rust && cargo build --release`
Expected: it compiles.

- [ ] **Step 5: Verify the JUCE tree is untouched**

```bash
git diff --stat <commit-before-this-plan>..HEAD -- src/ libs/ resources/ CMakeLists.txt .github/
```

Expected: no output.

## Notes for the next plan

- The plugin owns the parameters behind an `Arc`, so the editor can read and write
  them. The editor's parameter setter comes from its `GuiContext`.
- `rippler-params` provides the generated `apply_parameter!` and
  `with_parameter!` macros, so the editor can set a parameter by its legacy
  identifier without hand-writing setters.
- The plugin's `type Editor` is currently `()`. The editor plan replaces it with
  an egui editor.
- The plugin does not yet write its own state beyond what nice-plug's `Params`
  provides. The `currentProgram` and the user sample are extras the JUCE build
  kept in its state; if they are wanted, they belong in the `Params` struct as
  `#[persist]` fields.
- The VST3 class ID is platform-dependent. **Do not** "simplify" it to a single
  value — the Windows and non-Windows builds of the JUCE plugin genuinely differ.
- The sidechain input bus needs verifying in a real host. It is a main input bus
  on an instrument, which some hosts present differently.
