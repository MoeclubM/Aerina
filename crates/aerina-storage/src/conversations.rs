use super::*;
use chrono::Utc;

impl Db {
    pub async fn insert_conversation(
        &self,
        conversation: &Conversation,
        branch: &Branch,
        settings: &ConversationSettings,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO conversations (id, workspace_id, title, mode, active_branch_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(conversation.id))
        .bind(id_str(conversation.workspace_id))
        .bind(&conversation.title)
        .bind(encode_json(&conversation.mode)?)
        .bind(conversation.active_branch_id.map(id_str))
        .bind(dt_str(conversation.created_at))
        .bind(dt_str(conversation.updated_at))
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO branches (id, conversation_id, parent_branch_id, fork_candidate_id, head_message_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(branch.id))
        .bind(id_str(branch.conversation_id))
        .bind(branch.parent_branch_id.map(id_str))
        .bind(branch.fork_candidate_id.map(id_str))
        .bind(branch.head_message_id.map(id_str))
        .bind(dt_str(branch.created_at))
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO conversation_settings (
                conversation_id, mode, system_prompt, temperature, model_preset_ids_json,
                candidate_pool_json, slot_count, arena_kind, arena_category, max_concurrency,
                image_size, image_aspect_ratio
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(settings.conversation_id))
        .bind(encode_json(&settings.mode)?)
        .bind(&settings.system_prompt)
        .bind(settings.temperature.map(|v| v as f64))
        .bind(encode_json(&settings.model_preset_ids)?)
        .bind(encode_json(&settings.candidate_pool)?)
        .bind(settings.slot_count as i64)
        .bind(settings.arena_kind.map(|v| encode_json(&v)).transpose()?)
        .bind(&settings.arena_category)
        .bind(settings.max_concurrency as i64)
        .bind(&settings.image_size)
        .bind(&settings.image_aspect_ratio)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_conversations(&self, workspace_id: WorkspaceId) -> Result<Vec<Conversation>> {
        let rows = sqlx::query_as::<_, ConversationRow>(
            "SELECT id, workspace_id, title, mode, active_branch_id, created_at, updated_at
             FROM conversations WHERE workspace_id = ? ORDER BY updated_at DESC",
        )
        .bind(id_str(workspace_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Conversation {
                    id: parse_id(&row.id)?,
                    workspace_id: parse_id(&row.workspace_id)?,
                    title: row.title,
                    mode: decode_json(&row.mode)?,
                    active_branch_id: row.active_branch_id.as_deref().map(parse_id).transpose()?,
                    created_at: parse_dt(&row.created_at)?,
                    updated_at: parse_dt(&row.updated_at)?,
                })
            })
            .collect()
    }

    pub async fn get_conversation(&self, id: ConversationId) -> Result<Option<Conversation>> {
        let row = sqlx::query_as::<_, ConversationRow>(
            "SELECT id, workspace_id, title, mode, active_branch_id, created_at, updated_at
             FROM conversations WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(Conversation {
                id: parse_id(&row.id)?,
                workspace_id: parse_id(&row.workspace_id)?,
                title: row.title,
                mode: decode_json(&row.mode)?,
                active_branch_id: row.active_branch_id.as_deref().map(parse_id).transpose()?,
                created_at: parse_dt(&row.created_at)?,
                updated_at: parse_dt(&row.updated_at)?,
            }),
            None => None,
        })
    }

    pub async fn get_settings(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Option<ConversationSettings>> {
        let row = sqlx::query_as::<_, SettingsRow>(
            "SELECT conversation_id, mode, system_prompt, temperature, model_preset_ids_json,
                    candidate_pool_json, slot_count, arena_kind, arena_category, max_concurrency,
                    image_size, image_aspect_ratio
             FROM conversation_settings WHERE conversation_id = ?",
        )
        .bind(id_str(conversation_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(ConversationSettings {
                conversation_id: parse_id(&row.conversation_id)?,
                mode: decode_json(&row.mode)?,
                system_prompt: row.system_prompt,
                temperature: row.temperature.map(|v| v as f32),
                model_preset_ids: decode_json(&row.model_preset_ids_json)?,
                candidate_pool: decode_json(&row.candidate_pool_json)?,
                slot_count: row.slot_count as u32,
                arena_kind: row.arena_kind.as_deref().map(decode_json).transpose()?,
                arena_category: row.arena_category,
                max_concurrency: row.max_concurrency as u32,
                image_size: row.image_size,
                image_aspect_ratio: row.image_aspect_ratio,
            }),
            None => None,
        })
    }

    pub async fn update_settings(&self, settings: &ConversationSettings) -> Result<()> {
        sqlx::query(
            "UPDATE conversation_settings SET
                mode = ?, system_prompt = ?, temperature = ?, model_preset_ids_json = ?,
                candidate_pool_json = ?, slot_count = ?, arena_kind = ?, arena_category = ?,
                max_concurrency = ?, image_size = ?, image_aspect_ratio = ?
             WHERE conversation_id = ?",
        )
        .bind(encode_json(&settings.mode)?)
        .bind(&settings.system_prompt)
        .bind(settings.temperature.map(|v| v as f64))
        .bind(encode_json(&settings.model_preset_ids)?)
        .bind(encode_json(&settings.candidate_pool)?)
        .bind(settings.slot_count as i64)
        .bind(settings.arena_kind.map(|v| encode_json(&v)).transpose()?)
        .bind(&settings.arena_category)
        .bind(settings.max_concurrency as i64)
        .bind(&settings.image_size)
        .bind(&settings.image_aspect_ratio)
        .bind(id_str(settings.conversation_id))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_branch(&self, id: BranchId) -> Result<Option<Branch>> {
        let row = sqlx::query_as::<_, BranchRow>(
            "SELECT id, conversation_id, parent_branch_id, fork_candidate_id, head_message_id, created_at
             FROM branches WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(Branch {
                id: parse_id(&row.id)?,
                conversation_id: parse_id(&row.conversation_id)?,
                parent_branch_id: row.parent_branch_id.as_deref().map(parse_id).transpose()?,
                fork_candidate_id: row.fork_candidate_id.as_deref().map(parse_id).transpose()?,
                head_message_id: row.head_message_id.as_deref().map(parse_id).transpose()?,
                created_at: parse_dt(&row.created_at)?,
            }),
            None => None,
        })
    }

    pub async fn upsert_branch(&self, branch: &Branch) -> Result<()> {
        sqlx::query(
            "INSERT INTO branches (id, conversation_id, parent_branch_id, fork_candidate_id, head_message_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                parent_branch_id = excluded.parent_branch_id,
                fork_candidate_id = excluded.fork_candidate_id,
                head_message_id = excluded.head_message_id",
        )
        .bind(id_str(branch.id))
        .bind(id_str(branch.conversation_id))
        .bind(branch.parent_branch_id.map(id_str))
        .bind(branch.fork_candidate_id.map(id_str))
        .bind(branch.head_message_id.map(id_str))
        .bind(dt_str(branch.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn touch_conversation(&self, id: ConversationId) -> Result<()> {
        sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
            .bind(dt_str(Utc::now()))
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_active_branch(
        &self,
        conversation_id: ConversationId,
        branch_id: BranchId,
    ) -> Result<()> {
        sqlx::query("UPDATE conversations SET active_branch_id = ?, updated_at = ? WHERE id = ?")
            .bind(id_str(branch_id))
            .bind(dt_str(Utc::now()))
            .bind(id_str(conversation_id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_conversation_mode(
        &self,
        id: ConversationId,
        mode: ConversationMode,
    ) -> Result<()> {
        sqlx::query("UPDATE conversations SET mode = ?, updated_at = ? WHERE id = ?")
            .bind(encode_json(&mode)?)
            .bind(dt_str(Utc::now()))
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
