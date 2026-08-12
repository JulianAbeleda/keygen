//! Reachability and localization validation over imported metadata only.
//!
//! The checker consumes logical identifiers and hashes; it never embeds or
//! reconstructs recovered game content.  This makes it useful both for a
//! player-owned import and for synthetic CI fixtures.

use super::{locales::LocaleMetadata, story::StoryMetadata};
use crate::assets::AssetCatalog;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentReference {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReachabilityReport {
    pub schema: String,
    pub roots: Vec<String>,
    pub reachable: Vec<String>,
    pub unreachable: Vec<String>,
    pub dangling: Vec<ContentReference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentManifest {
    pub schema: String,
    pub roots: Vec<String>,
    pub nodes: Vec<String>,
    pub assets: Vec<String>,
    pub locales: Vec<String>,
    pub stories: Vec<String>,
    pub references: Vec<ContentReference>,
    pub reachability: ReachabilityReport,
    pub package_sha256: String,
}

impl ContentManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "kg_ddlc_plus.content.v1" {
            return Err("unsupported content manifest schema".into());
        }
        for values in [
            &self.roots,
            &self.nodes,
            &self.assets,
            &self.locales,
            &self.stories,
        ] {
            if !is_sorted_unique(values) {
                return Err("content manifest identifiers must be sorted and unique".into());
            }
        }
        self.reachability.validate()?;
        if self.reachability.reachable != self.nodes {
            return Err("content manifest must contain only reachable nodes".into());
        }
        if !self.reachability.unreachable.is_empty() {
            return Err("content manifest contains unreachable nodes".into());
        }
        if !self.reachability.dangling.is_empty() {
            return Err("content manifest contains dangling references".into());
        }
        if !is_sha256(&self.package_sha256) {
            return Err("invalid content manifest package hash".into());
        }
        Ok(())
    }
    pub fn write(&self, path: &std::path::Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| format!("encode manifest: {e}"))?;
        std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
    }
}

pub fn compile_content_manifest(
    catalog: &AssetCatalog,
    stories: &[StoryMetadata],
    locales: &[LocaleMetadata],
    roots: &[String],
    references: &[ContentReference],
) -> Result<ContentManifest, Vec<String>> {
    let mut errors = Vec::new();
    if let Err(error) = catalog.validate() {
        errors.push(error);
    }
    if let Err(locale_errors) = validate_locales(locales) {
        errors.extend(locale_errors);
    }
    let assets = imported_asset_ids(catalog);
    let story_ids = story_reference_ids(stories);
    let mut locale_ids: Vec<String> = locales.iter().map(|l| l.locale.clone()).collect();
    locale_ids.sort();
    locale_ids.dedup();
    let mut nodes = assets.clone();
    nodes.extend(story_ids);
    nodes.extend(locale_ids.clone());
    nodes.sort();
    nodes.dedup();
    let report = reachable_content(roots, &nodes, references);
    if roots.iter().any(|root| !nodes.contains(root)) {
        errors.push("root references unknown content".into());
    }
    if !report.dangling.is_empty() {
        errors.push("content graph contains dangling references".into());
    }
    if !report.unreachable.is_empty() {
        errors.push("content graph contains unreachable nodes".into());
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut stories_out: Vec<String> = stories.iter().map(|s| s.id.clone()).collect();
    stories_out.sort();
    stories_out.dedup();
    let mut manifest = ContentManifest {
        schema: "kg_ddlc_plus.content.v1".into(),
        roots: report.roots.clone(),
        nodes: report.reachable.clone(),
        assets,
        locales: locale_ids,
        stories: stories_out,
        references: references.to_vec(),
        reachability: report,
        package_sha256: String::new(),
    };
    let mut unsigned = manifest.clone();
    unsigned.package_sha256.clear();
    let bytes =
        serde_json::to_vec(&unsigned).map_err(|e| vec![format!("encode manifest hash: {e}")])?;
    manifest.package_sha256 = format!("{:x}", Sha256::digest(bytes));
    Ok(manifest)
}

impl ReachabilityReport {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "kg_ddlc_plus.reachability.v1" {
            return Err("unsupported reachability schema".into());
        }
        if !is_sorted_unique(&self.reachable)
            || !is_sorted_unique(&self.unreachable)
            || !is_sorted_unique(&self.roots)
        {
            return Err("reachability identifiers must be sorted and unique".into());
        }
        if self
            .reachable
            .iter()
            .any(|id| self.unreachable.contains(id))
        {
            return Err("content cannot be both reachable and unreachable".into());
        }
        Ok(())
    }
}

/// Compute a strict graph closure. Every reference target must be an imported
/// logical identifier or is reported as dangling; no implicit edges are made.
pub fn reachable_content(
    roots: &[String],
    nodes: &[String],
    references: &[ContentReference],
) -> ReachabilityReport {
    let node_set: BTreeSet<_> = nodes.iter().cloned().collect();
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut dangling = Vec::new();
    for reference in references {
        if node_set.contains(&reference.from) && node_set.contains(&reference.to) {
            adjacency
                .entry(reference.from.clone())
                .or_default()
                .push(reference.to.clone());
        } else {
            dangling.push(reference.clone());
        }
    }
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<_> = roots
        .iter()
        .filter(|id| node_set.contains(*id))
        .cloned()
        .collect();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(next) = adjacency.get(&id) {
            queue.extend(next.iter().cloned());
        }
    }
    let mut reachable: Vec<_> = seen.into_iter().collect();
    let mut unreachable: Vec<_> = node_set
        .difference(&reachable.iter().cloned().collect())
        .cloned()
        .collect();
    reachable.sort();
    unreachable.sort();
    dangling.sort_by(|a, b| (&a.from, &a.to, &a.kind).cmp(&(&b.from, &b.to, &b.kind)));
    let mut roots = roots.to_vec();
    roots.sort();
    roots.dedup();
    ReachabilityReport {
        schema: "kg_ddlc_plus.reachability.v1".into(),
        roots,
        reachable,
        unreachable,
        dangling,
    }
}

/// Validate locale fallback chains and references without inspecting string bytes.
pub fn validate_locales(locales: &[LocaleMetadata]) -> Result<(), Vec<String>> {
    let known: BTreeSet<_> = locales
        .iter()
        .map(|locale| locale.locale.as_str())
        .collect();
    let mut errors = Vec::new();
    for locale in locales {
        let mut ids = BTreeSet::new();
        for entry in &locale.strings {
            if entry.id.is_empty() || !ids.insert(&entry.id) {
                errors.push(format!("{}: duplicate or empty string id", locale.locale));
            }
        }
        for fallback in fallback_chain(locale, locales) {
            if !known.contains(fallback.as_str()) {
                errors.push(format!("{}: missing fallback {fallback}", locale.locale));
            }
        }
        if fallback_chain(locale, locales).contains(&locale.locale) {
            errors.push(format!("{}: fallback cycle", locale.locale));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn fallback_chain(locale: &LocaleMetadata, locales: &[LocaleMetadata]) -> Vec<String> {
    let by_id: BTreeMap<_, _> = locales
        .iter()
        .map(|item| (item.locale.as_str(), item))
        .collect();
    let mut result = Vec::new();
    let mut current = locale.fallback.clone();
    let mut seen = BTreeSet::new();
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            result.push(id);
            break;
        }
        result.push(id.clone());
        current = by_id
            .get(id.as_str())
            .and_then(|item| item.fallback.clone());
    }
    result
}

pub fn imported_asset_ids(catalog: &AssetCatalog) -> Vec<String> {
    let mut ids: Vec<_> = catalog
        .assets
        .iter()
        .map(|asset| asset.logical_id.clone())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

pub fn story_reference_ids(stories: &[StoryMetadata]) -> Vec<String> {
    let mut ids = Vec::new();
    for story in stories {
        ids.push(story.id.clone());
        ids.extend(story.blocks.iter().cloned());
        ids.extend(story.labels.iter().cloned());
        ids.extend(story.descriptor_variants.iter().cloned());
    }
    ids.sort();
    ids.dedup();
    ids
}

fn is_sorted_unique(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::locales::StringEntry;

    #[test]
    fn closure_reports_unreachable_and_dangling() {
        let report = reachable_content(
            &["root".into()],
            &["root".into(), "child".into(), "orphan".into()],
            &[
                ContentReference {
                    from: "root".into(),
                    to: "child".into(),
                    kind: "scene".into(),
                },
                ContentReference {
                    from: "child".into(),
                    to: "missing".into(),
                    kind: "sprite".into(),
                },
            ],
        );
        assert_eq!(report.reachable, vec!["child", "root"]);
        assert_eq!(report.unreachable, vec!["orphan"]);
        assert_eq!(report.dangling.len(), 1);
        assert!(report.validate().is_ok());
    }

    #[test]
    fn locale_fallbacks_are_checked() {
        let en = LocaleMetadata {
            locale: "en".into(),
            strings: vec![StringEntry {
                id: "title".into(),
                value_hash: "h".into(),
            }],
            sprite_ids: vec![],
            font_ids: vec![],
            bundle_variants: vec![],
            fallback: None,
        };
        let ja = LocaleMetadata {
            locale: "ja".into(),
            strings: vec![],
            sprite_ids: vec![],
            font_ids: vec![],
            bundle_variants: vec![],
            fallback: Some("en".into()),
        };
        assert!(validate_locales(&[en, ja]).is_ok());
        let bad = LocaleMetadata {
            locale: "fr".into(),
            strings: vec![],
            sprite_ids: vec![],
            font_ids: vec![],
            bundle_variants: vec![],
            fallback: Some("missing".into()),
        };
        assert!(validate_locales(&[bad]).is_err());
    }
}
