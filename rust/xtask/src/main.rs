//! The build tasks of the RipplerX plugin.
//!
//! The task that matters here is `cargo xtask bundle -p rippler-plugin
//! --release`: it builds the plugin, bundles it with nice-plug's bundler, and
//! writes the VST3 module information file into the bundle.
//!
//! The JUCE build ships a `Contents/Resources/moduleinfo.json` in its VST3
//! bundle, and hosts and Steinberg's own tools read that file to find out
//! what a plugin is without loading its binary. nice-plug's bundler does not
//! write such a file, so this task writes one after the bundling is done.
//!
//! The file is written from the plugin's own constants, so it cannot drift
//! away from the binary it describes.

use anyhow::{Context, Result};
use nice_plug::prelude::*;
use rippler_plugin::RipplerX;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The version of the VST3 SDK the plugin is written against.
///
/// The JUCE build was made against the VST 3.8.0 SDK, and the module
/// information file spells that version out.
const SDK_VERSION: &str = "VST 3.8.0";

/// The category of the plugin's single class.
///
/// nice-plug's plugins are a single component class, where the JUCE build has
/// a component class and a controller class.
const CATEGORY: &str = "Audio Module Class";

/// The number of instances of the plugin a host may create.
const CARDINALITY: u32 = i32::MAX as u32;

/// The class flags of the plugin's class.
///
/// This is the value the JUCE build writes for its class.
const CLASS_FLAGS: u32 = 2;

fn main() -> Result<()> {
    // The packages that are being bundled, so that the module information file
    // can be written into the bundles that the bundling produced.
    let packages = packages_from_args()?;

    // nice-plug's bundler builds the plugin and copies it into its bundle.
    nice_plug_xtask::main()?;

    write_module_info(&packages)
}

/// The packages that are being bundled.
///
/// The bundler accepts the packages either as `-p <package>` flags or as a
/// single package name, and this reads them the same way.
fn packages_from_args() -> Result<Vec<String>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args
        .get(1..)
        .context("The xtask needs a command, like 'bundle'")?;

    let mut packages = Vec::new();
    let mut index = 0;
    while rest.get(index).map(String::as_str) == Some("-p") {
        let package = rest
            .get(index + 1)
            .context("Missing package name after '-p'")?;
        packages.push(package.clone());
        index += 2;
    }

    if packages.is_empty() {
        let package = rest.first().context("Missing the package to bundle")?;
        packages.push(package.clone());
    }

    Ok(packages)
}

/// The directory the cargo build puts its artefacts in.
fn target_directory() -> Result<PathBuf> {
    // The bundler changed the working directory to the root of the workspace,
    // so the workspace's manifest is the one this finds.
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .exec()
        .context("Could not parse the cargo metadata of the workspace")?;

    Ok(metadata.target_directory.into_std_path_buf())
}

/// The name of the bundle of a package, as the bundler's configuration says.
fn bundle_names() -> Result<HashMap<String, String>> {
    let path = Path::new("bundler.toml");
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let contents =
        fs::read_to_string(path).with_context(|| format!("Could not read '{}'", path.display()))?;
    let configuration: HashMap<String, toml::Value> = toml::from_str(&contents)
        .with_context(|| format!("Could not parse '{}'", path.display()))?;

    Ok(configuration
        .into_iter()
        .filter_map(|(package, value)| {
            let name = value.get("name")?.as_str()?.to_owned();
            Some((package, name))
        })
        .collect())
}

/// Writes the VST3 module information file into the bundle of every bundled
/// package.
fn write_module_info(packages: &[String]) -> Result<()> {
    let bundled = target_directory()?.join("bundled");
    let names = bundle_names()?;

    let mut written = 0;
    for package in packages {
        let name = names.get(package).map_or(package.clone(), Clone::clone);
        let bundle = bundled.join(format!("{name}.vst3"));
        if !bundle.is_dir() {
            // The package does not have a VST3 bundle, so there is nothing to
            // write a module information file into.
            continue;
        }

        let resources = bundle.join("Contents").join("Resources");
        fs::create_dir_all(&resources)
            .with_context(|| format!("Could not create '{}'", resources.display()))?;

        let path = resources.join("moduleinfo.json");
        fs::write(&path, module_info())
            .with_context(|| format!("Could not write '{}'", path.display()))?;

        eprintln!(
            "Wrote the VST3 module information file at '{}'",
            path.display()
        );

        // The bundler signs the bundle on macOS, and the signature covers the
        // resources of the bundle, so the bundle is signed again now that the
        // module information file is in it.
        #[cfg(target_os = "macos")]
        nice_plug_xtask::maybe_codesign(
            &bundle,
            nice_plug_xtask::CompilationTarget::MacOS(nice_plug_xtask::Architecture::X86_64),
        );

        written += 1;
    }

    if written == 0 {
        anyhow::bail!("Could not find a VST3 bundle to write the module information file into");
    }

    Ok(())
}

/// The contents of the module information file.
fn module_info() -> String {
    let module_info = ModuleInfo {
        name: <RipplerX as Plugin>::NAME,
        version: <RipplerX as Plugin>::VERSION,
        factory_info: FactoryInfo {
            vendor: <RipplerX as Plugin>::VENDOR,
            url: <RipplerX as Plugin>::URL,
            email: <RipplerX as Plugin>::EMAIL,
            flags: Flags {
                unicode: true,
                classes_discardable: false,
                component_non_discardable: false,
            },
        },
        classes: vec![ClassInfo {
            cid: class_id_hex(&<RipplerX as Vst3Plugin>::VST3_CLASS_ID),
            category: CATEGORY,
            name: <RipplerX as Plugin>::NAME,
            vendor: <RipplerX as Plugin>::VENDOR,
            version: <RipplerX as Plugin>::VERSION,
            sdk_version: SDK_VERSION,
            sub_categories: <RipplerX as Vst3Plugin>::VST3_SUBCATEGORIES
                .iter()
                .map(|sub_category| sub_category.as_str())
                .collect(),
            class_flags: CLASS_FLAGS,
            cardinality: CARDINALITY,
            snapshots: Vec::new(),
        }],
    };

    let mut json =
        serde_json::to_string_pretty(&module_info).expect("The module information is JSON");
    json.push('\n');

    json
}

/// The class ID as the hexadecimal string the module information file holds.
///
/// Steinberg's tools write the class ID as the raw bytes in upper case
/// hexadecimal.
fn class_id_hex(class_id: &[u8; 16]) -> String {
    let mut hex = String::with_capacity(32);
    for byte in class_id {
        hex.push_str(&format!("{byte:02X}"));
    }

    hex
}

/// The module information file.
#[derive(Serialize)]
struct ModuleInfo {
    #[serde(rename = "Name")]
    name: &'static str,
    #[serde(rename = "Version")]
    version: &'static str,
    #[serde(rename = "Factory Info")]
    factory_info: FactoryInfo,
    #[serde(rename = "Classes")]
    classes: Vec<ClassInfo>,
}

/// The factory information of the module.
#[derive(Serialize)]
struct FactoryInfo {
    #[serde(rename = "Vendor")]
    vendor: &'static str,
    #[serde(rename = "URL")]
    url: &'static str,
    #[serde(rename = "E-Mail")]
    email: &'static str,
    #[serde(rename = "Flags")]
    flags: Flags,
}

/// The flags of the factory.
#[derive(Serialize)]
struct Flags {
    #[serde(rename = "Unicode")]
    unicode: bool,
    #[serde(rename = "Classes Discardable")]
    classes_discardable: bool,
    #[serde(rename = "Component Non Discardable")]
    component_non_discardable: bool,
}

/// A class of the module.
#[derive(Serialize)]
struct ClassInfo {
    #[serde(rename = "CID")]
    cid: String,
    #[serde(rename = "Category")]
    category: &'static str,
    #[serde(rename = "Name")]
    name: &'static str,
    #[serde(rename = "Vendor")]
    vendor: &'static str,
    #[serde(rename = "Version")]
    version: &'static str,
    #[serde(rename = "SDKVersion")]
    sdk_version: &'static str,
    #[serde(rename = "Sub Categories")]
    sub_categories: Vec<&'static str>,
    #[serde(rename = "Class Flags")]
    class_flags: u32,
    #[serde(rename = "Cardinality")]
    cardinality: u32,
    #[serde(rename = "Snapshots")]
    snapshots: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_information_file_is_valid_json() {
        let json = module_info();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("The module information is JSON");
        assert_eq!(parsed["Name"], "RipplerX");
    }

    #[test]
    fn the_module_information_file_holds_the_canonical_class_id() {
        let json = module_info();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("The module information is JSON");
        assert_eq!(
            parsed["Classes"][0]["CID"],
            "ABCDEF019182FAEB54696C7252697078"
        );
    }

    #[test]
    fn the_class_id_is_written_in_upper_case_hexadecimal() {
        assert_eq!(
            class_id_hex(&<RipplerX as Vst3Plugin>::VST3_CLASS_ID),
            "ABCDEF019182FAEB54696C7252697078"
        );
    }

    /// The canonical class ID, the way the module information file spells it,
    /// and the class ID of the platform, the way the factory reports it.
    #[test]
    fn the_canonical_and_the_platform_class_id_are_different_on_windows() {
        let canonical = class_id_hex(&<RipplerX as Vst3Plugin>::VST3_CLASS_ID);
        let platform = class_id_hex(&<RipplerX as Vst3Plugin>::PLATFORM_VST3_CLASS_ID);
        println!("canonical class ID: {canonical}");
        println!("platform class ID:   {platform}");

        // The canonical class ID is the one the JUCE build's module information
        // file holds, on every platform.
        assert_eq!(canonical, "ABCDEF019182FAEB54696C7252697078");

        #[cfg(target_os = "windows")]
        {
            // Windows reads the first two words of the class ID least
            // significant byte first, so the factory reports a different class
            // ID there.
            assert_ne!(canonical, platform);
            assert_eq!(platform, "01EFCDAB8291EBFA54696C7252697078");
        }

        #[cfg(not(target_os = "windows"))]
        assert_eq!(canonical, platform);
    }
}
