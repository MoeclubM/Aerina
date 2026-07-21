use super::*;

impl Db {
    pub async fn replace_message_blocks(
        &self,
        message_id: MessageNodeId,
        blocks: &[ContentBlock],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM content_blocks WHERE message_id = ?")
            .bind(id_str(message_id))
            .execute(&mut *tx)
            .await?;
        for (position, block) in blocks.iter().enumerate() {
            sqlx::query(
                "INSERT INTO content_blocks (message_id, position, block_json) VALUES (?, ?, ?)",
            )
            .bind(id_str(message_id))
            .bind(position as i64)
            .bind(encode_json(block)?)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn backup_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        let path_str = path.to_string_lossy().replace('\'', "''");
        sqlx::query(&format!("VACUUM INTO '{path_str}'"))
            .execute(&self.pool)
            .await
            .with_context(|| format!("failed to backup sqlite to {}", path.display()))?;
        Ok(())
    }
}
