# RipplerX → Rust / nice-plug Migration — Design

**Date:** 2026-11-16
**Status:** Approved
**Target framework:** [nice-plug](https://codeberg.org/RustAudio/nice-plug) (Rust Audio, Codeberg)

## Summary

Port RipplerX from JUCE 8 / C++ to Rust on top of the nice-plug framework. The Rust
plugin ships **VST3 and CLAP** (the AU and LV2 builds are dropped, since nice-plug
cannot produce them). The DSP core is ported faithfully; the JUCE editor is
recreated in egui; parameter IDs are preserved verbatim so legacy patches remain
readable; MTS-ESP microtuning is retained.

The JUCE tree stays at the repository root, untouched and buildable, until the Rust
port is validated.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Plugin formats | VST3 + CLAP | nice-plug's native formats. AU and LV2 are dropped. |
| GUI framework | egui via `nice-plug-egui`, faithful re-creation | Keeps the bespoke look; the palette is already centralised. |
| Parameter compatibility | Keep legacy parameter IDs verbatim | Enables legacy XML import and identical knob feel. |
| Legacy state | Import via editor action + CLI converter | nice-plug owns the state codec; intercepting legacy blobs would require forking nice-plug. No fork. |
| Validation | Analytic unit tests | Fast CI, no binary fixtures, no JUCE dependency in the test loop. |
| Microtuning | Keep MTS-ESP via `mts-client-rs` | Preserves an advertised feature; FFI isolated behind a trait. |
| Repository layout | `rust/` subdirectory, JUCE tree retained | The old plugin stays the reference implementation until the port is validated. |
| Crate structure | Three-crate workspace | Isolates the DSP from the plugin framework; makes the compatibility surface auditable on its own. |

## 1. Shape of the port

```
rust/
├── Cargo.toml                    # workspace
└── crates/
    ├── rippler-dsp/              # pure DSP — no nice-plug, no egui, no framework at all
    │   └── src/
    │       ├── models.rs         # modal ratio + gain tables (was dsp/Models.*)
    │       ├── partial.rs        # 2-pole bandpass partial (was dsp/Partial.*)
    │       ├── resonator.rs      # 64-partial bank + waveguide (was dsp/Resonator.*)
    │       ├── voice.rs          # per-voice state, pitch/ratio/coupling (was dsp/Voice.*)
    │       ├── mallet.rs         # impulse + sampled mallet (was dsp/Mallet.*)
    │       ├── sampler.rs        # FLAC/WAV decode, cubic interp (was dsp/Sampler.*)
    │       ├── noise.rs          # noise + envelope (was dsp/Noise.*, dsp/Envelope.*)
    │       ├── filter.rs         # RBJ biquad (was dsp/Filter.*)
    │       ├── limiter.rs        # brickwall limiter (was dsp/Limiter.h)
    │       ├── comb.rs           # stereoizer (was dsp/Comb.h)
    │       ├── lut.rs            # LookupTable (was Utils.h)
    │       └── rng.rs            # per-voice xorshift PRNG
    ├── rippler-params/           # the compatibility surface
    │   ├── src/
    │   │   ├── params.rs         # #[derive(Params)] RipplerParams
    │   │   ├── ranges.rs         # JUCE NormalisableRange → nice-plug FloatRange
    │   │   ├── enums.rs          # MalletType, ModalModel, PartialCount, …
    │   │   ├── legacy.rs         # JUCE XML → plain param values
    │   │   └── bin/import.rs     # CLI: legacy XML → .ripx patch
    └── rippler-plugin/           # the plugin shell
        └── src/
            ├── lib.rs            # nice-plug Plugin impl, exports
            ├── midi.rs           # event decoding, sustain, pitch bend
            ├── voices.rs         # voice allocation
            └── ui/               # egui editor (knob.rs, tension.rs, meter.rs, …)
```

**Why three crates.** `rippler-dsp` compiles in about a second and has zero framework
dependencies, so the analytic test suite is fast and the DSP cannot reach for a plugin
API. `rippler-params` holds every compatibility decision — parameter IDs, skew mapping,
legacy XML — so that surface is auditable on its own. `rippler-plugin` is glue and owns
no DSP logic.

## 2. The parameter layer

All 77 parameters keep their **exact legacy IDs** (`a_decay`, `mallet_type`,
`noise_filter_mode`, …) as nice-plug `#[id]` values.

### 2.1 Range mapping

JUCE's `NormalisableRange` with skew `s > 0` normalizes as `p^(1/(1+s))`. nice-plug's
`FloatRange::Skewed` normalizes as `p^factor`. Therefore `factor = 1/(1+s)` reproduces
JUCE's skew exactly, and every skewed parameter in RipplerX uses a positive skew
(0.2, 0.3, 0.5, 1.0). Unskewed ranges map to `FloatRange::Linear`.

The knob feel, automation curves, and the host's generic editor therefore behave
identically to the JUCE build, and legacy XML values transfer without round-trip loss.

JUCE's `interval` (step size) maps to nice-plug's `with_step_size`.

### 2.2 Enums

Enums keep their JUCE index order so legacy state maps by index:

- `MalletType` — 26 entries, including the 10 `Reserved` slots (dropping them would
  shift every later index)
- `ModalModel` — 12 entries
- `PartialCount` — 7 entries, in the order `4, 8, 16, 32, 64, 1, 2`
- `NoiseFilterMode` — 3 entries
- `Coupling` — 2 entries

### 2.3 Grouping

Resonator A, Resonator B, Noise and Mallet become nested `Params` structs for host-side
grouping, using `#[nested(group_name = …)]` **without** `id_prefix`, so inner IDs stay
verbatim.

### 2.4 Safety nets

Both are unit tests:

- Every parameter ID hashes to a distinct value under nice-plug's `hash_param_id`
  (a 31-bit Java-style hash; a collision would silently merge two controls).
- Every parameter ID round-trips through the legacy XML reader to the same plain value
  the JUCE build produced.

## 3. DSP port

The port is a faithful translation with four deliberate departures, each fixing a latent
defect in the original:

| Original | Rust | Why |
|---|---|---|
| `Models::recalc*` mutates tables on the message thread while the audio thread reads them | `ModelTables` is immutable; beam/membrane/plate tables are recomputed into a fresh value on the audio thread when `ratio` changes | Removes a genuine data race |
| `Noise::process` calls global `std::rand()` | Per-voice xorshift64* PRNG, seeded from the voice index | Removes shared global state and the `rand()` lock |
| `Partial::a1LUT` / `sinLUT` are mutable function-local statics | `Lut` values owned by the plugin, passed by reference | No global mutable state |
| `Voice` holds `Models&`; `Mallet` holds `Sampler&` | `Models` passed as an argument at update time; `Sampler` owned by the plugin and shared via `Arc` | Removes the lifetime coupling |

Everything else is a line-by-line translation:

- Internal arithmetic stays `f64`, as in the JUCE build, converting only at the buffer
  boundary.
- `LookupTable`, `Filter`, `Limiter`, `Comb`, `Envelope` and `Waveguide` port directly.
- The `Waveguide`'s 20,000-sample tube is a preallocated buffer with no per-sample
  allocation.
- `Utils::normalToFreq` / `freqToNormal` and the dB helpers port as-is.

Two porting specifics:

- **Sampling.** `Sampler` uses JUCE's `AudioFormatManager` to decode FLAC/WAV/AIFF. The
  Rust equivalent is `symphonia`, which covers all three plus the user-importable
  formats. The 14 bundled mallet FLACs stay byte-identical and are embedded with
  `include_bytes!` so the bundle has no runtime file dependency. User-imported audio is
  mono-summed, truncated to `min(3 × sample_rate, len)`, and peak-normalised, as in the
  original.
- **Pitch bend.** The `BEND_GLIDE_MS` interpolation ramp and its `remainingSamplesBend`
  state machine carry over unchanged.

## 4. Plugin shell

`AudioIOLayout`: `main_input_channels: Some(2)`, `main_output_channels: Some(2)`, no aux
ports. `MIDI_INPUT: Basic`, `SAMPLE_ACCURATE_AUTOMATION: true`.

That input bus is RipplerX's "Audio In" feature, mirroring the JUCE build: a synth
component with a normal stereo input bus, which is what makes the feature work when
routing a track into the plugin in Reaper, Ableton and Bitwig. nice-plug's
`aux_input_ports` is a true VST3 sidechain bus — a different thing — and is not used.

*Verification item:* confirm the input bus is still exposed in Reaper and one CLAP host.
If a host balks at a main input on an instrument, the fallback is the aux-port path.

### 4.1 MIDI

`context.next_event()` decodes note on/off, sustain pedal (CC64), pitch bend,
all-notes-off and all-sound-off, with the same sustain-pedal semantics as the original —
including the original's fix where a note re-pressed while the pedal is down is removed
from the pedal's held-note list.

### 4.2 Parameter application

The JUCE build reads every parameter once per block via `onSlider()` and pushes values
into the voices. The port does the same: at the top of each block, a `BlockParams`
snapshot is read from the param atomics into plain fields, and the inner per-sample loop
touches only that snapshot. No atomic loads in the hot loop.

### 4.3 Side effects

Model changes reset `a_ratio`/`b_ratio` to a model-specific default (2.0 for Beam, 1.0
for Djembe, 0.78 otherwise). Model, partial-count and mallet changes clear the voices.

In JUCE this ran on the message thread and wrote the ratio back to the host. In Rust,
`FloatParam::with_callback` provides a value-change hook. The callback only sets an
atomic flag; the audio thread applies the effect at the next block boundary. This is
realtime-safe and fires for host automation exactly as the JUCE version did.

**Accepted divergence:** the ratio is applied *internally* rather than written back into
the host's parameter, so a host's generic editor may show a stale ratio until the user
touches it. The egui editor mirrors the value, so the plugin's own UI is always correct.

### 4.4 Voice allocation

Keeps the original priority order: same-note reuse when `reuse_voices` is on, then
released-before-pressed, then oldest release, then oldest press.

## 5. The editor

The JUCE editor is a 650×485 panel: a top bar (logo, theme toggle, velocity-mode button,
polyphony combo, preset menu, settings), then the Noise, Mallet, Resonator A, Resonator
B and Coupling sections laid out on a grid, with a MIDI keyboard along the bottom.

The egui version keeps that layout and the palette from `Globals.h`
(light `#eeeeee` / `#333333` / `#008070` / `#aa5050`; dark `#424242` / `#eeeeee` /
`#00ccab` / `#d76464`).

Custom widgets to build:

- `Rotary` — the arc knob, custom-painted, drag-to-turn with shift for fine control
- `TensionCtrl` — the envelope-tension control
- `Meter` — the stereo output meter, fed by an `AtomicF32` from the audio thread with a
  `RepaintNotifier`
- `Pitch` — the coarse/fine pitch control
- `Keyboard` — the MIDI keyboard strip, sending note events to the host
- on/off toggle buttons (A/B enable), the sun/moon theme toggle, and the logo

Menu-backed parameters (model, partials, mallet type, noise filter mode, coupling)
become egui combo boxes. The settings menu carries polyphony, the velocity-edit-mode
toggle, UI scale, patch import/export, audio import, and About.

**Persistence of editor-level settings.** Polyphony, UI scale and the theme are *not*
plugin parameters in the JUCE build — they live in a global JUCE `PropertiesFile` shared
across plugin instances. In the port they become `#[persist]` fields on the params
struct, so they travel with the host's saved state and are per-instance. This is a
deliberate behaviour change: the JUCE build's global settings file is not reproduced.

Patch files keep the `.ripx` extension. Export writes the parameter snapshot plus, for a
user mallet, the base64-encoded waveform — matching what the JUCE build stores.

## 6. State and legacy import

nice-plug owns the state codec: the VST3 wrapper parses the host's blob as JSON
(or zstd-compressed JSON) before the plugin sees it, and `Plugin::filter_state` is the
plugin's migration hook for JSON→JSON changes. The Rust plugin's own state is a
nice-plug `PluginState`.

Legacy JUCE XML cannot be intercepted there, so it is handled by an explicit import
path:

- **`rippler-params` exposes a legacy reader** that parses the JUCE `ValueTree` XML,
  walks each `<PARAMETER id=… value=…/>` child, converts JUCE's normalized value to a
  plain value using JUCE's exact skew formula, and looks up the matching parameter by
  legacy ID.
- **The editor gets an "Import legacy patch (.xml)" action** that runs that reader and
  applies the values through the editor's param setter.
- **A CLI, `rippler-import`**, converts legacy XML files — including the 28 factory
  preset files in `resources/presets/` (`Init.xml` plus 27 presets) — into the new
  `.ripx` format, so the presets ship in the new format from day one.

**Accepted limitation:** legacy *sessions* do not auto-restore. Users recover patches
through the editor import action or the CLI converter. This is the consequence of not
forking nice-plug to add a pluggable state codec.

Two wrinkles inherited from the original are reproduced by the legacy reader:

- The `a_cut`/`b_cut` parameters were re-normalised at some point; the JUCE build
  migrates old values on load, and so does the reader.
- The `userSample` property in old state — a base64 blob of raw doubles — is decoded by
  the reader and stored into the new `#[persist]` field.

## 7. Microtuning

MTS-ESP stays, via the `mts-client-rs` crate (a Rust port of the MTS-ESP client). The
`Voice::note2freq` call becomes a call into that crate, behind a small `TuningSource`
trait, so `rippler-dsp` stays free of FFI and the MTS dependency can be compiled out.

## 8. Testing

Analytic tests, living in `rippler-dsp` and `rippler-params`, asserting against theory
rather than golden files:

- Modal partial frequencies land exactly on the model's ratio table, for each of the 12
  models
- Decay time scales as specified for the `a_decay`/`b_decay` mapping
- Envelope stages follow attack → decay → sustain → release, with tension reshaping the
  curve
- Biquads hit their nominal −3 dB point and Q
- The limiter bounds output at the threshold
- Comb delay length matches its 20 ms specification
- Inharmonicity raises `f_k` by the documented factor
- Every parameter ID is collision-free under `hash_param_id`
- Every legacy XML fixture maps to the expected plain parameter values

## 9. Build and release

`cargo-nice-plug` for bundling, `cargo xtask` as the alternative. The existing
`.github/workflows/release.yml` gains a Rust job; the CMake job stays until the port is
validated, then the JUCE tree and `CMakeLists.txt` are removed in a final commit.

The **VST3 class ID must be preserved** so hosts treat the Rust build as the same plugin
rather than a missing plugin in old sessions. The current ID is read from a reference
build of the JUCE plugin and hard-coded in the `Vst3Plugin` impl. The CLAP ID stays
`com.tilr.ripplerx`.

## 10. Risks

| Risk | Mitigation |
|---|---|
| The VST3 class ID isn't recovered correctly | Read it from a reference build of the current plugin; verify byte-for-byte |
| egui's widget set can't reproduce the JUCE look | Custom-paint the knob, tension and meter widgets; the palette is already centralised in `Globals.h` |
| `symphonia` decodes the bundled FLACs differently than JUCE | Assert decoded length and peak normalisation in a test against known values |
| The sidechain input bus isn't exposed as a main input in some hosts | Verify in Reaper and a CLAP host early; the aux-port path is the fallback |
| 77 parameters with skewed ranges drift from the JUCE feel | The skew equivalence is exact, and a test asserts it for every parameter |
| Legacy sessions do not restore | Accepted: editor import action plus CLI converter |

## 11. Out of scope

- AU and LV2 builds
- A patchable nice-plug fork, or any change to nice-plug itself
- Golden-file or offline A/B parity testing against the JUCE build
- The drag-and-drop modulation system, dual-layer architecture, and preset browser that
  the commercial RipplerX has
