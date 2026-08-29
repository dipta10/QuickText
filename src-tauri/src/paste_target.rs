//! Platform seam for paste-to-target dictation.
//!
//! Synthesizes a paste keystroke into whichever application owns keyboard
//! focus when transcription finishes. The transcript itself is written to the
//! clipboard by the caller through the Tauri clipboard plugin.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

#[cfg(not(target_os = "macos"))]
const PASTE_MODIFIER: Key = Key::Control;
#[cfg(target_os = "macos")]
const PASTE_MODIFIER: Key = Key::Meta;

pub fn send_paste_keystroke() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if imp::is_hyprland_session() {
        return imp::send_wtype_paste_keystroke();
    }

    send_enigo_paste_keystroke()
}

fn send_enigo_paste_keystroke() -> Result<(), String> {
    let mut synthesizer = Enigo::new(&Settings::default())
        .map_err(|error| format!("Could not initialize input synthesis: {error}"))?;

    synthesizer
        .key(PASTE_MODIFIER, Direction::Press)
        .map_err(|error| format!("Could not press the paste modifier: {error}"))?;

    let paste_result = synthesizer.key(Key::Unicode('v'), Direction::Click);
    let release_result = synthesizer.key(PASTE_MODIFIER, Direction::Release);

    paste_result
        .and(release_result)
        .map_err(|error| format!("Could not synthesize the paste keystroke: {error}"))
}

#[cfg(target_os = "linux")]
mod imp {
    use std::process::Command;

    pub fn is_hyprland_session() -> bool {
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    pub fn send_wtype_paste_keystroke() -> Result<(), String> {
        let output = Command::new("wtype")
            .args(["-M", "ctrl", "v", "-m", "ctrl"])
            .output()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    "The wtype utility is required for paste-to-target on Wayland.".to_string()
                } else {
                    format!("Could not run wtype: {error}")
                }
            })?;

        if !output.status.success() {
            return Err(format!(
                "Could not synthesize the paste keystroke: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        Ok(())
    }
}
