use super::*;

impl Db {
    pub async fn insert_provider(&self, provider: &Provider) -> Result<()> {
        sqlx::query(
            "INSERT INTO providers (id, workspace_id, name, kind, base_url, secret_ref, enabled, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(provider.id))
        .bind(id_str(provider.workspace_id))
        .bind(&provider.name)
        .bind(encode_json(&provider.kind)?)
        .bind(&provider.base_url)
        .bind(&provider.secret_ref)
        .bind(provider.enabled as i64)
        .bind(dt_str(provider.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_providers(&self, workspace_id: WorkspaceId) -> Result<Vec<Provider>> {
        let rows = sqlx::query_as::<_, ProviderRow>(
            "SELECT id, workspace_id, name, kind, base_url, secret_ref, enabled, created_at
             FROM providers WHERE workspace_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(workspace_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Provider {
                    id: parse_id(&row.id)?,
                    workspace_id: parse_id(&row.workspace_id)?,
                    name: row.name,
                    kind: decode_json(&row.kind)?,
                    base_url: row.base_url,
                    secret_ref: row.secret_ref,
                    enabled: row.enabled != 0,
                    created_at: parse_dt(&row.created_at)?,
                })
            })
            .collect()
    }

    pub async fn get_provider(&self, id: ProviderId) -> Result<Option<Provider>> {
        let row = sqlx::query_as::<_, ProviderRow>(
            "SELECT id, workspace_id, name, kind, base_url, secret_ref, enabled, created_at
             FROM providers WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(Provider {
                id: parse_id(&row.id)?,
                workspace_id: parse_id(&row.workspace_id)?,
                name: row.name,
                kind: decode_json(&row.kind)?,
                base_url: row.base_url,
                secret_ref: row.secret_ref,
                enabled: row.enabled != 0,
                created_at: parse_dt(&row.created_at)?,
            }),
            None => None,
        })
    }

    pub async fn insert_model_preset(&self, preset: &ModelPreset) -> Result<()> {
        sqlx::query(
            "INSERT INTO model_presets (
                id, workspace_id, provider_id, model_id, name, model_name, capabilities_json,
                temperature, system_prompt, in_random_pool, enabled, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(preset.id))
        .bind(id_str(preset.workspace_id))
        .bind(id_str(preset.provider_id))
        .bind(preset.model_id.map(id_str))
        .bind(&preset.name)
        .bind(&preset.model_name)
        .bind(encode_json(&preset.capabilities)?)
        .bind(preset.temperature.map(|v| v as f64))
        .bind(&preset.system_prompt)
        .bind(preset.in_random_pool as i64)
        .bind(preset.enabled as i64)
        .bind(dt_str(preset.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_model_presets(&self, workspace_id: WorkspaceId) -> Result<Vec<ModelPreset>> {
        let rows = sqlx::query_as::<_, ModelPresetRow>(
            "SELECT id, workspace_id, provider_id, model_id, name, model_name, capabilities_json,
                    temperature, system_prompt, in_random_pool, enabled, created_at
             FROM model_presets WHERE workspace_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(workspace_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ModelPreset {
                    id: parse_id(&row.id)?,
                    workspace_id: parse_id(&row.workspace_id)?,
                    provider_id: parse_id(&row.provider_id)?,
                    model_id: row.model_id.as_deref().map(parse_id).transpose()?,
                    name: row.name,
                    model_name: row.model_name,
                    capabilities: decode_json(&row.capabilities_json)?,
                    temperature: row.temperature.map(|v| v as f32),
                    system_prompt: row.system_prompt,
                    in_random_pool: row.in_random_pool != 0,
                    enabled: row.enabled != 0,
                    created_at: parse_dt(&row.created_at)?,
                })
            })
            .collect()
    }

    pub async fn get_model_preset(&self, id: ModelPresetId) -> Result<Option<ModelPreset>> {
        let row = sqlx::query_as::<_, ModelPresetRow>(
            "SELECT id, workspace_id, provider_id, model_id, name, model_name, capabilities_json,
                    temperature, system_prompt, in_random_pool, enabled, created_at
             FROM model_presets WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(ModelPreset {
                id: parse_id(&row.id)?,
                workspace_id: parse_id(&row.workspace_id)?,
                provider_id: parse_id(&row.provider_id)?,
                model_id: row.model_id.as_deref().map(parse_id).transpose()?,
                name: row.name,
                model_name: row.model_name,
                capabilities: decode_json(&row.capabilities_json)?,
                temperature: row.temperature.map(|v| v as f32),
                system_prompt: row.system_prompt,
                in_random_pool: row.in_random_pool != 0,
                enabled: row.enabled != 0,
                created_at: parse_dt(&row.created_at)?,
            }),
            None => None,
        })
    }

    pub async fn update_provider(&self, provider: &Provider) -> Result<()> {
        sqlx::query(
            "UPDATE providers SET name = ?, kind = ?, base_url = ?, secret_ref = ?, enabled = ?
             WHERE id = ?",
        )
        .bind(&provider.name)
        .bind(encode_json(&provider.kind)?)
        .bind(&provider.base_url)
        .bind(&provider.secret_ref)
        .bind(provider.enabled as i64)
        .bind(id_str(provider.id))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_provider(&self, id: ProviderId) -> Result<()> {
        sqlx::query("DELETE FROM model_presets WHERE provider_id = ?")
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_model_preset(&self, preset: &ModelPreset) -> Result<()> {
        sqlx::query(
            "UPDATE model_presets SET
                provider_id = ?, name = ?, model_name = ?, capabilities_json = ?,
                temperature = ?, system_prompt = ?, in_random_pool = ?, enabled = ?
             WHERE id = ?",
        )
        .bind(id_str(preset.provider_id))
        .bind(&preset.name)
        .bind(&preset.model_name)
        .bind(encode_json(&preset.capabilities)?)
        .bind(preset.temperature.map(|v| v as f64))
        .bind(&preset.system_prompt)
        .bind(preset.in_random_pool as i64)
        .bind(preset.enabled as i64)
        .bind(id_str(preset.id))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_model_preset(&self, id: ModelPresetId) -> Result<()> {
        sqlx::query("DELETE FROM model_presets WHERE id = ?")
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_model_presets_for_provider(
        &self,
        provider_id: ProviderId,
    ) -> Result<Vec<ModelPreset>> {
        let rows = sqlx::query_as::<_, ModelPresetRow>(
            "SELECT id, workspace_id, provider_id, model_id, name, model_name, capabilities_json,
                    temperature, system_prompt, in_random_pool, enabled, created_at
             FROM model_presets WHERE provider_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(provider_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ModelPreset {
                    id: parse_id(&row.id)?,
                    workspace_id: parse_id(&row.workspace_id)?,
                    provider_id: parse_id(&row.provider_id)?,
                    model_id: row.model_id.as_deref().map(parse_id).transpose()?,
                    name: row.name,
                    model_name: row.model_name,
                    capabilities: decode_json(&row.capabilities_json)?,
                    temperature: row.temperature.map(|v| v as f32),
                    system_prompt: row.system_prompt,
                    in_random_pool: row.in_random_pool != 0,
                    enabled: row.enabled != 0,
                    created_at: parse_dt(&row.created_at)?,
                })
            })
            .collect()
    }
}
