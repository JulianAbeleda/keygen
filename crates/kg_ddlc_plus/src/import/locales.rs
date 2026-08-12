//! Locale/string/sprite/font/bundle-variant contract (KGD-129).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocaleMetadata {
    pub locale: String,
    pub strings: Vec<StringEntry>,
    pub sprite_ids: Vec<String>,
    pub font_ids: Vec<String>,
    pub bundle_variants: Vec<String>,
    pub fallback: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StringEntry {
    pub id: String,
    pub value_hash: String,
}
