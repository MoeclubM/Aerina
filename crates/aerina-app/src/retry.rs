use super::*;
use aerina_analytics::new_event;
use aerina_conversation::build_chat_messages;
use aerina_generation::{GenerationTarget, RoundContext};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures::StreamExt;

impl AppState {
    pub async fn retry_candidate<F>(
        &self,
        conversation_id: &str,
        candidate_id: &str,
        mut on_event: F,
    ) -> Result<ConversationDetail>
    where
        F: FnMut(GenerationEvent) + Send,
    {
        let conversation_id_parsed = parse_entity_id(conversation_id)?;
        let candidate_id = parse_entity_id(candidate_id)?;
        let mut candidate = self
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
        let conversation = self
            .inner
            .db
            .get_conversation(conversation_id_parsed)
            .await?
            .ok_or_else(|| anyhow!("conversation not found"))?;
        let settings = self
            .inner
            .db
            .get_settings(conversation_id_parsed)
            .await?
            .ok_or_else(|| anyhow!("settings not found"))?;
        let branch_id = conversation
            .active_branch_id
            .ok_or_else(|| anyhow!("missing active branch"))?;

        let (user_message, user_blocks) = self
            .inner
            .db
            .get_message(round.user_message_id)
            .await?
            .ok_or_else(|| anyhow!("user message not found"))?;

        let prev = self
            .inner
            .db
            .get_candidate_message_blocks(candidate.id)
            .await?;
        let require_image = prev
            .as_ref()
            .map(|(_, blocks)| {
                blocks
                    .iter()
                    .any(|b| matches!(b, ContentBlock::Image { .. }))
            })
            .unwrap_or(false);

        candidate.status = CandidateStatus::Pending;
        candidate.error_message = None;
        candidate.completed_at = None;
        self.inner.db.update_candidate(&candidate).await?;

        let resolved = self.resolve_preset(candidate.model_preset_id).await?;
        let started = new_event(NewAnalyticsEvent {
            event_type: AnalyticsEventType::GenerationStarted,
            workspace_id: self.current_workspace().await,
            conversation_id: Some(conversation_id_parsed),
            round_id: Some(round.id),
            candidate_id: Some(candidate.id),
            payload: serde_json::json!({
                "model": resolved.model_name,
                "slot": candidate.slot_label,
                "retry": true,
            }),
        });
        self.inner.db.insert_analytics_event(&started).await?;

        let history = self.inner.db.list_messages(branch_id).await?;
        let mut history_with_user = Vec::new();
        for item in history {
            let done = item.0.id == user_message.id;
            history_with_user.push(item);
            if done {
                break;
            }
        }

        let mut media_urls = HashMap::new();
        for (_, blocks) in &history_with_user {
            for block in blocks {
                if let ContentBlock::Image { media_id, .. } = block {
                    if let Some(media) = self.inner.db.get_media_object(*media_id).await? {
                        media_urls.insert(
                            media_id.to_string(),
                            self.inner
                                .media
                                .data_url(&media.relative_path, &media.mime)
                                .await?,
                        );
                    }
                }
            }
        }

        let user_text = user_blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let wants_tools = resolved.capabilities.contains(&CapabilityTag::ToolCalling);
        let (tools, executor) = if wants_tools {
            self.build_tool_executor().await?
        } else {
            (Vec::new(), None)
        };

        let context = RoundContext {
            messages: build_chat_messages(&history_with_user, &media_urls),
            system_prompt: settings.system_prompt.clone(),
            temperature: settings.temperature,
            image_prompt: if require_image { Some(user_text) } else { None },
            image_size: settings.image_size.clone(),
            require_image,
            tools,
        };

        let cancel = CancellationToken::new();
        {
            let mut jobs = self.inner.active_jobs.lock().await;
            jobs.insert(conversation_id.to_string(), cancel.clone());
        }

        let targets = vec![GenerationTarget {
            candidate_id: candidate.id,
            slot_label: candidate.slot_label.clone(),
            preset: resolved,
        }];
        let mut stream = self
            .inner
            .generation
            .generate(context, targets, cancel, executor)
            .await?;

        let mut output = String::new();
        let mut thinking = String::new();
        let mut image: Option<GeneratedImage> = None;
        let mut usage: Option<UsageReport> = None;
        let mut failure: Option<String> = None;

        while let Some(event) = stream.next().await {
            match &event {
                GenerationEvent::TextDelta { delta, .. } => output.push_str(delta),
                GenerationEvent::ThinkingDelta { delta, .. } => thinking.push_str(delta),
                GenerationEvent::ImageReady { image: img, .. } => image = Some(img.clone()),
                GenerationEvent::Usage { usage: u, .. } => usage = Some(u.clone()),
                GenerationEvent::Error { message, .. } => failure = Some(message.clone()),
                _ => {}
            }
            on_event(event);
        }

        {
            let mut jobs = self.inner.active_jobs.lock().await;
            jobs.remove(conversation_id);
        }

        if let Some(message) = failure {
            candidate.status = if message == "cancelled" {
                CandidateStatus::Cancelled
            } else {
                CandidateStatus::Failed
            };
            candidate.error_message = Some(message.clone());
            candidate.completed_at = Some(Utc::now());
            self.inner.db.update_candidate(&candidate).await?;
            let event_type = if message == "cancelled" {
                AnalyticsEventType::GenerationCancelled
            } else {
                AnalyticsEventType::GenerationFailed
            };
            let failed = new_event(NewAnalyticsEvent {
                event_type,
                workspace_id: self.current_workspace().await,
                conversation_id: Some(conversation_id_parsed),
                round_id: Some(round.id),
                candidate_id: Some(candidate.id),
                payload: serde_json::json!({ "error": message }),
            });
            self.inner.db.insert_analytics_event(&failed).await?;
            return self.get_conversation_ui(conversation_id).await;
        }

        candidate.status = CandidateStatus::Completed;
        candidate.completed_at = Some(Utc::now());
        self.inner.db.update_candidate(&candidate).await?;

        let mut blocks = Vec::new();
        let thinking_usage = usage.as_ref();
        if !thinking.is_empty()
            || thinking_usage
                .and_then(|value| value.reasoning_tokens)
                .is_some()
            || thinking_usage
                .and_then(|value| value.reasoning_duration_ms)
                .is_some()
        {
            blocks.push(ContentBlock::Thinking {
                text: thinking,
                reasoning_tokens: thinking_usage.and_then(|value| value.reasoning_tokens),
                reasoning_duration_ms: thinking_usage.and_then(|value| value.reasoning_duration_ms),
            });
        }
        if !output.is_empty() {
            blocks.push(ContentBlock::text(output));
        }
        if let Some(image) = image {
            let bytes = if let Some(b64) = &image.b64_json {
                STANDARD.decode(b64)?
            } else if let Some(url) = &image.url {
                reqwest::get(url).await?.bytes().await?.to_vec()
            } else {
                return Err(anyhow!("image result missing payload"));
            };
            let (relative, _) = self.inner.media.save_generated_image(&bytes, "png").await?;
            let media = MediaObject {
                id: MediaObjectId::new(),
                workspace_id: self.current_workspace().await,
                relative_path: relative,
                mime: image.mime.clone(),
                width: image.width,
                height: image.height,
                created_at: Utc::now(),
            };
            self.inner.db.insert_media_object(&media).await?;
            blocks.push(ContentBlock::Image {
                media_id: media.id,
                alt: None,
                revised_prompt: image.revised_prompt.clone(),
            });
            let saved = new_event(NewAnalyticsEvent {
                event_type: AnalyticsEventType::ImageSaved,
                workspace_id: self.current_workspace().await,
                conversation_id: Some(conversation_id_parsed),
                round_id: Some(round.id),
                candidate_id: Some(candidate.id),
                payload: serde_json::json!({ "media_id": media.id.to_string() }),
            });
            self.inner.db.insert_analytics_event(&saved).await?;
        }
        if blocks.is_empty() {
            blocks.push(ContentBlock::text(String::new()));
        }
        if let Some(usage) = usage {
            blocks.push(ContentBlock::UsageMeta {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
                cost_usd: usage.cost_usd,
                latency_ms: usage.latency_ms,
                ttft_ms: usage.ttft_ms,
            });
            self.inner
                .db
                .insert_usage(&UsageRecord {
                    candidate_id: candidate.id,
                    prompt_tokens: usage.prompt_tokens,
                    completion_tokens: usage.completion_tokens,
                    total_tokens: usage.total_tokens,
                    cost_usd: usage.cost_usd,
                    latency_ms: usage.latency_ms,
                    ttft_ms: usage.ttft_ms,
                })
                .await?;
        }

        if let Some((message, _)) = prev {
            self.inner
                .db
                .replace_message_blocks(message.id, &blocks)
                .await?;
        } else {
            let assistant_message = MessageNode {
                id: MessageNodeId::new(),
                conversation_id: conversation_id_parsed,
                branch_id,
                parent_message_id: Some(user_message.id),
                role: MessageRole::Assistant,
                round_id: Some(round.id),
                candidate_id: Some(candidate.id),
                created_at: Utc::now(),
            };
            self.inner
                .db
                .insert_message(&assistant_message, &blocks)
                .await?;
        }

        let completed = new_event(NewAnalyticsEvent {
            event_type: AnalyticsEventType::GenerationCompleted,
            workspace_id: self.current_workspace().await,
            conversation_id: Some(conversation_id_parsed),
            round_id: Some(round.id),
            candidate_id: Some(candidate.id),
            payload: serde_json::json!({ "retry": true }),
        });
        self.inner.db.insert_analytics_event(&completed).await?;
        self.inner
            .db
            .touch_conversation(conversation_id_parsed)
            .await?;
        self.get_conversation_ui(conversation_id).await
    }
}
