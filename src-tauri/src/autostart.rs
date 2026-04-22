//! Thin wrapper around `tauri-plugin-autostart` so the rest of the app
//! does not need to know about the plugin internals.

use anyhow::Result;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn apply(app: &AppHandle, enabled: bool) -> Result<()> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()?;
        log::info!("auto-start enabled");
    } else {
        manager.disable()?;
        log::info!("auto-start disabled");
    }
    Ok(())
}

pub fn is_enabled(app: &AppHandle) -> Result<bool> {
    Ok(app.autolaunch().is_enabled()?)
}
