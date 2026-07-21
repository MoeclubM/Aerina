use aerina_domain::*;
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::{auth_bearer, EventStream};

/// OpenAI Responses API provider (`POST /responses`).
#[derive(Clone)]
pub struct OpenAiResponsesProvider {
    client: Client,
    config: ProviderConfig,
}

impl OpenAiResponsesProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn base(&self) -> &str {
        self.config.base_url.trim_end_matches('/')
    }

    fn responses_url(&self) -> String {
        format!("{}/responses", self.base())
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base())
    }

    fn build_input(request: &TextGenerationRequest) -> Result<Value> {
        let mut input = Vec::new();
        if let Some(system_prompt) = &request.system_prompt {
            input.push(json!({
                "role": "system",
                "content": [{"type": "input_text", "text": system_prompt}]
            }));
        }
        for message in &request.messages {
            let role = message.role.as_str();
            if role == "tool" {
                let output = match &message.content {
                    ChatContent::Text(text) => text.clone(),
                    ChatContent::Parts(parts) => parts
                        .iter()
                        .filter_map(|p| match p {
                            ChatContentPart::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                };
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": message.tool_call_id.clone().unwrap_or_default(),
                    "output": output,
                }));
                continue;
            }

            if role == "assistant" {
                if let Some(tool_calls) = &message.tool_calls {
                    if let ChatContent::Text(text) = &message.content {
                        if !text.is_empty() {
                            input.push(json!({
                                "role": "assistant",
                                "content": [{"type": "output_text", "text": text}]
                            }));
                        }
                    }
                    for call in tool_calls {
                        input.push(json!({
                            "type": "function_call",
                            "call_id": call.id,
                            "name": call.name,
                            "arguments": call.arguments,
                        }));
                    }
                    continue;
                }
            }

            let content = match &message.content {
                ChatContent::Text(text) => {
                    let ty = if role == "assistant" {
                        "output_text"
                    } else {
                        "input_text"
                    };
                    vec![json!({"type": ty, "text": text})]
                }
                ChatContent::Parts(parts) => {
                    let mut items = Vec::new();
                    for part in parts {
                        match part {
                            ChatContentPart::Text { text } => {
                                let ty = if role == "assistant" {
                                    "output_text"
                                } else {
                                    "input_text"
                                };
                                items.push(json!({"type": ty, "text": text}));
                            }
                            ChatContentPart::ImageUrl { image_url } => {
                                items.push(json!({
                                    "type": "input_image",
                                    "image_url": image_url.url
                                }));
                            }
                        }
                    }
                    items
                }
            };
            input.push(json!({"role": role, "content": content}));
        }
        Ok(Value::Array(input))
    }
}

#[async_trait]
impl crate::ModelProvider for OpenAiResponsesProvider {
    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let req = auth_bearer(self.client.get(self.models_url()), &self.config.api_key);
        let response = req.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        let data = body
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(enrich_model_list(data.into_iter().filter_map(|item| {
            let id = item.get("id")?.as_str()?.to_string();
            Some((id, None))
        })))
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
        let mut body = json!({
            "model": request.model,
            "input": Self::build_input(&request)?,
            "stream": true,
        });
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(request
                .tools
                .iter()
                .map(|t| json!({
                    "type": "function",
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }))
                .collect::<Vec<_>>());
        }

        let req = auth_bearer(
            self.client.post(self.responses_url()).json(&body),
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
            // call_id / item_id -> (id, name, args)
            let mut tool_acc: HashMap<String, (String, String, String)> = HashMap::new();

            while let Some(chunk) = byte_stream.next().await {
                if cancel.is_cancelled() {
                    yield GenerationEvent::Error {
                        candidate_id: candidate_id.clone(),
                        message: "cancelled".into(),
                    };
                    return;
                }
                let chunk = match chunk {
                    Ok(v) => v,
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
                    if line.is_empty() || line.starts_with("event:") {
                        continue;
                    }
                    if !line.starts_with("data:") {
                        continue;
                    }
                    let data = line.trim_start_matches("data:").trim();
                    if data == "[DONE]" {
                        break;
                    }
                    let Ok(parsed) = serde_json::from_str::<Value>(data) else {
                        continue;
                    };
                    let event_type = parsed
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    match event_type {
                        "response.output_text.delta" | "response.refusal.delta" => {
                            if let Some(delta) = parsed.get("delta").and_then(|v| v.as_str()) {
                                if !delta.is_empty() {
                                    if first_token_at.is_none() {
                                        first_token_at = Some(Instant::now());
                                    }
                                    yield GenerationEvent::TextDelta {
                                        candidate_id: candidate_id.clone(),
                                        delta: delta.to_string(),
                                    };
                                }
                            }
                        }
                        "response.reasoning_summary_text.delta"
                        | "response.reasoning_text.delta"
                        | "response.output_item.reasoning.delta" => {
                            if let Some(delta) = parsed.get("delta").and_then(|v| v.as_str()) {
                                if !delta.is_empty() {
                                    if first_token_at.is_none() {
                                        first_token_at = Some(Instant::now());
                                    }
                                    yield GenerationEvent::ThinkingDelta {
                                        candidate_id: candidate_id.clone(),
                                        delta: delta.to_string(),
                                    };
                                }
                            }
                        }
                        "response.output_item.added" | "response.output_item.done" => {
                            let item = parsed.get("item").cloned().unwrap_or(Value::Null);
                            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            if item_type == "function_call" {
                                let call_id = item
                                    .get("call_id")
                                    .or_else(|| item.get("id"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = item
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let args = item
                                    .get("arguments")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !call_id.is_empty() {
                                    let entry = tool_acc
                                        .entry(call_id.clone())
                                        .or_insert_with(|| (call_id, String::new(), String::new()));
                                    if !name.is_empty() {
                                        entry.1 = name;
                                    }
                                    if !args.is_empty() {
                                        entry.2 = args;
                                    }
                                }
                            }
                        }
                        "response.function_call_arguments.delta" => {
                            let item_id = parsed
                                .get("item_id")
                                .or_else(|| parsed.get("call_id"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let delta = parsed
                                .get("delta")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if !item_id.is_empty() && !delta.is_empty() {
                                let entry = tool_acc
                                    .entry(item_id.clone())
                                    .or_insert_with(|| (item_id, String::new(), String::new()));
                                entry.2.push_str(delta);
                            }
                        }
                        "response.function_call_arguments.done" => {
                            let item_id = parsed
                                .get("item_id")
                                .or_else(|| parsed.get("call_id"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let args = parsed
                                .get("arguments")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if !item_id.is_empty() {
                                let entry = tool_acc
                                    .entry(item_id.clone())
                                    .or_insert_with(|| (item_id, String::new(), String::new()));
                                if !args.is_empty() {
                                    entry.2 = args.to_string();
                                }
                            }
                        }
                        "response.completed" => {
                            if let Some(usage) = parsed.pointer("/response/usage") {
                                prompt_tokens = usage
                                    .get("input_tokens")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32)
                                    .or(prompt_tokens);
                                completion_tokens = usage
                                    .get("output_tokens")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32)
                                    .or(completion_tokens);
                                total_tokens = usage
                                    .get("total_tokens")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32)
                                    .or(total_tokens);
                            }
                        }
                        "error" => {
                            let message = parsed
                                .pointer("/error/message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("responses api error")
                                .to_string();
                            yield GenerationEvent::Error {
                                candidate_id: candidate_id.clone(),
                                message,
                            };
                            return;
                        }
                        _ => {}
                    }
                }
            }

            if !tool_acc.is_empty() {
                let mut calls = Vec::new();
                let mut keys: Vec<_> = tool_acc.keys().cloned().collect();
                keys.sort();
                for k in keys {
                    if let Some((id, name, args)) = tool_acc.get(&k) {
                        if !name.is_empty() {
                            calls.push(ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                arguments: args.clone(),
                            });
                        }
                    }
                }
                if !calls.is_empty() {
                    yield GenerationEvent::ToolCalls {
                        candidate_id: candidate_id.clone(),
                        calls,
                    };
                }
            }

            let latency_ms = started.elapsed().as_millis() as u64;
            let ttft_ms =
                first_token_at.map(|ts| ts.duration_since(started).as_millis() as u64);
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
        let chat = crate::openai_chat::OpenAiChatProvider::new(self.config.clone());
        chat.generate_image(candidate_id, slot_label, request, cancel)
            .await
    }
}
