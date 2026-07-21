use super::*;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

impl AppState {
    pub async fn create_backup(&self) -> Result<BackupInfo> {
        let root = self.inner.media.root().to_path_buf();
        let name = Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let dest = root.join("backups").join(&name);
        fs::create_dir_all(&dest)?;

        self.inner.db.backup_to(dest.join("aerina.db")).await?;

        let secrets = root.join("secrets.json");
        if secrets.exists() {
            fs::copy(&secrets, dest.join("secrets.json"))?;
        }
        for dir_name in ["generated-images", "attachments"] {
            let src = root.join(dir_name);
            if src.exists() {
                copy_dir(&src, &dest.join(dir_name))?;
            }
        }

        Ok(BackupInfo {
            name,
            path: dest.display().to_string(),
        })
    }

    pub async fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let root = self.inner.media.root().join("backups");
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut items = Vec::new();
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let path = entry.path();
                items.push(BackupInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: path.display().to_string(),
                });
            }
        }
        items.sort_by(|a, b| b.name.cmp(&a.name));
        Ok(items)
    }

    pub async fn restore_backup(&self, name: &str) -> Result<String> {
        let root = self.inner.media.root().to_path_buf();
        let src = root.join("backups").join(name);
        if !src.is_dir() {
            return Err(anyhow!("backup not found: {name}"));
        }
        let db_src = src.join("aerina.db");
        if !db_src.exists() {
            return Err(anyhow!("backup missing aerina.db"));
        }

        fs::copy(&db_src, root.join("aerina.db.incoming"))?;

        let secrets_src = src.join("secrets.json");
        if secrets_src.exists() {
            fs::copy(&secrets_src, root.join("secrets.json"))?;
        }

        for dir_name in ["generated-images", "attachments"] {
            let from = src.join(dir_name);
            let to = root.join(dir_name);
            if from.exists() {
                if to.exists() {
                    fs::remove_dir_all(&to)?;
                }
                copy_dir(&from, &to)?;
            }
        }

        Ok(format!(
            "Restore staged from {name}. Restart Aerina to load the restored database."
        ))
    }

    pub(crate) fn apply_pending_db_restore(data_dir: &Path) -> Result<()> {
        let incoming = data_dir.join("aerina.db.incoming");
        if !incoming.exists() {
            return Ok(());
        }
        let db_path = data_dir.join("aerina.db");
        if db_path.exists() {
            fs::remove_file(&db_path)?;
        }
        let _ = fs::remove_file(data_dir.join("aerina.db-wal"));
        let _ = fs::remove_file(data_dir.join("aerina.db-shm"));
        fs::rename(&incoming, &db_path)?;
        Ok(())
    }
}
