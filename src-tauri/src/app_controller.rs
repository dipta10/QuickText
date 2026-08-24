use serde::Serialize;

use crate::audio_recorder::{AudioCaptureStats, AudioFormat};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppStatus {
    Idle,
    Starting,
    Recording,
    Stopping,
    Transcribed,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TranscriptResult {
    pub text: String,
    pub provider: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppError {
    MissingApiKey { message: String },
    CredentialStore { message: String },
    MicrophoneUnavailable { message: String },
    ProviderUnavailable { message: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub status: AppStatus,
    pub transcript: Option<TranscriptResult>,
    pub error: Option<AppError>,
    pub audio_format: Option<AudioFormat>,
    pub audio_stats: Option<AudioCaptureStats>,
}

#[derive(Debug)]
pub struct AppController {
    snapshot: AppSnapshot,
    active_session_id: Option<u64>,
    next_session_id: u64,
}

impl Default for AppController {
    fn default() -> Self {
        Self {
            snapshot: AppSnapshot {
                status: AppStatus::Idle,
                transcript: None,
                error: None,
                audio_format: None,
                audio_stats: None,
            },
            active_session_id: None,
            next_session_id: 1,
        }
    }
}

impl AppController {
    pub fn snapshot(&self) -> AppSnapshot {
        self.snapshot.clone()
    }

    pub fn active_session_id(&self) -> Option<u64> {
        self.active_session_id
    }

    pub fn is_recording_session(&self, session_id: u64) -> bool {
        self.snapshot.status == AppStatus::Recording && self.active_session_id == Some(session_id)
    }

    pub fn begin_start(&mut self) -> AppSnapshot {
        match self.snapshot.status {
            AppStatus::Idle | AppStatus::Transcribed | AppStatus::Error => {
                let session_id = self.next_session_id;
                self.next_session_id += 1;
                self.active_session_id = Some(session_id);
                self.snapshot = AppSnapshot {
                    status: AppStatus::Starting,
                    transcript: None,
                    error: None,
                    audio_format: None,
                    audio_stats: None,
                };
            }
            AppStatus::Starting | AppStatus::Recording | AppStatus::Stopping => {}
        }

        self.snapshot()
    }

    pub fn finish_start(&mut self, audio_format: AudioFormat) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Starting {
            self.snapshot = AppSnapshot {
                status: AppStatus::Recording,
                transcript: None,
                error: None,
                audio_format: Some(audio_format),
                audio_stats: None,
            };
        }

        self.snapshot()
    }

    pub fn fail_start(&mut self, error: AppError) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Starting {
            self.snapshot = AppSnapshot {
                status: AppStatus::Error,
                transcript: None,
                error: Some(error),
                audio_format: None,
                audio_stats: None,
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }

    pub fn fail_recording(&mut self, error: AppError) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Recording {
            self.snapshot = AppSnapshot {
                status: AppStatus::Error,
                transcript: None,
                error: Some(error),
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: None,
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }

    pub fn fail_stop(&mut self, error: AppError) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Stopping {
            self.snapshot = AppSnapshot {
                status: AppStatus::Error,
                transcript: None,
                error: Some(error),
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: self.snapshot.audio_stats.clone(),
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }

    pub fn begin_stop(&mut self) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Recording {
            self.snapshot = AppSnapshot {
                status: AppStatus::Stopping,
                transcript: None,
                error: None,
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: None,
            };
        }

        self.snapshot()
    }

    pub fn finish_stop(
        &mut self,
        transcript: TranscriptResult,
        audio_stats: AudioCaptureStats,
    ) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Stopping {
            self.snapshot = AppSnapshot {
                status: AppStatus::Transcribed,
                transcript: Some(transcript),
                error: None,
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: Some(audio_stats),
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }
}

#[cfg(test)]
fn fake_transcript(audio_stats: &AudioCaptureStats) -> TranscriptResult {
    TranscriptResult {
        text: format!(
            "Captured {} audio chunks ({} samples, {} bytes). Soniox streaming is next.",
            audio_stats.chunk_count, audio_stats.sample_count, audio_stats.byte_count
        ),
        provider: "fake".to_string(),
    }
}

#[cfg(test)]
fn test_audio_format() -> AudioFormat {
    AudioFormat {
        sample_rate: 48_000,
        channels: 1,
        encoding: crate::audio_recorder::AudioEncoding::F32,
    }
}

#[cfg(test)]
fn test_audio_stats() -> AudioCaptureStats {
    AudioCaptureStats {
        chunk_count: 2,
        sample_count: 960,
        byte_count: 3_840,
        elapsed_ms: 20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_from_idle() {
        let mut controller = AppController::default();

        assert_eq!(controller.begin_start().status, AppStatus::Starting);
        assert_eq!(controller.active_session_id(), Some(1));
        let snapshot = controller.finish_start(test_audio_format());
        assert_eq!(snapshot.status, AppStatus::Recording);
        assert_eq!(snapshot.audio_format, Some(test_audio_format()));
    }

    #[test]
    fn ignores_duplicate_start_while_recording() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start(test_audio_format());

        assert_eq!(controller.begin_start().status, AppStatus::Recording);
    }

    #[test]
    fn stops_recording_with_transcript() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start(test_audio_format());
        assert_eq!(controller.begin_stop().status, AppStatus::Stopping);

        let audio_stats = test_audio_stats();
        let snapshot = controller.finish_stop(fake_transcript(&audio_stats), audio_stats);

        assert_eq!(snapshot.status, AppStatus::Transcribed);
        assert_eq!(
            snapshot.transcript.map(|transcript| transcript.provider),
            Some("fake".to_string())
        );
        assert_eq!(controller.active_session_id(), None);
        assert_eq!(snapshot.audio_stats, Some(test_audio_stats()));
    }

    #[test]
    fn missing_key_moves_starting_to_error() {
        let mut controller = AppController::default();

        controller.begin_start();
        let snapshot = controller.fail_start(AppError::MissingApiKey {
            message: "Add your Soniox API key before recording.".to_string(),
        });

        assert_eq!(snapshot.status, AppStatus::Error);
        assert_eq!(controller.active_session_id(), None);
        assert!(matches!(
            snapshot.error,
            Some(AppError::MissingApiKey { .. })
        ));
    }

    #[test]
    fn credential_store_error_moves_starting_to_error() {
        let mut controller = AppController::default();

        controller.begin_start();
        let snapshot = controller.fail_start(AppError::CredentialStore {
            message: "Could not read Soniox API key.".to_string(),
        });

        assert_eq!(snapshot.status, AppStatus::Error);
        assert_eq!(controller.active_session_id(), None);
        assert!(matches!(
            snapshot.error,
            Some(AppError::CredentialStore { .. })
        ));
    }

    #[test]
    fn microphone_error_moves_starting_to_error() {
        let mut controller = AppController::default();

        controller.begin_start();
        let snapshot = controller.fail_start(AppError::MicrophoneUnavailable {
            message: "No microphone input device was found.".to_string(),
        });

        assert_eq!(snapshot.status, AppStatus::Error);
        assert_eq!(controller.active_session_id(), None);
        assert!(matches!(
            snapshot.error,
            Some(AppError::MicrophoneUnavailable { .. })
        ));
    }

    #[test]
    fn provider_error_moves_stopping_to_error() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start(test_audio_format());
        controller.begin_stop();
        let snapshot = controller.fail_stop(AppError::ProviderUnavailable {
            message: "Soniox finalization failed.".to_string(),
        });

        assert_eq!(snapshot.status, AppStatus::Error);
        assert_eq!(controller.active_session_id(), None);
        assert!(matches!(
            snapshot.error,
            Some(AppError::ProviderUnavailable { .. })
        ));
    }

    #[test]
    fn provider_error_during_recording_moves_to_error() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start(test_audio_format());
        let snapshot = controller.fail_recording(AppError::ProviderUnavailable {
            message: "Could not connect to Soniox.".to_string(),
        });

        assert_eq!(snapshot.status, AppStatus::Error);
        assert_eq!(controller.active_session_id(), None);
        assert_eq!(snapshot.audio_format, Some(test_audio_format()));
        assert!(matches!(
            snapshot.error,
            Some(AppError::ProviderUnavailable { .. })
        ));
    }

    #[test]
    fn provider_failure_ignores_non_recording_status() {
        let mut controller = AppController::default();

        controller.begin_start();
        let snapshot = controller.fail_recording(AppError::ProviderUnavailable {
            message: "Could not connect to Soniox.".to_string(),
        });

        assert_eq!(snapshot.status, AppStatus::Starting);
        assert_ne!(controller.active_session_id(), None);
    }

    #[test]
    fn stale_session_id_does_not_match_new_recording() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start(test_audio_format());
        let first_session_id = controller.active_session_id().unwrap();
        controller.begin_stop();
        let audio_stats = test_audio_stats();
        controller.finish_stop(fake_transcript(&audio_stats), audio_stats);

        controller.begin_start();
        controller.finish_start(test_audio_format());

        assert!(!controller.is_recording_session(first_session_id));
        assert!(controller.is_recording_session(2));
    }
}
