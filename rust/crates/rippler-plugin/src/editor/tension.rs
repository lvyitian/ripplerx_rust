//! The tension control.
//!
//! Port of `src/ui/TensionCtrl.cpp` and of `src/ui/TensionCtrl.h`.
//!
//! The control is a small square with a quadratic Bézier curve in it. The
//! curve's control point follows the parameter, and the parameter runs from
//! minus one to one.

use egui::{Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget, vec2};

use crate::editor::Params;
use crate::editor::theme::Theme;

/// The width of the control, in points.
///
/// The JUCE editor's tension controls are fifteen points wide.
pub(crate) const TENSION_SIZE: f32 = 15.0;

/// The number of pixels a drag has to cover to move the control by one
/// percent.
///
/// The JUCE build's `TensionCtrl::pixels_per_percent`.
const PIXELS_PER_PERCENT: f32 = 100.0;

/// Normalises a tension value from minus one to one into zero to one.
///
/// The JUCE build does the same in `TensionCtrl::paint`.
pub(crate) fn normalise(value: f32) -> f32 {
    (value + 1.0) * 0.5
}

/// The point the curve starts at.
pub(crate) fn curve_start(rect: Rect, invert: bool) -> Pos2 {
    if invert {
        rect.left_top()
    } else {
        rect.left_bottom()
    }
}

/// The point the curve ends at.
pub(crate) fn curve_end(rect: Rect, invert: bool) -> Pos2 {
    if invert {
        rect.right_bottom()
    } else {
        rect.right_top()
    }
}

/// The control point of the curve.
///
/// The value is normalised from zero to one. A curve that is not inverted
/// runs from the bottom left to the top right, and its control point travels
/// from the top left to the bottom right as the value grows. A curve that is
/// inverted runs from the top left to the bottom right, and its control point
/// travels the other way round.
pub(crate) fn control_point(rect: Rect, value: f32, invert: bool) -> Pos2 {
    let top_left = rect.left_top();
    let bottom_right = rect.right_bottom();
    let width = bottom_right.x - top_left.x;
    let height = bottom_right.y - top_left.y;

    if invert {
        Pos2::new(
            top_left.x + width * (1.0 - value),
            bottom_right.y - height * (1.0 - value),
        )
    } else {
        Pos2::new(top_left.x + width * value, top_left.y + height * value)
    }
}

/// A tension control.
///
/// Port of `TensionCtrl` in `src/ui/TensionCtrl.cpp`.
pub(crate) struct Tension<'a> {
    /// The parameters and the setter.
    params: &'a Params<'a>,
    /// The identifier of the parameter the control edits.
    id: &'static str,
    /// Whether the curve is inverted.
    invert: bool,
    /// The colours.
    theme: Theme,
}

impl<'a> Tension<'a> {
    /// Builds a tension control.
    pub(crate) fn new(params: &'a Params<'a>, id: &'static str, invert: bool) -> Self {
        Self {
            params,
            id,
            invert,
            theme: Theme::LIGHT,
        }
    }

    /// Sets the colours of the control.
    pub(crate) fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for Tension<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(TENSION_SIZE, TENSION_SIZE), Sense::click_and_drag());

        let modifiers = ui.input(|input| input.modifiers);
        let normalised = self.params.normalized(self.id);

        // The drag is remembered in egui's memory, because the control is
        // built again every frame.
        let memory = ui.next_auto_id().with(("tension", self.id));
        let start_id = memory.with("start");
        let sum_id = memory.with("sum");

        if response.drag_started() {
            self.params.begin(self.id);
            ui.memory_mut(|memory| {
                memory.data.insert_temp(start_id, normalised);
                memory.data.insert_temp(sum_id, 0.0f32);
            });
        }

        if response.dragged() {
            // The change is the horizontal and the vertical movement of the
            // mouse, and it is subtracted from the value.
            let speed = if modifiers.ctrl { 40.0 } else { 4.0 } * PIXELS_PER_PERCENT;
            let delta = response.drag_delta();
            let change = (delta.x - delta.y) / speed;

            let sum =
                ui.memory(|memory| memory.data.get_temp::<f32>(sum_id).unwrap_or(0.0)) + change;
            ui.memory_mut(|memory| memory.data.insert_temp(sum_id, sum));

            let start =
                ui.memory(|memory| memory.data.get_temp::<f32>(start_id).unwrap_or(normalised));
            let value = (start - sum).clamp(0.0, 1.0);
            self.params.set_normalized(self.id, value);
        }

        if response.drag_stopped() {
            self.params.end(self.id);
        }

        if response.double_clicked() {
            self.params.begin(self.id);
            self.params
                .set_normalized(self.id, self.params.default_normalized(self.id));
            self.params.end(self.id);
        }

        self.paint(ui, rect);

        response
    }
}

impl Tension<'_> {
    /// Draws the control.
    fn paint(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();
        let theme = &self.theme;

        // The background of the control is black at a tenth of its opacity.
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 26));

        // The value of the parameter, normalised from minus one to one.
        let value = normalise(self.params.plain(self.id));

        let curve = egui::epaint::QuadraticBezierShape::from_points_stroke(
            [
                curve_start(rect, self.invert),
                control_point(rect, value, self.invert),
                curve_end(rect, self.invert),
            ],
            false,
            Color32::TRANSPARENT,
            Stroke::new(1.0, theme.neutral_light),
        );
        painter.add(Shape::QuadraticBezier(curve));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rectangle the tests draw in.
    fn rect() -> Rect {
        Rect::from_min_size(Pos2::new(0.0, 0.0), vec2(10.0, 20.0))
    }

    #[test]
    fn the_control_point_follows_the_value() {
        // The curve that is not inverted runs from the bottom left to the
        // top right, and its control point travels from the top left to the
        // bottom right.
        let plain = control_point(rect(), 0.0, false);
        assert_eq!(plain, rect().left_top());
        let plain = control_point(rect(), 1.0, false);
        assert_eq!(plain, rect().right_bottom());

        // The curve that is inverted runs from the top left to the bottom
        // right, and its control point travels from the top right to the
        // bottom left.
        let inverted = control_point(rect(), 0.0, true);
        assert_eq!(inverted, Pos2::new(rect().right(), rect().top()));
        let inverted = control_point(rect(), 1.0, true);
        assert_eq!(inverted, Pos2::new(rect().left(), rect().bottom()));
    }

    #[test]
    fn the_value_is_normalised_from_minus_one_to_one() {
        assert_eq!(normalise(-1.0), 0.0);
        assert_eq!(normalise(0.0), 0.5);
        assert_eq!(normalise(1.0), 1.0);
    }

    #[test]
    fn the_curve_runs_from_corner_to_corner() {
        assert_eq!(curve_start(rect(), false), rect().left_bottom());
        assert_eq!(curve_end(rect(), false), rect().right_top());
        assert_eq!(curve_start(rect(), true), rect().left_top());
        assert_eq!(curve_end(rect(), true), rect().right_bottom());
    }
}
