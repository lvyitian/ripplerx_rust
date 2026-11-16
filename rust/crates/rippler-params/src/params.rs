//! The parameter set.
//!
//! The parameters are the JUCE build's parameters, one for one: the same
//! identifiers, the same ranges, the same defaults and the same choices. The
//! identifiers are the legacy ones, verbatim, so a patch saved by either
//! build means the same thing to the other.
//!
//! The ranges are JUCE's `NormalisableRange`s. JUCE normalises a value with a
//! positive skew `s` as `p^(1/(1+s))`, and nice-plug's `FloatRange::Skewed`
//! normalises as `p^factor`, so the factor is `1/(1+s)` and the curve matches
//! exactly. JUCE's `interval` is nice-plug's step size.
//!
//! The values themselves are stored plain, never normalised: JUCE's parameter
//! adapter writes the unnormalised value into the state, so a float parameter
//! holds its plain value, a choice parameter its index and a boolean `0.0` or
//! `1.0`.

use nice_plug_core::params::enums::Enum;
use nice_plug_core::params::range::IntRange;
use nice_plug_core::params::{BoolParam, EnumParam, FloatParam, IntParam, Params};
use rippler_dsp::mallet::MalletType;
use rippler_dsp::models::ModalModel;

use crate::enums::{Coupling, NoiseFilterMode, PartialCount};
use crate::ranges;

/// The number of mallet choices, `mallet_type`.
const MALLET_CHOICES: f32 = 26.0;
/// The number of model choices, `a_model` and `b_model`.
const MODEL_CHOICES: f32 = 12.0;
/// The number of partial choices, `a_partials` and `b_partials`.
const PARTIAL_CHOICES: f32 = 7.0;
/// The number of noise filter choices, `noise_filter_mode`.
const FILTER_MODE_CHOICES: f32 = 3.0;
/// The number of coupling choices, `couple`.
const COUPLING_CHOICES: f32 = 2.0;

/// The kind of a parameter, which decides how a legacy value is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParameterKind {
    /// A continuous float parameter.
    Float,
    /// An integer parameter.
    Int,
    /// A boolean parameter, stored as `0.0` or `1.0`.
    Bool,
    /// A choice parameter, stored as its index.
    Choice,
}

/// What the parameter layer needs to know about one parameter.
///
/// The table is the single source of truth: the parameters are built from it
/// and the legacy reader reads the legal values of a patch from it.
pub(crate) struct ParameterSpec {
    /// The legacy identifier, the one the state file holds.
    pub(crate) id: &'static str,
    /// The name the host shows.
    pub(crate) name: &'static str,
    /// The kind of the parameter.
    pub(crate) kind: ParameterKind,
    /// The default value, plain and unnormalised.
    pub(crate) default: f64,
    /// The lowest legal value.
    pub(crate) min: f32,
    /// The highest legal value.
    pub(crate) max: f32,
    /// JUCE's skew, if the parameter has one.
    pub(crate) skew: Option<f32>,
    /// JUCE's interval, if the parameter has one.
    pub(crate) step: Option<f32>,
}

/// Every parameter, in the order the groups hold them.
pub(crate) const SPECS: &[ParameterSpec] = &[
    // The mallet.
    ParameterSpec {
        id: "mallet_type",
        name: "Mallet Type",
        kind: ParameterKind::Choice,
        default: 0.0,
        min: 0.0,
        max: MALLET_CHOICES - 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "mallet_pitch",
        name: "Mallet Pitch",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -24.0,
        max: 24.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "mallet_filter",
        name: "Mallet Filter",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "mallet_mix",
        name: "Mallet Mix",
        kind: ParameterKind::Float,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "mallet_res",
        name: "Mallet Resonance",
        kind: ParameterKind::Float,
        default: 0.8,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "mallet_stiff",
        name: "Mallet Stiffness",
        kind: ParameterKind::Float,
        default: 600.0,
        min: 100.0,
        max: 5000.0,
        skew: Some(0.3),
        step: Some(1.0),
    },
    ParameterSpec {
        id: "mallet_ktrack",
        name: "Mallet Keytrack",
        kind: ParameterKind::Float,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    // The noise.
    ParameterSpec {
        id: "noise_osc",
        name: "Noise DC",
        kind: ParameterKind::Float,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: Some(1.0),
        step: Some(0.0001),
    },
    ParameterSpec {
        id: "noise_mix",
        name: "Noise Mix",
        kind: ParameterKind::Float,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: Some(0.3),
        step: Some(0.0001),
    },
    ParameterSpec {
        id: "noise_res",
        name: "Noise Resonance",
        kind: ParameterKind::Float,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: Some(0.3),
        step: Some(0.0001),
    },
    ParameterSpec {
        id: "noise_filter_mode",
        name: "Noise Filter Mode",
        kind: ParameterKind::Choice,
        default: 2.0,
        min: 0.0,
        max: FILTER_MODE_CHOICES - 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "noise_filter_freq",
        name: "Noise Filter Freq",
        kind: ParameterKind::Float,
        default: 20.0,
        min: 20.0,
        max: 20000.0,
        skew: Some(0.3),
        step: Some(1.0),
    },
    ParameterSpec {
        id: "noise_filter_q",
        name: "Noise Filter Q",
        kind: ParameterKind::Float,
        default: 0.707,
        min: 0.707,
        max: 4.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "noise_att",
        name: "Noise Attack",
        kind: ParameterKind::Float,
        default: 1.0,
        min: 1.0,
        max: 20000.0,
        skew: Some(0.3),
        step: Some(1.0),
    },
    ParameterSpec {
        id: "noise_dec",
        name: "Noise Decay",
        kind: ParameterKind::Float,
        default: 500.0,
        min: 1.0,
        max: 20000.0,
        skew: Some(0.3),
        step: Some(1.0),
    },
    ParameterSpec {
        id: "noise_sus",
        name: "Noise Sustain",
        kind: ParameterKind::Float,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "noise_rel",
        name: "Noise Release",
        kind: ParameterKind::Float,
        default: 500.0,
        min: 1.0,
        max: 20000.0,
        skew: Some(0.3),
        step: Some(1.0),
    },
    ParameterSpec {
        id: "noise_att_ten",
        name: "Noise Attack Tension",
        kind: ParameterKind::Float,
        default: 0.4,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "noise_dec_ten",
        name: "Noise Decay Tension",
        kind: ParameterKind::Float,
        default: 0.4,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "noise_rel_ten",
        name: "Noise Release Tension",
        kind: ParameterKind::Float,
        default: 0.4,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    // The first resonator.
    ParameterSpec {
        id: "a_on",
        name: "A ON",
        kind: ParameterKind::Bool,
        default: 1.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_model",
        name: "A Model",
        kind: ParameterKind::Choice,
        default: 0.0,
        min: 0.0,
        max: MODEL_CHOICES - 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_partials",
        name: "A Partials",
        kind: ParameterKind::Choice,
        default: 3.0,
        min: 0.0,
        max: PARTIAL_CHOICES - 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_decay",
        name: "A Decay",
        kind: ParameterKind::Float,
        default: 1.0,
        min: 0.01,
        max: 100.0,
        skew: Some(0.2),
        step: Some(0.01),
    },
    ParameterSpec {
        id: "a_damp",
        name: "A Material",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_tone",
        name: "A Tone",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_hit",
        name: "A HitPos",
        kind: ParameterKind::Float,
        default: 0.26,
        min: 0.02,
        max: 0.5,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_rel",
        name: "A Release",
        kind: ParameterKind::Float,
        default: 1.0,
        min: 0.001,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_inharm",
        name: "A Inharmonic",
        kind: ParameterKind::Float,
        default: 0.0001,
        min: 0.0001,
        max: 1.0,
        skew: Some(0.3),
        step: Some(0.0001),
    },
    ParameterSpec {
        id: "a_ratio",
        name: "A Ratio",
        kind: ParameterKind::Float,
        default: 1.0,
        min: 0.1,
        max: 10.0,
        skew: Some(0.3),
        step: Some(0.01),
    },
    ParameterSpec {
        id: "a_cut",
        name: "A LowCut",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_radius",
        name: "A Tube Radius",
        kind: ParameterKind::Float,
        default: 0.5,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "a_coarse",
        name: "A coarse pitch",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -48.0,
        max: 48.0,
        skew: Some(1.0),
        step: Some(1.0),
    },
    ParameterSpec {
        id: "a_fine",
        name: "A fine pitch",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -99.0,
        max: 99.0,
        skew: Some(1.0),
        step: Some(1.0),
    },
    // The second resonator.
    ParameterSpec {
        id: "b_on",
        name: "B ON",
        kind: ParameterKind::Bool,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_model",
        name: "B Model",
        kind: ParameterKind::Choice,
        default: 0.0,
        min: 0.0,
        max: MODEL_CHOICES - 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_partials",
        name: "B Partials",
        kind: ParameterKind::Choice,
        default: 3.0,
        min: 0.0,
        max: PARTIAL_CHOICES - 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_decay",
        name: "B Decay",
        kind: ParameterKind::Float,
        default: 1.0,
        min: 0.01,
        max: 100.0,
        skew: Some(0.2),
        step: Some(0.01),
    },
    ParameterSpec {
        id: "b_damp",
        name: "B Material",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_tone",
        name: "B Tone",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_hit",
        name: "B HitPos",
        kind: ParameterKind::Float,
        default: 0.26,
        min: 0.02,
        max: 0.5,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_rel",
        name: "B Release",
        kind: ParameterKind::Float,
        default: 1.0,
        min: 0.001,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_inharm",
        name: "B Inharmonic",
        kind: ParameterKind::Float,
        default: 0.0001,
        min: 0.0001,
        max: 1.0,
        skew: Some(0.3),
        step: Some(0.0001),
    },
    ParameterSpec {
        id: "b_ratio",
        name: "B Ratio",
        kind: ParameterKind::Float,
        default: 1.0,
        min: 0.1,
        max: 10.0,
        skew: Some(0.3),
        step: Some(0.01),
    },
    ParameterSpec {
        id: "b_cut",
        name: "B LowCut",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_radius",
        name: "B Tube Radius",
        kind: ParameterKind::Float,
        default: 0.5,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "b_coarse",
        name: "B coarse pitch",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -48.0,
        max: 48.0,
        skew: Some(1.0),
        step: Some(1.0),
    },
    ParameterSpec {
        id: "b_fine",
        name: "B fine pitch",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -99.0,
        max: 99.0,
        skew: Some(1.0),
        step: Some(1.0),
    },
    // The velocity amounts.
    ParameterSpec {
        id: "vel_mallet_mix",
        name: "Vel Mallet Mix",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_mallet_res",
        name: "Vel Mallet Resonance",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_mallet_stiff",
        name: "Vel Mallet Stiffness",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_mix",
        name: "Vel Noise Mix",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_res",
        name: "Vel Noise Resonance",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_freq",
        name: "Vel Noise Frequency",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_att",
        name: "Vel Noise Attack",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_dec",
        name: "Vel Noise Decay",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_sus",
        name: "Vel Noise Sustain",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_rel",
        name: "Vel Noise Release",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_noise_q",
        name: "Vel Noise Q",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_a_decay",
        name: "Vel A Decay",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_a_hit",
        name: "Vel A Hit",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_a_inharm",
        name: "Vel A Inharmonic",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_a_damp",
        name: "Vel A Material",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_a_tone",
        name: "Vel A Tone",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_b_decay",
        name: "Vel B Decay",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_b_hit",
        name: "Vel B Hit",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_b_inharm",
        name: "Vel B Inharmonic",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_b_damp",
        name: "Vel B Material",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "vel_b_tone",
        name: "Vel B Tone",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -1.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    // The top level.
    ParameterSpec {
        id: "couple",
        name: "Coupling",
        kind: ParameterKind::Choice,
        default: 0.0,
        min: 0.0,
        max: COUPLING_CHOICES - 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "ab_mix",
        name: "A+B Mix",
        kind: ParameterKind::Float,
        default: 0.5,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "ab_split",
        name: "A>B Split",
        kind: ParameterKind::Float,
        default: 0.01,
        min: 0.01,
        max: 1.0,
        skew: Some(0.5),
        step: Some(0.001),
    },
    ParameterSpec {
        id: "gain",
        name: "Res Gain",
        kind: ParameterKind::Float,
        default: 0.0,
        min: -24.0,
        max: 24.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "bend_range",
        name: "PitchBend Range",
        kind: ParameterKind::Int,
        default: 2.0,
        min: 1.0,
        max: 24.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "stereoizer",
        name: "Stereoizer",
        kind: ParameterKind::Bool,
        default: 1.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "reuse_voices",
        name: "Reuse Voices",
        kind: ParameterKind::Bool,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
    ParameterSpec {
        id: "fadeout_repeats",
        name: "Fadeout Repeated Notes",
        kind: ParameterKind::Bool,
        default: 0.0,
        min: 0.0,
        max: 1.0,
        skew: None,
        step: None,
    },
];

/// The specification of a parameter, if the plugin has one.
pub(crate) fn spec(id: &str) -> Option<&'static ParameterSpec> {
    SPECS.iter().find(|spec| spec.id == id)
}

/// Builds the float parameter a specification describes.
fn float_param(id: &str) -> FloatParam {
    let spec = spec(id).expect("the parameter has a specification");
    let range = match spec.skew {
        Some(skew) => ranges::skewed(spec.min, spec.max, skew),
        None => ranges::linear(spec.min, spec.max),
    };
    let param = FloatParam::new(spec.name, spec.default as f32, range);
    match spec.step {
        Some(step) => param.with_step_size(step),
        None => param,
    }
}

/// Builds the integer parameter a specification describes.
fn int_param(id: &str) -> IntParam {
    let spec = spec(id).expect("the parameter has a specification");
    IntParam::new(
        spec.name,
        spec.default as i32,
        IntRange::Linear {
            min: spec.min as i32,
            max: spec.max as i32,
        },
    )
}

/// Builds the boolean parameter a specification describes.
fn bool_param(id: &str) -> BoolParam {
    let spec = spec(id).expect("the parameter has a specification");
    BoolParam::new(spec.name, spec.default != 0.0)
}

/// The mallet parameters.
#[derive(Params)]
pub struct MalletParams {
    /// The kind of mallet that excites the resonators.
    #[id = "mallet_type"]
    pub mallet_type: EnumParam<MalletType>,
    /// The mallet's pitch, in semitones.
    #[id = "mallet_pitch"]
    pub mallet_pitch: FloatParam,
    /// The mallet's filter, in the normalised range.
    #[id = "mallet_filter"]
    pub mallet_filter: FloatParam,
    /// How much of the mallet's sound reaches the resonators.
    #[id = "mallet_mix"]
    pub mallet_mix: FloatParam,
    /// The mallet's resonance.
    #[id = "mallet_res"]
    pub mallet_res: FloatParam,
    /// The mallet's stiffness.
    #[id = "mallet_stiff"]
    pub mallet_stiff: FloatParam,
    /// How much the key changes the mallet's stiffness.
    #[id = "mallet_ktrack"]
    pub mallet_ktrack: FloatParam,
}

impl Default for MalletParams {
    fn default() -> Self {
        Self {
            mallet_type: EnumParam::new("Mallet Type", MalletType::Impulse),
            mallet_pitch: float_param("mallet_pitch"),
            mallet_filter: float_param("mallet_filter"),
            mallet_mix: float_param("mallet_mix"),
            mallet_res: float_param("mallet_res"),
            mallet_stiff: float_param("mallet_stiff"),
            mallet_ktrack: float_param("mallet_ktrack"),
        }
    }
}

/// The noise parameters.
#[derive(Params)]
pub struct NoiseParams {
    /// The noise's offset voltage.
    #[id = "noise_osc"]
    pub noise_osc: FloatParam,
    /// How much noise reaches the resonators.
    #[id = "noise_mix"]
    pub noise_mix: FloatParam,
    /// The noise's resonance.
    #[id = "noise_res"]
    pub noise_res: FloatParam,
    /// The filter the noise passes through.
    #[id = "noise_filter_mode"]
    pub noise_filter_mode: EnumParam<NoiseFilterMode>,
    /// The filter's cutoff frequency.
    #[id = "noise_filter_freq"]
    pub noise_filter_freq: FloatParam,
    /// The filter's quality factor.
    #[id = "noise_filter_q"]
    pub noise_filter_q: FloatParam,
    /// The noise envelope's attack time.
    #[id = "noise_att"]
    pub noise_att: FloatParam,
    /// The noise envelope's decay time.
    #[id = "noise_dec"]
    pub noise_dec: FloatParam,
    /// The noise envelope's sustain level.
    #[id = "noise_sus"]
    pub noise_sus: FloatParam,
    /// The noise envelope's release time.
    #[id = "noise_rel"]
    pub noise_rel: FloatParam,
    /// How much the velocity tensions the attack.
    #[id = "noise_att_ten"]
    pub noise_att_ten: FloatParam,
    /// How much the velocity tensions the decay.
    #[id = "noise_dec_ten"]
    pub noise_dec_ten: FloatParam,
    /// How much the velocity tensions the release.
    #[id = "noise_rel_ten"]
    pub noise_rel_ten: FloatParam,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            noise_osc: float_param("noise_osc"),
            noise_mix: float_param("noise_mix"),
            noise_res: float_param("noise_res"),
            noise_filter_mode: EnumParam::new("Noise Filter Mode", NoiseFilterMode::HighPass),
            noise_filter_freq: float_param("noise_filter_freq"),
            noise_filter_q: float_param("noise_filter_q"),
            noise_att: float_param("noise_att"),
            noise_dec: float_param("noise_dec"),
            noise_sus: float_param("noise_sus"),
            noise_rel: float_param("noise_rel"),
            noise_att_ten: float_param("noise_att_ten"),
            noise_dec_ten: float_param("noise_dec_ten"),
            noise_rel_ten: float_param("noise_rel_ten"),
        }
    }
}

/// The parameters of the first resonator.
#[derive(Params)]
pub struct ResonatorAParams {
    /// Whether the resonator sounds at all.
    #[id = "a_on"]
    pub a_on: BoolParam,
    /// The acoustic model the resonator plays.
    #[id = "a_model"]
    pub a_model: EnumParam<ModalModel>,
    /// How many partials the resonator plays.
    #[id = "a_partials"]
    pub a_partials: EnumParam<PartialCount>,
    /// How long the resonator rings.
    #[id = "a_decay"]
    pub a_decay: FloatParam,
    /// The material's damping.
    #[id = "a_damp"]
    pub a_damp: FloatParam,
    /// The resonator's tone.
    #[id = "a_tone"]
    pub a_tone: FloatParam,
    /// Where the mallet hits the resonator.
    #[id = "a_hit"]
    pub a_hit: FloatParam,
    /// How long the resonator takes to release.
    #[id = "a_rel"]
    pub a_rel: FloatParam,
    /// How inharmonic the partials are.
    #[id = "a_inharm"]
    pub a_inharm: FloatParam,
    /// The ratio between the two coupling points.
    #[id = "a_ratio"]
    pub a_ratio: FloatParam,
    /// Where the low cut sits on the logarithmic scale.
    #[id = "a_cut"]
    pub a_cut: FloatParam,
    /// The tube's radius.
    #[id = "a_radius"]
    pub a_radius: FloatParam,
    /// The coarse pitch, in semitones.
    #[id = "a_coarse"]
    pub a_coarse: FloatParam,
    /// The fine pitch, in cents.
    #[id = "a_fine"]
    pub a_fine: FloatParam,
}

impl Default for ResonatorAParams {
    fn default() -> Self {
        Self {
            a_on: bool_param("a_on"),
            a_model: EnumParam::new("A Model", ModalModel::String),
            a_partials: EnumParam::new("A Partials", PartialCount::ThirtyTwo),
            a_decay: float_param("a_decay"),
            a_damp: float_param("a_damp"),
            a_tone: float_param("a_tone"),
            a_hit: float_param("a_hit"),
            a_rel: float_param("a_rel"),
            a_inharm: float_param("a_inharm"),
            a_ratio: float_param("a_ratio"),
            a_cut: float_param("a_cut"),
            a_radius: float_param("a_radius"),
            a_coarse: float_param("a_coarse"),
            a_fine: float_param("a_fine"),
        }
    }
}

/// The parameters of the second resonator.
#[derive(Params)]
pub struct ResonatorBParams {
    /// Whether the resonator sounds at all.
    #[id = "b_on"]
    pub b_on: BoolParam,
    /// The acoustic model the resonator plays.
    #[id = "b_model"]
    pub b_model: EnumParam<ModalModel>,
    /// How many partials the resonator plays.
    #[id = "b_partials"]
    pub b_partials: EnumParam<PartialCount>,
    /// How long the resonator rings.
    #[id = "b_decay"]
    pub b_decay: FloatParam,
    /// The material's damping.
    #[id = "b_damp"]
    pub b_damp: FloatParam,
    /// The resonator's tone.
    #[id = "b_tone"]
    pub b_tone: FloatParam,
    /// Where the mallet hits the resonator.
    #[id = "b_hit"]
    pub b_hit: FloatParam,
    /// How long the resonator takes to release.
    #[id = "b_rel"]
    pub b_rel: FloatParam,
    /// How inharmonic the partials are.
    #[id = "b_inharm"]
    pub b_inharm: FloatParam,
    /// The ratio between the two coupling points.
    #[id = "b_ratio"]
    pub b_ratio: FloatParam,
    /// Where the low cut sits on the logarithmic scale.
    #[id = "b_cut"]
    pub b_cut: FloatParam,
    /// The tube's radius.
    #[id = "b_radius"]
    pub b_radius: FloatParam,
    /// The coarse pitch, in semitones.
    #[id = "b_coarse"]
    pub b_coarse: FloatParam,
    /// The fine pitch, in cents.
    #[id = "b_fine"]
    pub b_fine: FloatParam,
}

impl Default for ResonatorBParams {
    fn default() -> Self {
        Self {
            b_on: bool_param("b_on"),
            b_model: EnumParam::new("B Model", ModalModel::String),
            b_partials: EnumParam::new("B Partials", PartialCount::ThirtyTwo),
            b_decay: float_param("b_decay"),
            b_damp: float_param("b_damp"),
            b_tone: float_param("b_tone"),
            b_hit: float_param("b_hit"),
            b_rel: float_param("b_rel"),
            b_inharm: float_param("b_inharm"),
            b_ratio: float_param("b_ratio"),
            b_cut: float_param("b_cut"),
            b_radius: float_param("b_radius"),
            b_coarse: float_param("b_coarse"),
            b_fine: float_param("b_fine"),
        }
    }
}

/// The parameters that the velocity bends.
#[derive(Params)]
pub struct VelocityParams {
    /// How much the velocity bends the mallet's mix.
    #[id = "vel_mallet_mix"]
    pub vel_mallet_mix: FloatParam,
    /// How much the velocity bends the mallet's resonance.
    #[id = "vel_mallet_res"]
    pub vel_mallet_res: FloatParam,
    /// How much the velocity bends the mallet's stiffness.
    #[id = "vel_mallet_stiff"]
    pub vel_mallet_stiff: FloatParam,
    /// How much the velocity bends the noise's mix.
    #[id = "vel_noise_mix"]
    pub vel_noise_mix: FloatParam,
    /// How much the velocity bends the noise's resonance.
    #[id = "vel_noise_res"]
    pub vel_noise_res: FloatParam,
    /// How much the velocity bends the noise's frequency.
    #[id = "vel_noise_freq"]
    pub vel_noise_freq: FloatParam,
    /// How much the velocity bends the noise's attack.
    #[id = "vel_noise_att"]
    pub vel_noise_att: FloatParam,
    /// How much the velocity bends the noise's decay.
    #[id = "vel_noise_dec"]
    pub vel_noise_dec: FloatParam,
    /// How much the velocity bends the noise's sustain.
    #[id = "vel_noise_sus"]
    pub vel_noise_sus: FloatParam,
    /// How much the velocity bends the noise's release.
    #[id = "vel_noise_rel"]
    pub vel_noise_rel: FloatParam,
    /// How much the velocity bends the noise's quality factor.
    #[id = "vel_noise_q"]
    pub vel_noise_q: FloatParam,
    /// How much the velocity bends the first resonator's decay.
    #[id = "vel_a_decay"]
    pub vel_a_decay: FloatParam,
    /// How much the velocity bends the first resonator's hit position.
    #[id = "vel_a_hit"]
    pub vel_a_hit: FloatParam,
    /// How much the velocity bends the first resonator's inharmonicity.
    #[id = "vel_a_inharm"]
    pub vel_a_inharm: FloatParam,
    /// How much the velocity bends the first resonator's damping.
    #[id = "vel_a_damp"]
    pub vel_a_damp: FloatParam,
    /// How much the velocity bends the first resonator's tone.
    #[id = "vel_a_tone"]
    pub vel_a_tone: FloatParam,
    /// How much the velocity bends the second resonator's decay.
    #[id = "vel_b_decay"]
    pub vel_b_decay: FloatParam,
    /// How much the velocity bends the second resonator's hit position.
    #[id = "vel_b_hit"]
    pub vel_b_hit: FloatParam,
    /// How much the velocity bends the second resonator's inharmonicity.
    #[id = "vel_b_inharm"]
    pub vel_b_inharm: FloatParam,
    /// How much the velocity bends the second resonator's damping.
    #[id = "vel_b_damp"]
    pub vel_b_damp: FloatParam,
    /// How much the velocity bends the second resonator's tone.
    #[id = "vel_b_tone"]
    pub vel_b_tone: FloatParam,
}

impl Default for VelocityParams {
    fn default() -> Self {
        Self {
            vel_mallet_mix: float_param("vel_mallet_mix"),
            vel_mallet_res: float_param("vel_mallet_res"),
            vel_mallet_stiff: float_param("vel_mallet_stiff"),
            vel_noise_mix: float_param("vel_noise_mix"),
            vel_noise_res: float_param("vel_noise_res"),
            vel_noise_freq: float_param("vel_noise_freq"),
            vel_noise_att: float_param("vel_noise_att"),
            vel_noise_dec: float_param("vel_noise_dec"),
            vel_noise_sus: float_param("vel_noise_sus"),
            vel_noise_rel: float_param("vel_noise_rel"),
            vel_noise_q: float_param("vel_noise_q"),
            vel_a_decay: float_param("vel_a_decay"),
            vel_a_hit: float_param("vel_a_hit"),
            vel_a_inharm: float_param("vel_a_inharm"),
            vel_a_damp: float_param("vel_a_damp"),
            vel_a_tone: float_param("vel_a_tone"),
            vel_b_decay: float_param("vel_b_decay"),
            vel_b_hit: float_param("vel_b_hit"),
            vel_b_inharm: float_param("vel_b_inharm"),
            vel_b_damp: float_param("vel_b_damp"),
            vel_b_tone: float_param("vel_b_tone"),
        }
    }
}

/// Every parameter of the plugin.
#[derive(Params)]
pub struct RipplerParams {
    /// The mallet parameters.
    #[nested]
    pub mallet: MalletParams,
    /// The noise parameters.
    #[nested]
    pub noise: NoiseParams,
    /// The first resonator's parameters.
    #[nested]
    pub resonator_a: ResonatorAParams,
    /// The second resonator's parameters.
    #[nested]
    pub resonator_b: ResonatorBParams,
    /// The velocity parameters.
    #[nested]
    pub velocity: VelocityParams,
    /// How the two resonators are coupled.
    #[id = "couple"]
    pub couple: EnumParam<Coupling>,
    /// How much of each resonator reaches the output.
    #[id = "ab_mix"]
    pub ab_mix: FloatParam,
    /// Where the second resonator is fed from in a serial coupling.
    #[id = "ab_split"]
    pub ab_split: FloatParam,
    /// The output's gain, in decibels.
    #[id = "gain"]
    pub gain: FloatParam,
    /// The pitch bend's range, in semitones.
    #[id = "bend_range"]
    pub bend_range: IntParam,
    /// Whether the output is spread over the stereo field.
    #[id = "stereoizer"]
    pub stereoizer: BoolParam,
    /// Whether a voice is reused for the next note.
    #[id = "reuse_voices"]
    pub reuse_voices: BoolParam,
    /// Whether a repeated note fades the previous one out.
    #[id = "fadeout_repeats"]
    pub fadeout_repeats: BoolParam,
}

impl Default for RipplerParams {
    fn default() -> Self {
        Self {
            mallet: MalletParams::default(),
            noise: NoiseParams::default(),
            resonator_a: ResonatorAParams::default(),
            resonator_b: ResonatorBParams::default(),
            velocity: VelocityParams::default(),
            couple: EnumParam::new("Coupling", Coupling::Parallel),
            ab_mix: float_param("ab_mix"),
            ab_split: float_param("ab_split"),
            gain: float_param("gain"),
            bend_range: int_param("bend_range"),
            stereoizer: bool_param("stereoizer"),
            reuse_voices: bool_param("reuse_voices"),
            fadeout_repeats: bool_param("fadeout_repeats"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nice_plug_core::params::range::FloatRange;
    use nice_plug_core::params::Param;

    use super::*;

    /// The identifiers of `src/PluginProcessor.cpp`, in the order they are
    /// declared there.
    const LEGACY_IDS: [&str; 77] = [
        "mallet_type",
        "mallet_pitch",
        "mallet_filter",
        "mallet_mix",
        "mallet_res",
        "mallet_stiff",
        "mallet_ktrack",
        "a_on",
        "a_model",
        "a_partials",
        "a_decay",
        "a_damp",
        "a_tone",
        "a_hit",
        "a_rel",
        "a_inharm",
        "a_ratio",
        "a_cut",
        "a_radius",
        "a_coarse",
        "a_fine",
        "b_on",
        "b_model",
        "b_partials",
        "b_decay",
        "b_damp",
        "b_tone",
        "b_hit",
        "b_rel",
        "b_inharm",
        "b_ratio",
        "b_cut",
        "b_radius",
        "b_coarse",
        "b_fine",
        "noise_osc",
        "noise_mix",
        "noise_res",
        "noise_filter_mode",
        "noise_filter_freq",
        "noise_filter_q",
        "noise_att",
        "noise_dec",
        "noise_sus",
        "noise_rel",
        "noise_att_ten",
        "noise_dec_ten",
        "noise_rel_ten",
        "vel_mallet_mix",
        "vel_mallet_res",
        "vel_mallet_stiff",
        "vel_noise_mix",
        "vel_noise_res",
        "vel_noise_freq",
        "vel_noise_att",
        "vel_noise_dec",
        "vel_noise_sus",
        "vel_noise_rel",
        "vel_noise_q",
        "vel_a_decay",
        "vel_a_hit",
        "vel_a_inharm",
        "vel_a_damp",
        "vel_a_tone",
        "vel_b_decay",
        "vel_b_hit",
        "vel_b_inharm",
        "vel_b_damp",
        "vel_b_tone",
        "couple",
        "ab_mix",
        "ab_split",
        "gain",
        "bend_range",
        "stereoizer",
        "reuse_voices",
        "fadeout_repeats",
    ];

    /// The default of every parameter, taken from `src/PluginProcessor.cpp`.
    ///
    /// A choice parameter's default is its index and a boolean's is `0.0` or
    /// `1.0`, because the legacy state stores plain values.
    const DEFAULTS: [(&str, f64); 77] = [
        ("mallet_type", 0.0),
        ("mallet_pitch", 0.0),
        ("mallet_filter", 0.0),
        ("mallet_mix", 0.0),
        ("mallet_res", 0.8),
        ("mallet_stiff", 600.0),
        ("mallet_ktrack", 0.0),
        ("a_on", 1.0),
        ("a_model", 0.0),
        ("a_partials", 3.0),
        ("a_decay", 1.0),
        ("a_damp", 0.0),
        ("a_tone", 0.0),
        ("a_hit", 0.26),
        ("a_rel", 1.0),
        ("a_inharm", 0.0001),
        ("a_ratio", 1.0),
        ("a_cut", 0.0),
        ("a_radius", 0.5),
        ("a_coarse", 0.0),
        ("a_fine", 0.0),
        ("b_on", 0.0),
        ("b_model", 0.0),
        ("b_partials", 3.0),
        ("b_decay", 1.0),
        ("b_damp", 0.0),
        ("b_tone", 0.0),
        ("b_hit", 0.26),
        ("b_rel", 1.0),
        ("b_inharm", 0.0001),
        ("b_ratio", 1.0),
        ("b_cut", 0.0),
        ("b_radius", 0.5),
        ("b_coarse", 0.0),
        ("b_fine", 0.0),
        ("noise_osc", 0.0),
        ("noise_mix", 0.0),
        ("noise_res", 0.0),
        ("noise_filter_mode", 2.0),
        ("noise_filter_freq", 20.0),
        ("noise_filter_q", 0.707),
        ("noise_att", 1.0),
        ("noise_dec", 500.0),
        ("noise_sus", 0.0),
        ("noise_rel", 500.0),
        ("noise_att_ten", 0.4),
        ("noise_dec_ten", 0.4),
        ("noise_rel_ten", 0.4),
        ("vel_mallet_mix", 0.0),
        ("vel_mallet_res", 0.0),
        ("vel_mallet_stiff", 0.0),
        ("vel_noise_mix", 0.0),
        ("vel_noise_res", 0.0),
        ("vel_noise_freq", 0.0),
        ("vel_noise_att", 0.0),
        ("vel_noise_dec", 0.0),
        ("vel_noise_sus", 0.0),
        ("vel_noise_rel", 0.0),
        ("vel_noise_q", 0.0),
        ("vel_a_decay", 0.0),
        ("vel_a_hit", 0.0),
        ("vel_a_inharm", 0.0),
        ("vel_a_damp", 0.0),
        ("vel_a_tone", 0.0),
        ("vel_b_decay", 0.0),
        ("vel_b_hit", 0.0),
        ("vel_b_inharm", 0.0),
        ("vel_b_damp", 0.0),
        ("vel_b_tone", 0.0),
        ("couple", 0.0),
        ("ab_mix", 0.5),
        ("ab_split", 0.01),
        ("gain", 0.0),
        ("bend_range", 2.0),
        ("stereoizer", 1.0),
        ("reuse_voices", 0.0),
        ("fadeout_repeats", 0.0),
    ];

    /// The range of every float parameter, taken from `src/PluginProcessor.cpp`.
    ///
    /// The skew is JUCE's `NormalisableRange` skew, and the factor is the
    /// `1/(1+skew)` that reproduces JUCE's curve.
    const FLOAT_RANGES: [(&str, f32, f32, Option<f32>); 64] = [
        ("mallet_pitch", -24.0, 24.0, None),
        ("mallet_filter", -1.0, 1.0, None),
        ("mallet_mix", 0.0, 1.0, None),
        ("mallet_res", 0.0, 1.0, None),
        ("mallet_stiff", 100.0, 5000.0, Some(0.3)),
        ("mallet_ktrack", 0.0, 1.0, None),
        ("noise_osc", 0.0, 1.0, Some(1.0)),
        ("noise_mix", 0.0, 1.0, Some(0.3)),
        ("noise_res", 0.0, 1.0, Some(0.3)),
        ("noise_filter_freq", 20.0, 20000.0, Some(0.3)),
        ("noise_filter_q", 0.707, 4.0, None),
        ("noise_att", 1.0, 20000.0, Some(0.3)),
        ("noise_dec", 1.0, 20000.0, Some(0.3)),
        ("noise_sus", 0.0, 1.0, None),
        ("noise_rel", 1.0, 20000.0, Some(0.3)),
        ("noise_att_ten", -1.0, 1.0, None),
        ("noise_dec_ten", -1.0, 1.0, None),
        ("noise_rel_ten", -1.0, 1.0, None),
        ("a_decay", 0.01, 100.0, Some(0.2)),
        ("a_damp", -1.0, 1.0, None),
        ("a_tone", -1.0, 1.0, None),
        ("a_hit", 0.02, 0.5, None),
        ("a_rel", 0.001, 1.0, None),
        ("a_inharm", 0.0001, 1.0, Some(0.3)),
        ("a_ratio", 0.1, 10.0, Some(0.3)),
        ("a_cut", -1.0, 1.0, None),
        ("a_radius", 0.0, 1.0, None),
        ("a_coarse", -48.0, 48.0, Some(1.0)),
        ("a_fine", -99.0, 99.0, Some(1.0)),
        ("b_decay", 0.01, 100.0, Some(0.2)),
        ("b_damp", -1.0, 1.0, None),
        ("b_tone", -1.0, 1.0, None),
        ("b_hit", 0.02, 0.5, None),
        ("b_rel", 0.001, 1.0, None),
        ("b_inharm", 0.0001, 1.0, Some(0.3)),
        ("b_ratio", 0.1, 10.0, Some(0.3)),
        ("b_cut", -1.0, 1.0, None),
        ("b_radius", 0.0, 1.0, None),
        ("b_coarse", -48.0, 48.0, Some(1.0)),
        ("b_fine", -99.0, 99.0, Some(1.0)),
        ("vel_mallet_mix", -1.0, 1.0, None),
        ("vel_mallet_res", -1.0, 1.0, None),
        ("vel_mallet_stiff", -1.0, 1.0, None),
        ("vel_noise_mix", -1.0, 1.0, None),
        ("vel_noise_res", -1.0, 1.0, None),
        ("vel_noise_freq", -1.0, 1.0, None),
        ("vel_noise_att", -1.0, 1.0, None),
        ("vel_noise_dec", -1.0, 1.0, None),
        ("vel_noise_sus", -1.0, 1.0, None),
        ("vel_noise_rel", -1.0, 1.0, None),
        ("vel_noise_q", -1.0, 1.0, None),
        ("vel_a_decay", -1.0, 1.0, None),
        ("vel_a_hit", -1.0, 1.0, None),
        ("vel_a_inharm", -1.0, 1.0, None),
        ("vel_a_damp", -1.0, 1.0, None),
        ("vel_a_tone", -1.0, 1.0, None),
        ("vel_b_decay", -1.0, 1.0, None),
        ("vel_b_hit", -1.0, 1.0, None),
        ("vel_b_inharm", -1.0, 1.0, None),
        ("vel_b_damp", -1.0, 1.0, None),
        ("vel_b_tone", -1.0, 1.0, None),
        ("ab_mix", 0.0, 1.0, None),
        ("ab_split", 0.01, 1.0, Some(0.5)),
        ("gain", -24.0, 24.0, None),
    ];

    /// The default value of every parameter of a parameter set.
    fn defaults(params: &RipplerParams) -> Vec<(&'static str, f64)> {
        vec![
            (
                "mallet_type",
                params.mallet.mallet_type.default_plain_value().to_index() as f64,
            ),
            ("mallet_pitch", params.mallet.mallet_pitch.default_plain_value() as f64),
            (
                "mallet_filter",
                params.mallet.mallet_filter.default_plain_value() as f64,
            ),
            ("mallet_mix", params.mallet.mallet_mix.default_plain_value() as f64),
            ("mallet_res", params.mallet.mallet_res.default_plain_value() as f64),
            (
                "mallet_stiff",
                params.mallet.mallet_stiff.default_plain_value() as f64,
            ),
            (
                "mallet_ktrack",
                params.mallet.mallet_ktrack.default_plain_value() as f64,
            ),
            (
                "a_on",
                params.resonator_a.a_on.default_plain_value() as u8 as f64,
            ),
            (
                "a_model",
                params.resonator_a.a_model.default_plain_value().to_index() as f64,
            ),
            (
                "a_partials",
                params.resonator_a.a_partials.default_plain_value().to_index() as f64,
            ),
            ("a_decay", params.resonator_a.a_decay.default_plain_value() as f64),
            ("a_damp", params.resonator_a.a_damp.default_plain_value() as f64),
            ("a_tone", params.resonator_a.a_tone.default_plain_value() as f64),
            ("a_hit", params.resonator_a.a_hit.default_plain_value() as f64),
            ("a_rel", params.resonator_a.a_rel.default_plain_value() as f64),
            ("a_inharm", params.resonator_a.a_inharm.default_plain_value() as f64),
            ("a_ratio", params.resonator_a.a_ratio.default_plain_value() as f64),
            ("a_cut", params.resonator_a.a_cut.default_plain_value() as f64),
            ("a_radius", params.resonator_a.a_radius.default_plain_value() as f64),
            ("a_coarse", params.resonator_a.a_coarse.default_plain_value() as f64),
            ("a_fine", params.resonator_a.a_fine.default_plain_value() as f64),
            (
                "b_on",
                params.resonator_b.b_on.default_plain_value() as u8 as f64,
            ),
            (
                "b_model",
                params.resonator_b.b_model.default_plain_value().to_index() as f64,
            ),
            (
                "b_partials",
                params.resonator_b.b_partials.default_plain_value().to_index() as f64,
            ),
            ("b_decay", params.resonator_b.b_decay.default_plain_value() as f64),
            ("b_damp", params.resonator_b.b_damp.default_plain_value() as f64),
            ("b_tone", params.resonator_b.b_tone.default_plain_value() as f64),
            ("b_hit", params.resonator_b.b_hit.default_plain_value() as f64),
            ("b_rel", params.resonator_b.b_rel.default_plain_value() as f64),
            ("b_inharm", params.resonator_b.b_inharm.default_plain_value() as f64),
            ("b_ratio", params.resonator_b.b_ratio.default_plain_value() as f64),
            ("b_cut", params.resonator_b.b_cut.default_plain_value() as f64),
            ("b_radius", params.resonator_b.b_radius.default_plain_value() as f64),
            ("b_coarse", params.resonator_b.b_coarse.default_plain_value() as f64),
            ("b_fine", params.resonator_b.b_fine.default_plain_value() as f64),
            ("noise_osc", params.noise.noise_osc.default_plain_value() as f64),
            ("noise_mix", params.noise.noise_mix.default_plain_value() as f64),
            ("noise_res", params.noise.noise_res.default_plain_value() as f64),
            (
                "noise_filter_mode",
                params.noise.noise_filter_mode.default_plain_value().to_index() as f64,
            ),
            (
                "noise_filter_freq",
                params.noise.noise_filter_freq.default_plain_value() as f64,
            ),
            (
                "noise_filter_q",
                params.noise.noise_filter_q.default_plain_value() as f64,
            ),
            ("noise_att", params.noise.noise_att.default_plain_value() as f64),
            ("noise_dec", params.noise.noise_dec.default_plain_value() as f64),
            ("noise_sus", params.noise.noise_sus.default_plain_value() as f64),
            ("noise_rel", params.noise.noise_rel.default_plain_value() as f64),
            (
                "noise_att_ten",
                params.noise.noise_att_ten.default_plain_value() as f64,
            ),
            (
                "noise_dec_ten",
                params.noise.noise_dec_ten.default_plain_value() as f64,
            ),
            (
                "noise_rel_ten",
                params.noise.noise_rel_ten.default_plain_value() as f64,
            ),
            (
                "vel_mallet_mix",
                params.velocity.vel_mallet_mix.default_plain_value() as f64,
            ),
            (
                "vel_mallet_res",
                params.velocity.vel_mallet_res.default_plain_value() as f64,
            ),
            (
                "vel_mallet_stiff",
                params.velocity.vel_mallet_stiff.default_plain_value() as f64,
            ),
            (
                "vel_noise_mix",
                params.velocity.vel_noise_mix.default_plain_value() as f64,
            ),
            (
                "vel_noise_res",
                params.velocity.vel_noise_res.default_plain_value() as f64,
            ),
            (
                "vel_noise_freq",
                params.velocity.vel_noise_freq.default_plain_value() as f64,
            ),
            (
                "vel_noise_att",
                params.velocity.vel_noise_att.default_plain_value() as f64,
            ),
            (
                "vel_noise_dec",
                params.velocity.vel_noise_dec.default_plain_value() as f64,
            ),
            (
                "vel_noise_sus",
                params.velocity.vel_noise_sus.default_plain_value() as f64,
            ),
            (
                "vel_noise_rel",
                params.velocity.vel_noise_rel.default_plain_value() as f64,
            ),
            (
                "vel_noise_q",
                params.velocity.vel_noise_q.default_plain_value() as f64,
            ),
            (
                "vel_a_decay",
                params.velocity.vel_a_decay.default_plain_value() as f64,
            ),
            (
                "vel_a_hit",
                params.velocity.vel_a_hit.default_plain_value() as f64,
            ),
            (
                "vel_a_inharm",
                params.velocity.vel_a_inharm.default_plain_value() as f64,
            ),
            (
                "vel_a_damp",
                params.velocity.vel_a_damp.default_plain_value() as f64,
            ),
            (
                "vel_a_tone",
                params.velocity.vel_a_tone.default_plain_value() as f64,
            ),
            (
                "vel_b_decay",
                params.velocity.vel_b_decay.default_plain_value() as f64,
            ),
            (
                "vel_b_hit",
                params.velocity.vel_b_hit.default_plain_value() as f64,
            ),
            (
                "vel_b_inharm",
                params.velocity.vel_b_inharm.default_plain_value() as f64,
            ),
            (
                "vel_b_damp",
                params.velocity.vel_b_damp.default_plain_value() as f64,
            ),
            (
                "vel_b_tone",
                params.velocity.vel_b_tone.default_plain_value() as f64,
            ),
            ("couple", params.couple.default_plain_value().to_index() as f64),
            ("ab_mix", params.ab_mix.default_plain_value() as f64),
            ("ab_split", params.ab_split.default_plain_value() as f64),
            ("gain", params.gain.default_plain_value() as f64),
            ("bend_range", params.bend_range.default_plain_value() as f64),
            (
                "stereoizer",
                params.stereoizer.default_plain_value() as u8 as f64,
            ),
            (
                "reuse_voices",
                params.reuse_voices.default_plain_value() as u8 as f64,
            ),
            (
                "fadeout_repeats",
                params.fadeout_repeats.default_plain_value() as u8 as f64,
            ),
        ]
    }

    /// The range of every float parameter of a parameter set.
    fn float_ranges(params: &RipplerParams) -> Vec<(&'static str, FloatRange)> {
        vec![
            ("mallet_pitch", params.mallet.mallet_pitch.range()),
            ("mallet_filter", params.mallet.mallet_filter.range()),
            ("mallet_mix", params.mallet.mallet_mix.range()),
            ("mallet_res", params.mallet.mallet_res.range()),
            ("mallet_stiff", params.mallet.mallet_stiff.range()),
            ("mallet_ktrack", params.mallet.mallet_ktrack.range()),
            ("noise_osc", params.noise.noise_osc.range()),
            ("noise_mix", params.noise.noise_mix.range()),
            ("noise_res", params.noise.noise_res.range()),
            ("noise_filter_freq", params.noise.noise_filter_freq.range()),
            ("noise_filter_q", params.noise.noise_filter_q.range()),
            ("noise_att", params.noise.noise_att.range()),
            ("noise_dec", params.noise.noise_dec.range()),
            ("noise_sus", params.noise.noise_sus.range()),
            ("noise_rel", params.noise.noise_rel.range()),
            ("noise_att_ten", params.noise.noise_att_ten.range()),
            ("noise_dec_ten", params.noise.noise_dec_ten.range()),
            ("noise_rel_ten", params.noise.noise_rel_ten.range()),
            ("a_decay", params.resonator_a.a_decay.range()),
            ("a_damp", params.resonator_a.a_damp.range()),
            ("a_tone", params.resonator_a.a_tone.range()),
            ("a_hit", params.resonator_a.a_hit.range()),
            ("a_rel", params.resonator_a.a_rel.range()),
            ("a_inharm", params.resonator_a.a_inharm.range()),
            ("a_ratio", params.resonator_a.a_ratio.range()),
            ("a_cut", params.resonator_a.a_cut.range()),
            ("a_radius", params.resonator_a.a_radius.range()),
            ("a_coarse", params.resonator_a.a_coarse.range()),
            ("a_fine", params.resonator_a.a_fine.range()),
            ("b_decay", params.resonator_b.b_decay.range()),
            ("b_damp", params.resonator_b.b_damp.range()),
            ("b_tone", params.resonator_b.b_tone.range()),
            ("b_hit", params.resonator_b.b_hit.range()),
            ("b_rel", params.resonator_b.b_rel.range()),
            ("b_inharm", params.resonator_b.b_inharm.range()),
            ("b_ratio", params.resonator_b.b_ratio.range()),
            ("b_cut", params.resonator_b.b_cut.range()),
            ("b_radius", params.resonator_b.b_radius.range()),
            ("b_coarse", params.resonator_b.b_coarse.range()),
            ("b_fine", params.resonator_b.b_fine.range()),
            ("vel_mallet_mix", params.velocity.vel_mallet_mix.range()),
            ("vel_mallet_res", params.velocity.vel_mallet_res.range()),
            ("vel_mallet_stiff", params.velocity.vel_mallet_stiff.range()),
            ("vel_noise_mix", params.velocity.vel_noise_mix.range()),
            ("vel_noise_res", params.velocity.vel_noise_res.range()),
            ("vel_noise_freq", params.velocity.vel_noise_freq.range()),
            ("vel_noise_att", params.velocity.vel_noise_att.range()),
            ("vel_noise_dec", params.velocity.vel_noise_dec.range()),
            ("vel_noise_sus", params.velocity.vel_noise_sus.range()),
            ("vel_noise_rel", params.velocity.vel_noise_rel.range()),
            ("vel_noise_q", params.velocity.vel_noise_q.range()),
            ("vel_a_decay", params.velocity.vel_a_decay.range()),
            ("vel_a_hit", params.velocity.vel_a_hit.range()),
            ("vel_a_inharm", params.velocity.vel_a_inharm.range()),
            ("vel_a_damp", params.velocity.vel_a_damp.range()),
            ("vel_a_tone", params.velocity.vel_a_tone.range()),
            ("vel_b_decay", params.velocity.vel_b_decay.range()),
            ("vel_b_hit", params.velocity.vel_b_hit.range()),
            ("vel_b_inharm", params.velocity.vel_b_inharm.range()),
            ("vel_b_damp", params.velocity.vel_b_damp.range()),
            ("vel_b_tone", params.velocity.vel_b_tone.range()),
            ("ab_mix", params.ab_mix.range()),
            ("ab_split", params.ab_split.range()),
            ("gain", params.gain.range()),
        ]
    }

    /// The minimum, maximum and skew factor of a range.
    fn range_parts(range: FloatRange) -> (f32, f32, Option<f32>) {
        match range {
            FloatRange::Linear { min, max } => (min, max, None),
            FloatRange::Skewed { min, max, factor } => (min, max, Some(factor)),
            other => panic!("unexpected range {other:?}"),
        }
    }

    #[test]
    fn there_are_seventy_seven_parameters() {
        assert_eq!(RipplerParams::default().param_map().len(), 77);
        assert_eq!(SPECS.len(), 77);
    }

    #[test]
    fn every_parameter_id_is_the_legacy_one() {
        let ids: BTreeSet<String> = RipplerParams::default()
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        let expected: BTreeSet<String> = LEGACY_IDS.iter().map(|id| id.to_string()).collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn the_parameter_ids_are_unique() {
        let mut seen = BTreeSet::new();
        for (id, _, _) in RipplerParams::default().param_map() {
            assert!(seen.insert(id.clone()), "the id {id} appears twice");
        }
        assert_eq!(seen.len(), 77);
    }

    #[test]
    fn the_defaults_match_the_juce_build() {
        let defaults = defaults(&RipplerParams::default());
        assert_eq!(defaults.len(), DEFAULTS.len());
        for ((id, actual), (expected_id, expected)) in defaults.iter().zip(DEFAULTS.iter()) {
            assert_eq!(id, expected_id);
            assert!(
                (actual - expected).abs() < 1e-6,
                "{id} defaults to {actual}, not {expected}"
            );
        }
    }

    #[test]
    fn the_ranges_match_the_juce_build() {
        let ranges = float_ranges(&RipplerParams::default());
        assert_eq!(ranges.len(), FLOAT_RANGES.len());
        for ((id, range), (expected_id, min, max, skew)) in ranges.iter().zip(FLOAT_RANGES.iter()) {
            assert_eq!(id, expected_id);
            let (actual_min, actual_max, actual_factor) = range_parts(*range);
            assert_eq!(actual_min, *min, "{id} has the wrong minimum");
            assert_eq!(actual_max, *max, "{id} has the wrong maximum");
            match (actual_factor, skew) {
                (None, None) => {}
                (Some(factor), Some(skew)) => {
                    let expected_factor = 1.0 / (1.0 + skew);
                    assert!(
                        (factor - expected_factor).abs() < 1e-12,
                        "{id} has the factor {factor}, not {expected_factor}"
                    );
                }
                (actual, expected) => panic!("{id} has the skew {actual:?}, not {expected:?}"),
            }
        }
    }

    #[test]
    fn the_specifications_match_the_parameters() {
        for spec in SPECS {
            assert!(spec.min < spec.max, "{} has an empty range", spec.id);
            let default = spec.default as f32;
            assert!(
                default >= spec.min && default <= spec.max,
                "{} has a default outside its range",
                spec.id
            );
        }
    }
}
