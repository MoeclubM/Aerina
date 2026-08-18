use std::path::{Path, PathBuf};

use aerina_domain::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, SqlitePool};
use uuid::Uuid;

mod rows;
use rows::*;

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
    path: PathBuf,
}

fn parse_id(value: &str) -> Result<EntityId> {
    Ok(EntityId::from_uuid(Uuid::parse_str(value)?))
}

fn parse_dt(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

fn id_str(id: EntityId) -> String {
    id.to_string()
}

fn dt_str(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn encode_json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn decode_json<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    Ok(serde_json::from_str(value)?)
}

impl Db {
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("failed to open sqlite at {}", path.display()))?;

        sqlx::query("PRAGMA journal_mode = WAL;")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await?;

        let db = Self { pool, path };
        db.migrate().await?;
        Ok(db)
    }

    pub async fn connect_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await?;
        let db = Self {
            pool,
            path: PathBuf::from(":memory:"),
        };
        db.migrate().await?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn migrate(&self) -> Result<()> {
        for sql in [
            include_str!("../../../migrations/001_init.sql"),
            include_str!("../../../migrations/002_mcp_servers.sql"),
            include_str!("../../../migrations/003_app_settings.sql"),
        ] {
            sqlx::raw_sql(sql).execute(&self.pool).await?;
        }
        self.ensure_column("profiles", "avatar_path", "TEXT")
            .await?;
        self.ensure_column("profiles", "auth_subject", "TEXT")
            .await?;
        self.ensure_column("profiles", "auth_provider", "TEXT")
            .await?;
        self.ensure_column("usage_records", "output_tokens", "INTEGER")
            .await?;
        self.ensure_column("usage_records", "reasoning_tokens", "INTEGER")
            .await?;
        self.ensure_column("usage_records", "reasoning_duration_ms", "INTEGER")
            .await?;
        Ok(())
    }

    async fn ensure_column(&self, table: &str, column: &str, decl: &str) -> Result<()> {
        let rows = sqlx::query_as::<_, (i64, String, String, i64, Option<String>, i64)>(&format!(
            "PRAGMA table_info({table})"
        ))
        .fetch_all(&self.pool)
        .await?;
        let exists = rows.iter().any(|(_, name, _, _, _, _)| name == column);
        if !exists {
            let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {decl}");
            sqlx::raw_sql(&sql).execute(&self.pool).await?;
        }
        Ok(())
    }
}

mod analytics;
mod backup_ops;
mod bootstrap;
mod conversations;
mod extra_ops;
mod mcp_ops;
mod messages;
mod providers;
mod ranking_ops;

#[cfg(test)]
mod tests {
    use super::*;
    use aerina_domain::tree;

    #[tokio::test]
    async fn bootstrap_and_conversation_roundtrip() {
        let db = Db::connect_in_memory().await.unwrap();
        let (_profile, workspace) = db.ensure_bootstrap().await.unwrap();
        let (conversation, branch) =
            tree::create_conversation(workspace.id, "hello", ConversationMode::Chat);
        let settings = ConversationSettings {
            conversation_id: conversation.id,
            mode: ConversationMode::Chat,
            system_prompt: None,
            temperature: Some(0.7),
            model_preset_ids: vec![],
            candidate_pool: vec![],
            slot_count: 1,
            arena_kind: None,
            arena_category: None,
            max_concurrency: 1,
            image_size: None,
            image_aspect_ratio: None,
        };
        db.insert_conversation(&conversation, &branch, &settings)
            .await
            .unwrap();
        let listed = db.list_conversations(workspace.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "hello");
    }

    #[tokio::test]
    async fn repairs_missing_model_preset_references() {
        let db = Db::connect_in_memory().await.unwrap();
        let (_profile, workspace) = db.ensure_bootstrap().await.unwrap();
        let provider = Provider {
            id: ProviderId::new(),
            workspace_id: workspace.id,
            name: "provider".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "http://localhost".into(),
            secret_ref: None,
            enabled: true,
            created_at: Utc::now(),
        };
        db.insert_provider(&provider).await.unwrap();
        let preset = ModelPreset {
            id: ModelPresetId::new(),
            workspace_id: workspace.id,
            provider_id: provider.id,
            model_id: None,
            name: "valid".into(),
            model_name: "valid".into(),
            capabilities: vec![CapabilityTag::Text],
            temperature: None,
            system_prompt: None,
            in_random_pool: true,
            enabled: true,
            created_at: Utc::now(),
        };
        db.insert_model_preset(&preset).await.unwrap();
        let missing = ModelPresetId::new();

        let (empty_conversation, empty_branch) =
            tree::create_conversation(workspace.id, "empty", ConversationMode::Chat);
        let empty_settings = ConversationSettings {
            conversation_id: empty_conversation.id,
            mode: ConversationMode::Chat,
            system_prompt: None,
            temperature: None,
            model_preset_ids: vec![missing],
            candidate_pool: vec![missing],
            slot_count: 1,
            arena_kind: None,
            arena_category: None,
            max_concurrency: 1,
            image_size: None,
            image_aspect_ratio: None,
        };
        db.insert_conversation(&empty_conversation, &empty_branch, &empty_settings)
            .await
            .unwrap();

        let (mixed_conversation, mixed_branch) =
            tree::create_conversation(workspace.id, "mixed", ConversationMode::Sbs);
        let mixed_settings = ConversationSettings {
            conversation_id: mixed_conversation.id,
            mode: ConversationMode::Sbs,
            system_prompt: None,
            temperature: None,
            model_preset_ids: vec![missing, preset.id],
            candidate_pool: vec![missing, preset.id],
            slot_count: 2,
            arena_kind: None,
            arena_category: None,
            max_concurrency: 2,
            image_size: None,
            image_aspect_ratio: None,
        };
        db.insert_conversation(&mixed_conversation, &mixed_branch, &mixed_settings)
            .await
            .unwrap();

        assert_eq!(
            db.count_conversations_using_model_presets(workspace.id, &[missing])
                .await
                .unwrap(),
            2
        );
        db.repair_missing_model_preset_references(workspace.id)
            .await
            .unwrap();

        let repaired_empty = db
            .get_settings(empty_conversation.id)
            .await
            .unwrap()
            .unwrap();
        assert!(repaired_empty.model_preset_ids.is_empty());
        assert!(repaired_empty.candidate_pool.is_empty());
        assert_eq!(repaired_empty.slot_count, 0);

        let repaired_mixed = db
            .get_settings(mixed_conversation.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(repaired_mixed.model_preset_ids, vec![preset.id]);
        assert_eq!(repaired_mixed.candidate_pool, vec![preset.id]);
        assert_eq!(repaired_mixed.slot_count, 1);
    }

    #[tokio::test]
    async fn usage_metrics_roundtrip_for_conversation_and_workspace() {
        let db = Db::connect_in_memory().await.unwrap();
        let (_profile, workspace) = db.ensure_bootstrap().await.unwrap();
        let provider = Provider {
            id: ProviderId::new(),
            workspace_id: workspace.id,
            name: "provider".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "http://localhost".into(),
            secret_ref: None,
            enabled: true,
            created_at: Utc::now(),
        };
        db.insert_provider(&provider).await.unwrap();
        let preset = ModelPreset {
            id: ModelPresetId::new(),
            workspace_id: workspace.id,
            provider_id: provider.id,
            model_id: None,
            name: "model".into(),
            model_name: "model".into(),
            capabilities: vec![CapabilityTag::Text],
            temperature: None,
            system_prompt: None,
            in_random_pool: true,
            enabled: true,
            created_at: Utc::now(),
        };
        db.insert_model_preset(&preset).await.unwrap();
        let (conversation, branch) =
            tree::create_conversation(workspace.id, "metrics", ConversationMode::Chat);
        db.insert_conversation(
            &conversation,
            &branch,
            &ConversationSettings {
                conversation_id: conversation.id,
                mode: ConversationMode::Chat,
                system_prompt: None,
                temperature: None,
                model_preset_ids: vec![preset.id],
                candidate_pool: vec![preset.id],
                slot_count: 1,
                arena_kind: None,
                arena_category: None,
                max_concurrency: 1,
                image_size: None,
                image_aspect_ratio: None,
            },
        )
        .await
        .unwrap();
        let user = tree::create_user_message(conversation.id, branch.id, None);
        db.insert_message(&user, &[ContentBlock::text("hello")])
            .await
            .unwrap();
        let round = tree::create_round(conversation.id, branch.id, user.id);
        db.insert_round(
            &round,
            &RoundSnapshot {
                round_id: round.id,
                mode: ConversationMode::Chat,
                system_prompt: None,
                temperature: None,
                model_preset_ids: vec![preset.id],
                arena_kind: None,
                arena_category: None,
                image_size: None,
                image_aspect_ratio: None,
                created_at: Utc::now(),
            },
        )
        .await
        .unwrap();
        let candidate =
            tree::create_candidate(round.id, "A", preset.id, provider.id, "model", false);
        db.insert_candidate(&candidate).await.unwrap();
        db.insert_usage(&UsageRecord {
            candidate_id: candidate.id,
            prompt_tokens: Some(11),
            completion_tokens: Some(23),
            output_tokens: Some(17),
            total_tokens: Some(34),
            cost_usd: Some(0.02),
            latency_ms: Some(3429),
            ttft_ms: Some(3421),
            reasoning_tokens: Some(6),
            reasoning_duration_ms: Some(3421),
        })
        .await
        .unwrap();

        for usage in [
            db.list_usage_for_conversation(conversation.id)
                .await
                .unwrap(),
            db.list_usage_for_workspace(workspace.id).await.unwrap(),
        ] {
            assert_eq!(usage.len(), 1);
            assert_eq!(usage[0].output_tokens, Some(17));
            assert_eq!(usage[0].reasoning_tokens, Some(6));
            assert_eq!(usage[0].reasoning_duration_ms, Some(3421));
            assert_eq!(usage[0].ttft_ms, Some(3421));
        }
    }

    #[tokio::test]
    async fn truncate_branch_from_drops_cut_turn_and_keeps_prior_context() {
        let db = Db::connect_in_memory().await.unwrap();
        let (_profile, workspace) = db.ensure_bootstrap().await.unwrap();
        let (conversation, mut branch) =
            tree::create_conversation(workspace.id, "thread", ConversationMode::Chat);
        let settings = ConversationSettings {
            conversation_id: conversation.id,
            mode: ConversationMode::Chat,
            system_prompt: None,
            temperature: None,
            model_preset_ids: vec![],
            candidate_pool: vec![],
            slot_count: 1,
            arena_kind: None,
            arena_category: None,
            max_concurrency: 1,
            image_size: None,
            image_aspect_ratio: None,
        };
        db.insert_conversation(&conversation, &branch, &settings)
            .await
            .unwrap();

        let user1 = tree::create_user_message(conversation.id, branch.id, None);
        db.insert_message(&user1, &[ContentBlock::text("first")])
            .await
            .unwrap();
        let assistant1 = MessageNode {
            id: MessageNodeId::new(),
            conversation_id: conversation.id,
            branch_id: branch.id,
            parent_message_id: Some(user1.id),
            role: MessageRole::Assistant,
            round_id: None,
            candidate_id: None,
            created_at: Utc::now(),
        };
        db.insert_message(&assistant1, &[ContentBlock::text("first reply")])
            .await
            .unwrap();
        let user2 = tree::create_user_message(conversation.id, branch.id, Some(assistant1.id));
        db.insert_message(&user2, &[ContentBlock::text("second")])
            .await
            .unwrap();
        let round = tree::create_round(conversation.id, branch.id, user2.id);
        db.insert_round(
            &round,
            &RoundSnapshot {
                round_id: round.id,
                mode: ConversationMode::Chat,
                system_prompt: None,
                temperature: None,
                model_preset_ids: vec![],
                arena_kind: None,
                arena_category: None,
                image_size: None,
                image_aspect_ratio: None,
                created_at: Utc::now(),
            },
        )
        .await
        .unwrap();
        let assistant2 = MessageNode {
            id: MessageNodeId::new(),
            conversation_id: conversation.id,
            branch_id: branch.id,
            parent_message_id: Some(user2.id),
            role: MessageRole::Assistant,
            round_id: Some(round.id),
            candidate_id: None,
            created_at: Utc::now(),
        };
        db.insert_message(&assistant2, &[ContentBlock::text("second reply")])
            .await
            .unwrap();
        branch.head_message_id = Some(assistant1.id);
        db.upsert_branch(&branch).await.unwrap();

        db.truncate_branch_from(branch.id, user2.id).await.unwrap();

        let remaining = db.list_messages(branch.id).await.unwrap();
        let ids = remaining
            .iter()
            .map(|(message, _)| message.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![user1.id, assistant1.id]);
        assert!(db.get_round(round.id).await.unwrap().is_none());
    }
}
