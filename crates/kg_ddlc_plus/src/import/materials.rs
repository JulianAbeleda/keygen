//! Material/shader-property/texture-binding contract (KGD-128).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialMetadata {
    pub id: String,
    pub shader: String,
    pub properties: Vec<ShaderProperty>,
    pub texture_bindings: Vec<TextureBinding>,
    pub supported: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderProperty {
    pub name: String,
    pub value_type: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TextureBinding {
    pub property: String,
    pub image_id: String,
}
