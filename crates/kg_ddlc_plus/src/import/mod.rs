//! Metadata-only contracts for translating recovered Unity data into KeyGen.
//!
//! These modules deliberately do not decode or copy game assets. They describe
//! the metadata that an importer must preserve and return an explicit
//! `KGD-002` diagnostic for source features not implemented by this pass.

pub mod animation;
pub mod audio;
pub mod content;
pub mod dependencies;
pub mod fonts;
pub mod images;
pub mod locales;
pub mod materials;
pub mod scene;
pub mod sprites;
pub mod story;
pub mod ui;

use keygen_diagnostics::{Diagnostic, Severity, CODE_UNSUPPORTED, SOURCE_IMPORT};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReport<T> {
    pub records: Vec<T>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> ImportReport<T> {
    pub fn unsupported(feature: &str, detail: &str) -> Diagnostic {
        Diagnostic::new(
            CODE_UNSUPPORTED,
            Severity::Warning,
            SOURCE_IMPORT,
            format!("{feature} is not implemented in the metadata-only importer: {detail}"),
        )
        .field("feature", feature)
    }

    pub fn is_supported(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CODE_UNSUPPORTED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_round_trips_without_asset_bytes() {
        let report = ImportReport {
            records: vec![crate::import::images::ImageMetadata {
                id: "synthetic-image".into(),
                width: 2,
                height: 2,
                channels: 4,
                color_space: "sRGB".into(),
                alpha: crate::import::images::AlphaBounds {
                    min: 0,
                    max: 255,
                    nonzero_pixels: 3,
                },
                decoded_pixel_hash: "synthetic-hash".into(),
            }],
            diagnostics: Vec::new(),
        };
        let encoded = serde_json::to_vec(&report).unwrap();
        let decoded: ImportReport<crate::import::images::ImageMetadata> =
            serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, report);
        assert!(decoded.is_supported());
    }

    #[test]
    fn unsupported_source_detail_is_explicit_and_stable() {
        let diagnostic = ImportReport::<()>::unsupported("shader-graph", "custom pass");
        assert_eq!(diagnostic.code, CODE_UNSUPPORTED);
        assert_eq!(diagnostic.source, SOURCE_IMPORT);
        assert_eq!(
            diagnostic.fields.get("feature").map(String::as_str),
            Some("shader-graph")
        );
        assert!(diagnostic.message.contains("not implemented"));
    }
}
