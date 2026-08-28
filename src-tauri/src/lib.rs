use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

mod app_controller;
mod audio_recorder;
mod autostart;
mod companion_cli;
mod ipc;
#[cfg(unix)]
mod ipc_server;
mod logger;
mod soniox_provider;

#[cfg(test)]
mod transcription_fixture_tests;

use app_controller::{AppController, AppError, AppSnapshot, AppStatus, TranscriptResult};
use audio_recorder::{AudioCaptureStats, AudioRecorder};
use keyring::{Entry, Error as KeyringError};
use logger::{
    AppLogger, BuildInfo, DebugEvent, DebugPhase, ErrorEvent, FailureCategory, InfoEvent,
    LogContext, LoggedAppState, Logger, Operation, TriggerSource, WarnEvent,
};
use soniox_provider::{PartialTranscript, SonioxSession};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const SONIOX_KEY_SERVICE: &str = "com.dipta.stt";
const SONIOX_KEY_ACCOUNT: &str = "soniox-api-key";
const DEFAULT_MAX_RECORDING_SECONDS: u64 = 5 * 60;

struct ShortcutSettings {
    active_shortcut: Mutex<Option<String>>,
    focus_on_start: Mutex<bool>,
    hide_on_stop: Mutex<bool>,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            active_shortcut: Mutex::new(None),
            focus_on_start: Mutex::new(true),
            hide_on_stop: Mutex::new(false),
        }
    }
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

#[derive(Default)]
struct LoggingContextState {
    active: Mutex<Option<LogContext>>,
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

    let result = match status {
        AppStatus::Starting | AppStatus::Recording | AppStatus::Stopping => {
            let tooltip = if *status == AppStatus::Stopping {
                "QuickText — Finalizing"
            } else {
                "QuickText — Recording"
            };
            tray.set_icon(Some(recording_tray_icon()))
                .and_then(|()| tray.set_tooltip(Some(tooltip)))
        }
        AppStatus::Idle | AppStatus::Transcribed | AppStatus::Error => app
            .default_window_icon()
            .cloned()
            .map(|icon| tray.set_icon(Some(icon)))
            .unwrap_or(Ok(()))
            .and_then(|()| tray.set_tooltip(Some("QuickText"))),
    };

    if result.is_err() {
        app.state::<AppLogger>().warn(
            active_log_context(app),
            WarnEvent::OperationFailed {
                operation: Operation::TrayUpdate,
            },
        );
    }
}

fn emit_app_snapshot(app: &tauri::AppHandle, snapshot: &AppSnapshot) {
    update_tray_icon(app, &snapshot.status);
    let _ = app.emit("app-state-changed", snapshot);
}

async fn forward_partial_transcripts(
    app: tauri::AppHandle,
    mut partial_rx: tokio::sync::mpsc::UnboundedReceiver<PartialTranscript>,
) {
    while let Some(update) = partial_rx.recv().await {
        let _ = app.emit("partial-transcript", update);
    }
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

        let log_context = active_log_context(&app);
        app.state::<AppLogger>().info(
            log_context,
            InfoEvent::TriggerReceived {
                source: TriggerSource::DurationLimit,
            },
        );

        let stopping = match lock_controller(&controller) {
            Ok(mut controller) => controller.begin_stop(),
            Err(_) => return,
        };
        app.state::<AppLogger>().info(
            log_context,
            InfoEvent::StateChanged {
                from: LoggedAppState::Recording,
                to: LoggedAppState::Stopping,
            },
        );
        emit_app_snapshot(&app, &stopping);

        let recorder = app.state::<AudioRecorderState>();
        let audio_stats = match stop_audio_recorder(&recorder) {
            Ok(audio_stats) => audio_stats,
            Err(_) => return,
        };
        app.state::<AppLogger>().info(
            log_context,
            InfoEvent::RecordingStopped {
                duration_ms: u64::try_from(audio_stats.elapsed_ms).unwrap_or(u64::MAX),
                chunk_count: audio_stats.chunk_count,
                sample_count: audio_stats.sample_count,
                byte_count: audio_stats.byte_count,
            },
        );
        app.state::<AppLogger>()
            .info(log_context, InfoEvent::ProviderFinalizing);

        let transcription = app.state::<TranscriptionState>();
        let finalize_started = Instant::now();
        let transcript = match stop_transcription_session(&transcription).await {
            Ok(transcript) => transcript,
            Err(error) => {
                let reference = record_failure(
                    &app,
                    FailureCategory::ProviderUnavailable,
                    "provider_finalize_failed",
                );
                let error_snapshot = match lock_controller(&controller) {
                    Ok(mut controller) => {
                        controller.fail_stop(provider_unavailable_error(&error, reference))
                    }
                    Err(_) => return,
                };
                emit_app_snapshot(&app, &error_snapshot);
                return;
            }
        };
        app.state::<AppLogger>().debug(
            log_context,
            DebugEvent::Timing {
                phase: DebugPhase::ProviderFinalize,
                elapsed_ms: u64::try_from(finalize_started.elapsed().as_millis())
                    .unwrap_or(u64::MAX),
            },
        );

        let transcribed = match lock_controller(&controller) {
            Ok(mut controller) => controller.finish_stop(transcript, audio_stats),
            Err(_) => return,
        };
        app.state::<AppLogger>()
            .info(log_context, InfoEvent::ProviderCompleted);
        app.state::<AppLogger>().info(
            log_context,
            InfoEvent::StateChanged {
                from: LoggedAppState::Stopping,
                to: LoggedAppState::Transcribed,
            },
        );
        clear_log_context(&app);
        emit_app_snapshot(&app, &transcribed);
    });
}

async fn fail_recording_when_provider_unreachable(
    app: tauri::AppHandle,
    session_id: u64,
    provider_started: Instant,
    provider_ready_rx: tokio::sync::oneshot::Receiver<Result<(), String>>,
) {
    let outcome = provider_ready_rx
        .await
        .unwrap_or_else(|_| Err("The Soniox connection ended unexpectedly.".to_string()));
    app.state::<AppLogger>().debug(
        active_log_context(&app),
        DebugEvent::Timing {
            phase: DebugPhase::ProviderConnect,
            elapsed_ms: u64::try_from(provider_started.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
    );
    if outcome.is_ok() {
        app.state::<AppLogger>()
            .info(active_log_context(&app), InfoEvent::ProviderConnected);
        return;
    }

    let controller = app.state::<AppControllerState>();
    let still_recording = lock_controller(&controller)
        .map(|controller| controller.is_recording_session(session_id))
        .unwrap_or(false);
    if !still_recording {
        // The session was already stopped; the stop path surfaces the error.
        return;
    }

    let _ = stop_audio_recorder(&app.state::<AudioRecorderState>());
    let active_session = {
        let transcription = app.state::<TranscriptionState>();
        lock_transcription(&transcription)
            .ok()
            .and_then(|mut session| session.take())
    };
    if let Some(session) = active_session {
        session.cancel();
    }

    let error_message = match outcome {
        Ok(()) => return,
        Err(error) => error,
    };
    let reference = record_failure(
        &app,
        FailureCategory::ProviderUnavailable,
        "provider_connection_failed",
    );
    let error_snapshot = match lock_controller(&controller) {
        Ok(mut controller) => {
            controller.fail_recording(provider_unavailable_error(&error_message, reference))
        }
        Err(_) => return,
    };
    emit_app_snapshot(&app, &error_snapshot);
}

fn app_status_for_log(status: &AppStatus) -> LoggedAppState {
    match status {
        AppStatus::Idle => LoggedAppState::Idle,
        AppStatus::Starting => LoggedAppState::Starting,
        AppStatus::Recording => LoggedAppState::Recording,
        AppStatus::Stopping => LoggedAppState::Stopping,
        AppStatus::Transcribed => LoggedAppState::Transcribed,
        AppStatus::Error => LoggedAppState::Error,
    }
}

fn active_log_context(app: &tauri::AppHandle) -> LogContext {
    app.state::<LoggingContextState>()
        .active
        .lock()
        .ok()
        .and_then(|context| *context)
        .unwrap_or_default()
}

fn begin_log_context(app: &tauri::AppHandle) -> LogContext {
    let context = LogContext::recording();
    set_log_context(app, context);
    context
}

fn set_log_context(app: &tauri::AppHandle, context: LogContext) {
    if let Ok(mut active) = app.state::<LoggingContextState>().active.lock() {
        *active = Some(context);
    }
}

fn clear_log_context(app: &tauri::AppHandle) {
    if let Ok(mut active) = app.state::<LoggingContextState>().active.lock() {
        *active = None;
    }
}

fn record_failure(app: &tauri::AppHandle, category: FailureCategory, code: &'static str) -> String {
    let context = active_log_context(app).with_error();
    app.state::<AppLogger>()
        .error(context, ErrorEvent::Failure { category, code });
    let reference = context
        .support_reference()
        .unwrap_or_else(|| "QT-UNKNOWN".to_string());
    clear_log_context(app);
    reference
}

fn missing_api_key_error(support_reference: String) -> AppError {
    AppError::MissingApiKey {
        message: "Add your Soniox API key before recording.".to_string(),
        support_reference,
    }
}

fn credential_store_error(support_reference: String) -> AppError {
    AppError::CredentialStore {
        message: "QuickText could not access the credential store.".to_string(),
        support_reference,
    }
}

fn microphone_unavailable_error(support_reference: String) -> AppError {
    AppError::MicrophoneUnavailable {
        message: "QuickText could not start the microphone. Check microphone access and try again."
            .to_string(),
        support_reference,
    }
}

fn provider_unavailable_error(raw_error: &str, support_reference: String) -> AppError {
    let message = if raw_error.to_lowercase().contains("incorrect api key") {
        "Soniox rejected the saved API key. Delete and re-save it in Settings.".to_string()
    } else {
        "QuickText could not complete transcription. Check your connection and try again."
            .to_string()
    };

    AppError::ProviderUnavailable {
        message,
        support_reference,
    }
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

fn is_invisible(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{200B}'..='\u{200F}' | '\u{00AD}' | '\u{FEFF}')
}

fn sanitize_api_key(api_key: &str) -> String {
    api_key.chars().filter(|c| !is_invisible(*c)).collect()
}

fn read_soniox_api_key_from_store() -> Result<Option<String>, String> {
    match soniox_key_entry()?.get_password() {
        Ok(api_key) => {
            let api_key = sanitize_api_key(&api_key);
            if api_key.is_empty() {
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
    toggle_recording_for_app(app, max_recording_seconds, false, TriggerSource::Ui).await
}

async fn run_shortcut_toggle(app: tauri::AppHandle) {
    let settings = app.state::<ShortcutSettings>();
    let controller = app.state::<AppControllerState>();
    let was_recording = lock_controller(&controller)
        .map(|controller| controller.snapshot().status == AppStatus::Recording)
        .unwrap_or(false);

    if !was_recording && shortcut_focuses_on_start(&settings) {
        show_main_window(&app);
    }

    let _ = toggle_recording_for_app(app.clone(), None, false, TriggerSource::Shortcut).await;

    if was_recording && shortcut_hides_on_stop(&settings) {
        hide_main_window(&app);
    }
}

pub(crate) async fn toggle_recording_for_app(
    app: tauri::AppHandle,
    max_recording_seconds: Option<u64>,
    focus_window: bool,
    trigger_source: TriggerSource,
) -> Result<AppSnapshot, String> {
    let controller = app.state::<AppControllerState>();
    let credentials = app.state::<CredentialState>();
    let recorder = app.state::<AudioRecorderState>();
    let transcription = app.state::<TranscriptionState>();
    let max_recording_seconds = max_recording_seconds
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_MAX_RECORDING_SECONDS);
    let current_status = lock_controller(&controller)?.snapshot().status;
    app.state::<AppLogger>().info(
        active_log_context(&app),
        InfoEvent::TriggerReceived {
            source: trigger_source,
        },
    );

    match current_status {
        AppStatus::Idle | AppStatus::Transcribed | AppStatus::Error => {
            let mut log_context = begin_log_context(&app);
            let microphone_setup_started = Instant::now();
            let starting = {
                let mut controller = lock_controller(&controller)?;
                controller.begin_start()
            };
            app.state::<AppLogger>().info(
                log_context,
                InfoEvent::StateChanged {
                    from: app_status_for_log(&current_status),
                    to: LoggedAppState::Starting,
                },
            );
            emit_app_snapshot(&app, &starting);

            if focus_window {
                show_main_window(&app);
            }

            let api_key = match get_soniox_api_key_available(&credentials) {
                Ok(Some(api_key)) => {
                    app.state::<AppLogger>().info(
                        log_context,
                        InfoEvent::OperationSucceeded {
                            operation: Operation::CredentialRead,
                        },
                    );
                    api_key
                }
                Ok(None) => {
                    let reference =
                        record_failure(&app, FailureCategory::MissingApiKey, "missing_api_key");
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(missing_api_key_error(reference))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
                Err(_) => {
                    let reference = record_failure(
                        &app,
                        FailureCategory::CredentialStore,
                        "credential_read_failed",
                    );
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(credential_store_error(reference))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };

            let audio_format = match AudioRecorder::input_format() {
                Ok(audio_format) => audio_format,
                Err(_) => {
                    let reference = record_failure(
                        &app,
                        FailureCategory::NoMicrophoneDevice,
                        "microphone_format_unavailable",
                    );
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(microphone_unavailable_error(reference))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };

            log_context = log_context.with_provider_session();
            set_log_context(&app, log_context);
            let provider_started = Instant::now();
            let mut soniox_session = SonioxSession::start(api_key, audio_format.clone());
            app.state::<AppLogger>()
                .info(log_context, InfoEvent::ProviderConnectionAttempt);
            let provider_ready_rx = soniox_session.take_ready_receiver();

            if let Some(partial_rx) = soniox_session.take_partial_receiver() {
                tauri::async_runtime::spawn(forward_partial_transcripts(app.clone(), partial_rx));
            }

            // Capture starts before the provider connects; chunks buffer in
            // the session channel until the connection is ready.
            let audio_recorder = match AudioRecorder::start(soniox_session.audio_sender()) {
                Ok(recorder) => recorder,
                Err(_) => {
                    soniox_session.cancel();
                    let reference = record_failure(
                        &app,
                        FailureCategory::NoMicrophoneDevice,
                        "microphone_start_failed",
                    );
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_start(microphone_unavailable_error(reference))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };
            app.state::<AppLogger>().debug(
                log_context,
                DebugEvent::Timing {
                    phase: DebugPhase::MicrophoneSetup,
                    elapsed_ms: u64::try_from(microphone_setup_started.elapsed().as_millis())
                        .unwrap_or(u64::MAX),
                },
            );
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
            let encoding = match recording
                .audio_format
                .as_ref()
                .map(|format| &format.encoding)
            {
                Some(audio_recorder::AudioEncoding::F32) => "f32",
                Some(audio_recorder::AudioEncoding::I16) => "i16",
                Some(audio_recorder::AudioEncoding::U16) => "u16",
                None => "unknown",
            };
            app.state::<AppLogger>().info(
                log_context,
                InfoEvent::RecordingStarted {
                    sample_rate: recording
                        .audio_format
                        .as_ref()
                        .map(|format| format.sample_rate)
                        .unwrap_or_default(),
                    channels: recording
                        .audio_format
                        .as_ref()
                        .map(|format| format.channels)
                        .unwrap_or_default(),
                    encoding,
                    device_fallback: false,
                },
            );
            app.state::<AppLogger>().info(
                log_context,
                InfoEvent::StateChanged {
                    from: LoggedAppState::Starting,
                    to: LoggedAppState::Recording,
                },
            );
            let session_id = lock_controller(&controller)?.active_session_id();
            emit_app_snapshot(&app, &recording);
            if let Some(session_id) = session_id {
                schedule_max_recording_duration(app.clone(), session_id, max_recording_seconds);
                if let Some(provider_ready_rx) = provider_ready_rx {
                    tauri::async_runtime::spawn(fail_recording_when_provider_unreachable(
                        app.clone(),
                        session_id,
                        provider_started,
                        provider_ready_rx,
                    ));
                }
            }
            Ok(recording)
        }
        AppStatus::Recording => {
            let log_context = active_log_context(&app);
            let stopping = {
                let mut controller = lock_controller(&controller)?;
                controller.begin_stop()
            };
            app.state::<AppLogger>().info(
                log_context,
                InfoEvent::StateChanged {
                    from: LoggedAppState::Recording,
                    to: LoggedAppState::Stopping,
                },
            );
            emit_app_snapshot(&app, &stopping);

            let audio_stats = stop_audio_recorder(&recorder)?;
            app.state::<AppLogger>().info(
                log_context,
                InfoEvent::RecordingStopped {
                    duration_ms: u64::try_from(audio_stats.elapsed_ms).unwrap_or(u64::MAX),
                    chunk_count: audio_stats.chunk_count,
                    sample_count: audio_stats.sample_count,
                    byte_count: audio_stats.byte_count,
                },
            );
            app.state::<AppLogger>()
                .info(log_context, InfoEvent::ProviderFinalizing);
            let finalize_started = Instant::now();
            let transcript = match stop_transcription_session(&transcription).await {
                Ok(transcript) => transcript,
                Err(error) => {
                    let reference = record_failure(
                        &app,
                        FailureCategory::ProviderUnavailable,
                        "provider_finalize_failed",
                    );
                    let error_snapshot = {
                        let mut controller = lock_controller(&controller)?;
                        controller.fail_stop(provider_unavailable_error(&error, reference))
                    };
                    emit_app_snapshot(&app, &error_snapshot);
                    return Ok(error_snapshot);
                }
            };
            app.state::<AppLogger>().debug(
                log_context,
                DebugEvent::Timing {
                    phase: DebugPhase::ProviderFinalize,
                    elapsed_ms: u64::try_from(finalize_started.elapsed().as_millis())
                        .unwrap_or(u64::MAX),
                },
            );
            app.state::<AppLogger>()
                .info(log_context, InfoEvent::ProviderCompleted);
            app.state::<AppLogger>().info(
                log_context,
                InfoEvent::StateChanged {
                    from: LoggedAppState::Stopping,
                    to: LoggedAppState::Transcribed,
                },
            );
            clear_log_context(&app);

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
    logger: State<'_, AppLogger>,
    shortcut: String,
) -> Result<String, String> {
    let shortcut = shortcut.trim();

    if shortcut.is_empty() {
        return Err("Choose a shortcut first.".into());
    }

    if let Err(error) = app.global_shortcut().unregister_all() {
        logger.warn(
            LogContext::default(),
            WarnEvent::OperationFailed {
                operation: Operation::ShortcutRegistration,
            },
        );
        return Err(format!("Could not clear the previous shortcut: {error}"));
    }

    if let Err(error) = app.global_shortcut().register(shortcut) {
        logger.warn(
            LogContext::default(),
            WarnEvent::OperationFailed {
                operation: Operation::ShortcutRegistration,
            },
        );
        return Err(format!("Could not register shortcut: {error}"));
    }

    let mut active_shortcut = settings
        .active_shortcut
        .lock()
        .map_err(|_| "Could not update shortcut state.".to_string())?;

    *active_shortcut = Some(shortcut.to_string());
    logger.info(
        LogContext::default(),
        InfoEvent::OperationSucceeded {
            operation: Operation::ShortcutRegistration,
        },
    );

    Ok(shortcut.to_string())
}

#[tauri::command]
fn set_shortcut_behavior(
    settings: State<'_, ShortcutSettings>,
    logger: State<'_, AppLogger>,
    focus_on_start: bool,
    hide_on_stop: bool,
) -> Result<(), String> {
    let mut focus = settings
        .focus_on_start
        .lock()
        .map_err(|_| "Could not update shortcut state.".to_string())?;
    *focus = focus_on_start;

    let mut hide = settings
        .hide_on_stop
        .lock()
        .map_err(|_| "Could not update shortcut state.".to_string())?;
    *hide = hide_on_stop;

    logger.info(
        LogContext::default(),
        InfoEvent::OperationSucceeded {
            operation: Operation::SettingsUpdate,
        },
    );

    Ok(())
}

fn shortcut_focuses_on_start(settings: &ShortcutSettings) -> bool {
    settings
        .focus_on_start
        .lock()
        .map(|focus| *focus)
        .unwrap_or(true)
}

fn shortcut_hides_on_stop(settings: &ShortcutSettings) -> bool {
    settings
        .hide_on_stop
        .lock()
        .map(|hide| *hide)
        .unwrap_or(false)
}

#[tauri::command]
fn has_soniox_api_key(
    credentials: State<'_, CredentialState>,
    logger: State<'_, AppLogger>,
) -> Result<bool, String> {
    let result = has_soniox_api_key_available(&credentials);
    match &result {
        Ok(_) => logger.info(
            LogContext::default(),
            InfoEvent::OperationSucceeded {
                operation: Operation::CredentialRead,
            },
        ),
        Err(_) => logger.warn(
            LogContext::default(),
            WarnEvent::OperationFailed {
                operation: Operation::CredentialRead,
            },
        ),
    }
    result
}

#[tauri::command]
fn save_soniox_api_key(
    credentials: State<'_, CredentialState>,
    logger: State<'_, AppLogger>,
    api_key: String,
) -> Result<bool, String> {
    let api_key = sanitize_api_key(&api_key);

    if api_key.is_empty() {
        return Err("Enter a Soniox API key first.".into());
    }

    let result = soniox_key_entry().and_then(|entry| {
        entry
            .set_password(api_key.as_str())
            .map_err(|error| format!("Could not save Soniox API key: {error}"))
    });
    if let Err(error) = result {
        logger.warn(
            LogContext::default(),
            WarnEvent::OperationFailed {
                operation: Operation::CredentialSave,
            },
        );
        return Err(error);
    }

    cache_soniox_api_key(&credentials, api_key.as_str())?;
    logger.info(
        LogContext::default(),
        InfoEvent::OperationSucceeded {
            operation: Operation::CredentialSave,
        },
    );
    Ok(true)
}

#[tauri::command]
fn delete_soniox_api_key(
    credentials: State<'_, CredentialState>,
    logger: State<'_, AppLogger>,
) -> Result<(), String> {
    clear_cached_soniox_api_key(&credentials)?;

    let entry = match soniox_key_entry() {
        Ok(entry) => entry,
        Err(error) => {
            logger.warn(
                LogContext::default(),
                WarnEvent::OperationFailed {
                    operation: Operation::CredentialDelete,
                },
            );
            return Err(error);
        }
    };

    match entry.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => {
            logger.info(
                LogContext::default(),
                InfoEvent::OperationSucceeded {
                    operation: Operation::CredentialDelete,
                },
            );
            Ok(())
        }
        Err(error) => {
            logger.warn(
                LogContext::default(),
                WarnEvent::OperationFailed {
                    operation: Operation::CredentialDelete,
                },
            );
            Err(format!("Could not delete Soniox API key: {error}"))
        }
    }
}

#[tauri::command]
fn copy_text_to_clipboard(
    app: tauri::AppHandle,
    logger: State<'_, AppLogger>,
    text: String,
) -> Result<(), String> {
    match app.clipboard().write_text(text) {
        Ok(()) => {
            logger.info(
                LogContext::default(),
                InfoEvent::OperationSucceeded {
                    operation: Operation::ClipboardWrite,
                },
            );
            Ok(())
        }
        Err(_) => {
            logger.warn(
                LogContext::default(),
                WarnEvent::OperationFailed {
                    operation: Operation::ClipboardWrite,
                },
            );
            Err("QuickText could not write to the clipboard.".to_string())
        }
    }
}

#[tauri::command]
fn get_launch_on_startup(app: tauri::AppHandle) -> Result<bool, String> {
    autostart::is_enabled(&app)
}

#[tauri::command]
fn set_launch_on_startup(
    app: tauri::AppHandle,
    logger: State<'_, AppLogger>,
    enabled: bool,
) -> Result<bool, String> {
    let result = autostart::set_enabled(&app, enabled);
    match &result {
        Ok(_) => logger.info(
            LogContext::default(),
            InfoEvent::OperationSucceeded {
                operation: Operation::AutostartUpdate,
            },
        ),
        Err(_) => logger.warn(
            LogContext::default(),
            WarnEvent::OperationFailed {
                operation: Operation::AutostartUpdate,
            },
        ),
    }
    result
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LoggingStatus {
    debug_seconds_remaining: u64,
}

#[tauri::command]
fn get_logging_status(logger: State<'_, AppLogger>) -> Result<LoggingStatus, String> {
    Ok(LoggingStatus {
        debug_seconds_remaining: logger.debug_seconds_remaining()?,
    })
}

#[tauri::command]
fn enable_temporary_debug_logging(logger: State<'_, AppLogger>) -> Result<LoggingStatus, String> {
    let seconds = logger.enable_debug()?;
    logger.info(
        LogContext::default(),
        InfoEvent::OperationSucceeded {
            operation: Operation::DebugLogging,
        },
    );
    Ok(LoggingStatus {
        debug_seconds_remaining: seconds,
    })
}

#[tauri::command]
async fn export_logs(app: tauri::AppHandle, logger: State<'_, AppLogger>) -> Result<bool, String> {
    let dialog_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter("ZIP archive", &["zip"])
            .set_file_name("quicktext-diagnostics.zip")
            .blocking_save_file()
    })
    .await
    .map_err(|_| "The diagnostics save dialog stopped unexpectedly.".to_string())?;
    let Some(selected) = selected else {
        return Ok(false);
    };
    let mut destination = selected
        .into_path()
        .map_err(|_| "The selected diagnostics destination is not a local file.".to_string())?;
    if destination.extension().is_none() {
        destination.set_extension("zip");
    }

    let logger = logger.inner().clone();
    let export_logger = logger.clone();
    let result = tauri::async_runtime::spawn_blocking(move || export_logger.export(destination))
        .await
        .map_err(|_| "The diagnostics export task stopped unexpectedly.".to_string())?;
    match result {
        Ok(()) => {
            logger.info(
                LogContext::default(),
                InfoEvent::OperationSucceeded {
                    operation: Operation::Export,
                },
            );
            Ok(true)
        }
        Err(error) => {
            logger.warn(
                LogContext::default(),
                WarnEvent::OperationFailed {
                    operation: Operation::Export,
                },
            );
            Err(error)
        }
    }
}

#[tauri::command]
async fn delete_local_logs(logger: State<'_, AppLogger>) -> Result<(), String> {
    let logger = logger.inner().clone();
    let delete_logger = logger.clone();
    let result = tauri::async_runtime::spawn_blocking(move || delete_logger.delete_local_logs())
        .await
        .map_err(|_| "The local log deletion task stopped unexpectedly.".to_string())?;
    match result {
        Ok(()) => {
            logger.info(
                LogContext::default(),
                InfoEvent::OperationSucceeded {
                    operation: Operation::DeleteLogs,
                },
            );
            Ok(())
        }
        Err(error) => {
            logger.warn(
                LogContext::default(),
                WarnEvent::OperationFailed {
                    operation: Operation::DeleteLogs,
                },
            );
            Err(error)
        }
    }
}

pub fn entry() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let start_hidden = autostart::is_autostart_launch(&args);

    if !args.is_empty() && !start_hidden {
        return companion_cli::run(&args);
    }

    run_with_options(start_hidden);
    0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_with_options(false);
}

fn run_with_options(start_hidden: bool) {
    tauri::Builder::default()
        .manage(ShortcutSettings::default())
        .manage(AppControllerState::default())
        .manage(CredentialState::default())
        .manage(AudioRecorderState::default())
        .manage(TranscriptionState::default())
        .manage(LoggingContextState::default())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(autostart::plugin())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            run_shortcut_toggle(app).await;
                        });
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let log_dir = app.path().app_log_dir()?;
            let locale = std::env::var("LANG").unwrap_or_else(|_| "unknown".to_string());
            let build_info = BuildInfo::current(app.package_info().version.to_string(), locale);
            let logger =
                AppLogger::start(log_dir, build_info.clone()).map_err(std::io::Error::other)?;
            logger.info(
                LogContext::default(),
                InfoEvent::AppStarted {
                    app_version: build_info.app_version,
                    build_id: build_info.build_id,
                    source_revision: build_info.source_revision,
                    os_family: build_info.os_family,
                    os_version: build_info.os_version,
                    architecture: build_info.architecture,
                },
            );
            app.manage(logger);

            setup_tray(app)?;
            app.state::<AppLogger>().info(
                LogContext::default(),
                InfoEvent::OperationSucceeded {
                    operation: Operation::TrayUpdate,
                },
            );

            if start_hidden {
                hide_main_window(app.handle());
            }

            #[cfg(unix)]
            match ipc_server::start(app.handle().clone()) {
                Ok(ipc_server::ServerStart::Listening) => {
                    app.state::<AppLogger>().info(
                        LogContext::default(),
                        InfoEvent::OperationSucceeded {
                            operation: Operation::IpcListener,
                        },
                    );
                }
                Ok(ipc_server::ServerStart::AlreadyRunning) => {
                    app.state::<AppLogger>().warn(
                        LogContext::default(),
                        WarnEvent::OperationFailed {
                            operation: Operation::IpcListener,
                        },
                    );
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
            set_shortcut_behavior,
            has_soniox_api_key,
            save_soniox_api_key,
            delete_soniox_api_key,
            copy_text_to_clipboard,
            get_launch_on_startup,
            set_launch_on_startup,
            get_logging_status,
            enable_temporary_debug_logging,
            export_logs,
            delete_local_logs
        ])
        .build(tauri::generate_context!())
        .expect("error while building Tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(logger) = app.try_state::<AppLogger>() {
                    logger.info(LogContext::default(), InfoEvent::AppShutdown);
                    let _ = logger.shutdown();
                }
            }
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
