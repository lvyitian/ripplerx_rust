//! Converts the legacy JUCE patches into the `.ripx` format.
//!
//! ```text
//! rippler-import <legacy.xml> [-o <output.ripx>]
//! rippler-import --presets <directory> [-o <output-directory>]
//! ```
//!
//! Without `-o` the patch is written to the standard output.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rippler_dsp::sampler::Sampler;
use rippler_params::legacy::read_legacy_patch;
use rippler_params::patch::{Patch, convert_legacy_patch, write_patch};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rippler-import: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the converter.
fn run(arguments: &[String]) -> Result<(), String> {
    let mut arguments = arguments.iter().map(String::as_str);

    let Some(first) = arguments.next() else {
        return Err(usage());
    };

    let mut output: Option<PathBuf> = None;
    let mut input: Option<PathBuf> = None;
    let mut presets: Option<PathBuf> = None;

    if first == "--presets" {
        let directory = arguments
            .next()
            .ok_or_else(|| String::from("--presets needs a directory"))?;
        presets = Some(PathBuf::from(directory));
    } else {
        input = Some(PathBuf::from(first));
    }

    while let Some(argument) = arguments.next() {
        match argument {
            "-o" => {
                let path = arguments
                    .next()
                    .ok_or_else(|| String::from("-o needs a path"))?;
                output = Some(PathBuf::from(path));
            }
            other => return Err(format!("unexpected argument {other}\n{}", usage())),
        }
    }

    if let Some(directory) = presets {
        convert_presets(&directory, output.as_deref())
    } else {
        let input = input.ok_or_else(usage)?;
        convert_one(&input, output.as_deref())
    }
}

/// Converts one legacy patch.
fn convert_one(input: &Path, output: Option<&Path>) -> Result<(), String> {
    let patch = read_patch(input)?;
    let json = write_patch(&patch).map_err(|error| error.to_string())?;

    match output {
        Some(path) => {
            std::fs::write(path, json).map_err(|error| format!("{}: {error}", path.display()))?;
        }
        None => println!("{json}"),
    }

    Ok(())
}

/// Converts every legacy patch in a directory.
fn convert_presets(directory: &Path, output: Option<&Path>) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?;

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "xml"))
        .collect();
    paths.sort();

    if let Some(output) = output {
        std::fs::create_dir_all(output)
            .map_err(|error| format!("{}: {error}", output.display()))?;
    }

    for path in &paths {
        let patch = read_patch(path)?;
        let json = write_patch(&patch).map_err(|error| error.to_string())?;
        let name = path
            .file_stem()
            .map(|stem| format!("{}.ripx", stem.to_string_lossy()))
            .unwrap_or_else(|| String::from("patch.ripx"));

        match output {
            Some(directory) => {
                let target = directory.join(name);
                std::fs::write(&target, json)
                    .map_err(|error| format!("{}: {error}", target.display()))?;
            }
            None => println!("{json}"),
        }
    }

    Ok(())
}

/// Reads a legacy patch and converts it.
fn read_patch(path: &Path) -> Result<Patch, String> {
    let xml =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let legacy = read_legacy_patch(&xml).map_err(|error| format!("{}: {error}", path.display()))?;
    let patch = convert_legacy_patch(&legacy);

    if let Some(sample) = &patch.user_sample {
        let mut sampler = Sampler::new();
        sampler.load_encoded(sample);
        if sampler.waveform.is_empty() && !sample.is_empty() {
            return Err(format!(
                "{} has a user sample that does not decode",
                path.display()
            ));
        }
    }

    Ok(patch)
}

/// The usage of the converter.
fn usage() -> String {
    String::from(
        "usage:\n  rippler-import <legacy.xml> [-o <output.ripx>]\n  \
         rippler-import --presets <directory> [-o <output-directory>]",
    )
}
