use std::{future::Future, pin::Pin, str::FromStr};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::{app_controller::TranscriptResult, audio_recorder::AudioFormat};

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
        }

        assert!(ProviderId::from_str("other").is_err());
    }
}
