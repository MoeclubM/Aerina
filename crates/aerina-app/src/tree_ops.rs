use super::*;
use aerina_analytics::new_event;

impl AppState {
    pub async fn commit_candidate(
        &self,
        conversation_id: &str,
        candidate_id: &str,
    ) -> Result<ConversationDetail> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let candidate_id = parse_entity_id(candidate_id)?;
        let conversation = self
            .inner
            .db
            .get_conversation(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found"))?;
        let branch_id = conversation
            .active_branch_id
            .ok_or_else(|| anyhow!("missing active branch"))?;
        let mut branch = self
            .inner
            .db
            .get_branch(branch_id)
            .await?
            .ok_or_else(|| anyhow!("branch not found"))?;
        let candidate = self
            .inner
            .db
            .get_candidate(candidate_id)
            .await?
            .ok_or_else(|| anyhow!("candidate not found"))?;
        let mut round = self
            .inner
            .db
            .get_round(candidate.round_id)
            .await?
            .ok_or_else(|| anyhow!("round not found"))?;

        let messages = self.inner.db.list_messages(branch_id).await?;
        let assistant = messages
            .iter()
            .find(|(m, _)| m.candidate_id == Some(candidate.id))
            .ok_or_else(|| anyhow!("candidate message not found on branch"))?;
        let first_commit = round.selected_candidate_id.is_none();
        branch.head_message_id = Some(assistant.0.id);
        round.selected_candidate_id = Some(candidate.id);
        self.inner.db.upsert_branch(&branch).await?;
        self.inner.db.update_round(&round).await?;

        // Multi-candidate pick counts for the leaderboard (SBS / compare).
        if first_commit {
            let candidates = self.inner.db.list_candidates(candidate.round_id).await?;
            if candidates.len() >= 2 {
                let settings = self.inner.db.get_settings(conversation_id).await?;
                let arena_kind = settings
                    .as_ref()
                    .and_then(|s| s.arena_kind)
                    .unwrap_or(ArenaKind::Text);
                let category = settings.and_then(|s| s.arena_category);
                for loser in candidates.iter().filter(|c| c.id != candidate.id) {
                    let ranking = RankingEvent {
                        id: RankingEventId::new(),
                        workspace_id: self.current_workspace().await,
                        arena_kind,
                        category: category.clone(),
                        winner_preset_id: candidate.model_preset_id,
                        loser_preset_id: loser.model_preset_id,
                        round_id: candidate.round_id,
                        created_at: Utc::now(),
                    };
                    self.inner.db.insert_ranking_event(&ranking).await?;
                }
            }
        }

        let event = new_event(NewAnalyticsEvent {
            event_type: AnalyticsEventType::CandidateCommitted,
            workspace_id: self.current_workspace().await,
            conversation_id: Some(conversation_id),
            round_id: Some(candidate.round_id),
            candidate_id: Some(candidate.id),
            payload: serde_json::json!({}),
        });
        self.inner.db.insert_analytics_event(&event).await?;
        self.inner.db.touch_conversation(conversation_id).await?;
        self.get_conversation_ui(&conversation_id.to_string()).await
    }

    pub async fn fork_candidate(
        &self,
        conversation_id: &str,
        candidate_id: &str,
    ) -> Result<ConversationDetail> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let candidate_id = parse_entity_id(candidate_id)?;
        let conversation = self
            .inner
            .db
            .get_conversation(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found"))?;
        let branch_id = conversation
            .active_branch_id
            .ok_or_else(|| anyhow!("missing active branch"))?;
        let parent_branch = self
            .inner
            .db
            .get_branch(branch_id)
            .await?
            .ok_or_else(|| anyhow!("branch not found"))?;
        let candidate = self
            .inner
            .db
            .get_candidate(candidate_id)
            .await?
            .ok_or_else(|| anyhow!("candidate not found"))?;
        let round = self
            .inner
            .db
            .get_round(candidate.round_id)
            .await?
            .ok_or_else(|| anyhow!("round not found"))?;

        let source_blocks = self
            .inner
            .db
            .get_candidate_message_blocks(candidate.id)
            .await?
            .map(|(_, blocks)| blocks)
            .unwrap_or_else(|| vec![ContentBlock::text(String::new())]);

        let fork = tree::fork_from_candidate(
            conversation_id,
            &parent_branch,
            &candidate,
            round.user_message_id,
        );
        self.inner.db.upsert_branch(&fork.branch).await?;
        self.inner
            .db
            .insert_message(&fork.assistant_message, &source_blocks)
            .await?;
        self.inner
            .db
            .set_active_branch(conversation_id, fork.branch.id)
            .await?;

        let event = new_event(NewAnalyticsEvent {
            event_type: AnalyticsEventType::BranchForked,
            workspace_id: self.current_workspace().await,
            conversation_id: Some(conversation_id),
            round_id: Some(candidate.round_id),
            candidate_id: Some(candidate.id),
            payload: serde_json::json!({
                "branch_id": fork.branch.id.to_string(),
            }),
        });
        self.inner.db.insert_analytics_event(&event).await?;
        self.get_conversation_ui(&conversation_id.to_string()).await
    }

    pub async fn switch_branch(
        &self,
        conversation_id: &str,
        branch_id: &str,
    ) -> Result<ConversationDetail> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let branch_id = parse_entity_id(branch_id)?;
        let branch = self
            .inner
            .db
            .get_branch(branch_id)
            .await?
            .ok_or_else(|| anyhow!("branch not found"))?;
        if branch.conversation_id != conversation_id {
            return Err(anyhow!("branch does not belong to conversation"));
        }
        self.inner
            .db
            .set_active_branch(conversation_id, branch_id)
            .await?;
        self.get_conversation_ui(&conversation_id.to_string()).await
    }

    pub async fn edit_user_message<F>(
        &self,
        conversation_id: &str,
        message_id: &str,
        content: String,
        on_event: F,
    ) -> Result<ConversationDetail>
    where
        F: FnMut(GenerationEvent) + Send,
    {
        let conversation_id_parsed = parse_entity_id(conversation_id)?;
        let message_id = parse_entity_id(message_id)?;
        let (message, _) = self
            .inner
            .db
            .get_message(message_id)
            .await?
            .ok_or_else(|| anyhow!("message not found"))?;
        if !matches!(message.role, MessageRole::User) {
            return Err(anyhow!(
                "only user messages can be edited into a new branch"
            ));
        }
        if message.conversation_id != conversation_id_parsed {
            return Err(anyhow!("message conversation mismatch"));
        }

        let parent_branch = self
            .inner
            .db
            .get_branch(message.branch_id)
            .await?
            .ok_or_else(|| anyhow!("branch not found"))?;

        // Create a new branch forked from the edited message's parent context.
        let new_branch = Branch {
            id: BranchId::new(),
            conversation_id: conversation_id_parsed,
            parent_branch_id: Some(parent_branch.id),
            fork_candidate_id: None,
            head_message_id: message.parent_message_id,
            created_at: Utc::now(),
        };
        self.inner.db.upsert_branch(&new_branch).await?;
        self.inner
            .db
            .set_active_branch(conversation_id_parsed, new_branch.id)
            .await?;

        self.run_generation(
            SendMessageRequest {
                conversation_id: conversation_id.to_string(),
                content,
                image_data_urls: None,
                require_image: None,
                image_size: None,
            },
            None,
            on_event,
        )
        .await
    }

    pub async fn cast_arena_vote(
        &self,
        conversation_id: &str,
        round_id: &str,
        vote_kind: VoteKind,
        selected_candidate_id: Option<String>,
    ) -> Result<ConversationDetail> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let round_id = parse_entity_id(round_id)?;
        let selected = selected_candidate_id
            .as_deref()
            .map(parse_entity_id)
            .transpose()?;

        let candidates = self.inner.db.list_candidates(round_id).await?;
        if candidates.is_empty() {
            return Err(anyhow!("round has no candidates"));
        }

        let vote = ArenaVote {
            id: ArenaVoteId::new(),
            round_id,
            vote_kind,
            selected_candidate_id: selected,
            created_at: Utc::now(),
        };
        self.inner.db.insert_arena_vote(&vote).await?;

        let vote_event = new_event(NewAnalyticsEvent {
            event_type: AnalyticsEventType::ArenaVoteCast,
            workspace_id: self.current_workspace().await,
            conversation_id: Some(conversation_id),
            round_id: Some(round_id),
            candidate_id: selected,
            payload: serde_json::json!({ "vote_kind": vote_kind }),
        });
        self.inner.db.insert_analytics_event(&vote_event).await?;

        if vote_kind == VoteKind::Best {
            let winner =
                selected.ok_or_else(|| anyhow!("best vote requires selected candidate"))?;
            // Ranking is recorded inside commit_candidate for multi-candidate rounds.
            self.commit_candidate(&conversation_id.to_string(), &winner.to_string())
                .await?;
        }

        let reveal = new_event(NewAnalyticsEvent {
            event_type: AnalyticsEventType::ArenaRevealed,
            workspace_id: self.current_workspace().await,
            conversation_id: Some(conversation_id),
            round_id: Some(round_id),
            candidate_id: selected,
            payload: serde_json::json!({
                "candidates": candidates.iter().map(|c| serde_json::json!({
                    "id": c.id.to_string(),
                    "slot": c.slot_label,
                    "model": c.model_name,
                    "preset_id": c.model_preset_id.to_string(),
                })).collect::<Vec<_>>()
            }),
        });
        self.inner.db.insert_analytics_event(&reveal).await?;
        self.get_conversation_ui(&conversation_id.to_string()).await
    }
}
