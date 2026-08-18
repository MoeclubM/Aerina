use super::*;
use aerina_domain::descendant_ids_inclusive;
use std::collections::{HashMap, HashSet};

impl Db {
    pub async fn insert_message(
        &self,
        message: &MessageNode,
        blocks: &[ContentBlock],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO message_nodes (
                id, conversation_id, branch_id, parent_message_id, role, round_id, candidate_id, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(message.id))
        .bind(id_str(message.conversation_id))
        .bind(id_str(message.branch_id))
        .bind(message.parent_message_id.map(id_str))
        .bind(encode_json(&message.role)?)
        .bind(message.round_id.map(id_str))
        .bind(message.candidate_id.map(id_str))
        .bind(dt_str(message.created_at))
        .execute(&mut *tx)
        .await?;

        for (position, block) in blocks.iter().enumerate() {
            sqlx::query(
                "INSERT INTO content_blocks (message_id, position, block_json) VALUES (?, ?, ?)",
            )
            .bind(id_str(message.id))
            .bind(position as i64)
            .bind(encode_json(block)?)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn hydrate_messages(
        &self,
        rows: Vec<MessageRow>,
    ) -> Result<Vec<(MessageNode, Vec<ContentBlock>)>> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = rows.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT message_id, position, block_json FROM content_blocks
             WHERE message_id IN ({placeholders})
             ORDER BY message_id ASC, position ASC"
        );
        let mut query = sqlx::query_as::<_, ContentBlockRow>(&sql);
        for row in &rows {
            query = query.bind(&row.id);
        }
        let block_rows = query.fetch_all(&self.pool).await?;

        let mut blocks_by_message: HashMap<String, Vec<ContentBlock>> = HashMap::new();
        for block in block_rows {
            let decoded = decode_json(&block.block_json)?;
            blocks_by_message
                .entry(block.message_id)
                .or_default()
                .push(decoded);
        }

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let blocks = blocks_by_message.remove(&row.id).unwrap_or_default();
            result.push((
                MessageNode {
                    id: parse_id(&row.id)?,
                    conversation_id: parse_id(&row.conversation_id)?,
                    branch_id: parse_id(&row.branch_id)?,
                    parent_message_id: row
                        .parent_message_id
                        .as_deref()
                        .map(parse_id)
                        .transpose()?,
                    role: decode_json(&row.role)?,
                    round_id: row.round_id.as_deref().map(parse_id).transpose()?,
                    candidate_id: row.candidate_id.as_deref().map(parse_id).transpose()?,
                    created_at: parse_dt(&row.created_at)?,
                },
                blocks,
            ));
        }
        Ok(result)
    }

    pub async fn list_messages(
        &self,
        branch_id: BranchId,
    ) -> Result<Vec<(MessageNode, Vec<ContentBlock>)>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, branch_id, parent_message_id, role, round_id, candidate_id, created_at
             FROM message_nodes WHERE branch_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(id_str(branch_id))
        .fetch_all(&self.pool)
        .await?;
        self.hydrate_messages(rows).await
    }

    /// Walk `parent_message_id` from `head_id`, including messages on ancestor branches.
    pub async fn list_message_path(
        &self,
        head_id: Option<MessageNodeId>,
    ) -> Result<Vec<(MessageNode, Vec<ContentBlock>)>> {
        let mut path = Vec::new();
        let mut current = head_id;
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                break;
            }
            match self.get_message(id).await? {
                Some((message, blocks)) => {
                    current = message.parent_message_id;
                    path.push((message, blocks));
                }
                None => break,
            }
        }
        path.reverse();
        Ok(path)
    }

    /// Delete `from_id` and every descendant on this branch, plus related rounds.
    pub async fn truncate_branch_from(
        &self,
        branch_id: BranchId,
        from_id: MessageNodeId,
    ) -> Result<()> {
        let messages = self.list_messages(branch_id).await?;
        let nodes = messages
            .iter()
            .map(|(message, _)| message.clone())
            .collect::<Vec<_>>();
        let drop_ids = descendant_ids_inclusive(&nodes, from_id);
        if drop_ids.is_empty() {
            return Ok(());
        }
        let drop_set = drop_ids.iter().copied().collect::<HashSet<_>>();

        let mut round_ids = HashSet::new();
        for (message, _) in &messages {
            if drop_set.contains(&message.id) {
                if let Some(round_id) = message.round_id {
                    round_ids.insert(round_id);
                }
            }
        }
        if let Some((_, conversation_id)) = messages
            .first()
            .map(|(message, _)| (message.branch_id, message.conversation_id))
        {
            for round in self.list_rounds_for_conversation(conversation_id).await? {
                if round.branch_id == branch_id && drop_set.contains(&round.user_message_id) {
                    round_ids.insert(round.id);
                }
            }
        }

        let mut tx = self.pool.begin().await?;
        for round_id in &round_ids {
            sqlx::query(
                "DELETE FROM usage_records WHERE candidate_id IN (
                    SELECT id FROM candidate_generations WHERE round_id = ?
                )",
            )
            .bind(id_str(*round_id))
            .execute(&mut *tx)
            .await?;
            sqlx::query("DELETE FROM arena_votes WHERE round_id = ?")
                .bind(id_str(*round_id))
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM ranking_events WHERE round_id = ?")
                .bind(id_str(*round_id))
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM candidate_generations WHERE round_id = ?")
                .bind(id_str(*round_id))
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM round_snapshots WHERE round_id = ?")
                .bind(id_str(*round_id))
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM rounds WHERE id = ?")
                .bind(id_str(*round_id))
                .execute(&mut *tx)
                .await?;
        }
        for message_id in &drop_ids {
            sqlx::query("DELETE FROM content_blocks WHERE message_id = ?")
                .bind(id_str(*message_id))
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM message_nodes WHERE id = ?")
                .bind(id_str(*message_id))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Latest `limit` messages in chronological order, plus whether older messages exist.
    pub async fn list_messages_tail(
        &self,
        branch_id: BranchId,
        limit: u32,
    ) -> Result<(Vec<(MessageNode, Vec<ContentBlock>)>, bool)> {
        let fetch = (limit as i64) + 1;
        let mut rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, branch_id, parent_message_id, role, round_id, candidate_id, created_at
             FROM message_nodes
             WHERE branch_id = ?
             ORDER BY created_at DESC, id DESC
             LIMIT ?",
        )
        .bind(id_str(branch_id))
        .bind(fetch)
        .fetch_all(&self.pool)
        .await?;

        let has_more = rows.len() as i64 > limit as i64;
        if has_more {
            rows.truncate(limit as usize);
        }
        rows.reverse();
        Ok((self.hydrate_messages(rows).await?, has_more))
    }

    /// Messages strictly older than the given message, chronological order, windowed.
    pub async fn list_messages_before(
        &self,
        branch_id: BranchId,
        before_message_id: EntityId,
        limit: u32,
    ) -> Result<(Vec<(MessageNode, Vec<ContentBlock>)>, bool)> {
        let before = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, branch_id, parent_message_id, role, round_id, candidate_id, created_at
             FROM message_nodes WHERE id = ?",
        )
        .bind(id_str(before_message_id))
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("before message not found"))?;

        let fetch = (limit as i64) + 1;
        let mut rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, branch_id, parent_message_id, role, round_id, candidate_id, created_at
             FROM message_nodes
             WHERE branch_id = ?
               AND (
                 created_at < ?
                 OR (created_at = ? AND id < ?)
               )
             ORDER BY created_at DESC, id DESC
             LIMIT ?",
        )
        .bind(id_str(branch_id))
        .bind(&before.created_at)
        .bind(&before.created_at)
        .bind(&before.id)
        .bind(fetch)
        .fetch_all(&self.pool)
        .await?;

        let has_more = rows.len() as i64 > limit as i64;
        if has_more {
            rows.truncate(limit as usize);
        }
        rows.reverse();
        Ok((self.hydrate_messages(rows).await?, has_more))
    }

    pub async fn insert_round(&self, round: &Round, snapshot: &RoundSnapshot) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO rounds (id, conversation_id, branch_id, user_message_id, selected_candidate_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(round.id))
        .bind(id_str(round.conversation_id))
        .bind(id_str(round.branch_id))
        .bind(id_str(round.user_message_id))
        .bind(round.selected_candidate_id.map(id_str))
        .bind(dt_str(round.created_at))
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO round_snapshots (
                round_id, mode, system_prompt, temperature, model_preset_ids_json,
                arena_kind, arena_category, image_size, image_aspect_ratio, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(snapshot.round_id))
        .bind(encode_json(&snapshot.mode)?)
        .bind(&snapshot.system_prompt)
        .bind(snapshot.temperature.map(|v| v as f64))
        .bind(encode_json(&snapshot.model_preset_ids)?)
        .bind(snapshot.arena_kind.map(|v| encode_json(&v)).transpose()?)
        .bind(&snapshot.arena_category)
        .bind(&snapshot.image_size)
        .bind(&snapshot.image_aspect_ratio)
        .bind(dt_str(snapshot.created_at))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_round(&self, round: &Round) -> Result<()> {
        sqlx::query("UPDATE rounds SET selected_candidate_id = ? WHERE id = ?")
            .bind(round.selected_candidate_id.map(id_str))
            .bind(id_str(round.id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_candidate(&self, candidate: &CandidateGeneration) -> Result<()> {
        sqlx::query(
            "INSERT INTO candidate_generations (
                id, round_id, slot_label, model_preset_id, provider_id, model_name,
                status, anonymous, error_message, created_at, completed_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(candidate.id))
        .bind(id_str(candidate.round_id))
        .bind(&candidate.slot_label)
        .bind(id_str(candidate.model_preset_id))
        .bind(id_str(candidate.provider_id))
        .bind(&candidate.model_name)
        .bind(encode_json(&candidate.status)?)
        .bind(candidate.anonymous as i64)
        .bind(&candidate.error_message)
        .bind(dt_str(candidate.created_at))
        .bind(candidate.completed_at.map(dt_str))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_candidate(&self, candidate: &CandidateGeneration) -> Result<()> {
        sqlx::query(
            "UPDATE candidate_generations
             SET status = ?, error_message = ?, completed_at = ?
             WHERE id = ?",
        )
        .bind(encode_json(&candidate.status)?)
        .bind(&candidate.error_message)
        .bind(candidate.completed_at.map(dt_str))
        .bind(id_str(candidate.id))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_candidate(&self, id: CandidateId) -> Result<Option<CandidateGeneration>> {
        let row = sqlx::query_as::<_, CandidateRow>(
            "SELECT id, round_id, slot_label, model_preset_id, provider_id, model_name,
                    status, anonymous, error_message, created_at, completed_at
             FROM candidate_generations WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(CandidateGeneration {
                id: parse_id(&row.id)?,
                round_id: parse_id(&row.round_id)?,
                slot_label: row.slot_label,
                model_preset_id: parse_id(&row.model_preset_id)?,
                provider_id: parse_id(&row.provider_id)?,
                model_name: row.model_name,
                status: decode_json(&row.status)?,
                anonymous: row.anonymous != 0,
                error_message: row.error_message,
                created_at: parse_dt(&row.created_at)?,
                completed_at: row.completed_at.as_deref().map(parse_dt).transpose()?,
            }),
            None => None,
        })
    }

    pub async fn list_candidates(&self, round_id: RoundId) -> Result<Vec<CandidateGeneration>> {
        let rows = sqlx::query_as::<_, CandidateRow>(
            "SELECT id, round_id, slot_label, model_preset_id, provider_id, model_name,
                    status, anonymous, error_message, created_at, completed_at
             FROM candidate_generations WHERE round_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(round_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(CandidateGeneration {
                    id: parse_id(&row.id)?,
                    round_id: parse_id(&row.round_id)?,
                    slot_label: row.slot_label,
                    model_preset_id: parse_id(&row.model_preset_id)?,
                    provider_id: parse_id(&row.provider_id)?,
                    model_name: row.model_name,
                    status: decode_json(&row.status)?,
                    anonymous: row.anonymous != 0,
                    error_message: row.error_message,
                    created_at: parse_dt(&row.created_at)?,
                    completed_at: row.completed_at.as_deref().map(parse_dt).transpose()?,
                })
            })
            .collect()
    }

    pub async fn list_rounds_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<Round>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, String)>(
            "SELECT id, conversation_id, branch_id, user_message_id, selected_candidate_id, created_at
             FROM rounds WHERE conversation_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(conversation_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Round {
                    id: parse_id(&row.0)?,
                    conversation_id: parse_id(&row.1)?,
                    branch_id: parse_id(&row.2)?,
                    user_message_id: parse_id(&row.3)?,
                    selected_candidate_id: row.4.as_deref().map(parse_id).transpose()?,
                    created_at: parse_dt(&row.5)?,
                })
            })
            .collect()
    }

    pub async fn list_candidates_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<CandidateGeneration>> {
        let rows = sqlx::query_as::<_, CandidateRow>(
            "SELECT c.id, c.round_id, c.slot_label, c.model_preset_id, c.provider_id, c.model_name,
                    c.status, c.anonymous, c.error_message, c.created_at, c.completed_at
             FROM candidate_generations c
             JOIN rounds r ON r.id = c.round_id
             WHERE r.conversation_id = ?
             ORDER BY c.created_at ASC",
        )
        .bind(id_str(conversation_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(CandidateGeneration {
                    id: parse_id(&row.id)?,
                    round_id: parse_id(&row.round_id)?,
                    slot_label: row.slot_label,
                    model_preset_id: parse_id(&row.model_preset_id)?,
                    provider_id: parse_id(&row.provider_id)?,
                    model_name: row.model_name,
                    status: decode_json(&row.status)?,
                    anonymous: row.anonymous != 0,
                    error_message: row.error_message,
                    created_at: parse_dt(&row.created_at)?,
                    completed_at: row.completed_at.as_deref().map(parse_dt).transpose()?,
                })
            })
            .collect()
    }

    pub async fn insert_usage(&self, usage: &UsageRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO usage_records (
                candidate_id, prompt_tokens, completion_tokens, output_tokens, total_tokens,
                cost_usd, latency_ms, ttft_ms, reasoning_tokens, reasoning_duration_ms
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(candidate_id) DO UPDATE SET
                prompt_tokens = excluded.prompt_tokens,
                completion_tokens = excluded.completion_tokens,
                output_tokens = excluded.output_tokens,
                total_tokens = excluded.total_tokens,
                cost_usd = excluded.cost_usd,
                latency_ms = excluded.latency_ms,
                ttft_ms = excluded.ttft_ms,
                reasoning_tokens = excluded.reasoning_tokens,
                reasoning_duration_ms = excluded.reasoning_duration_ms",
        )
        .bind(id_str(usage.candidate_id))
        .bind(usage.prompt_tokens.map(|v| v as i64))
        .bind(usage.completion_tokens.map(|v| v as i64))
        .bind(usage.output_tokens.map(|v| v as i64))
        .bind(usage.total_tokens.map(|v| v as i64))
        .bind(usage.cost_usd)
        .bind(usage.latency_ms.map(|v| v as i64))
        .bind(usage.ttft_ms.map(|v| v as i64))
        .bind(usage.reasoning_tokens.map(|v| v as i64))
        .bind(usage.reasoning_duration_ms.map(|v| v as i64))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
