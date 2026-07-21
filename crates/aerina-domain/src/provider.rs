use serde::{Deserialize, Serialize};

use crate::ids::{ModelPresetId, ProviderId};
use crate::{CapabilityTag, ProviderKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: ProviderId,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl ChatMessage {
    pub fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: ChatContent::Text(content.into()),
            tool_call_id: None,
            tool_calls: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub tool_choice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationRequest {
    pub model: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub size: Option<String>,
    pub aspect_ratio: Option<String>,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub b64_json: Option<String>,
    pub url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub mime: String,
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<u64>,
    pub ttft_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenerationEvent {
    StreamStart {
        candidate_id: String,
        slot_label: String,
    },
    TextDelta {
        candidate_id: String,
        delta: String,
    },
    ThinkingDelta {
        candidate_id: String,
        delta: String,
    },
    ImageReady {
        candidate_id: String,
        image: GeneratedImage,
    },
    Usage {
        candidate_id: String,
        usage: UsageReport,
    },
    Done {
        candidate_id: String,
    },
    Error {
        candidate_id: String,
        message: String,
    },
    ToolCalls {
        candidate_id: String,
        calls: Vec<ToolCall>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_name: String,
    pub display_name: String,
    pub capabilities: Vec<CapabilityTag>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedModelPreset {
    pub preset_id: ModelPresetId,
    pub provider_id: ProviderId,
    pub provider_kind: ProviderKind,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model_name: String,
    pub display_name: String,
    pub capabilities: Vec<CapabilityTag>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

pub fn filter_by_capabilities(
    presets: &[ResolvedModelPreset],
    required: &[CapabilityTag],
) -> Vec<ResolvedModelPreset> {
    presets
        .iter()
        .filter(|preset| required.iter().all(|cap| preset.capabilities.contains(cap)))
        .cloned()
        .collect()
}
