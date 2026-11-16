//! The parameters of one block of audio.
//!
//! The JUCE build reads every parameter once per block into locals and then
//! works from those. This is the same thing: the values are plain, so the
//! loop never has to look at the parameter struct.

use nice_plug::prelude::*;
use rippler_dsp::mallet::MalletType;
use rippler_dsp::models::ModalModel;
use rippler_dsp::noise::NoiseSettings;
use rippler_dsp::resonator::ResonatorSettings;
use rippler_params::enums::{Coupling, NoiseFilterMode, PartialCount};
use rippler_params::params::RipplerParams;

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
    /// How much of the mallet reaches the output directly.
    pub mallet_mix: f64,
    /// How much of the mallet reaches the resonators.
    pub mallet_res: f64,

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
    ///
    /// The ranges of the noise's mix and resonance are the ones that were
    /// cached when the plugin was activated: the ranges never change, so
    /// there is no point in reading them again for every block.
    pub(crate) fn read(
        params: &RipplerParams,
        noise_mix_range: FloatRange,
        noise_res_range: FloatRange,
    ) -> Self {
        let couple = params.couple.value() == Coupling::Series;

        Self {
            mallet_type: params.mallet.mallet_type.value(),
            mallet_pitch: params.mallet.mallet_pitch.value() as f64,
            mallet_filter: params.mallet.mallet_filter.value() as f64,
            mallet_stiff: params.mallet.mallet_stiff.value() as f64,
            mallet_ktrack: params.mallet.mallet_ktrack.value() as f64,
            mallet_mix: params.mallet.mallet_mix.value() as f64,
            mallet_res: params.mallet.mallet_res.value() as f64,

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::set;
    use rippler_params::params::RipplerParams;

    /// The parameters of a plugin that has not been touched.
    fn params() -> RipplerParams {
        RipplerParams::default()
    }

    /// The ranges the plugin caches when it is activated.
    fn ranges() -> (FloatRange, FloatRange) {
        let params = params();
        (
            params.noise.noise_mix.range(),
            params.noise.noise_res.range(),
        )
    }

    #[test]
    fn the_partial_counts_match_the_choices() {
        assert_eq!(partial_count(PartialCount::Four), 4);
        assert_eq!(partial_count(PartialCount::Eight), 8);
        assert_eq!(partial_count(PartialCount::Sixteen), 16);
        assert_eq!(partial_count(PartialCount::ThirtyTwo), 32);
        assert_eq!(partial_count(PartialCount::SixtyFour), 64);
        assert_eq!(partial_count(PartialCount::One), 1);
        assert_eq!(partial_count(PartialCount::Two), 2);
    }

    #[test]
    fn the_velocity_scaled_parameters_are_read_once_per_block() {
        let params = params();
        set(&params.mallet.mallet_mix, 0.25f32);
        set(&params.mallet.mallet_res, 0.5f32);
        set(&params.velocity.vel_mallet_mix, 0.75f32);
        set(&params.velocity.vel_mallet_res, 0.125f32);
        set(&params.velocity.vel_noise_mix, 0.25f32);
        set(&params.velocity.vel_noise_res, 0.5f32);

        let (mix_range, res_range) = ranges();
        let block = BlockParams::read(&params, mix_range, res_range);

        assert_eq!(block.mallet_mix, 0.25);
        assert_eq!(block.mallet_res, 0.5);
        assert_eq!(block.vel_mallet_mix, 0.75);
        assert_eq!(block.vel_mallet_res, 0.125);
        assert_eq!(block.vel_noise_mix, 0.25);
        assert_eq!(block.vel_noise_res, 0.5);
    }

    #[test]
    fn the_noise_is_normalised_with_the_cached_ranges() {
        let params = params();
        let (mix_range, res_range) = ranges();
        set(&params.noise.noise_mix, 0.5f32);
        set(&params.noise.noise_res, 0.25f32);

        let block = BlockParams::read(&params, mix_range, res_range);

        assert_eq!(block.noise_mix, mix_range.normalize(0.5) as f64);
        assert_eq!(block.noise_res, res_range.normalize(0.25) as f64);
    }
}
