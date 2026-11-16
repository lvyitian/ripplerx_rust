//! The parameter set.
//!
//! The parameters are the JUCE build's parameters, one for one: the same
//! identifiers, the same ranges, the same defaults and the same choices. The
//! identifiers are the legacy ones, verbatim, so a patch saved by either
//! build means the same thing to the other.
//!
//! The ranges are JUCE's `NormalisableRange`s. JUCE normalises a value as the
//! proportion of its place in the range raised to the power of the range's
//! skew, so the skew is the exponent of the curve and a skew of exactly `1.0`
//! is a linear range. nice-plug's `FloatRange::Skewed` normalises as
//! `proportion^factor`, so JUCE's skew is nice-plug's factor. JUCE's
//! `interval` is nice-plug's step size.
//!
//! The values themselves are stored plain, never normalised: JUCE's parameter
//! adapter writes the unnormalised value into the state, so a float parameter
//! holds its plain value, a choice parameter its index and a boolean `0.0` or
//! `1.0`.

use nice_plug_core::params::enums::Enum;
use nice_plug_core::params::range::{FloatRange, IntRange};
use nice_plug_core::params::{BoolParam, EnumParam, FloatParam, IntParam, Param, Params};
use rippler_dsp::mallet::MalletType;
use rippler_dsp::models::ModalModel;

use crate::enums::{Coupling, NoiseFilterMode, PartialCount};
use crate::ranges;

/// The number of mallet choices, `mallet_type`.
const MALLET_CHOICES: f64 = 26.0;
/// The number of model choices, `a_model` and `b_model`.
const MODEL_CHOICES: f64 = 12.0;
/// The number of partial choices, `a_partials` and `b_partials`.
const PARTIAL_CHOICES: f64 = 7.0;
/// The number of noise filter choices, `noise_filter_mode`.
const FILTER_MODE_CHOICES: f64 = 3.0;
/// The number of coupling choices, `couple`.
const COUPLING_CHOICES: f64 = 2.0;

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
/// The table is the single source of truth: the parameters are built from it,
/// the legacy reader reads the legal values of a patch from it, and the
/// mapping from a legacy identifier to the typed parameter is generated from
/// it.
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
    pub(crate) min: f64,
    /// The highest legal value.
    pub(crate) max: f64,
    /// JUCE's skew, if the parameter has one.
    pub(crate) skew: Option<f32>,
    /// JUCE's interval, if the parameter has one.
    pub(crate) step: Option<f64>,
}

/// The definition of every parameter, in one place.
///
/// The macro is expanded more than once: into the [`SPECS`] table and into
/// the mapping from a legacy identifier to the typed parameter. The two
/// therefore cannot drift apart.
///
/// Every entry reads
///
/// ```text
/// ( kind, plain type, "identifier", "name", default, minimum, maximum, skew, step, path )
/// ```
///
/// where the kind is one of `Float`, `Int`, `Bool` and `Choice`, the plain type
/// is the type the parameter holds, and the path is where the parameter lives
/// in [`RipplerParams`].
macro_rules! parameter_definitions {
    ($macro:ident) => {
        $macro! {
            ( Choice, MalletType, "mallet_type", "Mallet Type", 0.0, 0.0, MALLET_CHOICES - 1.0, None, None, mallet.mallet_type );
            ( Float, f32, "mallet_pitch", "Mallet Pitch", 0.0, -24.0, 24.0, None, Some(0.01), mallet.mallet_pitch );
            ( Float, f32, "mallet_filter", "Mallet Filter", 0.0, -1.0, 1.0, None, Some(0.01), mallet.mallet_filter );
            ( Float, f32, "mallet_mix", "Mallet Mix", 0.0, 0.0, 1.0, None, Some(0.01), mallet.mallet_mix );
            ( Float, f32, "mallet_res", "Mallet Resonance", 0.8, 0.0, 1.0, None, Some(0.01), mallet.mallet_res );
            ( Float, f32, "mallet_stiff", "Mallet Stiffness", 600.0, 100.0, 5000.0, Some(0.3), Some(1.0), mallet.mallet_stiff );
            ( Float, f32, "mallet_ktrack", "Mallet Keytrack", 0.0, 0.0, 1.0, None, Some(0.01), mallet.mallet_ktrack );
            ( Float, f32, "noise_osc", "Noise DC", 0.0, 0.0, 1.0, Some(1.0), Some(0.0001), noise.noise_osc );
            ( Float, f32, "noise_mix", "Noise Mix", 0.0, 0.0, 1.0, Some(0.3), Some(0.0001), noise.noise_mix );
            ( Float, f32, "noise_res", "Noise Resonance", 0.0, 0.0, 1.0, Some(0.3), Some(0.0001), noise.noise_res );
            ( Choice, NoiseFilterMode, "noise_filter_mode", "Noise Filter Mode", 2.0, 0.0, FILTER_MODE_CHOICES - 1.0, None, None, noise.noise_filter_mode );
            ( Float, f32, "noise_filter_freq", "Noise Filter Freq", 20.0, 20.0, 20000.0, Some(0.3), Some(1.0), noise.noise_filter_freq );
            ( Float, f32, "noise_filter_q", "Noise Filter Q", 0.707, 0.707, 4.0, None, Some(0.01), noise.noise_filter_q );
            ( Float, f32, "noise_att", "Noise Attack", 1.0, 1.0, 20000.0, Some(0.3), Some(1.0), noise.noise_att );
            ( Float, f32, "noise_dec", "Noise Decay", 500.0, 1.0, 20000.0, Some(0.3), Some(1.0), noise.noise_dec );
            ( Float, f32, "noise_sus", "Noise Sustain", 0.0, 0.0, 1.0, None, Some(0.01), noise.noise_sus );
            ( Float, f32, "noise_rel", "Noise Release", 500.0, 1.0, 20000.0, Some(0.3), Some(1.0), noise.noise_rel );
            ( Float, f32, "noise_att_ten", "Noise Attack Tension", 0.4, -1.0, 1.0, None, Some(0.01), noise.noise_att_ten );
            ( Float, f32, "noise_dec_ten", "Noise Decay Tension", 0.4, -1.0, 1.0, None, Some(0.01), noise.noise_dec_ten );
            ( Float, f32, "noise_rel_ten", "Noise Release Tension", 0.4, -1.0, 1.0, None, Some(0.01), noise.noise_rel_ten );
            ( Bool, bool, "a_on", "A ON", 1.0, 0.0, 1.0, None, None, resonator_a.a_on );
            ( Choice, ModalModel, "a_model", "A Model", 0.0, 0.0, MODEL_CHOICES - 1.0, None, None, resonator_a.a_model );
            ( Choice, PartialCount, "a_partials", "A Partials", 3.0, 0.0, PARTIAL_CHOICES - 1.0, None, None, resonator_a.a_partials );
            ( Float, f32, "a_decay", "A Decay", 1.0, 0.01, 100.0, Some(0.2), Some(0.01), resonator_a.a_decay );
            ( Float, f32, "a_damp", "A Material", 0.0, -1.0, 1.0, None, Some(0.01), resonator_a.a_damp );
            ( Float, f32, "a_tone", "A Tone", 0.0, -1.0, 1.0, None, Some(0.01), resonator_a.a_tone );
            ( Float, f32, "a_hit", "A HitPos", 0.26, 0.02, 0.5, None, Some(0.01), resonator_a.a_hit );
            ( Float, f32, "a_rel", "A Release", 1.0, 0.001, 1.0, None, Some(0.01), resonator_a.a_rel );
            ( Float, f32, "a_inharm", "A Inharmonic", 0.0001, 0.0001, 1.0, Some(0.3), Some(0.0001), resonator_a.a_inharm );
            ( Float, f32, "a_ratio", "A Ratio", 1.0, 0.1, 10.0, Some(0.3), Some(0.01), resonator_a.a_ratio );
            ( Float, f32, "a_cut", "A LowCut", 0.0, -1.0, 1.0, None, Some(0.01), resonator_a.a_cut );
            ( Float, f32, "a_radius", "A Tube Radius", 0.5, 0.0, 1.0, None, Some(0.01), resonator_a.a_radius );
            ( Float, f32, "a_coarse", "A coarse pitch", 0.0, -48.0, 48.0, Some(1.0), Some(1.0), resonator_a.a_coarse );
            ( Float, f32, "a_fine", "A fine pitch", 0.0, -99.0, 99.0, Some(1.0), Some(1.0), resonator_a.a_fine );
            ( Bool, bool, "b_on", "B ON", 0.0, 0.0, 1.0, None, None, resonator_b.b_on );
            ( Choice, ModalModel, "b_model", "B Model", 0.0, 0.0, MODEL_CHOICES - 1.0, None, None, resonator_b.b_model );
            ( Choice, PartialCount, "b_partials", "B Partials", 3.0, 0.0, PARTIAL_CHOICES - 1.0, None, None, resonator_b.b_partials );
            ( Float, f32, "b_decay", "B Decay", 1.0, 0.01, 100.0, Some(0.2), Some(0.01), resonator_b.b_decay );
            ( Float, f32, "b_damp", "B Material", 0.0, -1.0, 1.0, None, Some(0.01), resonator_b.b_damp );
            ( Float, f32, "b_tone", "B Tone", 0.0, -1.0, 1.0, None, Some(0.01), resonator_b.b_tone );
            ( Float, f32, "b_hit", "B HitPos", 0.26, 0.02, 0.5, None, Some(0.01), resonator_b.b_hit );
            ( Float, f32, "b_rel", "B Release", 1.0, 0.001, 1.0, None, Some(0.01), resonator_b.b_rel );
            ( Float, f32, "b_inharm", "B Inharmonic", 0.0001, 0.0001, 1.0, Some(0.3), Some(0.0001), resonator_b.b_inharm );
            ( Float, f32, "b_ratio", "B Ratio", 1.0, 0.1, 10.0, Some(0.3), Some(0.01), resonator_b.b_ratio );
            ( Float, f32, "b_cut", "B LowCut", 0.0, -1.0, 1.0, None, Some(0.01), resonator_b.b_cut );
            ( Float, f32, "b_radius", "B Tube Radius", 0.5, 0.0, 1.0, None, Some(0.01), resonator_b.b_radius );
            ( Float, f32, "b_coarse", "B coarse pitch", 0.0, -48.0, 48.0, Some(1.0), Some(1.0), resonator_b.b_coarse );
            ( Float, f32, "b_fine", "B fine pitch", 0.0, -99.0, 99.0, Some(1.0), Some(1.0), resonator_b.b_fine );
            ( Float, f32, "vel_mallet_mix", "Vel Mallet Mix", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_mallet_mix );
            ( Float, f32, "vel_mallet_res", "Vel Mallet Resonance", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_mallet_res );
            ( Float, f32, "vel_mallet_stiff", "Vel Mallet Stiffness", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_mallet_stiff );
            ( Float, f32, "vel_noise_mix", "Vel Noise Mix", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_mix );
            ( Float, f32, "vel_noise_res", "Vel Noise Resonance", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_res );
            ( Float, f32, "vel_noise_freq", "Vel Noise Frequency", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_freq );
            ( Float, f32, "vel_noise_att", "Vel Noise Attack", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_att );
            ( Float, f32, "vel_noise_dec", "Vel Noise Decay", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_dec );
            ( Float, f32, "vel_noise_sus", "Vel Noise Sustain", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_sus );
            ( Float, f32, "vel_noise_rel", "Vel Noise Release", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_rel );
            ( Float, f32, "vel_noise_q", "Vel Noise Q", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_noise_q );
            ( Float, f32, "vel_a_decay", "Vel A Decay", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_a_decay );
            ( Float, f32, "vel_a_hit", "Vel A Hit", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_a_hit );
            ( Float, f32, "vel_a_inharm", "Vel A Inharmonic", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_a_inharm );
            ( Float, f32, "vel_a_damp", "Vel A Material", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_a_damp );
            ( Float, f32, "vel_a_tone", "Vel A Tone", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_a_tone );
            ( Float, f32, "vel_b_decay", "Vel B Decay", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_b_decay );
            ( Float, f32, "vel_b_hit", "Vel B Hit", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_b_hit );
            ( Float, f32, "vel_b_inharm", "Vel B Inharmonic", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_b_inharm );
            ( Float, f32, "vel_b_damp", "Vel B Material", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_b_damp );
            ( Float, f32, "vel_b_tone", "Vel B Tone", 0.0, -1.0, 1.0, None, Some(0.01), velocity.vel_b_tone );
            ( Choice, Coupling, "couple", "Coupling", 0.0, 0.0, COUPLING_CHOICES - 1.0, None, None, couple );
            ( Float, f32, "ab_mix", "A+B Mix", 0.5, 0.0, 1.0, None, Some(0.01), ab_mix );
            ( Float, f32, "ab_split", "A>B Split", 0.01, 0.01, 1.0, Some(0.5), Some(0.001), ab_split );
            ( Float, f32, "gain", "Res Gain", 0.0, -24.0, 24.0, None, Some(0.01), gain );
            ( Int, i32, "bend_range", "PitchBend Range", 2.0, 1.0, 24.0, None, Some(1.0), bend_range );
            ( Bool, bool, "stereoizer", "Stereoizer", 1.0, 0.0, 1.0, None, None, stereoizer );
            ( Bool, bool, "reuse_voices", "Reuse Voices", 0.0, 0.0, 1.0, None, None, reuse_voices );
            ( Bool, bool, "fadeout_repeats", "Fadeout Repeated Notes", 0.0, 0.0, 1.0, None, None, fadeout_repeats );
        }
    };
}

/// Builds the specification table from the definitions.
macro_rules! define_specifications {
    ($( ( $kind:ident, $plain:ident, $spec_id:literal, $spec_name:literal, $default:expr, $min:expr, $max:expr, $skew:expr, $step:expr, $($path:ident).+ ) );* $(;)?) => {
        /// Every parameter, in the order the groups hold them.
        pub(crate) const SPECS: &[ParameterSpec] = &[
            $(
                ParameterSpec {
                    id: $spec_id,
                    name: $spec_name,
                    kind: ParameterKind::$kind,
                    default: $default,
                    min: $min,
                    max: $max,
                    skew: $skew,
                    step: $step,
                },
            )*
        ];

        /// Checks that every parameter holds the plain type the table says.
        ///
        /// The check is made when the crate is built: the mapping names the
        /// same paths and the same plain types, so a table that has drifted
        /// would not build.
        #[cfg(test)]
        fn check_plain_types(parameters: &RipplerParams) {
            fn check<P, T>(_parameter: &P)
            where
                P: LegacyParameter + Param<Plain = T>,
            {
            }

            $(
                check::<_, $plain>(&parameters.$($path).+);
            )*
        }
    };
}

/// Builds the mapping from a legacy identifier to the typed parameter.
///
/// The mapping is a pair of macros. [`apply_parameter`] sets the parameter a
/// legacy identifier names, and [`with_parameter`] hands the typed parameter
/// to a callback of the caller's choosing. Both are generated from the same
/// definitions as the specification table, so they cannot drift apart.
macro_rules! define_mapping {
    ($( ( $kind:ident, $plain:ident, $spec_id:literal, $spec_name:literal, $default:expr, $min:expr, $max:expr, $skew:expr, $step:expr, $($path:ident).+ ) );* $(;)?) => {
        /// Sets the parameter a legacy identifier names.
        ///
        /// The setter is anything with a `set_parameter` method, such as
        /// nice-plug's `ParamSetter`. The value is a legacy plain value: a
        /// float parameter's value, a choice parameter's index and a boolean's
        /// `0.0` or `1.0`.
        ///
        /// The macro answers `true` when the identifier names a parameter and
        /// `false` when it does not.
        ///
        /// ```
        /// use rippler_params::params::RipplerParams;
        ///
        /// struct Setter;
        ///
        /// impl Setter {
        ///     fn set_parameter<P, V>(&self, _parameter: &P, _value: V) {}
        /// }
        ///
        /// let params = RipplerParams::default();
        /// let setter = Setter;
        ///
        /// assert!(rippler_params::apply_parameter!(&params, &setter, "a_decay", 1.0));
        /// assert!(!rippler_params::apply_parameter!(&params, &setter, "nope", 1.0));
        /// ```
        #[macro_export]
        macro_rules! apply_parameter {
            ($params:expr, $setter:expr, $id:expr, $value:expr) => {{
                match $id {
                    $(
                        $spec_id => {
                            $setter.set_parameter(
                                &$params.$($path).+,
                                $crate::params::LegacyParameter::legacy_value(
                                    &$params.$($path).+,
                                    $value,
                                ),
                            );
                            true
                        }
                    )*
                    _ => false,
                }
            }};
        }

        /// Hands the parameter a legacy identifier names to a callback.
        ///
        /// The callback is a macro of the caller's, which is called with the
        /// typed parameter and the typed value:
        ///
        /// ```
        /// use rippler_params::params::RipplerParams;
        ///
        /// macro_rules! show {
        ///     ($param:expr, $value:expr) => {
        ///         println!("{:?}", $value);
        ///     };
        /// }
        ///
        /// let params = RipplerParams::default();
        ///
        /// assert!(rippler_params::with_parameter!(&params, "a_decay", show, 1.0));
        /// assert!(!rippler_params::with_parameter!(&params, "nope", show, 1.0));
        /// ```
        ///
        /// The macro answers `true` when the identifier names a parameter and
        /// `false` when it does not.
        #[macro_export]
        macro_rules! with_parameter {
            ($params:expr, $id:expr, $callback:ident, $value:expr) => {{
                match $id {
                    $(
                        $spec_id => {
                            $callback!(
                                &$params.$($path).+,
                                $crate::params::LegacyParameter::legacy_value(
                                    &$params.$($path).+,
                                    $value,
                                )
                            );
                            true
                        }
                    )*
                    _ => false,
                }
            }};
        }
    };
}

parameter_definitions!(define_specifications);
parameter_definitions!(define_mapping);

/// A parameter whose plain value can be made from a legacy plain value.
///
/// The legacy state holds plain values: a float parameter its value, a choice
/// parameter its index and a boolean `0.0` or `1.0`. The mapping from a
/// legacy identifier to the typed parameter uses this to turn a legacy value
/// into the value the parameter holds.
pub trait LegacyParameter: Param {
    /// Makes the parameter's plain value from a legacy plain value.
    fn legacy_value(&self, value: f64) -> Self::Plain;
}

impl LegacyParameter for FloatParam {
    fn legacy_value(&self, value: f64) -> f32 {
        value as f32
    }
}

impl LegacyParameter for IntParam {
    fn legacy_value(&self, value: f64) -> i32 {
        value.round() as i32
    }
}

impl LegacyParameter for BoolParam {
    fn legacy_value(&self, value: f64) -> bool {
        value >= 0.5
    }
}

impl<T: Enum + PartialEq> LegacyParameter for EnumParam<T> {
    fn legacy_value(&self, value: f64) -> T {
        let last = T::variants().len().saturating_sub(1) as f64;
        let index = value.round().clamp(0.0, last) as usize;
        T::from_index(index)
    }
}

/// The specification of a parameter, if the plugin has one.
pub(crate) fn spec(id: &str) -> Option<&'static ParameterSpec> {
    SPECS.iter().find(|spec| spec.id == id)
}

/// The range of a parameter, JUCE's way.
///
/// A skew of exactly `1.0` is not a skew at all: JUCE's `NormalisableRange`
/// treats it as a linear range, so `a_coarse` and its kin are linear and
/// their middle sits in the middle of the knob.
fn range_of(spec: &ParameterSpec) -> FloatRange {
    match spec.skew {
        Some(skew) if skew != 1.0 => ranges::skewed(spec.min as f32, spec.max as f32, skew),
        _ => ranges::linear(spec.min as f32, spec.max as f32),
    }
}

/// Builds the float parameter a specification describes.
fn float_param(id: &str) -> FloatParam {
    let spec = spec(id).expect("the parameter has a specification");
    let param = FloatParam::new(spec.name, spec.default as f32, range_of(spec));
    match spec.step {
        Some(step) => param.with_step_size(step as f32),
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
    use std::cell::RefCell;
    use std::collections::BTreeSet;

    use nice_plug_core::params::Param;
    use nice_plug_core::params::enums::Enum;
    use nice_plug_core::params::range::FloatRange;

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
    /// The skew is JUCE's `NormalisableRange` skew, which is the exponent of
    /// the curve. A skew of exactly `1.0` is a linear range.
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

    /// The interval of every parameter, taken from `src/PluginProcessor.cpp`.
    ///
    /// A float parameter declared with JUCE's four-argument constructor has an
    /// interval of `0.01`, even though the source does not say so: the
    /// constructor fills it in. A parameter declared with an explicit
    /// `NormalisableRange` keeps the interval of that range. An integer
    /// parameter snaps to whole numbers, and a choice or a boolean is discrete
    /// by nature and has no interval of its own.
    const INTERVALS: [(&str, Option<f64>); 77] = [
        ("mallet_type", None),
        ("mallet_pitch", Some(0.01)),
        ("mallet_filter", Some(0.01)),
        ("mallet_mix", Some(0.01)),
        ("mallet_res", Some(0.01)),
        ("mallet_stiff", Some(1.0)),
        ("mallet_ktrack", Some(0.01)),
        ("noise_osc", Some(0.0001)),
        ("noise_mix", Some(0.0001)),
        ("noise_res", Some(0.0001)),
        ("noise_filter_mode", None),
        ("noise_filter_freq", Some(1.0)),
        ("noise_filter_q", Some(0.01)),
        ("noise_att", Some(1.0)),
        ("noise_dec", Some(1.0)),
        ("noise_sus", Some(0.01)),
        ("noise_rel", Some(1.0)),
        ("noise_att_ten", Some(0.01)),
        ("noise_dec_ten", Some(0.01)),
        ("noise_rel_ten", Some(0.01)),
        ("a_on", None),
        ("a_model", None),
        ("a_partials", None),
        ("a_decay", Some(0.01)),
        ("a_damp", Some(0.01)),
        ("a_tone", Some(0.01)),
        ("a_hit", Some(0.01)),
        ("a_rel", Some(0.01)),
        ("a_inharm", Some(0.0001)),
        ("a_ratio", Some(0.01)),
        ("a_cut", Some(0.01)),
        ("a_radius", Some(0.01)),
        ("a_coarse", Some(1.0)),
        ("a_fine", Some(1.0)),
        ("b_on", None),
        ("b_model", None),
        ("b_partials", None),
        ("b_decay", Some(0.01)),
        ("b_damp", Some(0.01)),
        ("b_tone", Some(0.01)),
        ("b_hit", Some(0.01)),
        ("b_rel", Some(0.01)),
        ("b_inharm", Some(0.0001)),
        ("b_ratio", Some(0.01)),
        ("b_cut", Some(0.01)),
        ("b_radius", Some(0.01)),
        ("b_coarse", Some(1.0)),
        ("b_fine", Some(1.0)),
        ("vel_mallet_mix", Some(0.01)),
        ("vel_mallet_res", Some(0.01)),
        ("vel_mallet_stiff", Some(0.01)),
        ("vel_noise_mix", Some(0.01)),
        ("vel_noise_res", Some(0.01)),
        ("vel_noise_freq", Some(0.01)),
        ("vel_noise_att", Some(0.01)),
        ("vel_noise_dec", Some(0.01)),
        ("vel_noise_sus", Some(0.01)),
        ("vel_noise_rel", Some(0.01)),
        ("vel_noise_q", Some(0.01)),
        ("vel_a_decay", Some(0.01)),
        ("vel_a_hit", Some(0.01)),
        ("vel_a_inharm", Some(0.01)),
        ("vel_a_damp", Some(0.01)),
        ("vel_a_tone", Some(0.01)),
        ("vel_b_decay", Some(0.01)),
        ("vel_b_hit", Some(0.01)),
        ("vel_b_inharm", Some(0.01)),
        ("vel_b_damp", Some(0.01)),
        ("vel_b_tone", Some(0.01)),
        ("couple", None),
        ("ab_mix", Some(0.01)),
        ("ab_split", Some(0.001)),
        ("gain", Some(0.01)),
        ("bend_range", Some(1.0)),
        ("stereoizer", None),
        ("reuse_voices", None),
        ("fadeout_repeats", None),
    ];

    /// A setter that records the parameters it was given.
    #[derive(Default)]
    struct RecordingSetter {
        seen: RefCell<Vec<String>>,
    }

    impl RecordingSetter {
        fn set_parameter<P: Param>(&self, param: &P, _value: P::Plain) {
            self.seen.borrow_mut().push(param.name().to_string());
        }
    }

    /// The default value of every parameter of a parameter set.
    fn defaults(params: &RipplerParams) -> Vec<(&'static str, f64)> {
        vec![
            (
                "mallet_type",
                params.mallet.mallet_type.default_plain_value().to_index() as f64,
            ),
            (
                "mallet_pitch",
                params.mallet.mallet_pitch.default_plain_value() as f64,
            ),
            (
                "mallet_filter",
                params.mallet.mallet_filter.default_plain_value() as f64,
            ),
            (
                "mallet_mix",
                params.mallet.mallet_mix.default_plain_value() as f64,
            ),
            (
                "mallet_res",
                params.mallet.mallet_res.default_plain_value() as f64,
            ),
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
                params
                    .resonator_a
                    .a_partials
                    .default_plain_value()
                    .to_index() as f64,
            ),
            (
                "a_decay",
                params.resonator_a.a_decay.default_plain_value() as f64,
            ),
            (
                "a_damp",
                params.resonator_a.a_damp.default_plain_value() as f64,
            ),
            (
                "a_tone",
                params.resonator_a.a_tone.default_plain_value() as f64,
            ),
            (
                "a_hit",
                params.resonator_a.a_hit.default_plain_value() as f64,
            ),
            (
                "a_rel",
                params.resonator_a.a_rel.default_plain_value() as f64,
            ),
            (
                "a_inharm",
                params.resonator_a.a_inharm.default_plain_value() as f64,
            ),
            (
                "a_ratio",
                params.resonator_a.a_ratio.default_plain_value() as f64,
            ),
            (
                "a_cut",
                params.resonator_a.a_cut.default_plain_value() as f64,
            ),
            (
                "a_radius",
                params.resonator_a.a_radius.default_plain_value() as f64,
            ),
            (
                "a_coarse",
                params.resonator_a.a_coarse.default_plain_value() as f64,
            ),
            (
                "a_fine",
                params.resonator_a.a_fine.default_plain_value() as f64,
            ),
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
                params
                    .resonator_b
                    .b_partials
                    .default_plain_value()
                    .to_index() as f64,
            ),
            (
                "b_decay",
                params.resonator_b.b_decay.default_plain_value() as f64,
            ),
            (
                "b_damp",
                params.resonator_b.b_damp.default_plain_value() as f64,
            ),
            (
                "b_tone",
                params.resonator_b.b_tone.default_plain_value() as f64,
            ),
            (
                "b_hit",
                params.resonator_b.b_hit.default_plain_value() as f64,
            ),
            (
                "b_rel",
                params.resonator_b.b_rel.default_plain_value() as f64,
            ),
            (
                "b_inharm",
                params.resonator_b.b_inharm.default_plain_value() as f64,
            ),
            (
                "b_ratio",
                params.resonator_b.b_ratio.default_plain_value() as f64,
            ),
            (
                "b_cut",
                params.resonator_b.b_cut.default_plain_value() as f64,
            ),
            (
                "b_radius",
                params.resonator_b.b_radius.default_plain_value() as f64,
            ),
            (
                "b_coarse",
                params.resonator_b.b_coarse.default_plain_value() as f64,
            ),
            (
                "b_fine",
                params.resonator_b.b_fine.default_plain_value() as f64,
            ),
            (
                "noise_osc",
                params.noise.noise_osc.default_plain_value() as f64,
            ),
            (
                "noise_mix",
                params.noise.noise_mix.default_plain_value() as f64,
            ),
            (
                "noise_res",
                params.noise.noise_res.default_plain_value() as f64,
            ),
            (
                "noise_filter_mode",
                params
                    .noise
                    .noise_filter_mode
                    .default_plain_value()
                    .to_index() as f64,
            ),
            (
                "noise_filter_freq",
                params.noise.noise_filter_freq.default_plain_value() as f64,
            ),
            (
                "noise_filter_q",
                params.noise.noise_filter_q.default_plain_value() as f64,
            ),
            (
                "noise_att",
                params.noise.noise_att.default_plain_value() as f64,
            ),
            (
                "noise_dec",
                params.noise.noise_dec.default_plain_value() as f64,
            ),
            (
                "noise_sus",
                params.noise.noise_sus.default_plain_value() as f64,
            ),
            (
                "noise_rel",
                params.noise.noise_rel.default_plain_value() as f64,
            ),
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
            (
                "couple",
                params.couple.default_plain_value().to_index() as f64,
            ),
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
                // A skew of exactly one is a linear range, not a skew.
                (None, Some(1.0)) => {}
                (Some(factor), Some(skew)) => {
                    assert!(
                        (factor - skew).abs() < 1e-12,
                        "{id} has the factor {factor}, not {skew}"
                    );
                }
                (actual, expected) => panic!("{id} has the skew {actual:?}, not {expected:?}"),
            }
        }
    }

    #[test]
    fn a_skew_of_one_is_a_linear_range() {
        let params = RipplerParams::default();
        for (range, middle) in [
            (params.resonator_a.a_coarse.range(), 0.0f32),
            (params.resonator_a.a_fine.range(), 0.0),
            (params.resonator_b.b_coarse.range(), 0.0),
            (params.resonator_b.b_fine.range(), 0.0),
            (params.noise.noise_osc.range(), 0.5),
        ] {
            assert!(
                matches!(range, FloatRange::Linear { .. }),
                "{range:?} is not linear"
            );
            assert_eq!(range.normalize(middle), 0.5, "{range:?} is not linear");
        }
    }

    #[test]
    fn a_skew_is_the_exponent_of_the_curve() {
        let params = RipplerParams::default();
        // JUCE normalises as `proportion^skew`, so the middle of a skewed
        // range is not in the middle of the knob.
        let decay = params.resonator_a.a_decay.range();
        assert!((decay.normalize(50.005) - 0.5f32.powf(0.2)).abs() < 1e-6);
        assert!((decay.normalize(50.005) - 0.5f32.powf(1.0 / 1.2)).abs() > 1e-2);
    }

    #[test]
    fn the_intervals_match_the_juce_build() {
        assert_eq!(INTERVALS.len(), SPECS.len());
        for (id, expected) in INTERVALS {
            let spec = spec(id).expect("the parameter has a specification");
            assert_eq!(spec.step, expected, "{id} has the wrong interval");
        }
    }

    #[test]
    fn every_parameter_holds_the_plain_type_the_table_says() {
        check_plain_types(&RipplerParams::default());
    }

    #[test]
    fn the_specifications_match_the_parameters() {
        for spec in SPECS {
            assert!(spec.min < spec.max, "{} has an empty range", spec.id);
            let default = spec.default;
            assert!(
                default >= spec.min && default <= spec.max,
                "{} has a default outside its range",
                spec.id
            );
        }
    }

    #[test]
    fn every_parameter_is_reachable_through_the_mapping() {
        let params = RipplerParams::default();
        let setter = RecordingSetter::default();

        for spec in SPECS {
            let before = setter.seen.borrow().len();
            assert!(
                apply_parameter!(&params, &setter, spec.id, 0.0),
                "{} is not reachable through the mapping",
                spec.id
            );
            let seen = setter.seen.borrow();
            assert_eq!(seen.len(), before + 1, "{} was not set", spec.id);
            assert_eq!(
                seen[before], spec.name,
                "{} reaches the wrong parameter",
                spec.id
            );
        }
    }

    #[test]
    fn an_unknown_identifier_is_rejected() {
        let params = RipplerParams::default();
        let setter = RecordingSetter::default();

        assert!(!apply_parameter!(&params, &setter, "not_a_parameter", 1.0));
        assert!(setter.seen.borrow().is_empty());
    }

    #[test]
    fn the_mapping_hands_over_the_typed_parameter() {
        let params = RipplerParams::default();
        let names = RefCell::new(Vec::new());

        macro_rules! record {
            ($param:expr, $value:expr) => {
                names.borrow_mut().push($param.name().to_string());
            };
        }

        assert!(with_parameter!(&params, "a_decay", record, 1.0));
        assert_eq!(names.borrow().as_slice(), ["A Decay"]);
        assert!(!with_parameter!(&params, "not_a_parameter", record, 1.0));
        assert_eq!(names.borrow().len(), 1);
    }

    #[test]
    fn the_mapping_converts_the_legacy_values() {
        let params = RipplerParams::default();

        assert_eq!(
            LegacyParameter::legacy_value(&params.resonator_a.a_decay, 1.5),
            1.5
        );
        assert_eq!(LegacyParameter::legacy_value(&params.bend_range, 2.6), 3);
        assert!(LegacyParameter::legacy_value(&params.resonator_a.a_on, 1.0));
        assert!(!LegacyParameter::legacy_value(
            &params.resonator_b.b_on,
            0.0
        ));
        assert_eq!(
            LegacyParameter::legacy_value(&params.couple, 1.0),
            Coupling::Series
        );
        assert_eq!(
            LegacyParameter::legacy_value(&params.couple, 9.0),
            Coupling::Series
        );
        assert_eq!(
            LegacyParameter::legacy_value(&params.couple, -1.0),
            Coupling::Parallel
        );
    }
}
