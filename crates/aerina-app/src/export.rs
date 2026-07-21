use super::*;
use aerina_conversation::default_settings;
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationExport {
    pub version: u32,
    pub conversation: Conversation,
    pub settings: ConversationSettings,
    pub branches: Vec<Branch>,
    pub messages: Vec<MessageView>,
}

impl AppState {
    pub async fn export_conversation(&self, conversation_id: &str) -> Result<ConversationExport> {
        let detail = self.get_conversation_detail(conversation_id).await?;
        // Export all messages from all branches.
        let mut messages = Vec::new();
        for branch in &detail.branches {
            let branch_messages = self.inner.db.list_messages(branch.id).await?;
            for (message, blocks) in branch_messages {
                messages.push(MessageView { message, blocks });
            }
        }
        Ok(ConversationExport {
            version: 1,
            conversation: detail.conversation,
            settings: detail.settings,
            branches: detail.branches,
            messages,
        })
    }

    pub async fn import_conversation(&self, payload: ConversationExport) -> Result<Conversation> {
        if payload.version != 1 {
            return Err(anyhow!("unsupported export version"));
        }

        let (conversation, root_branch) = tree::create_conversation(
            self.current_workspace().await,
            payload.conversation.title.clone(),
            payload.conversation.mode,
        );
        let mut settings = payload.settings.clone();
        settings.conversation_id = conversation.id;
        // Drop foreign preset IDs that do not exist locally.
        let local_presets = self.list_model_presets().await?;
        let local_ids = local_presets.iter().map(|p| p.id).collect::<Vec<_>>();
        settings
            .model_preset_ids
            .retain(|id| local_ids.contains(id));
        settings.candidate_pool.retain(|id| local_ids.contains(id));
        if settings.model_preset_ids.is_empty() {
            settings = default_settings(
                conversation.id,
                payload.conversation.mode,
                local_presets.into_iter().take(1).map(|p| p.id).collect(),
            );
        }

        self.inner
            .db
            .insert_conversation(&conversation, &root_branch, &settings)
            .await?;

        // Import linear active branch messages only for v1 simplicity.
        let active_old = payload.conversation.active_branch_id;
        let mut parent = None;
        let mut branch = root_branch;
        for item in payload.messages {
            if let Some(active) = active_old {
                if item.message.branch_id != active {
                    continue;
                }
            }
            if !matches!(
                item.message.role,
                MessageRole::User | MessageRole::Assistant
            ) {
                continue;
            }
            let message = MessageNode {
                id: MessageNodeId::new(),
                conversation_id: conversation.id,
                branch_id: branch.id,
                parent_message_id: parent,
                role: item.message.role,
                round_id: None,
                candidate_id: None,
                created_at: Utc::now(),
            };
            // Strip media/image blocks that cannot be resolved in this import path.
            let blocks = item
                .blocks
                .into_iter()
                .filter(|b| {
                    matches!(
                        b,
                        ContentBlock::Text { .. }
                            | ContentBlock::Thinking { .. }
                            | ContentBlock::Code { .. }
                            | ContentBlock::UsageMeta { .. }
                    )
                })
                .collect::<Vec<_>>();
            self.inner.db.insert_message(&message, &blocks).await?;
            parent = Some(message.id);
            branch.head_message_id = Some(message.id);
            self.inner.db.upsert_branch(&branch).await?;
        }

        let _ = json!({"imported": true});
        self.inner.db.touch_conversation(conversation.id).await?;
        Ok(conversation)
    }

    pub async fn get_media_data_url(&self, media_id: &str) -> Result<String> {
        let media_id = parse_entity_id(media_id)?;
        let media = self
            .inner
            .db
            .get_media_object(media_id)
            .await?
            .ok_or_else(|| anyhow!("media not found"))?;
        self.inner
            .media
            .data_url(&media.relative_path, &media.mime)
            .await
    }
}
