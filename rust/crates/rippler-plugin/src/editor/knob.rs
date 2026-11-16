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

use egui::{Color32, FontId, Painter, Pos2, Response, Sense, Shape, Stroke, Ui, Widget, vec2};

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
const RADIUS: f32 = 16.0;

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
        let (rect, response) =
            ui.allocate_exact_size(vec2(KNOB_WIDTH, KNOB_HEIGHT), Sense::click_and_drag());

        let modifiers = ui.input(|input| input.modifiers);
        let shift = modifiers.shift;
        let target = self.target(shift);

        let normalised = self.params.normalized(target);
        let plain = self.params.plain(target);

        // The drag is remembered in egui's memory, because the knob is built
        // again every frame and cannot keep the state of the drag itself.
        let memory = ui.next_auto_id().with(("knob", self.id));
        let start_id = memory.with("start");
        let sum_id = memory.with("sum");

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
            // is held.
            let value = if self.format == LabelFormat::PitchSemis && !modifiers.ctrl {
                let whole = self.params.plain(target).round();
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
                let mut value = self.params.normalized(target);

                self.params.begin(target);
                self.params
                    .set_normalized(target, (value + change).clamp(0.0, 1.0));

                let mut steps = 0;
                while wheel > 0.0 && self.params.normalized(target) == 0.0 && steps < 100 {
                    change += 0.05;
                    value = (value + change).clamp(0.0, 1.0);
                    self.params.set_normalized(target, value);
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

        // The drop shadow.
        let shadow = theme.shadow;
        for (grow, alpha) in [(1.0f32, 0.10f32), (2.5, 0.07), (4.0, 0.04)] {
            painter.circle_filled(
                centre + vec2(4.0, 4.0),
                RADIUS + grow,
                Color32::from_rgba_unmultiplied(
                    shadow.r(),
                    shadow.g(),
                    shadow.b(),
                    (alpha * 255.0) as u8,
                ),
            );
        }

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
}
