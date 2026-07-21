use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(pub Uuid);

impl EntityId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EntityId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<EntityId> for Uuid {
    fn from(value: EntityId) -> Self {
        value.0
    }
}

pub type ProfileId = EntityId;
pub type WorkspaceId = EntityId;
pub type ConversationId = EntityId;
pub type BranchId = EntityId;
pub type MessageNodeId = EntityId;
pub type RoundId = EntityId;
pub type CandidateId = EntityId;
pub type ProviderId = EntityId;
pub type ModelId = EntityId;
pub type ModelPresetId = EntityId;
pub type ArenaProfileId = EntityId;
pub type MediaObjectId = EntityId;
pub type AnalyticsEventId = EntityId;
pub type RankingEventId = EntityId;
pub type ArenaVoteId = EntityId;
pub type McpServerId = EntityId;
