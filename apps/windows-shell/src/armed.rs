use crate::credentials::{default_config_dir, ensure_parent, StoreError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Durable Armed/Paused + Quit auto-start opt-out (ADR-0006).
pub trait ArmedStateStoring {
    fn clear_quit_opt_out(&mut self);
    fn is_armed(&self) -> bool;
    fn quit_opted_out(&self) -> bool;
    fn set_armed(&mut self, armed: bool);
    fn set_quit_opted_out(&mut self, opted_out: bool);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ArmedDisk {
    armed: bool,
    quit_opted_out: bool,
}

impl Default for ArmedDisk {
    fn default() -> Self {
        Self {
            armed: true,
            quit_opted_out: false,
        }
    }
}

pub struct ArmedStateStore {
    path: PathBuf,
    state: ArmedDisk,
}

impl ArmedStateStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let path = path.into();
        let state = if path.exists() {
            serde_json::from_slice(&fs::read(&path)?)?
        } else {
            ArmedDisk::default()
        };
        Ok(Self { path, state })
    }

    pub fn default_path() -> PathBuf {
        default_config_dir().join("armed.json")
    }

    fn persist(&self) -> Result<(), StoreError> {
        ensure_parent(&self.path)?;
        fs::write(&self.path, serde_json::to_vec_pretty(&self.state)?)?;
        Ok(())
    }
}

impl ArmedStateStoring for ArmedStateStore {
    fn is_armed(&self) -> bool {
        self.state.armed
    }

    fn set_armed(&mut self, armed: bool) {
        self.state.armed = armed;
        if let Err(err) = self.persist() {
            eprintln!("sync-clip: failed to persist Armed state: {err}");
        }
    }

    fn quit_opted_out(&self) -> bool {
        self.state.quit_opted_out
    }

    fn set_quit_opted_out(&mut self, opted_out: bool) {
        self.state.quit_opted_out = opted_out;
        if let Err(err) = self.persist() {
            eprintln!("sync-clip: failed to persist Quit opt-out: {err}");
        }
    }

    fn clear_quit_opt_out(&mut self) {
        self.set_quit_opted_out(false);
    }
}

/// In-memory Armed store for tests.
#[derive(Debug)]
pub struct InMemoryArmedStateStore {
    armed: bool,
    quit_opted_out: bool,
}

impl Default for InMemoryArmedStateStore {
    fn default() -> Self {
        Self {
            armed: true,
            quit_opted_out: false,
        }
    }
}

impl ArmedStateStoring for InMemoryArmedStateStore {
    fn is_armed(&self) -> bool {
        self.armed
    }

    fn set_armed(&mut self, armed: bool) {
        self.armed = armed;
    }

    fn quit_opted_out(&self) -> bool {
        self.quit_opted_out
    }

    fn set_quit_opted_out(&mut self, opted_out: bool) {
        self.quit_opted_out = opted_out;
    }

    fn clear_quit_opt_out(&mut self) {
        self.quit_opted_out = false;
    }
}
