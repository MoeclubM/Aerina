use super::*;
use aerina_domain::{MediaObject, MediaObjectId};

impl Db {
    pub async fn list_branches(&self, conversation_id: ConversationId) -> Result<Vec<Branch>> {
        let rows = sqlx::query_as::<_, BranchRow>(
            "SELECT id, conversation_id, parent_branch_id, fork_candidate_id, head_message_id, created_at
             FROM branches WHERE conversation_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(conversation_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Branch {
                    id: parse_id(&row.id)?,
                    conversation_id: parse_id(&row.conversation_id)?,
                    parent_branch_id: row.parent_branch_id.as_deref().map(parse_id).transpose()?,
                    fork_candidate_id: row
                        .fork_candidate_id
                        .as_deref()
                        .map(parse_id)
                        .transpose()?,
                    head_message_id: row.head_message_id.as_deref().map(parse_id).transpose()?,
                    created_at: parse_dt(&row.created_at)?,
                })
            })
            .collect()
    }

    pub async fn get_message(
        &self,
        id: MessageNodeId,
    ) -> Result<Option<(MessageNode, Vec<ContentBlock>)>> {
        let row = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, branch_id, parent_message_id, role, round_id, candidate_id, created_at
             FROM message_nodes WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let blocks = sqlx::query_as::<_, ContentBlockRow>(
            "SELECT message_id, position, block_json FROM content_blocks
             WHERE message_id = ? ORDER BY position ASC",
        )
        .bind(&row.id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|block| decode_json(&block.block_json))
        .collect::<Result<Vec<_>>>()?;

        Ok(Some((
            MessageNode {
                id: parse_id(&row.id)?,
                conversation_id: parse_id(&row.conversation_id)?,
                branch_id: parse_id(&row.branch_id)?,
                parent_message_id: row.parent_message_id.as_deref().map(parse_id).transpose()?,
                role: decode_json(&row.role)?,
                round_id: row.round_id.as_deref().map(parse_id).transpose()?,
                candidate_id: row.candidate_id.as_deref().map(parse_id).transpose()?,
                created_at: parse_dt(&row.created_at)?,
            },
            blocks,
        )))
    }

    pub async fn insert_media_object(&self, media: &MediaObject) -> Result<()> {
        sqlx::query(
            "INSERT INTO media_objects (id, workspace_id, relative_path, mime, width, height, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(media.id))
        .bind(id_str(media.workspace_id))
        .bind(&media.relative_path)
        .bind(&media.mime)
        .bind(media.width.map(|v| v as i64))
        .bind(media.height.map(|v| v as i64))
        .bind(dt_str(media.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_media_object(&self, id: MediaObjectId) -> Result<Option<MediaObject>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<i64>,
                Option<i64>,
                String,
            ),
        >(
            "SELECT id, workspace_id, relative_path, mime, width, height, created_at
             FROM media_objects WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(MediaObject {
                id: parse_id(&row.0)?,
                workspace_id: parse_id(&row.1)?,
                relative_path: row.2,
                mime: row.3,
                width: row.4.map(|v| v as u32),
                height: row.5.map(|v| v as u32),
                created_at: parse_dt(&row.6)?,
            }),
            None => None,
        })
    }

    pub async fn list_usage_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<UsageRecord>> {
        let rows = sqlx::query_as::<_, (String, Option<i64>, Option<i64>, Option<i64>, Option<f64>, Option<i64>, Option<i64>)>(
            "SELECT u.candidate_id, u.prompt_tokens, u.completion_tokens, u.total_tokens, u.cost_usd, u.latency_ms, u.ttft_ms
             FROM usage_records u
             JOIN candidate_generations c ON c.id = u.candidate_id
             JOIN rounds r ON r.id = c.round_id
             JOIN conversations conv ON conv.id = r.conversation_id
             WHERE conv.workspace_id = ?",
        )
        .bind(id_str(workspace_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(UsageRecord {
                    candidate_id: parse_id(&row.0)?,
                    prompt_tokens: row.1.map(|v| v as u32),
                    completion_tokens: row.2.map(|v| v as u32),
                    total_tokens: row.3.map(|v| v as u32),
                    cost_usd: row.4,
                    latency_ms: row.5.map(|v| v as u64),
                    ttft_ms: row.6.map(|v| v as u64),
                })
            })
            .collect()
    }

    pub async fn get_candidate_message_blocks(
        &self,
        candidate_id: CandidateId,
    ) -> Result<Option<(MessageNode, Vec<ContentBlock>)>> {
        let row = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, branch_id, parent_message_id, role, round_id, candidate_id, created_at
             FROM message_nodes WHERE candidate_id = ? ORDER BY created_at ASC LIMIT 1",
        )
        .bind(id_str(candidate_id))
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        self.get_message(parse_id(&row.id)?).await
    }

    pub async fn update_conversation_title(&self, id: ConversationId, title: &str) -> Result<()> {
        sqlx::query("UPDATE conversations SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(dt_str(Utc::now()))
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
