//! Generic, editor-free project package schema.
//!
//! A host loads this manifest and supplies the referenced bytes to the engine.
//! Nothing in this module names a particular game, title, or asset lineage.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

pub const SCHEMA: &str = "keygen.project.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub schema: String,
    pub project: ProjectIdentity,
    pub viewport: Viewport,
    #[serde(default)]
    pub assets: Vec<ProjectAsset>,
    #[serde(default)]
    pub scenes: Vec<ProjectScene>,
    #[serde(default)]
    pub story: Option<ProjectStory>,
    #[serde(default)]
    pub persistence: PersistenceConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectIdentity {
    pub id: String,
    pub display_name: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAsset {
    pub id: String,
    pub kind: String,
    pub logical_path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectScene {
    pub id: String,
    #[serde(default)]
    pub asset_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectStory {
    pub entry: String,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistenceConfig {
    #[serde(default = "default_namespace")]
    pub namespace: String,
    #[serde(default = "default_schema")]
    pub schema: String,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            namespace: default_namespace(),
            schema: default_schema(),
        }
    }
}
fn default_namespace() -> String {
    "keygen.project".into()
}
fn default_schema() -> String {
    "keygen.project.state.v1".into()
}

impl ProjectManifest {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("decode project: {e}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read project: {e}"))?;
        Self::from_json(&bytes)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!(
                "project schema mismatch: expected {SCHEMA}, got {}",
                self.schema
            ));
        }
        if self.project.id.trim().is_empty() || self.project.version.trim().is_empty() {
            return Err("project id and version are required".into());
        }
        if self.viewport.width == 0 || self.viewport.height == 0 {
            return Err("viewport dimensions must be non-zero".into());
        }
        let mut ids = BTreeSet::new();
        for asset in &self.assets {
            if asset.id.is_empty() || !ids.insert(asset.id.as_str()) {
                return Err(format!("duplicate or empty asset id: {}", asset.id));
            }
            if asset.logical_path.is_empty() || asset.sha256.len() != 64 {
                return Err(format!("invalid asset metadata: {}", asset.id));
            }
        }
        for scene in &self.scenes {
            if scene.id.is_empty() {
                return Err("scene id is empty".into());
            }
            for id in &scene.asset_ids {
                if !ids.contains(id.as_str()) {
                    return Err(format!("scene {} references missing asset {id}", scene.id));
                }
            }
        }
        if let Some(story) = &self.story {
            if story.entry.is_empty() || !story.labels.iter().any(|l| l == &story.entry) {
                return Err("story entry label is missing".into());
            }
        }
        if self.persistence.namespace.trim().is_empty() || self.persistence.schema.trim().is_empty()
        {
            return Err("persistence namespace and schema are required".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ProjectManifest {
        ProjectManifest {
            schema: SCHEMA.into(),
            project: ProjectIdentity {
                id: "sample.project".into(),
                display_name: "Sample Project".into(),
                version: "0.1.0".into(),
            },
            viewport: Viewport {
                width: 1280,
                height: 720,
            },
            assets: vec![ProjectAsset {
                id: "sprite.hero".into(),
                kind: "image".into(),
                logical_path: "assets/hero.png".into(),
                sha256: "a".repeat(64),
            }],
            scenes: vec![ProjectScene {
                id: "scene.start".into(),
                asset_ids: vec!["sprite.hero".into()],
            }],
            story: Some(ProjectStory {
                entry: "start".into(),
                labels: vec!["start".into()],
            }),
            persistence: Default::default(),
        }
    }
    #[test]
    fn generic_fixture_validates() {
        fixture().validate().unwrap();
        let bytes = serde_json::to_vec(&fixture()).unwrap();
        ProjectManifest::from_json(&bytes).unwrap();
    }
    #[test]
    fn missing_asset_is_rejected() {
        let mut p = fixture();
        p.scenes[0].asset_ids.push("missing".into());
        assert!(p.validate().is_err());
    }
}
