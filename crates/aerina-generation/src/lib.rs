use aerina_domain::*;
use aerina_providers::{build_provider, EventStream};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
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

        let streams = targets
            .into_iter()
            .map(|target| {
                let candidate_id = target.candidate_id.to_string();
                let slot_label = target.slot_label.clone();
                let model_preset_id = target.preset.preset_id.to_string();
                let model_name = target.preset.model_name.clone();
                with_generation_metrics(
                    generate_target(
                        context.clone(),
                        target,
                        cancel.child_token(),
                        executor.clone(),
                    ),
                    cancel.child_token(),
                    candidate_id,
                    slot_label,
                    model_preset_id,
                    model_name,
                )
            })
            .collect::<Vec<_>>();

        Ok(Box::pin(stream::select_all(streams)))
    }
}

fn generate_target(
    context: RoundContext,
    target: GenerationTarget,
    cancel: CancellationToken,
    executor: Option<Arc<dyn ToolExecutor>>,
) -> EventStream {
    Box::pin(async_stream::stream! {
        let candidate_id = target.candidate_id.to_string();
        let slot_label = target.slot_label.clone();
        yield GenerationEvent::StreamStart {
            candidate_id: candidate_id.clone(),
            slot_label: slot_label.clone(),
        };
        yield GenerationEvent::CandidateStatus {
            candidate_id: candidate_id.clone(),
            slot_label: slot_label.clone(),
            model_preset_id: target.preset.preset_id.to_string(),
            model_name: target.preset.model_name.clone(),
            status: CandidateStatus::Streaming,
        };

        let provider = match build_provider(ProviderConfig {
            id: target.preset.provider_id,
            name: target.preset.display_name.clone(),
            kind: target.preset.provider_kind,
            base_url: target.preset.base_url.clone(),
            api_key: target.preset.api_key.clone(),
        }) {
            Ok(provider) => provider,
            Err(err) => {
                yield GenerationEvent::Error {
                    candidate_id,
                    message: err.to_string(),
                };
                return;
            }
        };

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
                yield GenerationEvent::Error {
                    candidate_id,
                    message: format!(
                        "model {} does not support image_generation",
                        target.preset.model_name
                    ),
                };
                return;
            }
            let prompt = context
                .image_prompt
                .clone()
                .or_else(|| {
                    context.messages.last().and_then(|message| match &message.content {
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
            match provider
                .generate_image(&candidate_id, &slot_label, request, cancel.child_token())
                .await
            {
                Ok(stream) => stream,
                Err(err) => {
                    yield GenerationEvent::Error {
                        candidate_id,
                        message: err.to_string(),
                    };
                    return;
                }
            }
        } else {
            if !target.preset.capabilities.contains(&CapabilityTag::Text) {
                yield GenerationEvent::Error {
                    candidate_id,
                    message: format!("model {} does not support text", target.preset.model_name),
                };
                return;
            }
            let has_image_parts = context.messages.iter().any(|message| match &message.content {
                ChatContent::Parts(parts) => parts
                    .iter()
                    .any(|part| matches!(part, ChatContentPart::ImageUrl { .. })),
                _ => false,
            });
            if has_image_parts && !target.preset.capabilities.contains(&CapabilityTag::Vision) {
                yield GenerationEvent::Error {
                    candidate_id,
                    message: format!("model {} does not support vision", target.preset.model_name),
                };
                return;
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
                let provider_config = ProviderConfig {
                    id: target.preset.provider_id,
                    name: target.preset.display_name.clone(),
                    kind: target.preset.provider_kind,
                    base_url: target.preset.base_url.clone(),
                    api_key: target.preset.api_key.clone(),
                };
                let tool_cancel = cancel.child_token();
                let tool_candidate_id = candidate_id.clone();
                let tool_slot_label = slot_label.clone();

                Box::pin(async_stream::stream! {
                    let provider = match build_provider(provider_config) {
                        Ok(provider) => provider,
                        Err(err) => {
                            yield GenerationEvent::Error {
                                candidate_id: tool_candidate_id.clone(),
                                message: err.to_string(),
                            };
                            return;
                        }
                    };

                    for _ in 0..6 {
                        if tool_cancel.is_cancelled() {
                            yield GenerationEvent::Error {
                                candidate_id: tool_candidate_id.clone(),
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
                            .generate_stream(
                                &tool_candidate_id,
                                &tool_slot_label,
                                request,
                                tool_cancel.child_token(),
                            )
                            .await
                        {
                            Ok(stream) => stream,
                            Err(err) => {
                                yield GenerationEvent::Error {
                                    candidate_id: tool_candidate_id.clone(),
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
                                GenerationEvent::StreamStart { .. } => {}
                                other => {
                                    if tool_calls.is_empty()
                                        || !matches!(
                                            other,
                                            GenerationEvent::Done { .. }
                                                | GenerationEvent::Usage { .. }
                                        )
                                    {
                                        yield event;
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
                            let args = serde_json::from_str::<Value>(&call.arguments)
                                .unwrap_or(Value::Object(Default::default()));
                            let result = match executor.execute(&call.name, args).await {
                                Ok(value) => value,
                                Err(err) => format!("tool error: {err}"),
                            };
                            messages.push(ChatMessage {
                                role: "tool".into(),
                                content: ChatContent::Text(result),
                                tool_call_id: Some(call.id),
                                tool_calls: None,
                            });
                        }
                    }

                    yield GenerationEvent::Error {
                        candidate_id: tool_candidate_id,
                        message: "tool loop exceeded max rounds".into(),
                    };
                }) as EventStream
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
                match provider
                    .generate_stream(&candidate_id, &slot_label, request, cancel.child_token())
                    .await
                {
                    Ok(stream) => stream,
                    Err(err) => {
                        yield GenerationEvent::Error {
                            candidate_id,
                            message: err.to_string(),
                        };
                        return;
                    }
                }
            }
        };

        futures::pin_mut!(stream);
        while let Some(event) = stream.next().await {
            if !matches!(event, GenerationEvent::StreamStart { .. }) {
                yield event;
            }
        }
    })
}

fn with_generation_metrics(
    mut stream: EventStream,
    cancel: CancellationToken,
    candidate_id: String,
    slot_label: String,
    model_preset_id: String,
    model_name: String,
) -> EventStream {
    Box::pin(async_stream::stream! {
        let started_at = Instant::now();
        let mut first_text_at = None;
        let mut thinking = String::new();
        let mut output = String::new();
        let mut usage_emitted = false;

        loop {
            let event = tokio::select! {
                _ = cancel.cancelled() => {
                    if !usage_emitted {
                        yield GenerationEvent::Usage {
                            candidate_id: candidate_id.clone(),
                            usage: finalize_usage(
                                UsageReport::default(),
                                started_at,
                                first_text_at,
                                &thinking,
                                &output,
                                Instant::now(),
                            ),
                        };
                    }
                    yield GenerationEvent::Error {
                        candidate_id: candidate_id.clone(),
                        message: "cancelled".into(),
                    };
                    yield GenerationEvent::CandidateStatus {
                        candidate_id,
                        slot_label,
                        model_preset_id,
                        model_name,
                        status: CandidateStatus::Cancelled,
                    };
                    return;
                }
                event = stream.next() => event,
            };

            let Some(event) = event else {
                if !usage_emitted {
                    yield GenerationEvent::Usage {
                        candidate_id: candidate_id.clone(),
                        usage: finalize_usage(
                            UsageReport::default(),
                            started_at,
                            first_text_at,
                            &thinking,
                            &output,
                            Instant::now(),
                        ),
                    };
                }
                yield GenerationEvent::Error {
                    candidate_id: candidate_id.clone(),
                    message: "generation stream ended without terminal event".into(),
                };
                yield GenerationEvent::CandidateStatus {
                    candidate_id,
                    slot_label,
                    model_preset_id,
                    model_name,
                    status: CandidateStatus::Failed,
                };
                return;
            };

            match event {
                GenerationEvent::ThinkingDelta { ref delta, .. } => {
                    thinking.push_str(delta);
                    yield event;
                }
                GenerationEvent::TextDelta { ref delta, .. } => {
                    if first_text_at.is_none() && !delta.is_empty() {
                        first_text_at = Some(Instant::now());
                    }
                    output.push_str(delta);
                    yield event;
                }
                GenerationEvent::Usage { candidate_id: event_candidate_id, usage } => {
                    usage_emitted = true;
                    yield GenerationEvent::Usage {
                        candidate_id: event_candidate_id,
                        usage: finalize_usage(
                            usage,
                            started_at,
                            first_text_at,
                            &thinking,
                            &output,
                            Instant::now(),
                        ),
                    };
                }
                GenerationEvent::Done { candidate_id: event_candidate_id } => {
                    if !usage_emitted {
                        yield GenerationEvent::Usage {
                            candidate_id: event_candidate_id.clone(),
                            usage: finalize_usage(
                                UsageReport::default(),
                                started_at,
                                first_text_at,
                                &thinking,
                                &output,
                                Instant::now(),
                            ),
                        };
                    }
                    yield GenerationEvent::Done {
                        candidate_id: event_candidate_id,
                    };
                    yield GenerationEvent::CandidateStatus {
                        candidate_id,
                        slot_label,
                        model_preset_id,
                        model_name,
                        status: CandidateStatus::Completed,
                    };
                    return;
                }
                GenerationEvent::Error { candidate_id: event_candidate_id, message } => {
                    if !usage_emitted {
                        yield GenerationEvent::Usage {
                            candidate_id: event_candidate_id.clone(),
                            usage: finalize_usage(
                                UsageReport::default(),
                                started_at,
                                first_text_at,
                                &thinking,
                                &output,
                                Instant::now(),
                            ),
                        };
                    }
                    let status = if message == "cancelled" {
                        CandidateStatus::Cancelled
                    } else {
                        CandidateStatus::Failed
                    };
                    yield GenerationEvent::Error {
                        candidate_id: event_candidate_id,
                        message,
                    };
                    yield GenerationEvent::CandidateStatus {
                        candidate_id,
                        slot_label,
                        model_preset_id,
                        model_name,
                        status,
                    };
                    return;
                }
                _ => yield event,
            }
        }
    })
}

fn finalize_usage(
    mut usage: UsageReport,
    started_at: Instant,
    first_text_at: Option<Instant>,
    thinking: &str,
    output: &str,
    finished_at: Instant,
) -> UsageReport {
    usage.latency_ms = Some(finished_at.duration_since(started_at).as_millis() as u64);
    usage.ttft_ms =
        first_text_at.map(|instant| instant.duration_since(started_at).as_millis() as u64);

    let reported_reasoning_tokens = usage.reasoning_tokens.filter(|value| *value > 0);
    if !thinking.is_empty() {
        if reported_reasoning_tokens.is_none() {
            usage.reasoning_tokens = Some(estimate_tokens(thinking));
        }
        usage.reasoning_duration_ms = Some(
            first_text_at
                .unwrap_or(finished_at)
                .duration_since(started_at)
                .as_millis() as u64,
        );
    }

    usage.output_tokens = if output.is_empty() {
        Some(0)
    } else if thinking.is_empty() {
        usage
            .completion_tokens
            .or_else(|| Some(estimate_tokens(output)))
    } else if let (Some(completion_tokens), Some(reasoning_tokens)) =
        (usage.completion_tokens, reported_reasoning_tokens)
    {
        if completion_tokens >= reasoning_tokens {
            Some(completion_tokens - reasoning_tokens)
        } else {
            Some(estimate_tokens(output))
        }
    } else {
        Some(estimate_tokens(output))
    };
    usage
}

fn estimate_tokens(text: &str) -> u32 {
    let mut tokens = 0u32;
    let mut ascii_run = 0u32;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '\'' | '_' | '-') {
            ascii_run += 1;
        } else {
            if ascii_run > 0 {
                tokens += ascii_run.div_ceil(4);
                ascii_run = 0;
            }
            if !ch.is_whitespace() {
                tokens += 1;
            }
        }
    }
    if ascii_run > 0 {
        tokens += ascii_run.div_ceil(4);
    }
    tokens.max(1)
}

impl Default for GenerationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_ttft_uses_first_answer_text_not_thinking() {
        let started_at = Instant::now();
        let usage = finalize_usage(
            UsageReport {
                prompt_tokens: Some(10),
                completion_tokens: Some(20),
                output_tokens: None,
                total_tokens: Some(30),
                cost_usd: None,
                latency_ms: Some(1),
                ttft_ms: Some(1),
                reasoning_tokens: Some(8),
                reasoning_duration_ms: Some(1),
            },
            started_at,
            Some(started_at + std::time::Duration::from_millis(3421)),
            "先分析问题",
            "最终答案",
            started_at + std::time::Duration::from_millis(3429),
        );

        assert_eq!(usage.latency_ms, Some(3429));
        assert_eq!(usage.ttft_ms, Some(3421));
        assert_eq!(usage.reasoning_duration_ms, Some(3421));
        assert_eq!(usage.output_tokens, Some(12));
    }

    #[test]
    fn summary_reasoning_duration_includes_hidden_wait() {
        let started_at = Instant::now();
        let usage = finalize_usage(
            UsageReport {
                prompt_tokens: None,
                completion_tokens: Some(12),
                output_tokens: None,
                total_tokens: None,
                cost_usd: None,
                latency_ms: Some(3429),
                ttft_ms: Some(3421),
                reasoning_tokens: None,
                reasoning_duration_ms: Some(1),
            },
            started_at,
            Some(started_at + std::time::Duration::from_millis(3421)),
            "总结思路",
            "回答",
            started_at + std::time::Duration::from_millis(3429),
        );

        assert_eq!(usage.reasoning_duration_ms, Some(3421));
        assert!(usage.reasoning_tokens.unwrap() > 0);
    }

    #[test]
    fn reasoning_token_fallback_handles_chinese_and_english() {
        assert_eq!(estimate_tokens("先分析问题再回答"), 8);
        assert!(estimate_tokens("reason through the complete answer") >= 7);
    }

    #[tokio::test]
    async fn emits_candidate_status_before_provider_initialization() {
        let candidate_id = CandidateId::new();
        let preset_id = ModelPresetId::new();
        let target = GenerationTarget {
            candidate_id,
            slot_label: "B".into(),
            preset: ResolvedModelPreset {
                preset_id,
                provider_id: ProviderId::new(),
                provider_kind: ProviderKind::Gemini,
                base_url: "http://unreachable.invalid".into(),
                api_key: None,
                model_name: "model-b".into(),
                display_name: "Model B".into(),
                capabilities: vec![CapabilityTag::Text],
                temperature: None,
                system_prompt: None,
            },
        };
        let mut stream = GenerationEngine::new()
            .generate(
                RoundContext {
                    messages: Vec::new(),
                    system_prompt: None,
                    temperature: None,
                    image_prompt: None,
                    image_size: None,
                    require_image: false,
                    tools: Vec::new(),
                },
                vec![target],
                CancellationToken::new(),
                None,
            )
            .await
            .unwrap();

        assert!(matches!(
            stream.next().await,
            Some(GenerationEvent::StreamStart { candidate_id: id, slot_label })
                if id == candidate_id.to_string() && slot_label == "B"
        ));
        assert!(matches!(
            stream.next().await,
            Some(GenerationEvent::CandidateStatus {
                candidate_id: id,
                slot_label,
                model_preset_id,
                model_name,
                status: CandidateStatus::Streaming,
            }) if id == candidate_id.to_string()
                && slot_label == "B"
                && model_preset_id == preset_id.to_string()
                && model_name == "model-b"
        ));
    }

    #[tokio::test]
    async fn cancellation_emits_metrics_and_terminal_candidate_status() {
        let cancel = CancellationToken::new();
        let mut stream = with_generation_metrics(
            Box::pin(stream::pending()),
            cancel.clone(),
            "candidate".into(),
            "A".into(),
            "preset".into(),
            "model".into(),
        );
        cancel.cancel();

        assert!(matches!(
            stream.next().await,
            Some(GenerationEvent::Usage { .. })
        ));
        assert!(matches!(
            stream.next().await,
            Some(GenerationEvent::Error { message, .. }) if message == "cancelled"
        ));
        assert!(matches!(
            stream.next().await,
            Some(GenerationEvent::CandidateStatus {
                status: CandidateStatus::Cancelled,
                ..
            })
        ));
    }
}
