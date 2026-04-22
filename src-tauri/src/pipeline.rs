//! End-to-end push-to-talk pipeline.
//!
//! Orchestrates: hotkey press → start audio → hotkey release →
//! stop audio → transcribe → paste → record history.
//!
//! Emits Tauri events so the GUI can render real-time status:
//!   - `pipeline://state` with payload `{ "state": "idle" | "recording" | "processing" | "done" | "error", "message": "?" }`
//!   - `pipeline://transcript` with the final text

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::AudioCaptureService;
use crate::config::AppConfig;
use crate::history::HistoryEntry;
use crate::models::ModelManager;
use crate::paste::{PasteOutcome, PasteService};
use crate::state::AppState;
use crate::transcription::TranscriptionService;

pub const EVT_STATE: &str = "pipeline://state";
pub const EVT_TRANSCRIPT: &str = "pipeline://transcript";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum PipelineState {
    Idle,
    Recording,
    Processing,
    Done {
        text: String,
        language: String,
        pasted: bool,
        recording_ms: u64,
        processing_ms: u64,
    },
    Error { message: String },
}

#[derive(Clone)]
pub struct Pipeline {
    audio: AudioCaptureService,
    transcription: Arc<TranscriptionService>,
    busy: Arc<Mutex<bool>>,
    /// True while the hotkey is currently held down. Used to ignore
    /// duplicate press events from key repeat.
    pressed: Arc<Mutex<bool>>,
    paused: Arc<Mutex<bool>>,
}

impl Pipeline {
    pub fn new(audio: AudioCaptureService, transcription: Arc<TranscriptionService>) -> Self {
        Self {
            audio,
            transcription,
            busy: Arc::new(Mutex::new(false)),
            pressed: Arc::new(Mutex::new(false)),
            paused: Arc::new(Mutex::new(false)),
        }
    }

    pub fn set_paused(&self, paused: bool) {
        *self.paused.lock() = paused;
    }

    pub fn is_paused(&self) -> bool {
        *self.paused.lock()
    }

    pub fn on_hotkey_press(&self, app: &AppHandle) {
        if self.is_paused() {
            return;
        }
        let mut pressed = self.pressed.lock();
        if *pressed {
            return; // ignore key repeat
        }
        *pressed = true;
        drop(pressed);

        if *self.busy.lock() {
            log::debug!("ignoring press while busy");
            return;
        }

        let cfg = read_config(app);
        let max = Duration::from_secs(cfg.max_recording_seconds.max(5) as u64);

        match self.audio.start(cfg.microphone.clone(), max) {
            Ok(()) => {
                emit_state(app, PipelineState::Recording);
            }
            Err(e) => {
                log::warn!("could not start recording: {e}");
                emit_state(
                    app,
                    PipelineState::Error {
                        message: format!("Falha ao iniciar gravação: {e}"),
                    },
                );
                *self.pressed.lock() = false;
            }
        }
    }

    pub fn on_hotkey_release(&self, app: &AppHandle) {
        let was_pressed = {
            let mut p = self.pressed.lock();
            let prev = *p;
            *p = false;
            prev
        };
        if !was_pressed || self.is_paused() {
            return;
        }

        if *self.busy.lock() {
            return;
        }
        *self.busy.lock() = true;

        let app = app.clone();
        let pipeline = self.clone();
        tauri::async_runtime::spawn(async move {
            pipeline.run_release(app).await;
        });
    }

    async fn run_release(self, app: AppHandle) {
        let result = self.do_release(app.clone()).await;
        if let Err(e) = result {
            log::warn!("pipeline error: {e}");
            emit_state(
                &app,
                PipelineState::Error {
                    message: e.to_string(),
                },
            );
        }
        *self.busy.lock() = false;
    }

    async fn do_release(&self, app: AppHandle) -> anyhow::Result<()> {
        emit_state(&app, PipelineState::Processing);
        let processing_start = Instant::now();

        let recording = self.audio.stop()?;
        let duration_ms = recording.duration.as_millis() as u64;

        // Reject ridiculously short clips outright (likely accidental tap).
        if recording.samples.len() < 1600 {
            emit_state(
                &app,
                PipelineState::Error {
                    message: "Gravação muito curta — fale por pelo menos 0,5s.".into(),
                },
            );
            return Ok(());
        }

        let cfg = read_config(&app);

        // Make sure the model is loaded.
        let paths = {
            let state: tauri::State<'_, AppState> = app.state();
            state.paths.clone()
        };
        let manager = ModelManager::new(paths.models_dir.clone());

        // Ensure the whisper-cli binary is present (download if first use).
        manager
            .ensure_cli_downloaded(|_downloaded, _total| {})
            .await
            .map_err(|e| anyhow::anyhow!("falha ao baixar whisper-cli: {e}"))?;

        let model_path = manager
            .validate(&cfg.model)
            .map_err(|e| anyhow::anyhow!("modelo não disponível: {e}. Baixe-o nas configurações."))?;
        self.transcription.ensure_loaded(model_path)?;

        // Transcription is CPU-bound — run in blocking pool.
        let transcription = self.transcription.clone();
        let samples = recording.samples;
        let lang = cfg.language.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            transcription.transcribe(&samples, &lang)
        })
        .await
        .map_err(|e| anyhow::anyhow!("inferência abortada: {e}"))??;

        if result.text.is_empty() {
            emit_state(
                &app,
                PipelineState::Error {
                    message: "Nada foi transcrito (silêncio?).".into(),
                },
            );
            return Ok(());
        }

        // Paste — runs in blocking pool because it touches the clipboard
        // and synthesizes input events.
        let text_for_paste = result.text.clone();
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            PasteService::deliver(&text_for_paste)
        })
        .await
        .map_err(|e| anyhow::anyhow!("paste task aborted: {e}"))??;
        let pasted = matches!(outcome, PasteOutcome::Pasted);

        // History (only if enabled).
        if cfg.keep_history {
            let entry = HistoryEntry {
                timestamp: Utc::now(),
                text: result.text.clone(),
                language: Some(result.language.clone()),
                duration_ms: Some(duration_ms),
            };
            let state: tauri::State<'_, AppState> = app.state();
            if let Err(e) = state.history.append(&entry) {
                log::warn!("could not append history: {e}");
            }
        }

        let _ = app.emit(EVT_TRANSCRIPT, &result.text);
        let processing_ms = processing_start.elapsed().as_millis() as u64;
        emit_state(
            &app,
            PipelineState::Done {
                text: result.text,
                language: result.language,
                pasted,
                recording_ms: duration_ms,
                processing_ms,
            },
        );
        Ok(())
    }
}

fn read_config(app: &AppHandle) -> AppConfig {
    let state: tauri::State<'_, AppState> = app.state();
    let cfg = state.config.read().clone();
    cfg
}

fn emit_state(app: &AppHandle, state: PipelineState) {
    if let Err(e) = app.emit(EVT_STATE, &state) {
        log::warn!("failed to emit pipeline state: {e}");
    }
}
