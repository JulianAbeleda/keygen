//! Read-only, local source adapters for the private DDLC Plus compatibility target.
//!
//! The adapter deliberately returns logical paths and hashes only.  It never copies
//! source content into the repository and never serializes an operator's absolute path.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

pub const SCHEMA: &str = "kg_ddlc_plus.source.v1";
pub const APP_ID: u32 = 1_388_880;
pub const BUILD_ID: u64 = 10_766_092;
pub const UNITY_VERSION: &str = "2019.4.20f1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFingerprint {
    pub schema: String,
    pub app_id: u32,
    pub build_id: u64,
    pub unity_version: String,
    pub recovery_format: String,
    pub files: Vec<SourceFile>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceObject {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Imported,
    UnreachableWithProof,
    Excluded,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassifiedObject {
    pub object_id: String,
    pub classification: Classification,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub schema: String,
    pub objects: Vec<SourceObject>,
    pub counts: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceGraph {
    pub schema: String,
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub dangling: Vec<String>,
}

pub fn discover_steam_app(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let candidates = if let Some(path) = explicit {
        vec![path.to_path_buf()]
    } else {
        let home = env::var_os("HOME").ok_or("source.not_found: HOME is unavailable")?;
        vec![PathBuf::from(home).join(
            "Library/Application Support/Steam/steamapps/common/Doki Doki Literature Club Plus",
        )]
    };
    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }
    Err("source.not_found: DDLC Plus Steam app was not found".into())
}

pub fn exported_project(explicit: &Path) -> Result<SourceFingerprint, String> {
    if !explicit.is_dir() {
        return Err("source.incomplete: ExportedProject directory is missing".into());
    }
    let version = fs::read_to_string(explicit.join("ProjectSettings/ProjectVersion.txt"))
        .map_err(|_| "source.incomplete: ProjectVersion.txt is missing".to_owned())?;
    if !version.lines().any(|line| line.contains(UNITY_VERSION)) {
        return Err(format!(
            "source.version_mismatch: expected Unity {UNITY_VERSION}"
        ));
    }
    let required = [
        "Assets/TextAsset/bios.txt",
        "Assets/TextAsset/bootlog.txt",
        "Assets/Font/ModernDOS8x16.ttf",
    ];
    for path in required {
        if !explicit.join(path).is_file() {
            return Err(format!("source.incomplete: required logical file {path}"));
        }
    }
    let mut files = Vec::new();
    collect_files(explicit, explicit, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(SourceFingerprint {
        schema: SCHEMA.into(),
        app_id: APP_ID,
        build_id: BUILD_ID,
        unity_version: UNITY_VERSION.into(),
        recovery_format: "AssetRipper ExportedProject".into(),
        files,
    })
}

pub fn inventory(fingerprint: &SourceFingerprint) -> Inventory {
    let mut counts = BTreeMap::new();
    let objects = fingerprint
        .files
        .iter()
        .map(|file| {
            *counts.entry(file.kind.clone()).or_insert(0) += 1;
            SourceObject {
                // Logical paths are the identity; hashes are content evidence.  This
                // prevents two different files with identical bytes colliding.
                id: format!("path:{}", file.path),
                path: file.path.clone(),
                kind: file.kind.clone(),
                sha256: file.sha256.clone(),
            }
        })
        .collect();
    Inventory {
        schema: "kg_ddlc_plus.inventory.v1".into(),
        objects,
        counts,
    }
}

pub fn reference_graph(inv: &Inventory) -> ReferenceGraph {
    let nodes: Vec<_> = inv.objects.iter().map(|o| o.id.clone()).collect();
    // AssetRipper's exported files are an inventory input, not a parsed
    // serialized-object graph. Do not invent relationships: later adapters add
    // edges only when a source format parser proves them.
    let edges = Vec::new();
    ReferenceGraph {
        schema: "kg_ddlc_plus.references.v1".into(),
        nodes,
        edges,
        dangling: Vec::new(),
    }
}

pub fn classify(inv: &Inventory) -> Vec<ClassifiedObject> {
    inv.objects
        .iter()
        .map(|object| ClassifiedObject {
            object_id: object.id.clone(),
            classification: if object.kind == "unknown" {
                Classification::Blocked
            } else {
                Classification::Imported
            },
            reason: if object.kind == "unknown" {
                "unrecognized source type"
            } else {
                "covered by diagnostic inventory"
            }
            .into(),
        })
        .collect()
}

fn collect_files(root: &Path, current: &Path, out: &mut Vec<SourceFile>) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|_| "source.read_error: cannot enumerate ExportedProject".to_owned())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "source.read_error: cannot enumerate ExportedProject".to_owned())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
            continue;
        }
        let bytes =
            fs::read(&path).map_err(|_| "source.read_error: cannot read source file".to_owned())?;
        let logical = path
            .strip_prefix(root)
            .map_err(|_| "source.path_error: invalid logical path".to_owned())?
            .to_string_lossy()
            .replace('\\', "/");
        out.push(SourceFile {
            path: logical.clone(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len() as u64,
            kind: kind_for(&logical).into(),
        });
    }
    Ok(())
}

fn kind_for(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" => "texture",
        "ttf" | "otf" => "font",
        "wav" | "mp3" | "ogg" => "audio",
        "prefab" => "prefab",
        "unity" => "scene",
        "anim" => "animation",
        "mat" => "material",
        "asset" => "asset",
        "txt" | "json" => "text",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_kind_and_graph_are_deterministic() {
        let f = SourceFingerprint {
            schema: SCHEMA.into(),
            app_id: APP_ID,
            build_id: BUILD_ID,
            unity_version: UNITY_VERSION.into(),
            recovery_format: "test".into(),
            files: vec![
                SourceFile {
                    path: "Assets/a.png".into(),
                    sha256: "a".into(),
                    bytes: 1,
                    kind: "texture".into(),
                },
                SourceFile {
                    path: "Assets/b.txt".into(),
                    sha256: "b".into(),
                    bytes: 1,
                    kind: "text".into(),
                },
            ],
        };
        let i = inventory(&f);
        assert_eq!(i.counts["texture"], 1);
        assert_eq!(reference_graph(&i), reference_graph(&i));
        assert!(classify(&i)
            .iter()
            .all(|x| x.classification == Classification::Imported));
    }
}
