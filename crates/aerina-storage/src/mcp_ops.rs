use super::*;

#[derive(Debug, sqlx::FromRow)]
struct McpServerRow {
    id: String,
    workspace_id: String,
    name: String,
    transport: String,
    url: String,
    headers_json: String,
    enabled: i64,
    created_at: String,
}

impl Db {
    pub async fn list_mcp_servers(&self, workspace_id: WorkspaceId) -> Result<Vec<McpServer>> {
        let rows = sqlx::query_as::<_, McpServerRow>(
            "SELECT id, workspace_id, name, transport, url, headers_json, enabled, created_at
             FROM mcp_servers WHERE workspace_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(workspace_id))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(McpServer {
                    id: parse_id(&row.id)?,
                    workspace_id: parse_id(&row.workspace_id)?,
                    name: row.name,
                    transport: decode_json(&row.transport)?,
                    url: row.url,
                    headers: decode_json(&row.headers_json)?,
                    enabled: row.enabled != 0,
                    created_at: parse_dt(&row.created_at)?,
                })
            })
            .collect()
    }

    pub async fn insert_mcp_server(&self, server: &McpServer) -> Result<()> {
        sqlx::query(
            "INSERT INTO mcp_servers (id, workspace_id, name, transport, url, headers_json, enabled, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(server.id))
        .bind(id_str(server.workspace_id))
        .bind(&server.name)
        .bind(encode_json(&server.transport)?)
        .bind(&server.url)
        .bind(encode_json(&server.headers)?)
        .bind(server.enabled as i64)
        .bind(dt_str(server.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_mcp_server(&self, server: &McpServer) -> Result<()> {
        sqlx::query(
            "UPDATE mcp_servers SET name = ?, transport = ?, url = ?, headers_json = ?, enabled = ?
             WHERE id = ?",
        )
        .bind(&server.name)
        .bind(encode_json(&server.transport)?)
        .bind(&server.url)
        .bind(encode_json(&server.headers)?)
        .bind(server.enabled as i64)
        .bind(id_str(server.id))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_mcp_server(&self, id: McpServerId) -> Result<()> {
        sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
