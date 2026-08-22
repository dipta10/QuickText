use std::time::Duration;

use tokio::time::timeout;

#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixStream;

use super::ipc::{
    format_status_text, IpcCommand, IpcRequest, CONNECT_TIMEOUT_SECONDS, RESPONSE_TIMEOUT_SECONDS,
};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_NOT_RUNNING: i32 = 2;

const USAGE: &str =
    "Usage: quicktext [toggle|status] [--json]\n\nRun without arguments to launch the app.";

pub fn run(args: &[String]) -> i32 {
    let mut json_output = false;
    let mut command: Option<IpcCommand> = None;

    for arg in args {
        match arg.as_str() {
            "--json" => json_output = true,
            "toggle" => command = Some(IpcCommand::Toggle),
            "status" => command = Some(IpcCommand::Status),
            _ => {
                eprintln!("Unknown argument: {arg}\n{USAGE}");
                return EXIT_FAILURE;
            }
        }
    }

    let Some(command) = command else {
        eprintln!("{USAGE}");
        return EXIT_FAILURE;
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Could not start the CLI runtime: {error}");
            return EXIT_FAILURE;
        }
    };

    match runtime.block_on(send_request(command)) {
        Ok(response) => print_response(&response, json_output),
        Err(ClientError::NotRunning(message)) => {
            eprintln!("{message}");
            EXIT_NOT_RUNNING
        }
        Err(ClientError::Failed(message)) => {
            eprintln!("{message}");
            EXIT_FAILURE
        }
    }
}

fn print_response(response: &serde_json::Value, json_output: bool) -> i32 {
    if json_output {
        println!(
            "{}",
            serde_json::to_string(response).unwrap_or_else(|_| response.to_string())
        );
    }

    let snapshot_ok = response.get("ok").and_then(serde_json::Value::as_bool);
    if snapshot_ok != Some(true) {
        if !json_output {
            let message = response
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("The request failed for an unknown reason.");
            eprintln!("{message}");
        }
        return EXIT_FAILURE;
    }

    if !json_output {
        match response.get("snapshot") {
            Some(snapshot) => println!("{}", format_status_text(snapshot)),
            None => eprintln!("The response did not include app state."),
        }
    }

    let errored_state = response
        .pointer("/snapshot/status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status == "error");

    if errored_state {
        EXIT_FAILURE
    } else {
        EXIT_SUCCESS
    }
}

enum ClientError {
    NotRunning(String),
    Failed(String),
}

async fn send_request(command: IpcCommand) -> Result<serde_json::Value, ClientError> {
    #[cfg(windows)]
    {
        let _ = command;
        return Err(ClientError::Failed(
            "IPC is not yet supported on this platform.".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        send_unix_request(command).await
    }
}

#[cfg(unix)]
async fn send_unix_request(command: IpcCommand) -> Result<serde_json::Value, ClientError> {
    let path = super::ipc::socket_path();
    let connect = timeout(
        Duration::from_secs(CONNECT_TIMEOUT_SECONDS),
        UnixStream::connect(&path),
    )
    .await;

    let mut stream = match connect {
        Ok(Ok(stream)) => stream,
        Ok(Err(error)) => {
            return Err(ClientError::NotRunning(format!(
                "QuickText is not running (could not connect: {error})."
            )));
        }
        Err(_) => {
            return Err(ClientError::NotRunning(
                "QuickText did not accept a connection in time.".to_string(),
            ));
        }
    };

    let request = serde_json::to_string(&IpcRequest::new(command))
        .map_err(|error| ClientError::Failed(format!("Could not encode the request: {error}")))?;

    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|error| ClientError::Failed(format!("Could not send the request: {error}")))?;
    stream
        .write_all(b"\n")
        .await
        .map_err(|error| ClientError::Failed(format!("Could not send the request: {error}")))?;

    let mut reader = tokio::io::BufReader::new(&mut stream);
    let mut line = String::new();
    let read = timeout(
        Duration::from_secs(RESPONSE_TIMEOUT_SECONDS),
        reader.read_line(&mut line),
    )
    .await;

    match read {
        Ok(Ok(0)) | Ok(Err(_)) => Err(ClientError::Failed(
            "QuickText closed the connection.".to_string(),
        )),
        Ok(Ok(_)) => serde_json::from_str(&line)
            .map_err(|error| ClientError::Failed(format!("Invalid IPC response: {error}"))),
        Err(_) => Err(ClientError::Failed(
            "QuickText did not respond in time.".to_string(),
        )),
    }
}
