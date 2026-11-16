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

pub mod knob;
pub mod meter;
pub mod pitch;
pub mod settings;
pub mod tension;
pub mod theme;

use nice_plug::util::AtomicF32;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use egui::{Context, Ui};
use nice_plug::context::gui::{GuiContext, ParamSetter};
use nice_plug::editor::dpi::LogicalSize;
use nice_plug_core::params::Params as ParamsTrait;
use nice_plug_core::params::internals::ParamPtr;
use nice_plug_egui::baseview::HandlerError;
use nice_plug_egui::{Frame, NiceEguiApp};
use rippler_params::params::RipplerParams;

use crate::editor::settings::Settings;
use crate::editor::theme::Theme;

/// The size of the editor, in points.
///
/// The JUCE editor is 650 by 485.
pub const EDITOR_SIZE: LogicalSize<f32> = LogicalSize::new(650.0, 485.0);

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
}

impl Parameters {
    /// Looks the parameters of a parameter set up.
    pub fn new(params: &RipplerParams) -> Self {
        let map = params
            .param_map()
            .into_iter()
            .map(|(id, pointer, _)| (id, pointer))
            .collect();
        Self { map }
    }

    /// The pointer of a parameter.
    fn pointer(&self, id: &str) -> Option<ParamPtr> {
        self.map.get(id).copied()
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
}

/// The plugin's parameters and the setter that writes them.
///
/// The setter is the one the plugin's context hands to the editor, so every
/// change the editor makes is recorded by the host as an automation point.
pub(crate) struct Params<'a> {
    /// The parameters.
    parameters: &'a Parameters,
    /// The setter.
    setter: &'a ParamSetter<'a>,
}

impl<'a> Params<'a> {
    /// Binds the parameters to a setter.
    pub fn new(parameters: &'a Parameters, setter: &'a ParamSetter<'a>) -> Self {
        Self { parameters, setter }
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
    /// it: it is a mode of the editor, not a setting of the plugin.
    velocity_mode: bool,
    /// The meter's value, shared with the audio thread.
    meter: Arc<AtomicF32>,
}

impl EditorState {
    /// Builds the state of an editor.
    pub fn new(
        params: Arc<RipplerParams>,
        settings: Arc<Mutex<Settings>>,
        polyphony: Arc<AtomicUsize>,
        meter: Arc<AtomicF32>,
    ) -> Self {
        Self {
            params,
            settings,
            polyphony,
            velocity_mode: false,
            meter,
        }
    }

    /// The plugin's parameters.
    pub fn params(&self) -> &Arc<RipplerParams> {
        &self.params
    }

    /// The settings that are not plugin parameters.
    pub fn settings(&self) -> &Arc<Mutex<Settings>> {
        &self.settings
    }

    /// The number of voices the plugin plays.
    pub fn polyphony(&self) -> &Arc<AtomicUsize> {
        &self.polyphony
    }

    /// The meter's value, shared with the audio thread.
    pub fn meter(&self) -> &Arc<AtomicF32> {
        &self.meter
    }

    /// Whether the velocity edit mode is on.
    pub fn velocity_mode(&self) -> bool {
        self.velocity_mode
    }

    /// Turns the velocity edit mode on or off.
    pub fn set_velocity_mode(&mut self, on: bool) {
        self.velocity_mode = on;
    }

    /// The theme the editor is drawn in.
    pub fn theme(&self) -> Theme {
        let settings = self.settings.lock().expect("the settings are not poisoned");
        Theme::new(settings.dark_theme)
    }

    /// The scale of the user interface.
    pub fn ui_scale(&self) -> f32 {
        let settings = self.settings.lock().expect("the settings are not poisoned");
        settings.ui_scale
    }
}

/// The state that only exists while the editor window is open.
struct OpenState {
    /// The egui context.
    ///
    /// It is held so that egui can clean up when the editor is closed.
    _context: Context,
    /// The context the plugin hands to the editor.
    #[allow(dead_code)]
    gui: GuiContext,
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
        self.open = Some(OpenState {
            _context: egui_ctx,
            gui: nice_gui_ctx,
        });
        Ok(())
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let theme = self.state.theme();
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme.background);

        // The panels are built in the tasks that follow; for now the editor
        // shows the theme it is drawn in, so that the plumbing can be seen
        // working.
        ui.label("RipplerX");
    }

    fn editor_closed(&mut self) {
        self.open = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let mut state = state();
        assert!(!state.velocity_mode());
        state.set_velocity_mode(true);
        assert!(state.velocity_mode());
    }
}
