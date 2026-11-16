//! Reading the state the JUCE build wrote.
//!
//! The JUCE build keeps its state in a `ValueTree` that it writes as XML:
//!
//! ```xml
//! <PARAMETERS>
//!   <PARAM id="a_decay" value="10.77999973297119"/>
//!   <PARAM id="a_coarse"/>
//! </PARAMETERS>
//! ```
//!
//! The values in that XML are **plain**, not normalised: JUCE's parameter
//! adapter writes the unnormalised value into the tree, so a float parameter
//! holds its plain value, a choice parameter its index and a boolean `0.0` or
//! `1.0`. The reader therefore does not apply the skew.
//!
//! Two things make the reader more than a plain XML walk:
//!
//! - A `<PARAM>` without a `value` attribute holds **zero**, because JUCE's
//!   `replaceState` reads a missing property as `var()`, which converts to
//!   `0.0f`. That is not the same as "keep the default", and it matters for
//!   every parameter whose default is not zero.
//! - The `a_cut` and `b_cut` parameters need the historical migration: the
//!   presets hold hertz values on the old scale, and the current parameter is
//!   a position on a logarithmic scale from 20 Hz to 20 kHz.

use std::collections::BTreeMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::params::{ParameterKind, spec};

/// The values read from a legacy patch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LegacyPatch {
    /// The parameter values, by legacy identifier.
    pub values: BTreeMap<String, f64>,
    /// The base64-encoded user sample, if the patch carried one.
    pub user_sample: Option<String>,
    /// The program the patch was on, if it named one.
    pub current_program: Option<i32>,
}

/// Reads a legacy JUCE patch.
///
/// Returns `None` if the XML is not a `PARAMETERS` tree.
pub fn read_legacy_patch(xml: &str) -> Option<LegacyPatch> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buffer = Vec::new();
    let mut patch = LegacyPatch::default();
    let mut is_parameters = false;
    let mut seen_root = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                if !seen_root {
                    seen_root = true;
                    if event.name().as_ref() != "PARAMETERS" {
                        return None;
                    }
                    is_parameters = true;
                    read_root_attributes(&event, &mut patch);
                }
            }
            Ok(Event::Empty(event)) => {
                if !seen_root {
                    seen_root = true;
                    if event.name().as_ref() != "PARAMETERS" {
                        return None;
                    }
                    is_parameters = true;
                    read_root_attributes(&event, &mut patch);
                } else if is_parameters && event.name().as_ref() == "PARAM" {
                    read_parameter(&event, &mut patch);
                }
            }
            Ok(Event::End(event)) => {
                if event.name().as_ref() == "PARAMETERS" {
                    is_parameters = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }

        buffer.clear();
    }

    if !seen_root {
        return None;
    }

    Some(patch)
}

/// Reads the attributes of the root element.
fn read_root_attributes(event: &BytesStart, patch: &mut LegacyPatch) {
    for (key, value) in attributes(event) {
        match key.as_str() {
            "userSample" => patch.user_sample = Some(value),
            "currentProgram" => {
                patch.current_program = value.parse::<i32>().ok().or(patch.current_program);
            }
            _ => {}
        }
    }
}

/// Reads one `<PARAM>` element.
fn read_parameter(event: &BytesStart, patch: &mut LegacyPatch) {
    let mut id = None;
    let mut value = None;

    for (key, attribute) in attributes(event) {
        match key.as_str() {
            "id" => id = Some(attribute),
            // A missing value is zero, not the default.
            "value" => value = Some(attribute.parse::<f64>().unwrap_or(0.0)),
            _ => {}
        }
    }

    let Some(id) = id else { return };
    if spec(&id).is_none() {
        return;
    }

    let value = value.unwrap_or(0.0);
    let value = match id.as_str() {
        "a_cut" | "b_cut" => migrate_cut(value),
        _ => value,
    };

    patch.values.insert(id, value);
}

/// The attributes of an element, by name.
fn attributes(event: &BytesStart) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    for attribute in event.attributes().with_checks(false) {
        let Ok(attribute) = attribute else { continue };
        let key = attribute.key.as_ref().to_string();
        let value = attribute
            .normalized_value(XmlVersion::Explicit1_0)
            .map(|value| value.into_owned())
            .unwrap_or_default();
        attributes.push((key, value));
    }
    attributes
}

/// Migrates a cut value from the old hertz scale to the current one.
///
/// The JUCE build migrates on load:
///
/// ```cpp
/// if (val >= 20.0f) {
///     val = std::log(val / 20.0f) / std::log(20000.0f / 20.0f);
/// }
/// ```
pub fn migrate_cut(value: f64) -> f64 {
    if value >= 20.0 {
        (value / 20.0).ln() / (20000.0f64 / 20.0).ln()
    } else {
        value
    }
}

/// Converts a legacy patch into plain parameter values.
///
/// Applies the `a_cut` and `b_cut` migration, and snaps each value to its
/// parameter's step size the way JUCE's `snapToLegalValue` does. A value with
/// no step size is left exactly as it is.
pub fn to_parameter_values(patch: &LegacyPatch) -> BTreeMap<String, f64> {
    let mut values = BTreeMap::new();

    for (id, value) in &patch.values {
        let Some(spec) = spec(id) else { continue };

        let value = match id.as_str() {
            "a_cut" | "b_cut" => migrate_cut(*value),
            _ => *value,
        };

        let value = match (spec.kind, spec.step) {
            (ParameterKind::Float, Some(step)) => {
                snap_to_step(value, spec.min, spec.max, step)
            }
            _ => value,
        };

        values.insert(id.clone(), value);
    }

    values
}

/// Snaps a value to a step size the way JUCE's `snapToLegalValue` does.
///
/// The step is counted from the bottom of the range, and the result is
/// clamped to the range.
fn snap_to_step(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let snapped = min + step * ((value - min) / step + 0.5).floor();
    if snapped <= min || max <= min {
        min
    } else if snapped >= max {
        max
    } else {
        snapped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A patch with a couple of parameters in it.
    const PATCH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PARAMETERS currentProgram="3" userSample="AAAA">
  <PARAM id="a_decay" value="10.77999973297119"/>
  <PARAM id="a_model" value="1.0"/>
  <PARAM id="mallet_type" value="0.0"/>
</PARAMETERS>"#;

    #[test]
    fn a_legacy_patch_reads_its_parameters() {
        let patch = read_legacy_patch(PATCH).expect("the patch is a parameters tree");
        assert_eq!(patch.values["a_decay"], 10.77999973297119);
        assert_eq!(patch.values["a_model"], 1.0);
        assert_eq!(patch.values["mallet_type"], 0.0);
        assert_eq!(patch.values.len(), 3);
    }

    #[test]
    fn a_parameter_without_a_value_reads_as_zero() {
        let patch = read_legacy_patch("<PARAMETERS><PARAM id=\"a_coarse\"/></PARAMETERS>")
            .expect("the patch is a parameters tree");
        assert_eq!(patch.values["a_coarse"], 0.0);
        assert_eq!(patch.values.len(), 1);
    }

    #[test]
    fn an_absent_parameter_is_simply_missing() {
        let patch = read_legacy_patch("<PARAMETERS><PARAM id=\"a_decay\" value=\"1.0\"/></PARAMETERS>")
            .expect("the patch is a parameters tree");
        assert_eq!(patch.values.len(), 1);
        assert!(!patch.values.contains_key("b_decay"));
        assert!(!patch.values.contains_key("mallet_type"));
    }

    #[test]
    fn an_unknown_identifier_is_ignored() {
        let patch = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"not_a_parameter\" value=\"1.0\"/><PARAM id=\"a_decay\" value=\"2.0\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");
        assert_eq!(patch.values.len(), 1);
        assert!(!patch.values.contains_key("not_a_parameter"));
        assert_eq!(patch.values["a_decay"], 2.0);
    }

    #[test]
    fn a_tree_that_is_not_a_parameters_tree_is_rejected() {
        assert!(read_legacy_patch("<OTHER/>").is_none());
        assert!(read_legacy_patch("not xml at all").is_none());
    }

    #[test]
    fn the_cut_parameters_are_migrated_from_hertz() {
        let patch = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"a_cut\" value=\"372.0\"/><PARAM id=\"b_cut\" value=\"20.0\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");

        let expected = (372.0f64 / 20.0).ln() / (20000.0f64 / 20.0).ln();
        assert!((patch.values["a_cut"] - expected).abs() < 1e-12);
        assert!((patch.values["b_cut"] - 0.0).abs() < 1e-12);
    }

    #[test]
    fn a_cut_below_twenty_is_left_alone() {
        let patch = read_legacy_patch("<PARAMETERS><PARAM id=\"a_cut\" value=\"-0.5\"/></PARAMETERS>")
            .expect("the patch is a parameters tree");
        assert_eq!(patch.values["a_cut"], -0.5);
    }

    #[test]
    fn the_user_sample_is_kept() {
        let patch = read_legacy_patch(PATCH).expect("the patch is a parameters tree");
        assert_eq!(patch.user_sample.as_deref(), Some("AAAA"));
    }

    #[test]
    fn the_current_program_is_kept() {
        let patch = read_legacy_patch(PATCH).expect("the patch is a parameters tree");
        assert_eq!(patch.current_program, Some(3));
    }

    #[test]
    fn values_are_snapped_to_their_step() {
        let patch = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"a_decay\" value=\"10.77999973297119\"/><PARAM id=\"a_cut\" value=\"0.5\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");
        let values = to_parameter_values(&patch);
        assert_eq!(values["a_decay"], 10.78);
        assert_eq!(values["a_cut"], 0.5);
    }
}
