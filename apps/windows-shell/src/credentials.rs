use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Link Key + ephemeral id + relay URL for joining a Sync Group.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShellCredentials {
    pub ephemeral_id: Vec<u8>,
    pub link_key: Vec<u8>,
    pub relay_ws_url: String,
}

pub trait LinkKeyStoring {
    fn delete(&mut self) -> Result<(), StoreError>;
    fn load(&self) -> Result<Option<ShellCredentials>, StoreError>;
    fn save(&mut self, credentials: &ShellCredentials) -> Result<(), StoreError>;
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Default)]
pub struct InMemoryCredentialStore {
    value: Option<ShellCredentials>,
}

impl LinkKeyStoring for InMemoryCredentialStore {
    fn save(&mut self, credentials: &ShellCredentials) -> Result<(), StoreError> {
        self.value = Some(credentials.clone());
        Ok(())
    }

    fn load(&self) -> Result<Option<ShellCredentials>, StoreError> {
        Ok(self.value.clone())
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        self.value = None;
        Ok(())
    }
}

/// JSON file credentials under the Shell config directory.
pub struct FileCredentialStore {
    path: PathBuf,
}

impl FileCredentialStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> PathBuf {
        default_config_dir().join("credentials.json")
    }
}

impl LinkKeyStoring for FileCredentialStore {
    fn save(&mut self, credentials: &ShellCredentials) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec_pretty(credentials)?;
        fs::write(&self.path, data)?;
        Ok(())
    }

    fn load(&self) -> Result<Option<ShellCredentials>, StoreError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let data = fs::read(&self.path)?;
        Ok(Some(serde_json::from_slice(&data)?))
    }

    fn delete(&mut self) -> Result<(), StoreError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

pub(crate) fn default_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SyncClip")
}

pub(crate) fn ensure_parent(path: &Path) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
