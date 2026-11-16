//! The per-sample processing loop.
//!
//! Port of `RipplerXAudioProcessor::processBlockByType` in
//! `src/PluginProcessor.cpp`.
//!
//! The JUCE build runs the loop once per sample, because the pitch bend
//! glides per sample. The same is done here.

use nice_plug::prelude::*;
use rippler_dsp::envelope::Stage;
use rippler_dsp::mallet::MalletType;
use rippler_dsp::models::ModalModel;
use rippler_dsp::noise::NoiseSettings;
use rippler_dsp::resonator::ResonatorSettings;
use rippler_dsp::voice::Voice;
use rippler_params::enums::{Coupling, NoiseFilterMode, PartialCount};
use rippler_params::params::RipplerParams;

use crate::RipplerX;
use crate::params::pick_voice;

/// The control change number of the sustain pedal.
const SUSTAIN_PEDAL: u8 = 64;

/// The control change number of "all sound off".
const ALL_SOUND_OFF: u8 = 120;

/// The control change number of "all notes off".
const ALL_NOTES_OFF: u8 = 123;

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

/// The parameters of one block of audio.
///
/// The JUCE build reads every parameter once per block into locals and then
/// works from those. This is the same thing: the values are plain, so the
/// loop never has to look at the parameter struct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BlockParams {
    /// The kind of mallet that excites the resonators.
    pub mallet_type: MalletType,
    /// The mallet's pitch, in semitones.
    pub mallet_pitch: f64,
    /// The mallet's filter.
    pub mallet_filter: f64,
    /// The mallet's stiffness.
    pub mallet_stiff: f64,
    /// The mallet's key tracking.
    pub mallet_ktrack: f64,

    /// The noise's offset voltage.
    pub noise_osc: f64,
    /// The noise's mix, normalised.
    pub noise_mix: f64,
    /// The noise's resonance, normalised.
    pub noise_res: f64,
    /// The filter the noise passes through.
    pub noise_filter_mode: NoiseFilterMode,
    /// The filter's cutoff frequency.
    pub noise_filter_freq: f64,
    /// The filter's quality factor.
    pub noise_filter_q: f64,
    /// The noise envelope's attack time.
    pub noise_att: f64,
    /// The noise envelope's decay time.
    pub noise_dec: f64,
    /// The noise envelope's sustain level.
    pub noise_sus: f64,
    /// The noise envelope's release time.
    pub noise_rel: f64,
    /// How much the velocity tensions the attack.
    pub noise_att_ten: f64,
    /// How much the velocity tensions the decay.
    pub noise_dec_ten: f64,
    /// How much the velocity tensions the release.
    pub noise_rel_ten: f64,

    /// Whether the first resonator sounds.
    pub a_on: bool,
    /// The first resonator's model.
    pub a_model: ModalModel,
    /// The first resonator's number of partials.
    pub a_partials: usize,
    /// The first resonator's decay.
    pub a_decay: f64,
    /// The first resonator's damping.
    pub a_damp: f64,
    /// The first resonator's tone.
    pub a_tone: f64,
    /// The first resonator's strike position.
    pub a_hit: f64,
    /// The first resonator's release.
    pub a_rel: f64,
    /// The first resonator's inharmonicity.
    pub a_inharm: f64,
    /// The first resonator's model ratio.
    pub a_ratio: f64,
    /// The first resonator's cutoff.
    pub a_cut: f64,
    /// The first resonator's tube radius.
    pub a_radius: f64,
    /// The first resonator's coarse pitch.
    pub a_coarse: f64,
    /// The first resonator's fine pitch.
    pub a_fine: f64,

    /// Whether the second resonator sounds.
    pub b_on: bool,
    /// The second resonator's model.
    pub b_model: ModalModel,
    /// The second resonator's number of partials.
    pub b_partials: usize,
    /// The second resonator's decay.
    pub b_decay: f64,
    /// The second resonator's damping.
    pub b_damp: f64,
    /// The second resonator's tone.
    pub b_tone: f64,
    /// The second resonator's strike position.
    pub b_hit: f64,
    /// The second resonator's release.
    pub b_rel: f64,
    /// The second resonator's inharmonicity.
    pub b_inharm: f64,
    /// The second resonator's model ratio.
    pub b_ratio: f64,
    /// The second resonator's cutoff.
    pub b_cut: f64,
    /// The second resonator's tube radius.
    pub b_radius: f64,
    /// The second resonator's coarse pitch.
    pub b_coarse: f64,
    /// The second resonator's fine pitch.
    pub b_fine: f64,

    /// How much the velocity bends the mallet's mix.
    pub vel_mallet_mix: f64,
    /// How much the velocity bends the mallet's resonance.
    pub vel_mallet_res: f64,
    /// How much the velocity bends the mallet's stiffness.
    pub vel_mallet_stiff: f64,
    /// How much the velocity bends the noise's mix.
    pub vel_noise_mix: f64,
    /// How much the velocity bends the noise's resonance.
    pub vel_noise_res: f64,
    /// How much the velocity bends the noise's frequency.
    pub vel_noise_freq: f64,
    /// How much the velocity bends the noise's attack.
    pub vel_noise_att: f64,
    /// How much the velocity bends the noise's decay.
    pub vel_noise_dec: f64,
    /// How much the velocity bends the noise's sustain.
    pub vel_noise_sus: f64,
    /// How much the velocity bends the noise's release.
    pub vel_noise_rel: f64,
    /// How much the velocity bends the noise's quality factor.
    pub vel_noise_q: f64,
    /// How much the velocity bends the first resonator's decay.
    pub vel_a_decay: f64,
    /// How much the velocity bends the first resonator's strike position.
    pub vel_a_hit: f64,
    /// How much the velocity bends the first resonator's inharmonicity.
    pub vel_a_inharm: f64,
    /// How much the velocity bends the first resonator's damping.
    pub vel_a_damp: f64,
    /// How much the velocity bends the first resonator's tone.
    pub vel_a_tone: f64,
    /// How much the velocity bends the second resonator's decay.
    pub vel_b_decay: f64,
    /// How much the velocity bends the second resonator's strike position.
    pub vel_b_hit: f64,
    /// How much the velocity bends the second resonator's inharmonicity.
    pub vel_b_inharm: f64,
    /// How much the velocity bends the second resonator's damping.
    pub vel_b_damp: f64,
    /// How much the velocity bends the second resonator's tone.
    pub vel_b_tone: f64,

    /// Whether the resonators are coupled in series.
    pub couple: bool,
    /// How much of each resonator reaches the output.
    pub ab_mix: f64,
    /// Where the second resonator is fed from in a serial coupling.
    pub ab_split: f64,
    /// The output's gain, as a factor.
    pub gain: f64,
    /// The pitch bend's range, in semitones.
    pub bend_range: f64,
    /// Whether the output is spread over the stereo field.
    pub stereoizer: bool,
    /// Whether a voice is reused for the next note.
    pub reuse_voices: bool,
    /// Whether a repeated note fades the previous one out.
    pub fadeout_repeats: bool,
}

impl BlockParams {
    /// Reads the parameters.
    pub(crate) fn read(params: &RipplerParams) -> Self {
        let noise_mix_range = params.noise.noise_mix.range();
        let noise_res_range = params.noise.noise_res.range();

        let couple = params.couple.value() == Coupling::Series;

        Self {
            mallet_type: params.mallet.mallet_type.value(),
            mallet_pitch: params.mallet.mallet_pitch.value() as f64,
            mallet_filter: params.mallet.mallet_filter.value() as f64,
            mallet_stiff: params.mallet.mallet_stiff.value() as f64,
            mallet_ktrack: params.mallet.mallet_ktrack.value() as f64,

            noise_osc: params.noise.noise_osc.value() as f64,
            // The JUCE build normalises the noise's mix and resonance once
            // per block, and unnormalises them again for every voice.
            noise_mix: noise_mix_range.normalize(params.noise.noise_mix.value()) as f64,
            noise_res: noise_res_range.normalize(params.noise.noise_res.value()) as f64,
            noise_filter_mode: params.noise.noise_filter_mode.value(),
            noise_filter_freq: params.noise.noise_filter_freq.value() as f64,
            noise_filter_q: params.noise.noise_filter_q.value() as f64,
            noise_att: params.noise.noise_att.value() as f64,
            noise_dec: params.noise.noise_dec.value() as f64,
            noise_sus: params.noise.noise_sus.value() as f64,
            noise_rel: params.noise.noise_rel.value() as f64,
            noise_att_ten: params.noise.noise_att_ten.value() as f64,
            noise_dec_ten: params.noise.noise_dec_ten.value() as f64,
            noise_rel_ten: params.noise.noise_rel_ten.value() as f64,

            a_on: params.resonator_a.a_on.value(),
            a_model: params.resonator_a.a_model.value(),
            a_partials: partial_count(params.resonator_a.a_partials.value()),
            a_decay: params.resonator_a.a_decay.value() as f64,
            a_damp: params.resonator_a.a_damp.value() as f64,
            a_tone: params.resonator_a.a_tone.value() as f64,
            a_hit: params.resonator_a.a_hit.value() as f64,
            a_rel: params.resonator_a.a_rel.value() as f64,
            a_inharm: params.resonator_a.a_inharm.value() as f64,
            a_ratio: params.resonator_a.a_ratio.value() as f64,
            a_cut: params.resonator_a.a_cut.value() as f64,
            a_radius: params.resonator_a.a_radius.value() as f64,
            a_coarse: params.resonator_a.a_coarse.value() as f64,
            a_fine: params.resonator_a.a_fine.value() as f64,

            b_on: params.resonator_b.b_on.value(),
            b_model: params.resonator_b.b_model.value(),
            b_partials: partial_count(params.resonator_b.b_partials.value()),
            b_decay: params.resonator_b.b_decay.value() as f64,
            b_damp: params.resonator_b.b_damp.value() as f64,
            b_tone: params.resonator_b.b_tone.value() as f64,
            b_hit: params.resonator_b.b_hit.value() as f64,
            b_rel: params.resonator_b.b_rel.value() as f64,
            b_inharm: params.resonator_b.b_inharm.value() as f64,
            b_ratio: params.resonator_b.b_ratio.value() as f64,
            b_cut: params.resonator_b.b_cut.value() as f64,
            b_radius: params.resonator_b.b_radius.value() as f64,
            b_coarse: params.resonator_b.b_coarse.value() as f64,
            b_fine: params.resonator_b.b_fine.value() as f64,

            vel_mallet_mix: params.velocity.vel_mallet_mix.value() as f64,
            vel_mallet_res: params.velocity.vel_mallet_res.value() as f64,
            vel_mallet_stiff: params.velocity.vel_mallet_stiff.value() as f64,
            vel_noise_mix: params.velocity.vel_noise_mix.value() as f64,
            vel_noise_res: params.velocity.vel_noise_res.value() as f64,
            vel_noise_freq: params.velocity.vel_noise_freq.value() as f64,
            vel_noise_att: params.velocity.vel_noise_att.value() as f64,
            vel_noise_dec: params.velocity.vel_noise_dec.value() as f64,
            vel_noise_sus: params.velocity.vel_noise_sus.value() as f64,
            vel_noise_rel: params.velocity.vel_noise_rel.value() as f64,
            vel_noise_q: params.velocity.vel_noise_q.value() as f64,
            vel_a_decay: params.velocity.vel_a_decay.value() as f64,
            vel_a_hit: params.velocity.vel_a_hit.value() as f64,
            vel_a_inharm: params.velocity.vel_a_inharm.value() as f64,
            vel_a_damp: params.velocity.vel_a_damp.value() as f64,
            vel_a_tone: params.velocity.vel_a_tone.value() as f64,
            vel_b_decay: params.velocity.vel_b_decay.value() as f64,
            vel_b_hit: params.velocity.vel_b_hit.value() as f64,
            vel_b_inharm: params.velocity.vel_b_inharm.value() as f64,
            vel_b_damp: params.velocity.vel_b_damp.value() as f64,
            vel_b_tone: params.velocity.vel_b_tone.value() as f64,

            couple,
            ab_mix: params.ab_mix.value() as f64,
            ab_split: params.ab_split.value() as f64 * 100.0,
            gain: 10.0f64.powf(params.gain.value() as f64 / 20.0),
            bend_range: params.bend_range.value() as f64,
            stereoizer: params.stereoizer.value(),
            reuse_voices: params.reuse_voices.value(),
            fadeout_repeats: params.fadeout_repeats.value(),
        }
    }

    /// The settings of the noise generator.
    pub(crate) fn noise_settings(&self) -> NoiseSettings {
        NoiseSettings {
            filter_mode: match self.noise_filter_mode {
                NoiseFilterMode::LowPass => 0,
                NoiseFilterMode::BandPass => 1,
                NoiseFilterMode::HighPass => 2,
            },
            freq: self.noise_filter_freq,
            q: self.noise_filter_q,
            attack: self.noise_att,
            decay: self.noise_dec,
            sustain: self.noise_sus,
            release: self.noise_rel,
            vel_freq: self.vel_noise_freq,
            vel_q: self.vel_noise_q,
            att_ten: self.noise_att_ten,
            dec_ten: self.noise_dec_ten,
            rel_ten: self.noise_rel_ten,
            vel_att: self.vel_noise_att,
            vel_dec: self.vel_noise_dec,
            vel_sus: self.vel_noise_sus,
            vel_rel: self.vel_noise_rel,
        }
    }

    /// The settings of the first resonator.
    pub(crate) fn resonator_a_settings(&self) -> ResonatorSettings {
        ResonatorSettings {
            on: self.a_on,
            model: self.a_model,
            partials: self.a_partials,
            decay: self.a_decay,
            damp: self.a_damp,
            tone: self.a_tone,
            hit: self.a_hit,
            rel: self.a_rel,
            inharm: self.a_inharm,
            cut: self.a_cut,
            radius: self.a_radius,
            vel_decay: self.vel_a_decay,
            vel_hit: self.vel_a_hit,
            vel_inharm: self.vel_a_inharm,
            vel_damp: self.vel_a_damp,
            vel_tone: self.vel_a_tone,
        }
    }

    /// The settings of the second resonator.
    pub(crate) fn resonator_b_settings(&self) -> ResonatorSettings {
        ResonatorSettings {
            on: self.b_on,
            model: self.b_model,
            partials: self.b_partials,
            decay: self.b_decay,
            damp: self.b_damp,
            tone: self.b_tone,
            hit: self.b_hit,
            rel: self.b_rel,
            inharm: self.b_inharm,
            cut: self.b_cut,
            radius: self.b_radius,
            vel_decay: self.vel_b_decay,
            vel_hit: self.vel_b_hit,
            vel_inharm: self.vel_b_inharm,
            vel_damp: self.vel_b_damp,
            vel_tone: self.vel_b_tone,
        }
    }
}

/// The number of partials a choice stands for.
///
/// The C++ converts the choice's index into a number of partials.
fn partial_count(count: PartialCount) -> usize {
    match count {
        PartialCount::Four => 4,
        PartialCount::Eight => 8,
        PartialCount::Sixteen => 16,
        PartialCount::ThirtyTwo => 32,
        PartialCount::SixtyFour => 64,
        PartialCount::One => 1,
        PartialCount::Two => 2,
    }
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
                self.push_midi(MidiMsg {
                    offset: timing as i32,
                    kind: MidiMsgType::NoteOn,
                    note: key.number_or_middle_c() as i32,
                    vel: velocity as f64,
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
        let block = BlockParams::read(&self.params);

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
            ..
        } = self;

        let srate = *srate;
        let polyphony = (*polyphony).min(voices.len());

        // The messages that were handled in an earlier block are dropped.
        midi.queue.retain(|msg| msg.offset >= 0);

        // The input is read from the same buffer the output is written to, so
        // the input of a sample is read before the output of that sample is
        // written.
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

            // The parameters that scale with the velocity are read for every
            // sample, the way the JUCE build reads them.
            let mallet_mix = params.mallet.mallet_mix.value() as f64;
            let mallet_res = params.mallet.mallet_res.value() as f64;
            let vel_mallet_mix = params.velocity.vel_mallet_mix.value() as f64;
            let vel_mallet_res = params.velocity.vel_mallet_res.value() as f64;
            let noise_osc = params.noise.noise_osc.value() as f64;
            let vel_noise_mix = params.velocity.vel_noise_mix.value() as f64;
            let vel_noise_res = params.velocity.vel_noise_res.value() as f64;

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
                        * (mallet_mix + vel_mallet_mix * voice.vel).clamp(0.0, 1.0)
                        * voice_fade_out;
                    res_out += m_sample * (mallet_res + vel_mallet_res * voice.vel).clamp(0.0, 1.0);
                }

                // The audio input.
                if audio_in != 0.0 && voice.is_pressed {
                    res_out += audio_in;
                }

                // The noise.
                let noise = voice.noise.process(&mut rngs[index]);
                if voice.noise.env.state != Stage::Off {
                    let osc = if noise_osc > 0.0 && (block.noise_res > 0.0 || vel_noise_res > 0.0) {
                        let b = voice.process_oscillators(false, luts);
                        let a = voice.process_oscillators(true, luts);
                        voice.noise.process_osc(a + b) * noise_osc
                    } else {
                        0.0
                    };

                    let mix = (block.noise_mix + vel_noise_mix * voice.vel).clamp(0.0, 1.0) as f32;
                    dir_out += noise * noise_mix_range.unnormalize(mix) as f64 * voice_fade_out;

                    let res = (block.noise_res + vel_noise_res * voice.vel).clamp(0.0, 1.0) as f32;
                    res_out +=
                        (noise * (1.0 - noise_osc) + osc) * noise_res_range.unnormalize(res) as f64;
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
            );

            // A note that is pressed again is not held by the sustain pedal.
            // This is the fix for a note that is pressed twice and held: it
            // should not be released with the pedal.
            sustain_pedal_notes.retain(|held| held.note != msg.note);
        }
        MidiMsgType::NoteOff => {
            if !*sustain_pedal {
                off_note(msg, voices, models, release_count, polyphony);
            } else {
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
    let freq = 440.0 * 2.0f64.powf((msg.note - 69) as f64 / 12.0);

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
    bend.step = (bend.target - bend.start) / bend.total as f64;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use nice_plug::midi::{Channel, Key, VoiceID};
    use nice_plug_core::params::InternalParamMut;

    /// The sample rate the tests run at.
    const SRATE: f64 = 44100.0;

    /// A plugin with its voices allocated.
    pub(crate) fn plugin() -> RipplerX {
        let mut plugin = RipplerX::default();
        plugin.prepare(SRATE);
        plugin
    }

    /// Sets a parameter's value, the way the host does.
    pub(crate) fn set<T: InternalParamMut>(param: &T, value: T::Plain) {
        unsafe {
            param._internal_set_plain_value(value);
        }
    }

    /// A note on event.
    pub(crate) fn note_on(note: u8, velocity: f32, timing: u32) -> NoteEvent<()> {
        NoteEvent::<()>::NoteOn {
            timing,
            voice_id: VoiceID::Wildcard,
            channel: Channel::Number(0),
            key: Key::Number(note),
            velocity,
        }
    }

    /// A note off event.
    pub(crate) fn note_off(note: u8, timing: u32) -> NoteEvent<()> {
        NoteEvent::<()>::NoteOff {
            timing,
            voice_id: VoiceID::Wildcard,
            channel: Channel::Number(0),
            key: Key::Number(note),
            velocity: 0.0,
        }
    }

    /// A control change event.
    pub(crate) fn control_change(cc: u8, value: f32, timing: u32) -> NoteEvent<()> {
        NoteEvent::<()>::MidiCC {
            timing,
            channel: 0,
            cc,
            value,
        }
    }

    /// The sustain pedal, down or up.
    pub(crate) fn sustain(down: bool, timing: u32) -> NoteEvent<()> {
        control_change(SUSTAIN_PEDAL, if down { 1.0 } else { 0.0 }, timing)
    }

    /// A pitch wheel event.
    pub(crate) fn pitch_wheel(value: u16, timing: u32) -> NoteEvent<()> {
        NoteEvent::<()>::MidiPitchBend {
            timing,
            channel: 0,
            value: value as f32 / 16383.0,
        }
    }

    /// Runs one block of audio through the plugin.
    pub(crate) fn run(
        plugin: &mut RipplerX,
        num_samples: usize,
        events: &[NoteEvent<()>],
    ) -> (Vec<f32>, Vec<f32>) {
        for event in events {
            plugin.queue_event(*event);
        }

        let mut left = vec![0.0f32; num_samples];
        let mut right = vec![0.0f32; num_samples];

        {
            let mut buffer = Buffer::default();
            unsafe {
                buffer.set_slices(num_samples, |slices| {
                    *slices = vec![&mut left[..], &mut right[..]];
                });
            }
            plugin.process_block(&mut buffer);
        }

        (left, right)
    }

    /// The loudest sample of a block.
    pub(crate) fn peak(samples: &[f32]) -> f32 {
        samples
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
    }

    /// Runs enough samples for the stereoizer's delay line to be flushed.
    ///
    /// The comb filter holds twenty milliseconds of the signal, so the
    /// output is not silent right after the voices are silenced.
    pub(crate) fn settle(plugin: &mut RipplerX) {
        run(plugin, 1024, &[]);
    }

    /// A plugin whose notes release quickly.
    pub(crate) fn short_release_plugin() -> RipplerX {
        let plugin = plugin();
        set(&plugin.params.resonator_a.a_rel, 0.01f32);
        set(&plugin.params.resonator_b.b_rel, 0.01f32);
        plugin
    }

    #[test]
    fn a_note_starts_a_voice() {
        let mut plugin = plugin();

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
        let mut plugin = plugin();

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
        let mut plugin = plugin();

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
        let mut plugin = plugin();

        let (left, right) = run(&mut plugin, 4096, &[note_on(60, 0.8, 0)]);

        assert!(
            left.iter().zip(right.iter()).any(|(l, r)| l != r),
            "the stereoizer did not spread the sound"
        );
    }
}
