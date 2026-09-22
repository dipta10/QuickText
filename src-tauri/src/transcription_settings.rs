use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::transcription::ProviderId;

const SETTINGS_FILE_NAME: &str = "transcription-provider.json";

#[derive(Debug, Default)]
pub struct TranscriptionSettingsState {
    active_provider: Mutex<ProviderId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionSettingsSnapshot {
    pub active_provider: ProviderId,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredTranscriptionSettings {
    #[serde(default)]
    active_provider: ProviderId,
}

pub fn snapshot(
    state: &TranscriptionSettingsState,
) -> Result<TranscriptionSettingsSnapshot, String> {
    Ok(TranscriptionSettingsSnapshot {
        active_provider: *lock(state)?,
    })
}

pub fn active_provider(state: &TranscriptionSettingsState) -> Result<ProviderId, String> {
    Ok(*lock(state)?)
}

pub fn set_active_provider(
    state: &TranscriptionSettingsState,
    provider: ProviderId,
) -> Result<ProviderId, String> {
    *lock(state)? = provider;
    Ok(provider)
}

pub fn load(app: &AppHandle, state: &TranscriptionSettingsState) -> Result<(), String> {
    let stored = read(&settings_path(app)?)?;
    set_active_provider(state, stored.active_provider)?;
    Ok(())
}

pub fn save(app: &AppHandle, provider: ProviderId) -> Result<(), String> {
    let path = settings_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Could not resolve the transcription settings folder.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create the settings folder: {error}"))?;
    let contents = serde_json::to_vec_pretty(&StoredTranscriptionSettings {
        active_provider: provider,
    })
    .map_err(|error| format!("Could not serialize transcription settings: {error}"))?;
    fs::write(path, contents)
        .map_err(|error| format!("Could not save transcription settings: {error}"))
}

fn read(path: &Path) -> Result<StoredTranscriptionSettings, String> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|error| format!("Could not read transcription settings: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(StoredTranscriptionSettings::default())
        }
        Err(error) => Err(format!("Could not read transcription settings: {error}")),
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(SETTINGS_FILE_NAME))
        .map_err(|error| format!("Could not resolve the settings folder: {error}"))
}

fn lock(state: &TranscriptionSettingsState) -> Result<MutexGuard<'_, ProviderId>, String> {
    state
        .active_provider
        .lock()
        .map_err(|_| "Could not access transcription settings.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_default_to_soniox() {
        let path = std::env::temp_dir().join(format!(
            "quicktext-missing-provider-settings-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        assert_eq!(read(&path).unwrap().active_provider, ProviderId::Soniox);
    }

    #[test]
    fn rejects_unknown_persisted_provider() {
        let error =
            serde_json::from_str::<StoredTranscriptionSettings>(r#"{"activeProvider":"unknown"}"#)
                .unwrap_err();
        assert!(error.to_string().contains("unknown variant"));
    }

    #[test]
    fn preserves_a_saved_deepgram_selection() {
        let stored: StoredTranscriptionSettings =
            serde_json::from_str(r#"{"activeProvider":"deepgram"}"#).unwrap();
        assert_eq!(stored.active_provider, ProviderId::Deepgram);
    }
}
