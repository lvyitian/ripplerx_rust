//! The RipplerX audio plugin.
//!
//! The plugin owns the voices, the sampler, the model tables and the
//! parameters, and drives the per-sample loop that `src/PluginProcessor.cpp`
//! runs.

pub mod identity;

use nice_plug::prelude::*;
use rippler_dsp::comb::Comb;
use rippler_dsp::limiter::Limiter;
use rippler_dsp::luts::Luts;
use rippler_dsp::models::ModelTables;
use rippler_dsp::rng::Rng;
use rippler_dsp::sampler::Sampler;
use rippler_dsp::voice::Voice;
use rippler_params::params::RipplerParams;
use std::sync::Arc;

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
    luts: Luts,
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
}

impl Default for RipplerX {
    fn default() -> Self {
        Self {
            params: Arc::new(RipplerParams::default()),
            voices: Vec::new(),
            sampler: Sampler::new(),
            models: ModelTables::new(),
            luts: Luts::new(44100.0),
            comb: Comb::new(),
            limiter: Limiter::new(44100.0),
            rngs: Vec::new(),
            srate: 44100.0,
            polyphony: DEFAULT_POLYPHONY,
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
        self.srate = buffer_config.sample_rate as f64;

        // The voices, the sampler and the lookup tables are allocated here,
        // because this is the only place where allocating is allowed.
        self.voices = (0..MAX_POLYPHONY).map(|_| Voice::new()).collect();
        self.rngs = (0..MAX_POLYPHONY)
            .map(|index| Rng::new(index as u64 + 1))
            .collect();
        self.sampler = Sampler::new();
        self.luts = Luts::new(self.srate);
        self.comb.init(self.srate);
        self.limiter = Limiter::new(self.srate);

        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // The processing loop arrives with the next task.
        for channel in buffer.as_slice() {
            channel.fill(0.0);
        }

        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {
        self.voices.clear();
        self.rngs.clear();
    }
}
