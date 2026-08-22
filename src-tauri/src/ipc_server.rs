use std::io::ErrorKind;
use std::os::unix::net::UnixListener as StdUnixListener;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::time::timeout;
use tokio::time::Duration;

use super::ipc::{IpcCommand, IpcRequest, IpcResponse};
use crate::AppControllerState;
use tauri::Manager;

pub enum ServerStart {
    Listening,
    AlreadyRunning,
}

pub fn start(app: tauri::AppHandle) -> Result<ServerStart, String> {
    let std_listener = match bind_socket()? {
        SocketBind::Bound(listener) => listener,
        SocketBind::InstanceAlreadyRunning => return Ok(ServerStart::AlreadyRunning),
    };

    tauri::async_runtime::spawn(async move {
        match UnixListener::from_std(std_listener) {
            Ok(listener) => accept_loop(app, listener).await,
            Err(error) => eprintln!("Could not start IPC listener: {error}"),
        }
    });

    Ok(ServerStart::Listening)
}

enum SocketBind {
    Bound(StdUnixListener),
    InstanceAlreadyRunning,
}

fn bind_socket() -> Result<SocketBind, String> {
    let path = super::ipc::socket_path();

    match StdUnixListener::bind(&path) {
        Ok(listener) => {
            restrict_socket_permissions(&path)?;
            finish_bind(listener)
        }
        Err(error) if error.kind() == ErrorKind::AddrInUse => {
            if live_instance_exists(&path) {
                return Ok(SocketBind::InstanceAlreadyRunning);
            }

            std::fs::remove_file(&path)
                .map_err(|error| format!("Could not remove stale IPC socket: {error}"))?;
            let listener = StdUnixListener::bind(&path)
                .map_err(|error| format!("Could not rebind IPC socket: {error}"))?;
            restrict_socket_permissions(&path)?;
            eprintln!("Removed a stale QuickText IPC socket and rebound it.");
            finish_bind(listener)
        }
        Err(error) => Err(format!("Could not bind IPC socket: {error}")),
    }
}

fn finish_bind(listener: StdUnixListener) -> Result<SocketBind, String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Could not prepare IPC socket: {error}"))?;
    Ok(SocketBind::Bound(listener))
}

fn live_instance_exists(path: &std::path::Path) -> bool {
    std::os::unix::net::UnixStream::connect(path).is_ok()
}

fn restrict_socket_permissions(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("Could not lock down IPC socket permissions: {error}"))
}

async fn accept_loop(app: tauri::AppHandle, listener: UnixListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let app = app.clone();
                tauri::async_runtime::spawn(handle_connection(app, stream));
            }
            Err(error) => {
                eprintln!("IPC listener accept failed: {error}");
                return;
            }
        }
    }
}

async fn handle_connection(app: tauri::AppHandle, stream: tokio::net::UnixStream) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    if reader.read_line(&mut line).await.is_err() {
        return;
    }

    let response = match IpcRequest::parse(line.trim()) {
        Ok(request) => dispatch(&app, &request).await,
        Err(message) => IpcResponse::error(message),
    };

    if let Ok(payload) = serde_json::to_string(&response) {
        let _ = writer.write_all(payload.as_bytes()).await;
        let _ = writer.write_all(b"\n").await;
    }
}

async fn dispatch(app: &tauri::AppHandle, request: &IpcRequest) -> IpcResponse {
    match request.command() {
        IpcCommand::Toggle => {
            let focus_window = request.wants_window_focus();
            let toggle = timeout(
                Duration::from_secs(super::ipc::RESPONSE_TIMEOUT_SECONDS),
                crate::toggle_recording_for_app(app.clone(), None, focus_window),
            )
            .await;

            match toggle {
                Ok(Ok(snapshot)) => IpcResponse::snapshot(&snapshot),
                Ok(Err(message)) => IpcResponse::error(message),
                Err(_) => IpcResponse::error("QuickText did not respond to the toggle in time."),
            }
        }
        IpcCommand::Status => {
            let controller = app.state::<AppControllerState>();
            let snapshot =
                crate::lock_controller(&controller).map(|controller| controller.snapshot());

            match snapshot {
                Ok(snapshot) => IpcResponse::snapshot(&snapshot),
                Err(message) => IpcResponse::error(message),
            }
        }
    }
}
