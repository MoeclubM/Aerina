use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use aerina_domain::*;
use aerina_generation::GenerationEngine;
use aerina_media::MediaStore;
use aerina_secrets::SecretStore;
use aerina_storage::Db;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod backup;
mod export;
mod generate;
mod mcp;
mod retry;
mod tree_ops;

pub use backup::*;
pub use export::*;

#[derive(Clone)]
pub struct AppState {
    pub(crate) inner: Arc<AppInner>,
}

pub(crate) struct AppInner {
    pub db: Db,
    pub secrets: SecretStore,
    pub media: MediaStore,
    pub generation: GenerationEngine,
    pub workspace_id: RwLock<WorkspaceId>,
    pub active_jobs: Mutex<HashMap<String, CancellationToken>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub profile: Profile,
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub mode: ConversationMode,
    pub model_preset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertProviderRequest {
    pub id: Option<String>,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertModelPresetRequest {
    pub id: Option<String>,
    pub provider_id: String,
    pub name: String,
    pub model_name: String,
    pub capabilities: Vec<CapabilityTag>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub in_random_pool: bool,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertMcpServerRequest {
    pub id: Option<String>,
    pub name: String,
    pub transport: McpTransport,
    pub url: String,
    pub headers: Option<Vec<(String, String)>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: String,
    pub content: String,
    pub image_data_urls: Option<Vec<String>>,
    pub require_image: Option<bool>,
    pub image_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInfo {
    pub id: String,
    pub round_id: String,
    pub slot_label: String,
    pub model_name: String,
    pub model_preset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundInfo {
    pub id: String,
    pub selected_candidate_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDetail {
    pub conversation: Conversation,
    pub settings: ConversationSettings,
    pub messages: Vec<MessageView>,
    pub branches: Vec<Branch>,
    pub rounds: Vec<RoundInfo>,
    pub candidates: Vec<CandidateInfo>,
    /// True when older messages exist beyond the current window.
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageView {
    pub message: MessageNode,
    pub blocks: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    pub request_count: u64,
    pub completed_count: u64,
    pub failed_count: u64,
    pub cancelled_count: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: Option<f64>,
    pub image_count: u64,
    pub arena_votes: u64,
    pub by_event_type: HashMap<String, u64>,
}

/// One day of local activity for GitHub-style heatmaps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDay {
    /// Local calendar date `YYYY-MM-DD`
    pub date: String,
    pub count: u64,
}

impl AppState {
    pub async fn bootstrap(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        Self::apply_pending_db_restore(&data_dir)?;
        let db_path = data_dir.join("aerina.db");
        let db = Db::connect(db_path).await?;
        let (_profile, workspace) = db.ensure_bootstrap().await?;
        let secrets = SecretStore::open(data_dir.join("secrets.json"))?;
        let media = MediaStore::new(data_dir.clone());
        Ok(Self {
            inner: Arc::new(AppInner {
                db,
                secrets,
                media,
                generation: GenerationEngine::new(),
                workspace_id: RwLock::new(workspace.id),
                active_jobs: Mutex::new(HashMap::new()),
            }),
        })
    }

    pub async fn workspace_id(&self) -> WorkspaceId {
        *self.inner.workspace_id.read().await
    }

    pub(crate) async fn current_workspace(&self) -> WorkspaceId {
        *self.inner.workspace_id.read().await
    }

    pub fn media_root(&self) -> &std::path::Path {
        self.inner.media.root()
    }

    pub async fn session_info(&self) -> Result<SessionInfo> {
        let workspace = self
            .inner
            .db
            .get_workspace(self.current_workspace().await)
            .await?
            .ok_or_else(|| anyhow!("active profile scope missing"))?;
        let profile = self
            .inner
            .db
            .get_profile(workspace.profile_id)
            .await?
            .ok_or_else(|| anyhow!("active profile missing"))?;
        Ok(SessionInfo {
            profile,
            profiles: self.inner.db.list_profiles().await?,
        })
    }

    pub async fn create_profile(&self, request: CreateProfileRequest) -> Result<SessionInfo> {
        let name = request.display_name.trim();
        if name.is_empty() {
            return Err(anyhow!("profile name is empty"));
        }
        let (_profile, workspace) = self
            .inner
            .db
            .create_profile_with_workspace(name, "Default")
            .await?;
        self.inner.db.set_active_workspace_id(workspace.id).await?;
        *self.inner.workspace_id.write().await = workspace.id;
        self.session_info().await
    }

    pub async fn rename_profile(&self, profile_id: &str, display_name: &str) -> Result<Profile> {
        let id = parse_entity_id(profile_id)?;
        self.inner.db.rename_profile(id, display_name).await
    }

    pub async fn set_profile_avatar(&self, profile_id: &str, data_url: &str) -> Result<Profile> {
        let id = parse_entity_id(profile_id)?;
        let profile = self
            .inner
            .db
            .get_profile(id)
            .await?
            .ok_or_else(|| anyhow!("profile not found"))?;
        let (mime, bytes) = MediaStore::decode_data_url(data_url)?;
        if !mime.starts_with("image/") {
            return Err(anyhow!("avatar must be an image"));
        }
        let ext = match mime.as_str() {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/webp" => "webp",
            "image/gif" => "gif",
            other => return Err(anyhow!("unsupported avatar mime: {other}")),
        };
        if let Some(old) = profile.avatar_path.as_deref() {
            self.inner.media.remove_if_exists(old).await?;
        }
        let (relative, _) = self
            .inner
            .media
            .save_profile_avatar(&id.to_string(), &bytes, ext)
            .await?;
        self.inner
            .db
            .update_profile_avatar(id, Some(&relative))
            .await
    }

    pub async fn clear_profile_avatar(&self, profile_id: &str) -> Result<Profile> {
        let id = parse_entity_id(profile_id)?;
        let profile = self
            .inner
            .db
            .get_profile(id)
            .await?
            .ok_or_else(|| anyhow!("profile not found"))?;
        if let Some(old) = profile.avatar_path.as_deref() {
            self.inner.media.remove_if_exists(old).await?;
        }
        self.inner.db.update_profile_avatar(id, None).await
    }

    pub async fn get_profile_avatar_data_url(&self, profile_id: &str) -> Result<Option<String>> {
        let id = parse_entity_id(profile_id)?;
        let profile = self
            .inner
            .db
            .get_profile(id)
            .await?
            .ok_or_else(|| anyhow!("profile not found"))?;
        let Some(path) = profile.avatar_path.as_deref() else {
            return Ok(None);
        };
        let mime = if path.ends_with(".png") {
            "image/png"
        } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg"
        } else if path.ends_with(".webp") {
            "image/webp"
        } else if path.ends_with(".gif") {
            "image/gif"
        } else {
            "application/octet-stream"
        };
        Ok(Some(self.inner.media.data_url(path, mime).await?))
    }

    pub async fn switch_profile(&self, profile_id: &str) -> Result<SessionInfo> {
        let id = parse_entity_id(profile_id)?;
        let workspace = self
            .inner
            .db
            .list_workspaces(id)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("profile data scope missing"))?;
        self.inner.db.set_active_workspace_id(workspace.id).await?;
        *self.inner.workspace_id.write().await = workspace.id;
        self.session_info().await
    }

    pub async fn rename_conversation(
        &self,
        conversation_id: &str,
        title: &str,
    ) -> Result<Conversation> {
        let id = parse_entity_id(conversation_id)?;
        let title = title.trim();
        if title.is_empty() {
            return Err(anyhow!("title is empty"));
        }
        self.inner.db.update_conversation_title(id, title).await?;
        self.inner
            .db
            .get_conversation(id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found"))
    }

    pub async fn list_conversations(&self) -> Result<Vec<Conversation>> {
        self.inner
            .db
            .list_conversations(self.current_workspace().await)
            .await
    }

    pub const UI_MESSAGE_WINDOW: u32 = 100;

    pub async fn get_conversation_detail(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationDetail> {
        // Full load — used by export/internal paths that need the complete branch.
        self.get_conversation_page(conversation_id, None, None)
            .await
    }

    /// Recent-window view for UI / mutation responses.
    pub async fn get_conversation_ui(&self, conversation_id: &str) -> Result<ConversationDetail> {
        self.get_conversation_page(conversation_id, Some(Self::UI_MESSAGE_WINDOW), None)
            .await
    }

    /// Windowed conversation view for UI. `limit = None` loads the full branch.
    pub async fn get_conversation_page(
        &self,
        conversation_id: &str,
        limit: Option<u32>,
        before_message_id: Option<&str>,
    ) -> Result<ConversationDetail> {
        let id = parse_entity_id(conversation_id)?;
        let conversation = self
            .inner
            .db
            .get_conversation(id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found"))?;
        let settings = self
            .inner
            .db
            .get_settings(id)
            .await?
            .ok_or_else(|| anyhow!("conversation settings not found"))?;
        let branch_id = conversation
            .active_branch_id
            .ok_or_else(|| anyhow!("conversation has no active branch"))?;

        let (raw, has_more) = match (limit, before_message_id) {
            (None, _) => {
                let all = self.inner.db.list_messages(branch_id).await?;
                (all, false)
            }
            (Some(limit), None) => self.inner.db.list_messages_tail(branch_id, limit).await?,
            (Some(limit), Some(before)) => {
                let before_id = parse_entity_id(before)?;
                self.inner
                    .db
                    .list_messages_before(branch_id, before_id, limit)
                    .await?
            }
        };

        let messages = raw
            .into_iter()
            .map(|(message, blocks)| MessageView { message, blocks })
            .collect();
        let branches = self.inner.db.list_branches(id).await?;
        let rounds = self
            .inner
            .db
            .list_rounds_for_conversation(id)
            .await?
            .into_iter()
            .map(|r| RoundInfo {
                id: r.id.to_string(),
                selected_candidate_id: r.selected_candidate_id.map(|v| v.to_string()),
            })
            .collect();
        let candidates = self
            .inner
            .db
            .list_candidates_for_conversation(id)
            .await?
            .into_iter()
            .map(|c| CandidateInfo {
                id: c.id.to_string(),
                round_id: c.round_id.to_string(),
                slot_label: c.slot_label,
                model_name: c.model_name,
                model_preset_id: c.model_preset_id.to_string(),
            })
            .collect();
        Ok(ConversationDetail {
            conversation,
            settings,
            messages,
            branches,
            rounds,
            candidates,
            has_more,
        })
    }

    pub async fn create_conversation(
        &self,
        request: CreateConversationRequest,
    ) -> Result<Conversation> {
        let title = request
            .title
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "New conversation".into());
        let (conversation, branch) =
            tree::create_conversation(self.current_workspace().await, title, request.mode);

        let model_preset_ids = if let Some(preset_id) = request.model_preset_id {
            vec![parse_entity_id(&preset_id)?]
        } else {
            self.inner
                .db
                .list_model_presets(self.current_workspace().await)
                .await?
                .into_iter()
                .filter(|preset| preset.enabled)
                .map(|preset| preset.id)
                .take(1)
                .collect()
        };

        let settings =
            aerina_conversation::default_settings(conversation.id, request.mode, model_preset_ids);
        self.inner
            .db
            .insert_conversation(&conversation, &branch, &settings)
            .await?;
        Ok(conversation)
    }

    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        let id = parse_entity_id(conversation_id)?;
        self.inner.db.delete_conversation(id).await
    }

    pub async fn list_providers(&self) -> Result<Vec<Provider>> {
        self.inner
            .db
            .list_providers(self.current_workspace().await)
            .await
    }

    pub async fn upsert_provider(&self, request: UpsertProviderRequest) -> Result<Provider> {
        if let Some(id) = request.id.as_deref() {
            let provider_id = parse_entity_id(id)?;
            let mut provider = self
                .inner
                .db
                .get_provider(provider_id)
                .await?
                .ok_or_else(|| anyhow!("provider not found"))?;
            provider.name = request.name;
            provider.kind = request.kind;
            provider.base_url = request.base_url;
            if let Some(enabled) = request.enabled {
                provider.enabled = enabled;
            }
            if let Some(api_key) = request.api_key.as_ref() {
                let key = format!("provider:{}", provider.id);
                self.inner.secrets.put(&key, api_key)?;
                provider.secret_ref = Some(key);
            }
            self.inner.db.update_provider(&provider).await?;
            return Ok(provider);
        }

        let provider_id = ProviderId::new();
        let secret_ref = if let Some(api_key) = request.api_key.as_ref() {
            let key = format!("provider:{}", provider_id);
            self.inner.secrets.put(&key, api_key)?;
            Some(key)
        } else {
            None
        };

        let provider = Provider {
            id: provider_id,
            workspace_id: self.current_workspace().await,
            name: request.name,
            kind: request.kind,
            base_url: request.base_url,
            secret_ref,
            enabled: request.enabled.unwrap_or(true),
            created_at: Utc::now(),
        };
        self.inner.db.insert_provider(&provider).await?;
        Ok(provider)
    }

    pub async fn delete_provider(&self, provider_id: &str) -> Result<()> {
        let id = parse_entity_id(provider_id)?;
        self.inner.db.delete_provider(id).await
    }

    pub async fn list_model_presets(&self) -> Result<Vec<ModelPreset>> {
        self.inner
            .db
            .list_model_presets(self.current_workspace().await)
            .await
    }

    pub async fn upsert_model_preset(
        &self,
        request: UpsertModelPresetRequest,
    ) -> Result<ModelPreset> {
        let provider_id = parse_entity_id(&request.provider_id)?;
        let provider = self
            .inner
            .db
            .get_provider(provider_id)
            .await?
            .ok_or_else(|| anyhow!("provider not found"))?;
        if provider.workspace_id != self.current_workspace().await {
            return Err(anyhow!("provider workspace mismatch"));
        }

        if let Some(id) = request.id.as_deref() {
            let preset_id = parse_entity_id(id)?;
            let mut preset = self
                .inner
                .db
                .get_model_preset(preset_id)
                .await?
                .ok_or_else(|| anyhow!("model preset not found"))?;
            preset.provider_id = provider_id;
            preset.name = request.name;
            preset.model_name = request.model_name;
            preset.capabilities = request.capabilities;
            preset.temperature = request.temperature;
            preset.system_prompt = request.system_prompt;
            preset.in_random_pool = request.in_random_pool;
            if let Some(enabled) = request.enabled {
                preset.enabled = enabled;
            }
            self.inner.db.update_model_preset(&preset).await?;
            return Ok(preset);
        }

        let preset = ModelPreset {
            id: ModelPresetId::new(),
            workspace_id: self.current_workspace().await,
            provider_id,
            model_id: None,
            name: request.name,
            model_name: request.model_name,
            capabilities: request.capabilities,
            temperature: request.temperature,
            system_prompt: request.system_prompt,
            in_random_pool: request.in_random_pool,
            enabled: request.enabled.unwrap_or(true),
            created_at: Utc::now(),
        };
        self.inner.db.insert_model_preset(&preset).await?;
        Ok(preset)
    }

    pub async fn delete_model_preset(&self, preset_id: &str) -> Result<()> {
        let id = parse_entity_id(preset_id)?;
        self.inner.db.delete_model_preset(id).await
    }

    pub fn infer_model_compat(&self, model_name: &str) -> ModelInfo {
        let profile = aerina_domain::infer_model_compat(model_name);
        ModelInfo {
            model_name: model_name.trim().to_string(),
            display_name: profile.display_name,
            capabilities: profile.capabilities,
            context_length: profile.context_length,
            family: profile.family.map(|s| s.to_string()),
        }
    }

    pub async fn list_provider_models(&self, provider_id: &str) -> Result<Vec<ModelInfo>> {
        let id = parse_entity_id(provider_id)?;
        let provider = self
            .inner
            .db
            .get_provider(id)
            .await?
            .ok_or_else(|| anyhow!("provider not found"))?;
        if provider.workspace_id != self.current_workspace().await {
            return Err(anyhow!("provider workspace mismatch"));
        }
        let api_key = match provider.secret_ref.as_ref() {
            Some(secret_ref) => self.inner.secrets.get(secret_ref)?,
            None => None,
        };
        let provider_client = aerina_providers::build_provider(ProviderConfig {
            id: provider.id,
            name: provider.name,
            kind: provider.kind,
            base_url: provider.base_url,
            api_key,
        })?;
        provider_client.list_models().await
    }

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServer>> {
        self.inner
            .db
            .list_mcp_servers(self.current_workspace().await)
            .await
    }

    pub async fn upsert_mcp_server(&self, request: UpsertMcpServerRequest) -> Result<McpServer> {
        if let Some(id) = request.id.as_deref() {
            let server_id = parse_entity_id(id)?;
            let servers = self.list_mcp_servers().await?;
            let mut server = servers
                .into_iter()
                .find(|s| s.id == server_id)
                .ok_or_else(|| anyhow!("mcp server not found"))?;
            server.name = request.name;
            server.transport = request.transport;
            server.url = request.url;
            if let Some(headers) = request.headers {
                server.headers = headers;
            }
            if let Some(enabled) = request.enabled {
                server.enabled = enabled;
            }
            self.inner.db.update_mcp_server(&server).await?;
            return Ok(server);
        }

        let server = McpServer {
            id: McpServerId::new(),
            workspace_id: self.current_workspace().await,
            name: request.name,
            transport: request.transport,
            url: request.url,
            headers: request.headers.unwrap_or_default(),
            enabled: request.enabled.unwrap_or(true),
            created_at: Utc::now(),
        };
        self.inner.db.insert_mcp_server(&server).await?;
        Ok(server)
    }

    pub async fn delete_mcp_server(&self, server_id: &str) -> Result<()> {
        let id = parse_entity_id(server_id)?;
        self.inner.db.delete_mcp_server(id).await
    }

    pub async fn update_conversation_model(
        &self,
        conversation_id: &str,
        model_preset_id: &str,
    ) -> Result<ConversationSettings> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let model_preset_id = parse_entity_id(model_preset_id)?;
        let mut settings = self
            .inner
            .db
            .get_settings(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("settings not found"))?;
        settings.model_preset_ids = vec![model_preset_id];
        if settings.candidate_pool.is_empty() {
            settings.candidate_pool = vec![model_preset_id];
        }
        self.inner.db.update_settings(&settings).await?;
        Ok(settings)
    }

    pub async fn update_system_prompt(
        &self,
        conversation_id: &str,
        system_prompt: Option<String>,
        temperature: Option<f32>,
    ) -> Result<ConversationSettings> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let mut settings = self
            .inner
            .db
            .get_settings(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("settings not found"))?;
        settings.system_prompt = system_prompt;
        settings.temperature = temperature;
        self.inner.db.update_settings(&settings).await?;
        Ok(settings)
    }

    pub async fn cancel_generation(&self, conversation_id: &str) -> Result<()> {
        let mut jobs = self.inner.active_jobs.lock().await;
        if let Some(token) = jobs.remove(conversation_id) {
            token.cancel();
        }
        Ok(())
    }

    pub async fn list_analytics_events(&self) -> Result<Vec<AnalyticsEvent>> {
        self.inner
            .db
            .list_analytics_events(self.current_workspace().await)
            .await
    }

    pub async fn leaderboard(&self) -> Result<Vec<EloEntry>> {
        let events = self
            .inner
            .db
            .list_ranking_events(self.current_workspace().await)
            .await?;
        Ok(aerina_ranking::rebuild_elo(&events))
    }

    pub async fn activity_heatmap(&self, days: u32) -> Result<Vec<ActivityDay>> {
        let days = days.clamp(1, 366) as i64;
        let events = self.list_analytics_events().await?;
        let today = Utc::now().date_naive();
        let start = today - chrono::Duration::days(days - 1);

        let mut counts: HashMap<String, u64> = HashMap::new();
        for event in &events {
            // Count generations (starts) as activity cells.
            if !matches!(event.event_type, AnalyticsEventType::GenerationStarted) {
                continue;
            }
            let day = event.created_at.date_naive();
            if day < start || day > today {
                continue;
            }
            let key = day.format("%Y-%m-%d").to_string();
            *counts.entry(key).or_insert(0) += 1;
        }

        let mut out = Vec::with_capacity(days as usize);
        let mut d = start;
        while d <= today {
            let key = d.format("%Y-%m-%d").to_string();
            out.push(ActivityDay {
                count: *counts.get(&key).unwrap_or(&0),
                date: key,
            });
            d += chrono::Duration::days(1);
        }
        Ok(out)
    }

    pub async fn stats_summary(&self) -> Result<StatsSummary> {
        let events = self.list_analytics_events().await?;
        let usage = self
            .inner
            .db
            .list_usage_for_workspace(self.current_workspace().await)
            .await?;

        let mut by_event_type = HashMap::new();
        let mut completed_count = 0;
        let mut failed_count = 0;
        let mut cancelled_count = 0;
        let mut image_count = 0;
        let mut arena_votes = 0;
        for event in &events {
            let key = format!("{:?}", event.event_type);
            *by_event_type.entry(key).or_insert(0) += 1;
            match event.event_type {
                AnalyticsEventType::GenerationCompleted => completed_count += 1,
                AnalyticsEventType::GenerationFailed => failed_count += 1,
                AnalyticsEventType::GenerationCancelled => cancelled_count += 1,
                AnalyticsEventType::ImageSaved => image_count += 1,
                AnalyticsEventType::CandidateCommitted => arena_votes += 1,
                AnalyticsEventType::ArenaVoteCast => arena_votes += 1,
                _ => {}
            }
        }
        // Prefer ranking rounds when available (true comparison picks).
        let ranking_events = self
            .inner
            .db
            .list_ranking_events(self.current_workspace().await)
            .await?;
        if !ranking_events.is_empty() {
            let mut rounds = std::collections::HashSet::new();
            for e in &ranking_events {
                rounds.insert(e.round_id);
            }
            arena_votes = rounds.len() as u64;
        }

        let total_tokens = usage
            .iter()
            .map(|u| u.total_tokens.unwrap_or(0) as u64)
            .sum();
        let total_cost_usd = usage.iter().filter_map(|u| u.cost_usd).sum();
        let latency_values = usage
            .iter()
            .filter_map(|u| u.latency_ms.map(|v| v as f64))
            .collect::<Vec<_>>();
        let avg_latency_ms = if latency_values.is_empty() {
            None
        } else {
            Some(latency_values.iter().sum::<f64>() / latency_values.len() as f64)
        };

        Ok(StatsSummary {
            request_count: events
                .iter()
                .filter(|e| matches!(e.event_type, AnalyticsEventType::GenerationStarted))
                .count() as u64,
            completed_count,
            failed_count,
            cancelled_count,
            total_tokens,
            total_cost_usd,
            avg_latency_ms,
            image_count,
            arena_votes,
            by_event_type,
        })
    }

    pub async fn set_conversation_models(
        &self,
        conversation_id: &str,
        model_preset_ids: Vec<String>,
        mode: ConversationMode,
    ) -> Result<ConversationSettings> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let ids = model_preset_ids
            .iter()
            .map(|id| parse_entity_id(id))
            .collect::<Result<Vec<_>>>()?;
        if ids.is_empty() {
            return Err(anyhow!("at least one model preset is required"));
        }
        let mut settings = self
            .inner
            .db
            .get_settings(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("settings not found"))?;
        settings.mode = mode;
        settings.model_preset_ids = ids.clone();
        settings.candidate_pool = ids;
        settings.slot_count = settings.model_preset_ids.len() as u32;
        self.inner.db.update_settings(&settings).await?;
        self.inner
            .db
            .update_conversation_mode(conversation_id, mode)
            .await?;
        Ok(settings)
    }

    pub async fn configure_arena(
        &self,
        conversation_id: &str,
        pool: Vec<String>,
        slot_count: u32,
        arena_kind: ArenaKind,
        category: Option<String>,
    ) -> Result<ConversationSettings> {
        let conversation_id = parse_entity_id(conversation_id)?;
        let ids = pool
            .iter()
            .map(|id| parse_entity_id(id))
            .collect::<Result<Vec<_>>>()?;
        if ids.len() < slot_count as usize {
            return Err(anyhow!("candidate pool smaller than slot count"));
        }
        let mut settings = self
            .inner
            .db
            .get_settings(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("settings not found"))?;
        settings.mode = ConversationMode::Arena;
        settings.model_preset_ids = ids.clone();
        settings.candidate_pool = ids;
        settings.slot_count = slot_count.max(2);
        settings.arena_kind = Some(arena_kind);
        settings.arena_category = category;
        self.inner.db.update_settings(&settings).await?;
        self.inner
            .db
            .update_conversation_mode(conversation_id, ConversationMode::Arena)
            .await?;
        Ok(settings)
    }

    pub async fn list_round_candidates(&self, round_id: &str) -> Result<Vec<serde_json::Value>> {
        let round_id = parse_entity_id(round_id)?;
        let candidates = self.inner.db.list_candidates(round_id).await?;
        let events = self
            .inner
            .db
            .list_analytics_events(self.current_workspace().await)
            .await?;
        let revealed = events.iter().any(|e| {
            e.round_id == Some(round_id)
                && matches!(e.event_type, AnalyticsEventType::ArenaRevealed)
        });

        Ok(candidates
            .into_iter()
            .map(|c| {
                if revealed || !c.anonymous {
                    serde_json::json!({
                        "id": c.id.to_string(),
                        "slot_label": c.slot_label,
                        "model_name": c.model_name,
                        "model_preset_id": c.model_preset_id.to_string(),
                        "status": c.status,
                        "anonymous": c.anonymous,
                        "revealed": true,
                    })
                } else {
                    serde_json::json!({
                        "id": c.id.to_string(),
                        "slot_label": c.slot_label,
                        "status": c.status,
                        "anonymous": true,
                        "revealed": false,
                    })
                }
            })
            .collect())
    }

    pub(crate) async fn resolve_preset(
        &self,
        preset_id: ModelPresetId,
    ) -> Result<ResolvedModelPreset> {
        let preset = self
            .inner
            .db
            .get_model_preset(preset_id)
            .await?
            .ok_or_else(|| anyhow!("model preset not found"))?;
        let provider = self
            .inner
            .db
            .get_provider(preset.provider_id)
            .await?
            .ok_or_else(|| anyhow!("provider not found"))?;
        let api_key = match provider.secret_ref.as_ref() {
            Some(secret_ref) => self.inner.secrets.get(secret_ref)?,
            None => None,
        };
        Ok(ResolvedModelPreset {
            preset_id: preset.id,
            provider_id: provider.id,
            provider_kind: provider.kind,
            base_url: provider.base_url,
            api_key,
            model_name: preset.model_name,
            display_name: preset.name,
            capabilities: preset.capabilities,
            temperature: preset.temperature,
            system_prompt: preset.system_prompt,
        })
    }
}

pub(crate) fn parse_entity_id(value: &str) -> Result<EntityId> {
    Ok(EntityId::from_uuid(Uuid::parse_str(value)?))
}
