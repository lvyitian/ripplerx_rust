//! The meter.
//!
//! Port of `src/ui/Meter.cpp` and of `src/ui/Meter.h`.
//!
//! The meter is a row of four bulbs that light up with the level of the
//! output. The bulbs are green at the bottom and red at the top, and they
//! stay on for a quarter of a second after the level drops, so that they do
//! not flicker.

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Widget, vec2};

use crate::editor::theme::Theme;

/// The number of bulbs the meter has.
///
/// The JUCE build's `Meter::totalBulbs`.
pub(crate) const TOTAL_BULBS: usize = 4;

/// The level at which every bulb is on.
///
/// The JUCE build compares the level with `0.8`, because there is a limiter
/// in front of the meter that keeps the level below that.
const FULL_LEVEL: f32 = 0.8;

/// The colour of a bulb that is on.
///
/// The JUCE build's bulbs are green at the bottom and red at the top, with
/// a yellow in the middle.
const BULB_COLOURS: [Color32; TOTAL_BULBS] = [
    Color32::from_rgb(0, 255, 0),
    Color32::from_rgb(170, 255, 0),
    Color32::from_rgb(255, 170, 0),
    Color32::from_rgb(255, 0, 0),
];

/// The number of ticks a bulb stays on after the level drops.
///
/// The JUCE build's `Bulb::elapsed`.
const BULB_ELAPSED: i32 = 6;

/// The level the meter shows, as the JUCE build measures it.
///
/// The level is the root mean square of the output, and the meter draws its
/// fourth root of it, which is what makes the meter look the way it does.
pub(crate) fn level(rms: f32) -> f32 {
    rms.min(1.0).powf(0.25)
}

/// The number of bulbs that are on at a level.
pub(crate) fn lit_bulbs(rms: f32) -> usize {
    let level = level(rms);
    (0..TOTAL_BULBS)
        .filter(|index| {
            level >= (*index + 1) as f32 / (TOTAL_BULBS + 1) as f32 || level >= FULL_LEVEL
        })
        .count()
}

/// The colour of a bulb that is off.
fn unlit_colour(theme: &Theme) -> Color32 {
    if theme.is_dark() {
        crate::editor::theme::darker(theme.background, 1.0)
    } else {
        Color32::from_rgb(0x44, 0x44, 0x44)
    }
}

/// The meter.
pub(crate) struct Meter {
    /// The level of the output.
    level: f32,
    /// The colours.
    theme: Theme,
}

impl Meter {
    /// Builds a meter.
    pub(crate) fn new(level: f32) -> Self {
        Self {
            level,
            theme: Theme::LIGHT,
        }
    }

    /// Sets the colours of the meter.
    pub(crate) fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for Meter {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(60.0, 55.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

        // The bulbs stay on for a moment after the level drops, which is
        // what keeps them from flickering. The counters live in egui's
        // memory, because the meter is built again every frame.
        let id = ui.next_auto_id().with("meter");
        let mut elapsed: Vec<i32> =
            ui.memory(|memory| memory.data.get_temp::<Vec<i32>>(id).unwrap_or_default());
        while elapsed.len() < TOTAL_BULBS {
            elapsed.push(0);
        }

        let lit_count = lit_bulbs(self.level);
        let mut lit = [false; TOTAL_BULBS];
        for (index, bulb) in lit.iter_mut().enumerate() {
            let on = index < lit_count;
            // A bulb that was on stays on until its counter runs out.
            let was_on = elapsed[index] > 0;
            if on {
                elapsed[index] = BULB_ELAPSED;
            } else if elapsed[index] <= 0 {
                elapsed[index] = 0;
            } else {
                elapsed[index] -= 1;
            }
            *bulb = on || was_on;
        }
        ui.memory_mut(|memory| memory.data.insert_temp(id, elapsed.clone()));

        // The bulbs are stacked from the bottom up.
        let bulb_height = rect.height() / TOTAL_BULBS as f32;
        for (index, bulb) in lit.iter().enumerate() {
            let top = rect.bottom() - bulb_height * (index + 1) as f32;
            let bounds = Rect::from_min_max(
                Pos2::new(rect.left(), top),
                Pos2::new(rect.right(), top + bulb_height),
            );
            self.draw_bulb(ui, bounds, index, *bulb);
        }

        response
    }
}

impl Meter {
    /// Draws one bulb.
    fn draw_bulb(&self, ui: &mut Ui, bounds: Rect, index: usize, on: bool) {
        let painter = ui.painter();
        let theme = &self.theme;

        // The bulb is a circle in the middle of its part of the meter.
        let inner = bounds.shrink(3.0);
        let side = inner.width().min(inner.height());
        let centre = Pos2::new(inner.center().x, inner.top() + side / 2.0);
        let radius = side / 2.0;

        let colour = if on {
            BULB_COLOURS[index]
        } else {
            unlit_colour(theme)
        };

        if on {
            // The glow around a bulb that is on.
            for (grow, alpha) in [(2.0f32, 0.10f32), (4.0, 0.06), (6.0, 0.03)] {
                painter.circle_filled(
                    centre,
                    radius + grow,
                    Color32::from_rgba_unmultiplied(
                        colour.r(),
                        colour.g(),
                        colour.b(),
                        (alpha * 255.0) as u8,
                    ),
                );
            }
        }

        painter.circle_filled(centre, radius, colour);
        painter.circle_stroke(
            centre,
            radius,
            Stroke::new(
                1.0,
                if theme.is_dark() {
                    Color32::TRANSPARENT
                } else {
                    Color32::from_rgb(0xc0, 0xc0, 0xc0)
                },
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_meter_lights_the_right_number_of_bulbs() {
        assert_eq!(lit_bulbs(0.0), 0);
        // The level is drawn as its fourth root, so a hundredth of the level
        // lights one of the four bulbs and a tenth lights two of them.
        assert_eq!(lit_bulbs(0.01), 1);
        assert_eq!(lit_bulbs(0.1), 2);
        // Past four fifths of the level every bulb is on.
        assert_eq!(lit_bulbs(0.5), TOTAL_BULBS);
        assert_eq!(lit_bulbs(1.0), TOTAL_BULBS);
    }

    #[test]
    fn the_level_is_the_fourth_root_of_the_output() {
        assert_eq!(level(0.0), 0.0);
        assert_eq!(level(1.0), 1.0);
        assert!((level(0.0625) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_level_above_one_is_brought_back() {
        assert_eq!(level(4.0), 1.0);
    }
}
