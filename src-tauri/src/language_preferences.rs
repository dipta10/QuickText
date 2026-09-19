use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const SETTINGS_FILE_NAME: &str = "language-preferences.json";

const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("af", "Afrikaans"),
    ("sq", "Albanian"),
    ("ar", "Arabic"),
    ("az", "Azerbaijani"),
    ("eu", "Basque"),
    ("be", "Belarusian"),
    ("bn", "Bengali"),
    ("bs", "Bosnian"),
    ("bg", "Bulgarian"),
    ("ca", "Catalan"),
    ("zh", "Chinese"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("et", "Estonian"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("gl", "Galician"),
    ("de", "German"),
    ("el", "Greek"),
    ("gu", "Gujarati"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hu", "Hungarian"),
    ("id", "Indonesian"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("kn", "Kannada"),
    ("kk", "Kazakh"),
    ("ko", "Korean"),
    ("lv", "Latvian"),
    ("lt", "Lithuanian"),
    ("mk", "Macedonian"),
    ("ms", "Malay"),
    ("ml", "Malayalam"),
    ("mr", "Marathi"),
    ("no", "Norwegian"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("pa", "Punjabi"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sr", "Serbian"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("es", "Spanish"),
    ("sw", "Swahili"),
    ("sv", "Swedish"),
    ("tl", "Tagalog"),
    ("ta", "Tamil"),
    ("te", "Telugu"),
    ("th", "Thai"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("vi", "Vietnamese"),
    ("cy", "Welsh"),
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SupportedLanguage {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePreferencesSnapshot {
    pub available_languages: Vec<SupportedLanguage>,
    pub selected_codes: Vec<String>,
}

#[derive(Debug, Default)]
pub struct LanguagePreferencesState {
    selected_codes: Mutex<Vec<String>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredLanguagePreferences {
    selected_codes: Vec<String>,
}

pub fn snapshot(state: &LanguagePreferencesState) -> Result<LanguagePreferencesSnapshot, String> {
    Ok(LanguagePreferencesSnapshot {
        available_languages: supported_languages(),
        selected_codes: lock(state)?.clone(),
    })
}

pub fn selected_codes(state: &LanguagePreferencesState) -> Result<Vec<String>, String> {
    Ok(lock(state)?.clone())
}

pub fn set_selected_codes(
    state: &LanguagePreferencesState,
    selected_codes: Vec<String>,
) -> Result<Vec<String>, String> {
    let selected_codes = normalize(selected_codes)?;
    *lock(state)? = selected_codes.clone();
    Ok(selected_codes)
}

pub fn load(app: &AppHandle, state: &LanguagePreferencesState) -> Result<(), String> {
    let path = settings_path(app)?;
    let stored = read(&path)?;

    set_selected_codes(state, stored.selected_codes)?;
    Ok(())
}

fn read(path: &Path) -> Result<StoredLanguagePreferences, String> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str::<StoredLanguagePreferences>(&contents)
            .map_err(|error| format!("Could not read language preferences: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(StoredLanguagePreferences::default())
        }
        Err(error) => Err(format!("Could not read language preferences: {error}")),
    }
}

pub fn save(app: &AppHandle, selected_codes: &[String]) -> Result<(), String> {
    let path = settings_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Could not resolve the language settings folder.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create the settings folder: {error}"))?;
    let contents = serde_json::to_vec_pretty(&StoredLanguagePreferences {
        selected_codes: selected_codes.to_vec(),
    })
    .map_err(|error| format!("Could not serialize language preferences: {error}"))?;
    fs::write(path, contents)
        .map_err(|error| format!("Could not save language preferences: {error}"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(SETTINGS_FILE_NAME))
        .map_err(|error| format!("Could not resolve the settings folder: {error}"))
}

fn supported_languages() -> Vec<SupportedLanguage> {
    SUPPORTED_LANGUAGES
        .iter()
        .map(|(code, name)| SupportedLanguage {
            code: (*code).to_string(),
            name: (*name).to_string(),
        })
        .collect()
}

fn normalize(selected_codes: Vec<String>) -> Result<Vec<String>, String> {
    let supported: HashSet<&str> = SUPPORTED_LANGUAGES.iter().map(|(code, _)| *code).collect();
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for code in selected_codes {
        let code = code.trim().to_lowercase();
        if !supported.contains(code.as_str()) {
            return Err(format!("Unsupported transcription language: {code}"));
        }
        if seen.insert(code.clone()) {
            normalized.push(code);
        }
    }

    normalized.sort_by_key(|code| {
        SUPPORTED_LANGUAGES
            .iter()
            .position(|(supported_code, _)| supported_code == code)
            .unwrap_or(usize::MAX)
    });
    Ok(normalized)
}

fn lock(state: &LanguagePreferencesState) -> Result<MutexGuard<'_, Vec<String>>, String> {
    state
        .selected_codes
        .lock()
        .map_err(|_| "Could not access language preferences.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_preferences_mean_automatic_detection() {
        assert_eq!(normalize(Vec::new()).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn rejects_unknown_language_codes() {
        assert_eq!(
            normalize(vec!["xx".to_string()]),
            Err("Unsupported transcription language: xx".to_string())
        );
    }

    #[test]
    fn normalizes_case_duplicates_and_catalog_order() {
        assert_eq!(
            normalize(vec!["ES".to_string(), "en".to_string(), "es".to_string()]).unwrap(),
            vec!["en".to_string(), "es".to_string()]
        );
    }

    #[test]
    fn missing_settings_file_defaults_to_automatic_detection() {
        let missing_path = std::env::temp_dir().join(format!(
            "quicktext-missing-language-preferences-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        assert_eq!(
            read(&missing_path).unwrap().selected_codes,
            Vec::<String>::new()
        );
    }
}
