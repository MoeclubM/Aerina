use serde::{Deserialize, Serialize};

use crate::ids::*;
use crate::{ArenaKind, CapabilityTag};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaDrawRequest {
    pub kind: ArenaKind,
    pub candidate_pool: Vec<ModelPresetId>,
    pub slot_count: usize,
    pub capability_requirements: Vec<CapabilityTag>,
    pub allow_same_provider: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaSlot {
    pub slot_label: String,
    pub model_preset_id: ModelPresetId,
    pub provider_id: ProviderId,
    pub model_name: String,
}
