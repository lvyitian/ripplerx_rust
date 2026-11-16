//! The parameters and the voice bank.
//!
//! Port of `RipplerXAudioProcessor::onSlider` and of
//! `RipplerXAudioProcessor::pickVoice` in `src/PluginProcessor.cpp`.

use rippler_dsp::mallet::MalletType;
use rippler_dsp::models::ModalModel;
use rippler_dsp::voice::Voice;

use crate::RipplerX;
use crate::process::BlockParams;

/// The ratio a model starts with.
///
/// The JUCE build sets the ratio to a model specific default when the model
/// changes: two for the beam, one for the djembe and 0.78 for every other
/// model.
pub(crate) fn default_ratio(model: ModalModel) -> f64 {
    match model {
        ModalModel::Beam => 2.0,
        ModalModel::Djembe => 1.0,
        _ => 0.78,
    }
}

/// The state of the parameters that have been applied to the voices.
///
/// The JUCE build keeps the last model and the last number of partials
/// around, so that it can tell whether the model changed. The same is done
/// here, together with the ratio the voices actually use.
#[derive(Default)]
pub(crate) struct ParamState {
    /// The parameters as they were last applied.
    applied: Option<BlockParams>,
    /// The first resonator's model as it was last applied.
    last_a_model: Option<ModalModel>,
    /// The second resonator's model as it was last applied.
    last_b_model: Option<ModalModel>,
    /// The first resonator's number of partials as it was last applied.
    last_a_partials: Option<usize>,
    /// The second resonator's number of partials as it was last applied.
    last_b_partials: Option<usize>,
    /// The mallet as it was last applied.
    last_mallet_type: Option<MalletType>,
    /// The ratio the first resonator uses.
    ratio_a: f64,
    /// The ratio the second resonator uses.
    ratio_b: f64,
    /// The ratio parameter of the first resonator as it was last seen.
    last_a_ratio: Option<f64>,
    /// The ratio parameter of the second resonator as it was last seen.
    last_b_ratio: Option<f64>,
}

impl RipplerX {
    /// Applies the parameters to the voices.
    ///
    /// The JUCE build calls `onSlider` whenever a parameter changes, and it
    /// is called here when the parameters differ from the ones that were
    /// applied last.
    pub(crate) fn apply_parameters(&mut self, block: &BlockParams) {
        if self.params_state.applied.as_ref() == Some(block) {
            return;
        }

        // A change of the model resets the ratio and clears the voices.
        if apply_ratio(
            &mut self.params_state.ratio_a,
            &mut self.params_state.last_a_ratio,
            &mut self.params_state.last_a_model,
            block.a_model,
            block.a_ratio,
        ) {
            self.clear_voices();
        }

        if apply_ratio(
            &mut self.params_state.ratio_b,
            &mut self.params_state.last_b_ratio,
            &mut self.params_state.last_b_model,
            block.b_model,
            block.b_ratio,
        ) {
            self.clear_voices();
        }

        // A change of the number of partials clears the voices.
        if self.params_state.last_a_partials != Some(block.a_partials) {
            self.clear_voices();
        }
        self.params_state.last_a_partials = Some(block.a_partials);

        if self.params_state.last_b_partials != Some(block.b_partials) {
            self.clear_voices();
        }
        self.params_state.last_b_partials = Some(block.b_partials);

        // The model tables are recomputed here, on the audio thread, before
        // the voices read them. The JUCE build does this on the message
        // thread, which is a data race.
        self.models
            .recalc(true, block.a_model, self.params_state.ratio_a);
        self.models
            .recalc(false, block.b_model, self.params_state.ratio_b);

        // A change of the mallet loads the new sample and clears the voices.
        if self.params_state.last_mallet_type != Some(block.mallet_type) {
            if (block.mallet_type as u8) > (MalletType::UserFile as u8) {
                // The sample is decoded here, on the audio thread. It is a
                // rare one-off allocation, and it removes the race the JUCE
                // build has between the message thread and the audio thread.
                self.sampler.load_internal_sample(block.mallet_type);
            }
            self.clear_voices();
        }
        self.params_state.last_mallet_type = Some(block.mallet_type);

        // The sampler's pitch.
        self.sampler.set_pitch(block.mallet_pitch);

        // The voices.
        let noise_settings = block.noise_settings();
        let resonator_a = block.resonator_a_settings();
        let resonator_b = block.resonator_b_settings();
        let bend = self.midi.bend.current;
        let polyphony = self.polyphony.min(self.voices.len());

        for voice in self.voices.iter_mut().take(polyphony) {
            voice.noise.init(self.srate, &noise_settings);
            voice.set_pitch(
                block.a_coarse,
                block.b_coarse,
                block.a_fine,
                block.b_fine,
                bend,
            );
            voice.set_ratio(self.params_state.ratio_a, self.params_state.ratio_b);
            voice.res_a.set_params(self.srate, &resonator_a);
            voice.res_b.set_params(self.srate, &resonator_b);
            voice.set_coupling(block.couple, block.ab_split);
            voice.update_resonators(&self.models);

            if (block.mallet_type as u8) >= (MalletType::UserFile as u8) {
                voice.mallet.set_filter(block.mallet_filter);
            }
        }

        self.params_state.applied = Some(*block);
    }
}

/// Applies a ratio, and tells whether the model changed.
///
/// The JUCE build writes the model's default ratio back to the host
/// parameter. The plugin cannot write a host parameter from the audio
/// thread, so the default is applied internally instead and the editor
/// mirrors it into the host parameter.
fn apply_ratio(
    current: &mut f64,
    last_parameter: &mut Option<f64>,
    last_model: &mut Option<ModalModel>,
    model: ModalModel,
    ratio: f64,
) -> bool {
    // The ratio parameter only wins when it has changed since the last
    // time, so that the model's default survives until the parameter is
    // moved.
    if last_parameter.is_none() || *last_parameter != Some(ratio) {
        *current = ratio;
    }

    // A change of the model resets the ratio to the model's default.
    let model_changed = match *last_model {
        Some(last) if last != model => {
            *current = default_ratio(model);
            true
        }
        _ => false,
    };

    *last_model = Some(model);
    *last_parameter = Some(ratio);

    model_changed
}

/// Picks the voice a note plays in.
///
/// Port of `RipplerXAudioProcessor::pickVoice`:
///
/// 1. a voice that already plays the note wins,
/// 2. a released voice beats a pressed one,
/// 3. among the released voices the one released longest ago wins,
/// 4. among the pressed voices the one pressed longest ago wins.
pub(crate) fn pick_voice(
    voices: &[Voice],
    polyphony: usize,
    reuse_voices: bool,
    note: i32,
) -> usize {
    if reuse_voices
        && let Some(index) = voices
            .iter()
            .take(polyphony)
            .position(|voice| voice.note == note)
    {
        return index;
    }

    let mut pick = 0;
    for index in 1..polyphony {
        let candidate = &voices[index];
        let current = &voices[pick];

        if !candidate.is_pressed && current.is_pressed {
            pick = index;
        } else if candidate.is_pressed && !current.is_pressed {
            // The current pick is released, which has priority.
        } else if !candidate.is_pressed && !current.is_pressed {
            if candidate.release_ts < current.release_ts {
                pick = index;
            }
        } else if candidate.pressed_ts < current.pressed_ts {
            pick = index;
        }
    }

    pick
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::tests::{note_off, note_on, run};
    use nice_plug_core::params::InternalParamMut;

    /// Sets a parameter's value, the way the host does.
    fn set<T: InternalParamMut>(param: &T, value: T::Plain) {
        unsafe {
            param._internal_set_plain_value(value);
        }
    }

    /// The voice that plays a note.
    fn voice_of(plugin: &RipplerX, note: u8) -> usize {
        (0..plugin.voices.len())
            .find(|index| {
                plugin.voices[*index].note == note as i32 && plugin.voices[*index].is_pressed
            })
            .expect("the note is not playing")
    }

    /// The number of voices that are held down.
    fn pressed_count(plugin: &RipplerX) -> usize {
        plugin
            .voices
            .iter()
            .filter(|voice| voice.is_pressed)
            .count()
    }

    #[test]
    fn the_oldest_voice_is_reused() {
        let mut plugin = crate::process::tests::plugin();

        // With every voice free, the first note takes the first voice.
        run(&mut plugin, 64, &[note_on(60, 0.8, 0)]);
        assert_eq!(voice_of(&plugin, 60), 0);

        // The next note takes the next free voice.
        run(&mut plugin, 64, &[note_on(64, 0.8, 0)]);
        assert_eq!(voice_of(&plugin, 64), 1);
    }

    #[test]
    fn a_released_voice_is_preferred_over_a_pressed_one() {
        let mut plugin = crate::process::tests::plugin();

        // Every voice is busy.
        for note in 60..68 {
            run(&mut plugin, 64, &[note_on(note, 0.8, 0)]);
        }
        assert_eq!(voice_of(&plugin, 60), 0);

        // One of the notes is released.
        run(&mut plugin, 64, &[note_off(60, 0)]);

        // The next note takes the released voice, because a released voice
        // comes before a pressed one.
        run(&mut plugin, 64, &[note_on(72, 0.8, 0)]);
        assert_eq!(voice_of(&plugin, 72), 0);
    }

    #[test]
    fn the_same_note_reuses_its_voice_when_reuse_voices_is_on() {
        let mut plugin = crate::process::tests::plugin();
        set(&plugin.params.reuse_voices, true);

        run(&mut plugin, 64, &[note_on(60, 0.8, 0)]);
        let first = voice_of(&plugin, 60);

        // The same note again takes the same voice, so only one voice is
        // held down.
        run(&mut plugin, 64, &[note_on(60, 0.8, 0)]);
        assert_eq!(voice_of(&plugin, 60), first);
        assert_eq!(pressed_count(&plugin), 1);
    }

    #[test]
    fn changing_the_model_resets_the_ratio() {
        let mut plugin = crate::process::tests::plugin();

        // The string model's ratio is the parameter's value.
        run(&mut plugin, 64, &[]);
        assert_eq!(plugin.voices[0].a_ratio, 1.0);

        // The beam model's ratio is two.
        set(&plugin.params.resonator_a.a_model, ModalModel::Beam);
        run(&mut plugin, 64, &[]);
        assert_eq!(plugin.voices[0].a_ratio, 2.0);

        // The djembe's ratio is one.
        set(&plugin.params.resonator_a.a_model, ModalModel::Djembe);
        run(&mut plugin, 64, &[]);
        assert_eq!(plugin.voices[0].a_ratio, 1.0);

        // Every other model's ratio is 0.78.
        set(&plugin.params.resonator_a.a_model, ModalModel::Membrane);
        run(&mut plugin, 64, &[]);
        assert_eq!(plugin.voices[0].a_ratio, 0.78);
    }

    #[test]
    fn changing_the_model_clears_the_voices() {
        let mut plugin = crate::process::tests::plugin();

        // A note is playing.
        run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        assert!(plugin.voices[0].mallet.countdown > 0);

        // The model changes, so the voices are cleared.
        set(&plugin.params.resonator_a.a_model, ModalModel::Beam);
        run(&mut plugin, 64, &[]);
        assert_eq!(plugin.voices[0].mallet.countdown, 0);
    }
}
