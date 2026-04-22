//! Resolves where the app stores config, models, history and logs.
//!
//! Dictum now runs in **always-portable mode**:
//! all files live next to the executable, inside `./data/`.
//! If the executable directory is not writable, startup fails with an
//! explicit error.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

const PORTABLE_SUBDIR: &str = "data";

#[derive(Debug, Clone, Serialize)]
pub struct AppPaths {
    pub root: PathBuf,
    pub config_file: PathBuf,
    pub models_dir: PathBuf,
    pub history_file: PathBuf,
    pub logs_dir: PathBuf,
    pub portable: bool,
}

impl AppPaths {
    fn new(root: PathBuf, portable: bool) -> Self {
        Self {
            config_file: root.join("config.json"),
            models_dir: root.join("models"),
            history_file: root.join("history.jsonl"),
            logs_dir: root.join("logs"),
            root,
            portable,
        }
    }
}

pub fn resolve() -> Result<AppPaths> {
    let exe_dir = current_exe_dir()?;
    let root = exe_dir.join(PORTABLE_SUBDIR);
    ensure_dir(&root)?;
    Ok(AppPaths::new(root, true))
}

fn current_exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("could not get current exe path")?;
    Ok(exe
        .parent()
        .ok_or_else(|| anyhow!("exe has no parent directory"))?
        .to_path_buf())
}

fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("creating directory {:?}", path))?;
    Ok(())
}
