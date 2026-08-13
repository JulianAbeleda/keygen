//! Metadata-only bridge from the adapter catalog to the generic KeyGen project.
//!
//! This is intentionally a declaration compiler: it carries logical paths and
//! hashes into `keygen.project.v1`, but never reads or embeds source bytes.

use super::{content::ContentManifest, locales::LocaleMetadata, story::StoryMetadata};
use crate::assets::AssetCatalog;
use keygen_engine::project::{
    PersistenceConfig, ProjectAsset, ProjectIdentity, ProjectManifest, ProjectScene, ProjectStory,
    Viewport, SCHEMA,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub struct ProjectCompileInput<'a> {
    pub identity: ProjectIdentity,
    pub viewport: Viewport,
    pub persistence: PersistenceConfig,
    pub catalog: &'a AssetCatalog,
    pub stories: &'a [StoryMetadata],
    pub locales: &'a [LocaleMetadata],
    pub content: &'a ContentManifest,
}

/// File-shaped input accepted by the adapter CLI. It contains metadata only.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMetadataFile {
    pub identity: ProjectIdentity,
    pub viewport: Viewport,
    #[serde(default)]
    pub persistence: PersistenceConfig,
    pub catalog: AssetCatalog,
    pub stories: Vec<StoryMetadata>,
    pub locales: Vec<LocaleMetadata>,
    pub content: ContentManifest,
}

pub fn compile_project_file(file: &ProjectMetadataFile) -> Result<ProjectManifest, Vec<String>> {
    compile_project_manifest(ProjectCompileInput {
        identity: file.identity.clone(),
        viewport: file.viewport.clone(),
        persistence: file.persistence.clone(),
        catalog: &file.catalog,
        stories: &file.stories,
        locales: &file.locales,
        content: &file.content,
    })
}

/// Compile adapter metadata into a title-neutral project declaration.
pub fn compile_project_manifest(
    input: ProjectCompileInput<'_>,
) -> Result<ProjectManifest, Vec<String>> {
    let mut errors = Vec::new();
    if let Err(error) = input.catalog.validate() {
        errors.push(format!("asset provenance: {error}"));
    }
    if let Err(error) = input.content.validate() {
        errors.push(format!("content reachability: {error}"));
    }
    if let Err(locale_errors) = super::content::validate_locales(input.locales) {
        errors.extend(
            locale_errors
                .into_iter()
                .map(|e| format!("localization: {e}")),
        );
    }

    let declared: BTreeSet<_> = input.content.nodes.iter().collect();
    for asset in &input.catalog.assets {
        if !declared.contains(&asset.logical_id) {
            errors.push(format!(
                "asset {} is absent from content graph",
                asset.logical_id
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut assets: Vec<_> = input
        .catalog
        .assets
        .iter()
        .map(|asset| ProjectAsset {
            id: asset.logical_id.clone(),
            kind: asset.kind.clone(),
            logical_path: asset.blob.clone(),
            sha256: asset.output_sha256.clone(),
        })
        .collect();
    assets.sort_by(|a, b| a.id.cmp(&b.id));

    let asset_ids: BTreeSet<_> = assets.iter().map(|asset| asset.id.clone()).collect();
    let mut scenes = Vec::new();
    for root in &input.content.roots {
        let scene_assets = input
            .content
            .references
            .iter()
            .filter(|reference| reference.from == *root && asset_ids.contains(&reference.to))
            .map(|reference| reference.to.clone())
            .collect();
        scenes.push(ProjectScene {
            id: root.clone(),
            asset_ids: scene_assets,
        });
    }
    scenes.sort_by(|a, b| a.id.cmp(&b.id));

    let story = input.stories.first().map(|first| {
        let mut labels: Vec<_> = input
            .stories
            .iter()
            .flat_map(|story| story.labels.iter().cloned())
            .collect();
        labels.sort();
        labels.dedup();
        ProjectStory {
            entry: first
                .labels
                .first()
                .cloned()
                .unwrap_or_else(|| first.id.clone()),
            labels,
        }
    });
    let manifest = ProjectManifest {
        schema: SCHEMA.into(),
        project: input.identity,
        viewport: input.viewport,
        assets,
        scenes,
        routes: Vec::new(),
        story,
        persistence: input.persistence,
    };
    manifest.validate().map_err(|error| vec![error])?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetCatalog, AssetRecord, ImportMode};
    use crate::import::content::{compile_content_manifest, ContentReference};

    #[test]
    fn compiles_logical_assets_without_bytes() {
        let mut catalog = AssetCatalog::new();
        catalog.blobs.push("blobs/a".into());
        catalog.assets.push(AssetRecord {
            logical_id: "sprite.a".into(),
            kind: "image".into(),
            source_sha256: "a".repeat(64),
            output_sha256: "b".repeat(64),
            import_mode: ImportMode::Copy,
            importer_version: "test".into(),
            blob: "blobs/a".into(),
            image: None,
        });
        let stories = vec![StoryMetadata {
            id: "story".into(),
            character: None,
            style: None,
            audio_table: None,
            blocks: vec![],
            labels: vec!["start".into()],
            descriptor_variants: vec![],
        }];
        let content = compile_content_manifest(
            &catalog,
            &stories,
            &[],
            &["story".into()],
            &[
                ContentReference {
                    from: "story".into(),
                    to: "sprite.a".into(),
                    kind: "asset".into(),
                },
                ContentReference {
                    from: "story".into(),
                    to: "start".into(),
                    kind: "label".into(),
                },
            ],
        )
        .unwrap();
        let project = compile_project_manifest(ProjectCompileInput {
            identity: ProjectIdentity {
                id: "example".into(),
                display_name: "Example".into(),
                version: "1".into(),
            },
            viewport: Viewport {
                width: 1,
                height: 1,
            },
            persistence: Default::default(),
            catalog: &catalog,
            stories: &stories,
            locales: &[],
            content: &content,
        })
        .unwrap();
        assert_eq!(project.assets[0].logical_path, "blobs/a");
        assert_eq!(project.schema, SCHEMA);
    }
}
