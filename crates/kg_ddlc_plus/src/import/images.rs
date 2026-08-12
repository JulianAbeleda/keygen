//! Texture/flat-image metadata contract (KGD-121).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageMetadata {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub color_space: String,
    pub alpha: AlphaBounds,
    pub decoded_pixel_hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AlphaBounds {
    pub min: u8,
    pub max: u8,
    pub nonzero_pixels: u64,
}
