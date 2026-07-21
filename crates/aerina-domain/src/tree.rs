use chrono::Utc;

use crate::ids::*;
use crate::{
    Branch, CandidateGeneration, CandidateStatus, Conversation, ConversationMode, DomainError,
    MessageNode, MessageRole, Round,
};

#[derive(Debug, Clone)]
pub struct CommitResult {
    pub assistant_message: MessageNode,
    pub branch: Branch,
    pub round: Round,
}

#[derive(Debug, Clone)]
pub struct ForkResult {
    pub branch: Branch,
    pub assistant_message: MessageNode,
}

pub fn create_conversation(
    workspace_id: WorkspaceId,
    title: impl Into<String>,
    mode: ConversationMode,
) -> (Conversation, Branch) {
    let conversation_id = ConversationId::new();
    let branch_id = BranchId::new();
    let now = Utc::now();

    let conversation = Conversation {
        id: conversation_id,
        workspace_id,
        title: title.into(),
        mode,
        active_branch_id: Some(branch_id),
        created_at: now,
        updated_at: now,
    };

    let branch = Branch {
        id: branch_id,
        conversation_id,
        parent_branch_id: None,
        fork_candidate_id: None,
        head_message_id: None,
        created_at: now,
    };

    (conversation, branch)
}

pub fn create_user_message(
    conversation_id: ConversationId,
    branch_id: BranchId,
    parent_message_id: Option<MessageNodeId>,
) -> MessageNode {
    MessageNode {
        id: MessageNodeId::new(),
        conversation_id,
        branch_id,
        parent_message_id,
        role: MessageRole::User,
        round_id: None,
        candidate_id: None,
        created_at: Utc::now(),
    }
}

pub fn create_round(
    conversation_id: ConversationId,
    branch_id: BranchId,
    user_message_id: MessageNodeId,
) -> Round {
    Round {
        id: RoundId::new(),
        conversation_id,
        branch_id,
        user_message_id,
        selected_candidate_id: None,
        created_at: Utc::now(),
    }
}

pub fn create_candidate(
    round_id: RoundId,
    slot_label: impl Into<String>,
    model_preset_id: ModelPresetId,
    provider_id: ProviderId,
    model_name: impl Into<String>,
    anonymous: bool,
) -> CandidateGeneration {
    CandidateGeneration {
        id: CandidateId::new(),
        round_id,
        slot_label: slot_label.into(),
        model_preset_id,
        provider_id,
        model_name: model_name.into(),
        status: CandidateStatus::Pending,
        anonymous,
        error_message: None,
        created_at: Utc::now(),
        completed_at: None,
    }
}

pub fn commit_candidate(
    mut branch: Branch,
    mut round: Round,
    candidate: &CandidateGeneration,
    parent_message_id: MessageNodeId,
) -> Result<CommitResult, DomainError> {
    if candidate.round_id != round.id {
        return Err(DomainError::InvalidOperation(
            "candidate does not belong to round".into(),
        ));
    }

    let assistant_message = MessageNode {
        id: MessageNodeId::new(),
        conversation_id: branch.conversation_id,
        branch_id: branch.id,
        parent_message_id: Some(parent_message_id),
        role: MessageRole::Assistant,
        round_id: Some(round.id),
        candidate_id: Some(candidate.id),
        created_at: Utc::now(),
    };

    branch.head_message_id = Some(assistant_message.id);
    round.selected_candidate_id = Some(candidate.id);

    Ok(CommitResult {
        assistant_message,
        branch,
        round,
    })
}

pub fn fork_from_candidate(
    conversation_id: ConversationId,
    parent_branch: &Branch,
    candidate: &CandidateGeneration,
    parent_message_id: MessageNodeId,
) -> ForkResult {
    let branch_id = BranchId::new();
    let assistant_message = MessageNode {
        id: MessageNodeId::new(),
        conversation_id,
        branch_id,
        parent_message_id: Some(parent_message_id),
        role: MessageRole::Assistant,
        round_id: Some(candidate.round_id),
        candidate_id: Some(candidate.id),
        created_at: Utc::now(),
    };

    let branch = Branch {
        id: branch_id,
        conversation_id,
        parent_branch_id: Some(parent_branch.id),
        fork_candidate_id: Some(candidate.id),
        head_message_id: Some(assistant_message.id),
        created_at: Utc::now(),
    };

    ForkResult {
        branch,
        assistant_message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityTag, ContentBlock};

    #[test]
    fn commit_moves_only_selected_candidate_to_mainline() {
        let workspace_id = WorkspaceId::new();
        let (conversation, branch) =
            create_conversation(workspace_id, "demo", ConversationMode::Chat);
        let user = create_user_message(conversation.id, branch.id, None);
        let round = create_round(conversation.id, branch.id, user.id);
        let winner = create_candidate(
            round.id,
            "A",
            ModelPresetId::new(),
            ProviderId::new(),
            "model-a",
            false,
        );
        let loser = create_candidate(
            round.id,
            "B",
            ModelPresetId::new(),
            ProviderId::new(),
            "model-b",
            false,
        );

        let commit = commit_candidate(branch.clone(), round.clone(), &winner, user.id).unwrap();
        assert_eq!(commit.round.selected_candidate_id, Some(winner.id));
        assert_eq!(commit.assistant_message.candidate_id, Some(winner.id));
        assert_ne!(commit.assistant_message.candidate_id, Some(loser.id));
        assert_eq!(
            commit.branch.head_message_id,
            Some(commit.assistant_message.id)
        );
    }

    #[test]
    fn fork_reuses_common_ancestor_and_sets_new_head() {
        let workspace_id = WorkspaceId::new();
        let (conversation, branch) =
            create_conversation(workspace_id, "demo", ConversationMode::Sbs);
        let user = create_user_message(conversation.id, branch.id, None);
        let round = create_round(conversation.id, branch.id, user.id);
        let candidate = create_candidate(
            round.id,
            "A",
            ModelPresetId::new(),
            ProviderId::new(),
            "model-a",
            false,
        );

        let fork = fork_from_candidate(conversation.id, &branch, &candidate, user.id);
        assert_eq!(fork.branch.parent_branch_id, Some(branch.id));
        assert_eq!(fork.branch.fork_candidate_id, Some(candidate.id));
        assert_eq!(fork.assistant_message.parent_message_id, Some(user.id));
        assert_eq!(fork.branch.head_message_id, Some(fork.assistant_message.id));
    }

    #[test]
    fn content_block_roundtrips() {
        let block = ContentBlock::text("hello");
        let json = serde_json::to_string(&block).unwrap();
        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ContentBlock::text("hello"));
        assert!(matches!(CapabilityTag::Text, CapabilityTag::Text));
    }
}
