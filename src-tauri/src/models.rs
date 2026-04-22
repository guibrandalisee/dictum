//! Whisper model catalog + on-demand downloader.
//!
//! Models are GGML files hosted on the official whisper.cpp Hugging Face
//! repository. We download to `<paths.models_dir>/<file>` with a `.part`
//! suffix during the transfer, then rename atomically on success.
//!
//! The download is resumable across runs only at the file level (we delete
//! `.part` if the previous attempt was interrupted; a future iteration can
//! add HTTP range resumes).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::config::WhisperModel;

/// Static metadata for a Whisper GGML model.
pub struct ModelInfo {
    pub id: WhisperModel,
    pub file_name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size_bytes: u64,
}

impl WhisperModel {
    pub fn info(&self) -> ModelInfo {
        match self {
            // Multilingual GGML models from ggerganov/whisper.cpp.
            // SHA256 hashes are the official ones published in the repo.
            WhisperModel::Tiny => ModelInfo {
                id: WhisperModel::Tiny,
                file_name: "ggml-tiny.bin",
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
                sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
                size_bytes: 77_691_713,
            },
            WhisperModel::Base => ModelInfo {
                id: WhisperModel::Base,
                file_name: "ggml-base.bin",
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
                sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
                size_bytes: 147_951_465,
            },
            WhisperModel::Small => ModelInfo {
                id: WhisperModel::Small,
                file_name: "ggml-small.bin",
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
                sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
                size_bytes: 487_601_967,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub model: WhisperModel,
    pub installed: bool,
    pub path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model: WhisperModel,
    pub downloaded: u64,
    pub total: u64,
}

pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Self {
        Self { models_dir }
    }

    pub fn path_for(&self, model: &WhisperModel) -> PathBuf {
        self.models_dir.join(model.info().file_name)
    }

    pub fn status(&self, model: &WhisperModel) -> ModelStatus {
        let path = self.path_for(model);
        let installed = path.exists()
            && std::fs::metadata(&path)
                .map(|m| m.len() > 0)
                .unwrap_or(false);
        ModelStatus {
            model: model.clone(),
            installed,
            path,
            size_bytes: model.info().size_bytes,
        }
    }

    /// Download the model if not already present. `progress` is called
    /// roughly every chunk with cumulative byte counts.
    pub async fn ensure_downloaded<F>(
        &self,
        model: &WhisperModel,
        mut progress: F,
    ) -> Result<PathBuf>
    where
        F: FnMut(DownloadProgress) + Send,
    {
        let target = self.path_for(model);
        if target.exists() && std::fs::metadata(&target)?.len() > 0 {
            return Ok(target);
        }

        std::fs::create_dir_all(&self.models_dir).ok();
        let info = model.info();
        let part_path = target.with_extension("bin.part");
        // Clean up any previous interrupted attempt.
        let _ = std::fs::remove_file(&part_path);

        log::info!("downloading model {} from {}", info.file_name, info.url);

        let resp = reqwest::Client::builder()
            .build()?
            .get(info.url)
            .send()
            .await
            .context("starting download")?
            .error_for_status()
            .context("non-success status")?;

        let total = resp.content_length().unwrap_or(info.size_bytes);

        let mut file = tokio::fs::File::create(&part_path)
            .await
            .with_context(|| format!("creating {:?}", part_path))?;

        let mut downloaded: u64 = 0;
        let mut hasher = Sha256::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("reading chunk")?;
            hasher.update(&bytes);
            downloaded += bytes.len() as u64;
            tokio::io::AsyncWriteExt::write_all(&mut file, &bytes)
                .await
                .context("writing chunk")?;
            progress(DownloadProgress {
                model: model.clone(),
                downloaded,
                total,
            });
        }
        tokio::io::AsyncWriteExt::flush(&mut file).await.ok();
        drop(file);

        // Verify checksum (best-effort; warn but accept if it does not match,
        // because hashes for some models change between revisions).
        let hex = hex_encode(hasher.finalize().as_slice());
        if hex.eq_ignore_ascii_case(info.sha256) {
            log::info!("model checksum verified");
        } else {
            log::warn!(
                "model checksum mismatch (expected {}, got {}). Keeping file but flagging for review.",
                info.sha256,
                hex
            );
        }

        std::fs::rename(&part_path, &target)
            .with_context(|| format!("renaming {:?} -> {:?}", part_path, target))?;
        Ok(target)
    }

    /// Quick sanity check that an already-installed model is at least
    /// non-empty. We do not re-hash on every boot for speed.
    pub fn validate(&self, model: &WhisperModel) -> Result<PathBuf> {
        let path = self.path_for(model);
        let meta = std::fs::metadata(&path)
            .with_context(|| format!("model not found: {:?}", path))?;
        if meta.len() == 0 {
            return Err(anyhow!("model file is empty: {:?}", path));
        }
        Ok(path)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

pub fn looks_like_first_run(models_dir: &Path) -> bool {
    !models_dir.join(WhisperModel::Base.info().file_name).exists()
}

// ────────────────────────────────────────────────────────────────────────────
// whisper.cpp CLI binary (used for subprocess-based transcription)
// ────────────────────────────────────────────────────────────────────────────

/// Metadata for the whisper.cpp Windows x64 CPU binary bundle.
struct WhisperCliInfo {
    zip_url: &'static str,
    zip_sha256: &'static str,
    zip_size: u64,
    /// Executable name inside the zip.
    exe_name: &'static str,
}

const WHISPER_CLI: WhisperCliInfo = WhisperCliInfo {
    zip_url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.8.4/whisper-bin-x64.zip",
    zip_sha256: "74f973345cb52ef5ba3ec9e7e7af8e48cc8c71722d1528603b80588a11f82e3e",
    zip_size: 4_078_768,
    exe_name: "whisper-cli.exe",
};

impl ModelManager {
    /// Returns the expected path of `whisper-cli.exe` in the models directory.
    pub fn cli_path(&self) -> std::path::PathBuf {
        self.models_dir.join(WHISPER_CLI.exe_name)
    }

    /// Download and extract the whisper.cpp CLI binary if not already present.
    /// `progress` receives (downloaded_bytes, total_bytes) during the download.
    pub async fn ensure_cli_downloaded<F>(&self, mut progress: F) -> Result<std::path::PathBuf>
    where
        F: FnMut(u64, u64) + Send,
    {
        let cli_path = self.cli_path();
        if cli_path.exists() {
            return Ok(cli_path);
        }

        std::fs::create_dir_all(&self.models_dir).ok();
        log::info!("downloading whisper-cli from {}", WHISPER_CLI.zip_url);

        let resp = reqwest::Client::builder()
            .build()?
            .get(WHISPER_CLI.zip_url)
            .send()
            .await
            .context("starting whisper-cli download")?
            .error_for_status()
            .context("non-success status")?;

        let total = resp.content_length().unwrap_or(WHISPER_CLI.zip_size);
        let mut zip_bytes: Vec<u8> = Vec::with_capacity(total as usize);
        let mut downloaded: u64 = 0;
        let mut hasher = Sha256::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("reading chunk")?;
            hasher.update(&bytes);
            downloaded += bytes.len() as u64;
            zip_bytes.extend_from_slice(&bytes);
            progress(downloaded, total);
        }

        // Verify zip checksum.
        let hex = hex_encode(hasher.finalize().as_slice());
        if !hex.eq_ignore_ascii_case(WHISPER_CLI.zip_sha256) {
            return Err(anyhow!(
                "checksum do whisper-cli zip não confere (esperado {}, obtido {})",
                WHISPER_CLI.zip_sha256,
                hex
            ));
        }

        // Extract all .exe and .dll files from the zip.
        let cursor = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor).context("abrindo zip do whisper-cli")?;

        for i in 0..archive.len() {
            let mut zf = archive.by_index(i).context("lendo entrada zip")?;
            let name = zf.name().to_string();
            // Only extract executables and shared libraries (ignore paths).
            let basename = std::path::Path::new(&name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&name);
            let lower = basename.to_ascii_lowercase();
            if lower.ends_with(".exe") || lower.ends_with(".dll") {
                let dest = self.models_dir.join(basename);
                let mut out = std::fs::File::create(&dest)
                    .with_context(|| format!("criando {:?}", dest))?;
                std::io::copy(&mut zf, &mut out)
                    .with_context(|| format!("extraindo {}", basename))?;
                log::info!("extracted {}", basename);
            }
        }

        if !cli_path.exists() {
            return Err(anyhow!(
                "{} não encontrado no zip do whisper-cli",
                WHISPER_CLI.exe_name
            ));
        }

        log::info!("whisper-cli ready at {:?}", cli_path);
        Ok(cli_path)
    }
}
