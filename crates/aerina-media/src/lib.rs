use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

#[derive(Clone)]
pub struct MediaStore {
    root: PathBuf,
}

impl MediaStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn save_generated_image(
        &self,
        bytes: &[u8],
        extension: &str,
    ) -> Result<(String, PathBuf)> {
        let dir = self.root.join("generated-images");
        fs::create_dir_all(&dir).await?;
        let relative = format!("generated-images/{}.{}", Uuid::now_v7(), extension);
        let absolute = self.root.join(&relative);
        fs::write(&absolute, bytes).await?;
        Ok((relative, absolute))
    }

    pub async fn save_attachment(
        &self,
        bytes: &[u8],
        extension: &str,
    ) -> Result<(String, PathBuf)> {
        let dir = self.root.join("attachments");
        fs::create_dir_all(&dir).await?;
        let relative = format!("attachments/{}.{}", Uuid::now_v7(), extension);
        let absolute = self.root.join(&relative);
        fs::write(&absolute, bytes).await?;
        Ok((relative, absolute))
    }

    pub async fn save_profile_avatar(
        &self,
        profile_id: &str,
        bytes: &[u8],
        extension: &str,
    ) -> Result<(String, PathBuf)> {
        let dir = self.root.join("profile-avatars");
        fs::create_dir_all(&dir).await?;
        let relative = format!("profile-avatars/{profile_id}.{extension}");
        let absolute = self.root.join(&relative);
        fs::write(&absolute, bytes).await?;
        Ok((relative, absolute))
    }

    pub async fn remove_if_exists(&self, relative_path: &str) -> Result<()> {
        let absolute = self.root.join(relative_path);
        if fs::try_exists(&absolute).await? {
            fs::remove_file(&absolute).await?;
        }
        Ok(())
    }

    pub async fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>> {
        let absolute = self.root.join(relative_path);
        Ok(fs::read(absolute).await?)
    }

    pub async fn data_url(&self, relative_path: &str, mime: &str) -> Result<String> {
        let bytes = self.read_bytes(relative_path).await?;
        Ok(format!("data:{};base64,{}", mime, STANDARD.encode(bytes)))
    }

    pub fn decode_data_url(data_url: &str) -> Result<(String, Vec<u8>)> {
        let (meta, data) = data_url
            .split_once(',')
            .ok_or_else(|| anyhow!("invalid data url"))?;
        let mime = meta
            .trim_start_matches("data:")
            .split(';')
            .next()
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = STANDARD.decode(data)?;
        Ok((mime, bytes))
    }
}
