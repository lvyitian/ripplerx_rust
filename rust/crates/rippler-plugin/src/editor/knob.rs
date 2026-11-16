//! The rotary knob.
//!
//! Port of `src/ui/Rotary.cpp` and of `src/ui/Rotary.h`.
//!
//! The knob is a circle of sixteen points with a pointer in it and an arc
//! around it. The arc sweeps two hundred and sixty degrees, from a hundred
//! and thirty degrees to the left of the top to a hundred and thirty degrees
//! to the right of it, and it is only drawn when the value is not at the
//! bottom of its range — or, for a control that is centred, not at the middle
//! of it.

use egui::{
    Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget, vec2,
};
use std::sync::Mutex;

use crate::editor::Params;
use crate::editor::theme::Theme;

/// The width of a knob, in points.
///
/// The JUCE editor's knobs are seventy points wide.
pub(crate) const KNOB_WIDTH: f32 = 70.0;

/// The height of a knob, in points.
///
/// The JUCE editor's knobs are seventy five points high.
pub(crate) const KNOB_HEIGHT: f32 = 75.0;

/// The radius of the knob's circle, in points.
pub(crate) const RADIUS: f32 = 16.0;

/// The radius of the arc around the knob, in points.
const ARC_RADIUS: f32 = RADIUS + 2.0;

/// The sweep of the arc, in degrees.
///
/// The arc goes from minus a hundred and thirty degrees to plus a hundred
/// and thirty degrees.
pub(crate) const SWEEP_DEGREES: f32 = 260.0;

/// The angle the arc starts at, in degrees.
pub(crate) const START_DEGREES: f32 = -130.0;

/// The number of pixels a drag has to cover to move a knob by one percent.
///
/// The JUCE build's `Rotary::pixels_per_percent`.
const PIXELS_PER_PERCENT: f32 = 100.0;

/// The blur radius of the knob's drop shadow.
///
/// The JUCE build uses a `DropShadow(15, {4, 4})`.
const SHADOW_BLUR: f32 = 15.0;

/// The offset of the knob's drop shadow.
const SHADOW_OFFSET: egui::Vec2 = egui::Vec2::new(4.0, 4.0);

/// The opacity of the middle of the knob's drop shadow.
///
/// The JUCE build's shadow is a black or a grey one at full opacity, but
/// the shadow of a knob that sits on the background is only seen through
/// its blur, so the middle of it is lighter than the original.
const SHADOW_OPACITY: f32 = 0.45;

/// What a knob is bound to.
///
/// The editor writes down what every knob is bound to, which the tests use
/// to check that the widgets are bound to the parameters the JUCE editor
/// binds them to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Binding {
    /// The parameter the knob edits.
    pub id: &'static str,
    /// The parameter the knob edits when the shift key is held, if it has
    /// one.
    pub velocity: Option<&'static str>,
    /// The rectangle the knob was drawn in.
    pub rect: Rect,
}

/// The formats a knob's label can have.
///
/// Port of `LabelFormat` in `src/ui/Rotary.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabelFormat {
    /// A frequency, in hertz.
    Hz,
    /// A level, in decibels.
    Db,
    /// A percentage.
    Percent,
    /// A number with one decimal.
    Float1,
    /// A number with two decimals, a hundred times the value.
    Float2Times100,
    /// A time, in milliseconds.
    Millis,
    /// A time, in seconds, with two decimals.
    Seconds2f,
    /// The mix of two resonators.
    AbMix,
    /// A filter that is either off, a low pass or a high pass.
    FilterLphp,
    /// A pitch, in semitones.
    PitchSemis,
}

impl LabelFormat {
    /// The text the format shows for a value.
    ///
    /// Port of `Rotary::draw_label`. The value is the plain value of the
    /// parameter, not its normalised value.
    pub fn format(&self, value: f32) -> String {
        match self {
            LabelFormat::Percent => format!("{} %", (value * 100.0).round() as i32),
            LabelFormat::Millis => {
                if value < 1000.0 {
                    format!("{} ms", value as i32)
                } else {
                    format!("{:.2} s", value / 1000.0)
                }
            }
            LabelFormat::Hz => {
                if value >= 1000.0 {
                    format!("{:.1} kHz", value / 1000.0)
                } else {
                    format!("{value:.0} Hz")
                }
            }
            LabelFormat::Float1 => format!("{value:.1}"),
            LabelFormat::Float2Times100 => format!("{:.2}", value * 100.0),
            LabelFormat::Seconds2f => format!("{value:.2} s"),
            LabelFormat::AbMix => format!(
                "{}:{}",
                ((1.0 - value) * 100.0) as i32,
                (value * 100.0) as i32
            ),
            LabelFormat::Db => format!("{} dB", value as i32),
            LabelFormat::FilterLphp => {
                if value == 0.0 {
                    return String::from("Off");
                }
                // The frequency the plugin computes: the map runs from twenty
                // hertz to twenty kilohertz, and the negative half of the
                // range is the low pass side of it.
                let frequency = 20.0
                    * 1000.0f64.powf(if value < 0.0 {
                        1.0 + value as f64
                    } else {
                        value as f64
                    });
                let prefix = if value < 0.0 { "LP " } else { "HP " };
                if frequency >= 1000.0 {
                    format!("{prefix}{:.1} kHz", frequency / 1000.0)
                } else {
                    format!("{prefix}{frequency:.0} Hz")
                }
            }
            LabelFormat::PitchSemis => format!("{value:.0} Sem"),
        }
    }
}

/// The angle of the knob's pointer, in degrees.
///
/// The pointer is at the top of the knob when the value is at the middle of
/// its range, and it sweeps two hundred and sixty degrees.
pub(crate) fn value_angle_degrees(normalized: f32) -> f32 {
    START_DEGREES + normalized * SWEEP_DEGREES
}

/// The angle of the knob's pointer, in radians.
pub(crate) fn value_angle(normalized: f32) -> f32 {
    value_angle_degrees(normalized).to_radians()
}

/// The angle of the knob's pointer, as a point on a circle.
///
/// The angle is measured clockwise from the top of the circle, the way
/// JUCE's `Point::getPointOnCircumference` measures it.
fn direction(angle: f32) -> egui::Vec2 {
    vec2(angle.sin(), -angle.cos())
}

/// Draws the drop shadow of a knob.
///
/// The JUCE build draws a blurred `DropShadow(15, {4, 4})` under the knob's
/// circle. egui can only blur a rectangle, so the shadow is drawn as a mesh
/// of rings whose colour fades out over the blur radius, which reads like
/// the original.
fn draw_shadow(painter: &Painter, centre: Pos2, radius: f32, colour: Color32) {
    /// The number of points around the circle.
    const SEGMENTS: usize = 32;

    /// The rings of the shadow: the radius and how much of the shadow is
    /// left at that radius.
    const RINGS: [(f32, f32); 4] = [(0.0, 1.0), (0.45, 0.45), (0.8, 0.12), (1.0, 0.0)];

    let centre = centre + SHADOW_OFFSET;
    let shadow = |opacity: f32| {
        Color32::from_rgba_unmultiplied(
            colour.r(),
            colour.g(),
            colour.b(),
            (opacity * SHADOW_OPACITY * 255.0) as u8,
        )
    };

    let mut mesh = egui::epaint::Mesh::default();

    // The middle of the shadow.
    let middle = mesh.vertices.len() as u32;
    mesh.vertices
        .push(egui::epaint::Vertex::untextured(centre, shadow(1.0)));

    let mut previous = middle;
    for (distance, opacity) in RINGS {
        let radius = radius + distance * SHADOW_BLUR;
        let colour = shadow(opacity);
        let first = mesh.vertices.len() as u32;

        for segment in 0..SEGMENTS {
            let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            mesh.vertices.push(egui::epaint::Vertex::untextured(
                centre + direction(angle) * radius,
                colour,
            ));
        }

        for segment in 0..SEGMENTS {
            let next = first + (segment as u32 + 1) % SEGMENTS as u32;
            mesh.add_triangle(previous, first + segment as u32, next);
        }

        previous = first;
    }

    painter.add(Shape::mesh(mesh));
}

/// A rotary knob.
///
/// Port of `Rotary` in `src/ui/Rotary.cpp`.
pub(crate) struct Knob<'a> {
    /// The parameters and the setter.
    params: &'a Params<'a>,
    /// The identifier of the parameter the knob edits.
    id: &'static str,
    /// The identifier of the velocity parameter, if the knob has one.
    velocity_id: Option<&'static str>,
    /// The name under the knob.
    name: &'static str,
    /// The format of the value the knob shows.
    format: LabelFormat,
    /// Whether the knob's value is centred.
    ///
    /// A centred knob draws its arc from the top of the knob rather than
    /// from the bottom of its range.
    symmetric: bool,
    /// Whether the velocity edit mode is on.
    velocity_mode: bool,
    /// The colours.
    theme: Theme,
    /// The log of the bindings, if the editor keeps one.
    bindings: Option<&'a Mutex<Vec<Binding>>>,
}

impl<'a> Knob<'a> {
    /// Builds a knob.
    pub fn new(
        params: &'a Params<'a>,
        id: &'static str,
        name: &'static str,
        format: LabelFormat,
    ) -> Self {
        Self {
            params,
            id,
            velocity_id: None,
            name,
            format,
            symmetric: false,
            velocity_mode: false,
            theme: Theme::LIGHT,
            bindings: None,
        }
    }

    /// Gives the knob a velocity parameter.
    ///
    /// The knob edits the velocity parameter instead of its own when the
    /// shift key is held or when the velocity mode is on.
    pub fn with_velocity(mut self, velocity_id: &'static str) -> Self {
        self.velocity_id = Some(velocity_id);
        self
    }

    /// Makes the knob's value centred.
    pub fn symmetric(mut self) -> Self {
        self.symmetric = true;
        self
    }

    /// Turns the velocity edit mode on or off.
    pub fn with_velocity_mode(mut self, on: bool) -> Self {
        self.velocity_mode = on;
        self
    }

    /// Sets the colours of the knob.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Keeps a record of what the knob is bound to.
    pub fn with_bindings(mut self, bindings: &'a Mutex<Vec<Binding>>) -> Self {
        self.bindings = Some(bindings);
        self
    }

    /// The identifier of the parameter the knob edits.
    ///
    /// It is the velocity parameter when the shift key is held or when the
    /// velocity mode is on, and the knob's own parameter otherwise.
    fn target(&self, shift: bool) -> &'static str {
        match self.velocity_id {
            Some(velocity_id) if shift || self.velocity_mode => velocity_id,
            _ => self.id,
        }
    }

    /// Draws the arc.
    fn draw_arc(
        &self,
        painter: &Painter,
        centre: Pos2,
        radius: f32,
        start: f32,
        end: f32,
        colour: Color32,
    ) {
        let sweep = end - start;
        let steps = ((sweep.abs() / 0.05).ceil() as usize).clamp(2, 256);
        let points: Vec<Pos2> = (0..=steps)
            .map(|step| {
                let angle = start + sweep * step as f32 / steps as f32;
                centre + direction(angle) * radius
            })
            .collect();
        painter.add(Shape::line(points, Stroke::new(2.0, colour)));
    }
}

impl Widget for Knob<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, _) = ui.allocate_exact_size(vec2(KNOB_WIDTH, KNOB_HEIGHT), Sense::hover());
        self.ui_at(ui, rect)
    }
}

impl Knob<'_> {
    /// Draws the knob in a rectangle.
    ///
    /// The mouse is only on the knob when it is on its circle, so that the
    /// tension control that sits in the corner of a knob is not in the way.
    pub(crate) fn ui_at(&self, ui: &mut Ui, rect: Rect) -> Response {
        let centre = rect.center() + vec2(0.0, -4.0);
        let circle = Rect::from_center_size(centre, vec2(RADIUS * 2.0, RADIUS * 2.0));
        let id = ui.make_persistent_id(("knob", self.id));
        let response = ui.interact(circle, id, Sense::click_and_drag());

        // The editor writes down what the knob is bound to, so that the
        // tests can check it.
        if let Some(bindings) = self.bindings
            && let Ok(mut bindings) = bindings.lock()
        {
            bindings.push(Binding {
                id: self.id,
                velocity: self.velocity_id,
                rect,
            });
        }

        let modifiers = ui.input(|input| input.modifiers);
        let shift = modifiers.shift;
        let target = self.target(shift);

        let normalised = self.params.normalized(target);
        let plain = self.params.plain(target);

        // The drag is remembered in egui's memory, because the knob is built
        // again every frame and cannot keep the state of the drag itself.
        let start_id = id.with("start");
        let sum_id = id.with("sum");

        if response.drag_started() {
            self.params.begin(target);
            ui.memory_mut(|memory| {
                memory.data.insert_temp(start_id, normalised);
                memory.data.insert_temp(sum_id, 0.0f32);
            });
        }

        if response.dragged() {
            // The change is the horizontal and the vertical movement of the
            // mouse, so that a knob can be turned in either direction.
            let speed = if modifiers.ctrl { 40.0 } else { 4.0 } * PIXELS_PER_PERCENT;
            let delta = response.drag_delta();
            let change = (delta.x - delta.y) / speed;

            let sum =
                ui.memory(|memory| memory.data.get_temp::<f32>(sum_id).unwrap_or(0.0)) + change;
            ui.memory_mut(|memory| memory.data.insert_temp(sum_id, sum));

            let start =
                ui.memory(|memory| memory.data.get_temp::<f32>(start_id).unwrap_or(normalised));
            let value = (start + sum).clamp(0.0, 1.0);

            // The pitch knobs snap to whole semitones, unless the control key
            // is held. The value that is snapped is the value that is being
            // dragged, not the value the knob had before the drag.
            let value = if self.format == LabelFormat::PitchSemis && !modifiers.ctrl {
                let whole = self.params.preview_plain(target, value).round();
                self.params.preview_normalized(target, whole)
            } else {
                value
            };

            self.params.set_normalized(target, value);
        }

        if response.drag_stopped() {
            self.params.end(target);
        }

        if response.double_clicked() {
            self.params.begin(target);
            self.params
                .set_normalized(target, self.params.default_normalized(target));
            self.params.end(target);
        }

        // The wheel steps the value. The JUCE build has a fix for the wheel
        // not working when the value is zero, where the first step takes
        // more than five percent, and the fix is ported here.
        if response.hovered() {
            let wheel: f32 = ui.input(|input| {
                input
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                        _ => None,
                    })
                    .sum()
            });
            if wheel != 0.0 && !ui.input(|input| input.pointer.any_down()) {
                let speed = if modifiers.ctrl { 0.01 } else { 0.05 };
                let mut change = if wheel > 0.0 { speed } else { -speed };

                self.params.begin(target);
                let value = self.params.normalized(target);
                self.params
                    .set_normalized(target, (value + change).clamp(0.0, 1.0));

                // The step is added to the value the parameter has, and not
                // to the value of the step before it, so that the first step
                // off zero is not lost.
                let mut steps = 0;
                while wheel > 0.0 && self.params.normalized(target) == 0.0 && steps < 100 {
                    change += 0.05;
                    let value = self.params.normalized(target);
                    self.params
                        .set_normalized(target, (value + change).clamp(0.0, 1.0));
                    steps += 1;
                }

                self.params.end(target);
            }
        }

        self.paint(ui, rect, &response, normalised, plain, shift);

        response
    }
}

impl Knob<'_> {
    /// Draws the knob.
    fn paint(
        &self,
        ui: &mut Ui,
        rect: egui::Rect,
        response: &Response,
        normalised: f32,
        plain: f32,
        shift: bool,
    ) {
        let painter = ui.painter();
        let theme = &self.theme;

        // The circle sits four points above the middle of the widget, the
        // way the JUCE build's circle does.
        let centre = rect.center() + vec2(0.0, -4.0);

        // The drop shadow. The JUCE build draws a blurred
        // `DropShadow(15, {4, 4})`, and egui has no blurred shadow for a
        // circle, so the shadow is a radial gradient that fades out over
        // the blur radius.
        draw_shadow(painter, centre, RADIUS, theme.shadow);

        // The knob itself.
        painter.circle_filled(centre, RADIUS, theme.background);

        // The arc. A centred knob starts its arc at the top of the knob, and
        // every other knob starts it at the bottom of its range.
        let angle = value_angle(normalised);
        let start = if self.symmetric {
            0.0
        } else {
            value_angle(0.0)
        };
        let drawn = if self.symmetric {
            normalised != 0.5
        } else {
            normalised != 0.0
        };
        if drawn {
            self.draw_arc(painter, centre, ARC_RADIUS, start, angle, theme.active);
        }

        // The velocity arc, which shows how far the velocity bends the knob.
        if let Some(velocity_id) = self.velocity_id {
            let velocity = self.params.plain(velocity_id);
            let limit = value_angle(1.0);
            let velocity_angle = (angle + velocity * (value_angle(1.0) - value_angle(0.0)))
                .clamp(value_angle(0.0), limit);
            if (velocity > 0.0 && angle < limit) || (velocity < 0.0 && angle > value_angle(0.0)) {
                let radius = rect.height() / 2.0 - 20.0 + 8.0;
                self.draw_arc(
                    painter,
                    centre,
                    radius,
                    angle,
                    velocity_angle,
                    theme.velocity,
                );
            }

            // The whole knob is washed over with the velocity colour when
            // the velocity mode is on.
            if self.velocity_mode {
                painter.circle_filled(
                    centre,
                    RADIUS,
                    Color32::from_rgba_unmultiplied(
                        theme.velocity.r(),
                        theme.velocity.g(),
                        theme.velocity.b(),
                        128,
                    ),
                );
            }
        }

        // The pointer, which is a short thick line from the middle of the
        // knob outwards.
        let direction = direction(angle);
        let from = centre + direction * 5.0;
        let to = centre + direction * (RADIUS - 5.0);
        painter.add(Shape::line_segment(
            [from, to],
            Stroke::new(3.0, theme.neutral),
        ));
        painter.circle_filled(from, 1.5, theme.neutral);
        painter.circle_filled(to, 1.5, theme.neutral);

        // The label. It shows the name of the parameter, and the value of the
        // parameter while the mouse is down.
        let held = response.is_pointer_button_down_on() || response.dragged();
        let text = if held {
            if (shift || self.velocity_mode) && self.velocity_id.is_some() {
                let velocity = self.params.plain(self.velocity_id.unwrap_or(self.id));
                format!("{} %", (velocity * 100.0).round() as i32)
            } else {
                self.format.format(plain)
            }
        } else {
            String::from(self.name)
        };

        let size = if self.format == LabelFormat::FilterLphp && held {
            14.0
        } else {
            15.0
        };
        let font = FontId::proportional(size);
        let band = egui::Rect::from_min_max(
            Pos2::new(rect.min.x, rect.max.y - 16.0),
            Pos2::new(rect.max.x, rect.max.y),
        );
        painter.text(
            band.center(),
            egui::Align2::CENTER_CENTER,
            text,
            font,
            theme.neutral,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Parameters, Params};
    use nice_plug::context::gui::GuiContext;
    use nice_plug_core::context::PluginApi;
    use nice_plug_core::context::gui::GuiContextInner;
    use nice_plug_core::params::Params as ParamsTrait;
    use nice_plug_core::params::internals::ParamPtr;
    use nice_plug_core::plugin::PluginState;
    use rippler_params::params::RipplerParams;
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// A context that records the values the editor writes.
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

    /// The rectangle a knob is drawn in.
    fn knob_rect() -> Rect {
        Rect::from_min_size(Pos2::new(100.0, 100.0), vec2(KNOB_WIDTH, KNOB_HEIGHT))
    }

    /// The middle of a knob's circle.
    fn knob_centre() -> Pos2 {
        knob_rect().center() + vec2(0.0, -4.0)
    }

    /// The input of a frame of the editor.
    fn input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(650.0, 485.0))),
            ..Default::default()
        }
    }

    /// Draws a knob in a frame.
    ///
    /// The textures of the frame are never painted, so their deltas are
    /// dropped on purpose.
    fn run_frame(
        context: &egui::Context,
        input: egui::RawInput,
        params: &Params,
        id: &'static str,
        format: LabelFormat,
    ) {
        let mut output = context.run_ui(input, |ui| {
            Knob::new(params, id, "Knob", format).ui_at(ui, knob_rect());
        });
        output.textures_delta.clear();
    }

    /// Drags a knob and returns the values that were written.
    fn drag(
        id: &'static str,
        format: LabelFormat,
        modifiers: egui::Modifiers,
        distance: f32,
    ) -> Vec<(String, f32)> {
        let context = egui::Context::default();
        let parameters = Box::new(RipplerParams::default());
        let map = Parameters::new(&parameters);
        let written = Arc::new(Mutex::new(Vec::new()));
        let gui = GuiContext::new(Arc::new(Mock::new(&parameters, written.clone())));
        let setter = gui.param_setter();
        let params = Params::new(&parameters, &map, &setter);

        let centre = knob_centre();

        // The first frame draws the knob, so that the pointer finds it.
        run_frame(&context, input(), &params, id, format);

        // The second frame presses the mouse on the knob.
        let mut press = input();
        press.events.push(egui::Event::ModifiersChanged(modifiers));
        press.events.push(egui::Event::PointerMoved(centre));
        press.events.push(egui::Event::PointerButton {
            pos: centre,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers,
        });
        run_frame(&context, press, &params, id, format);

        // The third frame moves the mouse, and the knob follows it.
        let mut moving = input();
        moving.events.push(egui::Event::ModifiersChanged(modifiers));
        moving
            .events
            .push(egui::Event::PointerMoved(centre + vec2(distance, 0.0)));
        run_frame(&context, moving, &params, id, format);

        // And the fourth one lets go of the mouse.
        let mut release = input();
        release.events.push(egui::Event::PointerButton {
            pos: centre + vec2(distance, 0.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers,
        });
        run_frame(&context, release, &params, id, format);

        written.lock().expect("the values are not poisoned").clone()
    }

    /// Scrolls the wheel over a knob and returns the values that were
    /// written.
    fn wheel(id: &'static str, format: LabelFormat, initial: Option<f64>) -> Vec<(String, f32)> {
        let context = egui::Context::default();
        let parameters = Box::new(RipplerParams::default());
        let map = Parameters::new(&parameters);
        let written = Arc::new(Mutex::new(Vec::new()));
        let gui = GuiContext::new(Arc::new(Mock::new(&parameters, written.clone())));
        let setter = gui.param_setter();
        let params = Params::new(&parameters, &map, &setter);

        let centre = knob_centre();

        // The parameter starts at the value the test wants it at.
        if let Some(value) = initial {
            params.set_plain(id, value);
            written.lock().expect("the values are not poisoned").clear();
        }

        // The first frame draws the knob, so that the pointer finds it.
        run_frame(&context, input(), &params, id, format);

        // The second frame moves the mouse over the knob.
        let mut hovering = input();
        hovering.events.push(egui::Event::PointerMoved(centre));
        run_frame(&context, hovering, &params, id, format);

        // And the third one turns the wheel.
        let mut turning = input();
        turning.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::default(),
        });
        run_frame(&context, turning, &params, id, format);

        written.lock().expect("the values are not poisoned").clone()
    }

    #[test]
    fn the_arc_sweeps_two_hundred_and_sixty_degrees() {
        assert_eq!(value_angle_degrees(0.0), -130.0);
        assert_eq!(value_angle_degrees(1.0), 130.0);
        assert_eq!(value_angle_degrees(0.5), 0.0);
        assert_eq!(
            value_angle_degrees(1.0) - value_angle_degrees(0.0),
            SWEEP_DEGREES
        );
    }

    #[test]
    fn the_value_formats_match_the_cpp() {
        assert_eq!(LabelFormat::Hz.format(1234.0), "1.2 kHz");
        assert_eq!(LabelFormat::Hz.format(500.0), "500 Hz");
        assert_eq!(LabelFormat::Db.format(6.0), "6 dB");
        assert_eq!(LabelFormat::Percent.format(0.5), "50 %");
        assert_eq!(LabelFormat::Float1.format(1.25), "1.2");
        assert_eq!(LabelFormat::Float2Times100.format(0.1234), "12.34");
        assert_eq!(LabelFormat::Millis.format(500.0), "500 ms");
        assert_eq!(LabelFormat::Millis.format(1000.0), "1.00 s");
        assert_eq!(LabelFormat::Seconds2f.format(1.2), "1.20 s");
        assert_eq!(LabelFormat::AbMix.format(0.5), "50:50");
        assert_eq!(LabelFormat::PitchSemis.format(12.0), "12 Sem");
    }

    #[test]
    fn the_filter_format_reports_the_frequency() {
        assert_eq!(LabelFormat::FilterLphp.format(0.0), "Off");
        assert_eq!(LabelFormat::FilterLphp.format(0.5), "HP 632 Hz");
        assert_eq!(LabelFormat::FilterLphp.format(-0.5), "LP 632 Hz");
        assert_eq!(LabelFormat::FilterLphp.format(1.0), "HP 20.0 kHz");
        assert_eq!(LabelFormat::FilterLphp.format(-1.0), "LP 20 Hz");
    }

    #[test]
    fn the_ab_mix_format_is_complementary() {
        assert_eq!(LabelFormat::AbMix.format(0.0), "100:0");
        assert_eq!(LabelFormat::AbMix.format(0.25), "75:25");
        assert_eq!(LabelFormat::AbMix.format(1.0), "0:100");
    }

    #[test]
    fn dragging_a_pitch_control_changes_it() {
        let written = drag(
            "mallet_pitch",
            LabelFormat::PitchSemis,
            egui::Modifiers::default(),
            100.0,
        );

        assert!(!written.is_empty(), "the drag did not write anything");
        let (id, value) = written.last().expect("the value was written");
        assert_eq!(id, "mallet_pitch");
        assert!(*value != 0.0, "the drag did not change the value");
    }

    #[test]
    fn a_pitch_control_snaps_to_whole_semitones() {
        // A drag of a hundred points is a quarter of the knob's range,
        // which is twelve semitones.
        let written = drag(
            "mallet_pitch",
            LabelFormat::PitchSemis,
            egui::Modifiers::default(),
            100.0,
        );

        let (_, value) = written.last().expect("the value was written");
        assert_eq!(*value, 12.0);
    }

    #[test]
    fn a_pitch_control_does_not_snap_when_the_control_key_is_held() {
        // The control key makes the knob ten times finer, so the drag is
        // worth a semitone and a fifth.
        let written = drag(
            "mallet_pitch",
            LabelFormat::PitchSemis,
            egui::Modifiers {
                ctrl: true,
                ..Default::default()
            },
            100.0,
        );

        let (_, value) = written.last().expect("the value was written");
        assert!((value - 1.2).abs() < 0.001, "the value is {value}, not 1.2");
    }

    #[test]
    fn the_wheel_moves_a_value_that_is_at_zero() {
        // The JUCE build has a fix for the wheel not working when the value
        // is zero: the step is added again and again until the value moves.
        let written = wheel("a_decay", LabelFormat::Seconds2f, Some(0.01));

        let (id, value) = written.last().expect("the wheel wrote something");
        assert_eq!(id, "a_decay");
        assert!(*value > 0.01, "the wheel did not move the value");
    }

    #[test]
    fn the_wheel_does_not_compound_its_step() {
        // The step of the wheel is added to the value the parameter has,
        // and not to the step before it, so the fix walks the steps of five
        // percent instead of jumping over one of them.
        let written = wheel("a_decay", LabelFormat::Seconds2f, Some(0.01));

        assert_eq!(written.len(), 3, "the wheel wrote {written:?}");
    }
}
