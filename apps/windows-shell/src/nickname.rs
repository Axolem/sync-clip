use crate::credentials::{default_config_dir, ensure_parent, StoreError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Optional Local Nickname stored only on this Device for UI (never on the wire).
pub trait LocalNicknameStoring {
    fn clear(&mut self);
    fn load(&self) -> Option<String>;
    fn save(&mut self, nickname: &str);
}

#[derive(Default, Serialize, Deserialize)]
struct NicknameDisk {
    nickname: Option<String>,
}

pub struct NicknameStore {
    path: PathBuf,
    state: NicknameDisk,
}

impl NicknameStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let path = path.into();
        let state = if path.exists() {
            serde_json::from_slice(&fs::read(&path)?)?
        } else {
            NicknameDisk::default()
        };
        Ok(Self { path, state })
    }

    pub fn default_path() -> PathBuf {
        default_config_dir().join("nickname.json")
    }

    fn persist(&self) -> Result<(), StoreError> {
        ensure_parent(&self.path)?;
        fs::write(&self.path, serde_json::to_vec_pretty(&self.state)?)?;
        Ok(())
    }
}

impl LocalNicknameStoring for NicknameStore {
    fn load(&self) -> Option<String> {
        self.state.nickname.clone()
    }

    fn save(&mut self, nickname: &str) {
        let trimmed = nickname.trim();
        if trimmed.is_empty() {
            self.clear();
            return;
        }
        self.state.nickname = Some(trimmed.to_string());
        let _ = self.persist();
    }

    fn clear(&mut self) {
        self.state.nickname = None;
        let _ = self.persist();
    }
}

#[derive(Default)]
pub struct InMemoryNicknameStore {
    value: Option<String>,
}

impl LocalNicknameStoring for InMemoryNicknameStore {
    fn load(&self) -> Option<String> {
        self.value.clone()
    }

    fn save(&mut self, nickname: &str) {
        let trimmed = nickname.trim();
        if trimmed.is_empty() {
            self.value = None;
        } else {
            self.value = Some(trimmed.to_string());
        }
    }

    fn clear(&mut self) {
        self.value = None;
    }
}
