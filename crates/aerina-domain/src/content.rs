use serde::{Deserialize, Serialize};

use crate::ids::MediaObjectId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_tokens: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_duration_ms: Option<u64>,
    },
    Image {
        media_id: MediaObjectId,
        alt: Option<String>,
        revised_prompt: Option<String>,
    },
    FileRef {
        media_id: MediaObjectId,
        name: String,
        mime: String,
    },
    Code {
        language: Option<String>,
        code: String,
    },
    UsageMeta {
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
        total_tokens: Option<u32>,
        cost_usd: Option<f64>,
        latency_ms: Option<u64>,
        ttft_ms: Option<u64>,
    },
    ToolCall {
        name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        name: String,
        result: serde_json::Value,
    },
    AgentStep {
        summary: String,
        run_ref: Option<String>,
    },
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}
