//! Versioned, asset-free evidence manifests for compatibility work.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const EVIDENCE_SCHEMA: &str = "keygen.evidence.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceManifest {
    pub schema: String,
    pub target: String,
    pub source_build: SourceBuild,
    pub observations: Vec<Observation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBuild {
    pub platform: String,
    pub build_id: String,
    pub recovery_format: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub id: String,
    pub source: String,
    pub kind: ObservationKind,
    pub result: ObservationResult,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub facts: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    AssetReuse,
    Behavior,
    Render,
    Schema,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationResult {
    Confirmed,
    Inconclusive,
    Rejected,
}

impl EvidenceManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != EVIDENCE_SCHEMA {
            return Err(format!("unsupported evidence schema: {}", self.schema));
        }
        if self.target.is_empty()
            || self.source_build.platform.is_empty()
            || self.source_build.build_id.is_empty()
        {
            return Err("evidence target and source identity are required".into());
        }
        for observation in &self.observations {
            if observation.id.is_empty() || observation.source.is_empty() {
                return Err("evidence observations require id and source".into());
            }
            if observation.facts.keys().any(|key| is_local_key(key)) {
                return Err(format!(
                    "evidence fact key is not portable: {}",
                    observation.id
                ));
            }
        }
        Ok(())
    }
}

fn is_local_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("path") || key.contains("home") || key.contains("token") || key.contains("secret")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> EvidenceManifest {
        EvidenceManifest {
            schema: EVIDENCE_SCHEMA.into(),
            target: "kg_ddlc_plus".into(),
            source_build: SourceBuild {
                platform: "macos-arm64".into(),
                build_id: "10766092".into(),
                recovery_format: "AssetRipper ExportedProject".into(),
            },
            observations: vec![Observation {
                id: "bios-font".into(),
                source: "font/ModernDOS8x16.ttf".into(),
                kind: ObservationKind::AssetReuse,
                result: ObservationResult::Confirmed,
                facts: BTreeMap::from([(String::from("sha256"), String::from("abc"))]),
            }],
        }
    }

    #[test]
    fn manifest_round_trips_and_validates() {
        let manifest = sample();
        manifest.validate().unwrap();
        let encoded = serde_json::to_string(&manifest).unwrap();
        let decoded: EvidenceManifest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn unknown_fields_and_local_facts_fail() {
        let unknown = serde_json::from_str::<EvidenceManifest>(
            r#"{"schema":"keygen.evidence.v1","target":"x","source_build":{"platform":"macos","build_id":"1","recovery_format":"x"},"observations":[],"extra":1}"#,
        );
        assert!(unknown.is_err());
        let mut manifest = sample();
        manifest.observations[0]
            .facts
            .insert("absolute_path".into(), "private".into());
        assert!(manifest.validate().is_err());
    }
}
