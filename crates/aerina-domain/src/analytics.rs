use serde::{Deserialize, Serialize};

use crate::ids::*;
use crate::AnalyticsEventType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAnalyticsEvent {
    pub event_type: AnalyticsEventType,
    pub workspace_id: WorkspaceId,
    pub conversation_id: Option<ConversationId>,
    pub round_id: Option<RoundId>,
    pub candidate_id: Option<CandidateId>,
    pub payload: serde_json::Value,
}
