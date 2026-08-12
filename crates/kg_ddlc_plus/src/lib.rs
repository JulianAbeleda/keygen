#![forbid(unsafe_code)]
//! Private compatibility compiler for a player-owned DDLC Plus recovery.

use keygen_engine::model::SceneSpec;
pub mod adapter;
pub mod assets;
pub mod evidence;
pub mod identity;
pub mod import;
pub mod launcher;
pub mod source;
pub mod state;
pub mod story;
pub mod vn;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Component, Path, PathBuf},
};

const PACKAGE_SCHEMA: &str = "keygen.package.v1";
const TARGET_ID: &str = "kg_ddlc_plus";
const DISPLAY_NAME: &str = "kg_ddlc_plus";
const STEAM_APP_ID: u32 = 1388880;
const STEAM_BUILD_ID: u64 = 10_766_092;
const UNITY_VERSION: &str = "2019.4.20f1";
const DEFAULT_PACKAGE: &str = "local/kg_ddlc_plus";

const SOURCE_FILES: &[SourceExpectation] = &[
    SourceExpectation {
        path: "ProjectSettings/ProjectVersion.txt",
        sha256: "16841e6750c4dd0f075f0642090e800d1113ebacf40d2fd53bab842c3a1bf71a",
    },
    SourceExpectation {
        path: "Assets/TextAsset/bios.txt",
        sha256: "0e4055693233f174132bca1304fef7dfd2bb3c224875b51531a284cdc1b94316",
    },
    SourceExpectation {
        path: "Assets/TextAsset/bootlog.txt",
        sha256: "769e75002bb7e62d9f2a2feda7673c5cd86a74d0ff2624725047e7ec36c6b345",
    },
    SourceExpectation {
        path: "Assets/Font/ModernDOS8x16.ttf",
        sha256: "f79ebf2f0bf038dcfcb4648be794603fac902af132d8730484650f7f88431a4f",
    },
    SourceExpectation {
        path: "Assets/Texture2D/MES Logo bios.png",
        sha256: "8d7dd42a803fbc4972a38648f2f51ff78866875e3de4770acb6fdd304f2ae2f8",
    },
    SourceExpectation {
        path: "Assets/Texture2D/MES Logo bios 2.png",
        sha256: "2f71fc978fa850bb7dedfcb52c13a47e3da264ac5c27d7c0a9040747d212112b",
    },
];

#[derive(Clone, Copy)]
struct SourceExpectation {
    path: &'static str,
    sha256: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageManifest {
    schema: String,
    id: String,
    display_name: String,
    compiler: String,
    source: SourceIdentity,
    entry_scene: String,
    inputs: Vec<FileDigest>,
    artifacts: Vec<FileDigest>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceIdentity {
    steam_app_id: u32,
    steam_build_id: u64,
    unity_version: String,
    recovery_format: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileDigest {
    path: String,
    sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimedLine {
    pub visible_at: f32,
    pub text: String,
}

pub fn parse_timed_lines(text: &str) -> Result<Vec<TimedLine>, String> {
    let mut visible_at = 0.0_f32;
    let mut output = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let close = raw
            .find(']')
            .ok_or_else(|| format!("timed text line {} is missing a delay", index + 1))?;
        if !raw.starts_with('[') {
            return Err(format!("timed text line {} is missing '['", index + 1));
        }
        let delay: f32 = raw[1..close]
            .parse()
            .map_err(|_| format!("timed text line {} has an invalid delay", index + 1))?;
        if !delay.is_finite() || delay < 0.0 {
            return Err(format!(
                "timed text line {} has an invalid delay",
                index + 1
            ));
        }
        let body = raw[close + 1..].trim_end_matches('\r');
        if !body.is_empty() {
            let body = if body == "[HIT_DEL]" {
                "Press DEL to enter SETUP"
            } else {
                body
            };
            output.push(TimedLine {
                visible_at,
                text: body.to_owned(),
            });
        }
        visible_at += delay;
    }
    Ok(output)
}

pub fn discover_source(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Some(path) = env::var_os("KG_DDLC_PLUS_SOURCE") {
        return Ok(PathBuf::from(path));
    }
    if let Some(home) = env::var_os("HOME") {
        let candidate = PathBuf::from(home)
            .join("ddlc-architecture-explorer/unpacked/assetripper-build-10766092/ExportedProject");
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }
    Err("source not found; pass --source PATH or set KG_DDLC_PLUS_SOURCE to the AssetRipper ExportedProject directory".into())
}

pub fn compile_package(source: &Path, package: &Path) -> Result<(), String> {
    let inputs = validate_source(source)?;
    let bios_path = source.join("Assets/TextAsset/bios.txt");
    let bios = fs::read_to_string(&bios_path)
        .map_err(|error| format!("cannot read {}: {error}", bios_path.display()))?;
    let timed = parse_timed_lines(&bios)?;

    fs::create_dir_all(package.join("assets"))
        .and_then(|_| fs::create_dir_all(package.join("scenes")))
        .map_err(|error| format!("cannot create package {}: {error}", package.display()))?;

    copy_file(
        &source.join("Assets/Font/ModernDOS8x16.ttf"),
        &package.join("assets/ModernDOS8x16.ttf"),
    )?;
    copy_file(
        &source.join("Assets/Texture2D/MES Logo bios 2.png"),
        &package.join("assets/mes-logo-bios.png"),
    )?;

    let scene = build_bios_scene(&timed);
    let scene_bytes = serde_json::to_vec_pretty(&scene)
        .map_err(|error| format!("cannot encode scene: {error}"))?;
    SceneSpec::from_json(&scene_bytes)?;
    write_file(&package.join("scenes/bios.json"), &scene_bytes)?;

    let artifacts = [
        "assets/ModernDOS8x16.ttf",
        "assets/mes-logo-bios.png",
        "scenes/bios.json",
    ]
    .into_iter()
    .map(|relative| digest_file(package, relative))
    .collect::<Result<Vec<_>, _>>()?;
    let manifest = PackageManifest {
        schema: PACKAGE_SCHEMA.into(),
        id: TARGET_ID.into(),
        display_name: DISPLAY_NAME.into(),
        compiler: format!("kg-ddlc-plus {}", env!("CARGO_PKG_VERSION")),
        source: SourceIdentity {
            steam_app_id: STEAM_APP_ID,
            steam_build_id: STEAM_BUILD_ID,
            unity_version: UNITY_VERSION.into(),
            recovery_format: "AssetRipper ExportedProject".into(),
        },
        entry_scene: "scenes/bios.json".into(),
        inputs,
        artifacts,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("cannot encode package manifest: {error}"))?;
    write_file(&package.join("package.json"), &manifest_bytes)?;
    validate_package(package).map(|_| ())
}

pub fn validate_package(package: &Path) -> Result<PathBuf, String> {
    let manifest_path = package.join("package.json");
    let bytes = fs::read(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let manifest: PackageManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid package manifest: {error}"))?;
    if manifest.schema != PACKAGE_SCHEMA || manifest.id != TARGET_ID {
        return Err("unsupported package schema or target id".into());
    }
    if manifest.source.steam_app_id != STEAM_APP_ID
        || manifest.source.steam_build_id != STEAM_BUILD_ID
        || manifest.source.unity_version != UNITY_VERSION
    {
        return Err("package source identity does not match the supported recovery".into());
    }
    for artifact in &manifest.artifacts {
        ensure_relative(&artifact.path)?;
        let actual = sha256_file(&package.join(&artifact.path))?;
        if actual != artifact.sha256 {
            return Err(format!("package artifact hash mismatch: {}", artifact.path));
        }
    }
    ensure_relative(&manifest.entry_scene)?;
    let scene = package.join(&manifest.entry_scene);
    keygen_player::load_scene(&scene)?;
    Ok(scene)
}

pub fn run_cli<I, S>(arguments: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut values = arguments
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned());
    let _program = values.next();
    let command = values.next().ok_or_else(usage)?;
    let options: Vec<String> = values.collect();
    match command.as_str() {
        "compile-project" => {
            reject_unknown(&options, &["--metadata", "--output"])?;
            let metadata = required_value(&options, "--metadata")?;
            let output = required_value(&options, "--output")?;
            let bytes = fs::read(&metadata)
                .map_err(|error| format!("cannot read metadata {}: {error}", metadata))?;
            let file: import::project::ProjectMetadataFile = serde_json::from_slice(&bytes)
                .map_err(|error| format!("invalid project metadata: {error}"))?;
            let project =
                import::project::compile_project_file(&file).map_err(|errors| errors.join("; "))?;
            let encoded = serde_json::to_vec_pretty(&project)
                .map_err(|error| format!("encode project manifest: {error}"))?;
            fs::write(&output, encoded)
                .map_err(|error| format!("cannot write project manifest {}: {error}", output))?;
            println!("compiled keygen.project.v1: {output}");
            Ok(())
        }
        "inspect" => {
            reject_unknown(&options, &["--source"])?;
            let source = discover_source(take_path(&options, "--source")?)?;
            validate_source(&source)?;
            println!(
                "source OK: Steam app {STEAM_APP_ID}, build {STEAM_BUILD_ID}, Unity {UNITY_VERSION}"
            );
            Ok(())
        }
        "compile" => {
            reject_unknown(&options, &["--source", "--output"])?;
            let source = discover_source(take_path(&options, "--source")?)?;
            let output =
                take_path(&options, "--output")?.unwrap_or_else(|| PathBuf::from(DEFAULT_PACKAGE));
            compile_package(&source, &output)?;
            println!("compiled {DISPLAY_NAME} package: {}", output.display());
            Ok(())
        }
        "validate" => {
            reject_unknown(&options, &["--package"])?;
            let package =
                take_path(&options, "--package")?.unwrap_or_else(|| PathBuf::from(DEFAULT_PACKAGE));
            validate_package(&package)?;
            println!("package OK: {}", package.display());
            Ok(())
        }
        "render" => {
            reject_unknown(&options, &["--package", "--output", "--time"])?;
            let package =
                take_path(&options, "--package")?.unwrap_or_else(|| PathBuf::from(DEFAULT_PACKAGE));
            let output = required_value(&options, "--output")?;
            let time = option_value(&options, "--time")?.unwrap_or_else(|| "4.25".into());
            let scene = validate_package(&package)?;
            keygen_player::run_cli([
                OsStr::new("keygen"),
                OsStr::new("--scene"),
                scene.as_os_str(),
                OsStr::new("--render"),
                OsStr::new(&output),
                OsStr::new("--time"),
                OsStr::new(&time),
            ])
        }
        "run" => {
            reject_unknown(&options, &["--package", "--smoke-seconds"])?;
            let package =
                take_path(&options, "--package")?.unwrap_or_else(|| PathBuf::from(DEFAULT_PACKAGE));
            let smoke = option_value(&options, "--smoke-seconds")?;
            let scene = validate_package(&package)?;
            let mut args = vec![
                OsString::from("keygen"),
                OsString::from("--scene"),
                scene.into_os_string(),
            ];
            if let Some(seconds) = smoke {
                args.push(OsString::from("--smoke-seconds"));
                args.push(OsString::from(seconds));
            }
            keygen_player::run_cli(args)
        }
        "--help" | "-h" | "help" => Err(usage()),
        _ => Err(format!("unknown command: {command}\n{}", usage())),
    }
}

fn validate_source(source: &Path) -> Result<Vec<FileDigest>, String> {
    if !source.is_dir() {
        return Err(format!(
            "source directory does not exist: {}",
            source.display()
        ));
    }
    let mut inputs = Vec::with_capacity(SOURCE_FILES.len());
    for expected in SOURCE_FILES {
        let path = source.join(expected.path);
        let actual = sha256_file(&path)?;
        if actual != expected.sha256 {
            return Err(format!(
                "unsupported source file {}: expected build {STEAM_BUILD_ID} fingerprint",
                expected.path
            ));
        }
        inputs.push(FileDigest {
            path: expected.path.into(),
            sha256: actual,
        });
    }
    let version = fs::read_to_string(source.join("ProjectSettings/ProjectVersion.txt"))
        .map_err(|error| format!("cannot read project version: {error}"))?;
    if !version.contains(&format!("m_EditorVersion: {UNITY_VERSION}")) {
        return Err(format!(
            "unsupported Unity editor version; expected {UNITY_VERSION}"
        ));
    }
    Ok(inputs)
}

fn build_bios_scene(lines: &[TimedLine]) -> serde_json::Value {
    let text_layers = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            serde_json::json!({
                "id": format!("bios-line-{index}"),
                "text": line.text,
                "x": 48.0,
                "y": 48.0 + index as f32 * 44.0,
                "font_size": 34.0,
                "color": [214, 214, 214, 255],
                "outline": [0, 0, 0, 255],
                "outline_width": 0,
                "visible_at": line.visible_at,
                "characters_per_second": null
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema": "keygen.scene.v1",
        "title": "kg_ddlc_plus",
        "design_width": 1920,
        "design_height": 1080,
        "clear": [0, 0, 0, 255],
        "font_path": "../assets/ModernDOS8x16.ttf",
        "layers": [{
            "id": "mes-logo",
            "path": "../assets/mes-logo-bios.png",
            "x": 1496.0,
            "y": 48.0,
            "scale": 1.0,
            "anchor": "top_left",
            "alpha": 1.0,
            "entrance": null,
            "motion": null
        }],
        "particle_insertions": [],
        "menu_insertion": null,
        "menu": null,
        "text_layers": text_layers,
        "particles": null,
        "fade": null
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn digest_file(base: &Path, relative: &str) -> Result<FileDigest, String> {
    ensure_relative(relative)?;
    Ok(FileDigest {
        path: relative.into(),
        sha256: sha256_file(&base.join(relative))?,
    })
}

fn copy_file(source: &Path, output: &Path) -> Result<(), String> {
    fs::copy(source, output).map(|_| ()).map_err(|error| {
        format!(
            "cannot copy {} to {}: {error}",
            source.display(),
            output.display()
        )
    })
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn ensure_relative(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("package path is not a safe relative path: {value}"));
    }
    Ok(())
}

fn option_value(options: &[String], name: &str) -> Result<Option<String>, String> {
    let mut found = None;
    let mut index = 0;
    while index < options.len() {
        if options[index] == name {
            if found.is_some() {
                return Err(format!("{name} may only be supplied once"));
            }
            found = Some(
                options
                    .get(index + 1)
                    .ok_or_else(|| format!("{name} needs a value"))?
                    .clone(),
            );
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(found)
}

fn take_path(options: &[String], name: &str) -> Result<Option<PathBuf>, String> {
    Ok(option_value(options, name)?.map(PathBuf::from))
}

fn required_value(options: &[String], name: &str) -> Result<String, String> {
    option_value(options, name)?.ok_or_else(|| format!("{name} is required"))
}

fn reject_unknown(options: &[String], names: &[&str]) -> Result<(), String> {
    let mut index = 0;
    while index < options.len() {
        if !names.contains(&options[index].as_str()) {
            return Err(format!("unknown option: {}", options[index]));
        }
        if index + 1 >= options.len() {
            return Err(format!("{} needs a value", options[index]));
        }
        index += 2;
    }
    Ok(())
}

fn usage() -> String {
    "usage:\n  kg-ddlc-plus inspect [--source PATH]\n  kg-ddlc-plus compile [--source PATH] [--output DIR]\n  kg-ddlc-plus validate [--package DIR]\n  kg-ddlc-plus render [--package DIR] --output PNG [--time SECONDS]\n  kg-ddlc-plus run [--package DIR] [--smoke-seconds SECONDS]".into()
}

#[cfg(test)]
mod tests {
    use super::{ensure_relative, parse_timed_lines};

    #[test]
    fn parses_recovered_timing_format() {
        let lines =
            parse_timed_lines("[0.02]\n[1.0]8:33\n[0.0]MEMORY OK\n[0.0][HIT_DEL]\n").unwrap();
        assert_eq!(lines.len(), 3);
        assert!((lines[0].visible_at - 0.02).abs() < f32::EPSILON);
        assert_eq!(lines[2].text, "Press DEL to enter SETUP");
    }

    #[test]
    fn rejects_malformed_timing() {
        assert!(parse_timed_lines("MEMORY OK").is_err());
        assert!(parse_timed_lines("[-1.0]MEMORY OK").is_err());
    }

    #[test]
    fn package_paths_cannot_escape() {
        assert!(ensure_relative("scenes/bios.json").is_ok());
        assert!(ensure_relative("../private.bin").is_err());
        assert!(ensure_relative("/private.bin").is_err());
    }
}
