//! Product adapter boundary for KeyGen.
//!
//! `kg_ddlc_plus` owns source discovery, asset conversion, and product identity.
//! The reusable engine only receives the generic [`ProjectManifest`] and asset
//! bytes; no product-specific schema is allowed to cross that boundary.

use keygen_engine::project::{ProjectManifest, SCHEMA as PROJECT_SCHEMA};

/// The minimum handoff from a product adapter to a generic KeyGen host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterHandoff {
    pub target_id: String,
    pub project: ProjectManifest,
}

impl AdapterHandoff {
    pub fn new(target_id: impl Into<String>, project: ProjectManifest) -> Result<Self, String> {
        project.validate()?;
        let target_id = target_id.into();
        if target_id.trim().is_empty() {
            return Err("adapter target id is required".into());
        }
        Ok(Self { target_id, project })
    }

    /// The adapter may identify the product, but the runtime consumes only
    /// this validated generic project document.
    pub fn into_project(self) -> ProjectManifest {
        self.project
    }
}

pub const GENERIC_PROJECT_SCHEMA: &str = PROJECT_SCHEMA;

#[cfg(test)]
mod tests {
    use super::*;
    use keygen_engine::project::{PersistenceConfig, ProjectIdentity, Viewport};

    fn generic_project() -> ProjectManifest {
        ProjectManifest {
            schema: PROJECT_SCHEMA.into(),
            project: ProjectIdentity {
                id: "sample.adapter-project".into(),
                display_name: "Sample Project".into(),
                version: "0.1.0".into(),
            },
            viewport: Viewport {
                width: 1280,
                height: 720,
            },
            assets: vec![],
            scenes: vec![],
            story: None,
            persistence: PersistenceConfig::default(),
        }
    }

    #[test]
    fn handoff_accepts_only_valid_generic_project() {
        let handoff = AdapterHandoff::new("example.adapter", generic_project()).unwrap();
        assert_eq!(handoff.project.schema, GENERIC_PROJECT_SCHEMA);
        assert_eq!(handoff.into_project().project.id, "sample.adapter-project");
    }

    #[test]
    fn handoff_rejects_empty_target() {
        assert!(AdapterHandoff::new(" ", generic_project()).is_err());
    }
}
