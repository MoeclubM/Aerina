use super::*;
use anyhow::anyhow;
use chrono::Utc;

impl Db {
    pub async fn ensure_bootstrap(&self) -> Result<(Profile, Workspace)> {
        if let Some(active_id) = self.get_active_workspace_id().await? {
            if let Some(workspace) = self.get_workspace(active_id).await? {
                let profile = self
                    .get_profile(workspace.profile_id)
                    .await?
                    .ok_or_else(|| anyhow!("profile missing for workspace"))?;
                return Ok((profile, workspace));
            }
        }

        if let Some(workspace) = self.get_default_workspace().await? {
            self.set_active_workspace_id(workspace.id).await?;
            let profile = self
                .get_profile(workspace.profile_id)
                .await?
                .ok_or_else(|| anyhow!("profile missing for workspace"))?;
            return Ok((profile, workspace));
        }

        let (profile, workspace) = self
            .create_profile_with_workspace("Local User", "Default")
            .await?;
        self.set_active_workspace_id(workspace.id).await?;
        Ok((profile, workspace))
    }

    pub async fn create_profile_with_workspace(
        &self,
        display_name: &str,
        workspace_name: &str,
    ) -> Result<(Profile, Workspace)> {
        let now = Utc::now();
        let profile = Profile {
            id: ProfileId::new(),
            display_name: display_name.trim().to_string(),
            avatar_path: None,
            auth_subject: None,
            auth_provider: None,
            created_at: now,
        };
        let workspace = Workspace {
            id: WorkspaceId::new(),
            profile_id: profile.id,
            name: workspace_name.trim().to_string(),
            created_at: now,
        };

        sqlx::query(
            "INSERT INTO profiles (id, display_name, avatar_path, auth_subject, auth_provider, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id_str(profile.id))
        .bind(&profile.display_name)
        .bind(profile.avatar_path.as_deref())
        .bind(profile.auth_subject.as_deref())
        .bind(profile.auth_provider.as_deref())
        .bind(dt_str(profile.created_at))
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO workspaces (id, profile_id, name, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(id_str(workspace.id))
        .bind(id_str(workspace.profile_id))
        .bind(&workspace.name)
        .bind(dt_str(workspace.created_at))
        .execute(&self.pool)
        .await?;

        Ok((profile, workspace))
    }

    pub async fn get_profile(&self, id: ProfileId) -> Result<Option<Profile>> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT id, display_name, avatar_path, auth_subject, auth_provider, created_at FROM profiles WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some((id, display_name, avatar_path, auth_subject, auth_provider, created_at)) => {
                Some(Profile {
                    id: parse_id(&id)?,
                    display_name,
                    avatar_path,
                    auth_subject,
                    auth_provider,
                    created_at: parse_dt(&created_at)?,
                })
            }
            None => None,
        })
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT id, display_name, avatar_path, auth_subject, auth_provider, created_at FROM profiles ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(
                |(id, display_name, avatar_path, auth_subject, auth_provider, created_at)| {
                    Ok(Profile {
                        id: parse_id(&id)?,
                        display_name,
                        avatar_path,
                        auth_subject,
                        auth_provider,
                        created_at: parse_dt(&created_at)?,
                    })
                },
            )
            .collect()
    }

    pub async fn rename_profile(&self, id: ProfileId, display_name: &str) -> Result<Profile> {
        let name = display_name.trim();
        if name.is_empty() {
            return Err(anyhow!("profile name is empty"));
        }
        sqlx::query("UPDATE profiles SET display_name = ? WHERE id = ?")
            .bind(name)
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        self.get_profile(id)
            .await?
            .ok_or_else(|| anyhow!("profile not found"))
    }

    pub async fn update_profile_avatar(
        &self,
        id: ProfileId,
        avatar_path: Option<&str>,
    ) -> Result<Profile> {
        sqlx::query("UPDATE profiles SET avatar_path = ? WHERE id = ?")
            .bind(avatar_path)
            .bind(id_str(id))
            .execute(&self.pool)
            .await?;
        self.get_profile(id)
            .await?
            .ok_or_else(|| anyhow!("profile not found"))
    }

    pub async fn get_workspace(&self, id: WorkspaceId) -> Result<Option<Workspace>> {
        let row = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, profile_id, name, created_at FROM workspaces WHERE id = ?",
        )
        .bind(id_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some((id, profile_id, name, created_at)) => Some(Workspace {
                id: parse_id(&id)?,
                profile_id: parse_id(&profile_id)?,
                name,
                created_at: parse_dt(&created_at)?,
            }),
            None => None,
        })
    }

    pub async fn list_workspaces(&self, profile_id: ProfileId) -> Result<Vec<Workspace>> {
        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, profile_id, name, created_at FROM workspaces WHERE profile_id = ? ORDER BY created_at ASC",
        )
        .bind(id_str(profile_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(id, profile_id, name, created_at)| {
                Ok(Workspace {
                    id: parse_id(&id)?,
                    profile_id: parse_id(&profile_id)?,
                    name,
                    created_at: parse_dt(&created_at)?,
                })
            })
            .collect()
    }

    pub async fn get_default_workspace(&self) -> Result<Option<Workspace>> {
        let row = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, profile_id, name, created_at FROM workspaces ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some((id, profile_id, name, created_at)) => Some(Workspace {
                id: parse_id(&id)?,
                profile_id: parse_id(&profile_id)?,
                name,
                created_at: parse_dt(&created_at)?,
            }),
            None => None,
        })
    }

    pub async fn get_active_workspace_id(&self) -> Result<Option<WorkspaceId>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM app_settings WHERE key = 'active_workspace_id'",
        )
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((value,)) => Ok(Some(parse_id(&value)?)),
            None => Ok(None),
        }
    }

    pub async fn set_active_workspace_id(&self, workspace_id: WorkspaceId) -> Result<()> {
        sqlx::query(
            "INSERT INTO app_settings (key, value) VALUES ('active_workspace_id', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(id_str(workspace_id))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
