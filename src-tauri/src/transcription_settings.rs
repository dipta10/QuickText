use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::transcription::ProviderId;

const SETTINGS_FILE_NAME: &str = "transcription-provider.json";

#[derive(Debug)]
pub struct TranscriptionSettingsState {
    active_provider: Mutex<Option<ProviderId>>,
}

impl Default for TranscriptionSettingsState {
    fn default() -> Self {
        Self {
            active_provider: Mutex::new(Some(ProviderId::Soniox)),
        }
    }
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
        active_provider: active_provider(state)?,
    })
}

pub fn active_provider(state: &TranscriptionSettingsState) -> Result<ProviderId, String> {
    (*lock(state)?).ok_or_else(|| "Could not load your transcription provider. Choose a provider in Settings before recording.".to_string())
}

pub fn set_active_provider(
    state: &TranscriptionSettingsState,
    provider: ProviderId,
) -> Result<ProviderId, String> {
    *lock(state)? = Some(provider);
    Ok(provider)
}

pub fn load(app: &AppHandle, state: &TranscriptionSettingsState) -> Result<(), String> {
    // A present but unreadable/invalid selection must not silently send the
    // next recording to the default provider.
    *lock(state)? = None;
    load_from_path(state, &settings_path(app)?)
}

fn load_from_path(state: &TranscriptionSettingsState, path: &Path) -> Result<(), String> {
    let mut selection = lock(state)?;
    *selection = None;
    let stored = read(path)?;
    *selection = Some(stored.active_provider);
    Ok(())
}

pub fn save(app: &AppHandle, provider: ProviderId) -> Result<(), String> {
    crate::settings_file::write_json(
        &settings_path(app)?,
        &StoredTranscriptionSettings {
            active_provider: provider,
        },
    )
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

fn lock(state: &TranscriptionSettingsState) -> Result<MutexGuard<'_, Option<ProviderId>>, String> {
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

    #[test]
    fn restores_each_saved_provider_into_fresh_app_state() {
        let path = std::env::temp_dir().join(format!(
            "quicktext-provider-restart-{}.json",
            std::process::id()
        ));
        for provider in [ProviderId::Deepgram, ProviderId::Soniox] {
            crate::settings_file::write_json(
                &path,
                &StoredTranscriptionSettings {
                    active_provider: provider,
                },
            )
            .unwrap();
            let restarted_state = TranscriptionSettingsState::default();
            load_from_path(&restarted_state, &path).unwrap();
            assert_eq!(
                snapshot(&restarted_state).unwrap().active_provider,
                provider
            );
        }
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_saved_selection_blocks_capture_until_the_user_selects_a_provider() {
        let path = std::env::temp_dir().join(format!(
            "quicktext-invalid-provider-{}.json",
            std::process::id()
        ));
        fs::write(&path, r#"{"activeProvider":"unknown"}"#).unwrap();
        let state = TranscriptionSettingsState::default();
        assert!(load_from_path(&state, &path).is_err());
        assert!(active_provider(&state).is_err());
        set_active_provider(&state, ProviderId::Deepgram).unwrap();
        assert_eq!(active_provider(&state).unwrap(), ProviderId::Deepgram);
        fs::remove_file(path).unwrap();
    }
}
