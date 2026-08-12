//! Sprite/atlas metadata contract (KGD-122).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteMetadata {
    pub id: String,
    pub texture_id: String,
    pub rect: Rect,
    pub pivot: [f32; 2],
    pub pixels_per_unit: f32,
    pub border: [f32; 4],
    pub mesh: String,
    pub order: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
