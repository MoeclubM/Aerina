use aerina_domain::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::EventStream;

#[derive(Clone)]
pub struct AnthropicProvider {
    client: Client,
    config: ProviderConfig,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn base(&self) -> &str {
        self.config.base_url.trim_end_matches('/')
    }

    fn messages_url(&self) -> String {
        let base = self.base();
        if base.ends_with("/v1") {
            format!("{base}/messages")
        } else if base.contains("/messages") {
            base.to_string()
        } else {
            format!("{base}/v1/messages")
        }
    }

    fn models_url(&self) -> String {
        let base = self.base();
        if base.ends_with("/v1") {
            format!("{base}/models")
        } else if base.ends_with("/models") {
            base.to_string()
        } else {
            format!("{base}/v1/models")
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut req = req
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");
        if let Some(key) = &self.config.api_key {
            if !key.is_empty() {
                req = req.header("x-api-key", key);
            }
        }
        req
    }

    fn convert_messages(request: &TextGenerationRequest) -> Result<(Option<String>, Vec<Value>)> {
        let system = request.system_prompt.clone();
        let mut messages = Vec::new();
        for message in &request.messages {
            if message.role == "tool" {
                let text = match &message.content {
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
                messages.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": message.tool_call_id.clone().unwrap_or_default(),
                        "content": text,
                    }]
                }));
                continue;
            }

            if message.role == "assistant" {
                if let Some(tool_calls) = &message.tool_calls {
                    let mut content = Vec::new();
                    match &message.content {
                        ChatContent::Text(text) if !text.is_empty() => {
                            content.push(json!({"type": "text", "text": text}));
                        }
                        ChatContent::Parts(parts) => {
                            for part in parts {
                                if let ChatContentPart::Text { text } = part {
                                    if !text.is_empty() {
                                        content.push(json!({"type": "text", "text": text}));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    for call in tool_calls {
                        let input = serde_json::from_str::<Value>(&call.arguments)
                            .unwrap_or_else(|_| json!({}));
                        content.push(json!({
                            "type": "tool_use",
                            "id": call.id,
                            "name": call.name,
                            "input": input,
                        }));
                    }
                    messages.push(json!({"role": "assistant", "content": content}));
                    continue;
                }
            }

            let role = match message.role.as_str() {
                "assistant" => "assistant",
                _ => "user",
            };
            let content = match &message.content {
                ChatContent::Text(text) => json!([{"type": "text", "text": text}]),
                ChatContent::Parts(parts) => {
                    let mut items = Vec::new();
                    for part in parts {
                        match part {
                            ChatContentPart::Text { text } => {
                                items.push(json!({"type": "text", "text": text}));
                            }
                            ChatContentPart::ImageUrl { image_url } => {
                                if let Some(rest) = image_url.url.strip_prefix("data:") {
                                    let (meta, data) = rest
                                        .split_once(',')
                                        .ok_or_else(|| anyhow!("invalid data url"))?;
                                    let media_type =
                                        meta.split(';').next().unwrap_or("image/png").to_string();
                                    items.push(json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": media_type,
                                            "data": data,
                                        }
                                    }));
                                } else {
                                    return Err(anyhow!(
                                        "anthropic vision requires data URL images, got remote URL"
                                    ));
                                }
                            }
                        }
                    }
                    Value::Array(items)
                }
            };
            messages.push(json!({"role": role, "content": content}));
        }
        Ok((system, messages))
    }
}

#[async_trait]
impl crate::ModelProvider for AnthropicProvider {
    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let req = self.auth(self.client.get(self.models_url()));
        let response = req.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        let data = body
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(enrich_model_list(data.into_iter().filter_map(|item| {
            let id = item.get("id")?.as_str()?.to_string();
            let display = item
                .get("display_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Some((id, display))
        })))
    }

    async fn validate_config(&self) -> Result<()> {
        if self
            .config
            .api_key
            .as_ref()
            .map(|s| s.is_empty())
            .unwrap_or(true)
        {
            return Err(anyhow!("anthropic api key required"));
        }
        Ok(())
    }

    async fn generate_stream(
        &self,
        candidate_id: &str,
        slot_label: &str,
        request: TextGenerationRequest,
        cancel: CancellationToken,
    ) -> Result<EventStream> {
        let (system, messages) = Self::convert_messages(&request)?;
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": 4096,
            "stream": true,
        });
        if let Some(system) = system {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(request
                .tools
                .iter()
                .map(|t| json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                }))
                .collect::<Vec<_>>());
        }

        let req = self.auth(self.client.post(self.messages_url()).json(&body));
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
            // index -> (id, name, args_json)
            let mut tool_acc: HashMap<u32, (String, String, String)> = HashMap::new();
            let mut current_tool_index: Option<u32> = None;

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
                    let Ok(parsed) = serde_json::from_str::<Value>(data) else {
                        continue;
                    };
                    let event_type = parsed
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    match event_type {
                        "content_block_start" => {
                            let idx = parsed
                                .get("index")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32)
                                .unwrap_or(0);
                            let block = parsed.get("content_block").cloned().unwrap_or(Value::Null);
                            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            if block_type == "tool_use" {
                                let id = block
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = block
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                tool_acc.insert(idx, (id, name, String::new()));
                                current_tool_index = Some(idx);
                            } else {
                                current_tool_index = None;
                            }
                        }
                        "content_block_delta" => {
                            let delta_type = parsed
                                .pointer("/delta/type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if delta_type == "thinking_delta" {
                                if let Some(delta) = parsed
                                    .pointer("/delta/thinking")
                                    .and_then(|v| v.as_str())
                                {
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
                            } else if let Some(delta) =
                                parsed.pointer("/delta/text").and_then(|v| v.as_str())
                            {
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
                            if let Some(partial) = parsed
                                .pointer("/delta/partial_json")
                                .and_then(|v| v.as_str())
                            {
                                let idx = parsed
                                    .get("index")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32)
                                    .or(current_tool_index)
                                    .unwrap_or(0);
                                if let Some(entry) = tool_acc.get_mut(&idx) {
                                    entry.2.push_str(partial);
                                }
                            }
                        }
                        "message_delta" => {
                            if let Some(usage) = parsed.get("usage") {
                                completion_tokens = usage
                                    .get("output_tokens")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32)
                                    .or(completion_tokens);
                            }
                        }
                        "message_start" => {
                            if let Some(usage) = parsed.pointer("/message/usage") {
                                prompt_tokens = usage
                                    .get("input_tokens")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32)
                                    .or(prompt_tokens);
                            }
                        }
                        "error" => {
                            let message = parsed
                                .pointer("/error/message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("anthropic error")
                                .to_string();
                            yield GenerationEvent::Error {
                                candidate_id: candidate_id.clone(),
                                message,
                            };
                            return;
                        }
                        "message_stop" | "content_block_stop" => {}
                        _ => {}
                    }
                }
            }

            if !tool_acc.is_empty() {
                let mut calls = Vec::new();
                let mut keys: Vec<_> = tool_acc.keys().copied().collect();
                keys.sort();
                for k in keys {
                    if let Some((id, name, args)) = tool_acc.get(&k) {
                        if !name.is_empty() {
                            calls.push(ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                arguments: if args.is_empty() {
                                    "{}".into()
                                } else {
                                    args.clone()
                                },
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

            if let (Some(p), Some(c)) = (prompt_tokens, completion_tokens) {
                total_tokens = Some(p + c);
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
                    reasoning_tokens: None,
                    reasoning_duration_ms: None,
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
        _candidate_id: &str,
        _slot_label: &str,
        _request: ImageGenerationRequest,
        _cancel: CancellationToken,
    ) -> Result<EventStream> {
        Err(anyhow!(
            "anthropic provider does not support image generation"
        ))
    }
}
