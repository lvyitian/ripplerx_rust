# Rust Editor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recreate RipplerX's JUCE editor in egui — the same layout, the same custom-painted widgets, the same palette — so the Rust plugin looks and feels like the plugin users already know.

**Architecture:** An egui editor inside `rippler-plugin`, built on `nice-plug-egui`. The custom JUCE widgets — the rotary knob, the tension control, the meter and the pitch control — become egui widgets that paint themselves.

**Tech Stack:** Rust 2024, `nice-plug-egui`, `egui`.

**Design doc:** `docs/superpowers/specs/2026-11-16-rust-nice-plug-migration-design.md`

---

## Position in the roadmap

| # | Plan | Status |
|---|---|---|
| 1 | Rust workspace and DSP primitives | Done |
| 2 | Modal core | Done |
| 3 | Excitation and voice | Done |
| 4 | Parameters and compatibility | Done |
| 5 | Plugin shell | Done |
| 6 | **This plan — editor** | Current |

## What already exists

- `rippler-dsp` — sixteen DSP modules.
- `rippler-params` — the 77 parameters, plus the generated `apply_parameter!` and `with_parameter!` macros that set a parameter by its legacy identifier.
- `rippler-plugin` — the plugin shell, with `type Editor = ();` waiting to be replaced.

The plugin owns the parameters behind an `Arc<RipplerParams>`, so the editor can read them, and the editor's `GuiContext` gives it a `ParamSetter` for writing them.

## Reference material

- `src/PluginEditor.{h,cpp}` — the layout, the top bar and the sections.
- `src/ui/Rotary.{h,cpp}` — the knob.
- `src/ui/TensionCtrl.{h,cpp}` — the tension control.
- `src/ui/Meter.{h,cpp}` — the meter.
- `src/ui/Pitch.{h,cpp}` — the pitch control.
- `src/ui/CustomLookAndFeel.{h,cpp}` — the shared drawing helpers.
- `src/Globals.h` — the palette.
- `resources/` — the logo, the theme icons and the font.

A clone of the nice-plug source is in the temp directory under a folder named
`nice-plug`. Its `examples/gain_egui` is the reference for the editor plumbing,
and `crates/nice-plug-egui` for the widget helpers.

---

## The palette

From `src/Globals.h`. The editor has a light and a dark variant.

| Role | Light | Dark |
|---|---|---|
| Background | `#eeeeee` | `#424242` |
| Neutral | `#333333` | `#eeeeee` |
| Neutral light | `#666666` | `#b1b1b1` |
| Active | `#008070` | `#00ccab` |
| Velocity | `#aa5050` | `#d76464` |

## The layout

The JUCE editor is 650 × 485. It has a top bar and then the sections, laid out
on a grid:

- **Top bar** — logo, the sun/moon theme toggle, the velocity-edit-mode button,
  the polyphony combo, the preset menu, and the settings button.
- **Noise** — a filter-mode combo; the `noise_osc` slider; and a 2 × 2 grid of
  knobs (`noise_mix`, `noise_res`, `noise_freq`, `noise_q`) beside a 2 × 2 grid
  of envelope knobs (`noise_a`, `noise_s`, `noise_d`, `noise_r`), each with a
  tension control beside it.
- **Mallet** — the mallet button, and knobs for `mallet_mix`, `mallet_res`,
  `mallet_stiff`, `mallet_pitch` and `mallet_filter`, plus the `mallet_ktrack`
  slider.
- **Resonator A** and **Resonator B** — each with an on/off button, a model
  combo, a partials combo, and knobs for decay, damp, tone, hit, release,
  inharmonicity, ratio, cutoff and radius.
- **Coupling** — the coupling combo, the `ab_mix` and `ab_split` knobs, the two
  pitch controls, the bend control, the gain knob and the meter.
- **The keyboard** — a MIDI keyboard along the bottom.

The knob's grid positions are in `PluginEditor.cpp`'s constructor. Reproduce the
arrangement, not the pixel positions — egui lays out with a grid.

---

## Decisions taken while implementing

- **The theme, the UI scale and the polyphony live in a JSON settings file** in
  the user's config directory, mirroring the JUCE build's
  `loadSettings`/`saveSettings`. They are **not** `#[persist]` fields on the
  parameter struct, because `#[persist]` fields cannot be written through
  `ParamSetter`, so the editor could not toggle the theme. The polyphony is
  shared with the audio thread through an `Arc<AtomicUsize>`, since the audio
  thread must not take a lock.
- **The images are embedded** with `include_bytes!` and decoded with the `image`
  crate, and the Ubuntu font is embedded and installed as egui's proportional
  font. Drawing substitutes would have looked wrong.
- **The MIDI keyboard is left out.** nice-plug offers no way for an editor to
  send a note to the host, so the host's own keyboard is used instead.
- **The file dialogs use `rfd`.** Note that `rfd` needs Rust 1.92, so the
  workspace's declared MSRV of 1.88 is optimistic.
- **The knob's mouse area is the circle** rather than the whole 70 × 75 cell, so
  that the tension control in the corner is not swallowed by the knob's drag.

---

### Task 1: The editor plumbing

**Files:**
- Modify: `rust/crates/rippler-plugin/Cargo.toml`
- Create: `rust/crates/rippler-plugin/src/editor/mod.rs`
- Create: `rust/crates/rippler-plugin/src/editor/theme.rs`
- Modify: `rust/crates/rippler-plugin/src/lib.rs`

- [ ] **Step 1: Add the dependency**

Add `nice-plug-egui` to `rippler-plugin`'s dependencies, with the `egui`
feature if the crate gates it that way — check the crate's manifest.

- [ ] **Step 2: Write the theme**

Create `rust/crates/rippler-plugin/src/editor/theme.rs` with the palette as
constants and a small struct holding the six colours for the current theme, plus
a function to switch between light and dark.

The theme choice is **not** a plugin parameter in the JUCE build — it lives in
the settings file, along with the UI scale and the polyphony. Decide where
these live in the Rust build and document it. The simplest is `#[persist]`
fields on the parameter struct, so they travel with the host's state.

- [ ] **Step 3: Write the editor state**

Create `rust/crates/rippler-plugin/src/editor/mod.rs` with an `EditorState`
holding everything the editor needs:

- the `Arc<RipplerParams>`,
- the theme,
- the UI scale,
- the polyphony,
- the velocity-edit-mode flag,
- the meter value, shared with the audio thread through an `Arc<AtomicF32>`.

- [ ] **Step 4: Implement the editor**

Implement `nice_plug_egui::EguiEditor` for the plugin, following the
`examples/gain_egui` example:

- `EguiEditorState` with the initial size and zoom factor.
- `RepaintNotifier` for the meter.
- `create_egui_editor(...)` in the plugin's `editor()` method.
- An `EditorApp` implementing `NiceEguiApp`, with `build`, `ui` and
  `editor_closed`.

Replace `type Editor = ();` with the egui editor in `lib.rs`.

For now the `ui` method can draw a placeholder, so the plumbing is verified
before the widgets are built.

- [ ] **Step 5: Run the tests**

Run: `cd rust && cargo test -p rippler-plugin`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/rippler-plugin
git commit -m "feat(editor): wire up the egui editor"
```

---

### Task 2: The rotary knob

**Files:**
- Create: `rust/crates/rippler-plugin/src/editor/knob.rs`

- [ ] **Step 1: Write the widget**

Create `knob.rs`. It is an `egui::Widget` that paints itself and handles the
drag, the wheel and the double-click.

The drawing, from `src/ui/Rotary.cpp`:

- A filled circle of radius 16 in the background colour, with a drop shadow.
- An arc in the active colour, sweeping from −130° to +130°, drawn only when
  the normalised value is not zero — or, for a symmetric control, not exactly
  0.5.
- A pointer line from the centre to the radius, in the neutral colour, rotated
  to the value's angle.
- When the velocity-edit mode is on and the knob has an associated velocity
  parameter, a filled circle in the velocity colour at half alpha.
- A label below: the parameter's name normally, and its formatted value while
  the mouse is down.

The arc sweep is 260°, from −130° to +130°.

The velocity arc, drawn when the knob has a velocity parameter, is a second arc
at a larger radius, in the velocity colour, running from the value's angle to
the angle the velocity parameter would move it to, clamped to the sweep.

- [ ] **Step 2: Write the value formats**

Port `LabelFormat` from `src/ui/Rotary.h` and its formatting from
`Rotary::draw_label`:

| Format | Output |
|---|---|
| `Hz` | `"1234 Hz"`, or `"1.2 kHz"` from 1000 Hz up |
| `dB` | `"6 dB"` |
| `Percent` | `"50 %"` |
| `float1` | `"1.2"` |
| `float2_100` | the value times a hundred, to two decimals |
| `millis` | `"500 ms"`, or `"1.20 s"` from 1000 ms up |
| `seconds2f` | `"1.20 s"` |
| `ABMix` | `"50:50"`, with the first number the complement |
| `FilterLPHP` | `"Off"` at zero, otherwise `"LP 1.2 kHz"` or `"HP 500 Hz"` |
| `PitchSemis` | `"12 Sem"` |

The `FilterLPHP` format computes the frequency the same way the plugin does:
`20 * 1000^(cut < 0 ? 1 + cut : cut)`.

- [ ] **Step 3: Write the interaction**

From `Rotary`'s mouse handling:

- **Drag**: the change is the vertical *and* horizontal mouse movement,
  `change.x - change.y`, divided by a speed that is four times the pixels per
  percent, or forty with the control key held. The value is set as a
  normalised value.
- **Pitch controls snap** to whole semitones unless the control key is held.
- **Shift**, or the velocity-edit mode, edits the associated velocity parameter
  instead of the main one.
- **Double-click** resets the parameter to its default.
- **Wheel** steps the parameter, with a step of 0.01 normalised with the
  control key and 0.05 without. The C++ has a fix for the wheel not working
  when the value is zero, where the first step takes more than 0.05 % — port
  that fix.

When the mouse is held, the cursor is hidden and the mouse position is restored
on release, so the knob can be turned past the window edge. egui's
`Response` and the drag behaviour may make that unnecessary — if so, say so in
a comment rather than emulating it.

- [ ] **Step 4: Write the tests**

Add tests asserting:

- `the_arc_sweeps_two_hundred_and_sixty_degrees` — that the value at the ends of the sweep is at −130° and +130°.
- `the_value_formats_match_the_cpp` — one case per format.
- `the_filter_format_reports_the_frequency` — that a cutoff of 0.5 gives "HP 632 Hz", since `20 * 1000^0.5` is 632.5.
- `the_ab_mix_format_is_complementary`.

These are pure functions, so test them without a window.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/rippler-plugin/src/editor/knob.rs
git commit -m "feat(editor): recreate the rotary knob"
```

---

### Task 3: The tension control

**Files:**
- Create: `rust/crates/rippler-plugin/src/editor/tension.rs`

- [ ] **Step 1: Write the widget**

Create `tension.rs`, an egui widget.

From `src/ui/TensionCtrl.cpp`: a rectangle with a black background at ten per
cent alpha, and a **quadratic Bézier** from one corner to the opposite corner.
The control point is placed at the parameter's value, normalised from −1…1 to
0…1:

- Not inverted: the curve runs from the bottom-left to the top-right, with the
  control point at `(left + width * v, top + height * v)`.
- Inverted: the curve runs from the top-left to the bottom-right, with the
  control point at `(left + width * (1 - v), bottom - height * (1 - v))`.

  Check the direction against the C++ rather than assuming: the inverted case
  moves the control point the other way.

The stroke is one pixel wide, in the neutral-light colour.

The interaction is a vertical drag: the change is `change.x - change.y`, divided
by the same speed the knob uses, and **subtracted** from the value. A
double-click resets to the default.

- [ ] **Step 2: Write the tests**

Add tests asserting:

- `the_control_point_follows_the_value` — that a value of 0 places the control point at the top-left of the curve's span and a value of 1 at the bottom-right, for both the inverted and the plain case.
- `the_value_is_normalised_from_minus_one_to_one` — that a parameter value of −1 maps to 0 and 1 maps to 1.

- [ ] **Step 3: Commit**

```bash
git add rust/crates/rippler-plugin/src/editor/tension.rs
git commit -m "feat(editor): recreate the tension control"
```

---

### Task 4: The meter and the pitch control

**Files:**
- Create: `rust/crates/rippler-plugin/src/editor/meter.rs`
- Create: `rust/crates/rippler-plugin/src/editor/pitch.rs`

- [ ] **Step 1: Write the meter**

Create `meter.rs`.

The JUCE meter is a row of four **bulbs** per channel, driven by a timer that
reads the processor's RMS value. In this port the value comes from the plugin
through an `Arc<AtomicF32>`, and the editor asks for a repaint when it changes.

Read `src/ui/Meter.cpp` for how the bulbs light up — the level thresholds and
the colours — and reproduce them.

**The lit bulbs use a green → yellow → red gradient**, not the active and
neutral-light colours, and the unlit bulbs are dark grey and black. The plan's
first draft had this wrong; the C++ wins.

The plugin already computes an RMS value; expose it through the shared atomic
rather than computing it in the editor.

- [ ] **Step 2: Write the pitch control**

Create `pitch.rs`.

From `src/ui/Pitch.{h,cpp}`: the control edits the **coarse** pitch parameter by
default and the **fine** one when the coarse value is at its limit, and it
snaps the coarse value to whole semitones. It shows the current pitch in
semitones.

**The C++ switches between coarse and fine by which half of the control you
click**, not by reaching the coarse limit. Read the C++ for the exact rule and
reproduce it.

- [ ] **Step 3: Write the tests**

Add tests asserting:

- `the_meter_lights_the_right_number_of_bulbs` — for a given level, the number of lit bulbs.
- `the_pitch_control_switches_to_the_fine_parameter_at_the_coarse_limit`.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-plugin/src/editor/meter.rs rust/crates/rippler-plugin/src/editor/pitch.rs
git commit -m "feat(editor): recreate the meter and the pitch control"
```

---

### Task 5: The layout

**Files:**
- Create: `rust/crates/rippler-plugin/src/editor/panels.rs`
- Modify: `rust/crates/rippler-plugin/src/editor/mod.rs`

- [ ] **Step 1: Write the parameter widgets**

Create a small set of helpers that bind a widget to a parameter:

- a knob bound to a parameter identifier, with a name and a format,
- a combo box bound to an enum parameter,
- a slider bound to a parameter.

They use `rippler_params`' generated `apply_parameter!` and `with_parameter!`
macros, so no identifier is ever written out by hand in two places.

The setter comes from the editor's `GuiContext`: `param_setter()` gives a
`ParamSetter` with `begin_set_parameter`, `set_parameter` and
`end_set_parameter`.

Note the C++'s behaviour: a drag begins a change gesture and ends it on
release, so the host records one automation point rather than a stream of them.
Do the same.

- [ ] **Step 2: Write the sections**

Create `panels.rs` with one function per section, each laying out its widgets
in an egui grid:

- the noise section, with the filter-mode combo, the noise-osc slider, the
  eight knobs in their two grids and the three tension controls,
- the mallet section, with the mallet button and its five knobs and the
  key-tracking slider,
- the resonator sections, one function used for both A and B, taking the
  parameter prefix so the same code serves both,
- the coupling section, with the coupling combo, the mix and split knobs, the
  two pitch controls, the bend control, the gain knob and the meter.

The resonator sections differ only in their prefix, so parameterise them.

- [ ] **Step 3: Write the top bar**

The logo, the theme toggle, the velocity-edit-mode button, the polyphony
combo, the preset menu and the settings button.

The images are in `resources/`. Embed them with `include_bytes!` the way the
mallet samples are, and decode them with the `image` crate, or draw simple
substitutes if a decoder is not worth the dependency. **Decide and document
which.**

- [ ] **Step 4: Write the keyboard**

The JUCE editor has a MIDI keyboard along the bottom, which sends note events
to the host.

Check what nice-plug's editor API offers for sending MIDI to the host — the
`EditorHandle` may have a method for it. If there is none, say so plainly and
leave the keyboard out, rather than inventing a mechanism that does not work.

- [ ] **Step 5: Write the tests**

egui widgets need a context to test. Use `egui::Context::run` with a headless
harness to check that the layout builds without panicking, and that the
sections contain the expected number of widgets.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/rippler-plugin/src/editor
git commit -m "feat(editor): lay out the plugin's panels"
```

---

### Task 6: The menus

**Files:**
- Create: `rust/crates/rippler-plugin/src/editor/menus.rs`
- Modify: `rust/crates/rippler-plugin/src/editor/mod.rs`

- [ ] **Step 1: Write the preset menu**

The JUCE editor's preset menu lists the twenty-eight factory presets and
switches between them. In this port the presets are `.ripx` files, and the
twenty-eight legacy XML presets convert to them with `rippler-import`.

Decide how the presets are delivered: embedded in the binary, or read from a
folder next to the plugin. Embedded is simpler and matches how the mallet
samples are handled; a folder is what a user would expect to be able to edit.

Whichever is chosen, the menu must be able to load a preset by setting every
parameter it names, through the `apply_parameter!` macro.

- [ ] **Step 2: Write the settings menu**

The JUCE editor's settings menu carries the polyphony, the velocity-edit-mode
toggle, the UI scale, patch import and export, audio import, and the about
box.

Implement each:

- **Polyphony** — a combo, 1 to 16.
- **Velocity mode** — a toggle.
- **UI scale** — a zoom factor, which egui applies with `set_zoom_factor`.
- **Export patch** — write a `.ripx` file. Use a file dialog; `rfd` is the
  usual choice on the desktop, and it is a new dependency. Alternatively write
  to a known folder and tell the user where.
- **Import patch** — read a `.ripx` file and apply it.
- **Import legacy patch** — read a JUCE XML file through
  `rippler_params::legacy::read_legacy_patch` and apply it. This is the path
  that makes old patches loadable, and it is the reason Plan 4 exists.
- **Import audio** — load a user mallet sample, through `rippler-dsp`'s
  `Sampler::load_sample`.
- **About** — the plugin's name, version and the project URL.

- [ ] **Step 3: Write the tests**

Add tests asserting:

- `a_legacy_patch_applies_its_values` — that the values from a legacy XML reach the parameters.
- `a_ripx_patch_round_trips_through_the_editor` — that writing and re-reading a patch gives the same values.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/rippler-plugin/src/editor
git commit -m "feat(editor): add the preset and settings menus"
```

---

### Task 7: Completion gates

- [ ] **Step 1: Run the whole test suite**

Run: `cd rust && cargo test`
Expected: every test passes.

- [ ] **Step 2: Check for warnings**

Run: `cd rust && cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: Check formatting**

Run: `cd rust && cargo fmt --check`
Expected: no diff.

- [ ] **Step 4: Build and bundle**

Run: `cd rust && cargo build --release && cargo nice-plug bundle -p rippler-plugin --release`
Expected: the bundles build, and the VST3 class ID is unchanged.

- [ ] **Step 5: Verify the JUCE tree is untouched**

```bash
git diff --stat <commit-before-this-plan>..HEAD -- src/ libs/ resources/ CMakeLists.txt .github/
```

Expected: no output.

- [ ] **Step 6: Look at the editor**

Open the bundled plugin in a host, or run the standalone build, and compare the
editor against the JUCE build's screenshot in `doc/ripplerx.png`. The layout,
the colours and the knob drawing should match.

## Notes for the next plan

There is no next plan. When this one is done, the port is complete and the
remaining work is:

- Removing the JUCE tree and `CMakeLists.txt`, and dropping the AU and LV2
  builds, once the Rust plugin is accepted.
- Updating the README to describe the Rust plugin, the CLAP build and the loss
  of the AU and LV2 formats.
- Adding the CI job that builds the Rust plugin and uploads the bundles.
- Deciding whether the legacy JUCE session format — a **binary** blob, not the
  plain XML the presets use — is worth a reader. The editor's import action
  reads the plain XML only.
