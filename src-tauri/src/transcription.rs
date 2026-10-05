use std::{future::Future, pin::Pin, str::FromStr};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::{app_controller::TranscriptResult, audio_recorder::AudioFormat};

pub const CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

// Dropping a session must also drop its socket, buffered audio, and pending
// connection. A queued Cancel cannot interrupt a stalled connect or send.
#[derive(Debug, Default)]
pub struct ProviderTask(pub Option<tauri::async_runtime::JoinHandle<()>>);

impl Drop for ProviderTask {
    fn drop(&mut self) {
        if let Some(task) = &self.0 {
            task.abort();
        }
    }
}

pub async fn connect_with_timeout<T>(
    provider: ProviderId,
    connect: impl Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tokio::time::timeout(CONNECTION_TIMEOUT, connect)
        .await
        .unwrap_or_else(|_| {
            Err(format!(
                "{} connection timed out. Check your network and try again.",
                provider.display_name()
            ))
        })
}

// Generate user guidance from bounded categories. WebSocket and provider
// errors can contain response bodies, headers, URLs, or user-authored context.
pub fn transport_error(
    provider: ProviderId,
    error: &tokio_tungstenite::tungstenite::Error,
) -> String {
    use tokio_tungstenite::tungstenite::Error;
    let guidance = match error {
        Error::Http(response) => match response.status().as_u16() {
            401 | 403 => "rejected the API key. Check the key and its permissions in Settings.",
            400 | 413 | 422 => {
                "rejected the transcription settings. Check your terms and provider settings."
            }
            429 => "is rate limited. Wait a moment and try again.",
            _ => "is unavailable. Try again later.",
        },
        Error::Io(_) | Error::Tls(_) => "connection failed. Check your network and try again.",
        _ => "stream failed. Try recording again.",
    };
    format!("{} {guidance}", provider.display_name())
}

pub type ProviderFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    #[default]
    Soniox,
    Deepgram,
}

impl ProviderId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Soniox => "soniox",
            Self::Deepgram => "deepgram",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Soniox => "Soniox",
            Self::Deepgram => "Deepgram",
        }
    }
}

impl FromStr for ProviderId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "soniox" => Ok(Self::Soniox),
            "deepgram" => Ok(Self::Deepgram),
            _ => Err(format!("Unknown transcription provider: {value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionOptions {
    pub api_key: String,
    pub audio_format: AudioFormat,
    pub language_hints: Vec<String>,
    pub description: String,
    pub terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PartialTranscript {
    pub final_text: String,
    pub partial_text: String,
}

pub trait TranscriptionSession: Send {
    fn audio_sender(&self) -> mpsc::UnboundedSender<Vec<u8>>;
    fn take_partial_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<PartialTranscript>>;
    fn take_ready_receiver(&mut self) -> Option<oneshot::Receiver<Result<(), String>>>;
    fn take_failure_receiver(&mut self) -> Option<oneshot::Receiver<String>>;
    fn stop(self: Box<Self>) -> ProviderFuture<Result<TranscriptResult, String>>;
    fn cancel(self: Box<Self>);
}

pub trait TranscriptionProvider {
    fn id(&self) -> ProviderId;
    fn start_session(
        &self,
        options: TranscriptionOptions,
    ) -> Result<Box<dyn TranscriptionSession>, String>;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::ProviderId;

    #[test]
    fn provider_ids_round_trip_and_reject_unknown_values() {
        for (value, provider) in [
            ("soniox", ProviderId::Soniox),
            ("deepgram", ProviderId::Deepgram),
        ] {
            assert_eq!(ProviderId::from_str(value), Ok(provider));
            assert_eq!(provider.as_str(), value);
            assert_eq!(provider.display_name().to_ascii_lowercase(), value);
        }

        assert!(ProviderId::from_str("other").is_err());
    }
}

#[cfg(test)]
pub(crate) async fn mock_socket_pair() -> (
    tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream>,
    tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream>,
) {
    use tokio_tungstenite::{tungstenite::protocol::Role, WebSocketStream};
    let (client, server) = tokio::io::duplex(65_536);
    (
        WebSocketStream::from_raw_socket(client, Role::Client, None).await,
        WebSocketStream::from_raw_socket(server, Role::Server, None).await,
    )
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn stalled_connection_has_a_deadline() {
        let before = tokio::time::Instant::now();
        let result = connect_with_timeout(
            ProviderId::Deepgram,
            std::future::pending::<Result<(), String>>(),
        )
        .await;
        assert!(result
            .unwrap_err()
            .contains("Deepgram connection timed out"));
        assert_eq!(before.elapsed(), CONNECTION_TIMEOUT);
    }

    #[tokio::test]
    async fn dropping_the_task_owner_cancels_pending_work() {
        let (dropped_tx, dropped_rx) = oneshot::channel();
        struct OnDrop(Option<oneshot::Sender<()>>);
        impl Drop for OnDrop {
            fn drop(&mut self) {
                let _ = self.0.take().unwrap().send(());
            }
        }
        let guard = OnDrop(Some(dropped_tx));
        let task = tokio::spawn(async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        });
        drop(ProviderTask(Some(tauri::async_runtime::JoinHandle::Tokio(
            task,
        ))));
        dropped_rx.await.unwrap();
    }
    #[test]
    fn transport_errors_discard_headers_and_bodies() {
        use tokio_tungstenite::tungstenite::{http::Response, Error};
        let error = Error::Http(Box::new(
            Response::builder()
                .status(401)
                .header("private-header", "private value")
                .body(Some(b"private provider body".to_vec()))
                .unwrap(),
        ));
        let message = transport_error(ProviderId::Deepgram, &error);
        assert_eq!(
            message,
            "Deepgram rejected the API key. Check the key and its permissions in Settings."
        );
    }
}
