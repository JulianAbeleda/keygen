//! Animation/controller metadata contract (KGD-125).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationClipMetadata {
    pub id: String,
    pub duration_ms: u64,
    pub bindings: Vec<Binding>,
    pub wrap: String,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub path: String,
    pub property: String,
    pub keyframes: u32,
    pub tangent: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub time_ms: u64,
    pub function: String,
}
