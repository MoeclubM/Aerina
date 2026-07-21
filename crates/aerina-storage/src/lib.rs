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
}
