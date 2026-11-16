//! The `.ripx` patch format.
//!
//! A `.ripx` file is a JSON document:
//!
//! ```json
//! {
//!   "version": 1,
//!   "params": {
//!     "a_decay": 10.78,
//!     "a_model": 1,
//!     "mallet_type": 0
//!   },
//!   "userSample": "<JUCE base64>",
//!   "currentProgram": 3
//! }
//! ```
//!
//! The `params` object holds **plain** values, keyed by the legacy
//! identifier, and the choice parameters hold their **index**, exactly as the
//! legacy state does. That keeps one conversion, not two.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::legacy::{LegacyPatch, to_parameter_values};

/// The version of the patch format this build reads and writes.
pub const PATCH_VERSION: u32 = 1;

/// A patch: the parameter values and the state that goes with them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    /// The version of the patch format.
    #[serde(default = "default_version")]
    pub version: u32,
    /// The parameter values, by legacy identifier.
    ///
    /// The values are plain, never normalised, and a choice parameter holds
    /// its index.
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
    /// The user sample, in JUCE's base64.
    #[serde(
        rename = "userSample",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub user_sample: Option<String>,
    /// The program the patch was on.
    #[serde(
        rename = "currentProgram",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub current_program: Option<i32>,
}

/// The version a patch without one holds.
fn default_version() -> u32 {
    PATCH_VERSION
}

/// What can go wrong when reading a patch.
#[derive(Debug)]
pub enum PatchError {
    /// The document is not valid JSON, or not a patch.
    InvalidJson(serde_json::Error),
    /// The document is a patch of a version this build does not know.
    UnsupportedVersion(u32),
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatchError::InvalidJson(error) => write!(f, "the patch is not valid JSON: {error}"),
            PatchError::UnsupportedVersion(version) => {
                write!(f, "the patch has the unsupported version {version}")
            }
        }
    }
}

impl std::error::Error for PatchError {}

/// Reads a `.ripx` patch.
pub fn read_patch(json: &str) -> Result<Patch, PatchError> {
    let patch: Patch = serde_json::from_str(json).map_err(PatchError::InvalidJson)?;
    if patch.version != PATCH_VERSION {
        return Err(PatchError::UnsupportedVersion(patch.version));
    }
    Ok(patch)
}

/// Writes a `.ripx` patch.
pub fn write_patch(patch: &Patch) -> String {
    serde_json::to_string_pretty(patch).unwrap_or_else(|_| String::from("{}"))
}

/// Converts a legacy patch into a patch.
///
/// The values are the plain ones the legacy state holds, with the `a_cut` and
/// `b_cut` migration applied and every value snapped to its parameter's step
/// size.
pub fn convert_legacy_patch(patch: &LegacyPatch) -> Patch {
    Patch {
        version: PATCH_VERSION,
        params: to_parameter_values(patch),
        user_sample: patch.user_sample.clone(),
        current_program: patch.current_program,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::read_legacy_patch;

    /// The factory presets of the repository.
    fn preset_directory() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/presets")
    }

    #[test]
    fn a_patch_round_trips() {
        let patch = Patch {
            version: PATCH_VERSION,
            params: BTreeMap::from([
                ("a_decay".to_string(), 10.78),
                ("a_model".to_string(), 1.0),
                ("mallet_type".to_string(), 0.0),
            ]),
            user_sample: Some("AAAA".to_string()),
            current_program: Some(3),
        };

        let json = write_patch(&patch);
        assert_eq!(read_patch(&json).expect("the patch reads back"), patch);
    }

    #[test]
    fn a_patch_without_the_optional_fields_round_trips() {
        let patch = Patch {
            version: PATCH_VERSION,
            params: BTreeMap::from([("gain".to_string(), 6.24)]),
            user_sample: None,
            current_program: None,
        };

        let json = write_patch(&patch);
        assert!(!json.contains("userSample"));
        assert_eq!(read_patch(&json).expect("the patch reads back"), patch);
    }

    #[test]
    fn a_patch_of_an_unknown_version_is_rejected() {
        let json = r#"{"version": 99, "params": {}}"#;
        assert!(matches!(
            read_patch(json),
            Err(PatchError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn a_legacy_patch_converts_to_a_patch() {
        let legacy = read_legacy_patch(
            "<PARAMETERS currentProgram=\"2\"><PARAM id=\"a_cut\" value=\"372.0\"/><PARAM id=\"a_decay\" value=\"10.77999973297119\"/><PARAM id=\"a_model\" value=\"1.0\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");

        let patch = convert_legacy_patch(&legacy);
        let expected = (372.0f64 / 20.0).ln() / (20000.0f64 / 20.0).ln();
        assert!((patch.params["a_cut"] - expected).abs() < 1e-12);
        assert_eq!(patch.params["a_decay"], 10.78);
        assert_eq!(patch.params["a_model"], 1.0);
        assert_eq!(patch.current_program, Some(2));
        assert_eq!(patch.version, PATCH_VERSION);
    }

    #[test]
    fn the_converter_handles_every_factory_preset() {
        let directory = preset_directory();
        let mut converted = 0;

        let mut entries: Vec<_> = std::fs::read_dir(&directory)
            .expect("the presets are there")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "xml"))
            .collect();
        entries.sort();

        for path in entries {
            let xml = std::fs::read_to_string(&path).expect("the preset reads");
            let legacy = read_legacy_patch(&xml)
                .unwrap_or_else(|| panic!("{} is a parameters tree", path.display()));
            let patch = convert_legacy_patch(&legacy);

            for id in ["a_cut", "b_cut"] {
                let value = patch
                    .params
                    .get(id)
                    .unwrap_or_else(|| panic!("{} has {id}", path.display()));
                assert!(
                    (-1.0..=1.0).contains(value),
                    "{} has {id} at {value}",
                    path.display()
                );
            }

            converted += 1;
        }

        assert_eq!(converted, 28);
    }
}
