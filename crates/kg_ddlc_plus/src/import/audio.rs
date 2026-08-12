//! Audio metadata contract (KGD-124).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AudioMetadata {
    pub id: String,
    pub codec: String,
    pub decoded_pcm_hash: String,
    pub channels: u16,
    pub sample_rate: u32,
    pub duration_ms: u64,
    pub loop_region: Option<LoopRegion>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoopRegion {
    pub start_sample: u64,
    pub end_sample: u64,
}
