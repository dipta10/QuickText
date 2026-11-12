//! Soniox synthesis protocol stays behind this provider boundary.
use crate::{speech::SpeechSettings, speech_playback::SAMPLE_RATE};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::tungstenite::{
    client::IntoClientRequest,
    http::{header::AUTHORIZATION, HeaderValue},
    Message,
};

const ENDPOINT: &str = "wss://tts-rt.soniox.com/tts-websocket";
const MODEL: &str = "tts-rt-v2";
const NETWORK_TIMEOUT: Duration = Duration::from_secs(15);

fn config(settings: &SpeechSettings, id: &str) -> serde_json::Value {
    json!({"model": MODEL, "stream_id": id, "voice": settings.voice,
        "language": settings.language, "speed": settings.speed,
        "audio_format": "pcm_s16le", "sample_rate": SAMPLE_RATE})
}
#[derive(Deserialize)]
struct Response {
    stream_id: Option<String>,
    audio: Option<String>,
    #[serde(default)]
    audio_end: bool,
    #[serde(default)]
    terminated: bool,
    error_code: Option<u16>,
    error_type: Option<String>,
}
fn provider_error(kind: Option<&str>, code: Option<u16>) -> String {
    match (kind, code) {
        (Some("max_audio_duration_reached"), _) => {
            "This reading exceeds the two-minute limit. Select a shorter passage."
        }
        (_, Some(401)) => "Your Soniox API key was rejected. Update it in Settings.",
        (_, Some(402)) => "Your Soniox account has reached its balance or budget limit.",
        (_, Some(403)) => "Enable text-to-speech permission for your Soniox API key.",
        (_, Some(429)) => "Soniox is busy or rate limited. Wait a moment and try again.",
        (_, Some(400)) => "Check your voice and language in Text-to-speech settings.",
        _ => "The speech service is unavailable. Try again shortly.",
    }
    .into()
}

pub async fn stream(
    text: &str,
    settings: &SpeechSettings,
    api_key: &str,
    session_id: u64,
    on_audio: impl FnMut(&[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let id = format!("quicktext-{session_id}");
    let mut request = ENDPOINT
        .into_client_request()
        .map_err(|_| "Could not prepare speech generation.".to_string())?;
    let mut header = HeaderValue::from_str(&format!("Bearer {api_key}"))
        .map_err(|_| "Invalid Soniox API key. Update it in Settings.".to_string())?;
    header.set_sensitive(true);
    request.headers_mut().insert(AUTHORIZATION, header);
    let (mut socket, _) = timeout(NETWORK_TIMEOUT, tokio_tungstenite::connect_async(request))
        .await
        .map_err(|_| "Connecting to the speech service timed out.".to_string())?
        .map_err(|_| {
            "Could not connect to Soniox. Check your connection and API key.".to_string()
        })?;
    timeout(NETWORK_TIMEOUT, async {
        socket
            .send(Message::Text(config(settings, &id).to_string().into()))
            .await?;
        socket
            .send(Message::Text(
                json!({"stream_id": id, "text": text, "text_end": true})
                    .to_string()
                    .into(),
            ))
            .await
    })
    .await
    .map_err(|_| "Sending text to the speech service timed out.".to_string())?
    .map_err(|_| "Could not send text to the speech service.".to_string())?;
    receive_audio(&mut socket, &id, on_audio).await
}

async fn receive_audio<S>(
    socket: &mut S,
    id: &str,
    mut on_audio: impl FnMut(&[u8]) -> Result<(), String>,
) -> Result<(), String>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
        + Unpin,
{
    let mut audio_ended = false;
    loop {
        let message = timeout(NETWORK_TIMEOUT, socket.next())
            .await
            .map_err(|_| "The speech service stopped responding. Try again.".to_string())?
            .ok_or_else(|| "The speech connection ended before completion. Try again.".to_string())?
            .map_err(|_| "The speech connection was interrupted. Try again.".to_string())?;
        match message {
            Message::Text(text) => {
                if text.len() > 2_000_000 {
                    return Err("The speech service returned invalid audio.".into());
                }
                let response: Response = serde_json::from_str(&text)
                    .map_err(|_| "The speech service returned an invalid response.".to_string())?;
                if response.error_code.is_some() || response.error_type.is_some() {
                    return Err(provider_error(
                        response.error_type.as_deref(),
                        response.error_code,
                    ));
                }
                if response.stream_id.as_deref() != Some(id) {
                    continue;
                }
                if let Some(audio) = response.audio.filter(|a| !a.is_empty()) {
                    if audio_ended {
                        return Err("The speech service returned audio after completion.".into());
                    }
                    let bytes = STANDARD
                        .decode(audio)
                        .map_err(|_| "The speech service returned invalid audio.".to_string())?;
                    on_audio(&bytes)?;
                }
                audio_ended |= response.audio_end;
                if response.terminated {
                    if !audio_ended {
                        return Err("Speech generation ended early. Try a shorter passage.".into());
                    }
                    return Ok(());
                }
            }
            Message::Ping(payload) => {
                socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|_| "The speech connection was interrupted.".to_string())?;
            }
            Message::Close(_) => {
                return Err("The speech connection ended before completion. Try again.".into())
            }
            _ => {}
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_current_model_raw_audio_and_no_secret() {
        let value = config(&SpeechSettings::default(), "test");
        assert_eq!(value["model"], "tts-rt-v2");
        assert_eq!(value["audio_format"], "pcm_s16le");
        assert!(value.get("api_key").is_none());
        assert_eq!(value["sample_rate"], 24000);
    }
    #[test]
    fn errors_are_actionable_without_provider_content() {
        assert!(provider_error(None, Some(403)).contains("permission"));
        assert!(
            provider_error(Some("max_audio_duration_reached"), Some(413)).contains("two-minute")
        );
    }
    struct Socket(tokio::sync::mpsc::UnboundedReceiver<Message>);
    impl futures_util::Stream for Socket {
        type Item = Result<Message, tokio_tungstenite::tungstenite::Error>;
        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            self.0.poll_recv(cx).map(|message| message.map(Ok))
        }
    }
    impl futures_util::Sink<Message> for Socket {
        type Error = tokio_tungstenite::tungstenite::Error;
        fn poll_ready(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }
        fn start_send(self: std::pin::Pin<&mut Self>, _: Message) -> Result<(), Self::Error> {
            Ok(())
        }
        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }
        fn poll_close(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }
    }
    fn frame(value: serde_json::Value) -> Message {
        Message::Text(value.to_string().into())
    }
    #[tokio::test]
    async fn delivers_audio_before_generation_finishes() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut socket = Socket(rx);
        let audio = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let output = audio.clone();
        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let observed = notify.clone();
        tx.send(frame(json!({"stream_id":"test", "audio":"AAABAA=="})))
            .unwrap();
        let reading = receive_audio(&mut socket, "test", move |chunk| {
            output.lock().unwrap().extend_from_slice(chunk);
            observed.notify_one();
            Ok(())
        });
        tokio::pin!(reading);
        tokio::select! {
            result = &mut reading => panic!("Reading completed before its terminal frame: {result:?}"),
            _ = notify.notified() => {},
        }
        assert_eq!(*audio.lock().unwrap(), [0, 0, 1, 0]);
        tx.send(frame(
            json!({"stream_id":"test", "audio":"AgA=", "audio_end":true}),
        ))
        .unwrap();
        tx.send(frame(json!({"stream_id":"test", "terminated":true})))
            .unwrap();
        assert!(reading.await.is_ok());
        assert_eq!(*audio.lock().unwrap(), [0, 0, 1, 0, 2, 0]);
    }
    #[tokio::test]
    async fn rejects_truncation_disconnect_and_invalid_audio_without_leaking_payloads() {
        for (frames, expected) in [
            (
                vec![json!({"stream_id":"test","terminated":true})],
                "ended early",
            ),
            (
                vec![json!({"stream_id":"test","audio":"private invalid audio"})],
                "invalid audio",
            ),
            (
                vec![json!({"error_code":403,"error_message":"private provider content"})],
                "permission",
            ),
            (vec![], "ended before completion"),
        ] {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            for value in frames {
                tx.send(frame(value)).unwrap();
            }
            drop(tx);
            let error = receive_audio(&mut Socket(rx), "test", |_| Ok(()))
                .await
                .unwrap_err();
            assert!(error.contains(expected), "{error}");
            assert!(!error.contains("private"));
        }
    }
    #[tokio::test(start_paused = true)]
    async fn times_out_a_silent_provider() {
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        assert!(receive_audio(&mut Socket(rx), "test", |_| Ok(()))
            .await
            .unwrap_err()
            .contains("stopped responding"));
    }
}
