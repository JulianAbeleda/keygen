//! Scene/prefab/GameObject hierarchy contract (KGD-126).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneObjectMetadata {
    pub stable_id: String,
    pub name: String,
    pub parent: Option<String>,
    pub components: Vec<String>,
    pub references: Vec<String>,
    pub sibling_index: u32,
}
