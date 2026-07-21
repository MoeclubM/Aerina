use super::*;

impl Db {
    pub async fn insert_ranking_event(&self, event: &RankingEvent) -> Result<()> {
        sqlx::query(
            "INSERT INTO ranking_events (
                id, workspace_id, arena_kind, category, winner_preset_id, loser_preset_id, round_id, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(event.id))
        .bind(id_str(event.workspace_id))
        .bind(encode_json(&event.arena_kind)?)
        .bind(&event.category)
        .bind(id_str(event.winner_preset_id))
        .bind(id_str(event.loser_preset_id))
        .bind(id_str(event.round_id))
        .bind(dt_str(event.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_ranking_events(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RankingEvent>> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, String, String, String, String)>(
            "SELECT id, workspace_id, arena_kind, category, winner_preset_id, loser_preset_id, round_id, created_at
             FROM ranking_events WHERE workspace_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(workspace_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(RankingEvent {
                    id: parse_id(&row.0)?,
                    workspace_id: parse_id(&row.1)?,
                    arena_kind: decode_json(&row.2)?,
                    category: row.3,
                    winner_preset_id: parse_id(&row.4)?,
                    loser_preset_id: parse_id(&row.5)?,
                    round_id: parse_id(&row.6)?,
                    created_at: parse_dt(&row.7)?,
                })
            })
            .collect()
    }

    pub async fn delete_conversation(&self, conversation_id: ConversationId) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "DELETE FROM content_blocks WHERE message_id IN (
                SELECT id FROM message_nodes WHERE conversation_id = ?
             )",
        )
        .bind(id_str(conversation_id))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM usage_records WHERE candidate_id IN (
            SELECT id FROM candidate_generations WHERE round_id IN (
                SELECT id FROM rounds WHERE conversation_id = ?
            )
        )",
        )
        .bind(id_str(conversation_id))
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM candidate_generations WHERE round_id IN (SELECT id FROM rounds WHERE conversation_id = ?)")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM round_snapshots WHERE round_id IN (SELECT id FROM rounds WHERE conversation_id = ?)")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM arena_votes WHERE round_id IN (SELECT id FROM rounds WHERE conversation_id = ?)")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM rounds WHERE conversation_id = ?")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM message_nodes WHERE conversation_id = ?")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM branches WHERE conversation_id = ?")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM conversation_settings WHERE conversation_id = ?")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM arena_profiles WHERE conversation_id = ?")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(id_str(conversation_id))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_round(&self, id: RoundId) -> Result<Option<Round>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, String)>(
            "SELECT id, conversation_id, branch_id, user_message_id, selected_candidate_id, created_at
             FROM rounds WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;
        Ok(match row {
            Some(row) => Some(Round {
                id: parse_id(&row.0)?,
                conversation_id: parse_id(&row.1)?,
                branch_id: parse_id(&row.2)?,
                user_message_id: parse_id(&row.3)?,
                selected_candidate_id: row.4.as_deref().map(parse_id).transpose()?,
                created_at: parse_dt(&row.5)?,
            }),
            None => None,
        })
    }

    pub async fn insert_arena_vote(&self, vote: &ArenaVote) -> Result<()> {
        sqlx::query(
            "INSERT INTO arena_votes (id, round_id, vote_kind, selected_candidate_id, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id_str(vote.id))
        .bind(id_str(vote.round_id))
        .bind(encode_json(&vote.vote_kind)?)
        .bind(vote.selected_candidate_id.map(id_str))
        .bind(dt_str(vote.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
