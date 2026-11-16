//! The plugin's identity: the class IDs and the identifiers.
//!
//! The VST3 class ID is the one the JUCE build uses, so that a host finds
//! the plugin where the JUCE plugin left it, and so that the presets of one
//! build work with the other.
//!
//! JUCE derives the class ID in `juce::VST3Interface::jucePluginId` from the
//! manufacturer code `0x54696C72` ("Tilr"), the plugin code `0x52697078`
//! ("Ripx") and two fixed words, `0xABCDEF01` and `0x9182FAEB` for the
//! component interface. The byte order of those two words depends on the
//! platform JUCE was compiled for, because `juce_VST3Interface.h` branches on
//! `JUCE_WINDOWS`:
//!
//! * on Windows the two words are read least significant byte first,
//! * on macOS and Linux they are read most significant byte first.
//!
//! Both values were verified against the released binaries: the Windows
//! `RipplerX.vst3` holds the byte sequence `01 EF CD AB 82 91 EB FA 54 69
//! 6C 72 52 69 70 78`, and the Linux `RipplerX.so` holds the words
//! `0xABCDEF01` and `0x9182FAEB` as native `uint32`s, which is what the
//! non-Windows branch produces.

/// The VST3 class ID the JUCE build uses on Windows.
///
/// The bytes are `0xABCDEF01` and `0x9182FAEB` read least significant byte
/// first, followed by the manufacturer code and the plugin code, which are
/// always read most significant byte first.
pub const VST3_CLASS_ID_WINDOWS: [u8; 16] = [
    0x01, 0xEF, 0xCD, 0xAB, 0x82, 0x91, 0xEB, 0xFA, 0x54, 0x69, 0x6C, 0x72, 0x52, 0x69, 0x70, 0x78,
];

/// The VST3 class ID the JUCE build uses on macOS and Linux.
///
/// The bytes are `0xABCDEF01` and `0x9182FAEB` read most significant byte
/// first, followed by the manufacturer code and the plugin code.
pub const VST3_CLASS_ID_POSIX: [u8; 16] = [
    0xAB, 0xCD, 0xEF, 0x01, 0x91, 0x82, 0xFA, 0xEB, 0x54, 0x69, 0x6C, 0x72, 0x52, 0x69, 0x70, 0x78,
];

/// The VST3 class ID of the build for the platform this was compiled for.
///
/// This is the value the host sees: the VST3 factory reports it as the
/// plugin's class ID.
#[cfg(target_os = "windows")]
pub const VST3_CLASS_ID: [u8; 16] = VST3_CLASS_ID_WINDOWS;

/// The VST3 class ID of the build for the platform this was compiled for.
///
/// This is the value the host sees: the VST3 factory reports it as the
/// plugin's class ID.
#[cfg(not(target_os = "windows"))]
pub const VST3_CLASS_ID: [u8; 16] = VST3_CLASS_ID_POSIX;

/// The CLAP identifier, unchanged from the JUCE build.
///
/// It is the bundle identifier `com.tilr.ripplerx` of `CMakeLists.txt`.
pub const CLAP_ID: &str = "com.tilr.ripplerx";

/// The Windows class ID as the hexadecimal string the VST3 module information
/// file holds.
///
/// Steinberg's tools write the class ID as the raw bytes in upper case
/// hexadecimal, which is how the released bundles spell it.
pub const VST3_CLASS_ID_WINDOWS_HEX: &str = "01EFCDAB8291EBFA54696C7252697078";

/// The macOS and Linux class ID as the hexadecimal string the VST3 module
/// information file holds.
pub const VST3_CLASS_ID_POSIX_HEX: &str = "ABCDEF019182FAEB54696C7252697078";

/// The class ID of this platform, as the hexadecimal string the VST3 module
/// information file holds.
#[cfg(target_os = "windows")]
pub const VST3_CLASS_ID_HEX: &str = VST3_CLASS_ID_WINDOWS_HEX;

/// The class ID of this platform, as the hexadecimal string the VST3 module
/// information file holds.
#[cfg(not(target_os = "windows"))]
pub const VST3_CLASS_ID_HEX: &str = VST3_CLASS_ID_POSIX_HEX;

#[cfg(test)]
mod tests {
    use super::*;

    /// The hexadecimal spelling of a class ID, the way Steinberg's tools write
    /// it: the raw bytes in upper case hexadecimal.
    fn hex(class_id: &[u8; 16]) -> String {
        let mut hex = String::with_capacity(32);
        for byte in class_id {
            hex.push_str(&format!("{byte:02X}"));
        }
        hex
    }

    /// The Windows class ID, as the hexadecimal string.
    const WINDOWS_HEX: &str = "01EFCDAB8291EBFA54696C7252697078";

    /// The macOS and Linux class ID, as the hexadecimal string.
    const POSIX_HEX: &str = "ABCDEF019182FAEB54696C7252697078";

    #[test]
    fn the_windows_class_id_has_the_bytes_the_juce_build_uses() {
        // The byte sequence that appears in the released `RipplerX.vst3`.
        assert_eq!(
            VST3_CLASS_ID_WINDOWS,
            [
                0x01, 0xEF, 0xCD, 0xAB, 0x82, 0x91, 0xEB, 0xFA, 0x54, 0x69, 0x6C, 0x72, 0x52, 0x69,
                0x70, 0x78
            ]
        );
        assert_eq!(hex(&VST3_CLASS_ID_WINDOWS), WINDOWS_HEX);
        assert_eq!(VST3_CLASS_ID_WINDOWS_HEX, WINDOWS_HEX);
    }

    #[test]
    fn the_posix_class_id_has_the_bytes_the_juce_build_uses() {
        // The two words the released `RipplerX.so` holds as native `uint32`s.
        assert_eq!(
            VST3_CLASS_ID_POSIX,
            [
                0xAB, 0xCD, 0xEF, 0x01, 0x91, 0x82, 0xFA, 0xEB, 0x54, 0x69, 0x6C, 0x72, 0x52, 0x69,
                0x70, 0x78
            ]
        );
        assert_eq!(hex(&VST3_CLASS_ID_POSIX), POSIX_HEX);
        assert_eq!(VST3_CLASS_ID_POSIX_HEX, POSIX_HEX);
    }

    #[test]
    fn the_two_class_ids_differ_only_in_the_first_two_words() {
        assert_eq!(
            VST3_CLASS_ID_WINDOWS[8..],
            VST3_CLASS_ID_POSIX[8..],
            "the manufacturer and the plugin code are the same"
        );
        assert_ne!(VST3_CLASS_ID_WINDOWS[..8], VST3_CLASS_ID_POSIX[..8]);
    }

    #[test]
    fn the_class_id_of_this_platform_is_the_platforms_one() {
        #[cfg(target_os = "windows")]
        assert_eq!(VST3_CLASS_ID, VST3_CLASS_ID_WINDOWS);

        #[cfg(not(target_os = "windows"))]
        assert_eq!(VST3_CLASS_ID, VST3_CLASS_ID_POSIX);
    }

    #[test]
    fn the_clap_identifier_is_the_bundle_identifier() {
        assert_eq!(CLAP_ID, "com.tilr.ripplerx");
    }
}
