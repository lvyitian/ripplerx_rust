//! The editor's palette.
//!
//! The colours are the ones of `src/Globals.h`, the light ones and the dark
//! ones. The JUCE build keeps a single set of global variables and rewrites
//! them when the theme is toggled; this build keeps both palettes and picks
//! one, which is the same thing without the global state.

use egui::Color32;

/// The colours of the editor.
///
/// The five colours are the ones of `globals::COLOR_*` in `src/Globals.h`,
/// and the shadow is the colour the knob's drop shadow is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// The editor's background.
    pub background: Color32,
    /// The colour of the knob's pointer and of the pitch control's text.
    pub neutral: Color32,
    /// The colour of the section labels and of the unlit meter bulbs.
    pub neutral_light: Color32,
    /// The colour of the arcs, of the buttons and of the meter.
    pub active: Color32,
    /// The colour of the velocity arcs.
    pub velocity: Color32,
    /// The colour of the knob's drop shadow.
    pub shadow: Color32,
}

impl Theme {
    /// The light palette.
    pub const LIGHT: Theme = Theme {
        background: Color32::from_rgb(0xee, 0xee, 0xee),
        neutral: Color32::from_rgb(0x33, 0x33, 0x33),
        neutral_light: Color32::from_rgb(0x66, 0x66, 0x66),
        active: Color32::from_rgb(0x00, 0x80, 0x70),
        velocity: Color32::from_rgb(0xaa, 0x50, 0x50),
        shadow: Color32::from_rgb(0x88, 0x88, 0x88),
    };

    /// The dark palette.
    pub const DARK: Theme = Theme {
        background: Color32::from_rgb(0x42, 0x42, 0x42),
        neutral: Color32::from_rgb(0xee, 0xee, 0xee),
        neutral_light: Color32::from_rgb(0xb1, 0xb1, 0xb1),
        active: Color32::from_rgb(0x00, 0xcc, 0xab),
        velocity: Color32::from_rgb(0xd7, 0x64, 0x64),
        shadow: Color32::from_rgb(0x00, 0x00, 0x00),
    };

    /// The palette of the light or of the dark theme.
    pub fn new(dark: bool) -> Self {
        if dark { Self::DARK } else { Self::LIGHT }
    }

    /// Whether this is the dark palette.
    pub fn is_dark(&self) -> bool {
        *self == Self::DARK
    }

    /// The theme with every colour half as opaque.
    ///
    /// The JUCE build dims the parts of a resonator that is switched off.
    pub fn dimmed(&self) -> Theme {
        Theme {
            background: dimmed(self.background),
            neutral: dimmed(self.neutral),
            neutral_light: dimmed(self.neutral_light),
            active: dimmed(self.active),
            velocity: dimmed(self.velocity),
            shadow: dimmed(self.shadow),
        }
    }

    /// The colour of the text on top of the active colour.
    ///
    /// The JUCE build uses white on the light theme and a very dark shade of
    /// the background on the dark one.
    pub fn on_active(&self) -> Color32 {
        if self.is_dark() {
            darker(self.background, 0.7)
        } else {
            Color32::WHITE
        }
    }
}

/// Halves the opacity of a colour.
fn dimmed(colour: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        colour.r(),
        colour.g(),
        colour.b(),
        (colour.a() as f32 * 0.5) as u8,
    )
}

/// Darkens a colour the way JUCE's `Colour::darker` does.
///
/// The amount is the fraction the colour is darkened by, so a colour
/// darkened by `0.7` is three tenths as bright as it was.
pub fn darker(color: Color32, amount: f32) -> Color32 {
    let factor = (1.0 - amount).clamp(0.0, 1.0);
    Color32::from_rgb(
        (color.r() as f32 * factor) as u8,
        (color.g() as f32 * factor) as u8,
        (color.b() as f32 * factor) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_light_palette_is_the_one_of_the_juce_build() {
        let theme = Theme::new(false);
        assert_eq!(theme.background, Color32::from_rgb(0xee, 0xee, 0xee));
        assert_eq!(theme.neutral, Color32::from_rgb(0x33, 0x33, 0x33));
        assert_eq!(theme.neutral_light, Color32::from_rgb(0x66, 0x66, 0x66));
        assert_eq!(theme.active, Color32::from_rgb(0x00, 0x80, 0x70));
        assert_eq!(theme.velocity, Color32::from_rgb(0xaa, 0x50, 0x50));
    }

    #[test]
    fn the_dark_palette_is_the_one_of_the_juce_build() {
        let theme = Theme::new(true);
        assert_eq!(theme.background, Color32::from_rgb(0x42, 0x42, 0x42));
        assert_eq!(theme.neutral, Color32::from_rgb(0xee, 0xee, 0xee));
        assert_eq!(theme.neutral_light, Color32::from_rgb(0xb1, 0xb1, 0xb1));
        assert_eq!(theme.active, Color32::from_rgb(0x00, 0xcc, 0xab));
        assert_eq!(theme.velocity, Color32::from_rgb(0xd7, 0x64, 0x64));
    }

    #[test]
    fn the_theme_is_told_apart_by_its_background() {
        assert!(Theme::new(true).is_dark());
        assert!(!Theme::new(false).is_dark());
    }
}
