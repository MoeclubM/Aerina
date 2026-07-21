use super::*;

impl Db {
    pub async fn insert_analytics_event(&self, event: &AnalyticsEvent) -> Result<()> {
        sqlx::query(
            "INSERT INTO analytics_events (
                id, event_type, workspace_id, conversation_id, round_id, candidate_id, payload_json, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(event.id))
        .bind(encode_json(&event.event_type)?)
        .bind(id_str(event.workspace_id))
        .bind(event.conversation_id.map(id_str))
        .bind(event.round_id.map(id_str))
        .bind(event.candidate_id.map(id_str))
        .bind(encode_json(&event.payload)?)
        .bind(dt_str(event.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_analytics_events(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<AnalyticsEvent>> {
        let rows = sqlx::query_as::<_, AnalyticsRow>(
            "SELECT id, event_type, workspace_id, conversation_id, round_id, candidate_id, payload_json, created_at
             FROM analytics_events WHERE workspace_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(workspace_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(AnalyticsEvent {
                    id: parse_id(&row.id)?,
                    event_type: decode_json(&row.event_type)?,
                    workspace_id: parse_id(&row.workspace_id)?,
                    conversation_id: row.conversation_id.as_deref().map(parse_id).transpose()?,
                    round_id: row.round_id.as_deref().map(parse_id).transpose()?,
                    candidate_id: row.candidate_id.as_deref().map(parse_id).transpose()?,
                    payload: decode_json(&row.payload_json)?,
                    created_at: parse_dt(&row.created_at)?,
                })
            })
            .collect()
    }
}
