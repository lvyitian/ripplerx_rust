//! The menus of the editor.
//!
//! Port of the preset menu and of the settings menu of `src/PluginEditor.cpp`.
//!
//! The factory presets are the legacy XML files of the `resources` folder.
//! They are embedded in the plugin and are converted to patches when they are
//! loaded, so that there is only one copy of every preset.
//!
//! The patches the user writes are `.ripx` files, which are the JSON
//! documents of `rippler_params::patch`. The legacy XML patches are read
//! through `rippler_params::legacy`, which is the path that makes the old
//! patches loadable.

use egui::{Response, Ui};
use rippler_dsp::mallet::MalletType;
use rippler_params::legacy::read_legacy_patch;
use rippler_params::patch::{PATCH_VERSION, Patch, convert_legacy_patch, read_patch, write_patch};
use std::sync::atomic::Ordering;

use crate::editor::panels::Panels;
use crate::editor::settings::UI_SCALES;

/// The factory presets, in the order the JUCE editor shows them in.
static PRESETS: [(&str, &[u8]); 28] = [
    (
        "Init",
        include_bytes!("../../../../../resources/presets/Init.xml"),
    ),
    (
        "Harpsi",
        include_bytes!("../../../../../resources/presets/Harpsi.xml"),
    ),
    (
        "Harp",
        include_bytes!("../../../../../resources/presets/Harp.xml"),
    ),
    (
        "Sankyo",
        include_bytes!("../../../../../resources/presets/Sankyo.xml"),
    ),
    (
        "Tubes",
        include_bytes!("../../../../../resources/presets/Tubes.xml"),
    ),
    (
        "Stars",
        include_bytes!("../../../../../resources/presets/Stars.xml"),
    ),
    (
        "DoorBell",
        include_bytes!("../../../../../resources/presets/DoorBell.xml"),
    ),
    (
        "Bells",
        include_bytes!("../../../../../resources/presets/Bells.xml"),
    ),
    (
        "Bells2",
        include_bytes!("../../../../../resources/presets/Bells2.xml"),
    ),
    (
        "KeyRing",
        include_bytes!("../../../../../resources/presets/KeyRing.xml"),
    ),
    (
        "Sink",
        include_bytes!("../../../../../resources/presets/Sink.xml"),
    ),
    (
        "Cans",
        include_bytes!("../../../../../resources/presets/Cans.xml"),
    ),
    (
        "Gong",
        include_bytes!("../../../../../resources/presets/Gong.xml"),
    ),
    (
        "Bong",
        include_bytes!("../../../../../resources/presets/Bong.xml"),
    ),
    (
        "Marimba",
        include_bytes!("../../../../../resources/presets/Marimba.xml"),
    ),
    (
        "Fight",
        include_bytes!("../../../../../resources/presets/Fight.xml"),
    ),
    (
        "Tabla",
        include_bytes!("../../../../../resources/presets/Tabla.xml"),
    ),
    (
        "Tabla2",
        include_bytes!("../../../../../resources/presets/Tabla2.xml"),
    ),
    (
        "Strings",
        include_bytes!("../../../../../resources/presets/Strings.xml"),
    ),
    (
        "OldClock",
        include_bytes!("../../../../../resources/presets/OldClock.xml"),
    ),
    (
        "Crystal",
        include_bytes!("../../../../../resources/presets/Crystal.xml"),
    ),
    (
        "Ride",
        include_bytes!("../../../../../resources/presets/Ride.xml"),
    ),
    (
        "Ride2",
        include_bytes!("../../../../../resources/presets/Ride2.xml"),
    ),
    (
        "Crash",
        include_bytes!("../../../../../resources/presets/Crash.xml"),
    ),
    (
        "Vibes",
        include_bytes!("../../../../../resources/presets/Vibes.xml"),
    ),
    (
        "Flute",
        include_bytes!("../../../../../resources/presets/Flute.xml"),
    ),
    (
        "Fifths",
        include_bytes!("../../../../../resources/presets/Fifths.xml"),
    ),
    (
        "Kalimba",
        include_bytes!("../../../../../resources/presets/Kalimba.xml"),
    ),
];

/// The name the patch menu shows.
///
/// The JUCE editor's patch menu shows the name of the program it is on, and
/// "Patch" when it is not on one of the factory presets.
fn program_name(program: i32) -> &'static str {
    usize::try_from(program)
        .ok()
        .and_then(|program| PRESETS.get(program))
        .map(|(name, _)| *name)
        .unwrap_or("Patch")
}

/// The patch menu.
///
/// The JUCE editor's patch menu is a combo box with the twenty-eight
/// factory presets in it, and the import and the export at the top. It
/// shows the name of the program the editor is on.
pub(crate) fn preset_menu(ui: &mut Ui, panels: &Panels) {
    // The number of the entry that was picked, if any.
    let mut action: Option<usize> = None;

    // The JUCE editor's patch menu shows the name of the program it is on,
    // and "Patch" when it is not on one of the factory presets.
    let program = panels.current_program.load(Ordering::Relaxed);

    egui::ComboBox::from_id_salt("presets")
        .selected_text(program_name(program))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut action, Some(0), "Import");
            ui.selectable_value(&mut action, Some(1), "Export");
            ui.separator();
            for (index, (name, _)) in PRESETS.iter().enumerate() {
                ui.selectable_value(&mut action, Some(index + 2), *name);
            }
        });

    match action {
        Some(0) => import_patch(panels),
        Some(1) => export_patch(panels),
        Some(index) => load_preset(panels, index - 2),
        None => {}
    }
}

/// The mallet menu.
///
/// The JUCE editor's mallet menu is opened by the label of the mallet
/// section, and it holds the bundled samples and the user's file.
pub(crate) fn mallet_menu(ui: &mut Ui, panels: &Panels, response: &Response) {
    let _ = ui;
    egui::Popup::menu(response).show(|ui| {
        // The number of the mallet that was picked, if any.
        let mut action: Option<usize> = None;

        ui.selectable_value(&mut action, Some(MalletType::Impulse as usize), "Impulse");
        ui.separator();
        ui.selectable_value(&mut action, Some(12), "Click 1");
        ui.selectable_value(&mut action, Some(13), "Click 2");
        ui.selectable_value(&mut action, Some(14), "Click 3");
        ui.selectable_value(&mut action, Some(23), "Strike");
        ui.selectable_value(&mut action, Some(22), "Wood");
        ui.selectable_value(&mut action, Some(20), "Metal 1");
        ui.selectable_value(&mut action, Some(21), "Metal 2");
        ui.selectable_value(&mut action, Some(24), "Perc 1");
        ui.selectable_value(&mut action, Some(25), "Perc 2");
        ui.selectable_value(&mut action, Some(18), "Blip");
        ui.selectable_value(&mut action, Some(19), "Blop");
        ui.separator();
        if ui.button("Load File").clicked() {
            import_audio(panels);
        }

        if let Some(index) = action {
            panels.params.set_legacy("mallet_type", index as f64);
        }
    });
}

/// The settings menu.
///
/// The JUCE editor's settings menu carries the polyphony, the velocity
/// mode, the scale of the user interface, the import and the export of
/// patches and the about box.
pub(crate) fn settings_menu(ui: &mut Ui, panels: &Panels, response: &Response) {
    let _ = ui;
    egui::Popup::menu(response).show(|ui| {
        ui.label("UI Scale");
        let scale = panels
            .settings
            .lock()
            .expect("the settings are not poisoned")
            .ui_scale;
        for value in UI_SCALES {
            let selected = (scale - value).abs() < 0.001;
            if ui
                .selectable_label(selected, format!("{}%", (value * 100.0).round() as i32))
                .clicked()
            {
                set_scale(ui, panels, value);
            }
        }

        ui.separator();
        ui.label("Polyphony");
        let polyphony = panels.polyphony.load(std::sync::atomic::Ordering::Relaxed);
        for value in 1..=16 {
            if ui
                .selectable_label(polyphony == value, value.to_string())
                .clicked()
            {
                set_polyphony(ui, panels, value);
            }
        }

        ui.separator();
        let velocity = panels.velocity.load(std::sync::atomic::Ordering::Relaxed);
        if ui.selectable_label(velocity, "Velocity mapping").clicked() {
            panels
                .velocity
                .store(!velocity, std::sync::atomic::Ordering::Relaxed);
            ui.ctx().request_repaint();
        }

        // The JUCE editor's settings menu carries these three parameters as
        // well.
        ui.separator();
        toggle(ui, panels, "stereoizer", "Stereoizer");
        toggle(ui, panels, "reuse_voices", "Reuse voices on repeated notes");
        toggle(ui, panels, "fadeout_repeats", "Fadeout repeated notes");

        ui.separator();
        if ui.button("Import patch").clicked() {
            import_patch(panels);
        }
        if ui.button("Import legacy patch").clicked() {
            import_legacy_patch(panels);
        }
        if ui.button("Export patch").clicked() {
            export_patch(panels);
        }
        if ui.button("Import audio").clicked() {
            import_audio(panels);
        }

        ui.separator();
        let about_id = ui.make_persistent_id("about");
        if ui.button("About").clicked() {
            ui.memory_mut(|memory| memory.data.insert_temp(about_id, true));
        }
        let about = ui
            .memory(|memory| memory.data.get_temp::<bool>(about_id))
            .unwrap_or(false);
        if about {
            ui.separator();
            ui.label("RipplerX");
            ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
            ui.label("Copyright (C) Tilr 2025");
            ui.label("github.com/lvyitian/ripplerx_rust");
            ui.label("- Ctrl for fine slider adjustments.");
            ui.label("- Shift to adjust velocity mapping.");
            ui.label("- Double click to reset sliders.");
            ui.label("- Rippler supports Audio In:");
            ui.label("  Play the audio and play notes, the sound should");
            ui.label("  excite the resonators.");
        }
    });
}

/// Loads one of the factory presets.
fn load_preset(panels: &Panels, index: usize) {
    let Some((_, bytes)) = PRESETS.get(index) else {
        return;
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return;
    };
    let Ok(legacy) = read_legacy_patch(text) else {
        return;
    };
    let patch = convert_legacy_patch(&legacy);
    apply_patch(panels, &patch);

    // The patch menu shows the program the editor is on.
    panels
        .current_program
        .store(index as i32, Ordering::Relaxed);
}

/// Sets every parameter a patch names.
fn apply_patch(panels: &Panels, patch: &Patch) {
    for (id, value) in &patch.params {
        panels.params.set_legacy(id, *value);
    }

    // The patch knows which program it was on, and the patch menu shows it.
    let program = patch.current_program.unwrap_or(crate::editor::NO_PROGRAM);
    panels.current_program.store(program, Ordering::Relaxed);
}

/// Asks the user for a patch to write.
fn export_patch(panels: &Panels) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Export patch")
        .add_filter("Patch", &["ripx"])
        .set_file_name("patch.ripx")
        .save_file()
    else {
        return;
    };

    let patch = Patch {
        version: PATCH_VERSION,
        params: panels.params.values(),
        user_sample: None,
        current_program: None,
    };
    let Ok(text) = write_patch(&patch) else {
        return;
    };
    let _ = std::fs::write(path, text);
}

/// Asks the user for a patch to read.
fn import_patch(panels: &Panels) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Import patch")
        .add_filter("Patch", &["ripx"])
        .pick_file()
    else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(patch) = read_patch(&text) else {
        return;
    };
    apply_patch(panels, &patch);
}

/// Asks the user for a legacy patch to read.
fn import_legacy_patch(panels: &Panels) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Import legacy patch")
        .add_filter("Patch", &["xml"])
        .pick_file()
    else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(legacy) = read_legacy_patch(&text) else {
        return;
    };
    let patch = convert_legacy_patch(&legacy);
    apply_patch(panels, &patch);
}

/// Asks the user for a mallet sample to load.
///
/// The file dialog of `rfd` is modal and blocks the thread of the editor
/// while it is open, and there is no way of opening it without blocking:
/// `rfd` has no asynchronous version of its dialogs. The JUCE editor opens
/// its file chooser asynchronously, so this is the one place where this
/// build is less responsive than the original.
///
/// The decoding of the file does not block: it happens on the plugin's
/// background thread, and the editor picks the sample up the next time it
/// draws itself.
fn import_audio(panels: &Panels) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Import audio file")
        .add_filter("Audio", &["wav", "aiff", "aif", "flac", "ogg", "mp3"])
        .pick_file()
    else {
        return;
    };

    // The sample is decoded on a background thread, because the audio
    // thread may not decode anything and the editor must not freeze while
    // a large file is decoded.
    panels.background.request_decode(path);
}

/// A menu entry that turns a boolean parameter on and off.
///
/// The entry is ticked when the parameter is on, the way the JUCE editor's
/// settings menu shows them.
fn toggle(ui: &mut Ui, panels: &Panels, id: &str, label: &str) {
    let on = panels.params.plain(id) >= 0.5;
    if ui.selectable_label(on, label).clicked() {
        panels.params.begin(id);
        panels.params.set_plain(id, if on { 0.0 } else { 1.0 });
        panels.params.end(id);
    }
}

/// Sets the scale of the user interface.
fn set_scale(ui: &mut Ui, panels: &Panels, scale: f32) {
    let settings = {
        let mut settings = panels
            .settings
            .lock()
            .expect("the settings are not poisoned");
        settings.ui_scale = scale;
        *settings
    };
    crate::editor::settings::save(&settings);
    ui.set_zoom_factor(scale);
    ui.ctx().request_repaint();
}

/// Sets the number of voices the plugin plays.
fn set_polyphony(ui: &mut Ui, panels: &Panels, polyphony: usize) {
    panels
        .polyphony
        .store(polyphony, std::sync::atomic::Ordering::Relaxed);
    let settings = {
        let mut settings = panels
            .settings
            .lock()
            .expect("the settings are not poisoned");
        settings.polyphony = polyphony;
        *settings
    };
    crate::editor::settings::save(&settings);
    ui.ctx().request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::Background;
    use crate::editor::knob::Binding;
    use crate::editor::panels::Images;
    use crate::editor::settings::Settings;
    use crate::editor::theme::Theme;
    use crate::editor::{Parameters, Params};
    use nice_plug::context::gui::GuiContext;
    use nice_plug_core::context::PluginApi;
    use nice_plug_core::context::gui::GuiContextInner;
    use nice_plug_core::params::Params as ParamsTrait;
    use nice_plug_core::params::internals::ParamPtr;
    use nice_plug_core::plugin::PluginState;
    use rippler_dsp::models::ModalModel;
    use rippler_params::params::RipplerParams;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize};
    use std::sync::{Arc, Mutex};

    /// A context that records what the editor writes.
    struct Mock {
        /// The identifier of every parameter.
        ids: HashMap<ParamPtr, String>,
        /// The values that were written.
        written: Arc<Mutex<Vec<(String, f32)>>>,
    }

    impl Mock {
        /// Builds a context that watches a parameter set.
        fn new(parameters: &RipplerParams, written: Arc<Mutex<Vec<(String, f32)>>>) -> Self {
            let ids = parameters
                .param_map()
                .into_iter()
                .map(|(id, pointer, _)| (pointer, id))
                .collect();
            Self { ids, written }
        }
    }

    impl GuiContextInner for Mock {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Vst3
        }

        unsafe fn raw_begin_set_parameter(&self, _param: ParamPtr) {}

        unsafe fn raw_set_parameter_normalized(&self, param: ParamPtr, normalized: f32) {
            // SAFETY: the pointer comes from the parameter set of the test,
            // which is alive for as long as the test runs.
            unsafe { param._internal_set_normalized_value(normalized) };

            let Some(id) = self.ids.get(&param) else {
                return;
            };
            // SAFETY: see above.
            let plain = unsafe { param.preview_plain(normalized) };
            self.written
                .lock()
                .expect("the values are not poisoned")
                .push((id.clone(), plain));
        }

        unsafe fn raw_end_set_parameter(&self, _param: ParamPtr) {}

        fn get_state(&self) -> PluginState {
            PluginState {
                version: String::new(),
                params: BTreeMap::new(),
                fields: BTreeMap::new(),
            }
        }

        fn set_state(&self, _state: PluginState) {}

        fn request_restart(&self) {}
    }

    /// The state of an editor that a test can use.
    struct Harness {
        /// The parameters.
        ///
        /// The parameters are boxed, because the context watches the
        /// addresses of the parameters and a move would change them.
        parameters: Box<RipplerParams>,
        /// The parameters, by their identifier.
        map: Parameters,
        /// The context the plugin hands to the editor.
        gui: GuiContext,
        /// The images.
        images: Images,
        /// The settings.
        settings: Arc<Mutex<Settings>>,
        /// The number of voices.
        polyphony: Arc<AtomicUsize>,
        /// The velocity mode.
        velocity: Arc<AtomicBool>,
        /// The background work.
        background: Background,
        /// The program the patch menu shows.
        current_program: Arc<AtomicI32>,
        /// The bindings the editor drew.
        bindings: Mutex<Vec<Binding>>,
        /// The values that were written.
        written: Arc<Mutex<Vec<(String, f32)>>>,
    }

    impl Harness {
        /// Builds the state of an editor.
        fn new() -> Self {
            let parameters = Box::new(RipplerParams::default());
            let map = Parameters::new(&parameters);
            let written = Arc::new(Mutex::new(Vec::new()));
            let gui = GuiContext::new(Arc::new(Mock::new(&parameters, written.clone())));
            let context = egui::Context::default();
            Self {
                parameters,
                map,
                gui,
                images: Images::load(&context),
                settings: Arc::new(Mutex::new(Settings::default())),
                polyphony: Arc::new(AtomicUsize::new(8)),
                velocity: Arc::new(AtomicBool::new(false)),
                background: Background::default(),
                current_program: Arc::new(AtomicI32::new(-1)),
                bindings: Mutex::new(Vec::new()),
                written,
            }
        }

        /// Runs something with the panels of the editor.
        fn with_panels<R>(&self, f: impl FnOnce(&Panels) -> R) -> R {
            let setter = self.gui.param_setter();
            let params = Params::new(&self.parameters, &self.map, &setter);
            let panels = Panels {
                params: &params,
                theme: Theme::LIGHT,
                velocity_mode: false,
                images: &self.images,
                meter: 0.0,
                settings: &self.settings,
                polyphony: &self.polyphony,
                velocity: &self.velocity,
                background: &self.background,
                current_program: &self.current_program,
                bindings: &self.bindings,
            };
            f(&panels)
        }

        /// The values that were written, by the identifier of the parameter.
        fn written(&self) -> Vec<(String, f32)> {
            self.written
                .lock()
                .expect("the values are not poisoned")
                .clone()
        }
    }

    #[test]
    fn there_are_twenty_eight_presets() {
        assert_eq!(PRESETS.len(), 28);
    }

    #[test]
    fn loading_a_preset_remembers_the_program() {
        let harness = Harness::new();

        harness.with_panels(|panels| load_preset(panels, 3));

        // The patch menu shows the program the editor is on.
        assert_eq!(harness.current_program.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn the_patch_menu_shows_the_name_of_the_program() {
        assert_eq!(program_name(0), "Init");
        assert_eq!(program_name(2), "Harp");
        // And "Patch" when the editor is not on one of the presets.
        assert_eq!(program_name(-1), "Patch");
        assert_eq!(program_name(99), "Patch");
    }

    #[test]
    fn every_preset_is_a_parameters_tree() {
        for (name, bytes) in PRESETS {
            let text = std::str::from_utf8(bytes).expect("the preset is text");
            let legacy = read_legacy_patch(text).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(!legacy.values.is_empty(), "{name} has no values");
        }
    }

    #[test]
    fn the_presets_are_the_ones_of_the_juce_build() {
        let names: Vec<&str> = PRESETS.iter().map(|(name, _)| *name).collect();
        assert_eq!(names[0], "Init");
        assert_eq!(names[1], "Harpsi");
        assert_eq!(names[13], "Bong");
        assert_eq!(names[27], "Kalimba");
    }

    #[test]
    fn a_legacy_patch_applies_its_values() {
        let harness = Harness::new();

        let legacy = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"a_decay\" value=\"10.78\"/><PARAM id=\"a_model\" value=\"3.0\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");
        let patch = convert_legacy_patch(&legacy);

        harness.with_panels(|panels| apply_patch(panels, &patch));

        let written = harness.written();
        let decay = written
            .iter()
            .find(|(id, _)| id == "a_decay")
            .map(|(_, value)| *value);
        assert_eq!(decay, Some(10.78));

        let model = written
            .iter()
            .find(|(id, _)| id == "a_model")
            .map(|(_, value)| *value);
        assert_eq!(model, Some(ModalModel::Membrane as usize as f32));
    }

    #[test]
    fn a_ripx_patch_round_trips_through_the_editor() {
        let harness = Harness::new();

        harness.with_panels(|panels| {
            panels.params.set_legacy("a_decay", 10.78);
            panels.params.set_legacy("a_model", 3.0);
        });

        let patch = harness.with_panels(|panels| Patch {
            version: PATCH_VERSION,
            params: panels.params.values(),
            user_sample: None,
            current_program: None,
        });

        let text = write_patch(&patch).expect("the patch is written");
        let read = read_patch(&text).expect("the patch reads back");

        assert_eq!(read.params, patch.params);
        assert!((read.params["a_decay"] - 10.78).abs() < 0.001);
        assert_eq!(read.params["a_model"], 3.0);
    }
}
