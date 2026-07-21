use serde::{Deserialize, Serialize};

use crate::ids::ModelPresetId;
use crate::ArenaKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EloEntry {
    pub model_preset_id: ModelPresetId,
    pub arena_kind: ArenaKind,
    pub category: Option<String>,
    pub rating: f64,
    pub games: u32,
    pub wins: u32,
}
