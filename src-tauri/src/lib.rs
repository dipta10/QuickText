use std::sync::Mutex;

use keyring::{Entry, Error as KeyringError};
use tauri::{Emitter, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const SONIOX_KEY_SERVICE: &str = "com.dipta.stt";
const SONIOX_KEY_ACCOUNT: &str = "soniox-api-key";

#[derive(Default)]
struct ShortcutSettings {
    active_shortcut: Mutex<Option<String>>,
}

fn soniox_key_entry() -> Result<Entry, String> {
    Entry::new(SONIOX_KEY_SERVICE, SONIOX_KEY_ACCOUNT)
        .map_err(|error| format!("Could not open credential store: {error}"))
}

#[tauri::command]
fn set_global_shortcut(
    app: tauri::AppHandle,
    settings: State<'_, ShortcutSettings>,
    shortcut: String,
) -> Result<String, String> {
    let shortcut = shortcut.trim();

    if shortcut.is_empty() {
        return Err("Choose a shortcut first.".into());
    }

    app.global_shortcut()
        .unregister_all()
        .map_err(|error| format!("Could not clear the previous shortcut: {error}"))?;

    app.global_shortcut()
        .register(shortcut)
        .map_err(|error| format!("Could not register shortcut: {error}"))?;

    let mut active_shortcut = settings
        .active_shortcut
        .lock()
        .map_err(|_| "Could not update shortcut state.".to_string())?;

    *active_shortcut = Some(shortcut.to_string());

    Ok(shortcut.to_string())
}

#[tauri::command]
fn has_soniox_api_key() -> Result<bool, String> {
    match soniox_key_entry()?.get_password() {
        Ok(api_key) => Ok(!api_key.trim().is_empty()),
        Err(KeyringError::NoEntry) => Ok(false),
        Err(error) => Err(format!("Could not read Soniox API key: {error}")),
    }
}

#[tauri::command]
fn save_soniox_api_key(api_key: String) -> Result<(), String> {
    let api_key = api_key.trim();

    if api_key.is_empty() {
        return Err("Enter a Soniox API key first.".into());
    }

    soniox_key_entry()?
        .set_password(api_key)
        .map_err(|error| format!("Could not save Soniox API key: {error}"))
}

#[tauri::command]
fn delete_soniox_api_key() -> Result<(), String> {
    match soniox_key_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not delete Soniox API key: {error}")),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ShortcutSettings::default())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let _ = app.emit("global-shortcut-pressed", shortcut.to_string());
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            set_global_shortcut,
            has_soniox_api_key,
            save_soniox_api_key,
            delete_soniox_api_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
