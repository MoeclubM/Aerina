use aerina_domain::*;
use aerina_providers::{build_provider, EventStream};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct GenerationTarget {
    pub candidate_id: CandidateId,
    pub slot_label: String,
    pub preset: ResolvedModelPreset,
}

#[derive(Clone)]
pub struct RoundContext {
    pub messages: Vec<ChatMessage>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub image_prompt: Option<String>,
    pub image_size: Option<String>,
    pub require_image: bool,
    pub tools: Vec<ToolDefinition>,
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, name: &str, arguments: Value) -> Result<String>;
}

pub struct GenerationEngine;

impl GenerationEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn generate(
        &self,
        context: RoundContext,
        targets: Vec<GenerationTarget>,
        cancel: CancellationToken,
        executor: Option<Arc<dyn ToolExecutor>>,
    ) -> Result<EventStream> {
        if targets.is_empty() {
            return Err(anyhow!("no generation targets"));
        }

        let mut streams = Vec::new();
        for target in targets {
            let child_cancel = cancel.child_token();
            let provider = build_provider(ProviderConfig {
                id: target.preset.provider_id,
                name: target.preset.display_name.clone(),
                kind: target.preset.provider_kind,
                base_url: target.preset.base_url.clone(),
                api_key: target.preset.api_key.clone(),
            })?;

            let requires_image = context.require_image
                || (target
                    .preset
                    .capabilities
                    .contains(&CapabilityTag::ImageGeneration)
                    && context.image_prompt.is_some()
                    && !target.preset.capabilities.contains(&CapabilityTag::Text));

            let stream = if requires_image {
                if !target
                    .preset
                    .capabilities
                    .contains(&CapabilityTag::ImageGeneration)
                {
                    return Err(anyhow!(
                        "model {} does not support image_generation",
                        target.preset.model_name
                    ));
                }
                let prompt = context
                    .image_prompt
                    .clone()
                    .or_else(|| {
                        context.messages.last().and_then(|m| match &m.content {
                            ChatContent::Text(text) => Some(text.clone()),
                            ChatContent::Parts(parts) => parts.iter().find_map(|part| match part {
                                ChatContentPart::Text { text } => Some(text.clone()),
                                _ => None,
                            }),
                        })
                    })
                    .unwrap_or_default();
                let request = ImageGenerationRequest {
                    model: target.preset.model_name.clone(),
                    prompt,
                    negative_prompt: None,
                    size: context.image_size.clone(),
                    aspect_ratio: None,
                    seed: None,
                };
                provider
                    .generate_image(
                        &target.candidate_id.to_string(),
                        &target.slot_label,
                        request,
                        child_cancel,
                    )
                    .await?
            } else {
                if !target.preset.capabilities.contains(&CapabilityTag::Text) {
                    return Err(anyhow!(
                        "model {} does not support text",
                        target.preset.model_name
                    ));
                }
                let has_image_parts = context.messages.iter().any(|m| match &m.content {
                    ChatContent::Parts(parts) => parts
                        .iter()
                        .any(|p| matches!(p, ChatContentPart::ImageUrl { .. })),
                    _ => false,
                });
                if has_image_parts && !target.preset.capabilities.contains(&CapabilityTag::Vision) {
                    return Err(anyhow!(
                        "model {} does not support vision",
                        target.preset.model_name
                    ));
                }

                let use_tools = !context.tools.is_empty()
                    && target
                        .preset
                        .capabilities
                        .contains(&CapabilityTag::ToolCalling)
                    && executor.is_some();

                if use_tools {
                    let executor = executor.clone().unwrap();
                    let tools = context.tools.clone();
                    let mut messages = context.messages.clone();
                    let system_prompt = context
                        .system_prompt
                        .clone()
                        .or(target.preset.system_prompt.clone());
                    let temperature = context.temperature.or(target.preset.temperature);
                    let model = target.preset.model_name.clone();
                    let candidate_id = target.candidate_id.to_string();
                    let slot_label = target.slot_label.clone();
                    let provider_kind = target.preset.provider_kind;
                    let provider_config = ProviderConfig {
                        id: target.preset.provider_id,
                        name: target.preset.display_name.clone(),
                        kind: provider_kind,
                        base_url: target.preset.base_url.clone(),
                        api_key: target.preset.api_key.clone(),
                    };

                    let stream = async_stream::stream! {
                        let provider = match build_provider(provider_config) {
                            Ok(p) => p,
                            Err(err) => {
                                yield GenerationEvent::Error {
                                    candidate_id: candidate_id.clone(),
                                    message: err.to_string(),
                                };
                                return;
                            }
                        };

                        for _round in 0..6 {
                            if child_cancel.is_cancelled() {
                                yield GenerationEvent::Error {
                                    candidate_id: candidate_id.clone(),
                                    message: "cancelled".into(),
                                };
                                return;
                            }
                            let request = TextGenerationRequest {
                                model: model.clone(),
                                messages: messages.clone(),
                                temperature,
                                system_prompt: system_prompt.clone(),
                                tools: tools.clone(),
                                tool_choice: Some("auto".into()),
                            };
                            let mut sub = match provider
                                .generate_stream(&candidate_id, &slot_label, request, child_cancel.child_token())
                                .await
                            {
                                Ok(s) => s,
                                Err(err) => {
                                    yield GenerationEvent::Error {
                                        candidate_id: candidate_id.clone(),
                                        message: err.to_string(),
                                    };
                                    return;
                                }
                            };

                            let mut text = String::new();
                            let mut tool_calls: Vec<ToolCall> = Vec::new();
                            while let Some(event) = sub.next().await {
                                match &event {
                                    GenerationEvent::TextDelta { delta, .. } => {
                                        text.push_str(delta);
                                        yield event;
                                    }
                                    GenerationEvent::ToolCalls { calls, .. } => {
                                        tool_calls = calls.clone();
                                        yield event;
                                    }
                                    GenerationEvent::Error { .. } => {
                                        yield event;
                                        return;
                                    }
                                    GenerationEvent::StreamStart { .. } if _round > 0 => {
                                        // skip duplicate stream start
                                    }
                                    other => {
                                        // forward usage/done only on final
                                        if tool_calls.is_empty()
                                            || !matches!(other, GenerationEvent::Done { .. } | GenerationEvent::Usage { .. })
                                        {
                                            yield event;
                                        } else if matches!(other, GenerationEvent::Usage { .. }) {
                                            // keep last usage for final
                                        }
                                    }
                                }
                            }

                            if tool_calls.is_empty() {
                                return;
                            }

                            messages.push(ChatMessage {
                                role: "assistant".into(),
                                content: ChatContent::Text(text),
                                tool_call_id: None,
                                tool_calls: Some(tool_calls.clone()),
                            });
                            for call in tool_calls {
                                let args: Value = serde_json::from_str(&call.arguments)
                                    .unwrap_or(Value::Object(Default::default()));
                                let result = match executor.execute(&call.name, args).await {
                                    Ok(v) => v,
                                    Err(err) => format!("tool error: {err}"),
                                };
                                messages.push(ChatMessage {
                                    role: "tool".into(),
                                    content: ChatContent::Text(result),
                                    tool_call_id: Some(call.id.clone()),
                                    tool_calls: None,
                                });
                            }
                        }

                        yield GenerationEvent::Error {
                            candidate_id: candidate_id.clone(),
                            message: "tool loop exceeded max rounds".into(),
                        };
                    };
                    Box::pin(stream) as EventStream
                } else {
                    let request = TextGenerationRequest {
                        model: target.preset.model_name.clone(),
                        messages: context.messages.clone(),
                        temperature: context.temperature.or(target.preset.temperature),
                        system_prompt: context
                            .system_prompt
                            .clone()
                            .or(target.preset.system_prompt.clone()),
                        tools: Vec::new(),
                        tool_choice: None,
                    };
                    provider
                        .generate_stream(
                            &target.candidate_id.to_string(),
                            &target.slot_label,
                            request,
                            child_cancel,
                        )
                        .await?
                }
            };
            streams.push(stream);
        }

        let merged = stream::select_all(streams);
        Ok(Box::pin(merged))
    }
}

impl Default for GenerationEngine {
    fn default() -> Self {
        Self::new()
    }
}
