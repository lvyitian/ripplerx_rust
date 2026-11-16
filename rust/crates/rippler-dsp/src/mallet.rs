//! The mallet generator.
//!
//! Port of `src/dsp/Mallet.{h,cpp}`.

/// The mallet that excites a resonator.
///
/// The discriminants match the `mallet_type` parameter's choice order, so a
/// variant's discriminant is the value the plugin stores. The ten reserved
/// slots must be kept: dropping them would shift every later index and
/// break every existing patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MalletType {
    /// A unit impulse through a band-pass filter.
    Impulse = 0,
    /// Reserved by the JUCE implementation.
    Reserved1 = 1,
    /// Reserved by the JUCE implementation.
    Reserved2 = 2,
    /// Reserved by the JUCE implementation.
    Reserved3 = 3,
    /// Reserved by the JUCE implementation.
    Reserved4 = 4,
    /// Reserved by the JUCE implementation.
    Reserved5 = 5,
    /// Reserved by the JUCE implementation.
    Reserved6 = 6,
    /// Reserved by the JUCE implementation.
    Reserved7 = 7,
    /// Reserved by the JUCE implementation.
    Reserved8 = 8,
    /// Reserved by the JUCE implementation.
    Reserved9 = 9,
    /// Reserved by the JUCE implementation.
    Reserved10 = 10,
    /// A user-supplied sample.
    UserFile = 11,
    /// The bundled `click1.flac` sample.
    Sample1 = 12,
    /// The bundled `click2.flac` sample.
    Sample2 = 13,
    /// The bundled `click3.flac` sample.
    Sample3 = 14,
    /// The bundled `click4.flac` sample.
    Sample4 = 15,
    /// The bundled `click5.flac` sample.
    Sample5 = 16,
    /// The bundled `click6.flac` sample.
    Sample6 = 17,
    /// The bundled `blip.flac` sample.
    Sample7 = 18,
    /// The bundled `blop.flac` sample.
    Sample8 = 19,
    /// The bundled `metal1.flac` sample.
    Sample9 = 20,
    /// The bundled `metal2.flac` sample.
    Sample10 = 21,
    /// The bundled `wood.flac` sample.
    Sample11 = 22,
    /// The bundled `strike.flac` sample.
    Sample12 = 23,
    /// The bundled `perc1.flac` sample.
    Sample13 = 24,
    /// The bundled `perc2.flac` sample.
    Sample14 = 25,
}
