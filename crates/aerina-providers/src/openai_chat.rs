use aerina_domain::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::{auth_bearer, EventStream};

#[derive(Clone)]
pub struct OpenAiChatProvider {
    client: Client,
    config: ProviderConfig,
}

impl OpenAiChatProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn base(&self) -> &str {
        self.config.base_url.trim_end_matches('/')
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base())
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base())
    }

    fn images_url(&self) -> String {
        format!("{}/images/generations", self.base())
    }
}

#[async_trait]
impl crate::ModelProvider for OpenAiChatProvider {
    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let req = auth_bearer(self.client.get(self.models_url()), &self.config.api_key);
        let response = req.send().await?.error_for_status()?;
        let body: ModelsResponse = response.json().await?;
        Ok(enrich_model_list(
            body.data.into_iter().map(|item| (item.id, None)),
        ))
    }

    async fn validate_config(&self) -> Result<()> {
        let _ = self.list_models().await?;
        Ok(())
    }

    async fn generate_stream(
        &self,
        candidate_id: &str,
        slot_label: &str,
        request: TextGenerationRequest,
        cancel: CancellationToken,
    ) -> Result<EventStream> {
        let mut messages = Vec::new();
        if let Some(system_prompt) = request.system_prompt.clone() {
            messages.push(json!({"role": "system", "content": system_prompt}));
        }
        for message in &request.messages {
            messages.push(openai_message(message)?);
        }

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "temperature": request.temperature,
        });
        if !request.tools.is_empty() {
            let tools: Vec<_> = request
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = json!(tools);
            if let Some(choice) = &request.tool_choice {
                body["tool_choice"] = json!(choice);
            }
        }

        let req = auth_bearer(
            self.client.post(self.chat_url()).json(&body),
            &self.config.api_key,
        );
        let response = req.send().await?.error_for_status()?;
        let candidate_id = candidate_id.to_string();
        let slot_label = slot_label.to_string();
        let started = Instant::now();
        let mut first_token_at: Option<Instant> = None;

        let stream = async_stream::stream! {
            yield GenerationEvent::StreamStart {
                candidate_id: candidate_id.clone(),
                slot_label: slot_label.clone(),
            };

            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut prompt_tokens = None;
            let mut completion_tokens = None;
            let mut total_tokens = None;
            let mut tool_acc: std::collections::HashMap<u32, (String, String, String)> = std::collections::HashMap::new();

            while let Some(chunk) = byte_stream.next().await {
                if cancel.is_cancelled() {
                    yield GenerationEvent::Error {
                        candidate_id: candidate_id.clone(),
                        message: "cancelled".into(),
                    };
                    return;
                }

                let chunk = match chunk {
                    Ok(value) => value,
                    Err(err) => {
                        yield GenerationEvent::Error {
                            candidate_id: candidate_id.clone(),
                            message: err.to_string(),
                        };
                        return;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(index) = buffer.find('\n') {
                    let line = buffer[..index].trim().to_string();
                    buffer = buffer[index + 1..].to_string();
                    if line.is_empty() || !line.starts_with("data:") {
                        continue;
                    }
                    let data = line.trim_start_matches("data:").trim();
                    if data == "[DONE]" {
                        if !tool_acc.is_empty() {
                            let mut calls = Vec::new();
                            let mut keys: Vec<_> = tool_acc.keys().copied().collect();
                            keys.sort();
                            for k in keys {
                                if let Some((id, name, args)) = tool_acc.get(&k) {
                                    calls.push(ToolCall {
                                        id: id.clone(),
                                        name: name.clone(),
                                        arguments: args.clone(),
                                    });
                                }
                            }
                            yield GenerationEvent::ToolCalls {
                                candidate_id: candidate_id.clone(),
                                calls,
                            };
                        }
                        let latency_ms = started.elapsed().as_millis() as u64;
                        let ttft_ms = first_token_at.map(|ts| ts.duration_since(started).as_millis() as u64);
                        yield GenerationEvent::Usage {
                            candidate_id: candidate_id.clone(),
                            usage: UsageReport {
                                prompt_tokens,
                                completion_tokens,
                                total_tokens,
                                cost_usd: None,
                                latency_ms: Some(latency_ms),
                                ttft_ms,
                            },
                        };
                        yield GenerationEvent::Done {
                            candidate_id: candidate_id.clone(),
                        };
                        return;
                    }

                    match serde_json::from_str::<ChatChunk>(data) {
                        Ok(parsed) => {
                            if let Some(usage) = parsed.usage {
                                prompt_tokens = usage.prompt_tokens.or(prompt_tokens);
                                completion_tokens = usage.completion_tokens.or(completion_tokens);
                                total_tokens = usage.total_tokens.or(total_tokens);
                            }
                            if let Some(choice) = parsed.choices.first() {
                                let thinking = choice
                                    .delta
                                    .reasoning_content
                                    .as_ref()
                                    .or(choice.delta.reasoning.as_ref())
                                    .or(choice.delta.thinking.as_ref());
                                if let Some(delta) = thinking {
                                    if !delta.is_empty() {
                                        if first_token_at.is_none() {
                                            first_token_at = Some(Instant::now());
                                        }
                                        yield GenerationEvent::ThinkingDelta {
                                            candidate_id: candidate_id.clone(),
                                            delta: delta.clone(),
                                        };
                                    }
                                }
                                if let Some(delta) = &choice.delta.content {
                                    if !delta.is_empty() {
                                        if first_token_at.is_none() {
                                            first_token_at = Some(Instant::now());
                                        }
                                        yield GenerationEvent::TextDelta {
                                            candidate_id: candidate_id.clone(),
                                            delta: delta.clone(),
                                        };
                                    }
                                }
                                if let Some(tool_calls) = &choice.delta.tool_calls {
                                    for tc in tool_calls {
                                        let entry = tool_acc.entry(tc.index.unwrap_or(0)).or_insert_with(|| {
                                            (String::new(), String::new(), String::new())
                                        });
                                        if let Some(id) = &tc.id {
                                            if !id.is_empty() {
                                                entry.0 = id.clone();
                                            }
                                        }
                                        if let Some(func) = &tc.function {
                                            if let Some(name) = &func.name {
                                                entry.1.push_str(name);
                                            }
                                            if let Some(args) = &func.arguments {
                                                entry.2.push_str(args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            yield GenerationEvent::Error {
                                candidate_id: candidate_id.clone(),
                                message: format!("invalid stream chunk: {err}; data={data}"),
                            };
                            return;
                        }
                    }
                }
            }

            yield GenerationEvent::Done {
                candidate_id: candidate_id.clone(),
            };
        };

        Ok(Box::pin(stream))
    }

    async fn generate_image(
        &self,
        candidate_id: &str,
        slot_label: &str,
        request: ImageGenerationRequest,
        cancel: CancellationToken,
    ) -> Result<EventStream> {
        if cancel.is_cancelled() {
            return Err(anyhow!("cancelled"));
        }
        let body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "size": request.size.unwrap_or_else(|| "1024x1024".into()),
            "response_format": "b64_json",
        });
        let req = auth_bearer(
            self.client.post(self.images_url()).json(&body),
            &self.config.api_key,
        );
        let response = req.send().await?.error_for_status()?;
        let payload: ImageResponse = response.json().await?;
        let image = payload
            .data
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("image provider returned empty data"))?;
        let candidate_id = candidate_id.to_string();
        let slot_label = slot_label.to_string();
        let stream = async_stream::stream! {
            yield GenerationEvent::StreamStart {
                candidate_id: candidate_id.clone(),
                slot_label: slot_label.clone(),
            };
            yield GenerationEvent::ImageReady {
                candidate_id: candidate_id.clone(),
                image: GeneratedImage {
                    b64_json: image.b64_json,
                    url: image.url,
                    width: None,
                    height: None,
                    mime: "image/png".into(),
                    revised_prompt: image.revised_prompt,
                },
            };
            yield GenerationEvent::Done {
                candidate_id: candidate_id.clone(),
            };
        };
        Ok(Box::pin(stream))
    }
}

pub fn openai_message(message: &ChatMessage) -> Result<serde_json::Value> {
    let mut value = match &message.content {
        ChatContent::Text(text) => json!({"role": message.role, "content": text}),
        ChatContent::Parts(parts) => {
            let mut out = Vec::new();
            for part in parts {
                match part {
                    ChatContentPart::Text { text } => {
                        out.push(json!({"type": "text", "text": text}));
                    }
                    ChatContentPart::ImageUrl { image_url } => {
                        out.push(json!({
                            "type": "image_url",
                            "image_url": {"url": image_url.url}
                        }));
                    }
                }
            }
            json!({"role": message.role, "content": out})
        }
    };
    if let Some(tool_call_id) = &message.tool_call_id {
        value["tool_call_id"] = json!(tool_call_id);
    }
    if let Some(tool_calls) = &message.tool_calls {
        value["tool_calls"] = json!(tool_calls
            .iter()
            .map(|call| json!({
                "id": call.id,
                "type": "function",
                "function": {
                    "name": call.name,
                    "arguments": call.arguments,
                }
            }))
            .collect::<Vec<_>>());
        // OpenAI expects content null or string alongside tool_calls.
        if value.get("content").and_then(|c| c.as_str()) == Some("") {
            value["content"] = Value::Null;
        }
    }
    Ok(value)
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelItem>,
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ChatChunk {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    delta: ChatDelta,
}

#[derive(Debug, Deserialize)]
struct ChatDelta {
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
    tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct DeltaToolCall {
    index: Option<u32>,
    id: Option<String>,
    function: Option<DeltaFunction>,
}

#[derive(Debug, Deserialize)]
struct DeltaFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ImageResponse {
    data: Vec<ImageItem>,
}

#[derive(Debug, Deserialize)]
struct ImageItem {
    b64_json: Option<String>,
    url: Option<String>,
    revised_prompt: Option<String>,
}
