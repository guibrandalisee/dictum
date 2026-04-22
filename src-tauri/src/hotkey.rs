//! Global push-to-talk hotkey service.
//!
//! Wraps `tauri-plugin-global-shortcut` and turns press/release events
//! into high-level `HotkeyEvent::Press` / `HotkeyEvent::Release` callbacks.
//!
//! The accelerator string follows Tauri's format (e.g. `"Ctrl+Alt+Space"`).
//! On Windows, modifier-only combinations like `"Ctrl+Alt"` are not always
//! accepted by the underlying `RegisterHotKey` API; in that case the UI is
//! expected to surface the registration error and ask the user to pick a
//! different combination.

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{
    GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::pipeline::Pipeline;

/// Tauri event names emitted to the frontend.
pub const EVT_HOTKEY_PRESS: &str = "hotkey://press";
pub const EVT_HOTKEY_RELEASE: &str = "hotkey://release";
/// Safe default on Windows (includes a non-modifier key).
pub const FALLBACK_HOTKEY: &str = "Ctrl+Alt+Space";

pub struct HotkeyService {
    current: Mutex<Option<Shortcut>>,
}

/// Returns true when the accelerator only contains modifier keys
/// (e.g. `Ctrl+Alt`), which are often rejected by Windows global hotkeys.
pub fn is_modifier_only(accelerator: &str) -> bool {
    let mut has_any = false;
    for token in accelerator.split('+').map(|t| t.trim().to_ascii_lowercase()) {
        if token.is_empty() {
            continue;
        }
        has_any = true;
        let is_modifier = matches!(
            token.as_str(),
            "ctrl"
                | "control"
                | "alt"
                | "shift"
                | "super"
                | "meta"
                | "command"
                | "cmd"
        );
        if !is_modifier {
            return false;
        }
    }
    has_any
}

impl HotkeyService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            current: Mutex::new(None),
        })
    }

    /// Parse and register a new shortcut, replacing any previous one.
    pub fn register(&self, app: &AppHandle, accelerator: &str) -> Result<()> {
        let parsed = Shortcut::from_str(accelerator)
            .map_err(|e| anyhow!("invalid hotkey \"{accelerator}\": {e}"))?;

        let manager = app.global_shortcut();

        // Unregister the previous shortcut first, if any.
        let mut current = self.current.lock();
        if let Some(prev) = current.take() {
            let _ = manager.unregister(prev);
        }

        manager
            .register(parsed.clone())
            .map_err(|e| anyhow!("could not register hotkey \"{accelerator}\": {e}"))?;

        *current = Some(parsed);
        log::info!("registered hotkey {accelerator}");
        Ok(())
    }

    pub fn unregister(&self, app: &AppHandle) {
        let mut current = self.current.lock();
        if let Some(prev) = current.take() {
            let _ = app.global_shortcut().unregister(prev);
        }
    }
}

/// Build the Tauri plugin with the press/release handler wired in.
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(handle_event)
        .build()
}

fn handle_event(app: &AppHandle, _shortcut: &Shortcut, event: ShortcutEvent) {
    match event.state() {
        ShortcutState::Pressed => {
            log::debug!("hotkey pressed");
            if let Err(e) = app.emit(EVT_HOTKEY_PRESS, ()) {
                log::warn!("failed to emit press event: {e}");
            }
            if let Some(pipeline) = app.try_state::<Arc<Pipeline>>() {
                pipeline.on_hotkey_press(app);
            }
        }
        ShortcutState::Released => {
            log::debug!("hotkey released");
            if let Err(e) = app.emit(EVT_HOTKEY_RELEASE, ()) {
                log::warn!("failed to emit release event: {e}");
            }
            if let Some(pipeline) = app.try_state::<Arc<Pipeline>>() {
                pipeline.on_hotkey_release(app);
            }
        }
    }
}
