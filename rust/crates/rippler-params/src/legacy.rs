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
//! Three things make the reader more than a plain XML walk:
//!
//! - A `<PARAM>` without a `value` attribute holds the parameter's **default**,
//!   because JUCE reads a missing property as the default:
//!
//!   ```cpp
//!   p->setDenormalisedValue (p->tree.getProperty (valuePropertyID, p->getDenormalisedDefaultValue()));
//!   ```
//!
//! - A value that is not a finite number is refused. JUCE would hold it, but
//!   it would poison every patch written from it.
//! - A document that is truncated or malformed is refused as well, rather than
//!   read as a patch with only some of its values.
//!
//! The reader keeps the values as the legacy state holds them. The migration
//! of the `a_cut` and `b_cut` parameters and the snapping to the parameters'
//! step sizes happen in [`to_parameter_values`].

use std::collections::BTreeMap;
use std::fmt;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::params::{ParameterKind, spec};

/// The values read from a legacy patch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LegacyPatch {
    /// The parameter values, by legacy identifier.
    ///
    /// The values are the plain ones the legacy state holds: a float
    /// parameter its value, a choice parameter its index and a boolean `0.0`
    /// or `1.0`. They are neither migrated nor snapped; [`to_parameter_values`]
    /// does that.
    pub values: BTreeMap<String, f64>,
    /// The base64-encoded user sample, if the patch carried one.
    pub user_sample: Option<String>,
    /// The program the patch was on, if it named one.
    pub current_program: Option<i32>,
}

/// What can go wrong when reading a legacy patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyError {
    /// The document is not well formed, or it was cut short.
    Malformed(String),
    /// The document is not a `PARAMETERS` tree.
    NotAParametersTree,
    /// A parameter's value is a number that is not finite.
    NotFinite(String),
}

impl fmt::Display for LegacyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LegacyError::Malformed(reason) => {
                write!(f, "the patch is not a well formed document: {reason}")
            }
            LegacyError::NotAParametersTree => write!(f, "the patch is not a parameters tree"),
            LegacyError::NotFinite(id) => {
                write!(f, "the parameter {id} has a value that is not a number")
            }
        }
    }
}

impl std::error::Error for LegacyError {}

/// Reads a legacy JUCE patch.
///
/// The patch is refused when it is not a `PARAMETERS` tree, when it is
/// truncated or malformed, and when one of its values is not a finite number.
pub fn read_legacy_patch(xml: &str) -> Result<LegacyPatch, LegacyError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buffer = Vec::new();
    let mut patch = LegacyPatch::default();
    let mut seen_root = false;
    let mut closed_root = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                if !seen_root {
                    seen_root = true;
                    if event.name().as_ref() != "PARAMETERS" {
                        return Err(LegacyError::NotAParametersTree);
                    }
                    read_root_attributes(&event, &mut patch);
                }
            }
            Ok(Event::Empty(event)) => {
                if !seen_root {
                    seen_root = true;
                    if event.name().as_ref() != "PARAMETERS" {
                        return Err(LegacyError::NotAParametersTree);
                    }
                    read_root_attributes(&event, &mut patch);
                    closed_root = true;
                } else if event.name().as_ref() == "PARAM" {
                    read_parameter(&event, &mut patch)?;
                }
            }
            Ok(Event::End(event)) => {
                if event.name().as_ref() == "PARAMETERS" {
                    closed_root = true;
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(LegacyError::Malformed(error.to_string())),
            _ => {}
        }

        buffer.clear();
    }

    if !seen_root {
        return Err(LegacyError::NotAParametersTree);
    }

    if !closed_root {
        return Err(LegacyError::Malformed(String::from(
            "the document ends before the parameters tree does",
        )));
    }

    Ok(patch)
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
fn read_parameter(event: &BytesStart, patch: &mut LegacyPatch) -> Result<(), LegacyError> {
    let mut id = None;
    let mut value = None;

    for (key, attribute) in attributes(event) {
        match key.as_str() {
            "id" => id = Some(attribute),
            // A value that is not a number at all is zero, the way JUCE's
            // `var` reads it. A value that is a number but not a finite one is
            // refused below.
            "value" => value = Some(attribute.parse::<f64>().unwrap_or(0.0)),
            _ => {}
        }
    }

    let Some(id) = id else { return Ok(()) };
    let Some(specification) = spec(&id) else {
        // An identifier the plugin does not know is simply ignored.
        return Ok(());
    };

    // A missing value is the parameter's default, the way JUCE reads it.
    let value = value.unwrap_or(specification.default);

    if !value.is_finite() {
        return Err(LegacyError::NotFinite(id));
    }

    patch.values.insert(id, value);
    Ok(())
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
/// The `a_cut` and `b_cut` migration is applied, and every value is snapped
/// to its parameter's step size the way JUCE's `snapToLegalValue` does.
pub fn to_parameter_values(patch: &LegacyPatch) -> BTreeMap<String, f64> {
    let mut values = BTreeMap::new();

    for (id, value) in &patch.values {
        let Some(spec) = spec(id) else { continue };

        let value = match id.as_str() {
            "a_cut" | "b_cut" => migrate_cut(*value),
            _ => *value,
        };

        let value = match spec.kind {
            ParameterKind::Float => match spec.step {
                Some(step) => snap_to_step(value, spec.min, spec.max, step),
                None => value.clamp(spec.min, spec.max),
            },
            ParameterKind::Int => snap_to_step(value, spec.min, spec.max, 1.0),
            ParameterKind::Bool => {
                if value >= 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
            ParameterKind::Choice => value.round().clamp(spec.min, spec.max),
        };

        values.insert(id.clone(), value);
    }

    values
}

/// Snaps a value to a step size the way JUCE's `snapToLegalValue` does.
///
/// The step is counted from the bottom of the range, and the result is
/// clamped to the range. The result is rounded to the number of decimals the
/// step implies, so that a value like `-0.6200000047683716` comes out as
/// `-0.62` rather than as the difference of two rounded numbers.
fn snap_to_step(value: f64, min: f64, max: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return value.clamp(min, max);
    }

    let snapped = min + step * ((value - min) / step + 0.5).floor();
    let snapped = snapped.clamp(min, max);

    let scale = 10f64.powi(decimals_of(step) as i32);
    (snapped * scale).round() / scale
}

/// The number of decimals a step size implies.
fn decimals_of(step: f64) -> u32 {
    let mut decimals = 0;
    let mut scaled = step;

    while (scaled - scaled.round()).abs() > 1e-9 && decimals < 12 {
        scaled *= 10.0;
        decimals += 1;
    }

    decimals
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
    fn a_parameter_without_a_value_reads_as_its_default() {
        let patch = read_legacy_patch("<PARAMETERS><PARAM id=\"a_decay\"/></PARAMETERS>")
            .expect("the patch is a parameters tree");
        assert_eq!(patch.values["a_decay"], 1.0);
        assert_eq!(patch.values.len(), 1);

        let patch = read_legacy_patch("<PARAMETERS><PARAM id=\"mallet_res\"/></PARAMETERS>")
            .expect("the patch is a parameters tree");
        assert_eq!(patch.values["mallet_res"], 0.8);
    }

    #[test]
    fn an_absent_parameter_is_simply_missing() {
        let patch =
            read_legacy_patch("<PARAMETERS><PARAM id=\"a_decay\" value=\"1.0\"/></PARAMETERS>")
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
        assert_eq!(
            read_legacy_patch("<OTHER/>"),
            Err(LegacyError::NotAParametersTree)
        );
        assert_eq!(
            read_legacy_patch("not xml at all"),
            Err(LegacyError::NotAParametersTree)
        );
    }

    #[test]
    fn a_truncated_patch_is_rejected() {
        let truncated = "<PARAMETERS><PARAM id=\"a_decay\" value=\"1.0\"/>";
        assert!(matches!(
            read_legacy_patch(truncated),
            Err(LegacyError::Malformed(_))
        ));

        let truncated = "<PARAMETERS><PARAM id=\"a_decay\" value=\"1.0\"";
        assert!(matches!(
            read_legacy_patch(truncated),
            Err(LegacyError::Malformed(_))
        ));
    }

    #[test]
    fn a_value_that_is_not_a_number_is_rejected() {
        let patch = "<PARAMETERS><PARAM id=\"a_decay\" value=\"nan\"/></PARAMETERS>";
        assert_eq!(
            read_legacy_patch(patch),
            Err(LegacyError::NotFinite("a_decay".to_string()))
        );

        let patch = "<PARAMETERS><PARAM id=\"a_decay\" value=\"inf\"/></PARAMETERS>";
        assert_eq!(
            read_legacy_patch(patch),
            Err(LegacyError::NotFinite("a_decay".to_string()))
        );
    }

    #[test]
    fn the_cut_parameters_are_migrated_from_hertz() {
        let patch = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"a_cut\" value=\"372.0\"/><PARAM id=\"b_cut\" value=\"20.0\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");

        let expected = (372.0f64 / 20.0).ln() / (20000.0f64 / 20.0).ln();
        // The reader keeps the values as the legacy state holds them: the
        // migration belongs to the conversion.
        assert_eq!(patch.values["a_cut"], 372.0);
        assert_eq!(patch.values["b_cut"], 20.0);

        let values = to_parameter_values(&patch);
        assert!((values["a_cut"] - expected).abs() < 0.01);
        assert!((values["b_cut"] - 0.0).abs() < 0.01);
    }

    #[test]
    fn the_cut_migration_happens_once() {
        let patch =
            read_legacy_patch("<PARAMETERS><PARAM id=\"a_cut\" value=\"372.0\"/></PARAMETERS>")
                .expect("the patch is a parameters tree");

        let values = to_parameter_values(&patch);
        let expected = (372.0f64 / 20.0).ln() / (20000.0f64 / 20.0).ln();
        assert!((values["a_cut"] - expected).abs() < 0.01);

        // The migration is not applied a second time.
        let again = to_parameter_values(&LegacyPatch {
            values: BTreeMap::from([("a_cut".to_string(), values["a_cut"])]),
            ..Default::default()
        });
        assert_eq!(again["a_cut"], values["a_cut"]);
    }

    #[test]
    fn a_cut_below_twenty_is_left_alone() {
        let patch =
            read_legacy_patch("<PARAMETERS><PARAM id=\"a_cut\" value=\"-0.5\"/></PARAMETERS>")
                .expect("the patch is a parameters tree");
        assert_eq!(patch.values["a_cut"], -0.5);

        let values = to_parameter_values(&patch);
        assert_eq!(values["a_cut"], -0.5);
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

    #[test]
    fn the_implicit_interval_of_the_juce_build_is_applied() {
        // JUCE's four-argument constructor gives every one of these an
        // interval of `0.01`.
        let patch = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"a_damp\" value=\"-0.6200000047683716\"/><PARAM id=\"gain\" value=\"6.239999771118164\"/><PARAM id=\"b_hit\" value=\"0.2599999904632568\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");

        let values = to_parameter_values(&patch);
        assert_eq!(values["a_damp"], -0.62);
        assert_eq!(values["gain"], 6.24);
        assert_eq!(values["b_hit"], 0.26);
    }

    #[test]
    fn a_choice_parameter_is_its_index() {
        let patch = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"a_model\" value=\"3.0\"/><PARAM id=\"couple\" value=\"1.0\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");
        let values = to_parameter_values(&patch);
        assert_eq!(values["a_model"], 3.0);
        assert_eq!(values["couple"], 1.0);
    }

    #[test]
    fn a_boolean_parameter_is_zero_or_one() {
        let patch = read_legacy_patch(
            "<PARAMETERS><PARAM id=\"a_on\" value=\"1.0\"/><PARAM id=\"b_on\" value=\"0.0\"/></PARAMETERS>",
        )
        .expect("the patch is a parameters tree");
        let values = to_parameter_values(&patch);
        assert_eq!(values["a_on"], 1.0);
        assert_eq!(values["b_on"], 0.0);
    }

    #[test]
    fn an_integer_parameter_is_a_whole_number() {
        let patch =
            read_legacy_patch("<PARAMETERS><PARAM id=\"bend_range\" value=\"7.0\"/></PARAMETERS>")
                .expect("the patch is a parameters tree");
        let values = to_parameter_values(&patch);
        assert_eq!(values["bend_range"], 7.0);
    }
}
