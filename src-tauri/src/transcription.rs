//! Whisper transcription via the official whisper.cpp CLI binary.
//!
//! Instead of compiling whisper.cpp at build-time (which has bindgen/CMake
//! issues on MSVC Windows), we invoke the pre-built `whisper-cli.exe` from the
//! official whisper.cpp GitHub release as a subprocess.
//!
//! Flow: f32 samples → temp WAV (16 kHz mono float32) → whisper-cli subprocess
//! → parse stdout → clean text.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::config::Language;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn whisper_cli_command(cli_path: &PathBuf) -> std::process::Command {
    let mut cmd = std::process::Command::new(cli_path);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub struct TranscriptionService {
    inner: Mutex<Option<LoadedModel>>,
}

struct LoadedModel {
    model_path: PathBuf,
    cli_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: String,
}

impl TranscriptionService {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Store the model path and resolve the CLI binary path.
    /// The CLI binary (`whisper-cli.exe`) is expected to live in the same
    /// directory as the model file (both downloaded by `ModelManager`).
    pub fn ensure_loaded(&self, model_path: PathBuf) -> Result<()> {
        let cli_path = model_path
            .parent()
            .ok_or_else(|| anyhow!("model path has no parent directory"))?
            .join("whisper-cli.exe");

        if !cli_path.exists() {
            return Err(anyhow!(
                "whisper-cli.exe não encontrado em {:?}. \
                 Baixe o modelo nas Configurações (o binário é incluído automaticamente).",
                cli_path.parent().unwrap()
            ));
        }

        *self.inner.lock() = Some(LoadedModel { model_path, cli_path });
        Ok(())
    }

    pub fn transcribe(
        &self,
        samples: &[f32],
        language: &Language,
    ) -> Result<TranscriptionResult> {
        let (model_path, cli_path) = {
            let guard = self.inner.lock();
            let loaded = guard
                .as_ref()
                .ok_or_else(|| anyhow!("transcription model not loaded"))?;
            (loaded.model_path.clone(), loaded.cli_path.clone())
        };

        // Write audio to a temporary WAV file.
        let wav_path = std::env::temp_dir().join("nwt-audio-in.wav");
        write_wav(&wav_path, samples).context("escrevendo arquivo WAV temporário")?;

        let lang_arg = match language {
            Language::Auto => "auto",
            Language::Pt => "pt",
            Language::En => "en",
        };

        let threads = num_threads_for_inference().to_string();

        // Call whisper-cli.exe synchronously.
        // -np   = no prints (suppress progress to stderr)
        // -nt   = no timestamps in stdout output
        // --no-gpu = CPU-only (avoids GPU driver issues)
        let output = whisper_cli_command(&cli_path)
            .args([
                "-m",
                model_path.to_str().unwrap_or_default(),
                "-f",
                wav_path.to_str().unwrap_or_default(),
                "-l",
                lang_arg,
                "-t",
                &threads,
                "-np",
                "-nt",
                "--no-gpu",
            ])
            .output()
            .with_context(|| format!("executando {:?}", cli_path))?;

        // Clean up temp file (best-effort).
        let _ = std::fs::remove_file(&wav_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("whisper-cli falhou ({}): {}", output.status, stderr));
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = parse_output(&raw);

        Ok(TranscriptionResult {
            text,
            language: match language {
                Language::Auto => "auto".into(),
                Language::Pt => "pt".into(),
                Language::En => "en".into(),
            },
        })
    }
}

/// Write mono f32 samples at 16 kHz to a WAV file.
fn write_wav(path: &std::path::Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("criando WAV em {:?}", path))?;
    for &s in samples {
        writer.write_sample(s).context("escrevendo amostra WAV")?;
    }
    writer.finalize().context("finalizando WAV")?;
    Ok(())
}

/// Strip timestamp prefixes like `[00:00:00.000 --> 00:00:04.260]` and
/// concatenate non-empty lines into a single string.
fn parse_output(raw: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Strip leading "[HH:MM:SS.mmm --> HH:MM:SS.mmm]" if present.
        let content = if line.starts_with('[') {
            if let Some(close) = line.find(']') {
                line[close + 1..].trim()
            } else {
                line
            }
        } else {
            line
        };
        if !content.is_empty() {
            parts.push(content);
        }
    }
    parts.join(" ").trim().to_string()
}

fn num_threads_for_inference() -> usize {
    let total = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    (total / 2).clamp(2, 8)
}

