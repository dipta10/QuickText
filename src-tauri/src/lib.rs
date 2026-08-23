use std::{sync::Mutex, time::Duration};

mod app_controller;
mod audio_recorder;
mod companion_cli;
mod ipc;
#[cfg(unix)]
mod ipc_server;
mod soniox_provider;

use app_controller::{AppController, AppError, AppSnapshot, AppStatus, TranscriptResult};
use audio_recorder::{AudioCaptureStats, AudioRecorder};
use keyring::{Entry, Error as KeyringError};
use soniox_provider::SonioxSession;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const SONIOX_KEY_SERVICE: &str = "com.dipta.stt";
const SONIOX_KEY_ACCOUNT: &str = "soniox-api-key";
const DEFAULT_MAX_RECORDING_SECONDS: u64 = 5 * 60;

#[derive(Default)]
struct ShortcutSettings {
    active_shortcut: Mutex<Option<String>>,
}

#[derive(Default)]
struct AppControllerState {
    controller: Mutex<AppController>,
}

#[derive(Default)]
struct CredentialState {
    soniox_api_key: Mutex<Option<String>>,
}

#[derive(Default)]
struct AudioRecorderState {
    recorder: Mutex<Option<AudioRecorder>>,
}

#[derive(Default)]
struct TranscriptionState {
    session: Mutex<Option<SonioxSession>>,
}

fn soniox_key_entry() -> Result<Entry, String> {
    Entry::new(SONIOX_KEY_SERVICE, SONIOX_KEY_ACCOUNT)
        .map_err(|error| format!("Could not open credential store: {error}"))
}

const RECORDING_TRAY_ICON_SIZE: u32 = 32;
const RECORDING_TRAY_ICON_RADIUS: f32 = 13.0;

fn recording_tray_icon() -> tauri::image::Image<'static> {
    let size = RECORDING_TRAY_ICON_SIZE;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = (size - 1) as f32 / 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if dx * dx + dy * dy <= RECORDING_TRAY_ICON_RADIUS * RECORDING_TRAY_ICON_RADIUS {
                let offset = ((y * size + x) * 4) as usize;
                rgba[offset] = 220;
                rgba[offset + 1] = 38;
                rgba[offset + 2] = 38;
                rgba[offset + 3] = 255;
            }
        }
    }

    tauri::image::Image::new_owned(rgba, size, size)
}

fn update_tray_icon(app: &tauri::AppHandle, status: &AppStatus) {
    let Some(tray) = app.tray_by_id("quicktext") else {
        return;
    };

    let result = if *status == AppStatus::Recording {
        tray.set_icon(Some(recording_tray_icon()))
            .and_then(|()| tray.set_tooltip(Some("QuickText — Recording")))
    } else {
        app.default_window_icon()
            .cloned()
            .map(|icon| tray.set_icon(Some(icon)))
            .unwrap_or(Ok(()))
            .and_then(|()| tray.set_tooltip(Some("QuickText")))
    };

    if let Err(error) = result {
        eprintln!("Could not update tray icon: {error}");
    }
}

fn emit_app_snapshot(app: &tauri::AppHandle, snapshot: &AppSnapshot) {
    update_tray_icon(app, &snapshot.status);
    let _ = app.emit("app-state-changed", snapshot);
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

    let mut tray = TrayIconBuilder::with_id("quicktext")
        .tooltip("QuickText")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "hide" => hide_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

fn schedule_max_recording_duration(
    app: tauri::AppHandle,
    session_id: u64,
    max_recording_seconds: u64,
) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(max_recording_seconds)).await;

        let controller = app.state::<AppControllerState>();
        let should_stop = match lock_controller(&controller) {
            Ok(controller) => controller.is_recording_session(session_id),
            Err(_) => false,
        };

        if !should_stop {
            return;
        }

        let stopping = match lock_controller(&controller) {
            Ok(mut controller) => controller.begin_stop(),
            Err(_) => return,
        };
        emit_app_snapshot(&app, &stopping);

        let recorder = app.state::<AudioRecorderState>();
        let audio_stats = match stop_audio_recorder(&recorder) {
            Ok(audio_stats) => audio_stats,
            Err(_) => return,
        };

        let transcription = app.state::<TranscriptionState>();
        let transcript = match stop_transcription_session(&transcription).await {
            Ok(transcript) => transcript,
            Err(error) => {
                let error_snapshot = match lock_controller(&controller) {
                    Ok(mut controller) => controller.fail_stop(provider_unavailable_error(error)),
                    Err(_) => return,
                };
                emit_app_snapshot(&app, &error_snapshot);
                return;
            }
        };

        let transcribed = match lock_controller(&controller) {
            Ok(mut controller) => controller.finish_stop(transcript, audio_stats),
            Err(_) => return,
        };
        emit_app_snapshot(&app, &transcribed);
    });
}

fn missing_api_key_error() -> AppError {
    AppError::MissingApiKey {
        message: "Add your Soniox API key before recording.".to_string(),
    }
}

fn credential_store_error(message: String) -> AppError {
    AppError::CredentialStore { message }
}

fn microphone_unavailable_error(message: String) -> AppError {
    AppError::MicrophoneUnavailable { message }
}

fn provider_unavailable_error(message: String) -> AppError {
    AppError::ProviderUnavailable { message }
}

pub(crate) fn lock_controller<'a>(
    controller: &'a State<'_, AppControllerState>,
) -> Result<std::sync::MutexGuard<'a, AppController>, String> {
    controller
        .controller
        .lock()
        .map_err(|_| "Could not update recording state.".to_string())
}

fn lock_recorder<'a>(
    recorder: &'a State<'_, AudioRecorderState>,
) -> Result<std::sync::MutexGuard<'a, Option<AudioRecorder>>, String> {
    recorder
        .recorder
        .lock()
        .map_err(|_| "Could not update microphone state.".to_string())
}

fn lock_transcription<'a>(
    transcription: &'a State<'_, TranscriptionState>,
) -> Result<std::sync::MutexGuard<'a, Option<SonioxSession>>, String> {
    transcription
        .session
        .lock()
        .map_err(|_| "Could not update transcription state.".to_string())
}

fn cache_soniox_api_key(
    credentials: &State<'_, CredentialState>,
    api_key: &str,
) -> Result<(), String> {
    let mut cached_key = credentials
        .soniox_api_key
        .lock()
        .map_err(|_| "Could not update credential state.".to_string())?;

    *cached_key = Some(api_key.to_string());
    Ok(())
}

fn clear_cached_soniox_api_key(credentials: &State<'_, CredentialState>) -> Result<(), String> {
    let mut cached_key = credentials
        .soniox_api_key
        .lock()
        .map_err(|_| "Could not update credential state.".to_string())?;

    *cached_key = None;
    Ok(())
}

fn get_cached_soniox_api_key(
    credentials: &State<'_, CredentialState>,
) -> Result<Option<String>, String> {
    let cached_key = credentials
        .soniox_api_key
        .lock()
        .map_err(|_| "Could not read credential state.".to_string())?;

    Ok(cached_key
        .as_ref()
        .filter(|api_key| !api_key.trim().is_empty())
        .cloned())
}

fn read_soniox_api_key_from_store() -> Result<Option<String>, String> {
    match soniox_key_entry()?.get_password() {
        Ok(api_key) => {
            if api_key.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(api_key))
            }
        }
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read Soniox API key: {error}")),
    }
}

fn has_soniox_api_key_available(credentials: &State<'_, CredentialState>) -> Result<bool, String> {
    Ok(get_soniox_api_key_available(credentials)?.is_some())
}

fn get_soniox_api_key_available(
    credentials: &State<'_, CredentialState>,
) -> Result<Option<String>, String> {
    if let Some(api_key) = get_cached_soniox_api_key(credentials)? {
        return Ok(Some(api_key));
    }

    match read_soniox_api_key_from_store()? {
        Some(api_key) => {
            cache_soniox_api_key(credentials, &api_key)?;
            Ok(Some(api_key))
        }
        None => Ok(None),
    }
}

#[tauri::command]
fn get_app_state(controller: State<'_, AppControllerState>) -> Result<AppSnapshot, String> {
    Ok(lock_controller(&controller)?.snapshot())
}

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
    max_recording_seconds: Option<u64>,
) -> Result<AppSnapshot, String> {
    show_main_window(&app);
    toggle_recording_for_app(app, max_recording_seconds, false).await
}

pub(crate) async fn toggle_recording_for_app(
    app: tauri::AppHandle,
    max_recording_seconds: Option<u64>,
    focus_window: bool,
) -> Result<AppSnapshot, String> {
    let controller = app.state::<AppControllerState>();
    let credentials = app.state::<CredentialState>();
    let recorder = app.state::<AudioRecorderState>();
    let transcription = app.state::<TranscriptionState>();
    let max_recording_seconds = max_recording_seconds
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_MAX_RECORDING_SECONDS);
    let current_status = lock_controller(&controller)?.snapshot().status;

    match current_status {
        AppStatus::Idle | AppStatus::Transcribed | AppStatus::Error => {
            let starting = {
                let mut controller = lock_controller(&controller)?;
                controller.begin_start()
            };
            emit_app_snapshot(&app, &starting);

            if focus_window {
                show_main_window(&app);
            }

            let api_key = match get_soniox_api_key_available(&credentials) {
                Ok(Some(api_key)) => api_key,
                Ok(None) => {
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(missing_api_key_error())
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
                Err(error) => {
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(credential_store_error(error))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };

            let audio_format = match AudioRecorder::input_format() {
                Ok(audio_format) => audio_format,
                Err(error) => {
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(microphone_unavailable_error(error))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };

            let soniox_session = match SonioxSession::start(api_key, audio_format.clone()).await {
                Ok(session) => session,
                Err(error) => {
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(provider_unavailable_error(error))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };

            let audio_recorder = match AudioRecorder::start(soniox_session.audio_sender()) {
                Ok(recorder) => recorder,
                Err(error) => {
                    soniox_session.cancel();
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(microphone_unavailable_error(error))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };
            {
                let mut active_recorder = lock_recorder(&recorder)?;
                *active_recorder = Some(audio_recorder);
            }
            {
                let mut active_session = lock_transcription(&transcription)?;
                *active_session = Some(soniox_session);
            }

            let recording = {
                let mut controller = lock_controller(&controller)?;
                controller.finish_start(audio_format)
            };
            let session_id = lock_controller(&controller)?.active_session_id();
            emit_app_snapshot(&app, &recording);
            if let Some(session_id) = session_id {
                schedule_max_recording_duration(app.clone(), session_id, max_recording_seconds);
            }
            Ok(recording)
        }
        AppStatus::Recording => {
            let stopping = {
                let mut controller = lock_controller(&controller)?;
                controller.begin_stop()
            };
            emit_app_snapshot(&app, &stopping);

            let audio_stats = stop_audio_recorder(&recorder)?;
            let transcript = match stop_transcription_session(&transcription).await {
                Ok(transcript) => transcript,
                Err(error) => {
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_stop(provider_unavailable_error(error))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };

            let transcribed = {
                let mut controller = lock_controller(&controller)?;
                controller.finish_stop(transcript, audio_stats)
            };
            emit_app_snapshot(&app, &transcribed);

            if focus_window {
                hide_main_window(&app);
            }

            Ok(transcribed)
        }
        AppStatus::Starting | AppStatus::Stopping => {
            let snapshot = lock_controller(&controller)?.snapshot();
            Ok(snapshot)
        }
    }
}

async fn stop_transcription_session(
    transcription: &State<'_, TranscriptionState>,
) -> Result<TranscriptResult, String> {
    let active_session = {
        let mut transcription = lock_transcription(transcription)?;
        transcription.take()
    };

    match active_session {
        Some(session) => session.stop().await,
        None => Err("No active Soniox transcription session was found.".to_string()),
    }
}

fn stop_audio_recorder(
    recorder: &State<'_, AudioRecorderState>,
) -> Result<AudioCaptureStats, String> {
    let active_recorder = {
        let mut recorder = lock_recorder(recorder)?;
        recorder.take()
    };

    Ok(active_recorder.map(AudioRecorder::stop).unwrap_or_default())
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
fn has_soniox_api_key(credentials: State<'_, CredentialState>) -> Result<bool, String> {
    has_soniox_api_key_available(&credentials)
}

#[tauri::command]
fn save_soniox_api_key(
    credentials: State<'_, CredentialState>,
    api_key: String,
) -> Result<bool, String> {
    let api_key = api_key.trim();

    if api_key.is_empty() {
        return Err("Enter a Soniox API key first.".into());
    }

    soniox_key_entry()?
        .set_password(api_key)
        .map_err(|error| format!("Could not save Soniox API key: {error}"))?;

    cache_soniox_api_key(&credentials, api_key)?;
    Ok(true)
}

#[tauri::command]
fn delete_soniox_api_key(credentials: State<'_, CredentialState>) -> Result<(), String> {
    clear_cached_soniox_api_key(&credentials)?;

    match soniox_key_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not delete Soniox API key: {error}")),
    }
}

pub fn entry() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if !args.is_empty() {
        return companion_cli::run(&args);
    }

    run();
    0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ShortcutSettings::default())
        .manage(AppControllerState::default())
        .manage(CredentialState::default())
        .manage(AudioRecorderState::default())
        .manage(TranscriptionState::default())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            show_main_window(&app);
                            let _ = toggle_recording_for_app(app, None, false).await;
                        });
                    }
                })
                .build(),
        )
        .setup(|app| {
            setup_tray(app)?;

            #[cfg(unix)]
            match ipc_server::start(app.handle().clone()) {
                Ok(ipc_server::ServerStart::Listening) => {}
                Ok(ipc_server::ServerStart::AlreadyRunning) => {
                    eprintln!("Another QuickText instance is already running.");
                    app.handle().exit(1);
                }
                Err(error) => return Err(error.into()),
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            toggle_recording,
            set_global_shortcut,
            has_soniox_api_key,
            save_soniox_api_key,
            delete_soniox_api_key
        ])
        .build(tauri::generate_context!())
        .expect("error while building Tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } = event
            {
                if label == "main" {
                    api.prevent_close();
                    hide_main_window(app);
                }
            }
        });
}
