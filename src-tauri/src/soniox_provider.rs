use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    app_controller::TranscriptResult,
    audio_recorder::{AudioEncoding, AudioFormat},
};

const SONIOX_WEBSOCKET_URL: &str = "wss://stt-rt.soniox.com/transcribe-websocket";
const SONIOX_MODEL: &str = "stt-rt-v5";
const FINALIZATION_TIMEOUT_SECONDS: u64 = 12;

// Stream-boundary control tokens Soniox emits as regular final tokens
// ("<end>" on endpoint detection, "<fin>" after manual finalization).
const SONIOX_STREAM_MARKERS: [&str; 2] = ["<end>", "<fin>"];

type SharedErrorSlot = Arc<Mutex<Option<String>>>;

#[derive(Debug)]
enum SonioxCommand {
    Finish(oneshot::Sender<Result<TranscriptResult, String>>),
    Cancel,
}

#[derive(Debug)]
pub struct SonioxSession {
    audio_tx: UnboundedSender<Vec<u8>>,
    command_tx: UnboundedSender<SonioxCommand>,
    stream_error: SharedErrorSlot,
}

impl SonioxSession {
    pub async fn start(api_key: String, audio_format: AudioFormat) -> Result<Self, String> {
        let (mut websocket, _) = connect_async(SONIOX_WEBSOCKET_URL)
            .await
            .map_err(|error| format!("Could not connect to Soniox: {error}"))?;

        let config = SonioxConfig {
            api_key,
            model: SONIOX_MODEL,
            audio_format: soniox_audio_format(&audio_format),
            sample_rate: audio_format.sample_rate,
            num_channels: audio_format.channels,
        };
        let config_message = serde_json::to_string(&config)
            .map_err(|error| format!("Could not create Soniox config: {error}"))?;

        websocket
            .send(Message::Text(config_message.into()))
            .await
            .map_err(|error| format!("Could not configure Soniox stream: {error}"))?;

        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let stream_error: SharedErrorSlot = Arc::new(Mutex::new(None));

        tauri::async_runtime::spawn(run_soniox_session(
            websocket,
            audio_rx,
            command_rx,
            Arc::clone(&stream_error),
        ));

        Ok(Self {
            audio_tx,
            command_tx,
            stream_error,
        })
    }

    pub fn audio_sender(&self) -> UnboundedSender<Vec<u8>> {
        self.audio_tx.clone()
    }

    pub async fn stop(self) -> Result<TranscriptResult, String> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(SonioxCommand::Finish(result_tx))
            .map_err(|_| {
                stored_stream_error(&self.stream_error)
                    .unwrap_or_else(|| "Could not finalize Soniox stream.".to_string())
            })?;
        drop(self.audio_tx);

        let result =
            tokio::time::timeout(Duration::from_secs(FINALIZATION_TIMEOUT_SECONDS), result_rx)
                .await
                .map_err(|_| "Soniox finalization timed out.".to_string())?
                .map_err(|_| "Soniox stream ended before finalization completed.".to_string())?;

        result
    }

    pub fn cancel(self) {
        let _ = self.command_tx.send(SonioxCommand::Cancel);
    }
}

async fn run_soniox_session(
    mut websocket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    mut audio_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    mut command_rx: mpsc::UnboundedReceiver<SonioxCommand>,
    stream_error: SharedErrorSlot,
) {
    let mut final_text = String::new();
    let mut finish_tx: Option<oneshot::Sender<Result<TranscriptResult, String>>> = None;

    loop {
        tokio::select! {
            biased;

            Some(command) = command_rx.recv(), if finish_tx.is_none() => {
                match command {
                    SonioxCommand::Finish(responder) => {
                        finish_tx = Some(responder);

                        while audio_rx.try_recv().is_ok() {}

                        if let Err(error) = websocket
                            .send(Message::Text(r#"{"type":"finalize"}"#.into()))
                            .await
                        {
                            let message =
                                format!("Could not finalize Soniox stream: {error}");
                            record_stream_error(&stream_error, &message);
                            respond(&mut finish_tx, Err(message));
                            return;
                        }

                        if let Err(error) = websocket.send(Message::Text("".into())).await {
                            let message =
                                format!("Could not finalize Soniox stream: {error}");
                            record_stream_error(&stream_error, &message);
                            respond(&mut finish_tx, Err(message));
                            return;
                        }
                    }
                    SonioxCommand::Cancel => {
                        let _ = websocket.close(None).await;
                        break;
                    }
                }
            }
            Some(audio) = audio_rx.recv(), if finish_tx.is_none() => {
                if let Err(error) = websocket.send(Message::Binary(audio.into())).await {
                    let message = format!("Could not send audio to Soniox: {error}");
                    record_stream_error(&stream_error, &message);
                    respond(&mut finish_tx, Err(message));
                    break;
                }
            }
            message = websocket.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        match handle_soniox_text_response(text.as_str(), &mut final_text) {
                            Ok(true) => {
                                respond(
                                    &mut finish_tx,
                                    Ok(TranscriptResult {
                                        text: final_text.trim().to_string(),
                                        provider: "soniox".to_string(),
                                    }),
                                );
                                break;
                            }
                            Ok(false) => {}
                            Err(error) => {
                                record_stream_error(&stream_error, &error);
                                respond(&mut finish_tx, Err(error));
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        respond(
                            &mut finish_tx,
                            Ok(TranscriptResult {
                                text: final_text.trim().to_string(),
                                provider: "soniox".to_string(),
                            }),
                        );
                        break;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = websocket.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Binary(_)))
                    | Some(Ok(Message::Pong(_)))
                    | Some(Ok(Message::Frame(_))) => {}
                    Some(Err(error)) => {
                        let message = format!("Soniox stream failed: {error}");
                        record_stream_error(&stream_error, &message);
                        respond(&mut finish_tx, Err(message));
                        break;
                    }
                }
            }
        }
    }
}

fn record_stream_error(stream_error: &SharedErrorSlot, message: &str) {
    eprintln!("QuickText Soniox session error: {message}");

    if let Ok(mut slot) = stream_error.lock() {
        *slot = Some(message.to_string());
    }
}

fn stored_stream_error(stream_error: &SharedErrorSlot) -> Option<String> {
    stream_error.lock().ok().and_then(|slot| slot.clone())
}

fn respond(
    finish_tx: &mut Option<oneshot::Sender<Result<TranscriptResult, String>>>,
    result: Result<TranscriptResult, String>,
) {
    if let Some(finish_tx) = finish_tx.take() {
        let _ = finish_tx.send(result);
    }
}

fn handle_soniox_text_response(text: &str, final_text: &mut String) -> Result<bool, String> {
    let response: SonioxResponse = serde_json::from_str(text)
        .map_err(|error| format!("Could not parse Soniox response: {error}"))?;

    if let Some(error_message) = response.error_message {
        return Err(format!("Soniox provider error: {error_message}"));
    }

    for token in response.tokens {
        if token.is_final.unwrap_or(false) {
            final_text.push_str(&strip_stream_markers(&token.text));
        }
    }

    Ok(response.finished.unwrap_or(false))
}

fn strip_stream_markers(text: &str) -> String {
    SONIOX_STREAM_MARKERS
        .iter()
        .fold(text.to_string(), |cleaned, marker| {
            cleaned.replace(marker, "")
        })
}

fn soniox_audio_format(audio_format: &AudioFormat) -> &'static str {
    match audio_format.encoding {
        AudioEncoding::F32 => "pcm_f32le",
        AudioEncoding::I16 => "pcm_s16le",
        AudioEncoding::U16 => "pcm_u16le",
    }
}

#[derive(Serialize)]
struct SonioxConfig<'a> {
    api_key: String,
    model: &'a str,
    audio_format: &'a str,
    sample_rate: u32,
    num_channels: u16,
}

#[derive(Deserialize)]
struct SonioxResponse {
    #[serde(default)]
    tokens: Vec<SonioxToken>,
    #[serde(default)]
    finished: Option<bool>,
    #[serde(default)]
    error_message: Option<String>,
}

#[derive(Deserialize)]
struct SonioxToken {
    text: String,
    #[serde(default)]
    is_final: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_finished_response() {
        let mut final_text = String::new();

        let finished = handle_soniox_text_response(
            r#"{"tokens":[{"text":"Hello","is_final":true},{"text":" world","is_final":true}],"finished":true}"#,
            &mut final_text,
        )
        .unwrap();

        assert!(finished);
        assert_eq!(final_text, "Hello world");
    }

    #[test]
    fn ignores_non_final_tokens() {
        let mut final_text = String::new();

        let finished = handle_soniox_text_response(
            r#"{"tokens":[{"text":"maybe","is_final":false}],"finished":false}"#,
            &mut final_text,
        )
        .unwrap();

        assert!(!finished);
        assert_eq!(final_text, "");
    }

    #[test]
    fn maps_error_response() {
        let mut final_text = String::new();

        let error = handle_soniox_text_response(
            r#"{"tokens":[],"error_code":400,"error_type":"invalid_request","error_message":"Bad audio"}"#,
            &mut final_text,
        )
        .unwrap_err();

        assert_eq!(error, "Soniox provider error: Bad audio");
    }

    #[test]
    fn strips_stream_marker_tokens() {
        let mut final_text = String::new();

        handle_soniox_text_response(
            r#"{"tokens":[{"text":"Hello world","is_final":true},{"text":"<end>","is_final":true}],"finished":true}"#,
            &mut final_text,
        )
        .unwrap();

        assert_eq!(final_text, "Hello world");
    }

    #[test]
    fn strips_finalization_markers() {
        let mut final_text = String::new();

        handle_soniox_text_response(
            r#"{"tokens":[{"text":"done.","is_final":true},{"text":"<fin>","is_final":true},{"text":"more","is_final":true},{"text":"<fin>","is_final":true}],"finished":true}"#,
            &mut final_text,
        )
        .unwrap();

        assert_eq!(final_text, "done.more");
    }

    #[test]
    fn strips_markers_embedded_in_token_text() {
        let mut final_text = String::new();

        handle_soniox_text_response(
            r#"{"tokens":[{"text":"amén.<end>","is_final":true},{"text":"<fin>","is_final":true}],"finished":true}"#,
            &mut final_text,
        )
        .unwrap();

        assert_eq!(final_text, "amén.");
    }
}
