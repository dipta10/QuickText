use std::sync::Mutex;

use tauri::{Emitter, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Default)]
struct ShortcutSettings {
    active_shortcut: Mutex<Option<String>>,
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
        .invoke_handler(tauri::generate_handler![set_global_shortcut])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
