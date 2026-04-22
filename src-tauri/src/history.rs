//! Local transcription history persisted as a JSONL file.
//!
//! One JSON object per line keeps appends cheap and crash-safe; we never
//! rewrite previous entries. `clear()` truncates the file. The MVP stores
//! only timestamp and text — no audio.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: Option<u64>,
}

pub struct HistoryStore {
    file: PathBuf,
    write_lock: Mutex<()>,
}

impl HistoryStore {
    pub fn open(file: &Path) -> Result<Self> {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if !file.exists() {
            std::fs::File::create(file)
                .with_context(|| format!("creating history file {:?}", file))?;
        }
        Ok(Self {
            file: file.to_path_buf(),
            write_lock: Mutex::new(()),
        })
    }

    pub fn append(&self, entry: &HistoryEntry) -> Result<()> {
        let _guard = self.write_lock.lock();
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
            .with_context(|| format!("opening history file {:?}", self.file))?;
        let line = serde_json::to_string(entry)?;
        writeln!(f, "{line}")?;
        Ok(())
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let f = std::fs::File::open(&self.file)
            .with_context(|| format!("opening history file {:?}", self.file))?;
        let reader = BufReader::new(f);
        let mut all: Vec<HistoryEntry> = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<HistoryEntry>(&line) {
                Ok(e) => all.push(e),
                Err(e) => log::warn!("skipping corrupt history line: {e}"),
            }
        }
        let start = all.len().saturating_sub(limit);
        Ok(all.split_off(start))
    }

    pub fn clear(&self) -> Result<()> {
        let _guard = self.write_lock.lock();
        std::fs::write(&self.file, b"")
            .with_context(|| format!("truncating history file {:?}", self.file))?;
        Ok(())
    }

    /// Delete the first history entry that matches the provided payload.
    ///
    /// Returns `true` when an entry was removed, `false` otherwise.
    pub fn delete_one(&self, target: &HistoryEntry) -> Result<bool> {
        let _guard = self.write_lock.lock();

        let f = std::fs::File::open(&self.file)
            .with_context(|| format!("opening history file {:?}", self.file))?;
        let reader = BufReader::new(f);

        let mut removed = false;
        let mut kept_lines: Vec<String> = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if !removed {
                match serde_json::from_str::<HistoryEntry>(&line) {
                    Ok(entry) if entries_match(&entry, target) => {
                        removed = true;
                        continue;
                    }
                    Ok(_) => {}
                    Err(_) => {
                        // Preserve unparseable lines instead of dropping data.
                    }
                }
            }

            kept_lines.push(line);
        }

        let mut out = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file)
            .with_context(|| format!("rewriting history file {:?}", self.file))?;

        for line in kept_lines {
            writeln!(out, "{line}")?;
        }

        Ok(removed)
    }
}

fn entries_match(a: &HistoryEntry, b: &HistoryEntry) -> bool {
    a.timestamp == b.timestamp
        && a.text == b.text
        && a.language == b.language
        && a.duration_ms == b.duration_ms
}
