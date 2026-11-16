//! The per-sample processing loop.
//!
//! Port of `RipplerXAudioProcessor::processBlockByType` in
//! `src/PluginProcessor.cpp`.
//!
//! The JUCE build runs the loop once per sample, because the pitch bend
//! glides per sample. The same is done here.

use nice_plug::prelude::*;
use rippler_dsp::envelope::Stage;
use rippler_dsp::voice::Voice;
use rippler_params::params::RipplerParams;

use crate::RipplerX;
use crate::block_params::BlockParams;
use crate::params::pick_voice;

/// The control change number of the sustain pedal.
pub(crate) const SUSTAIN_PEDAL: u8 = 64;

/// The control change number of "all sound off".
pub(crate) const ALL_SOUND_OFF: u8 = 120;

/// The control change number of "all notes off".
pub(crate) const ALL_NOTES_OFF: u8 = 123;

/// The number of MIDI messages the queue holds.
///
/// The queue is allocated once, so that the audio thread never allocates.
pub(crate) const MIDI_QUEUE_CAPACITY: usize = 1024;

/// The kind of a queued MIDI message.
///
/// The same list as the C++'s `MIDIMsgType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MidiMsgType {
    /// A note was pressed.
    NoteOn,
    /// A note was released.
    NoteOff,
    /// The sustain pedal was pressed.
    SustainPedalOn,
    /// The sustain pedal was released.
    SustainPedalOff,
    /// The pitch wheel moved.
    PitchWheel,
}

/// A queued MIDI message.
///
/// Port of the C++'s `MIDIMsg`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MidiMsg {
    /// The sample the message happens at.
    pub offset: i32,
    /// What kind of message it is.
    pub kind: MidiMsgType,
    /// The note the message is about.
    pub note: i32,
    /// The velocity of a note, or the pitch wheel's position.
    pub vel: f64,
}

/// The state of the pitch bend.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bend {
    /// The bend per sample.
    pub step: f64,
    /// The bend the glide starts at.
    pub start: f64,
    /// The bend the glide is at.
    pub current: f64,
    /// The bend the glide ends at.
    pub target: f64,
    /// The number of samples the glide has left.
    pub remaining: i64,
    /// The number of samples a glide takes.
    pub total: i64,
}

impl Default for Bend {
    fn default() -> Self {
        Self {
            step: 0.1,
            start: 1.0,
            current: 1.0,
            target: 1.0,
            remaining: -1,
            total: 0,
        }
    }
}

/// The state of the MIDI input.
#[derive(Default)]
pub(crate) struct MidiState {
    /// The queued messages, each with the sample offset it happens at.
    pub queue: Vec<MidiMsg>,
    /// Whether the sustain pedal is down.
    pub sustain_pedal: bool,
    /// The notes the sustain pedal holds.
    pub sustain_pedal_notes: Vec<MidiMsg>,
    /// The number of notes that were pressed.
    pub press_count: u64,
    /// The number of notes that were released.
    pub release_count: u64,
    /// The pitch bend's state.
    pub bend: Bend,
}

impl RipplerX {
    /// Queues a note event for the block that is about to be processed.
    ///
    /// The JUCE build keeps a queue of MIDI messages with a per-sample
    /// offset, because JUCE hands it the MIDI at block granularity. nice-plug
    /// hands the events over with their own sample offsets, so the same queue
    /// is used here.
    pub(crate) fn queue_event(&mut self, event: NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn {
                timing,
                key,
                velocity,
                ..
            } => {
                // JUCE's `juce::MidiMessage::isNoteOff` is true for a note
                // on with a velocity of zero, so the C++ releases the note
                // rather than pressing it. nice-plug does not normalise the
                // velocity, so the same is done here: a note on without a
                // velocity is a note off.
                let (kind, vel) = if velocity <= 0.0 {
                    (MidiMsgType::NoteOff, 0.0)
                } else {
                    (MidiMsgType::NoteOn, velocity as f64)
                };

                self.push_midi(MidiMsg {
                    offset: timing as i32,
                    kind,
                    note: key.number_or_middle_c() as i32,
                    vel,
                });
            }
            NoteEvent::NoteOff { timing, key, .. } => {
                self.push_midi(MidiMsg {
                    offset: timing as i32,
                    kind: MidiMsgType::NoteOff,
                    note: key.number_or_middle_c() as i32,
                    vel: 0.0,
                });
            }
            NoteEvent::MidiCC {
                timing, cc, value, ..
            } => match cc {
                SUSTAIN_PEDAL => {
                    let kind = if value >= 0.5 {
                        MidiMsgType::SustainPedalOn
                    } else {
                        MidiMsgType::SustainPedalOff
                    };
                    self.push_midi(MidiMsg {
                        offset: timing as i32,
                        kind,
                        note: 0,
                        vel: 0.0,
                    });
                }
                // The JUCE build clears the voices when it sees either of
                // these, without waiting for the sample they happen at.
                ALL_SOUND_OFF | ALL_NOTES_OFF => self.clear_voices(),
                _ => {}
            },
            NoteEvent::MidiPitchBend { timing, value, .. } => {
                // The JUCE build's pitch wheel value runs from zero to
                // sixteen thousand three hundred and eighty three, with
                // eight thousand one hundred and ninety two in the middle.
                self.push_midi(MidiMsg {
                    offset: timing as i32,
                    kind: MidiMsgType::PitchWheel,
                    note: 0,
                    vel: (value as f64 * 16383.0).round(),
                });
            }
            _ => {}
        }
    }

    /// Adds a message to the MIDI queue.
    ///
    /// The queue never grows beyond its capacity, so that the audio thread
    /// never allocates.
    fn push_midi(&mut self, msg: MidiMsg) {
        if self.midi.queue.len() < MIDI_QUEUE_CAPACITY {
            self.midi.queue.push(msg);
        }
    }

    /// Silences every voice.
    pub(crate) fn clear_voices(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.clear();
        }
    }

    /// Processes one block of audio.
    ///
    /// The events of the block have to be queued with
    /// [`RipplerX::queue_event`] before this is called.
    pub(crate) fn process_block(&mut self, buffer: &mut Buffer) {
        let num_samples = buffer.samples();
        let block = BlockParams::read(&self.params, self.noise_mix_range, self.noise_res_range);

        // The parameters are applied to the voices, the way the JUCE build's
        // `onSlider` does. This happens before the MIDI is handled, so a
        // note that starts in this block already uses the new parameters.
        self.apply_parameters(&block);

        let Self {
            voices,
            sampler,
            models,
            luts,
            comb,
            limiter,
            rngs,
            params,
            midi,
            srate,
            polyphony,
            noise_mix_range,
            noise_res_range,
            tuning,
            ..
        } = self;

        // The lookup tables are built when the plugin is activated. A plugin
        // that was never activated has none, and there is nothing it can do
        // with the block, so it leaves the buffer alone.
        let Some(luts) = luts.as_ref() else {
            return;
        };

        let srate = *srate;
        let polyphony = (*polyphony).min(voices.len()).min(rngs.len());

        // The messages that were handled in an earlier block are dropped.
        midi.queue.retain(|msg| msg.offset >= 0);

        // The input is read from the same buffer the output is written to, so
        // the input of a sample is read before the output of that sample is
        // written.
        //
        // nice-plug exposes the main input only through the output buffer,
        // so this is the only place the input can come from. A host that
        // connects no input bus leaves whatever it had in the buffer there,
        // and the plugin sums that, where the C++ sums zero channels.
        let channels = buffer.as_slice();
        let num_channels = channels.len();

        for sample in 0..num_samples {
            // The pitch bend glides one sample towards its target.
            if midi.bend.remaining > 0 {
                midi.bend.current += midi.bend.step;
                midi.bend.remaining -= 1;

                if midi.bend.remaining == 0 {
                    midi.bend.current = midi.bend.target;
                }
            }

            // The MIDI messages that happen at this sample are handled.
            for index in 0..midi.queue.len() {
                if midi.queue[index].offset == 0 {
                    let msg = midi.queue[index];
                    handle_message(
                        msg,
                        voices,
                        models,
                        sampler,
                        midi,
                        params,
                        srate,
                        block.bend_range,
                        polyphony,
                        tuning.as_ref(),
                    );
                }

                midi.queue[index].offset -= 1;
            }

            // The input is summed into a mono signal.
            let mut audio_in = 0.0;
            let mut input_channels = 0;
            for channel in channels.iter().take(num_channels) {
                audio_in += channel[sample] as f64;
                input_channels += 1;
            }
            if input_channels > 0 {
                audio_in /= input_channels as f64;
            }

            let mut dir_out = 0.0;
            let mut a_out = 0.0;
            let mut b_out = 0.0;

            for (index, voice) in voices.iter_mut().enumerate().take(polyphony) {
                let mut res_out = 0.0;

                if midi.bend.remaining >= 0 {
                    voice.apply_pitch_bend(midi.bend.current, luts);
                    if midi.bend.remaining == 0 {
                        midi.bend.remaining = -1;
                    }
                }

                // The fade out is used to declick a repeated note.
                let voice_fade_out = if voice.is_fading {
                    voice.fade_out(models, sampler)
                } else {
                    1.0
                };

                // The mallet.
                let m_sample = voice.mallet.process(sampler);
                if m_sample != 0.0 {
                    dir_out += m_sample
                        * (block.mallet_mix + block.vel_mallet_mix * voice.vel).clamp(0.0, 1.0)
                        * voice_fade_out;
                    res_out += m_sample
                        * (block.mallet_res + block.vel_mallet_res * voice.vel).clamp(0.0, 1.0);
                }

                // The audio input.
                if audio_in != 0.0 && voice.is_pressed {
                    res_out += audio_in;
                }

                // The noise.
                let noise = voice.noise.process(&mut rngs[index]);
                if voice.noise.env.state != Stage::Off {
                    let osc = if block.noise_osc > 0.0
                        && (block.noise_res > 0.0 || block.vel_noise_res > 0.0)
                    {
                        let b = voice.process_oscillators(false, luts);
                        let a = voice.process_oscillators(true, luts);
                        voice.noise.process_osc(a + b) * block.noise_osc
                    } else {
                        0.0
                    };

                    let mix =
                        (block.noise_mix + block.vel_noise_mix * voice.vel).clamp(0.0, 1.0) as f32;
                    dir_out += noise * noise_mix_range.unnormalize(mix) as f64 * voice_fade_out;

                    let res =
                        (block.noise_res + block.vel_noise_res * voice.vel).clamp(0.0, 1.0) as f32;
                    res_out += (noise * (1.0 - block.noise_osc) + osc)
                        * noise_res_range.unnormalize(res) as f64;
                }

                // The first resonator.
                let mut out_from_a = 0.0;
                if block.a_on {
                    let mut out = voice.res_a.process(res_out);
                    if voice.res_a.cut != 0.0 {
                        out = voice.res_a.filter.process(out);
                    }
                    a_out += out * voice_fade_out;
                    out_from_a = out;
                }

                // The second resonator.
                if block.b_on {
                    let input = if block.a_on && block.couple {
                        out_from_a
                    } else {
                        res_out
                    };
                    let mut out = voice.res_b.process(input);
                    if voice.res_b.cut != 0.0 {
                        out = voice.res_b.filter.process(out);
                    }
                    b_out += out * voice_fade_out;
                }
            }

            // The two resonators are mixed.
            let res_out = if block.a_on && block.b_on {
                if block.couple {
                    b_out
                } else {
                    a_out * (1.0 - block.ab_mix) + b_out * block.ab_mix
                }
            } else {
                a_out + b_out
            };

            let total_out = dir_out + res_out * block.gain;

            // The stereoizer and the limiter.
            let (spl0, spl1) = if block.stereoizer {
                comb.process(total_out)
            } else {
                (total_out, total_out)
            };
            let (left, right) = limiter.process(spl0, spl1);

            // The output is written into the buffer the input came from.
            if num_channels > 0 {
                channels[0][sample] = left as f32;
            }
            for channel in channels
                .iter_mut()
                .skip(1)
                .take(num_channels.saturating_sub(1))
            {
                channel[sample] = right as f32;
            }
        }
    }
}

/// Handles a queued MIDI message.
#[allow(clippy::too_many_arguments)]
fn handle_message(
    msg: MidiMsg,
    voices: &mut [Voice],
    models: &rippler_dsp::models::ModelTables,
    sampler: &rippler_dsp::sampler::Sampler,
    midi: &mut MidiState,
    params: &RipplerParams,
    srate: f64,
    bend_range: f64,
    polyphony: usize,
    tuning: Option<&mts_client_rs::Client>,
) {
    let MidiState {
        sustain_pedal,
        sustain_pedal_notes,
        press_count,
        release_count,
        bend,
        ..
    } = midi;

    match msg.kind {
        MidiMsgType::NoteOn => {
            on_note(
                msg,
                voices,
                models,
                sampler,
                press_count,
                params,
                srate,
                polyphony,
                tuning,
            );

            // A note that is pressed again is not held by the sustain pedal.
            // This is the fix for a note that is pressed twice and held: it
            // should not be released with the pedal.
            sustain_pedal_notes.retain(|held| held.note != msg.note);
        }
        MidiMsgType::NoteOff => {
            if !*sustain_pedal {
                off_note(msg, voices, models, release_count, polyphony);
            } else if sustain_pedal_notes.len() < MIDI_QUEUE_CAPACITY {
                // The list never grows beyond its capacity, so that the
                // audio thread never allocates.
                sustain_pedal_notes.push(msg);
            }
        }
        MidiMsgType::SustainPedalOn => *sustain_pedal = true,
        MidiMsgType::SustainPedalOff => {
            *sustain_pedal = false;
            for held in sustain_pedal_notes.iter() {
                off_note(*held, voices, models, release_count, polyphony);
            }
            sustain_pedal_notes.clear();
        }
        MidiMsgType::PitchWheel => set_bend_target(bend, msg.vel, bend_range),
    }
}

/// Starts a note.
///
/// Port of `RipplerXAudioProcessor::onNote`.
#[allow(clippy::too_many_arguments)]
fn on_note(
    msg: MidiMsg,
    voices: &mut [Voice],
    models: &rippler_dsp::models::ModelTables,
    sampler: &rippler_dsp::sampler::Sampler,
    press_count: &mut u64,
    params: &RipplerParams,
    srate: f64,
    polyphony: usize,
    tuning: Option<&mts_client_rs::Client>,
) {
    let reuse_voices = params.reuse_voices.value();
    let nvoice = pick_voice(voices, polyphony, reuse_voices, msg.note);
    let voice = &mut voices[nvoice];

    let fadeout_repeats = params.fadeout_repeats.value();
    let skip_fadeout = reuse_voices && !fadeout_repeats && voice.note == msg.note;

    let mallet_type = params.mallet.mallet_type.value();
    let mallet_stiff = params.mallet.mallet_stiff.value() as f64;
    let mallet_ktrack = params.mallet.mallet_ktrack.value() as f64;
    let vel_mallet_stiff = params.velocity.vel_mallet_stiff.value() as f64;

    let vel = msg.vel;
    let mallet_freq = (mallet_stiff.ln()
        + vel * vel_mallet_stiff * 2.0 * (5000.0f64.ln() - 100.0f64.ln()))
    .exp()
    .clamp(100.0, 5000.0);

    // The JUCE build asks MTS-ESP for the frequency and falls back to the
    // equal temperament formula.
    let freq = crate::tuning::note_to_frequency(msg.note, tuning);

    *press_count += 1;
    voice.trigger(
        *press_count,
        srate,
        msg.note,
        vel,
        mallet_type,
        mallet_freq,
        mallet_ktrack,
        skip_fadeout,
        freq,
        models,
        sampler,
    );
}

/// Releases a note.
///
/// Port of `RipplerXAudioProcessor::offNote`.
fn off_note(
    msg: MidiMsg,
    voices: &mut [Voice],
    models: &rippler_dsp::models::ModelTables,
    release_count: &mut u64,
    polyphony: usize,
) {
    for voice in voices.iter_mut().take(polyphony) {
        if voice.note == msg.note && !voice.is_release {
            *release_count += 1;
            voice.release(*release_count, models);
        }
    }
}

/// Sets the pitch bend's target.
///
/// The bend glides to the target over the glide time.
fn set_bend_target(bend: &mut Bend, pitch_wheel: f64, bend_range: f64) {
    let normalized = (pitch_wheel - 8192.0) / 8191.0;
    bend.start = bend.current;
    bend.target = 2.0f64.powf(normalized * bend_range / 12.0);
    bend.remaining = bend.total;

    // The glide time is the number of samples the bend takes, so the step
    // is the difference between the two bends over that time.
    bend.step = if bend.total > 0 {
        (bend.target - bend.start) / bend.total as f64
    } else {
        0.0
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        control_change, new_plugin, note_off, note_on, peak, pitch_wheel, run, run_with_input, set,
        settle, short_release_plugin, sustain,
    };
    use rippler_params::enums::Coupling;

    #[test]
    fn a_note_starts_a_voice() {
        let mut plugin = new_plugin();

        let (left, right) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);

        assert!(peak(&left) > 0.0, "the note did not make a sound");
        assert!(peak(&right) > 0.0, "the note did not make a sound");
    }

    #[test]
    fn a_note_off_stops_the_voice() {
        let mut plugin = short_release_plugin();

        // The note sounds.
        let (left, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        assert!(peak(&left) > 0.0, "the note did not make a sound");

        // The note is released, and the voice falls silent.
        run(&mut plugin, 64, &[note_off(60, 0)]);
        run(&mut plugin, 4096, &[]);
        settle(&mut plugin);
        let (left, _) = run(&mut plugin, 1024, &[]);
        assert!(
            peak(&left) < 1e-6,
            "the voice is still sounding: {}",
            peak(&left)
        );
    }

    #[test]
    fn a_note_on_without_a_velocity_releases_the_note() {
        let mut plugin = short_release_plugin();

        // The note sounds.
        run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        assert!(plugin.voices[0].is_pressed, "the note is not playing");
        assert_eq!(plugin.midi.press_count, 1);

        // A note on with a velocity of zero is a note off, the way JUCE
        // reads it, so the note is released rather than pressed again.
        run(&mut plugin, 64, &[note_on(60, 0.0, 0)]);
        assert!(
            !plugin.voices[0].is_pressed,
            "the note was pressed again instead of released"
        );
        assert_eq!(
            plugin.midi.press_count, 1,
            "the note was pressed again instead of released"
        );
    }

    #[test]
    fn the_sustain_pedal_holds_a_released_note() {
        let mut plugin = short_release_plugin();

        // The pedal is down and the note is played and released.
        let (left, _) = run(
            &mut plugin,
            1024,
            &[sustain(true, 0), note_on(60, 0.8, 0), note_off(60, 1)],
        );
        assert!(peak(&left) > 0.0, "the note did not make a sound");

        // The pedal holds the note, so it keeps sounding.
        let (left, _) = run(&mut plugin, 4096, &[]);
        assert!(
            peak(&left) > 1e-4,
            "the pedal did not hold the note: {}",
            peak(&left)
        );

        // The pedal comes up, so the note is released and falls silent.
        run(&mut plugin, 64, &[sustain(false, 0)]);
        run(&mut plugin, 4096, &[]);
        settle(&mut plugin);
        let (left, _) = run(&mut plugin, 1024, &[]);
        assert!(
            peak(&left) < 1e-6,
            "the voice is still sounding: {}",
            peak(&left)
        );
    }

    #[test]
    fn pressing_a_note_twice_while_the_pedal_is_down_does_not_release_it() {
        let mut plugin = short_release_plugin();

        // The pedal is down, the note is played and released, and then it is
        // played again.
        run(
            &mut plugin,
            1024,
            &[
                sustain(true, 0),
                note_on(60, 0.8, 0),
                note_off(60, 1),
                note_on(60, 0.8, 2),
            ],
        );

        // The pedal comes up. The note was pressed again, so it is not
        // released and keeps sounding.
        run(&mut plugin, 64, &[sustain(false, 0)]);
        let (left, _) = run(&mut plugin, 4096, &[]);
        assert!(
            peak(&left) > 1e-4,
            "the note was released with the pedal: {}",
            peak(&left)
        );
    }

    #[test]
    fn all_notes_off_clears_the_voices() {
        let mut plugin = new_plugin();

        let (left, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        assert!(peak(&left) > 0.0, "the note did not make a sound");

        // The voices are cleared, so the plugin falls silent once the
        // stereoizer's delay line has been flushed.
        run(&mut plugin, 1024, &[control_change(ALL_NOTES_OFF, 0.0, 0)]);
        settle(&mut plugin);
        let (left, right) = run(&mut plugin, 1024, &[]);
        assert_eq!(peak(&left), 0.0, "the voice is still sounding");
        assert_eq!(peak(&right), 0.0, "the voice is still sounding");
    }

    #[test]
    fn the_pitch_wheel_glides_rather_than_jumping() {
        let mut plugin = new_plugin();

        // The pitch wheel is bent all the way up.
        run(&mut plugin, 1, &[pitch_wheel(16383, 0)]);

        let target = plugin.midi.bend.target;
        assert!(target > 1.0, "the bend did not move");
        assert!(
            plugin.midi.bend.current < target,
            "the bend jumped to its target"
        );

        // The bend arrives after the glide time.
        run(&mut plugin, 1000, &[]);
        assert_eq!(plugin.midi.bend.current, target);
    }

    #[test]
    fn the_output_is_stereo() {
        let mut plugin = new_plugin();

        let (left, right) = run(&mut plugin, 4096, &[note_on(60, 0.8, 0)]);

        assert!(
            left.iter().zip(right.iter()).any(|(l, r)| l != r),
            "the stereoizer did not spread the sound"
        );
    }

    #[test]
    fn the_audio_input_excites_the_resonators() {
        // The note sounds, and the input is silent.
        let mut plugin = new_plugin();
        run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        let (without_input, _) = run(&mut plugin, 1024, &[]);

        // The same note, with a signal on the input.
        let mut plugin = new_plugin();
        run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        let input = vec![0.5f32; 1024];
        let (with_input, _) = run_with_input(&mut plugin, 1024, &input, &[]);

        assert!(
            with_input
                .iter()
                .zip(without_input.iter())
                .any(|(with, without)| with != without),
            "the input did not reach the resonators"
        );
        assert!(
            peak(&with_input) > peak(&without_input),
            "the input did not make the resonators louder"
        );
    }

    #[test]
    fn the_audio_input_only_reaches_a_pressed_voice() {
        // The C++ only adds the input to the resonators of the voices that
        // are pressed, so an input without a note makes no sound at all.
        let mut plugin = new_plugin();
        let input = vec![0.5f32; 1024];
        let (left, right) = run_with_input(&mut plugin, 1024, &input, &[]);

        assert_eq!(peak(&left), 0.0, "the input made a sound");
        assert_eq!(peak(&right), 0.0, "the input made a sound");
    }

    #[test]
    fn the_mix_follows_the_mix_parameter() {
        // With the mix at zero the output is the first resonator's, and
        // with the mix at one it is the second's.
        let mut plugin = new_plugin();
        set(&plugin.params.resonator_b.b_on, true);
        set(&plugin.params.ab_mix, 0.0f32);
        let (all_a, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);

        let mut plugin = new_plugin();
        set(&plugin.params.resonator_b.b_on, true);
        set(&plugin.params.ab_mix, 1.0f32);
        let (all_b, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);

        // The first resonator on its own gives the same output as the mix
        // at zero.
        let mut plugin = new_plugin();
        set(&plugin.params.resonator_b.b_on, false);
        let (only_a, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        assert_eq!(all_a, only_a, "the mix at zero is not the first resonator");

        // The second resonator on its own gives the same output as the mix
        // at one.
        let mut plugin = new_plugin();
        set(&plugin.params.resonator_a.a_on, false);
        set(&plugin.params.resonator_b.b_on, true);
        let (only_b, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        assert_eq!(all_b, only_b, "the mix at one is not the second resonator");
    }

    #[test]
    fn the_serial_coupling_ignores_the_mix_and_uses_the_split() {
        // With the resonators coupled in series the output is the second
        // resonator's, whatever the mix says.
        let mut plugin = new_plugin();
        set(&plugin.params.resonator_b.b_on, true);
        set(&plugin.params.couple, Coupling::Series);
        set(&plugin.params.ab_mix, 0.0f32);
        let (mix_zero, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);

        let mut plugin = new_plugin();
        set(&plugin.params.resonator_b.b_on, true);
        set(&plugin.params.couple, Coupling::Series);
        set(&plugin.params.ab_mix, 1.0f32);
        let (mix_one, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);

        assert_eq!(
            mix_zero, mix_one,
            "the mix changed the output of the serial coupling"
        );

        // The split is the strength of the coupling, and it changes the
        // sound the two resonators make together.
        let mut plugin = new_plugin();
        set(&plugin.params.resonator_b.b_on, true);
        set(&plugin.params.couple, Coupling::Series);
        set(&plugin.params.ab_split, 0.5f32);
        let (split, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);

        assert!(
            split != mix_zero,
            "the split did not change the sound of the coupling"
        );
    }

    #[test]
    fn the_limiter_bounds_a_loud_signal() {
        let mut plugin = new_plugin();

        // A note, and a loud signal on the input.
        run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        let input = vec![4.0f32; 8192];
        let (left, _) = run_with_input(&mut plugin, 8192, &input, &[]);

        assert!(peak(&left) > 0.0, "the signal made no sound at all");
        assert!(
            left.iter().all(|sample| sample.is_finite()),
            "the signal is not finite"
        );

        // The limiter lets the first samples through, but it bounds the
        // signal, which came in at four.
        assert!(
            peak(&left) < 2.0,
            "the limiter did not bound the signal: {}",
            peak(&left)
        );

        let settled = &left[1024..];
        assert!(
            peak(settled) < 1.5,
            "the limiter did not bound the signal: {}",
            peak(settled)
        );
    }

    #[test]
    fn a_control_change_clears_the_voices() {
        let mut plugin = new_plugin();

        let (left, _) = run(&mut plugin, 1024, &[note_on(60, 0.8, 0)]);
        assert!(peak(&left) > 0.0, "the note did not make a sound");

        // The "all sound off" control change clears the voices.
        run(&mut plugin, 1024, &[control_change(ALL_SOUND_OFF, 0.0, 0)]);
        settle(&mut plugin);
        let (left, _) = run(&mut plugin, 1024, &[]);
        assert!(peak(&left) < 1e-6, "the voice is still sounding");
    }
}
