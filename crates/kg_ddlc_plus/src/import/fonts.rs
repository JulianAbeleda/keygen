//! Font/TextMeshPro metadata contract (KGD-123).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FontMetadata {
    pub id: String,
    pub face: String,
    pub glyph_count: u32,
    pub line_height: f32,
    pub ascent: f32,
    pub descent: f32,
    pub kerning_pairs: u32,
    pub fallbacks: Vec<String>,
    pub sprite_font_mappings: Vec<String>,
    pub byte_hash: String,
}
