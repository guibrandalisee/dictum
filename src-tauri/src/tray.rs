//! System tray icon with a minimal context menu.
//!
//! Menu items: Open, Pause/Resume, Settings, Quit. The "Pause" toggle is
//! a placeholder hook for Phase 3+ to disable hotkey processing without
//! exiting the app.

use anyhow::Result;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub const TRAY_ID: &str = "main-tray";

pub fn build(app: &AppHandle) -> Result<()> {
    let open_item = MenuItem::with_id(app, "tray:open", "Abrir", true, None::<&str>)?;
    let settings_item =
        MenuItem::with_id(app, "tray:settings", "Configurações", true, None::<&str>)?;
    let pause_item =
        MenuItem::with_id(app, "tray:toggle_pause", "Pausar ditado", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "tray:quit", "Sair", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&open_item, &settings_item, &pause_item, &separator, &quit_item],
    )?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Dictum")
        .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
            // Fallback: empty icon. tauri.conf.json should provide one.
            tauri::image::Image::new(&[0, 0, 0, 0], 1, 1)
        }))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray:open" | "tray:settings" => show_main_window(app),
            "tray:toggle_pause" => {
                // Placeholder for Phase 3+: toggle hotkey processing.
                log::info!("tray: toggle pause requested");
            }
            "tray:quit" => {
                log::info!("tray: quit requested");
                app.exit(0);
            }
            other => log::debug!("unknown tray menu id: {other}"),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
