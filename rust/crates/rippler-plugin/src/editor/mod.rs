//! The editor.
//!
//! The editor is an egui application, drawn by `nice-plug-egui`. It is the
//! JUCE editor of `src/PluginEditor.cpp`, widget for widget: the same
//! sections, the same knobs and the same palette.
//!
//! The state is in two parts. [`EditorState`] holds everything that has to
//! survive the editor being closed — the parameters, the theme, the scale of
//! the user interface, the polyphony, the velocity mode and the meter — and
//! it is owned by the plugin, which hands it to the editor when the editor is
//! opened. The rest is in [`OpenState`], which only exists while the editor
//! window is open.
//!
//! # The decisions of this port
//!
//! - **The theme, the scale and the polyphony** are not plugin parameters in
//!   the JUCE build either: they live in a settings file. They live in one
//!   here as well — a small JSON file in the user's configuration directory,
//!   written by [`settings`]. The polyphony is shared with the audio thread
//!   through an atomic, because the audio thread may not lock a mutex.
//! - **The images** — the logo, the sun and the moon and the buttons of the
//!   resonators — are the PNG files of the `resources` folder, embedded in
//!   the plugin with `include_bytes!` and decoded with the `image` crate when
//!   the editor opens. The font of the JUCE build is embedded the same way.
//! - **The keyboard** of the JUCE editor is not there. nice-plug's wrappers
//!   do support MIDI output — the plugin would declare
//!   `MIDI_OUTPUT = MidiConfig::Basic` and the editor would hand the notes
//!   to the plugin, which would send them to the host from its process
//!   function — but that is a feature of the plugin rather than a port of
//!   a widget, and it changes the plugin's bus layout, so it is a decision
//!   of its own. The host's own keyboard is used until then.
//! - **The file dialogs** are the ones of `rfd`, which are the native
//!   dialogs of the system. The patch is written as a `.ripx` file, which is
//!   the JSON document of `rippler_params::patch`, and the old XML patches
//!   are read through `rippler_params::legacy`.
//! - **The factory presets** are the legacy XML files of the `resources`
//!   folder, embedded in the plugin and converted to patches when they are
//!   loaded, so that there is only one copy of every preset.

pub mod knob;
pub mod menus;
pub mod meter;
pub mod panels;
pub mod pitch;
pub mod settings;
pub mod tension;
pub mod theme;

use nice_plug::util::AtomicF32;
#[cfg(test)]
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use egui::{Context, Ui};
use nice_plug::context::gui::{GuiContext, ParamSetter};
use nice_plug::editor::dpi::LogicalSize;
use nice_plug_core::params::Params as ParamsTrait;
use nice_plug_core::params::internals::ParamPtr;
use nice_plug_egui::baseview::HandlerError;
use nice_plug_egui::{Frame, NiceEguiApp};
use rippler_params::apply_parameter;
use rippler_params::params::RipplerParams;

use crate::background::Background;
use crate::editor::settings::Settings;
use crate::editor::theme::Theme;
use crate::params::UserSample;

/// The size of the editor, in points.
///
/// The JUCE editor is 650 by 485.
pub const EDITOR_SIZE: LogicalSize<f32> = LogicalSize::new(650.0, 485.0);

/// The program the patch menu shows when the editor is not on one.
///
/// The JUCE build's patch menu shows the name of the program it is on, and
/// "Patch" when it is not on one of the factory presets.
pub(crate) const NO_PROGRAM: i32 = -1;

/// The font of the JUCE build.
static UBUNTU_MEDIUM: &[u8] = include_bytes!("../../../../../resources/Ubuntu-Medium.ttf");

/// Sets the font of the editor.
///
/// The JUCE build uses the font of the plugin as its default typeface.
fn set_font(context: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        String::from("ubuntu"),
        std::sync::Arc::new(egui::FontData::from_static(UBUNTU_MEDIUM)),
    );
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(0, String::from("ubuntu"));
    }
    context.set_fonts(fonts);
}

/// The plugin's parameters, by their legacy identifier.
///
/// The parameters are looked up once, when the editor opens, and the pointers
/// stay valid for as long as the parameter set is alive, which it is for as
/// long as the editor holds on to it.
///
/// The identifiers are the legacy ones, so a widget names a parameter in one
/// place only and the same name is used everywhere else.
pub(crate) struct Parameters {
    /// The pointer of every parameter, by its identifier.
    map: HashMap<String, ParamPtr>,
    /// The identifiers the editor has asked for.
    ///
    /// The editor writes down every identifier it looks up, so that the
    /// tests can check that it only ever asks for parameters the plugin
    /// has.
    #[cfg(test)]
    used: Mutex<BTreeSet<String>>,
}

impl Parameters {
    /// Looks the parameters of a parameter set up.
    pub fn new(params: &RipplerParams) -> Self {
        let map = params
            .param_map()
            .into_iter()
            .map(|(id, pointer, _)| (id, pointer))
            .collect();
        Self {
            map,
            #[cfg(test)]
            used: Mutex::new(BTreeSet::new()),
        }
    }

    /// The pointer of a parameter.
    ///
    /// A parameter the plugin does not have is a mistake in the editor, and
    /// a debug build says so instead of quietly doing nothing: a typo in an
    /// identifier would otherwise turn a widget into a dead control without
    /// anyone noticing.
    ///
    /// The generated `apply_parameter!` and `with_parameter!` macros of
    /// `rippler-params` cannot catch such a typo, because they match the
    /// identifier at run time as well: they keep the table of the
    /// identifiers and the parameters apart, but a call site that names a
    /// parameter that does not exist still compiles.
    fn pointer(&self, id: &str) -> Option<ParamPtr> {
        #[cfg(test)]
        if let Ok(mut used) = self.used.lock() {
            used.insert(id.to_string());
        }

        let pointer = self.map.get(id).copied();
        debug_assert!(
            pointer.is_some(),
            "the editor asks for a parameter the plugin does not have: {id}"
        );
        pointer
    }

    /// The identifiers the editor has asked for.
    #[cfg(test)]
    pub fn used_ids(&self) -> BTreeSet<String> {
        self.used
            .lock()
            .map(|used| used.clone())
            .unwrap_or_default()
    }

    /// The normalised value of a parameter.
    pub fn normalized(&self, id: &str) -> f32 {
        let Some(pointer) = self.pointer(id) else {
            return 0.0;
        };
        // SAFETY: the pointers point into the parameter set, which the editor
        // holds in an `Arc` for as long as the editor lives.
        unsafe { pointer.modulated_normalized_value() }
    }

    /// The plain value of a parameter.
    ///
    /// A choice parameter holds its index and a boolean holds `0.0` or `1.0`,
    /// the way the legacy state holds them.
    pub fn plain(&self, id: &str) -> f32 {
        let Some(pointer) = self.pointer(id) else {
            return 0.0;
        };
        // SAFETY: see `normalized`.
        unsafe { pointer.modulated_plain_value() }
    }

    /// The normalised default value of a parameter.
    pub fn default_normalized(&self, id: &str) -> f32 {
        let Some(pointer) = self.pointer(id) else {
            return 0.0;
        };
        // SAFETY: see `normalized`.
        unsafe { pointer.default_normalized_value() }
    }

    /// The normalised value of a plain value.
    pub fn preview_normalized(&self, id: &str, plain: f32) -> f32 {
        let Some(pointer) = self.pointer(id) else {
            return 0.0;
        };
        // SAFETY: see `normalized`.
        unsafe { pointer.preview_normalized(plain) }
    }

    /// The plain value of a normalised value.
    pub fn preview_plain(&self, id: &str, normalized: f32) -> f32 {
        let Some(pointer) = self.pointer(id) else {
            return 0.0;
        };
        // SAFETY: see `normalized`.
        unsafe { pointer.preview_plain(normalized) }
    }

    /// The plain value of every parameter, by its identifier.
    ///
    /// The values are the ones the legacy state holds: a float parameter its
    /// value, a choice parameter its index and a boolean `0.0` or `1.0`.
    pub fn values(&self) -> std::collections::BTreeMap<String, f64> {
        let mut values = std::collections::BTreeMap::new();
        for (id, pointer) in &self.map {
            // SAFETY: see `normalized`.
            let value = unsafe { pointer.modulated_plain_value() };
            values.insert(id.clone(), value as f64);
        }
        values
    }
}

/// The plugin's parameters and the setter that writes them.
///
/// The setter is the one the plugin's context hands to the editor, so every
/// change the editor makes is recorded by the host as an automation point.
pub(crate) struct Params<'a> {
    /// Every parameter of the plugin.
    all: &'a RipplerParams,
    /// The parameters.
    parameters: &'a Parameters,
    /// The setter.
    setter: &'a ParamSetter<'a>,
}

impl<'a> Params<'a> {
    /// Binds the parameters to a setter.
    pub fn new(
        all: &'a RipplerParams,
        parameters: &'a Parameters,
        setter: &'a ParamSetter<'a>,
    ) -> Self {
        Self {
            all,
            parameters,
            setter,
        }
    }

    /// Sets a parameter to a legacy plain value.
    ///
    /// The value is a legacy one: a float parameter's value, a choice
    /// parameter's index and a boolean's `0.0` or `1.0`. The conversion is
    /// the one of `rippler_params`, so the identifiers and the values are
    /// the ones of the legacy state.
    pub fn set_legacy(&self, id: &str, value: f64) -> bool {
        self.begin(id);
        let set = apply_parameter!(self.all, self.setter, id, value);
        self.end(id);
        set
    }

    /// The normalised value of a parameter.
    pub fn normalized(&self, id: &str) -> f32 {
        self.parameters.normalized(id)
    }

    /// The plain value of a parameter.
    pub fn plain(&self, id: &str) -> f32 {
        self.parameters.plain(id)
    }

    /// The normalised default value of a parameter.
    pub fn default_normalized(&self, id: &str) -> f32 {
        self.parameters.default_normalized(id)
    }

    /// The normalised value of a plain value.
    pub fn preview_normalized(&self, id: &str, plain: f32) -> f32 {
        self.parameters.preview_normalized(id, plain)
    }

    /// The plain value of a normalised value.
    pub fn preview_plain(&self, id: &str, normalized: f32) -> f32 {
        self.parameters.preview_plain(id, normalized)
    }

    /// The plain value of every parameter, by its identifier.
    pub fn values(&self) -> std::collections::BTreeMap<String, f64> {
        self.parameters.values()
    }

    /// Tells the host that a parameter is about to change.
    pub fn begin(&self, id: &str) {
        let Some(pointer) = self.parameters.pointer(id) else {
            return;
        };
        // SAFETY: the pointer comes from the plugin's own parameter map, so
        // the parameter exists.
        unsafe { self.setter.raw_context.raw_begin_set_parameter(pointer) };
    }

    /// Tells the host that a parameter has changed.
    pub fn set_normalized(&self, id: &str, value: f32) {
        let Some(pointer) = self.parameters.pointer(id) else {
            return;
        };
        // SAFETY: the pointer comes from the plugin's own parameter map, so
        // the parameter exists. The value is snapped to the parameter's step
        // size first, the way JUCE's `setValueNotifyingHost` does.
        unsafe {
            let plain = pointer.preview_plain(value);
            let snapped = pointer.preview_normalized(plain);
            self.setter
                .raw_context
                .raw_set_parameter_normalized(pointer, snapped);
        }
    }

    /// Tells the host that a parameter has a new plain value.
    pub fn set_plain(&self, id: &str, value: f64) {
        let Some(pointer) = self.parameters.pointer(id) else {
            return;
        };
        // SAFETY: the pointer comes from the plugin's own parameter map, so
        // the parameter exists. The value is snapped to the parameter's step
        // size first, the way JUCE's `setValueNotifyingHost` does.
        unsafe {
            let normalized = pointer.preview_normalized(value as f32);
            self.setter
                .raw_context
                .raw_set_parameter_normalized(pointer, normalized);
        }
    }

    /// Tells the host that a parameter has finished changing.
    pub fn end(&self, id: &str) {
        let Some(pointer) = self.parameters.pointer(id) else {
            return;
        };
        // SAFETY: the pointer comes from the plugin's own parameter map, so
        // the parameter exists.
        unsafe { self.setter.raw_context.raw_end_set_parameter(pointer) };
    }
}

/// The state the editor keeps between openings.
///
/// The plugin owns this state and hands it to the editor when the editor is
/// opened, so that the theme, the scale and the polyphony are still there
/// the next time the editor is opened.
pub struct EditorState {
    /// The plugin's parameters.
    params: Arc<RipplerParams>,
    /// The settings that are not plugin parameters.
    settings: Arc<Mutex<Settings>>,
    /// The number of voices the plugin plays.
    ///
    /// The audio thread reads this, so it is an atomic rather than a lock.
    polyphony: Arc<AtomicUsize>,
    /// Whether the velocity edit mode is on.
    ///
    /// The JUCE build keeps this in its processor as well, and does not save
    /// it: it is a mode of the editor, not a setting of the plugin. It is
    /// an atomic, because the editor changes it while it draws itself.
    velocity_mode: Arc<AtomicBool>,
    /// A user sample that the editor decoded, waiting for the audio thread.
    user_sample: Arc<Mutex<Option<UserSample>>>,
    /// The meter's value, shared with the audio thread.
    meter: Arc<AtomicF32>,
    /// The work the editor hands to the background thread.
    background: Background,
    /// The program the patch menu shows.
    ///
    /// It is the index of the factory preset the editor is on, or `-1` when
    /// it is not on one. The JUCE build shows the name of the program in
    /// the patch menu.
    current_program: Arc<AtomicI32>,
    /// The bindings the editor drew.
    ///
    /// The editor writes down what every knob is bound to, which the tests
    /// use to check that the widgets are bound to the parameters the JUCE
    /// editor binds them to.
    bindings: Arc<Mutex<Vec<knob::Binding>>>,
}

impl EditorState {
    /// Builds the state of an editor.
    pub(crate) fn new(
        params: Arc<RipplerParams>,
        settings: Arc<Mutex<Settings>>,
        polyphony: Arc<AtomicUsize>,
        meter: Arc<AtomicF32>,
        user_sample: Arc<Mutex<Option<UserSample>>>,
        background: Background,
    ) -> Self {
        Self {
            params,
            settings,
            polyphony,
            velocity_mode: Arc::new(AtomicBool::new(false)),
            meter,
            user_sample,
            background,
            current_program: Arc::new(AtomicI32::new(NO_PROGRAM)),
            bindings: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A user sample that the editor decoded, waiting for the audio thread.
    pub(crate) fn user_sample(&self) -> &Arc<Mutex<Option<UserSample>>> {
        &self.user_sample
    }

    /// The work the editor hands to the background thread.
    pub(crate) fn background(&self) -> &Background {
        &self.background
    }

    /// The program the patch menu shows.
    pub(crate) fn current_program(&self) -> &Arc<AtomicI32> {
        &self.current_program
    }

    /// The bindings the editor drew.
    pub(crate) fn bindings(&self) -> &Mutex<Vec<knob::Binding>> {
        &self.bindings
    }

    /// The plugin's parameters.
    pub(crate) fn params(&self) -> &Arc<RipplerParams> {
        &self.params
    }

    /// The settings that are not plugin parameters.
    pub(crate) fn settings(&self) -> &Arc<Mutex<Settings>> {
        &self.settings
    }

    /// The number of voices the plugin plays.
    pub(crate) fn polyphony(&self) -> &Arc<AtomicUsize> {
        &self.polyphony
    }

    /// The meter's value, shared with the audio thread.
    pub(crate) fn meter(&self) -> &Arc<AtomicF32> {
        &self.meter
    }

    /// Whether the velocity edit mode is on.
    pub(crate) fn velocity_mode(&self) -> &Arc<AtomicBool> {
        &self.velocity_mode
    }

    /// The theme the editor is drawn in.
    pub(crate) fn theme(&self) -> Theme {
        let settings = self.settings.lock().expect("the settings are not poisoned");
        Theme::new(settings.dark_theme)
    }
}

/// The state that only exists while the editor window is open.
struct OpenState {
    /// The egui context.
    ///
    /// It is held so that egui can clean up when the editor is closed.
    _context: Context,
    /// The context the plugin hands to the editor.
    gui: GuiContext,
    /// The plugin's parameters, by their identifier.
    parameters: Parameters,
    /// The images of the editor.
    images: panels::Images,
}

/// The editor application.
///
/// The plugin builds one of these and hands it to `nice-plug-egui`, which
/// draws it.
pub struct RipplerEditor {
    /// The state that survives the editor being closed.
    state: EditorState,
    /// The state that only exists while the editor is open.
    open: Option<OpenState>,
}

impl RipplerEditor {
    /// Builds an editor for a plugin's state.
    pub fn new(state: EditorState) -> Self {
        Self { state, open: None }
    }
}

impl NiceEguiApp for RipplerEditor {
    fn build(
        &mut self,
        egui_ctx: Context,
        nice_gui_ctx: GuiContext,
        _frame: &mut Frame,
    ) -> Result<(), HandlerError> {
        // The font of the JUCE build is the one of the plugin, and the
        // images are decoded once, when the editor opens.
        set_font(&egui_ctx);
        let images = panels::Images::load(&egui_ctx);
        let parameters = Parameters::new(self.state.params());

        self.open = Some(OpenState {
            _context: egui_ctx,
            gui: nice_gui_ctx,
            parameters,
            images,
        });
        Ok(())
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let theme = self.state.theme();
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme.background);

        // The bindings of the last frame are of no use any more.
        if let Ok(mut bindings) = self.state.bindings().lock() {
            bindings.clear();
        }

        let Some(open) = self.open.as_mut() else {
            return;
        };

        let setter = open.gui.param_setter();
        let params = Params::new(self.state.params(), &open.parameters, &setter);

        // A sample that was decoded in the background is handed to the
        // audio thread here, and the mallet becomes the user's file.
        if let Some(sample) = self.state.background().take_decoded() {
            if let Ok(mut slot) = self.state.user_sample().lock() {
                *slot = Some(sample);
            }
            params.set_legacy(
                "mallet_type",
                rippler_dsp::mallet::MalletType::UserFile as usize as f64,
            );
        }

        let panels = panels::Panels {
            params: &params,
            theme,
            velocity_mode: self.state.velocity_mode().load(Ordering::Relaxed),
            images: &open.images,
            meter: self.state.meter().load(Ordering::Relaxed),
            settings: self.state.settings(),
            polyphony: self.state.polyphony(),
            velocity: self.state.velocity_mode(),
            background: self.state.background(),
            current_program: self.state.current_program(),
            bindings: self.state.bindings(),
        };

        panels::top_bar(ui, &panels);

        // Every section is a column of its own, the way the sections of the
        // JUCE editor are.
        ui.horizontal_top(|ui| {
            // The sections are next to each other, the way they are in the
            // JUCE editor.
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.vertical(|ui| panels::noise_section(ui, &panels));
            ui.vertical(|ui| panels::mallet_section(ui, &panels));
            ui.vertical(|ui| {
                panels::resonator_section(ui, &panels, &panels::resonator_a());
                panels::resonator_section(ui, &panels, &panels::resonator_b());
            });
            ui.vertical(|ui| panels::coupling_section(ui, &panels));
        });
    }

    fn editor_closed(&mut self) {
        self.open = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::Background;
    use crate::editor::settings::DEFAULT_POLYPHONY;
    use std::sync::atomic::Ordering;

    /// Builds the state of an editor.
    fn state() -> EditorState {
        let settings = Settings::default();
        EditorState::new(
            Arc::new(RipplerParams::default()),
            Arc::new(Mutex::new(settings)),
            Arc::new(AtomicUsize::new(DEFAULT_POLYPHONY)),
            Arc::new(AtomicF32::new(0.0)),
            Arc::new(Mutex::new(None)),
            Background::default(),
        )
    }

    #[test]
    fn the_editor_starts_with_the_light_theme() {
        assert_eq!(state().theme(), Theme::LIGHT);
    }

    #[test]
    fn the_editor_holds_the_polyphony() {
        let state = state();
        assert_eq!(state.polyphony().load(Ordering::Relaxed), DEFAULT_POLYPHONY);
    }

    #[test]
    fn the_velocity_mode_is_off_until_it_is_turned_on() {
        let state = state();
        assert!(!state.velocity_mode().load(Ordering::Relaxed));
        state.velocity_mode().store(true, Ordering::Relaxed);
        assert!(state.velocity_mode().load(Ordering::Relaxed));
    }
}
