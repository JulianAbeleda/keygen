//! Reachability and localization validation over imported metadata only.
//!
//! The checker consumes logical identifiers and hashes; it never embeds or
//! reconstructs recovered game content.  This makes it useful both for a
//! player-owned import and for synthetic CI fixtures.

use super::{locales::LocaleMetadata, story::StoryMetadata};
use crate::assets::AssetCatalog;
use serde::{Deserialize, Serialize};
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
