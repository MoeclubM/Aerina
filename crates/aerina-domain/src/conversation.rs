use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::*;
use crate::{
    AnalyticsEventType, ArenaKind, CandidateStatus, CapabilityTag, ConversationMode, MessageRole,
    ProviderKind, VoteKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub display_name: String,
    /// Relative path under media root (e.g. profile-avatars/{id}.png). None = letter initial in UI.
    pub avatar_path: Option<String>,
    /// Future remote account binding (e.g. OAuth subject). Local-only until login ships.
    pub auth_subject: Option<String>,
    /// Future auth provider id (e.g. "github", "apple"). Local-only until login ships.
    pub auth_provider: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub profile_id: ProfileId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub mode: ConversationMode,
    pub active_branch_id: Option<BranchId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSettings {
    pub conversation_id: ConversationId,
    pub mode: ConversationMode,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub model_preset_ids: Vec<ModelPresetId>,
    pub candidate_pool: Vec<ModelPresetId>,
    pub slot_count: u32,
    pub arena_kind: Option<ArenaKind>,
    pub arena_category: Option<String>,
    pub max_concurrency: u32,
    pub image_size: Option<String>,
    pub image_aspect_ratio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: BranchId,
    pub conversation_id: ConversationId,
    pub parent_branch_id: Option<BranchId>,
    pub fork_candidate_id: Option<CandidateId>,
    pub head_message_id: Option<MessageNodeId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageNode {
    pub id: MessageNodeId,
    pub conversation_id: ConversationId,
    pub branch_id: BranchId,
    pub parent_message_id: Option<MessageNodeId>,
    pub role: MessageRole,
    pub round_id: Option<RoundId>,
    pub candidate_id: Option<CandidateId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub id: RoundId,
    pub conversation_id: ConversationId,
    pub branch_id: BranchId,
    pub user_message_id: MessageNodeId,
    pub selected_candidate_id: Option<CandidateId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateGeneration {
    pub id: CandidateId,
    pub round_id: RoundId,
    pub slot_label: String,
    pub model_preset_id: ModelPresetId,
    pub provider_id: ProviderId,
    pub model_name: String,
    pub status: CandidateStatus,
    pub anonymous: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundSnapshot {
    pub round_id: RoundId,
    pub mode: ConversationMode,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub model_preset_ids: Vec<ModelPresetId>,
    pub arena_kind: Option<ArenaKind>,
    pub arena_category: Option<String>,
    pub image_size: Option<String>,
    pub image_aspect_ratio: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub candidate_id: CandidateId,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<u64>,
    pub ttft_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub secret_ref: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: ModelId,
    pub provider_id: ProviderId,
    pub model_name: String,
    pub display_name: String,
    pub capabilities: Vec<CapabilityTag>,
    pub context_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreset {
    pub id: ModelPresetId,
    pub workspace_id: WorkspaceId,
    pub provider_id: ProviderId,
    pub model_id: Option<ModelId>,
    pub name: String,
    pub model_name: String,
    pub capabilities: Vec<CapabilityTag>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub in_random_pool: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaProfile {
    pub id: ArenaProfileId,
    pub conversation_id: ConversationId,
    pub kind: ArenaKind,
    pub category: Option<String>,
    pub candidate_pool: Vec<ModelPresetId>,
    pub slot_count: u32,
    pub capability_requirements: Vec<CapabilityTag>,
    pub allow_same_provider: bool,
    pub scoring_profile: String,
    pub reveal_policy: String,
    pub continuation_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaVote {
    pub id: ArenaVoteId,
    pub round_id: RoundId,
    pub vote_kind: VoteKind,
    pub selected_candidate_id: Option<CandidateId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub id: AnalyticsEventId,
    pub event_type: AnalyticsEventType,
    pub workspace_id: WorkspaceId,
    pub conversation_id: Option<ConversationId>,
    pub round_id: Option<RoundId>,
    pub candidate_id: Option<CandidateId>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingEvent {
    pub id: RankingEventId,
    pub workspace_id: WorkspaceId,
    pub arena_kind: ArenaKind,
    pub category: Option<String>,
    pub winner_preset_id: ModelPresetId,
    pub loser_preset_id: ModelPresetId,
    pub round_id: RoundId,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaObject {
    pub id: MediaObjectId,
    pub workspace_id: WorkspaceId,
    pub relative_path: String,
    pub mime: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_at: DateTime<Utc>,
}
