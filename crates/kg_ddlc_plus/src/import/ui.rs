//! Camera/Canvas/RectTransform/mask/navigation/sorting contract (KGD-127).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UiMetadata {
    pub object_id: String,
    pub canvas_mode: String,
    pub reference_resolution: [u32; 2],
    pub transform: RectTransform,
    pub mask: Option<String>,
    pub navigation: Option<Navigation>,
    pub sorting_layer: String,
    pub sorting_order: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RectTransform {
    pub anchor_min: [f32; 2],
    pub anchor_max: [f32; 2],
    pub pivot: [f32; 2],
    pub position: [f32; 2],
    pub size: [f32; 2],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Navigation {
    pub mode: String,
    pub up: Option<String>,
    pub down: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}
