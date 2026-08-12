//! Typed story descriptor contract (KGD-130).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoryMetadata {
    pub id: String,
    pub character: Option<String>,
    pub style: Option<String>,
    pub audio_table: Option<String>,
    pub blocks: Vec<String>,
    pub labels: Vec<String>,
    pub descriptor_variants: Vec<String>,
}
