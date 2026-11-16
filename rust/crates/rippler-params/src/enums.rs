//! The choice parameters' enumerations.
//!
//! Every enumeration here has the same contract: the variant order is the
//! legacy choice order, so the index of a variant is the value the JUCE build
//! stores in its state. Reordering a variant would silently change every
//! saved patch, so the order is part of the format.

use nice_plug_core::params::enums::Enum;

/// The number of partials a resonator plays.
///
/// The order is the legacy choice order and is deliberately not sorted: the
/// legacy state stores the index, so "1" has to stay at index five.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum PartialCount {
    /// Four partials.
    #[name = "4"]
    #[id = "4"]
    Four,
    /// Eight partials.
    #[name = "8"]
    #[id = "8"]
    Eight,
    /// Sixteen partials.
    #[name = "16"]
    #[id = "16"]
    Sixteen,
    /// Thirty two partials.
    #[name = "32"]
    #[id = "32"]
    ThirtyTwo,
    /// Sixty four partials.
    #[name = "64"]
    #[id = "64"]
    SixtyFour,
    /// One partial.
    #[name = "1"]
    #[id = "1"]
    One,
    /// Two partials.
    #[name = "2"]
    #[id = "2"]
    Two,
}

/// The filter the noise passes through.
///
/// The order is the legacy choice order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum NoiseFilterMode {
    /// A low pass filter.
    #[name = "LP"]
    #[id = "lp"]
    LowPass,
    /// A band pass filter.
    #[name = "BP"]
    #[id = "bp"]
    BandPass,
    /// A high pass filter.
    #[name = "HP"]
    #[id = "hp"]
    HighPass,
}

/// How the two resonators are coupled.
///
/// The order is the legacy choice order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Coupling {
    /// The resonators are coupled in parallel.
    #[name = "A+B"]
    #[id = "a_plus_b"]
    Parallel,
    /// The resonators are coupled in series.
    #[name = "A>B"]
    #[id = "a_over_b"]
    Series,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `a_model` and `b_model` choice list of `src/PluginProcessor.cpp`.
    const MODELS: [&str; 12] = [
        "String",
        "Beam",
        "Squared",
        "Membrane",
        "Plate",
        "Drumhead",
        "Marimba",
        "Open Tube",
        "Closed Tube",
        "Marimba2",
        "Bell",
        "Djembe",
    ];

    /// The `mallet_type` choice list of `src/PluginProcessor.cpp`.
    const MALLETS: [&str; 26] = [
        "Impulse",
        "Reserved",
        "Reserved",
        "Reserved",
        "Reserved",
        "Reserved",
        "Reserved",
        "Reserved",
        "Reserved",
        "Reserved",
        "Reserved",
        "User File",
        "Click 1",
        "Click 2",
        "Click 3",
        "Click 4",
        "Click 5",
        "Click 6",
        "Blip",
        "Blop",
        "Metal 1",
        "Metal 2",
        "Wood",
        "Strike",
        "Perc 1",
        "Perc 2",
    ];

    /// The `a_partials` and `b_partials` choice list of
    /// `src/PluginProcessor.cpp`.
    const PARTIALS: [&str; 7] = ["4", "8", "16", "32", "64", "1", "2"];

    /// The `noise_filter_mode` choice list of `src/PluginProcessor.cpp`.
    const FILTER_MODES: [&str; 3] = ["LP", "BP", "HP"];

    /// The `couple` choice list of `src/PluginProcessor.cpp`.
    const COUPLINGS: [&str; 2] = ["A+B", "A>B"];

    #[test]
    fn the_enum_indices_match_the_legacy_choice_order() {
        assert_eq!(
            <rippler_dsp::models::ModalModel as Enum>::variants(),
            &MODELS
        );
        assert_eq!(
            <rippler_dsp::mallet::MalletType as Enum>::variants(),
            &MALLETS
        );
        assert_eq!(PartialCount::variants(), &PARTIALS);
        assert_eq!(NoiseFilterMode::variants(), &FILTER_MODES);
        assert_eq!(Coupling::variants(), &COUPLINGS);
    }

    #[test]
    fn the_enum_indices_round_trip() {
        for index in 0..MODELS.len() {
            let model = <rippler_dsp::models::ModalModel as Enum>::from_index(index);
            assert_eq!(
                <rippler_dsp::models::ModalModel as Enum>::to_index(model),
                index
            );
        }

        for index in 0..MALLETS.len() {
            let mallet = <rippler_dsp::mallet::MalletType as Enum>::from_index(index);
            assert_eq!(
                <rippler_dsp::mallet::MalletType as Enum>::to_index(mallet),
                index
            );
        }

        for index in 0..PARTIALS.len() {
            let count = PartialCount::from_index(index);
            assert_eq!(PartialCount::to_index(count), index);
        }

        for index in 0..FILTER_MODES.len() {
            let mode = NoiseFilterMode::from_index(index);
            assert_eq!(NoiseFilterMode::to_index(mode), index);
        }

        for index in 0..COUPLINGS.len() {
            let coupling = Coupling::from_index(index);
            assert_eq!(Coupling::to_index(coupling), index);
        }
    }

    #[test]
    fn an_index_out_of_range_is_the_first_variant() {
        assert_eq!(
            <rippler_dsp::models::ModalModel as Enum>::from_index(99),
            rippler_dsp::models::ModalModel::String
        );
        assert_eq!(
            <rippler_dsp::mallet::MalletType as Enum>::from_index(99),
            rippler_dsp::mallet::MalletType::Impulse
        );
        assert_eq!(PartialCount::from_index(99), PartialCount::Four);
        assert_eq!(NoiseFilterMode::from_index(99), NoiseFilterMode::LowPass);
        assert_eq!(Coupling::from_index(99), Coupling::Parallel);
    }

    #[test]
    fn the_partial_count_order_is_not_sorted() {
        assert_eq!(
            PartialCount::variants(),
            ["4", "8", "16", "32", "64", "1", "2"]
        );
        assert_eq!(PartialCount::from_index(5), PartialCount::One);
        assert_eq!(PartialCount::from_index(6), PartialCount::Two);
    }

    #[test]
    fn the_stable_ids_match_the_variants_in_order() {
        fn check(variants: &'static [&'static str], ids: &'static [&'static str]) {
            assert_eq!(variants.len(), ids.len());
            for (index, id) in ids.iter().enumerate() {
                assert!(!id.is_empty(), "the id at index {index} is empty");
                assert!(!ids[..index].contains(id), "the id {id} is used twice");
            }
        }

        check(
            <rippler_dsp::models::ModalModel as Enum>::variants(),
            <rippler_dsp::models::ModalModel as Enum>::ids().unwrap(),
        );
        check(
            <rippler_dsp::mallet::MalletType as Enum>::variants(),
            <rippler_dsp::mallet::MalletType as Enum>::ids().unwrap(),
        );
        check(PartialCount::variants(), PartialCount::ids().unwrap());
        check(NoiseFilterMode::variants(), NoiseFilterMode::ids().unwrap());
        check(Coupling::variants(), Coupling::ids().unwrap());
    }
}
