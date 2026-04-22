//! Shared application state injected into Tauri commands via `State<AppState>`.

use anyhow::Result;

use crate::config::{AppConfig, ConfigStore};
use crate::history::HistoryStore;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub config: ConfigStore,
    pub history: HistoryStore,
}

impl AppState {
    pub fn new(paths: AppPaths, config: ConfigStore, history: HistoryStore) -> Self {
        Self {
            paths,
            config,
            history,
        }
    }

    pub fn update_config(&self, new_cfg: AppConfig) -> Result<AppConfig> {
        self.config.replace(new_cfg)
    }
}
