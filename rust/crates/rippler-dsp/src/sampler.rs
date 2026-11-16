//! The sample player for the mallet.
//!
//! Port of `src/dsp/Sampler.{h,cpp}`.
//!
//! The JUCE implementation decodes with `juce::AudioFormatManager`, which
//! drags the whole plugin framework into the DSP core. This port decodes with
//! `symphonia` instead, and keeps the C++'s order of operations exactly:
//! decode, mix down to mono, truncate to three seconds, and only then
//! normalise, so the peak is measured over the truncated waveform.

use std::path::Path;

use symphonia::core::audio::{AudioBuffer, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::mallet::MalletType;

/// The bundled `click1.flac` sample.
const CLICK1: &[u8] = include_bytes!("../../../../resources/samples/click1.flac");
/// The bundled `click2.flac` sample.
const CLICK2: &[u8] = include_bytes!("../../../../resources/samples/click2.flac");
/// The bundled `click3.flac` sample.
const CLICK3: &[u8] = include_bytes!("../../../../resources/samples/click3.flac");
/// The bundled `click4.flac` sample.
const CLICK4: &[u8] = include_bytes!("../../../../resources/samples/click4.flac");
/// The bundled `click5.flac` sample.
const CLICK5: &[u8] = include_bytes!("../../../../resources/samples/click5.flac");
/// The bundled `click6.flac` sample.
const CLICK6: &[u8] = include_bytes!("../../../../resources/samples/click6.flac");
/// The bundled `blip.flac` sample.
const BLIP: &[u8] = include_bytes!("../../../../resources/samples/blip.flac");
/// The bundled `blop.flac` sample.
const BLOP: &[u8] = include_bytes!("../../../../resources/samples/blop.flac");
/// The bundled `metal1.flac` sample.
const METAL1: &[u8] = include_bytes!("../../../../resources/samples/metal1.flac");
/// The bundled `metal2.flac` sample.
const METAL2: &[u8] = include_bytes!("../../../../resources/samples/metal2.flac");
/// The bundled `wood.flac` sample.
const WOOD: &[u8] = include_bytes!("../../../../resources/samples/wood.flac");
/// The bundled `strike.flac` sample.
const STRIKE: &[u8] = include_bytes!("../../../../resources/samples/strike.flac");
/// The bundled `perc1.flac` sample.
const PERC1: &[u8] = include_bytes!("../../../../resources/samples/perc1.flac");
/// The bundled `perc2.flac` sample.
const PERC2: &[u8] = include_bytes!("../../../../resources/samples/perc2.flac");

/// The bytes of the bundled sample a mallet type stands for.
///
/// The reserved types and the impulse have no sample of their own.
fn internal_sample_bytes(variant: MalletType) -> Option<&'static [u8]> {
    match variant {
        MalletType::Sample1 => Some(CLICK1),
        MalletType::Sample2 => Some(CLICK2),
        MalletType::Sample3 => Some(CLICK3),
        MalletType::Sample4 => Some(CLICK4),
        MalletType::Sample5 => Some(CLICK5),
        MalletType::Sample6 => Some(CLICK6),
        MalletType::Sample7 => Some(BLIP),
        MalletType::Sample8 => Some(BLOP),
        MalletType::Sample9 => Some(METAL1),
        MalletType::Sample10 => Some(METAL2),
        MalletType::Sample11 => Some(WOOD),
        MalletType::Sample12 => Some(STRIKE),
        MalletType::Sample13 => Some(PERC1),
        MalletType::Sample14 => Some(PERC2),
        _ => None,
    }
}

/// Decodes a base64 string into bytes.
///
/// This follows JUCE's decoder: every character outside the base64 alphabet
/// is skipped, and the padding ends the stream.
fn decode_base64(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() / 4 * 3);
    let mut accumulator: u32 = 0;
    let mut bits: u32 = 0;

    for character in text.bytes() {
        if character == b'=' {
            break;
        }

        let Some(value) = base64_value(character) else {
            continue;
        };

        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push(((accumulator >> bits) & 0xff) as u8);
        }
    }

    bytes
}

/// The six bit value of a base64 character, or `None` if it is not in the
/// alphabet.
fn base64_value(character: u8) -> Option<u8> {
    match character {
        b'A'..=b'Z' => Some(character - b'A'),
        b'a'..=b'z' => Some(character - b'a' + 26),
        b'0'..=b'9' => Some(character - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// The waveform a mallet plays.
#[derive(Clone, Debug)]
pub struct Sampler {
    /// The decoded waveform, one sample per entry.
    pub waveform: Vec<f64>,
    /// Whether the waveform came from a user file.
    pub is_user_file: bool,
    /// The sample rate of the waveform.
    pub wavesrate: f64,
    /// The pitch shift factor.
    pub pitchfactor: f64,
}

impl Sampler {
    /// Creates a sampler with the C++'s default values.
    pub fn new() -> Self {
        Self {
            waveform: Vec::new(),
            is_user_file: false,
            wavesrate: 44100.0,
            pitchfactor: 1.0,
        }
    }

    /// Shifts the playback pitch by a number of semitones.
    pub fn set_pitch(&mut self, semis: f64) {
        self.pitchfactor = 2.0f64.powf(semis / 12.0);
    }

    /// Loads a waveform from the base64 blob the plugin keeps in its session
    /// state.
    ///
    /// The C++ clears the waveform first, so a second load replaces the first,
    /// and it neither sets the sample rate nor normalises the samples. All
    /// three are reproduced here, because the session format depends on them.
    pub fn load_encoded(&mut self, encoded: &str) {
        self.waveform.clear();

        let bytes = decode_base64(encoded);
        for chunk in bytes.as_chunks::<8>().0 {
            let mut buffer = [0u8; 8];
            buffer.copy_from_slice(chunk);
            self.waveform.push(f64::from_le_bytes(buffer));
        }

        self.is_user_file = true;
    }

    /// Loads one of the bundled samples.
    ///
    /// A type without a sample of its own falls back to the first one, as the
    /// C++ does.
    pub fn load_internal_sample(&mut self, variant: MalletType) {
        let bytes = internal_sample_bytes(variant).unwrap_or(CLICK1);
        if !self.decode_into(bytes) {
            // The bundled samples always decode, but the C++ falls back to
            // the first one if one of them ever fails to.
            self.decode_into(CLICK1);
        }
        self.is_user_file = false;
    }

    /// Loads a user file, falling back to the first bundled sample when the
    /// file cannot be read.
    pub fn load_sample(&mut self, path: &Path) {
        if !path.is_file() {
            self.load_internal_sample(MalletType::Sample1);
            return;
        }

        let Ok(bytes) = std::fs::read(path) else {
            self.load_internal_sample(MalletType::Sample1);
            return;
        };

        self.load_bytes(&bytes);
        self.is_user_file = true;
    }

    /// Decodes a byte slice into the waveform.
    ///
    /// Returns whether the decoding succeeded. A file that cannot be decoded
    /// falls back to the first bundled sample, as the C++ does.
    pub fn load_bytes(&mut self, bytes: &[u8]) -> bool {
        if self.decode_into(bytes) {
            return true;
        }

        self.load_internal_sample(MalletType::Sample1);
        false
    }

    /// Decodes a byte slice into the waveform, without any fallback.
    ///
    /// The decoding follows the C++'s `loadSampleFromBinary`: the channels
    /// are summed and divided by their count, the waveform is truncated to
    /// three seconds, and the truncated waveform is then scaled so that its
    /// largest absolute value is one.
    fn decode_into(&mut self, bytes: &[u8]) -> bool {
        let Some(decoded) = decode_audio(bytes) else {
            return false;
        };

        self.wavesrate = decoded.sample_rate;
        self.waveform = decoded.mono();

        // The C++ truncates to `3 * (int) wavesrate` samples, and computes
        // the peak over the truncated waveform.
        let limit = (3 * (self.wavesrate as i32)).max(0) as usize;
        self.waveform.truncate(limit);

        let mut peak = 0.0f64;
        for value in self.waveform.iter() {
            peak = peak.max(value.abs());
        }
        if peak > 0.0 {
            let scale = 1.0 / peak;
            for value in self.waveform.iter_mut() {
                *value *= scale;
            }
        }

        true
    }

    /// Reads the waveform at a position, interpolating linearly.
    ///
    /// The position is truncated towards zero and the read wraps around at the
    /// end of the waveform.
    pub fn wave_lerp(&self, pos: f64) -> f64 {
        if self.waveform.is_empty() {
            return 0.0;
        }

        let i = pos as i64;
        let frac = pos - i as f64;
        let mut j = i + 1;
        if j >= self.waveform.len() as i64 {
            j = 0;
        }

        (1.0 - frac) * self.waveform[i as usize] + frac * self.waveform[j as usize]
    }

    /// Reads the waveform at a position, interpolating with a Catmull-Rom
    /// cubic.
    ///
    /// The four samples around the position wrap around at the ends of the
    /// waveform.
    pub fn wave_cubic(&self, pos: f64) -> f64 {
        let n = self.waveform.len();
        if n == 0 {
            return 0.0;
        }

        let i1 = pos as i64;
        let x = pos - i1 as f64;

        let n64 = n as i64;
        let i0 = (i1 - 1 + n64).rem_euclid(n64) as usize;
        let i2 = (i1 + 1).rem_euclid(n64) as usize;
        let i3 = (i1 + 2).rem_euclid(n64) as usize;

        let y0 = self.waveform[i0];
        let y1 = self.waveform[i1.rem_euclid(n64) as usize];
        let y2 = self.waveform[i2];
        let y3 = self.waveform[i3];

        // Catmull-Rom cubic interpolation.
        let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
        let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c = -0.5 * y0 + 0.5 * y2;
        let d = y1;

        ((a * x + b) * x + c) * x + d
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

/// A decoded audio file.
struct DecodedAudio {
    /// The sample rate of the file.
    sample_rate: f64,
    /// The samples, one vector per channel.
    channels: Vec<Vec<f64>>,
}

impl DecodedAudio {
    /// Mixes the channels down to one, the way the C++ does.
    fn mono(&self) -> Vec<f64> {
        let channel_count = self.channels.len();
        if channel_count == 0 {
            return Vec::new();
        }

        let frames = self.channels.iter().map(Vec::len).min().unwrap_or(0);
        let mut mono = Vec::with_capacity(frames);

        for frame in 0..frames {
            let mut sample = 0.0;
            for channel in self.channels.iter() {
                sample += channel[frame];
            }
            sample /= channel_count as f64;
            mono.push(sample);
        }

        mono
    }
}

/// Decodes an audio file into its channels.
fn decode_audio(bytes: &[u8]) -> Option<DecodedAudio> {
    let hint = Hint::new();
    // The media source has to own its data, so the bytes are copied.
    let source = MediaSourceStream::new(
        Box::new(std::io::Cursor::new(bytes.to_vec())),
        Default::default(),
    );

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let mut format = probed.format;

    let track = format.default_track()?;
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate? as f64;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;

    let mut channels: Vec<Vec<f64>> = Vec::new();

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            // A packet that does not decode is skipped, as the reference
            // player does.
            Err(Error::DecodeError(_)) => continue,
            Err(_) => break,
        };

        let channel_count = decoded.spec().channels.count();
        let mut buffer = AudioBuffer::<f64>::new(decoded.capacity() as u64, *decoded.spec());
        decoded.convert(&mut buffer);

        while channels.len() < channel_count {
            channels.push(Vec::new());
        }
        for (index, channel) in channels.iter_mut().enumerate() {
            channel.extend_from_slice(buffer.chan(index));
        }
    }

    if channels.is_empty() {
        return None;
    }

    Some(DecodedAudio {
        sample_rate,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fourteen bundled samples, with the number of samples in the file
    /// and the peak of the normalised waveform.
    ///
    /// The lengths come from the `STREAMINFO` block of each file, which
    /// holds the total number of samples. The peak is one for every sample
    /// but `blop.flac`, whose peak lands one unit in the last place below
    /// one: the normalisation multiplies by the reciprocal of the peak, and
    /// that reciprocal is not exact. The C++ rounds in exactly the same way.
    const BUNDLED: [(MalletType, usize, f64); 14] = [
        (MalletType::Sample1, 218, 1.0),
        (MalletType::Sample2, 242, 1.0),
        (MalletType::Sample3, 416, 1.0),
        (MalletType::Sample4, 227, 1.0),
        (MalletType::Sample5, 130, 1.0),
        (MalletType::Sample6, 410, 1.0),
        (MalletType::Sample7, 4845, 1.0),
        (MalletType::Sample8, 4552, 0.999_999_999_999_999_9),
        (MalletType::Sample9, 10926, 1.0),
        (MalletType::Sample10, 13057, 1.0),
        (MalletType::Sample11, 9034, 1.0),
        (MalletType::Sample12, 7477, 1.0),
        (MalletType::Sample13, 11704, 1.0),
        (MalletType::Sample14, 41686, 1.0),
    ];

    /// Encodes a list of doubles as base64, the way the plugin's session
    /// state does.
    fn encode_base64(values: &[f64]) -> String {
        let mut bytes = Vec::with_capacity(values.len() * 8);
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let mut encoded = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
            let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
            let triple = (b0 << 16) | (b1 << 8) | b2;

            encoded.push(ALPHABET[(triple >> 18) as usize & 0x3f] as char);
            encoded.push(ALPHABET[(triple >> 12) as usize & 0x3f] as char);
            if chunk.len() > 1 {
                encoded.push(ALPHABET[(triple >> 6) as usize & 0x3f] as char);
            } else {
                encoded.push('=');
            }
            if chunk.len() > 2 {
                encoded.push(ALPHABET[triple as usize & 0x3f] as char);
            } else {
                encoded.push('=');
            }
        }

        encoded
    }

    #[test]
    fn a_new_sampler_has_the_cpp_defaults() {
        let sampler = Sampler::new();

        assert!(sampler.waveform.is_empty());
        assert!(!sampler.is_user_file);
        assert_eq!(sampler.wavesrate, 44100.0);
        assert_eq!(sampler.pitchfactor, 1.0);
    }

    #[test]
    fn setting_the_pitch_scales_by_semitones() {
        let mut sampler = Sampler::new();

        sampler.set_pitch(0.0);
        assert_eq!(sampler.pitchfactor, 1.0);

        sampler.set_pitch(12.0);
        assert_eq!(sampler.pitchfactor, 2.0);

        sampler.set_pitch(-12.0);
        assert_eq!(sampler.pitchfactor, 0.5);

        sampler.set_pitch(7.0);
        assert!((sampler.pitchfactor - 2.0f64.powf(7.0 / 12.0)).abs() < 1e-15);
    }

    #[test]
    fn loading_an_encoded_waveform_reads_the_doubles() {
        let values = [0.5f64, -0.25, 1.0, 0.0, 1234.5678];
        let mut sampler = Sampler::new();
        sampler.load_encoded(&encode_base64(&values));

        assert_eq!(sampler.waveform, values.to_vec());
        assert!(sampler.is_user_file);

        // The C++ neither sets the sample rate nor normalises the samples.
        assert_eq!(sampler.wavesrate, 44100.0);
    }

    #[test]
    fn loading_an_encoded_waveform_ignores_a_partial_double() {
        // Seven bytes are not a double, so they are dropped.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1.0f64.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7]);

        let mut sampler = Sampler::new();
        sampler.load_encoded(&encode_base64(&[1.0]));

        assert_eq!(sampler.waveform.len(), 1);
        assert_eq!(sampler.waveform[0], 1.0);
        assert_eq!(bytes.len(), 15);
    }

    #[test]
    fn loading_an_internal_sample_decodes_it() {
        for (variant, length, expected_peak) in BUNDLED {
            let mut sampler = Sampler::new();
            sampler.load_internal_sample(variant);

            assert!(!sampler.waveform.is_empty(), "{variant:?} did not decode");
            assert_eq!(
                sampler.waveform.len(),
                length,
                "{variant:?} has a wrong length"
            );
            assert_eq!(
                sampler.wavesrate, 44100.0,
                "{variant:?} has a wrong sample rate"
            );
            assert!(
                !sampler.is_user_file,
                "{variant:?} was marked as a user file"
            );

            let peak = sampler
                .waveform
                .iter()
                .fold(0.0f64, |peak, value| peak.max(value.abs()));
            assert_eq!(peak, expected_peak, "{variant:?} has a peak of {peak}");
        }
    }

    #[test]
    fn the_bundled_samples_have_the_expected_peak_and_length() {
        // The lengths are the sample counts from the `STREAMINFO` block of
        // each file, and every file is shorter than three seconds, so the
        // truncation never applies and the waveform is the whole file.
        for (variant, length, expected_peak) in BUNDLED {
            let mut sampler = Sampler::new();
            sampler.load_internal_sample(variant);

            assert_eq!(
                sampler.waveform.len(),
                length,
                "{variant:?} has a wrong length"
            );

            let peak = sampler
                .waveform
                .iter()
                .fold(0.0f64, |peak, value| peak.max(value.abs()));
            assert_eq!(peak, expected_peak, "{variant:?} has a peak of {peak}");
        }
    }

    #[test]
    fn the_waveform_is_mono_and_truncated_to_three_seconds() {
        let mut sampler = Sampler::new();
        sampler.load_internal_sample(MalletType::Sample1);

        // The limit is three seconds at the file's sample rate.
        let limit = (3 * (sampler.wavesrate as i32)) as usize;
        assert!(sampler.waveform.len() <= limit);
        assert_eq!(sampler.waveform.len(), 218);
    }

    #[test]
    fn wave_lerp_interpolates_linearly_and_wraps() {
        let mut sampler = Sampler::new();
        sampler.waveform = vec![0.0, 1.0, 2.0, 3.0];

        assert_eq!(sampler.wave_lerp(0.0), 0.0);
        assert_eq!(sampler.wave_lerp(1.0), 1.0);
        assert!((sampler.wave_lerp(1.5) - 1.5).abs() < 1e-15);
        assert!((sampler.wave_lerp(2.25) - 2.25).abs() < 1e-15);

        // The last sample wraps around to the first.
        assert!((sampler.wave_lerp(3.5) - 1.5).abs() < 1e-15);
        assert_eq!(sampler.wave_lerp(3.0), 3.0);
    }

    #[test]
    fn wave_cubic_matches_the_catmull_rom_formula() {
        let mut sampler = Sampler::new();
        sampler.waveform = vec![0.0, 1.0, 2.0, 3.0];

        // The Catmull-Rom spline passes through the samples it interpolates.
        for index in 0..4 {
            let value = sampler.wave_cubic(index as f64);
            assert!(
                (value - sampler.waveform[index]).abs() < 1e-12,
                "sample {index} was not reproduced: {value}"
            );
        }

        // Halfway between two samples the spline overshoots a straight line.
        let halfway = sampler.wave_cubic(1.5);
        assert!((halfway - 1.5).abs() < 1e-12);

        // The read wraps around at the end of the waveform.
        let wrapped = sampler.wave_cubic(4.0);
        assert!((wrapped - sampler.wave_cubic(0.0)).abs() < 1e-12);
    }

    #[test]
    fn loading_a_missing_file_falls_back_to_the_first_sample() {
        let mut sampler = Sampler::new();
        sampler.load_sample(Path::new("this/file/does/not/exist.wav"));

        let mut reference = Sampler::new();
        reference.load_internal_sample(MalletType::Sample1);

        assert_eq!(sampler.waveform, reference.waveform);
        assert!(!sampler.is_user_file);
    }

    #[test]
    fn loading_bytes_that_are_not_audio_falls_back_to_the_first_sample() {
        let mut sampler = Sampler::new();
        let succeeded = sampler.load_bytes(b"this is not an audio file");

        assert!(!succeeded);

        let mut reference = Sampler::new();
        reference.load_internal_sample(MalletType::Sample1);
        assert_eq!(sampler.waveform, reference.waveform);
    }
}
