use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Default, Serialize, Deserialize)]
struct SecretFile {
    values: HashMap<String, String>,
}

pub struct SecretStore {
    path: PathBuf,
    memory: RwLock<HashMap<String, String>>,
}

impl SecretStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let memory = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read secrets at {}", path.display()))?;
            let parsed: SecretFile = serde_json::from_str(&raw)?;
            parsed.values
        } else {
            HashMap::new()
        };
        Ok(Self {
            path,
            memory: RwLock::new(memory),
        })
    }

    pub fn put(&self, key: &str, value: &str) -> Result<()> {
        {
            self.memory
                .write()
                .expect("secret store lock")
                .insert(key.to_string(), value.to_string());
        }
        self.persist()
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .memory
            .read()
            .expect("secret store lock")
            .get(key)
            .cloned())
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        self.memory.write().expect("secret store lock").remove(key);
        self.persist()
    }

    fn persist(&self) -> Result<()> {
        let values = self.memory.read().expect("secret store lock").clone();
        let payload = SecretFile { values };
        let raw = serde_json::to_string_pretty(&payload)?;
        std::fs::write(&self.path, raw)?;
        Ok(())
    }
}
