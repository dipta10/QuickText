use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime},
};

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

const SCHEMA_VERSION: u16 = 1;
const ACTIVE_LOG_NAME: &str = "quicktext-current.jsonl";
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FILES: usize = 5;
const MAX_FILE_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);
const DEBUG_DURATION: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Debug)]
pub struct BuildInfo {
    pub app_version: String,
    pub build_id: String,
    pub source_revision: String,
    pub os_family: String,
    pub os_version: String,
    pub architecture: String,
    pub locale: String,
}

impl BuildInfo {
    pub fn current(app_version: impl Into<String>, locale: impl Into<String>) -> Self {
        let os = os_info::get();
        Self {
            app_version: app_version.into(),
            build_id: option_env!("QUICKTEXT_BUILD_ID")
                .unwrap_or("development")
                .to_string(),
            source_revision: option_env!("QUICKTEXT_SOURCE_REVISION")
                .unwrap_or("unknown")
                .to_string(),
            os_family: std::env::consts::OS.to_string(),
            os_version: os.version().to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            locale: locale.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LogContext {
    pub recording_session_id: Option<Uuid>,
    pub provider_session_id: Option<Uuid>,
    pub error_id: Option<Uuid>,
}

impl LogContext {
    pub fn recording() -> Self {
        Self {
            recording_session_id: Some(Uuid::new_v4()),
            ..Self::default()
        }
    }

    pub fn with_provider_session(mut self) -> Self {
        self.provider_session_id = Some(Uuid::new_v4());
        self
    }

    pub fn with_error(mut self) -> Self {
        self.error_id = Some(Uuid::new_v4());
        self
    }

    pub fn support_reference(&self) -> Option<String> {
        self.error_id.map(support_reference)
    }
}

pub fn support_reference(error_id: Uuid) -> String {
    let compact = error_id.simple().to_string().to_uppercase();
    format!("QT-{}", &compact[..12])
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    Ui,
    Shortcut,
    Ipc,
    DurationLimit,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoggedAppState {
    Idle,
    Starting,
    Recording,
    Stopping,
    Transcribed,
    Error,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    ShortcutRegistration,
    CredentialRead,
    CredentialSave,
    CredentialDelete,
    ClipboardWrite,
    TrayUpdate,
    IpcListener,
    Export,
    DeleteLogs,
    DebugLogging,
    SettingsUpdate,
    AutostartUpdate,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum FailureCategory {
    MissingApiKey,
    InvalidApiKey,
    CredentialStore,
    MicrophonePermissionDenied,
    NoMicrophoneDevice,
    NetworkUnavailable,
    ProviderUnavailable,
    ProviderTimeout,
    EmptyAudio,
    Internal,
}

#[derive(Clone, Debug)]
pub enum DebugEvent {
    Timing { phase: DebugPhase, elapsed_ms: u64 },
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugPhase {
    MicrophoneSetup,
    ProviderConnect,
    ProviderFinalize,
}

#[derive(Clone, Debug)]
pub enum InfoEvent {
    AppStarted {
        app_version: String,
        build_id: String,
        source_revision: String,
        os_family: String,
        os_version: String,
        architecture: String,
    },
    AppShutdown,
    TriggerReceived {
        source: TriggerSource,
    },
    StateChanged {
        from: LoggedAppState,
        to: LoggedAppState,
    },
    RecordingStarted {
        sample_rate: u32,
        channels: u16,
        encoding: &'static str,
        device_fallback: bool,
    },
    RecordingStopped {
        duration_ms: u64,
        chunk_count: u64,
        sample_count: u64,
        byte_count: u64,
    },
    ProviderConnectionAttempt,
    ProviderConnected,
    ProviderFinalizing,
    ProviderCompleted,
    OperationSucceeded {
        operation: Operation,
    },
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum WarnEvent {
    OperationFailed { operation: Operation },
    ProviderRetry { attempt: u16 },
    DeviceFallback,
}

#[derive(Clone, Debug)]
pub enum ErrorEvent {
    Failure {
        category: FailureCategory,
        code: &'static str,
    },
}

pub trait Logger: Send + Sync {
    fn debug(&self, context: LogContext, event: DebugEvent);
    fn info(&self, context: LogContext, event: InfoEvent);
    fn warn(&self, context: LogContext, event: WarnEvent);
    fn error(&self, context: LogContext, event: ErrorEvent);
}

#[derive(Clone)]
pub struct AppLogger {
    run_id: Uuid,
    sender: Sender<WriterRequest>,
}

impl AppLogger {
    pub fn start(log_dir: PathBuf, build_info: BuildInfo) -> Result<Self, String> {
        fs::create_dir_all(&log_dir)
            .map_err(|_| "Could not create the QuickText log directory.".to_string())?;

        let run_id = Uuid::new_v4();
        let (sender, receiver) = mpsc::channel();
        let mut writer = WriterState::open(log_dir, build_info)?;
        thread::Builder::new()
            .name("quicktext-logger".to_string())
            .spawn(move || writer.run(receiver))
            .map_err(|_| "Could not start the QuickText logger.".to_string())?;

        Ok(Self { run_id, sender })
    }

    pub fn enable_debug(&self) -> Result<u64, String> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(WriterRequest::EnableDebug { reply })
            .map_err(|_| "The QuickText logger is unavailable.".to_string())?;
        receiver
            .recv()
            .map_err(|_| "The QuickText logger did not respond.".to_string())?
    }

    pub fn debug_seconds_remaining(&self) -> Result<u64, String> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(WriterRequest::DebugStatus { reply })
            .map_err(|_| "The QuickText logger is unavailable.".to_string())?;
        receiver
            .recv()
            .map_err(|_| "The QuickText logger did not respond.".to_string())
    }

    pub fn export(&self, destination: PathBuf) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(WriterRequest::Export { destination, reply })
            .map_err(|_| "The QuickText logger is unavailable.".to_string())?;
        receiver
            .recv()
            .map_err(|_| "The QuickText logger did not respond.".to_string())?
    }

    pub fn delete_local_logs(&self) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(WriterRequest::Delete { reply })
            .map_err(|_| "The QuickText logger is unavailable.".to_string())?;
        receiver
            .recv()
            .map_err(|_| "The QuickText logger did not respond.".to_string())?
    }

    #[cfg(test)]
    pub fn flush(&self) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(WriterRequest::Flush { reply })
            .map_err(|_| "The QuickText logger is unavailable.".to_string())?;
        receiver
            .recv()
            .map_err(|_| "The QuickText logger did not respond.".to_string())?
    }

    pub fn shutdown(&self) -> Result<(), String> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(WriterRequest::Shutdown { reply })
            .map_err(|_| "The QuickText logger is unavailable.".to_string())?;
        receiver
            .recv()
            .map_err(|_| "The QuickText logger did not respond.".to_string())?
    }

    fn send(&self, level: Level, context: LogContext, event: EventData) {
        let _ = self.sender.send(WriterRequest::Event(LogRecord {
            schema_version: SCHEMA_VERSION,
            timestamp_utc: now_utc(),
            level,
            component: event.component,
            event: event.name,
            run_id: self.run_id,
            recording_session_id: context.recording_session_id,
            provider_session_id: context.provider_session_id,
            error_id: context.error_id,
            metadata: event.metadata,
        }));
    }
}

impl Logger for AppLogger {
    fn debug(&self, context: LogContext, event: DebugEvent) {
        self.send(Level::Debug, context, event.into());
    }

    fn info(&self, context: LogContext, event: InfoEvent) {
        self.send(Level::Info, context, event.into());
    }

    fn warn(&self, context: LogContext, event: WarnEvent) {
        self.send(Level::Warn, context, event.into());
    }

    fn error(&self, context: LogContext, event: ErrorEvent) {
        self.send(Level::Error, context, event.into());
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

struct EventData {
    component: &'static str,
    name: &'static str,
    metadata: Value,
}

impl From<DebugEvent> for EventData {
    fn from(event: DebugEvent) -> Self {
        match event {
            DebugEvent::Timing { phase, elapsed_ms } => Self {
                component: "app_controller",
                name: "timing",
                metadata: json!({ "phase": phase, "elapsed_ms": elapsed_ms }),
            },
        }
    }
}

impl From<InfoEvent> for EventData {
    fn from(event: InfoEvent) -> Self {
        match event {
            InfoEvent::AppStarted {
                app_version,
                build_id,
                source_revision,
                os_family,
                os_version,
                architecture,
            } => Self {
                component: "app",
                name: "started",
                metadata: json!({
                    "app_version": app_version,
                    "build_id": build_id,
                    "source_revision": source_revision,
                    "os_family": os_family,
                    "os_version": os_version,
                    "architecture": architecture,
                }),
            },
            InfoEvent::AppShutdown => Self {
                component: "app",
                name: "shutdown",
                metadata: json!({}),
            },
            InfoEvent::TriggerReceived { source } => Self {
                component: "app_controller",
                name: "trigger_received",
                metadata: json!({ "source": source }),
            },
            InfoEvent::StateChanged { from, to } => Self {
                component: "app_controller",
                name: "state_changed",
                metadata: json!({ "from": from, "to": to }),
            },
            InfoEvent::RecordingStarted {
                sample_rate,
                channels,
                encoding,
                device_fallback,
            } => Self {
                component: "audio",
                name: "recording_started",
                metadata: json!({
                    "sample_rate": sample_rate,
                    "channels": channels,
                    "encoding": encoding,
                    "device_fallback": device_fallback,
                }),
            },
            InfoEvent::RecordingStopped {
                duration_ms,
                chunk_count,
                sample_count,
                byte_count,
            } => Self {
                component: "audio",
                name: "recording_stopped",
                metadata: json!({
                    "duration_ms": duration_ms,
                    "chunk_count": chunk_count,
                    "sample_count": sample_count,
                    "byte_count": byte_count,
                }),
            },
            InfoEvent::ProviderConnectionAttempt => Self {
                component: "transcription_provider",
                name: "connection_attempt",
                metadata: json!({}),
            },
            InfoEvent::ProviderConnected => Self {
                component: "transcription_provider",
                name: "connected",
                metadata: json!({}),
            },
            InfoEvent::ProviderFinalizing => Self {
                component: "transcription_provider",
                name: "finalizing",
                metadata: json!({}),
            },
            InfoEvent::ProviderCompleted => Self {
                component: "transcription_provider",
                name: "completed",
                metadata: json!({}),
            },
            InfoEvent::OperationSucceeded { operation } => Self {
                component: "app",
                name: "operation_succeeded",
                metadata: json!({ "operation": operation }),
            },
        }
    }
}

impl From<WarnEvent> for EventData {
    fn from(event: WarnEvent) -> Self {
        match event {
            WarnEvent::OperationFailed { operation } => Self {
                component: "app",
                name: "operation_failed",
                metadata: json!({ "operation": operation }),
            },
            WarnEvent::ProviderRetry { attempt } => Self {
                component: "transcription_provider",
                name: "retry",
                metadata: json!({ "attempt": attempt }),
            },
            WarnEvent::DeviceFallback => Self {
                component: "audio",
                name: "device_fallback",
                metadata: json!({}),
            },
        }
    }
}

impl From<ErrorEvent> for EventData {
    fn from(event: ErrorEvent) -> Self {
        match event {
            ErrorEvent::Failure { category, code } => Self {
                component: "app_controller",
                name: "failure",
                metadata: json!({ "category": category, "code": code }),
            },
        }
    }
}

#[derive(Serialize)]
struct LogRecord {
    schema_version: u16,
    timestamp_utc: String,
    level: Level,
    component: &'static str,
    event: &'static str,
    run_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    recording_session_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_session_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_id: Option<Uuid>,
    metadata: Value,
}

enum WriterRequest {
    Event(LogRecord),
    EnableDebug {
        reply: Sender<Result<u64, String>>,
    },
    DebugStatus {
        reply: Sender<u64>,
    },
    Export {
        destination: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    Delete {
        reply: Sender<Result<(), String>>,
    },
    #[cfg(test)]
    Flush {
        reply: Sender<Result<(), String>>,
    },
    Shutdown {
        reply: Sender<Result<(), String>>,
    },
}

struct WriterState {
    log_dir: PathBuf,
    active: Option<BufWriter<File>>,
    active_bytes: u64,
    debug_until: Option<Instant>,
    build_info: BuildInfo,
}

impl WriterState {
    fn open(log_dir: PathBuf, build_info: BuildInfo) -> Result<Self, String> {
        cleanup_logs(&log_dir)?;
        let (active, active_bytes) = open_active_file(&log_dir)?;
        Ok(Self {
            log_dir,
            active: Some(active),
            active_bytes,
            debug_until: None,
            build_info,
        })
    }

    fn run(&mut self, receiver: Receiver<WriterRequest>) {
        while let Ok(request) = receiver.recv() {
            match request {
                WriterRequest::Event(record) => {
                    if matches!(record.level, Level::Debug) && !self.debug_enabled() {
                        continue;
                    }
                    let should_flush = matches!(record.level, Level::Warn | Level::Error);
                    if self.write(record).is_ok() && should_flush {
                        let _ = self.flush();
                    }
                }
                WriterRequest::EnableDebug { reply } => {
                    self.debug_until = Some(Instant::now() + DEBUG_DURATION);
                    let _ = reply.send(Ok(DEBUG_DURATION.as_secs()));
                }
                WriterRequest::DebugStatus { reply } => {
                    let _ = reply.send(self.debug_remaining());
                }
                WriterRequest::Export { destination, reply } => {
                    let _ = reply.send(self.export(&destination));
                }
                WriterRequest::Delete { reply } => {
                    let _ = reply.send(self.delete());
                }
                #[cfg(test)]
                WriterRequest::Flush { reply } => {
                    let _ = reply.send(self.flush());
                }
                WriterRequest::Shutdown { reply } => {
                    let result = self.flush();
                    self.active.take();
                    let _ = reply.send(result);
                    break;
                }
            }
        }
        let _ = self.flush();
    }

    fn debug_enabled(&mut self) -> bool {
        if self.debug_remaining() == 0 {
            self.debug_until = None;
            false
        } else {
            true
        }
    }

    fn debug_remaining(&self) -> u64 {
        self.debug_until
            .map(|until| until.saturating_duration_since(Instant::now()).as_secs())
            .unwrap_or(0)
    }

    fn write(&mut self, record: LogRecord) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(&record)
            .map_err(|_| "Could not encode a QuickText log event.".to_string())?;
        bytes.push(b'\n');
        if self.active_bytes > 0 && self.active_bytes + bytes.len() as u64 > MAX_FILE_BYTES {
            self.rotate()?;
        }
        self.active
            .as_mut()
            .ok_or_else(|| "The QuickText log file is unavailable.".to_string())?
            .write_all(&bytes)
            .map_err(|_| "Could not write a QuickText log event.".to_string())?;
        self.active_bytes += bytes.len() as u64;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        self.active
            .as_mut()
            .ok_or_else(|| "The QuickText log file is unavailable.".to_string())?
            .flush()
            .map_err(|_| "Could not flush the QuickText log file.".to_string())
    }

    fn rotate(&mut self) -> Result<(), String> {
        self.flush()?;
        self.active.take();
        let rotated_name = format!(
            "quicktext-{}-{}.jsonl",
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ"),
            &Uuid::new_v4().simple().to_string()[..8]
        );
        fs::rename(
            self.log_dir.join(ACTIVE_LOG_NAME),
            self.log_dir.join(rotated_name),
        )
        .map_err(|_| "Could not rotate the QuickText log file.".to_string())?;
        let (active, active_bytes) = open_active_file(&self.log_dir)?;
        self.active = Some(active);
        self.active_bytes = active_bytes;
        cleanup_logs(&self.log_dir)?;
        Ok(())
    }

    fn export(&mut self, destination: &Path) -> Result<(), String> {
        self.flush()?;
        let logs = log_files(&self.log_dir)?;
        let file = File::create(destination)
            .map_err(|_| "Could not create the diagnostics archive.".to_string())?;
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        for path in logs {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            archive
                .start_file(name, options)
                .map_err(|_| "Could not add a log file to the diagnostics archive.".to_string())?;
            let mut source = File::open(&path)
                .map_err(|_| "Could not read a log file for export.".to_string())?;
            std::io::copy(&mut source, &mut archive).map_err(|_| {
                "Could not copy a log file into the diagnostics archive.".to_string()
            })?;
        }

        archive
            .start_file("manifest.json", options)
            .map_err(|_| "Could not add the diagnostics manifest.".to_string())?;
        let manifest = json!({
            "diagnostics_schema_version": SCHEMA_VERSION,
            "quicktext_version": self.build_info.app_version,
            "build_id": self.build_info.build_id,
            "source_revision": self.build_info.source_revision,
            "os_family": self.build_info.os_family,
            "os_version": self.build_info.os_version,
            "architecture": self.build_info.architecture,
            "locale": self.build_info.locale,
            "exported_at_utc": now_utc(),
        });
        archive
            .write_all(
                serde_json::to_string_pretty(&manifest)
                    .map_err(|_| "Could not encode the diagnostics manifest.".to_string())?
                    .as_bytes(),
            )
            .map_err(|_| "Could not write the diagnostics manifest.".to_string())?;
        archive
            .finish()
            .map_err(|_| "Could not finish the diagnostics archive.".to_string())?;
        Ok(())
    }

    fn delete(&mut self) -> Result<(), String> {
        self.flush()?;
        self.active.take();
        for path in log_files(&self.log_dir)? {
            fs::remove_file(path)
                .map_err(|_| "Could not delete a QuickText log file.".to_string())?;
        }
        let (active, active_bytes) = open_active_file(&self.log_dir)?;
        self.active = Some(active);
        self.active_bytes = active_bytes;
        Ok(())
    }
}

fn open_active_file(log_dir: &Path) -> Result<(BufWriter<File>, u64), String> {
    let path = log_dir.join(ACTIVE_LOG_NAME);
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|_| "Could not open the QuickText log file.".to_string())?;
    let bytes = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    Ok((BufWriter::new(file), bytes))
}

fn cleanup_logs(log_dir: &Path) -> Result<(), String> {
    let now = SystemTime::now();
    let mut files = log_files(log_dir)?;
    for path in &files {
        let expired = fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .is_some_and(|modified| is_expired(now, modified));
        if expired {
            fs::remove_file(path)
                .map_err(|_| "Could not remove an expired QuickText log file.".to_string())?;
        }
    }

    files = log_files(log_dir)?;
    while files.len() > MAX_FILES {
        let removable = files
            .iter()
            .position(|path| path.file_name().is_some_and(|name| name != ACTIVE_LOG_NAME));
        let Some(index) = removable else { break };
        let path = files.remove(index);
        fs::remove_file(path)
            .map_err(|_| "Could not prune an old QuickText log file.".to_string())?;
    }
    Ok(())
}

fn is_expired(now: SystemTime, modified: SystemTime) -> bool {
    now.duration_since(modified)
        .is_ok_and(|age| age > MAX_FILE_AGE)
}

fn log_files(log_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(log_dir)
        .map_err(|_| "Could not inspect the QuickText log directory.".to_string())?;
    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("quicktext-") && name.ends_with(".jsonl"))
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    Ok(files)
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("quicktext-logger-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn build_info() -> BuildInfo {
        BuildInfo {
            app_version: "0.1.0".into(),
            build_id: "test-build".into(),
            source_revision: "test-revision".into(),
            os_family: "test-os".into(),
            os_version: "1".into(),
            architecture: "test-arch".into(),
            locale: "en-US".into(),
        }
    }

    fn app_started() -> InfoEvent {
        let build = build_info();
        InfoEvent::AppStarted {
            app_version: build.app_version,
            build_id: build.build_id,
            source_revision: build.source_revision,
            os_family: build.os_family,
            os_version: build.os_version,
            architecture: build.architecture,
        }
    }

    fn test_record() -> LogRecord {
        LogRecord {
            schema_version: SCHEMA_VERSION,
            timestamp_utc: now_utc(),
            level: Level::Info,
            component: "test",
            event: "event",
            run_id: Uuid::new_v4(),
            recording_session_id: None,
            provider_session_id: None,
            error_id: None,
            metadata: json!({}),
        }
    }

    #[test]
    fn writes_structured_json_lines_without_free_form_messages() {
        let dir = temp_log_dir();
        let logger = AppLogger::start(dir.clone(), build_info()).unwrap();
        let context = LogContext::recording().with_provider_session();
        logger.info(context, InfoEvent::ProviderConnectionAttempt);
        logger.flush().unwrap();

        let text = fs::read_to_string(dir.join(ACTIVE_LOG_NAME)).unwrap();
        let record: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(record["level"], "info");
        assert_eq!(record["event"], "connection_attempt");
        assert!(record["recording_session_id"].is_string());
        assert!(record["provider_session_id"].is_string());
        assert!(record.get("message").is_none());
        logger.shutdown().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn debug_events_are_dropped_until_temporary_debug_is_enabled() {
        let dir = temp_log_dir();
        let logger = AppLogger::start(dir.clone(), build_info()).unwrap();
        logger.debug(
            LogContext::default(),
            DebugEvent::Timing {
                phase: DebugPhase::ProviderConnect,
                elapsed_ms: 5,
            },
        );
        logger.flush().unwrap();
        assert_eq!(fs::read_to_string(dir.join(ACTIVE_LOG_NAME)).unwrap(), "");

        assert_eq!(logger.enable_debug().unwrap(), DEBUG_DURATION.as_secs());
        logger.debug(
            LogContext::default(),
            DebugEvent::Timing {
                phase: DebugPhase::ProviderConnect,
                elapsed_ms: 5,
            },
        );
        logger.flush().unwrap();
        assert!(fs::read_to_string(dir.join(ACTIVE_LOG_NAME))
            .unwrap()
            .contains("\"level\":\"debug\""));
        logger.shutdown().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn delete_closes_files_and_starts_a_fresh_log() {
        let dir = temp_log_dir();
        let logger = AppLogger::start(dir.clone(), build_info()).unwrap();
        logger.info(LogContext::default(), app_started());
        logger.delete_local_logs().unwrap();
        logger.info(LogContext::default(), InfoEvent::AppShutdown);
        logger.flush().unwrap();

        let text = fs::read_to_string(dir.join(ACTIVE_LOG_NAME)).unwrap();
        assert!(!text.contains("\"event\":\"started\""));
        assert!(text.contains("\"event\":\"shutdown\""));
        logger.shutdown().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn exports_logs_and_safe_manifest() {
        let dir = temp_log_dir();
        let archive_path = dir.join("support.zip");
        let logger = AppLogger::start(dir.clone(), build_info()).unwrap();
        logger.info(LogContext::default(), app_started());
        logger.export(archive_path.clone()).unwrap();

        let file = File::open(archive_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name(ACTIVE_LOG_NAME).is_ok());
        let manifest = archive.by_name("manifest.json").unwrap();
        assert!(manifest.size() > 0);
        drop(manifest);
        drop(archive);
        logger.shutdown().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn support_reference_uses_twelve_uppercase_hex_characters() {
        let id = Uuid::parse_str("7f3a91c2-d4e8-4000-8000-000000000000").unwrap();
        assert_eq!(support_reference(id), "QT-7F3A91C2D4E8");
    }

    #[test]
    fn rotates_before_an_event_would_cross_the_file_limit() {
        let dir = temp_log_dir();
        let mut writer = WriterState::open(dir.clone(), build_info()).unwrap();
        writer.active_bytes = MAX_FILE_BYTES;
        writer.write(test_record()).unwrap();
        writer.flush().unwrap();

        let files = log_files(&dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with(ACTIVE_LOG_NAME)));
        writer.active.take();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cleanup_retains_only_the_newest_five_log_files() {
        let dir = temp_log_dir();
        for index in 0..7 {
            File::create(dir.join(format!("quicktext-{index}.jsonl"))).unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }

        cleanup_logs(&dir).unwrap();
        assert_eq!(log_files(&dir).unwrap().len(), MAX_FILES);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn age_boundary_expires_only_files_older_than_fourteen_days() {
        let modified = SystemTime::UNIX_EPOCH;
        assert!(!is_expired(modified + MAX_FILE_AGE, modified));
        assert!(is_expired(
            modified + MAX_FILE_AGE + Duration::from_secs(1),
            modified
        ));
    }
}
