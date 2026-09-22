use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const MAX_DESCRIPTION_CHARACTERS: usize = 10_000;

const SETTINGS_FILE_NAME: &str = "transcription-description.json";

#[derive(Debug, Default)]
pub struct TranscriptionDescriptionState {
    description: Mutex<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredTranscriptionDescription {
    description: String,
}

pub fn description(state: &TranscriptionDescriptionState) -> Result<String, String> {
    Ok(lock(state)?.clone())
}

pub fn set_description(
    state: &TranscriptionDescriptionState,
    description: String,
) -> Result<String, String> {
    validate(&description)?;
    *lock(state)? = description.clone();
    Ok(description)
}

pub fn load(app: &AppHandle, state: &TranscriptionDescriptionState) -> Result<(), String> {
    let stored = read(&settings_path(app)?)?;
    set_description(state, stored.description)?;
    Ok(())
}

pub fn save(app: &AppHandle, description: &str) -> Result<(), String> {
    validate(description)?;
    let path = settings_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Could not resolve the transcription description folder.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create the settings folder: {error}"))?;
    let contents = serde_json::to_vec_pretty(&StoredTranscriptionDescription {
        description: description.to_string(),
    })
    .map_err(|error| format!("Could not serialize the transcription description: {error}"))?;
    fs::write(path, contents)
        .map_err(|error| format!("Could not save the transcription description: {error}"))
}

fn read(path: &Path) -> Result<StoredTranscriptionDescription, String> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str::<StoredTranscriptionDescription>(&contents)
            .map_err(|error| format!("Could not read the transcription description: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(StoredTranscriptionDescription::default())
        }
        Err(error) => Err(format!(
            "Could not read the transcription description: {error}"
        )),
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(SETTINGS_FILE_NAME))
        .map_err(|error| format!("Could not resolve the settings folder: {error}"))
}

fn validate(description: &str) -> Result<(), String> {
    if description.encode_utf16().count() > MAX_DESCRIPTION_CHARACTERS {
        return Err(format!(
            "Description must be {MAX_DESCRIPTION_CHARACTERS} characters or fewer."
        ));
    }

    Ok(())
}

fn lock(state: &TranscriptionDescriptionState) -> Result<MutexGuard<'_, String>, String> {
    state
        .description
        .lock()
        .map_err(|_| "Could not access the transcription description.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_description_exactly() {
        let state = TranscriptionDescriptionState::default();
        let value = "  Project Atlas\nKeep the spacing.  ".to_string();

        assert_eq!(set_description(&state, value.clone()).unwrap(), value);
        assert_eq!(description(&state).unwrap(), value);
    }

    #[test]
    fn empty_description_is_valid() {
        assert_eq!(
            set_description(&TranscriptionDescriptionState::default(), String::new()).unwrap(),
            ""
        );
    }

    #[test]
    fn enforces_the_frontend_character_limit() {
        assert!(validate(&"a".repeat(MAX_DESCRIPTION_CHARACTERS)).is_ok());
        assert!(validate(&"a".repeat(MAX_DESCRIPTION_CHARACTERS + 1)).is_err());
        assert!(validate(&"😀".repeat(MAX_DESCRIPTION_CHARACTERS / 2)).is_ok());
        assert!(validate(&"😀".repeat(MAX_DESCRIPTION_CHARACTERS / 2 + 1)).is_err());
    }

    #[test]
    fn missing_settings_file_defaults_to_empty() {
        let missing_path = std::env::temp_dir().join(format!(
            "quicktext-missing-transcription-description-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        assert_eq!(read(&missing_path).unwrap().description, "");
    }
}
