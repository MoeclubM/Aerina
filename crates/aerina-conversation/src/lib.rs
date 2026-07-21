use aerina_domain::*;

pub fn build_chat_messages(
    history: &[(MessageNode, Vec<ContentBlock>)],
    media_data_urls: &std::collections::HashMap<String, String>,
) -> Vec<ChatMessage> {
    history
        .iter()
        .filter_map(|(message, blocks)| {
            let role = match message.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };

            let mut parts = Vec::new();
            let mut plain = String::new();
            for block in blocks {
                match block {
                    ContentBlock::Text { text } => {
                        if !text.is_empty() {
                            plain.push_str(text);
                            parts.push(ChatContentPart::Text { text: text.clone() });
                        }
                    }
                    ContentBlock::Image { media_id, .. } => {
                        if let Some(url) = media_data_urls.get(&media_id.to_string()) {
                            parts.push(ChatContentPart::ImageUrl {
                                image_url: ImageUrl { url: url.clone() },
                            });
                        }
                    }
                    _ => {}
                }
            }

            if parts.is_empty() {
                return None;
            }

            let content = if parts.len() == 1 {
                match &parts[0] {
                    ChatContentPart::Text { text } => ChatContent::Text(text.clone()),
                    _ => ChatContent::Parts(parts),
                }
            } else if parts
                .iter()
                .all(|p| matches!(p, ChatContentPart::Text { .. }))
            {
                ChatContent::Text(plain)
            } else {
                ChatContent::Parts(parts)
            };

            Some(ChatMessage {
                role: role.into(),
                content,
                tool_call_id: None,
                tool_calls: None,
            })
        })
        .collect()
}

pub fn default_settings(
    conversation_id: ConversationId,
    mode: ConversationMode,
    model_preset_ids: Vec<ModelPresetId>,
) -> ConversationSettings {
    ConversationSettings {
        conversation_id,
        mode,
        system_prompt: None,
        temperature: Some(0.7),
        model_preset_ids: model_preset_ids.clone(),
        candidate_pool: model_preset_ids,
        slot_count: 1,
        arena_kind: None,
        arena_category: None,
        max_concurrency: 4,
        image_size: Some("1024x1024".into()),
        image_aspect_ratio: None,
    }
}
