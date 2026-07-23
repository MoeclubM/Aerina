use super::*;
use aerina_analytics::new_event;
use aerina_arena::draw_arena_slots;
use aerina_conversation::build_chat_messages;
use aerina_generation::{GenerationTarget, RoundContext};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures::StreamExt;

impl AppState {
    pub async fn send_message<F>(
        &self,
        request: SendMessageRequest,
        on_event: F,
    ) -> Result<ConversationDetail>
    where
        F: FnMut(GenerationEvent) + Send,
    {
        self.run_generation(request, None, on_event).await
    }

    pub async fn regenerate<F>(
        &self,
        conversation_id: &str,
        on_event: F,
    ) -> Result<ConversationDetail>
    where
        F: FnMut(GenerationEvent) + Send,
    {
        let detail = self.get_conversation_ui(conversation_id).await?;
        let last_user = detail
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.message.role, MessageRole::User))
            .ok_or_else(|| anyhow!("no user message to regenerate"))?;

        // Move branch head back to the parent of the last user message and resend.
        let conversation_id_parsed = parse_entity_id(conversation_id)?;
        let branch_id = detail
            .conversation
            .active_branch_id
            .ok_or_else(|| anyhow!("missing active branch"))?;
        let mut branch = self
            .inner
            .db
            .get_branch(branch_id)
            .await?
            .ok_or_else(|| anyhow!("branch not found"))?;
        branch.head_message_id = last_user.message.parent_message_id;
        self.inner.db.upsert_branch(&branch).await?;

        let content = last_user
            .blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let image_data_urls = {
            let mut urls = Vec::new();
            for block in &last_user.blocks {
                if let ContentBlock::Image { media_id, .. } = block {
                    if let Some(media) = self.inner.db.get_media_object(*media_id).await? {
                        urls.push(
                            self.inner
                                .media
                                .data_url(&media.relative_path, &media.mime)
                                .await?,
                        );
                    }
                }
            }
            if urls.is_empty() {
                None
            } else {
                Some(urls)
            }
        };

        self.run_generation(
            SendMessageRequest {
                conversation_id: conversation_id.to_string(),
                content,
                image_data_urls,
                require_image: None,
                image_size: detail.settings.image_size.clone(),
            },
            Some(conversation_id_parsed),
            on_event,
        )
        .await
    }

    pub async fn run_generation<F>(
        &self,
        request: SendMessageRequest,
        _ignored: Option<EntityId>,
        mut on_event: F,
    ) -> Result<ConversationDetail>
    where
        F: FnMut(GenerationEvent) + Send,
    {
        let conversation_id = parse_entity_id(&request.conversation_id)?;
        let conversation = self
            .inner
            .db
            .get_conversation(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found"))?;
        let settings = self
            .inner
            .db
            .get_settings(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("settings not found"))?;
        let branch_id = conversation
            .active_branch_id
            .ok_or_else(|| anyhow!("missing active branch"))?;
        let mut branch = self
            .inner
            .db
            .get_branch(branch_id)
            .await?
            .ok_or_else(|| anyhow!("branch not found"))?;

        let history = self.inner.db.list_messages(branch_id).await?;
        let parent_message_id = branch.head_message_id;
        let user_message = tree::create_user_message(conversation_id, branch_id, parent_message_id);

        let mut user_blocks = vec![ContentBlock::text(request.content.clone())];
        if let Some(images) = &request.image_data_urls {
            for data_url in images {
                let (mime, bytes) = MediaStore::decode_data_url(data_url)?;
                let ext = if mime.contains("png") {
                    "png"
                } else if mime.contains("jpeg") || mime.contains("jpg") {
                    "jpg"
                } else if mime.contains("webp") {
                    "webp"
                } else {
                    "bin"
                };
                let (relative, _) = self.inner.media.save_attachment(&bytes, ext).await?;
                let media = MediaObject {
                    id: MediaObjectId::new(),
                    workspace_id: self.current_workspace().await,
                    relative_path: relative,
                    mime,
                    width: None,
                    height: None,
                    created_at: Utc::now(),
                };
                self.inner.db.insert_media_object(&media).await?;
                user_blocks.push(ContentBlock::Image {
                    media_id: media.id,
                    alt: None,
                    revised_prompt: None,
                });
            }
        }

        self.inner
            .db
            .insert_message(&user_message, &user_blocks)
            .await?;
        branch.head_message_id = Some(user_message.id);
        self.inner.db.upsert_branch(&branch).await?;

        // Auto-title first user message.
        if history.is_empty() {
            let title = request.content.chars().take(48).collect::<String>();
            if !title.trim().is_empty() {
                self.inner
                    .db
                    .update_conversation_title(conversation_id, title.trim())
                    .await?;
            }
        }

        let round = tree::create_round(conversation_id, branch_id, user_message.id);
        let snapshot = RoundSnapshot {
            round_id: round.id,
            mode: settings.mode,
            system_prompt: settings.system_prompt.clone(),
            temperature: settings.temperature,
            model_preset_ids: settings.model_preset_ids.clone(),
            arena_kind: settings.arena_kind,
            arena_category: settings.arena_category.clone(),
            image_size: request.image_size.clone().or(settings.image_size.clone()),
            image_aspect_ratio: settings.image_aspect_ratio.clone(),
            created_at: Utc::now(),
        };
        self.inner.db.insert_round(&round, &snapshot).await?;

        let mut targets = Vec::new();
        let mut candidates = Vec::new();
        let require_image = request.require_image.unwrap_or(false);
        let capability_requirements = if require_image {
            vec![CapabilityTag::ImageGeneration]
        } else if request
            .image_data_urls
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            vec![CapabilityTag::Text, CapabilityTag::Vision]
        } else {
            vec![CapabilityTag::Text]
        };

        let selected_presets: Vec<(ModelPresetId, String, bool)> =
            if settings.mode == ConversationMode::Arena {
                let pool_ids = if settings.candidate_pool.is_empty() {
                    settings.model_preset_ids.clone()
                } else {
                    settings.candidate_pool.clone()
                };
                let mut resolved_pool = Vec::new();
                for id in &pool_ids {
                    resolved_pool.push(self.resolve_preset(*id).await?);
                }
                let draw = draw_arena_slots(
                    &ArenaDrawRequest {
                        kind: settings.arena_kind.unwrap_or(ArenaKind::Text),
                        candidate_pool: pool_ids,
                        slot_count: settings.slot_count.max(2) as usize,
                        capability_requirements: capability_requirements.clone(),
                        allow_same_provider: true,
                    },
                    &resolved_pool,
                )?;
                draw.into_iter()
                    .map(|slot| (slot.model_preset_id, slot.slot_label, true))
                    .collect()
            } else if settings.mode == ConversationMode::Chat {
                let id = settings
                    .model_preset_ids
                    .first()
                    .copied()
                    .ok_or_else(|| anyhow!("conversation has no model presets configured"))?;
                vec![(id, "A".into(), false)]
            } else {
                settings
                    .model_preset_ids
                    .iter()
                    .enumerate()
                    .map(|(index, id)| (*id, format!("{}", (b'A' + index as u8) as char), false))
                    .collect()
            };

        if selected_presets.is_empty() {
            return Err(anyhow!("conversation has no model presets configured"));
        }

        for (preset_id, slot, anonymous) in selected_presets {
            let resolved = self.resolve_preset(preset_id).await?;
            for cap in &capability_requirements {
                if !resolved.capabilities.contains(cap) {
                    return Err(anyhow!(
                        "model {} missing required capability {:?}",
                        resolved.model_name,
                        cap
                    ));
                }
            }
            let candidate = tree::create_candidate(
                round.id,
                slot.clone(),
                resolved.preset_id,
                resolved.provider_id,
                resolved.model_name.clone(),
                anonymous,
            );
            self.inner.db.insert_candidate(&candidate).await?;
            let started = new_event(NewAnalyticsEvent {
                event_type: AnalyticsEventType::GenerationStarted,
                workspace_id: self.current_workspace().await,
                conversation_id: Some(conversation_id),
                round_id: Some(round.id),
                candidate_id: Some(candidate.id),
                payload: serde_json::json!({
                    "model": resolved.model_name,
                    "slot": slot,
                    "anonymous": candidate.anonymous,
                    "require_image": require_image,
                }),
            });
            self.inner.db.insert_analytics_event(&started).await?;
            targets.push(GenerationTarget {
                candidate_id: candidate.id,
                slot_label: slot,
                preset: resolved,
            });
            candidates.push(candidate);
        }

        let mut history_with_user = history;
        history_with_user.push((user_message.clone(), user_blocks.clone()));

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

        let wants_tools = targets
            .iter()
            .any(|t| t.preset.capabilities.contains(&CapabilityTag::ToolCalling));
        let (tools, executor) = if wants_tools {
            self.build_tool_executor().await?
        } else {
            (Vec::new(), None)
        };

        let context = RoundContext {
            messages: build_chat_messages(&history_with_user, &media_urls),
            system_prompt: settings.system_prompt.clone(),
            temperature: settings.temperature,
            image_prompt: if require_image {
                Some(request.content.clone())
            } else {
                None
            },
            image_size: request.image_size.clone().or(settings.image_size.clone()),
            require_image,
            tools,
        };

        let cancel = CancellationToken::new();
        {
            let mut jobs = self.inner.active_jobs.lock().await;
            jobs.insert(request.conversation_id.clone(), cancel.clone());
        }

        let mut stream = self
            .inner
            .generation
            .generate(context, targets, cancel, executor)
            .await?;

        let mut outputs: HashMap<String, String> = HashMap::new();
        let mut thoughts: HashMap<String, String> = HashMap::new();
        let mut images: HashMap<String, GeneratedImage> = HashMap::new();
        let mut usages: HashMap<String, UsageReport> = HashMap::new();
        let mut failures: HashMap<String, String> = HashMap::new();

        while let Some(event) = stream.next().await {
            match &event {
                GenerationEvent::TextDelta {
                    candidate_id,
                    delta,
                } => {
                    outputs
                        .entry(candidate_id.clone())
                        .or_default()
                        .push_str(delta);
                }
                GenerationEvent::ThinkingDelta {
                    candidate_id,
                    delta,
                } => {
                    thoughts
                        .entry(candidate_id.clone())
                        .or_default()
                        .push_str(delta);
                }
                GenerationEvent::ImageReady {
                    candidate_id,
                    image,
                } => {
                    images.insert(candidate_id.clone(), image.clone());
                }
                GenerationEvent::Usage {
                    candidate_id,
                    usage,
                } => {
                    usages.insert(candidate_id.clone(), usage.clone());
                }
                GenerationEvent::Error {
                    candidate_id,
                    message,
                } => {
                    failures.insert(candidate_id.clone(), message.clone());
                }
                _ => {}
            }
            on_event(event);
        }

        {
            let mut jobs = self.inner.active_jobs.lock().await;
            jobs.remove(&request.conversation_id);
        }

        for candidate in candidates {
            let id = candidate.id.to_string();
            let mut candidate = self
                .inner
                .db
                .get_candidate(candidate.id)
                .await?
                .ok_or_else(|| anyhow!("candidate missing after generation"))?;

            if let Some(message) = failures.get(&id) {
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
                    conversation_id: Some(conversation_id),
                    round_id: Some(round.id),
                    candidate_id: Some(candidate.id),
                    payload: serde_json::json!({ "error": message }),
                });
                self.inner.db.insert_analytics_event(&failed).await?;
                continue;
            }

            candidate.status = CandidateStatus::Completed;
            candidate.completed_at = Some(Utc::now());
            self.inner.db.update_candidate(&candidate).await?;

            let mut blocks = Vec::new();
            let usage = usages.get(&id);
            let thinking = thoughts.get(&id).cloned().unwrap_or_default();
            if !thinking.is_empty()
                || usage.and_then(|value| value.reasoning_tokens).is_some()
                || usage
                    .and_then(|value| value.reasoning_duration_ms)
                    .is_some()
            {
                blocks.push(ContentBlock::Thinking {
                    text: thinking,
                    reasoning_tokens: usage.and_then(|value| value.reasoning_tokens),
                    reasoning_duration_ms: usage.and_then(|value| value.reasoning_duration_ms),
                });
            }
            if let Some(text) = outputs.get(&id) {
                if !text.is_empty() {
                    blocks.push(ContentBlock::text(text.clone()));
                }
            }
            if let Some(image) = images.get(&id) {
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
                    conversation_id: Some(conversation_id),
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

            if settings.mode == ConversationMode::Chat {
                let commit = tree::commit_candidate(
                    branch.clone(),
                    round.clone(),
                    &candidate,
                    user_message.id,
                )?;
                self.inner
                    .db
                    .insert_message(&commit.assistant_message, &blocks)
                    .await?;
                self.inner.db.upsert_branch(&commit.branch).await?;
                self.inner.db.update_round(&commit.round).await?;
            } else {
                let assistant_message = MessageNode {
                    id: MessageNodeId::new(),
                    conversation_id,
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
                conversation_id: Some(conversation_id),
                round_id: Some(round.id),
                candidate_id: Some(candidate.id),
                payload: serde_json::json!({ "mode": settings.mode }),
            });
            self.inner.db.insert_analytics_event(&completed).await?;
        }

        self.inner.db.touch_conversation(conversation_id).await?;
        self.get_conversation_ui(&request.conversation_id).await
    }
}
