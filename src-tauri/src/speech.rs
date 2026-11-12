//! Backend-owned reading lifecycle; all triggers share this controller.
use crate::{
    speech_playback::AudioPlayback, transcription::ProviderId, AppControllerState, CredentialState,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::watch;

pub const MAX_TEXT_CHARACTERS: usize = 5_000;
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct SpeechSettings {
    pub voice: String,
    pub language: String,
    pub speed: f32,
}
impl Default for SpeechSettings {
    fn default() -> Self {
        Self {
            voice: "Adrian".into(),
            language: "en".into(),
            speed: 1.0,
        }
    }
}
impl SpeechSettings {
    fn validate(&self) -> Result<(), String> {
        if self.voice.trim().is_empty()
            || self.voice.chars().count() > 50
            || self.voice.chars().any(char::is_control)
        {
            return Err("Enter a Soniox voice name or voice ID (up to 50 characters).".into());
        }
        if !crate::language_preferences::supported_languages()
            .iter()
            .any(|language| language.code == self.language)
        {
            return Err("Choose a supported speech language.".into());
        }
        crate::speech_playback::validate_speed(self.speed)
    }
}
#[derive(Clone, Copy, Default, Serialize, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Idle,
    Preparing,
    Playing,
    Completed,
    Error,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: Status,
    pub session_id: u64,
    pub revision: u64,
    pub error: Option<String>,
    pub playback_speed: f32,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            status: Status::Idle,
            session_id: 0,
            revision: 0,
            error: None,
            playback_speed: 1.0,
        }
    }
}
#[derive(Default)]
struct Inner {
    snapshot: Snapshot,
    cancel: Option<watch::Sender<bool>>,
}
impl Inner {
    fn begin(&mut self) -> (u64, watch::Receiver<bool>) {
        let (cancel, receiver) = watch::channel(false);
        self.snapshot.session_id += 1;
        self.snapshot.status = Status::Preparing;
        self.snapshot.error = None;
        self.cancel = Some(cancel);
        (self.snapshot.session_id, receiver)
    }
    fn stop(&mut self) -> bool {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(true);
        }
        if !active(self.snapshot.status) {
            return false;
        }
        self.snapshot.status = Status::Idle;
        self.snapshot.error = None;
        true
    }
    fn transition(&mut self, id: u64, status: Status, error: Option<String>) -> bool {
        if self.snapshot.session_id != id || !active(self.snapshot.status) {
            return false;
        }
        self.snapshot.status = status;
        self.snapshot.error = error;
        if !active(status) {
            self.cancel = None;
        }
        true
    }
}
pub struct SpeechState {
    inner: Mutex<Inner>,
    settings: Mutex<Result<SpeechSettings, String>>,
    playback_speed: Arc<AtomicU32>,
}
// Result has no Default; keep a load failure visible instead of overwriting a corrupt file.

impl Default for SpeechState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            settings: Mutex::new(Ok(SpeechSettings::default())),
            playback_speed: Arc::new(AtomicU32::new(1.0_f32.to_bits())),
        }
    }
}
fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|p| p.join("speech.json"))
        .map_err(|_| "Could not find speech settings.".into())
}
fn read_settings(path: &Path) -> Result<SpeechSettings, String> {
    let settings: SpeechSettings = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|_| "Could not read speech settings.".to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SpeechSettings::default(),
        Err(_) => return Err("Could not read speech settings.".into()),
    };
    settings.validate()?;
    Ok(settings)
}
pub fn load(app: &AppHandle) {
    let saved = settings_path(app).and_then(|p| read_settings(&p));
    let state = app.state::<SpeechState>();
    if let Ok(settings) = &saved {
        state
            .playback_speed
            .store(settings.speed.to_bits(), Ordering::Release);
        state.inner.lock().unwrap().snapshot.playback_speed = settings.speed;
    }
    *state.settings.lock().unwrap() = saved;
}
fn publish(app: &AppHandle, inner: &mut Inner) -> Snapshot {
    inner.snapshot.revision += 1;
    let snapshot = inner.snapshot.clone();
    let _ = app.emit("speech-state-changed", &snapshot);
    snapshot
}
fn active(status: Status) -> bool {
    matches!(status, Status::Preparing | Status::Playing)
}
fn validate_text(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("Add some text or select a passage to read aloud.".into());
    }
    if text.chars().count() > MAX_TEXT_CHARACTERS {
        return Err("Select a shorter passage (up to 5,000 characters).".into());
    }
    Ok(())
}
#[tauri::command]
pub fn get_speech_state(state: State<'_, SpeechState>) -> Result<Snapshot, String> {
    Ok(state
        .inner
        .lock()
        .map_err(|_| "Speech is unavailable.".to_string())?
        .snapshot
        .clone())
}
#[tauri::command]
pub async fn list_speech_voices(
    credentials: State<'_, CredentialState>,
) -> Result<Vec<crate::soniox_speech::Voice>, String> {
    let key = crate::get_api_key_available(&credentials, ProviderId::Soniox)?
        .ok_or_else(|| "Add a Soniox API key to load the available voices.".to_string())?;
    crate::soniox_speech::list_voices(&key).await
}
#[tauri::command]
pub fn get_speech_settings(state: State<'_, SpeechState>) -> Result<SpeechSettings, String> {
    state
        .settings
        .lock()
        .map_err(|_| "Speech settings are unavailable.".to_string())?
        .clone()
}
#[tauri::command]
pub fn set_speech_settings(
    app: AppHandle,
    state: State<'_, SpeechState>,
    settings: SpeechSettings,
) -> Result<SpeechSettings, String> {
    save_preferences(&app, &state, |saved| *saved = settings)?;
    get_speech_settings(state)
}

// Both save paths use the same lock order as reading start. Save before changing
// live playback so a disk failure leaves the previous preference and rate intact.
fn save_preferences(
    app: &AppHandle,
    state: &SpeechState,
    update: impl FnOnce(&mut SpeechSettings),
) -> Result<Snapshot, String> {
    let mut inner = state
        .inner
        .lock()
        .map_err(|_| "Speech is unavailable.".to_string())?;
    let mut saved = state
        .settings
        .lock()
        .map_err(|_| "Speech settings are unavailable.".to_string())?;
    let mut next = saved.as_ref().map_err(Clone::clone)?.clone();
    update(&mut next);
    next.validate()?;
    crate::settings_file::write_json(&settings_path(app)?, &next)?;
    state
        .playback_speed
        .store(next.speed.to_bits(), Ordering::Release);
    inner.snapshot.playback_speed = next.speed;
    *saved = Ok(next);
    Ok(publish(app, &mut inner))
}
#[tauri::command]
pub fn set_playback_speed(
    app: AppHandle,
    state: State<'_, SpeechState>,
    speed: f32,
) -> Result<Snapshot, String> {
    save_preferences(&app, &state, |saved| saved.speed = speed)
}
#[tauri::command]
pub fn stop_speech(app: AppHandle) -> Result<Snapshot, String> {
    stop(&app)
}
pub fn stop(app: &AppHandle) -> Result<Snapshot, String> {
    let state = app.state::<SpeechState>();
    let mut inner = state
        .inner
        .lock()
        .map_err(|_| "Speech is unavailable.".to_string())?;
    if inner.stop() {
        return Ok(publish(app, &mut inner));
    }
    Ok(inner.snapshot.clone())
}
#[tauri::command]
pub fn toggle_speech(app: AppHandle, text: Option<String>) -> Result<Snapshot, String> {
    toggle(&app, text)
}
pub fn toggle(app: &AppHandle, text: Option<String>) -> Result<Snapshot, String> {
    let result = toggle_inner(app, text);
    if let Err(error) = &result {
        let state = app.state::<SpeechState>();
        if let Ok(mut inner) = state.inner.lock() {
            if !active(inner.snapshot.status) {
                inner.snapshot.status = Status::Error;
                inner.snapshot.error = Some(error.clone());
                publish(app, &mut inner);
            }
        };
    }
    result
}
fn toggle_inner(app: &AppHandle, text: Option<String>) -> Result<Snapshot, String> {
    // Same lock order as recording start: recording controller, then reading state.
    let controller_state = app.state::<AppControllerState>();
    let controller = crate::lock_controller(&controller_state)?;
    let state = app.state::<SpeechState>();
    let mut inner = state
        .inner
        .lock()
        .map_err(|_| "Speech is unavailable.".to_string())?;
    if active(inner.snapshot.status) {
        inner.stop();
        return Ok(publish(app, &mut inner));
    }
    if matches!(
        controller.snapshot().status,
        crate::AppStatus::Starting | crate::AppStatus::Recording | crate::AppStatus::Stopping
    ) {
        return Err("Finish your recording before reading text aloud.".into());
    }
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Speech settings are unavailable.".to_string())?
        .clone()?;
    if let Some(text) = &text {
        validate_text(text)?;
    }
    let credentials = app.state::<CredentialState>();
    let key = crate::get_api_key_available(&credentials, ProviderId::Soniox)?
        .ok_or_else(|| "Add a Soniox API key in Settings before reading aloud.".to_string())?;
    let speed = state.playback_speed.clone();
    let (id, mut cancelled) = inner.begin();
    let snapshot = publish(app, &mut inner);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let work = async {
            let text = match text {
                Some(text) => text,
                None => crate::selected_text::read(&app).await?,
            };
            validate_text(&text)?;
            let mut playback = AudioPlayback::start(speed).await?;
            let mut started = false;
            crate::soniox_speech::stream(&text, &settings, &key, id, |bytes| {
                playback.push(bytes)?;
                if !started && !bytes.is_empty() {
                    started = true;
                    transition(&app, id, Status::Playing, None);
                }
                Ok(())
            })
            .await?;
            playback.finish().await
        };
        let outcome = tokio::select! {
            biased;
            _ = cancelled.changed() => return,
            result = tokio::time::timeout(std::time::Duration::from_secs(300), work) => result.unwrap_or_else(|_| Err("Reading timed out. Try a shorter passage.".into())),
        };
        match outcome {
            Ok(()) => transition(&app, id, Status::Completed, None),
            Err(error) => transition(&app, id, Status::Error, Some(error)),
        }
    });
    Ok(snapshot)
}
fn transition(app: &AppHandle, id: u64, status: Status, error: Option<String>) {
    let state = app.state::<SpeechState>();
    if let Ok(mut inner) = state.inner.lock() {
        if !inner.transition(id, status, error) {
            return;
        }
        publish(app, &mut inner);
    };
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopping_preparation_cancels_work_and_obsolete_updates_cannot_change_a_new_reading() {
        let mut reading = Inner::default();
        let (first, cancelled) = reading.begin();
        assert_eq!(reading.snapshot.status, Status::Preparing);
        assert!(reading.stop());
        assert!(*cancelled.borrow());
        assert!(!reading.transition(first, Status::Playing, None));
        assert!(!reading.stop());
        let (second, next_cancelled) = reading.begin();
        assert!(second > first);
        assert!(!reading.transition(first, Status::Error, Some("obsolete".into())));
        assert!(reading.transition(second, Status::Playing, None));
        assert_eq!(reading.snapshot.status, Status::Playing);
        assert!(reading.stop());
        assert!(*next_cancelled.borrow());
        assert_eq!(reading.snapshot.status, Status::Idle);
    }
    #[test]
    fn completion_releases_the_reading_without_accepting_late_errors() {
        let mut reading = Inner::default();
        let (id, _) = reading.begin();
        assert!(reading.transition(id, Status::Playing, None));
        assert!(reading.transition(id, Status::Completed, None));
        assert!(reading.cancel.is_none());
        assert!(!reading.transition(id, Status::Error, Some("late".into())));
        assert_eq!(reading.snapshot.status, Status::Completed);
    }
    #[test]
    fn validates_unicode_text_and_never_accepts_empty_or_oversized_input() {
        assert!(validate_text(" \n ").is_err());
        assert!(validate_text(&"আ".repeat(5000)).is_ok());
        assert!(validate_text(&"আ".repeat(5001)).is_err());
    }
    #[test]
    fn settings_validate_bounds_and_round_trip() {
        let mut settings = SpeechSettings::default();
        settings.speed = f32::NAN;
        assert!(settings.validate().is_err());
        settings.speed = 2.01;
        assert!(settings.validate().is_err());
        settings.speed = 0.5;
        settings.language = "bn".into();
        settings.voice = "Adrian".into();
        assert!(settings.validate().is_ok());
        let path =
            std::env::temp_dir().join(format!("quicktext-speech-{}.json", std::process::id()));
        crate::settings_file::write_json(&path, &settings).unwrap();
        assert_eq!(read_settings(&path).unwrap(), settings);
        fs::remove_file(path).unwrap();
    }
}
