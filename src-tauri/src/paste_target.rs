//! Platform seam for paste-to-target dictation.
//!
//! Detects whether a foreign application owns keyboard focus and synthesizes
//! the paste keystroke. The transcript itself is written to the clipboard by
//! the caller through the Tauri clipboard plugin.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

#[cfg(not(target_os = "macos"))]
const PASTE_MODIFIER: Key = Key::Control;
#[cfg(target_os = "macos")]
const PASTE_MODIFIER: Key = Key::Meta;

pub fn foreign_app_has_focus() -> Result<bool, String> {
    imp::foreign_app_has_focus()
}

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
        .and_then(|()| synthesizer.key(Key::Unicode('v'), Direction::Click))
        .and_then(|()| synthesizer.key(PASTE_MODIFIER, Direction::Release))
        .map_err(|error| format!("Could not synthesize the paste keystroke: {error}"))
}

#[cfg(target_os = "linux")]
mod imp {
    use std::process::Command;

    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    use x11rb::rust_connection::RustConnection;

    const NO_FOCUSED_APP: &str = "No application currently has keyboard focus.";

    pub fn is_hyprland_session() -> bool {
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    pub fn foreign_app_has_focus() -> Result<bool, String> {
        if is_hyprland_session() {
            hyprland_foreign_app_has_focus()
        } else {
            x11_foreign_app_has_focus()
        }
    }

    fn hyprland_foreign_app_has_focus() -> Result<bool, String> {
        let output = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .map_err(|error| format!("Could not query Hyprland for the focused window: {error}"))?;

        let owner_pid = parse_active_window_pid(String::from_utf8_lossy(&output.stdout).trim())?;
        Ok(owner_pid.is_none_or(|pid| pid != std::process::id()))
    }

    fn parse_active_window_pid(hyprctl_output: &str) -> Result<Option<u32>, String> {
        let active_window: serde_json::Value =
            serde_json::from_str(hyprctl_output).map_err(|_| NO_FOCUSED_APP.to_string())?;
        let Some(pid) = active_window.get("pid").and_then(serde_json::Value::as_u64) else {
            return Err(NO_FOCUSED_APP.to_string());
        };

        Ok(u32::try_from(pid).ok())
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

    fn x11_foreign_app_has_focus() -> Result<bool, String> {
        let (connection, screen_number) = x11rb::connect(None)
            .map_err(|error| format!("Could not reach the display server: {error}"))?;
        let root = connection
            .setup()
            .roots
            .get(screen_number)
            .ok_or("The display server reported no usable screen.")?
            .root;

        let active_window =
            read_u32_property(&connection, root, b"_NET_ACTIVE_WINDOW", AtomEnum::WINDOW)?;
        let Some(active_window) = active_window else {
            return Err(NO_FOCUSED_APP.to_string());
        };

        let window_owner = read_u32_property(
            &connection,
            active_window,
            b"_NET_WM_PID",
            AtomEnum::CARDINAL,
        )?;

        // Without an owner PID we cannot prove the window is ours; assume it
        // belongs to the foreign app so dictation can proceed.
        Ok(window_owner.is_none_or(|pid| pid != std::process::id()))
    }

    fn read_u32_property(
        connection: &RustConnection,
        window: u32,
        property_name: &[u8],
        property_type: AtomEnum,
    ) -> Result<Option<u32>, String> {
        let property = intern_atom(connection, property_name)?;
        let reply = connection
            .get_property(false, window, property, property_type, 0, 1)
            .map_err(|error| format!("Could not query the focused window: {error}"))?
            .reply()
            .map_err(|error| format!("Could not query the focused window: {error}"))?;

        Ok(reply.value32().and_then(|mut values| values.next()))
    }

    fn intern_atom(connection: &RustConnection, name: &[u8]) -> Result<u32, String> {
        connection
            .intern_atom(false, name)
            .map_err(|error| format!("Could not query the display server: {error}"))?
            .reply()
            .map(|reply| reply.atom)
            .map_err(|error| format!("Could not query the display server: {error}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_active_window_pid() {
            let output = r#"{"address":"0x55f0","mapped":true,"hidden":false,"at":[11,11],"size":[100,100],"class":"foot","pid":4242,"title":"term"}"#;

            assert_eq!(parse_active_window_pid(output).unwrap(), Some(4242));
        }

        #[test]
        fn reports_no_focus_for_invalid_output() {
            assert!(parse_active_window_pid("Invalid").is_err());
            assert!(parse_active_window_pid("").is_err());
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    pub fn foreign_app_has_focus() -> Result<bool, String> {
        let foreground_window = unsafe { GetForegroundWindow() };
        if foreground_window.is_invalid() {
            return Err("No application currently has keyboard focus.".to_string());
        }

        let mut owner_process_id = 0u32;
        unsafe { GetWindowThreadProcessId(foreground_window, Some(&mut owner_process_id)) };

        Ok(owner_process_id != std::process::id())
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2_app_kit::NSWorkspace;

    pub fn foreign_app_has_focus() -> Result<bool, String> {
        let workspace = unsafe { NSWorkspace::sharedWorkspace() };
        let frontmost_app = unsafe { workspace.frontmostApplication() }
            .ok_or("No application currently has keyboard focus.")?;
        let frontmost_process_id = unsafe { frontmost_app.processIdentifier() };

        Ok(frontmost_process_id != std::process::id() as i32)
    }
}
