//! Output service: writes transcription text into the active text field.
//!
//! Strategy:
//!   1. Save the current clipboard text (best-effort).
//!   2. Write the transcription to the clipboard.
//!   3. Synthesize `Ctrl+V` via `SendInput` so the foreground window
//!      receives a paste event.
//!   4. Restore the previous clipboard contents after a short delay.
//!
//! If anything fails, we still leave the transcription in the clipboard
//! and surface a notification so the user can paste manually.

use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_V,
};

pub struct PasteService;

#[derive(Debug, Clone)]
pub enum PasteOutcome {
    /// Transcription was placed on the clipboard and Ctrl+V was sent.
    Pasted,
    /// Clipboard was set but the keystroke could not be sent — user must
    /// paste manually.
    ClipboardOnly { reason: String },
}

impl PasteService {
    pub fn deliver(text: &str) -> Result<PasteOutcome> {
        if text.is_empty() {
            return Ok(PasteOutcome::ClipboardOnly {
                reason: "empty transcription".into(),
            });
        }

        let previous = read_clipboard().ok();
        write_clipboard(text).context("writing transcription to clipboard")?;

        // Give the foreground app a beat to settle (helps with browsers
        // that briefly steal focus on hotkey release).
        thread::sleep(Duration::from_millis(50));

        let outcome = match send_paste_keystroke() {
            Ok(()) => PasteOutcome::Pasted,
            Err(e) => {
                log::warn!("paste keystroke failed: {e}");
                PasteOutcome::ClipboardOnly {
                    reason: e.to_string(),
                }
            }
        };

        // Restore clipboard a bit later so the paste actually consumes our text.
        if let Some(prev) = previous {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(400));
                if let Err(e) = write_clipboard(&prev) {
                    log::debug!("could not restore clipboard: {e}");
                }
            });
        }

        Ok(outcome)
    }
}

fn read_clipboard() -> Result<String> {
    let mut cb = arboard::Clipboard::new().context("opening clipboard")?;
    Ok(cb.get_text().context("reading clipboard text")?)
}

fn write_clipboard(text: &str) -> Result<()> {
    let mut cb = arboard::Clipboard::new().context("opening clipboard")?;
    cb.set_text(text.to_string())
        .context("setting clipboard text")?;
    Ok(())
}

#[cfg(windows)]
fn send_paste_keystroke() -> Result<()> {
    unsafe {
        let inputs = [
            key_event(VK_CONTROL, false),
            key_event(VK_V, false),
            key_event(VK_V, true),
            key_event(VK_CONTROL, true),
        ];
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent as usize != inputs.len() {
            return Err(anyhow::anyhow!(
                "SendInput injected {sent}/{} events",
                inputs.len()
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
unsafe fn key_event(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(not(windows))]
fn send_paste_keystroke() -> Result<()> {
    Err(anyhow::anyhow!("paste keystroke only implemented on Windows"))
}
