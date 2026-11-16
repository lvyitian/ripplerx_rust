//! The microtuning.
//!
//! Port of `Voice::note2freq` in `src/dsp/Voice.cpp`.
//!
//! The JUCE build asks MTS-ESP for the frequency of a note and falls back
//! to the equal temperament formula when there is no MTS-ESP master.

use mts_client_rs::Client;

/// Turns a note number into a frequency in hertz.
///
/// Uses MTS-ESP when a tuning client is available, and equal temperament
/// otherwise.
///
/// The equal temperament formula is the C++'s: `440 * 2^((note - 69) / 12)`.
/// The MTS-ESP path is the C++'s: `MTS_NoteToFrequency(mts, note, -1)`.
pub fn note_to_frequency(note: i32, tuning: Option<&Client>) -> f64 {
    match tuning {
        Some(client) => client.note_to_frequency(note as u8, None),
        None => equal_temperament(note),
    }
}

/// The frequency of a note in the equal temperament.
///
/// The C++'s `440 * pow(2.0, (note - 69) / 12.0)`.
fn equal_temperament(note: i32) -> f64 {
    440.0 * 2.0f64.powf((note - 69) as f64 / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_temperament_matches_the_cpp_formula() {
        assert_eq!(equal_temperament(69), 440.0);
        assert!((equal_temperament(60) - 261.625_565_300_598_6).abs() < 1e-9);
        assert_eq!(equal_temperament(81), 880.0);

        // The formula is the C++'s, so the octaves are exact.
        assert_eq!(equal_temperament(69 + 12), 880.0);
        assert_eq!(equal_temperament(69 - 12), 220.0);
    }

    #[test]
    fn the_tuning_falls_back_when_mts_is_unavailable() {
        // Without a client the equal temperament is used.
        assert_eq!(note_to_frequency(69, None), 440.0);
        assert!((note_to_frequency(60, None) - 261.625_565_300_598_6).abs() < 1e-9);
        assert_eq!(note_to_frequency(81, None), 880.0);

        // The same is true when a client is there but no master is
        // connected, which is the case on a machine without the library.
        if let Ok(client) = Client::new() {
            assert!(
                (note_to_frequency(69, Some(&client)) - 440.0).abs() < 1e-9,
                "the tuning client did not fall back to the equal temperament"
            );
        }
    }
}
