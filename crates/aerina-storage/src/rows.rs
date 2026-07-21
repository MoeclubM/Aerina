#![allow(dead_code)]
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct ConversationRow {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub mode: String,
    pub active_branch_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct BranchRow {
    pub id: String,
    pub conversation_id: String,
    pub parent_branch_id: Option<String>,
    pub fork_candidate_id: Option<String>,
    pub head_message_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct MessageRow {
    pub id: String,
    pub conversation_id: String,
    pub branch_id: String,
    pub parent_message_id: Option<String>,
    pub role: String,
    pub round_id: Option<String>,
    pub candidate_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct ContentBlockRow {
    pub message_id: String,
    pub position: i64,
    pub block_json: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct RoundRow {
    pub id: String,
    pub conversation_id: String,
    pub branch_id: String,
    pub user_message_id: String,
    pub selected_candidate_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct CandidateRow {
    pub id: String,
    pub round_id: String,
    pub slot_label: String,
    pub model_preset_id: String,
    pub provider_id: String,
    pub model_name: String,
    pub status: String,
    pub anonymous: i64,
    pub error_message: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, FromRow)]
pub(crate) struct ProviderRow {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub secret_ref: Option<String>,
    pub enabled: i64,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct ModelPresetRow {
    pub id: String,
    pub workspace_id: String,
    pub provider_id: String,
    pub model_id: Option<String>,
    pub name: String,
    pub model_name: String,
    pub capabilities_json: String,
    pub temperature: Option<f64>,
    pub system_prompt: Option<String>,
    pub in_random_pool: i64,
    pub enabled: i64,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct SettingsRow {
    pub conversation_id: String,
    pub mode: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f64>,
    pub model_preset_ids_json: String,
    pub candidate_pool_json: String,
    pub slot_count: i64,
    pub arena_kind: Option<String>,
    pub arena_category: Option<String>,
    pub max_concurrency: i64,
    pub image_size: Option<String>,
    pub image_aspect_ratio: Option<String>,
}

#[derive(Debug, FromRow)]
pub(crate) struct AnalyticsRow {
    pub id: String,
    pub event_type: String,
    pub workspace_id: String,
    pub conversation_id: Option<String>,
    pub round_id: Option<String>,
    pub candidate_id: Option<String>,
    pub payload_json: String,
    pub created_at: String,
}
