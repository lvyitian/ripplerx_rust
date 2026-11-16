//! The settings that are not plugin parameters.
//!
//! The JUCE build keeps three things out of its parameters: the theme, the
//! scale of the user interface and the polyphony. They live in a settings
//! file, which `RipplerXAudioProcessor::loadSettings` reads and
//! `RipplerXAudioProcessor::saveSettings` writes.
//!
//! This build does the same. The settings live in a small JSON file in the
//! user's configuration directory, and the polyphony is shared with the audio
//! thread through an atomic, because the audio thread may not lock a mutex.
//!
//! The file is the plugin's own, and it is written when a setting changes.
//! A file that cannot be read or written is not an error: the plugin falls
//! back to the defaults, exactly as the JUCE build does when its settings
//! file is missing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The number of voices the plugin plays at most.
///
/// `globals::MAX_POLYPHONY` in `src/Globals.h`.
pub const MAX_POLYPHONY: usize = 16;

/// The number of voices the plugin plays when nothing says otherwise.
///
/// The JUCE build reads eight out of its settings file.
pub const DEFAULT_POLYPHONY: usize = 8;

/// The scales the user interface can be shown at.
///
/// The JUCE build's settings menu offers exactly these five.
pub const UI_SCALES: [f32; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

/// The settings that are not plugin parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Whether the dark theme is on.
    pub dark_theme: bool,
    /// The scale of the user interface.
    pub ui_scale: f32,
    /// The number of voices the plugin plays.
    pub polyphony: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark_theme: false,
            ui_scale: 1.0,
            polyphony: DEFAULT_POLYPHONY,
        }
    }
}

impl Settings {
    /// Reads the settings from a file.
    ///
    /// A file that is missing, that cannot be read and that cannot be parsed
    /// all give the defaults, because a plugin that refuses to start over a
    /// settings file is worse than a plugin that forgets a setting.
    pub fn read_from(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(settings) = serde_json::from_str::<Self>(&text) else {
            return Self::default();
        };
        settings.sanitised()
    }

    /// Writes the settings to a file.
    ///
    /// The directory is created if it is not there. The file is written to a
    /// temporary file in the same directory and that file is renamed over
    /// the settings, which is atomic: two plugin instances that write their
    /// settings at the same time cannot leave a half-written file behind,
    /// which would silently revert the settings to the defaults.
    ///
    /// A file that cannot be written is ignored: the settings simply do not
    /// survive the session.
    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

        let temporary = temporary_path(path);
        std::fs::write(&temporary, text)?;
        if let Err(error) = std::fs::rename(&temporary, path) {
            // The temporary file is of no use any more.
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(())
    }

    /// Brings the settings back into range.
    ///
    /// A file that was written by another build, or that a user edited, can
    /// hold a polyphony of zero or a scale of zero. The polyphony is
    /// clamped to the range the plugin knows and the scale is snapped to one
    /// of the scales the menu offers.
    fn sanitised(self) -> Self {
        let polyphony = self.polyphony.clamp(1, MAX_POLYPHONY);
        let ui_scale = UI_SCALES
            .iter()
            .copied()
            .min_by(|a, b| {
                (a - self.ui_scale)
                    .abs()
                    .partial_cmp(&(b - self.ui_scale).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(1.0);
        Self {
            dark_theme: self.dark_theme,
            ui_scale,
            polyphony,
        }
    }
}

/// The path of the temporary file the settings are written to.
///
/// The file is in the same directory as the settings, so that renaming it
/// over them stays on the same file system and is therefore atomic.
fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("settings.json"));
    name.push_str(".tmp");
    path.with_file_name(name)
}

/// The path of the settings file.
///
/// The file lives in the user's configuration directory: the `APPDATA`
/// directory on Windows, the `XDG_CONFIG_HOME` directory on Linux and the
/// `Library/Application Support` directory on macOS.
pub fn settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(
                PathBuf::from(appdata)
                    .join("RipplerX")
                    .join("settings.json"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Some(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("RipplerX")
                    .join("settings.json"),
            );
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(config) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(config).join("ripplerx").join("settings.json"));
        }
        if let Some(home) = std::env::var_os("HOME") {
            return Some(
                PathBuf::from(home)
                    .join(".config")
                    .join("ripplerx")
                    .join("settings.json"),
            );
        }
    }

    None
}

/// Reads the settings from the user's settings file.
///
/// A file that cannot be found gives the defaults.
pub fn load() -> Settings {
    match settings_path() {
        Some(path) => Settings::read_from(&path),
        None => Settings::default(),
    }
}

/// Writes the settings to the user's settings file.
///
/// A file that cannot be written is ignored.
pub fn save(settings: &Settings) {
    if let Some(path) = settings_path() {
        let _ = settings.write_to(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory that is thrown away when the test is over.
    fn temporary_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("ripplerx-settings-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory is made");
        directory
    }

    #[test]
    fn the_defaults_are_the_ones_of_the_juce_build() {
        let settings = Settings::default();
        assert!(!settings.dark_theme);
        assert_eq!(settings.ui_scale, 1.0);
        assert_eq!(settings.polyphony, DEFAULT_POLYPHONY);
    }

    #[test]
    fn the_settings_round_trip_through_the_file() {
        let directory = temporary_directory("round-trip");
        let path = directory.join("settings.json");

        let settings = Settings {
            dark_theme: true,
            ui_scale: 1.5,
            polyphony: 12,
        };
        settings.write_to(&path).expect("the settings are written");

        assert_eq!(Settings::read_from(&path), settings);
    }

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let directory = temporary_directory("missing");
        let path = directory.join("settings.json");
        assert_eq!(Settings::read_from(&path), Settings::default());
    }

    #[test]
    fn a_file_that_is_not_json_gives_the_defaults() {
        let directory = temporary_directory("broken");
        let path = directory.join("settings.json");
        std::fs::write(&path, "this is not json").expect("the file is written");
        assert_eq!(Settings::read_from(&path), Settings::default());
    }

    #[test]
    fn a_polyphony_out_of_range_is_brought_back() {
        let settings = Settings {
            dark_theme: false,
            ui_scale: 1.0,
            polyphony: 99,
        };
        assert_eq!(settings.sanitised().polyphony, MAX_POLYPHONY);

        let settings = Settings {
            dark_theme: false,
            ui_scale: 1.0,
            polyphony: 0,
        };
        assert_eq!(settings.sanitised().polyphony, 1);
    }

    #[test]
    fn a_scale_that_is_not_offered_is_snapped_to_one_that_is() {
        let settings = Settings {
            dark_theme: false,
            ui_scale: 1.3,
            polyphony: 8,
        };
        assert_eq!(settings.sanitised().ui_scale, 1.25);
    }

    #[test]
    fn the_settings_are_written_atomically() {
        let directory = temporary_directory("atomic");
        let path = directory.join("settings.json");

        let settings = Settings {
            dark_theme: true,
            ui_scale: 1.5,
            polyphony: 12,
        };
        settings.write_to(&path).expect("the settings are written");

        // The file that was written is the whole of the settings...
        assert_eq!(Settings::read_from(&path), settings);

        // ...and the temporary file it was written through is gone.
        let files: Vec<String> = std::fs::read_dir(&directory)
            .expect("the directory can be read")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(files, ["settings.json"]);
    }
}
