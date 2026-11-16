//! The pitch control.
//!
//! Port of `src/ui/Pitch.cpp` and of `src/ui/Pitch.h`.
//!
//! The control shows a pitch in semitones. The left half of it edits the
//! coarse pitch, which is a whole number of semitones, and the right half
//! edits the fine pitch, which is a hundredth of a semitone. When the fine
//! pitch runs out, it rolls over into the coarse one.

use egui::{Align2, FontId, Rect, Response, Sense, Ui, Widget, vec2};

use crate::editor::Params;
use crate::editor::theme::Theme;

/// The width of the control, in points.
///
/// The JUCE editor's pitch controls are sixty points wide.
pub(crate) const PITCH_WIDTH: f32 = 60.0;

/// The height of the control, in points.
pub(crate) const PITCH_HEIGHT: f32 = 20.0;

/// The number of pixels a drag has to cover to move the control by one unit.
///
/// The JUCE build's `Pitch::mouseDrag` uses ten pixels per percent, which
/// is a tenth of what the knob uses.
const PIXELS_PER_UNIT: f32 = 10.0;

/// The parameter a pitch control edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PitchTarget {
    /// The coarse parameter, which is a whole number of semitones.
    Coarse,
    /// The fine parameter, which is a hundredth of a semitone.
    Fine,
}

/// The parameter a press at `x` in a control of `width` edits.
///
/// The left half of the control edits the coarse parameter and the right
/// half the fine one, the way the JUCE build does. A control without a fine
/// parameter only has the coarse one.
pub(crate) fn target_at(x: f32, width: f32, has_fine: bool) -> PitchTarget {
    if !has_fine || x < width / 2.0 {
        PitchTarget::Coarse
    } else {
        PitchTarget::Fine
    }
}

/// What a pitch control does when it is moved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PitchChange {
    /// The new value of the coarse parameter.
    pub coarse: f64,
    /// The new value of the fine parameter.
    pub fine: f64,
    /// The parameters that are written, and the values they are written with.
    pub writes: [(PitchTarget, f64); 2],
    /// How many of the writes there are.
    pub written: usize,
}

/// Moves a pitch control.
///
/// This is `Pitch::applyChange` in `src/ui/Pitch.cpp`. The coarse value is
/// a whole number of semitones and the fine value is a hundredth of one, so
/// the fine value rolls over into the coarse one when it runs out.
pub(crate) fn apply_change(
    coarse: f64,
    fine: f64,
    change: f64,
    target: PitchTarget,
    has_fine: bool,
) -> PitchChange {
    let mut coarse = coarse;
    let mut fine = fine;

    if target == PitchTarget::Coarse {
        coarse += change;
    } else {
        fine += change;
    }

    // A control without a fine parameter only has the coarse one, and it is
    // not snapped to whole semitones.
    if !has_fine {
        return PitchChange {
            coarse,
            fine,
            writes: [(PitchTarget::Coarse, coarse), (PitchTarget::Coarse, 0.0)],
            written: 1,
        };
    }

    let truncated = coarse.trunc();
    let runs_out = fine >= 100.0
        || fine <= -100.0
        || (truncated < 0.0 && fine >= 1.0)
        || (truncated > 0.0 && fine < 0.0);

    let mut writes = [(PitchTarget::Coarse, 0.0f64), (PitchTarget::Fine, 0.0f64)];
    let mut written = 0;

    if target == PitchTarget::Fine && runs_out {
        coarse = truncated;
        if fine >= 100.0 {
            coarse += 1.0;
            fine = 0.0;
        } else if fine <= -100.0 {
            coarse -= 1.0;
            fine = 0.0;
        } else if coarse > 0.0 {
            coarse += if fine >= 100.0 { 1.0 } else { -1.0 };
            fine = if fine < 0.0 { 99.0 } else { 0.0 };
        } else {
            coarse += if fine <= -100.0 { -1.0 } else { 1.0 };
            fine = if fine < -99.0 { 0.0 } else { -99.0 };
        }

        writes[written] = (PitchTarget::Coarse, coarse);
        written += 1;
    }

    let value = if target == PitchTarget::Coarse {
        coarse.trunc()
    } else {
        fine.trunc()
    };
    writes[written] = (target, value);
    written += 1;

    PitchChange {
        coarse,
        fine,
        writes,
        written,
    }
}

/// The text the control shows.
pub(crate) fn label(coarse: f64, fine: f64, has_fine: bool) -> String {
    let sign = if coarse < 0.0 || fine < 0.0 { "-" } else { "" };
    if has_fine {
        format!("{sign}{:.0}.{:02}", coarse.abs(), fine.abs().round() as i64)
    } else {
        format!("{sign}{:.0}", coarse.abs())
    }
}

/// A pitch control.
///
/// Port of `Pitch` in `src/ui/Pitch.cpp`.
pub(crate) struct Pitch<'a> {
    /// The parameters and the setter.
    params: &'a Params<'a>,
    /// The identifier of the coarse parameter.
    coarse: &'static str,
    /// The identifier of the fine parameter, if the control has one.
    fine: Option<&'static str>,
    /// The colours.
    theme: Theme,
}

impl<'a> Pitch<'a> {
    /// Builds a pitch control.
    pub(crate) fn new(
        params: &'a Params<'a>,
        coarse: &'static str,
        fine: Option<&'static str>,
    ) -> Self {
        Self {
            params,
            coarse,
            fine,
            theme: Theme::LIGHT,
        }
    }

    /// Sets the colours of the control.
    pub(crate) fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// The value the control shows.
    fn value(&self) -> (f64, f64) {
        let coarse = self.params.plain(self.coarse) as f64;
        let fine = match self.fine {
            Some(fine) => self.params.plain(fine) as f64,
            None => 0.0,
        };
        (coarse, fine)
    }

    /// Writes the values the control has.
    fn write(&self, change: &PitchChange) {
        for index in 0..change.written {
            let (target, value) = change.writes[index];
            let id = match target {
                PitchTarget::Coarse => self.coarse,
                PitchTarget::Fine => self.fine.unwrap_or(self.coarse),
            };
            self.params.begin(id);
            self.params.set_plain(id, value);
            self.params.end(id);
        }
    }
}

impl Widget for Pitch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(PITCH_WIDTH, PITCH_HEIGHT), Sense::click_and_drag());

        let modifiers = ui.input(|input| input.modifiers);
        let has_fine = self.fine.is_some();

        // The drag is remembered in egui's memory, because the control is
        // built again every frame.
        let id = ui.next_auto_id().with(("pitch", self.coarse));
        let target_id = id.with("target");
        let coarse_id = id.with("coarse");
        let fine_id = id.with("fine");

        if response.drag_started() {
            let target = target_at(
                response
                    .interact_pointer_pos()
                    .map_or(0.0, |position| position.x - rect.left()),
                rect.width(),
                has_fine,
            );
            let (coarse, fine) = self.value();
            ui.memory_mut(|memory| {
                memory.data.insert_temp(target_id, target);
                memory.data.insert_temp(coarse_id, coarse);
                memory.data.insert_temp(fine_id, fine);
            });
        }

        let target: PitchTarget = ui
            .memory(|memory| memory.data.get_temp::<PitchTarget>(target_id))
            .unwrap_or(PitchTarget::Coarse);
        let coarse: f64 = ui
            .memory(|memory| memory.data.get_temp::<f64>(coarse_id))
            .unwrap_or(0.0);
        let fine: f64 = ui
            .memory(|memory| memory.data.get_temp::<f64>(fine_id))
            .unwrap_or(0.0);

        if response.dragged() {
            // The control moves with the vertical movement of the mouse.
            let speed = if modifiers.ctrl { 40.0 } else { 4.0 } * PIXELS_PER_UNIT;
            let change = (-response.drag_delta().y / speed) as f64;
            let result = apply_change(coarse, fine, change, target, has_fine);
            self.write(&result);
            ui.memory_mut(|memory| {
                memory.data.insert_temp(coarse_id, result.coarse);
                memory.data.insert_temp(fine_id, result.fine);
            });
        }

        if response.double_clicked() {
            let id = match target {
                PitchTarget::Coarse => self.coarse,
                PitchTarget::Fine => self.fine.unwrap_or(self.coarse),
            };
            self.params.begin(id);
            self.params
                .set_normalized(id, self.params.default_normalized(id));
            self.params.end(id);
        }

        // The wheel moves the control by a whole unit.
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
                let change = if wheel > 0.0 { 1.0 } else { -1.0 };
                let (coarse, fine) = self.value();
                let result = apply_change(coarse, fine, change, target, has_fine);
                self.write(&result);
            }
        }

        self.paint(ui, rect);

        response
    }
}

impl Pitch<'_> {
    /// Draws the control.
    fn paint(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();
        let theme = &self.theme;

        painter.rect_filled(rect, 0.0, theme.background);
        painter.rect_filled(rect, 2.0, theme.active);

        let (coarse, fine) = self.value();
        let text = label(coarse, fine, self.fine.is_some());

        // A control with a fine parameter leaves room on the right for the
        // fine part of the pitch.
        let area = match self.fine {
            Some(_) => Rect::from_min_max(
                rect.left_top(),
                egui::Pos2::new(rect.right() - 15.0, rect.bottom()),
            ),
            None => rect,
        };

        // A control with a fine parameter shows the pitch on the right, and
        // a control without one shows it in the middle.
        let (position, anchor) = match self.fine {
            Some(_) => (area.right_center(), Align2::RIGHT_CENTER),
            None => (area.center(), Align2::CENTER_CENTER),
        };

        painter.text(
            position,
            anchor,
            text,
            FontId::proportional(15.0),
            theme.on_active(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pitch_control_switches_to_the_fine_parameter_at_the_coarse_limit() {
        // The right half of the control edits the fine parameter, and the
        // fine parameter takes over when the coarse one has run out.
        assert_eq!(target_at(10.0, 60.0, true), PitchTarget::Coarse);
        assert_eq!(target_at(40.0, 60.0, true), PitchTarget::Fine);
        // A control without a fine parameter only has the coarse one.
        assert_eq!(target_at(40.0, 60.0, false), PitchTarget::Coarse);

        // The fine value rolls over into the coarse one when it runs out.
        let change = apply_change(0.0, 99.5, 1.0, PitchTarget::Fine, true);
        assert_eq!(change.coarse, 1.0);
        assert_eq!(change.fine, 0.0);
        assert_eq!(change.writes[0], (PitchTarget::Coarse, 1.0));

        // The other way round, the fine value follows the coarse one down.
        let change = apply_change(1.0, -0.5, 0.0, PitchTarget::Fine, true);
        assert_eq!(change.coarse, 0.0);
        assert_eq!(change.fine, 99.0);
    }

    #[test]
    fn the_coarse_value_is_a_whole_number_of_semitones() {
        let change = apply_change(0.0, 0.0, 1.4, PitchTarget::Coarse, true);
        assert_eq!(change.writes[0], (PitchTarget::Coarse, 1.0));
    }

    #[test]
    fn a_control_without_a_fine_parameter_is_not_snapped() {
        let change = apply_change(2.0, 0.0, 0.5, PitchTarget::Coarse, false);
        assert_eq!(change.writes[0], (PitchTarget::Coarse, 2.5));
    }

    #[test]
    fn the_label_shows_the_pitch_in_semitones() {
        assert_eq!(label(3.0, 0.0, true), "3.00");
        assert_eq!(label(-3.0, 50.0, true), "-3.50");
        assert_eq!(label(3.0, 0.0, false), "3");
        assert_eq!(label(-3.0, 0.0, false), "-3");
    }
}
