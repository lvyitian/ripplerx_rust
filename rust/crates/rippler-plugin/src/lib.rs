//! The RipplerX audio plugin.
//!
//! The plugin owns the voices, the sampler, the model tables and the
//! parameters, and drives the per-sample loop that `src/PluginProcessor.cpp`
//! runs.

pub mod block_params;
pub mod identity;
pub mod params;
pub mod process;
pub mod tuning;

#[cfg(test)]
pub(crate) mod test_helpers;

use mts_client_rs::Client;
use nice_plug::prelude::*;
use rippler_dsp::comb::Comb;
use rippler_dsp::limiter::Limiter;
use rippler_dsp::luts::Luts;
use rippler_dsp::mallet::MalletType;
use rippler_dsp::models::ModelTables;
use rippler_dsp::rng::Rng;
use rippler_dsp::sampler::Sampler;
use rippler_dsp::voice::Voice;
use rippler_params::params::RipplerParams;
use std::sync::Arc;

use crate::params::{MALLET_TYPE_COUNT, ParamState, PreparedSample, prepare_internal_samples};
use crate::process::{MIDI_QUEUE_CAPACITY, MidiState};

/// The number of voices the plugin has, `globals::MAX_POLYPHONY`.
///
/// The JUCE build keeps sixteen voices around and uses the first
/// `polyphony` of them, which is eight by default.
pub const MAX_POLYPHONY: usize = 16;

/// The number of voices the plugin starts with.
///
/// The JUCE build keeps the polyphony in its settings file, and reads eight
/// when the file does not say otherwise.
pub const DEFAULT_POLYPHONY: usize = 8;

/// The time the pitch bend takes to glide, in milliseconds.
///
/// The C++'s `globals::BEND_GLIDE_MS`.
const BEND_GLIDE_MS: f64 = 2.0;

/// The plugin.
pub struct RipplerX {
    /// The parameters, shared with the editor.
    params: Arc<RipplerParams>,
    /// The voices.
    voices: Vec<Voice>,
    /// The sampler, shared with the voices.
    sampler: Sampler,
    /// The current model tables.
    models: ModelTables,
    /// The lookup tables the partials need.
    ///
    /// The tables are built when the plugin is activated, because they are
    /// expensive and a plugin that was never activated does not need them.
    luts: Option<Luts>,
    /// The comb filter that spreads the output over the stereo field.
    comb: Comb,
    /// The limiter.
    limiter: Limiter,
    /// The noise generator's random number generators, one per voice.
    rngs: Vec<Rng>,
    /// The sample rate.
    srate: f64,
    /// The number of voices in use.
    polyphony: usize,
    /// The state of the MIDI input.
    midi: MidiState,
    /// The state of the parameters that have been applied to the voices.
    params_state: ParamState,
    /// The range of the noise's mix.
    noise_mix_range: FloatRange,
    /// The range of the noise's resonance.
    noise_res_range: FloatRange,
    /// The bundled samples, decoded when the plugin was activated.
    ///
    /// The cache holds every bundled sample but the one that is in the
    /// sampler, so that a change of the mallet type is a move of a
    /// waveform rather than a decoding on the audio thread.
    internal_samples: [Option<PreparedSample>; MALLET_TYPE_COUNT],
    /// The mallet type whose sample is in the sampler.
    ///
    /// It is `None` when the sampler does not hold one of the bundled
    /// samples, which is the case before the first one is loaded and after
    /// a user file has been loaded instead.
    loaded_sample: Option<MalletType>,
    /// The MTS-ESP client, if the library is installed.
    ///
    /// The client is created on the thread that builds the plugin, which is
    /// not the audio thread, and it is dropped with the plugin.
    tuning: Option<Client>,
}

impl Default for RipplerX {
    /// Builds a plugin that has not been activated yet.
    ///
    /// Everything that depends on the sample rate is provisional here and is
    /// built again by [`RipplerX::prepare`], which is where the plugin is
    /// activated. The defaults are deliberately cheap: the lookup tables are
    /// the expensive part, so they are not built at all until the plugin is
    /// activated.
    fn default() -> Self {
        Self {
            params: Arc::new(RipplerParams::default()),
            voices: Vec::new(),
            sampler: Sampler::new(),
            models: ModelTables::new(),
            luts: None,
            comb: Comb::new(),
            // The limiter and the sample rate are provisional: they are
            // built again for the plugin's own sample rate when the plugin
            // is activated.
            limiter: Limiter::new(44100.0),
            rngs: Vec::new(),
            srate: 44100.0,
            polyphony: DEFAULT_POLYPHONY,
            midi: MidiState::default(),
            params_state: ParamState::default(),
            noise_mix_range: FloatRange::Linear { min: 0.0, max: 1.0 },
            noise_res_range: FloatRange::Linear { min: 0.0, max: 1.0 },
            internal_samples: std::array::from_fn(|_| None),
            loaded_sample: None,
            // The client is optional: without the MTS-ESP library the
            // plugin falls back to the equal temperament.
            tuning: Client::new().ok(),
        }
    }
}

impl Plugin for RipplerX {
    const NAME: &'static str = "RipplerX";
    const VENDOR: &'static str = "Tilr";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";

    const VERSION: &'static str = "1.5.18";

    /// The plugin has a stereo input and a stereo output.
    ///
    /// The input is the "Sidechain" bus of the JUCE build, which is a normal
    /// main input and not an auxiliary bus: the JUCE build declares it with
    /// `BusesProperties().withInput("Sidechain", stereo, true)`, and the
    /// README documents routing a track into the plugin.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    /// The plugin needs the MIDI control changes as well as the notes.
    ///
    /// The JUCE build listens to the sustain pedal and to the pitch wheel,
    /// and nice-plug only hands those over when the plugin asks for the MIDI
    /// control changes.
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    /// The editor comes with the next plan.
    type Editor = ();
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn activate(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        self.prepare(buffer_config.sample_rate as f64);

        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // The events of the block are queued, the way the JUCE build queues
        // the MIDI messages it is handed.
        while let Some(event) = context.next_event() {
            self.queue_event(event);
        }

        self.process_block(buffer);

        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {
        self.voices.clear();
        self.rngs.clear();
    }
}

impl RipplerX {
    /// Allocates everything that depends on the sample rate.
    ///
    /// This is the only place where allocating is allowed, because it is
    /// called from [`Plugin::activate`].
    pub(crate) fn prepare(&mut self, srate: f64) {
        self.srate = srate;

        // The voices, the sampler and the lookup tables are allocated here,
        // because this is the only place where allocating is allowed.
        self.voices = (0..MAX_POLYPHONY).map(|_| Voice::new()).collect();
        self.rngs = (0..MAX_POLYPHONY)
            .map(|index| Rng::new(index as u64 + 1))
            .collect();
        self.sampler = Sampler::new();
        self.luts = Some(Luts::new(self.srate));
        self.comb.init(self.srate);
        self.limiter = Limiter::new(self.srate);

        // The bundled samples are decoded here, so that a change of the
        // mallet type does not decode anything on the audio thread.
        self.internal_samples = prepare_internal_samples();
        self.loaded_sample = None;

        // The ranges of the noise's mix and resonance never change, so they
        // are read once here.
        self.noise_mix_range = self.params.noise.noise_mix.range();
        self.noise_res_range = self.params.noise.noise_res.range();

        // The MIDI queue is allocated once, so that the audio thread never
        // allocates.
        self.midi.queue.reserve(MIDI_QUEUE_CAPACITY);
        self.midi.sustain_pedal_notes.reserve(MIDI_QUEUE_CAPACITY);

        // The pitch bend glides over two milliseconds.
        self.midi.bend.total = (BEND_GLIDE_MS * 0.001 * self.srate) as i64;
    }
}

impl ClapPlugin for RipplerX {
    /// The bundle identifier of the JUCE build.
    const CLAP_ID: &'static str = identity::CLAP_ID;

    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A physically modelled modal synthesiser with two resonators");

    const CLAP_MANUAL_URL: Option<&'static str> = Some("https://github.com/tiagolr/ripplerx");

    const CLAP_SUPPORT_URL: Option<&'static str> = Some("https://github.com/tiagolr/ripplerx");

    /// The plugin is an instrument with a stereo output.
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument, ClapFeature::Stereo];
}

impl Vst3Plugin for RipplerX {
    /// The class ID of the JUCE build, in its canonical order.
    ///
    /// The JUCE build's class ID is not the same on every platform: the
    /// byte order of its first two words depends on the platform JUCE was
    /// compiled for, and it is not the byte swap nice-plug does on Windows.
    /// The platform's own class ID is therefore given in
    /// [`PLATFORM_VST3_CLASS_ID`][Self::PLATFORM_VST3_CLASS_ID].
    const VST3_CLASS_ID: [u8; 16] = identity::VST3_CLASS_ID_POSIX;

    /// The class ID of the JUCE build on this platform.
    ///
    /// This is the value the host sees: the VST3 factory reports it as the
    /// plugin's class ID, and it is what the module information file holds.
    const PLATFORM_VST3_CLASS_ID: [u8; 16] = identity::VST3_CLASS_ID;

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument];
}

nice_export_clap!(RipplerX);
nice_export_vst3!(RipplerX);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_class_id_of_this_platform_is_the_juce_builds_class_id() {
        assert_eq!(
            <RipplerX as Vst3Plugin>::PLATFORM_VST3_CLASS_ID,
            identity::VST3_CLASS_ID
        );
    }

    #[test]
    fn the_clap_identifier_is_the_juce_builds_identifier() {
        assert_eq!(<RipplerX as ClapPlugin>::CLAP_ID, "com.tilr.ripplerx");
    }
}
