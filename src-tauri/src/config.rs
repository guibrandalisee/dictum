//! Persistent app configuration.
//!
//! Stored as JSON at `paths.config_file`. Loaded once on startup and kept
//! behind a `RwLock` inside `AppState`. Updates from the GUI go through
//! `AppState::update_config`, which validates and persists atomically.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Auto,
    Pt,
    En,
}

impl Default for Language {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
}

impl Default for WhisperModel {
    fn default() -> Self {
        Self::Base
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Hotkey accelerator string in Tauri format, e.g. `"Ctrl+Alt"` or
    /// `"Ctrl+Shift+Space"`. Push-to-talk semantics: held = recording.
    pub hotkey: String,
    pub language: Language,
    pub model: WhisperModel,
    /// Optional input device name. `None` = system default microphone.
    pub microphone: Option<String>,
    pub auto_start: bool,
    pub keep_history: bool,
    /// Maximum duration of a single recording, in seconds. Safety net.
    pub max_recording_seconds: u32,
    /// Whether onboarding (first-run wizard) has been completed.
    pub onboarded: bool,
    /// Schema version, for future migrations.
    pub version: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt+Space".to_string(),
            language: Language::default(),
            model: WhisperModel::default(),
            microphone: None,
            auto_start: false,
            keep_history: true,
            max_recording_seconds: 60,
            onboarded: false,
            version: 1,
        }
    }
}

pub struct ConfigStore {
    file: PathBuf,
    inner: RwLock<AppConfig>,
}

impl ConfigStore {
    pub fn load_or_default(file: &Path) -> Result<Self> {
        let cfg = if file.exists() {
            let bytes = std::fs::read(file)
                .with_context(|| format!("reading config from {:?}", file))?;
            match serde_json::from_slice::<AppConfig>(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("config at {:?} is invalid ({e}); falling back to defaults", file);
                    AppConfig::default()
                }
            }
        } else {
            let cfg = AppConfig::default();
            persist(file, &cfg)?;
            cfg
        };

        Ok(Self {
            file: file.to_path_buf(),
            inner: RwLock::new(cfg),
        })
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, AppConfig> {
        self.inner.read()
    }

    pub fn replace(&self, new_cfg: AppConfig) -> Result<AppConfig> {
        persist(&self.file, &new_cfg)?;
        let mut guard = self.inner.write();
        *guard = new_cfg.clone();
        Ok(new_cfg)
    }
}

fn persist(file: &Path, cfg: &AppConfig) -> Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let tmp = file.with_extension("json.tmp");
    let serialized = serde_json::to_vec_pretty(cfg)?;
    std::fs::write(&tmp, &serialized)
        .with_context(|| format!("writing temp config {:?}", tmp))?;
    std::fs::rename(&tmp, file)
        .with_context(|| format!("renaming config to {:?}", file))?;
    Ok(())
}
