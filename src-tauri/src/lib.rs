//! Dictum — push-to-talk dictation backend.

mod audio;
mod autostart;
mod config;
mod history;
mod hotkey;
mod models;
mod paste;
mod paths;
mod pipeline;
mod state;
mod transcription;
mod tray;

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::audio::AudioCaptureService;
use crate::hotkey::HotkeyService;
use crate::pipeline::Pipeline;
use crate::state::AppState;
use crate::transcription::TranscriptionService;

pub fn run() {
    let hotkey_service = HotkeyService::new();
    let audio_service = AudioCaptureService::spawn();
    let transcription_service = Arc::new(TranscriptionService::new());
    let pipeline = Arc::new(Pipeline::new(audio_service.clone(), transcription_service.clone()));

    // Build persistent stores before spinning up windows so commands never run
    // before `AppState` is managed.
    let paths = paths::resolve().expect("failed to resolve app paths");
    std::fs::create_dir_all(&paths.root).ok();
    std::fs::create_dir_all(&paths.models_dir).ok();
    std::fs::create_dir_all(&paths.logs_dir).ok();
    let config_store =
        config::ConfigStore::load_or_default(&paths.config_file)
            .expect("failed to load config store");
    let history_store =
        history::HistoryStore::open(&paths.history_file)
            .expect("failed to open history store");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Dictum")
                .args(["--minimized"])
                .build(),
        )
        .plugin(hotkey::plugin())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Webview),
                ])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .manage(hotkey_service.clone())
        .manage(audio_service.clone())
        .manage(transcription_service.clone())
        .manage(pipeline.clone())
        .manage(AppState::new(paths, config_store, history_store))
        .setup(move |app| {
            let handle = app.handle().clone();
            let state: tauri::State<'_, AppState> = app.state();

            log::info!(
                "Booting Dictum ({} mode) at {:?}",
                if state.paths.portable { "portable" } else { "localappdata" },
                state.paths.root
            );

            // Sync auto-start preference with the OS on startup.
            let auto_start_pref = state.config.read().auto_start;
            if let Err(e) = autostart::apply(&handle, auto_start_pref) {
                log::warn!("could not apply auto-start preference: {e}");
            }

            // Register hotkey from config.
            // If config contains a modifier-only combo (e.g. Ctrl+Alt),
            // migrate to a Windows-safe fallback and persist it.
            let mut hotkey_str = state.config.read().hotkey.clone();
            if hotkey::is_modifier_only(&hotkey_str) {
                log::warn!(
                    "configured hotkey \"{}\" is modifier-only; migrating to fallback {}",
                    hotkey_str,
                    hotkey::FALLBACK_HOTKEY
                );
                hotkey_str = hotkey::FALLBACK_HOTKEY.to_string();
                let mut cfg = state.config.read().clone();
                cfg.hotkey = hotkey_str.clone();
                if let Err(e) = state.config.replace(cfg) {
                    log::warn!("failed to persist migrated fallback hotkey: {e}");
                }
            }

            if let Err(e) = hotkey_service.register(&handle, &hotkey_str) {
                log::warn!("could not register hotkey on startup: {e}");

                // Last-resort fallback registration.
                if hotkey_str != hotkey::FALLBACK_HOTKEY {
                    let fallback = hotkey::FALLBACK_HOTKEY.to_string();
                    match hotkey_service.register(&handle, &fallback) {
                        Ok(()) => {
                            log::warn!(
                                "registered fallback hotkey {} after startup failure",
                                fallback
                            );
                            let mut cfg = state.config.read().clone();
                            cfg.hotkey = fallback;
                            if let Err(e) = state.config.replace(cfg) {
                                log::warn!("failed to persist fallback hotkey: {e}");
                            }
                        }
                        Err(e2) => {
                            log::error!(
                                "fallback hotkey {} also failed: {e2}",
                                hotkey::FALLBACK_HOTKEY
                            );
                        }
                    }
                }
            }

            // Build tray icon.
            if let Err(e) = tray::build(&handle) {
                log::error!("failed to build tray icon: {e}");
            }

            // If launched with --minimized (e.g. by autostart), hide the window.
            let argv: Vec<String> = std::env::args().collect();
            if argv.iter().any(|a| a == "--minimized") {
                if let Some(win) = handle.get_webview_window("main") {
                    let _ = win.hide();
                }
            }

            // Hide window to tray on close instead of exiting.
            if let Some(win) = handle.get_webview_window("main") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_paths,
            commands::get_config,
            commands::update_config,
            commands::get_history,
            commands::delete_history_entry,
            commands::clear_history,
            commands::set_hotkey,
            commands::list_microphones,
            commands::get_model_status,
            commands::download_model,
            commands::toggle_pause,
            commands::is_paused,
            commands::test_recording,
            commands::set_auto_start,
            commands::show_window,
            commands::hide_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod commands {
    use std::sync::Arc;
    use std::time::Duration;

    use tauri::{AppHandle, Emitter, Manager, State};

    use crate::audio::AudioCaptureService;
    use crate::autostart;
    use crate::config::AppConfig;
    use crate::history::HistoryEntry;
    use crate::hotkey::HotkeyService;
    use crate::models::{ModelManager, ModelStatus};
    use crate::paths::AppPaths;
    use crate::pipeline::Pipeline;
    use crate::state::AppState;
    use crate::transcription::TranscriptionService;

    #[tauri::command]
    pub fn get_app_paths(state: State<'_, AppState>) -> AppPaths {
        state.paths.clone()
    }

    #[tauri::command]
    pub fn get_config(state: State<'_, AppState>) -> AppConfig {
        state.config.read().clone()
    }

    #[tauri::command]
    pub fn update_config(
        app: AppHandle,
        state: State<'_, AppState>,
        hotkey: State<'_, Arc<HotkeyService>>,
        config: AppConfig,
    ) -> Result<AppConfig, String> {
        let prev = state.config.read().clone();
        let new_cfg = config;

        if new_cfg.hotkey != prev.hotkey {
            if crate::hotkey::is_modifier_only(&new_cfg.hotkey) {
                return Err(
                    "Atalho inválido: use ao menos uma tecla principal (ex.: Ctrl+Alt+Space)."
                        .to_string(),
                );
            }
            hotkey
                .register(&app, &new_cfg.hotkey)
                .map_err(|e| e.to_string())?;
        }
        if new_cfg.auto_start != prev.auto_start {
            autostart::apply(&app, new_cfg.auto_start).map_err(|e| e.to_string())?;
        }

        state.update_config(new_cfg).map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn set_hotkey(
        app: AppHandle,
        state: State<'_, AppState>,
        hotkey_service: State<'_, Arc<HotkeyService>>,
        accelerator: String,
    ) -> Result<AppConfig, String> {
        if crate::hotkey::is_modifier_only(&accelerator) {
            return Err(
                "Atalho inválido: use ao menos uma tecla principal (ex.: Ctrl+Alt+Space)."
                    .to_string(),
            );
        }

        hotkey_service
            .register(&app, &accelerator)
            .map_err(|e| e.to_string())?;

        let mut cfg = state.config.read().clone();
        cfg.hotkey = accelerator;
        state.update_config(cfg).map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn set_auto_start(
        app: AppHandle,
        state: State<'_, AppState>,
        enabled: bool,
    ) -> Result<AppConfig, String> {
        autostart::apply(&app, enabled).map_err(|e| e.to_string())?;
        let mut cfg = state.config.read().clone();
        cfg.auto_start = enabled;
        state.update_config(cfg).map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn get_history(
        state: State<'_, AppState>,
        limit: Option<usize>,
    ) -> Result<Vec<HistoryEntry>, String> {
        state
            .history
            .recent(limit.unwrap_or(100))
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn delete_history_entry(
        state: State<'_, AppState>,
        entry: HistoryEntry,
    ) -> Result<bool, String> {
        state
            .history
            .delete_one(&entry)
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
        state.history.clear().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn show_window(app: AppHandle) {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
        }
    }

    #[tauri::command]
    pub fn hide_window(app: AppHandle) {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
        }
    }

    #[tauri::command]
    pub fn list_microphones(audio: State<'_, AudioCaptureService>) -> Result<Vec<String>, String> {
        audio.list_devices().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn get_model_status(state: State<'_, AppState>) -> ModelStatus {
        let cfg = state.config.read().clone();
        let manager = ModelManager::new(state.paths.models_dir.clone());
        manager.status(&cfg.model)
    }

    /// Download (or re-download) the active model. Emits
    /// `model://download` events with `{ downloaded, total }` payloads.
    #[tauri::command]
    pub async fn download_model(app: AppHandle) -> Result<ModelStatus, String> {
        let (model, models_dir) = {
            let state: State<'_, AppState> = app.state();
            let model = state.config.read().model.clone();
            let models_dir = state.paths.models_dir.clone();
            (model, models_dir)
        };
        let manager = ModelManager::new(models_dir);

        let app_for_progress = app.clone();
        let model_for_progress = model.clone();
        manager
            .ensure_downloaded(&model, move |p| {
                let _ = app_for_progress.emit("model://download", &p);
            })
            .await
            .map_err(|e| e.to_string())?;

        // Try to warm up the context so the first transcription is faster.
        let path = manager.path_for(&model_for_progress);
        let transcription: State<'_, Arc<TranscriptionService>> = app.state();
        if let Err(e) = transcription.ensure_loaded(path) {
            log::warn!("could not warm up model after download: {e}");
        }

        Ok(manager.status(&model))
    }

    #[tauri::command]
    pub fn toggle_pause(pipeline: State<'_, Arc<Pipeline>>) -> bool {
        let new_state = !pipeline.is_paused();
        pipeline.set_paused(new_state);
        new_state
    }

    #[tauri::command]
    pub fn is_paused(pipeline: State<'_, Arc<Pipeline>>) -> bool {
        pipeline.is_paused()
    }

    /// Manual recording trigger for the GUI: records for `seconds` then
    /// transcribes and pastes, just like a hotkey hold would.
    #[tauri::command]
    pub async fn test_recording(
        app: AppHandle,
        seconds: u32,
    ) -> Result<(), String> {
        let pipeline: State<'_, Arc<Pipeline>> = app.state();
        let pipeline = pipeline.inner().clone();
        pipeline.on_hotkey_press(&app);
        let dur = Duration::from_secs(seconds.clamp(1, 30) as u64);
        tokio::time::sleep(dur).await;
        pipeline.on_hotkey_release(&app);
        Ok(())
    }
}
