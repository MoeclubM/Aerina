use aerina_domain::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::RequestBuilder;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

mod anthropic;
mod openai_chat;
mod openai_responses;

pub use anthropic::AnthropicProvider;
pub use openai_chat::OpenAiChatProvider;
pub use openai_responses::OpenAiResponsesProvider;

pub type EventStream = Pin<Box<dyn Stream<Item = GenerationEvent> + Send>>;

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn validate_config(&self) -> Result<()>;
    async fn generate_stream(
        &self,
        candidate_id: &str,
        slot_label: &str,
        request: TextGenerationRequest,
        cancel: CancellationToken,
    ) -> Result<EventStream>;
    async fn generate_image(
        &self,
        candidate_id: &str,
        slot_label: &str,
        request: ImageGenerationRequest,
        cancel: CancellationToken,
    ) -> Result<EventStream>;
}

pub(crate) fn auth_bearer(req: RequestBuilder, api_key: &Option<String>) -> RequestBuilder {
    match api_key {
        Some(key) if !key.is_empty() => req.bearer_auth(key),
        _ => req,
    }
}

pub fn build_provider(config: ProviderConfig) -> Result<Box<dyn ModelProvider>> {
    match config.kind {
        ProviderKind::OpenAiCompatible
        | ProviderKind::OpenAi
        | ProviderKind::Ollama
        | ProviderKind::LmStudio
        | ProviderKind::Custom => Ok(Box::new(OpenAiChatProvider::new(config))),
        ProviderKind::OpenAiResponses => Ok(Box::new(OpenAiResponsesProvider::new(config))),
        ProviderKind::Anthropic => Ok(Box::new(AnthropicProvider::new(config))),
        other => Err(anyhow!("provider kind not implemented: {other:?}")),
    }
}
