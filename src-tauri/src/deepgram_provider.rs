use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tokio::{net::lookup_host, net::TcpStream};
use tokio_tungstenite::{
    client_async_tls,
    tungstenite::{
        client::IntoClientRequest,
        http::{header::AUTHORIZATION, HeaderValue},
        Message,
    },
};

use crate::{
    app_controller::TranscriptResult,
    audio_recorder::{AudioEncoding, AudioFormat},
    transcription::{
        connect_with_timeout, transport_error, PartialTranscript, ProviderId, ProviderTask,
        TranscriptionOptions, TranscriptionProvider, TranscriptionSession,
    },
};

const DEEPGRAM_ENDPOINT: &str = "wss://api.deepgram.com/v1/listen";
const DEEPGRAM_HOST: &str = "api.deepgram.com";
const DEEPGRAM_PORT: u16 = 443;
const FINALIZATION_TIMEOUT_SECONDS: u64 = 5;
const KEEPALIVE_INTERVAL_SECONDS: u64 = 5;

type DeepgramSocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type SharedError = Arc<Mutex<Option<String>>>;

struct DeepgramSessionTask {
    url: String,
    api_key: String,
    audio_format: AudioFormat,
    audio_rx: UnboundedReceiver<Vec<u8>>,
    command_rx: UnboundedReceiver<DeepgramCommand>,
    partial_tx: UnboundedSender<PartialTranscript>,
    stream_error: SharedError,
    ready_tx: oneshot::Sender<Result<(), String>>,
}

#[derive(Debug, PartialEq, Eq)]
enum ResponseOutcome {
    Update(PartialTranscript),
    Complete,
    Ignore,
}

#[derive(Default)]
struct TranscriptAccumulator {
    final_segments: BTreeMap<u64, String>,
    next_untimed_segment: u64,
}

impl TranscriptAccumulator {
    fn final_text(&self) -> String {
        self.final_segments
            .values()
            .filter(|text| !text.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn add_final(&mut self, start: Option<f64>, transcript: String) {
        if transcript.is_empty() {
            return;
        }
        let key = start
            .map(|seconds| (seconds.max(0.0) * 1_000_000.0).round() as u64)
            .unwrap_or_else(|| {
                let key = u64::MAX / 2 + self.next_untimed_segment;
                self.next_untimed_segment = self.next_untimed_segment.saturating_add(1);
                key
            });
        self.final_segments.insert(key, transcript);
    }
}

#[derive(Debug)]
enum DeepgramCommand {
    Finish(oneshot::Sender<Result<TranscriptResult, String>>),
    Cancel,
}

pub struct DeepgramProvider;

impl TranscriptionProvider for DeepgramProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Deepgram
    }

    fn start_session(
        &self,
        options: TranscriptionOptions,
    ) -> Result<Box<dyn TranscriptionSession>, String> {
        Ok(Box::new(DeepgramSession::start(options)?))
    }
}

pub struct DeepgramSession {
    audio_tx: UnboundedSender<Vec<u8>>,
    command_tx: UnboundedSender<DeepgramCommand>,
    partial_rx: Option<UnboundedReceiver<PartialTranscript>>,
    ready_rx: Option<oneshot::Receiver<Result<(), String>>>,
    stream_error: SharedError,
    failure_rx: Option<oneshot::Receiver<String>>,
    _task: ProviderTask,
}

impl DeepgramSession {
    fn start(options: TranscriptionOptions) -> Result<Self, String> {
        let url = deepgram_url(&options.audio_format, &options.terms)?;
        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (partial_tx, partial_rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel();
        let (failure_tx, failure_rx) = oneshot::channel();
        let stream_error = Arc::new(Mutex::new(None));
        let task_error = Arc::clone(&stream_error);

        let task = tauri::async_runtime::spawn(async move {
            run_configured_session(DeepgramSessionTask {
                url,
                api_key: options.api_key,
                audio_format: options.audio_format,
                audio_rx,
                command_rx,
                partial_tx,
                stream_error: Arc::clone(&task_error),
                ready_tx,
            })
            .await;
            if let Some(error) = stored_error(&task_error) {
                let _ = failure_tx.send(error);
            }
        });

        Ok(Self {
            audio_tx,
            command_tx,
            partial_rx: Some(partial_rx),
            ready_rx: Some(ready_rx),
            failure_rx: Some(failure_rx),
            _task: ProviderTask(Some(task)),
            stream_error,
        })
    }

    async fn finish(self) -> Result<TranscriptResult, String> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(DeepgramCommand::Finish(result_tx))
            .map_err(|_| {
                stored_error(&self.stream_error)
                    .unwrap_or_else(|| "Could not finalize Deepgram stream.".to_string())
            })?;
        drop(self.audio_tx);

        match tokio::time::timeout(Duration::from_secs(FINALIZATION_TIMEOUT_SECONDS), result_rx)
            .await
        {
            Ok(result) => result
                .map_err(|_| "Deepgram stream ended before finalization completed.".to_string())?,
            Err(_) => {
                let _ = self.command_tx.send(DeepgramCommand::Cancel);
                Err("Deepgram finalization timed out.".to_string())
            }
        }
    }
}

impl TranscriptionSession for DeepgramSession {
    fn audio_sender(&self) -> UnboundedSender<Vec<u8>> {
        self.audio_tx.clone()
    }

    fn take_partial_receiver(&mut self) -> Option<UnboundedReceiver<PartialTranscript>> {
        self.partial_rx.take()
    }

    fn take_ready_receiver(&mut self) -> Option<oneshot::Receiver<Result<(), String>>> {
        self.ready_rx.take()
    }

    fn take_failure_receiver(&mut self) -> Option<oneshot::Receiver<String>> {
        self.failure_rx.take()
    }

    fn stop(
        self: Box<Self>,
    ) -> crate::transcription::ProviderFuture<Result<TranscriptResult, String>> {
        Box::pin(async move { self.finish().await })
    }

    fn cancel(self: Box<Self>) {
        let _ = self.command_tx.send(DeepgramCommand::Cancel);
    }
}

pub(crate) fn validate_terms(terms: &[String]) -> Result<(), String> {
    if terms.len() > 100 {
        return Err("Deepgram accepts at most 100 terms and phrases.".to_string());
    }

    let token_estimate = terms
        .iter()
        .map(|term| term.split_whitespace().count().max(1))
        .sum::<usize>();
    if token_estimate > 500 {
        return Err("Deepgram accepts at most 500 tokens across keyterms.".to_string());
    }

    Ok(())
}

fn deepgram_url(audio_format: &AudioFormat, terms: &[String]) -> Result<String, String> {
    validate_terms(terms)?;
    let encoding = match audio_format.encoding {
        AudioEncoding::I16 | AudioEncoding::U16 => "linear16",
        AudioEncoding::F32 => "linear32",
    };
    let mut url = format!(
        "{DEEPGRAM_ENDPOINT}?model=nova-3&interim_results=true&endpointing=false&punctuate=true&encoding={encoding}&sample_rate={}&channels={}",
        audio_format.sample_rate, audio_format.channels,
    );
    for term in terms {
        url.push_str("&keyterm=");
        url.push_str(&utf8_percent_encode(term, NON_ALPHANUMERIC).to_string());
    }
    Ok(url)
}

fn normalize_audio(audio: Vec<u8>, audio_format: &AudioFormat) -> Vec<u8> {
    if audio_format.encoding != AudioEncoding::U16 {
        return audio;
    }

    audio
        .chunks_exact(2)
        .flat_map(|bytes| {
            let sample = u16::from_le_bytes([bytes[0], bytes[1]]).wrapping_sub(32_768) as i16;
            sample.to_le_bytes()
        })
        .collect()
}

async fn connect(url: &str, api_key: &str) -> Result<DeepgramSocket, String> {
    let mut request = url
        .into_client_request()
        .map_err(|error| transport_error(ProviderId::Deepgram, &error))?;
    let authorization = HeaderValue::from_str(&format!("Token {api_key}"))
        .map_err(|_| "Deepgram API key contains invalid header characters.".to_string())?;
    request.headers_mut().insert(AUTHORIZATION, authorization);

    let socket = connect_ipv4(DEEPGRAM_HOST, DEEPGRAM_PORT).await?;

    client_async_tls(request, socket)
        .await
        .map(|(socket, _)| socket)
        .map_err(|error| transport_error(ProviderId::Deepgram, &error))
}

async fn connect_ipv4(host: &str, port: u16) -> Result<TcpStream, String> {
    let resolved = lookup_host((host, port))
        .await
        .map_err(|error| format!("Could not resolve Deepgram IPv4 addresses: {error}"))?;
    let addresses = ipv4_addresses(resolved);
    if addresses.is_empty() {
        return Err("Deepgram did not resolve to an IPv4 address.".to_string());
    }

    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect(address).await {
            Ok(socket) => return Ok(socket),
            Err(error) => last_error = Some(error),
        }
    }

    Err(format!(
        "Could not connect to Deepgram over IPv4: {}",
        last_error.expect("at least one IPv4 connection was attempted")
    ))
}

fn ipv4_addresses(addresses: impl IntoIterator<Item = SocketAddr>) -> Vec<SocketAddr> {
    addresses.into_iter().filter(SocketAddr::is_ipv4).collect()
}

async fn run_configured_session(task: DeepgramSessionTask) {
    let DeepgramSessionTask {
        url,
        api_key,
        audio_format,
        audio_rx,
        command_rx,
        partial_tx,
        stream_error,
        ready_tx,
    } = task;
    match connect_with_timeout(ProviderId::Deepgram, connect(&url, &api_key)).await {
        Ok(socket) => {
            let _ = ready_tx.send(Ok(()));
            run_session(
                socket,
                audio_format,
                audio_rx,
                command_rx,
                partial_tx,
                stream_error,
            )
            .await;
        }
        Err(message) => {
            record_error(&stream_error, &message);
            let _ = ready_tx.send(Err(message.clone()));
            fail_queued_finishes(command_rx, message);
        }
    }
}

async fn run_session<S>(
    mut socket: tokio_tungstenite::WebSocketStream<S>,
    audio_format: AudioFormat,
    mut audio_rx: UnboundedReceiver<Vec<u8>>,
    mut command_rx: UnboundedReceiver<DeepgramCommand>,
    partial_tx: UnboundedSender<PartialTranscript>,
    stream_error: SharedError,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut transcript = TranscriptAccumulator::default();
    let mut finish_tx = None;
    let mut completion_metadata_received = false;
    let keepalive_period = Duration::from_secs(KEEPALIVE_INTERVAL_SECONDS);
    let mut keepalive = tokio::time::interval_at(
        tokio::time::Instant::now() + keepalive_period,
        keepalive_period,
    );
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_audio_sent = tokio::time::Instant::now();

    loop {
        tokio::select! {
            biased;

            Some(command) = command_rx.recv() => match command {
                DeepgramCommand::Finish(responder) => {
                    if finish_tx.is_some() {
                        let _ = responder.send(Err("Deepgram finalization is already in progress.".to_string()));
                        continue;
                    }
                    finish_tx = Some(responder);
                    while let Some(audio) = audio_rx.recv().await {
                        let audio = normalize_audio(audio, &audio_format);
                        if let Err(error) = socket.send(Message::Binary(audio.into())).await {
                            fail(&stream_error, &mut finish_tx, transport_error(ProviderId::Deepgram, &error));
                            return;
                        }
                    }
                    if let Err(error) = socket
                        .send(Message::Text(r#"{"type":"CloseStream"}"#.into()))
                        .await
                    {
                        fail(&stream_error, &mut finish_tx, transport_error(ProviderId::Deepgram, &error));
                        return;
                    }
                }
                DeepgramCommand::Cancel => {
                    let _ = socket.close(None).await;
                    return;
                }
            },
            Some(audio) = audio_rx.recv(), if finish_tx.is_none() => {
                let audio = normalize_audio(audio, &audio_format);
                if let Err(error) = socket.send(Message::Binary(audio.into())).await {
                    fail(&stream_error, &mut finish_tx, transport_error(ProviderId::Deepgram, &error));
                    return;
                }
                last_audio_sent = tokio::time::Instant::now();
            },
            _ = keepalive.tick(), if finish_tx.is_none() => {
                if last_audio_sent.elapsed() >= keepalive_period {
                    if let Err(error) = socket
                        .send(Message::Text(r#"{"type":"KeepAlive"}"#.into()))
                        .await
                    {
                        fail(&stream_error, &mut finish_tx, transport_error(ProviderId::Deepgram, &error));
                        return;
                    }
                }
            },
            message = socket.next() => match message {
                Some(Ok(Message::Text(text))) => match handle_text_response(text.as_str(), &mut transcript) {
                    Ok(ResponseOutcome::Update(update)) => { let _ = partial_tx.send(update); }
                    Ok(ResponseOutcome::Complete) => {
                        if finish_tx.is_some() {
                            completion_metadata_received = true;
                        }
                    }
                    Ok(ResponseOutcome::Ignore) => {}
                    Err(error) => {
                        fail(&stream_error, &mut finish_tx, error);
                        return;
                    }
                },
                Some(Ok(Message::Ping(payload))) => { let _ = socket.send(Message::Pong(payload)).await; }
                Some(Ok(Message::Close(frame))) => {
                    let graceful = frame.as_ref().is_none_or(|frame| frame.code == tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal);
                    if completion_metadata_received && graceful {
                        respond(&mut finish_tx, Ok(TranscriptResult {
                            text: transcript.final_text().trim().to_string(),
                            provider: ProviderId::Deepgram.as_str().to_string(),
                        }));
                    } else {
                        fail(&stream_error, &mut finish_tx, "Deepgram stream closed before successful completion.".to_string());
                    }
                    return;
                }
                None => {
                    fail(&stream_error, &mut finish_tx, "Deepgram stream ended before successful completion.".to_string());
                    return;
                }
                Some(Ok(Message::Binary(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {}
                Some(Err(error)) => {
                    fail(&stream_error, &mut finish_tx, transport_error(ProviderId::Deepgram, &error));
                    return;
                }
            }
        }
    }
}

fn handle_text_response(
    text: &str,
    accumulator: &mut TranscriptAccumulator,
) -> Result<ResponseOutcome, String> {
    let response: DeepgramResponse = serde_json::from_str(text).map_err(|_| {
        "Deepgram sent an invalid transcription response. Try recording again.".to_string()
    })?;

    match response.message_type.as_deref() {
        Some("Metadata") => Ok(ResponseOutcome::Complete),
        Some("Results") => {
            let result_text = response
                .channel
                .and_then(|channel| channel.alternatives.into_iter().next())
                .map(|alternative| alternative.transcript.trim().to_string())
                .unwrap_or_default();
            let is_final = response.is_final.unwrap_or(false);
            if is_final && !result_text.is_empty() {
                accumulator.add_final(response.start, result_text.clone());
            }
            Ok(ResponseOutcome::Update(PartialTranscript {
                final_text: accumulator.final_text(),
                partial_text: if is_final { String::new() } else { result_text },
            }))
        }
        Some("Error") => Err("Deepgram could not process the transcription stream.".to_string()),
        _ => Ok(ResponseOutcome::Ignore),
    }
}

fn fail_queued_finishes(mut receiver: UnboundedReceiver<DeepgramCommand>, message: String) {
    while let Ok(command) = receiver.try_recv() {
        if let DeepgramCommand::Finish(responder) = command {
            let _ = responder.send(Err(message.clone()));
        }
    }
}

fn record_error(slot: &SharedError, message: &str) {
    eprintln!("QuickText Deepgram session error: {message}");
    if let Ok(mut slot) = slot.lock() {
        *slot = Some(message.to_string());
    }
}

fn stored_error(slot: &SharedError) -> Option<String> {
    slot.lock().ok().and_then(|slot| slot.clone())
}

fn respond(
    responder: &mut Option<oneshot::Sender<Result<TranscriptResult, String>>>,
    result: Result<TranscriptResult, String>,
) {
    if let Some(responder) = responder.take() {
        let _ = responder.send(result);
    }
}

fn fail(
    slot: &SharedError,
    responder: &mut Option<oneshot::Sender<Result<TranscriptResult, String>>>,
    message: String,
) {
    record_error(slot, &message);
    respond(responder, Err(message));
}

#[derive(Deserialize)]
struct DeepgramResponse {
    #[serde(rename = "type")]
    message_type: Option<String>,
    is_final: Option<bool>,
    start: Option<f64>,
    channel: Option<DeepgramChannel>,
}

#[derive(Deserialize)]
struct DeepgramChannel {
    #[serde(default)]
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Deserialize)]
struct DeepgramAlternative {
    #[serde(default)]
    transcript: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i16_format() -> AudioFormat {
        AudioFormat {
            sample_rate: 16_000,
            channels: 1,
            encoding: AudioEncoding::I16,
        }
    }

    #[test]
    fn configures_nova_interims_punctuation_manual_completion_and_plain_keyterms() {
        let url = deepgram_url(
            &i16_format(),
            &["QuickText".to_string(), "speech API".to_string()],
        )
        .unwrap();
        assert_eq!(url, "wss://api.deepgram.com/v1/listen?model=nova-3&interim_results=true&endpointing=false&punctuate=true&encoding=linear16&sample_rate=16000&channels=1&keyterm=QuickText&keyterm=speech%20API");
    }

    #[test]
    fn maps_interim_and_final_results_without_using_speech_final() {
        let mut transcript = TranscriptAccumulator::default();
        let interim = handle_text_response(
            r#"{"type":"Results","is_final":false,"speech_final":true,"channel":{"alternatives":[{"transcript":"hello wor"}]}}"#,
            &mut transcript,
        )
        .unwrap();
        assert_eq!(
            interim,
            ResponseOutcome::Update(PartialTranscript {
                final_text: String::new(),
                partial_text: "hello wor".to_string()
            })
        );

        let finalized = handle_text_response(
            r#"{"type":"Results","is_final":true,"speech_final":false,"channel":{"alternatives":[{"transcript":"hello world"}]}}"#,
            &mut transcript,
        )
        .unwrap();
        assert_eq!(
            finalized,
            ResponseOutcome::Update(PartialTranscript {
                final_text: "hello world".to_string(),
                partial_text: String::new()
            })
        );
        assert_eq!(
            handle_text_response(r#"{"type":"Metadata"}"#, &mut transcript).unwrap(),
            ResponseOutcome::Complete
        );
    }

    #[test]
    fn replaces_duplicate_final_segments_by_start_time() {
        let mut transcript = TranscriptAccumulator::default();
        handle_text_response(
            r#"{"type":"Results","is_final":true,"start":0.0,"duration":1.0,"channel":{"alternatives":[{"transcript":"hello"}]}}"#,
            &mut transcript,
        )
        .unwrap();
        let update = handle_text_response(
            r#"{"type":"Results","is_final":true,"start":0.0,"duration":1.0,"channel":{"alternatives":[{"transcript":"hello world"}]}}"#,
            &mut transcript,
        )
        .unwrap();

        assert_eq!(
            update,
            ResponseOutcome::Update(PartialTranscript {
                final_text: "hello world".to_string(),
                partial_text: String::new(),
            })
        );
    }

    #[test]
    fn provider_errors_do_not_expose_raw_descriptions() {
        let mut transcript = TranscriptAccumulator::default();
        let error = handle_text_response(
            r#"{"type":"Error","description":"secret provider detail"}"#,
            &mut transcript,
        )
        .unwrap_err();

        assert_eq!(
            error,
            "Deepgram could not process the transcription stream."
        );
        assert!(!error.contains("secret provider detail"));
    }

    #[test]
    fn converts_unsigned_pcm_to_signed_little_endian() {
        let format = AudioFormat {
            sample_rate: 16_000,
            channels: 1,
            encoding: AudioEncoding::U16,
        };
        assert_eq!(
            normalize_audio(vec![0, 0, 0, 128, 255, 255], &format),
            vec![0, 128, 0, 0, 255, 127]
        );
    }

    #[test]
    fn deepgram_connection_keeps_only_ipv4_addresses() {
        let addresses = [
            "[2606:4700::6812:16bf]:443".parse().unwrap(),
            "104.18.23.191:443".parse().unwrap(),
            "104.18.22.191:443".parse().unwrap(),
        ];

        assert_eq!(
            ipv4_addresses(addresses),
            vec![
                "104.18.23.191:443".parse().unwrap(),
                "104.18.22.191:443".parse().unwrap(),
            ]
        );
    }

    #[test]
    fn rejects_more_than_one_hundred_keyterms() {
        assert!(deepgram_url(&i16_format(), &vec!["term".to_string(); 101]).is_err());
    }

    #[test]
    fn rejects_more_than_five_hundred_keyterm_tokens() {
        let oversized_phrase = vec!["word"; 501].join(" ");
        assert!(deepgram_url(&i16_format(), &[oversized_phrase]).is_err());
    }
    async fn mock_session() -> (
        DeepgramSession,
        tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream>,
    ) {
        let (socket, server) = crate::transcription::mock_socket_pair().await;
        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (partial_tx, partial_rx) = mpsc::unbounded_channel();
        let stream_error = Arc::new(Mutex::new(None));
        let error = Arc::clone(&stream_error);
        let task = tokio::spawn(run_session(
            socket,
            i16_format(),
            audio_rx,
            command_rx,
            partial_tx,
            error,
        ));
        (
            DeepgramSession {
                audio_tx,
                command_tx,
                partial_rx: Some(partial_rx),
                ready_rx: None,
                stream_error,
                failure_rx: None,
                _task: ProviderTask(Some(tauri::async_runtime::JoinHandle::Tokio(task))),
            },
            server,
        )
    }

    #[tokio::test]
    async fn stop_drains_audio_and_collects_trailing_results_before_graceful_close() {
        let (session, mut server) = mock_session().await;
        let sender = session.audio_sender();
        sender.send(vec![1, 2]).unwrap();
        sender.send(vec![3, 4]).unwrap();
        drop(sender);
        let stop = tokio::spawn(session.finish());
        assert_eq!(
            server.next().await.unwrap().unwrap(),
            Message::Binary(vec![1, 2].into())
        );
        assert_eq!(
            server.next().await.unwrap().unwrap(),
            Message::Binary(vec![3, 4].into())
        );
        assert_eq!(
            server.next().await.unwrap().unwrap(),
            Message::Text(r#"{"type":"CloseStream"}"#.into())
        );
        server.send(Message::Text(r#"{"type":"Results","is_final":true,"start":0,"channel":{"alternatives":[{"transcript":"last words"}]}}"#.into())).await.unwrap();
        server
            .send(Message::Text(r#"{"type":"Metadata"}"#.into()))
            .await
            .unwrap();
        // Metadata alone must not publish a transcript before closure.
        tokio::task::yield_now().await;
        assert!(!stop.is_finished());
        server.close(None).await.unwrap();
        let result = stop.await.unwrap().unwrap();
        assert_eq!(result.text, "last words");
        assert_eq!(result.provider, "deepgram");
    }

    #[tokio::test]
    async fn abnormal_close_after_metadata_is_an_error() {
        use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};
        let (session, mut server) = mock_session().await;
        let stop = tokio::spawn(session.finish());
        server.next().await.unwrap().unwrap();
        server
            .send(Message::Text(r#"{"type":"Metadata"}"#.into()))
            .await
            .unwrap();
        server
            .close(Some(CloseFrame {
                code: CloseCode::Error,
                reason: "private provider detail".into(),
            }))
            .await
            .unwrap();
        let error = stop.await.unwrap().unwrap_err();
        assert!(error.contains("before successful completion"));
        assert!(!error.contains("private provider detail"));
    }

    #[tokio::test]
    async fn close_without_metadata_never_returns_partial_text_as_success() {
        let (session, mut server) = mock_session().await;
        let stop = tokio::spawn(session.finish());
        server.next().await.unwrap().unwrap();
        server.send(Message::Text(r#"{"type":"Results","is_final":true,"start":0,"channel":{"alternatives":[{"transcript":"incomplete"}]}}"#.into())).await.unwrap();
        server.close(None).await.unwrap();
        assert!(stop.await.unwrap().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_drops_the_socket_even_when_audio_drain_is_blocked() {
        let (session, mut server) = mock_session().await;
        let _held_sender = session.audio_sender(); // Prevent drain from completing.
        let error = session.finish().await.unwrap_err();
        assert!(error.contains("finalization timed out"));
        assert!(matches!(server.next().await, Some(Err(_)) | None));
    }

    #[tokio::test]
    async fn cancel_drops_the_socket_without_finalizing_or_publishing_text() {
        let (mut session, mut server) = mock_session().await;
        let mut updates = session.take_partial_receiver().unwrap();
        Box::new(session).cancel();
        assert!(matches!(server.next().await, Some(Err(_)) | None));
        assert!(updates.recv().await.is_none());
    }

    #[tokio::test]
    async fn failure_while_recording_is_retained_for_the_stop_path() {
        let (session, mut server) = mock_session().await;
        let error = Arc::clone(&session.stream_error);
        server.close(None).await.unwrap();
        // The task exits and closes its audio receiver on failure.
        session.audio_tx.closed().await;
        assert!(stored_error(&error)
            .unwrap()
            .contains("before successful completion"));
        assert!(session.finish().await.is_err());
    }
    #[test]
    fn malformed_responses_do_not_expose_payload_values() {
        let error = handle_text_response(
            r#"{"type":"Results","is_final":"private payload"}"#,
            &mut TranscriptAccumulator::default(),
        )
        .unwrap_err();
        assert!(!error.contains("private payload"));
        assert!(error.contains("invalid transcription response"));
    }
}
