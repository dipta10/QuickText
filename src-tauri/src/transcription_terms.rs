use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const MAX_TERMS_CHARACTERS: usize = 10_000;

const SETTINGS_FILE_NAME: &str = "transcription-terms.json";

#[derive(Debug, Default)]
pub struct TranscriptionTermsState {
    terms_text: Mutex<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredTranscriptionTerms {
    terms_text: String,
}

pub fn terms_text(state: &TranscriptionTermsState) -> Result<String, String> {
    Ok(lock(state)?.clone())
}

pub fn terms(state: &TranscriptionTermsState) -> Result<Vec<String>, String> {
    Ok(parse_terms(&lock(state)?))
}

pub fn set_terms_text(
    state: &TranscriptionTermsState,
    terms_text: String,
) -> Result<String, String> {
    validate(&terms_text)?;
    *lock(state)? = terms_text.clone();
    Ok(terms_text)
}

pub fn load(app: &AppHandle, state: &TranscriptionTermsState) -> Result<(), String> {
    let stored = read(&settings_path(app)?)?;
    set_terms_text(state, stored.terms_text)?;
    Ok(())
}

pub fn save(app: &AppHandle, terms_text: &str) -> Result<(), String> {
    validate(terms_text)?;
    let path = settings_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Could not resolve the transcription terms folder.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create the settings folder: {error}"))?;
    let contents = serde_json::to_vec_pretty(&StoredTranscriptionTerms {
        terms_text: terms_text.to_string(),
    })
    .map_err(|error| format!("Could not serialize the transcription terms: {error}"))?;
    fs::write(path, contents)
        .map_err(|error| format!("Could not save the transcription terms: {error}"))
}

fn parse_terms(terms_text: &str) -> Vec<String> {
    terms_text
        .lines()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(str::to_string)
        .collect()
}

fn read(path: &Path) -> Result<StoredTranscriptionTerms, String> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str::<StoredTranscriptionTerms>(&contents)
            .map_err(|error| format!("Could not read the transcription terms: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(StoredTranscriptionTerms::default())
        }
        Err(error) => Err(format!("Could not read the transcription terms: {error}")),
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(SETTINGS_FILE_NAME))
        .map_err(|error| format!("Could not resolve the settings folder: {error}"))
}

fn validate(terms_text: &str) -> Result<(), String> {
    if terms_text.encode_utf16().count() > MAX_TERMS_CHARACTERS {
        return Err(format!(
            "Terms must be {MAX_TERMS_CHARACTERS} characters or fewer."
        ));
    }

    Ok(())
}

fn lock(state: &TranscriptionTermsState) -> Result<MutexGuard<'_, String>, String> {
    state
        .terms_text
        .lock()
        .map_err(|_| "Could not access the transcription terms.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_the_editor_text_exactly() {
        let state = TranscriptionTermsState::default();
        let value = "  Soniox  \nHyprland\n".to_string();

        assert_eq!(set_terms_text(&state, value.clone()).unwrap(), value);
        assert_eq!(terms_text(&state).unwrap(), value);
    }

    #[test]
    fn parses_one_trimmed_term_per_nonblank_line() {
        assert_eq!(
            parse_terms(" Soniox \n\n  Hyprland compositor  \r\nSoniox"),
            vec!["Soniox", "Hyprland compositor", "Soniox"]
        );
    }

    #[test]
    fn empty_and_whitespace_only_text_produce_no_terms() {
        assert!(parse_terms("").is_empty());
        assert!(parse_terms("  \n\t\r\n").is_empty());
    }

    #[test]
    fn enforces_the_frontend_character_limit() {
        assert!(validate(&"a".repeat(MAX_TERMS_CHARACTERS)).is_ok());
        assert!(validate(&"a".repeat(MAX_TERMS_CHARACTERS + 1)).is_err());
        assert!(validate(&"😀".repeat(MAX_TERMS_CHARACTERS / 2)).is_ok());
        assert!(validate(&"😀".repeat(MAX_TERMS_CHARACTERS / 2 + 1)).is_err());
    }

    #[test]
    fn missing_settings_file_defaults_to_empty() {
        let missing_path = std::env::temp_dir().join(format!(
            "quicktext-missing-transcription-terms-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        assert_eq!(read(&missing_path).unwrap().terms_text, "");
    }
}
