use serde::{Deserialize, Serialize};

pub const IPC_PROTOCOL_VERSION: u32 = 1;
pub const CONNECT_TIMEOUT_SECONDS: u64 = 2;
pub const RESPONSE_TIMEOUT_SECONDS: u64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcCommand {
    Toggle,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcRequest {
    v: u32,
    cmd: IpcCommand,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    focus: bool,
}

impl IpcRequest {
    pub fn new(cmd: IpcCommand) -> Self {
        Self {
            v: IPC_PROTOCOL_VERSION,
            cmd,
            focus: false,
        }
    }

    pub fn new_focus(cmd: IpcCommand) -> Self {
        Self {
            v: IPC_PROTOCOL_VERSION,
            cmd,
            focus: true,
        }
    }

    pub fn parse(line: &str) -> Result<Self, String> {
        let request: IpcRequest = serde_json::from_str(line)
            .map_err(|error| format!("Malformed IPC request: {error}"))?;

        if request.v != IPC_PROTOCOL_VERSION {
            return Err(format!(
                "Unsupported IPC protocol version {} (expected {}).",
                request.v, IPC_PROTOCOL_VERSION
            ));
        }

        Ok(request)
    }

    pub fn command(&self) -> IpcCommand {
        self.cmd
    }

    pub fn wants_window_focus(&self) -> bool {
        self.focus
    }
}

#[derive(Debug, Serialize)]
pub struct IpcResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

impl IpcResponse {
    pub fn snapshot(snapshot: &crate::app_controller::AppSnapshot) -> Self {
        match serde_json::to_value(snapshot) {
            Ok(value) => Self {
                ok: true,
                error: None,
                snapshot: Some(value),
            },
            Err(error) => Self::error(format!("Could not serialize app state: {error}")),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(message.into()),
            snapshot: None,
        }
    }
}

pub fn status_label(status: &str) -> &'static str {
    match status {
        "idle" => "Idle",
        "starting" => "Starting",
        "recording" => "Recording",
        "stopping" => "Finalizing",
        "transcribed" => "Transcript ready",
        "error" => "Error",
        _ => "Unknown state",
    }
}

pub fn format_status_text(response_snapshot: &serde_json::Value) -> String {
    let status = response_snapshot
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let label = status_label(status);

    match response_snapshot
        .pointer("/error/message")
        .and_then(serde_json::Value::as_str)
    {
        Some(message) => format!("{label}: {message}"),
        None => label.to_string(),
    }
}

#[cfg(unix)]
pub fn socket_path() -> std::path::PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(runtime_dir).join("quicktext.sock");
    }

    let user = std::env::var("USER").unwrap_or_else(|_| "anonymous".to_string());
    std::path::PathBuf::from(format!("/tmp/quicktext-{user}.sock"))
}

#[cfg(windows)]
pub fn pipe_name() -> String {
    "\\\\.\\pipe\\quicktext".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_toggle_request() {
        let request = IpcRequest::parse(r#"{"v":1,"cmd":"toggle"}"#).unwrap();

        assert_eq!(request.command(), IpcCommand::Toggle);
    }

    #[test]
    fn parses_valid_status_request() {
        let request = IpcRequest::parse(r#"{"v":1,"cmd":"status"}"#).unwrap();

        assert_eq!(request.command(), IpcCommand::Status);
    }

    #[test]
    fn rejects_unknown_version() {
        assert!(IpcRequest::parse(r#"{"v":99,"cmd":"toggle"}"#).is_err());
    }

    #[test]
    fn rejects_malformed_json_and_unknown_commands() {
        assert!(IpcRequest::parse("not json").is_err());
        assert!(IpcRequest::parse(r#"{"v":1,"cmd":"explode"}"#).is_err());
    }

    #[test]
    fn serializes_request_as_versioned_json() {
        let line = serde_json::to_string(&IpcRequest::new(IpcCommand::Toggle)).unwrap();

        assert_eq!(line, r#"{"v":1,"cmd":"toggle"}"#);
    }

    #[test]
    fn parses_focus_flag_and_defaults_to_false() {
        let focused = IpcRequest::parse(r#"{"v":1,"cmd":"toggle","focus":true}"#).unwrap();

        assert!(focused.wants_window_focus());
        assert!(IpcRequest::new_focus(IpcCommand::Toggle).wants_window_focus());

        let plain = IpcRequest::parse(r#"{"v":1,"cmd":"toggle"}"#).unwrap();

        assert!(!plain.wants_window_focus());
    }

    #[test]
    fn labels_every_app_status() {
        assert_eq!(status_label("idle"), "Idle");
        assert_eq!(status_label("starting"), "Starting");
        assert_eq!(status_label("recording"), "Recording");
        assert_eq!(status_label("stopping"), "Finalizing");
        assert_eq!(status_label("transcribed"), "Transcript ready");
        assert_eq!(status_label("error"), "Error");
        assert_eq!(status_label("mystery"), "Unknown state");
    }

    #[test]
    fn formats_status_text_with_error_message() {
        let with_error: serde_json::Value = serde_json::from_str(
            r#"{
                "status": "error",
                "error": {"type": "missing_api_key", "message": "Add your Soniox API key before recording."}
            }"#,
        )
        .unwrap();

        assert_eq!(
            format_status_text(&with_error),
            "Error: Add your Soniox API key before recording."
        );

        let without_error: serde_json::Value =
            serde_json::from_str(r#"{"status": "recording"}"#).unwrap();

        assert_eq!(format_status_text(&without_error), "Recording");
    }
}
