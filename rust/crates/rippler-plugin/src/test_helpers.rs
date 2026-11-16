//! The helpers the tests share.
//!
//! The tests of the crate all need a plugin with its voices allocated, a
//! way to set a parameter and a way to run a block of audio through the
//! plugin, so those live here rather than in one of the test modules.

use nice_plug::midi::{Channel, Key, VoiceID};
use nice_plug::prelude::*;
use nice_plug_core::params::InternalParamMut;

use crate::RipplerX;
use crate::process::SUSTAIN_PEDAL;

/// The sample rate the tests run at.
pub(crate) const SRATE: f64 = 44100.0;

/// A plugin with its voices allocated.
pub(crate) fn new_plugin() -> RipplerX {
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
    run_with_input(plugin, num_samples, &[], events)
}

/// Runs one block of audio through the plugin, with a signal on the input.
///
/// The plugin reads the main input from the same buffer it writes the
/// output to, so the input is what the buffer holds before the block is
/// processed.
pub(crate) fn run_with_input(
    plugin: &mut RipplerX,
    num_samples: usize,
    input: &[f32],
    events: &[NoteEvent<()>],
) -> (Vec<f32>, Vec<f32>) {
    assert!(
        input.is_empty() || input.len() == num_samples,
        "the input is the wrong length"
    );

    for event in events {
        plugin.queue_event(*event);
    }

    let mut left = vec![0.0f32; num_samples];
    let mut right = vec![0.0f32; num_samples];
    if !input.is_empty() {
        left.copy_from_slice(input);
        right.copy_from_slice(input);
    }

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
    let plugin = new_plugin();
    set(&plugin.params.resonator_a.a_rel, 0.01f32);
    set(&plugin.params.resonator_b.b_rel, 0.01f32);
    plugin
}
