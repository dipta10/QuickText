use serde::Serialize;

use crate::audio_recorder::{AudioCaptureStats, AudioFormat};
use crate::transcript_cleanup::{validate_input, CleanupError};

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
    MissingApiKey { provider: String, message: String },
    CredentialStore { message: String },
    MicrophoneUnavailable { message: String },
    ProviderUnavailable { message: String },
    PasteFailed { message: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSnapshot {
    pub revision: u64,
    pub running: bool,
    pub cleaned: bool,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct CleanupRequest {
    session_id: u64,
    revision: u64,
    pub original: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub status: AppStatus,
    pub session_id: Option<u64>,
    pub transcript: Option<TranscriptResult>,
    pub error: Option<AppError>,
    pub audio_format: Option<AudioFormat>,
    pub audio_stats: Option<AudioCaptureStats>,
    pub cleanup: CleanupSnapshot,
}

#[derive(Debug)]
pub struct AppController {
    snapshot: AppSnapshot,
    active_session_id: Option<u64>,
    next_session_id: u64,
    original_transcript: Option<String>,
}

impl Default for AppController {
    fn default() -> Self {
        Self {
            snapshot: AppSnapshot {
                status: AppStatus::Idle,
                session_id: None,
                transcript: None,
                error: None,
                audio_format: None,
                audio_stats: None,
                cleanup: CleanupSnapshot::default(),
            },
            active_session_id: None,
            next_session_id: 1,
            original_transcript: None,
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
                self.original_transcript = None;
                let session_id = self.next_session_id;
                self.next_session_id += 1;
                self.active_session_id = Some(session_id);
                self.snapshot = AppSnapshot {
                    status: AppStatus::Starting,
                    session_id: self.active_session_id,
                    transcript: None,
                    error: None,
                    audio_format: None,
                    audio_stats: None,
                    cleanup: CleanupSnapshot::default(),
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
                session_id: self.active_session_id,
                transcript: None,
                error: None,
                audio_format: Some(audio_format),
                audio_stats: None,
                cleanup: CleanupSnapshot::default(),
            };
        }

        self.snapshot()
    }

    pub fn fail_start(&mut self, error: AppError) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Starting {
            self.snapshot = AppSnapshot {
                status: AppStatus::Error,
                session_id: self.active_session_id,
                transcript: None,
                error: Some(error),
                audio_format: None,
                audio_stats: None,
                cleanup: CleanupSnapshot::default(),
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }

    pub fn fail_recording(&mut self, error: AppError) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Recording {
            self.snapshot = AppSnapshot {
                status: AppStatus::Error,
                session_id: self.active_session_id,
                transcript: None,
                error: Some(error),
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: None,
                cleanup: CleanupSnapshot::default(),
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }

    pub fn fail_stop(&mut self, error: AppError) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Stopping {
            self.snapshot = AppSnapshot {
                status: AppStatus::Error,
                session_id: self.active_session_id,
                transcript: None,
                error: Some(error),
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: self.snapshot.audio_stats.clone(),
                cleanup: CleanupSnapshot::default(),
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }

    pub fn begin_stop(&mut self) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Recording {
            self.snapshot = AppSnapshot {
                status: AppStatus::Stopping,
                session_id: self.active_session_id,
                transcript: None,
                error: None,
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: None,
                cleanup: CleanupSnapshot::default(),
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
            self.original_transcript = Some(transcript.text.clone());
            self.snapshot = AppSnapshot {
                status: AppStatus::Transcribed,
                session_id: self.active_session_id,
                transcript: Some(transcript),
                error: None,
                audio_format: self.snapshot.audio_format.clone(),
                audio_stats: Some(audio_stats),
                cleanup: CleanupSnapshot::default(),
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }

    pub fn begin_cleanup(&mut self) -> Result<CleanupRequest, CleanupError> {
        if self.snapshot.cleanup.running {
            return Err(CleanupError::Busy);
        }
        if !matches!(
            self.snapshot.status,
            AppStatus::Transcribed | AppStatus::Error
        ) {
            return Err(CleanupError::InvalidResponse);
        }
        let original = self
            .original_transcript
            .clone()
            .ok_or(CleanupError::InvalidResponse)?;
        validate_input(&original)?;
        let session_id = self
            .snapshot
            .session_id
            .ok_or(CleanupError::InvalidResponse)?;
        self.snapshot.cleanup.revision += 1;
        self.snapshot.cleanup.running = true;
        self.snapshot.cleanup.message = "Cleaning…".into();
        Ok(CleanupRequest {
            session_id,
            revision: self.snapshot.cleanup.revision,
            original,
        })
    }

    /// The caller holds the controller lock through acceptance and clipboard write,
    /// so a new recording cannot start between checking identity and delivery.
    pub fn finish_cleanup(
        &mut self,
        request: &CleanupRequest,
        result: Result<String, CleanupError>,
        mut copy: impl FnMut(&str) -> Result<(), ()>,
    ) -> bool {
        if self.snapshot.session_id != Some(request.session_id)
            || self.snapshot.cleanup.revision != request.revision
            || !self.snapshot.cleanup.running
        {
            return false;
        }
        self.snapshot.cleanup.running = false;
        self.snapshot.cleanup.revision += 1;
        match result {
            Ok(text) if !text.trim().is_empty() => {
                if let Some(transcript) = self.snapshot.transcript.as_mut() {
                    transcript.text = text.clone();
                    self.snapshot.cleanup.cleaned = true;
                    self.snapshot.cleanup.message = if copy(&text).is_ok() {
                        "Cleaned and copied.".into()
                    } else {
                        "Cleaned, but couldn't copy. Use Copy.".into()
                    };
                }
            }
            Ok(_) => self.snapshot.cleanup.message = CleanupError::InvalidResponse.message().into(),
            Err(error) => self.snapshot.cleanup.message = error.message().into(),
        }
        true
    }

    pub fn restore_original(&mut self) -> Result<AppSnapshot, CleanupError> {
        if self.snapshot.cleanup.running {
            return Err(CleanupError::Busy);
        }
        let original = self
            .original_transcript
            .clone()
            .ok_or(CleanupError::InvalidResponse)?;
        let transcript = self
            .snapshot
            .transcript
            .as_mut()
            .ok_or(CleanupError::InvalidResponse)?;
        transcript.text = original;
        self.snapshot.cleanup.cleaned = false;
        self.snapshot.cleanup.revision += 1;
        self.snapshot.cleanup.message = "Original restored. Press Copy to copy it.".into();
        Ok(self.snapshot())
    }

    pub fn report_paste_failure(&mut self, error: AppError) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Transcribed {
            self.snapshot.status = AppStatus::Error;
            self.snapshot.error = Some(error);
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

    fn completed_controller() -> AppController {
        let mut controller = AppController::default();
        controller.begin_start();
        controller.finish_start(test_audio_format());
        controller.begin_stop();
        controller.finish_stop(
            TranscriptResult {
                text: "Um, move it, move it to Friday.".into(),
                provider: "soniox".into(),
            },
            test_audio_stats(),
        );
        controller
    }

    struct FakeCleaner(Result<String, CleanupError>);
    impl crate::transcript_cleanup::TranscriptCleaner for FakeCleaner {
        fn clean<'a>(&'a self, _: &'a str) -> crate::transcript_cleanup::CleanupFuture<'a> {
            Box::pin(async { self.0.clone() })
        }
    }

    #[tokio::test]
    async fn cleanup_uses_replaceable_cleaner_and_copies_once_then_restores_without_delivery() {
        use crate::transcript_cleanup::TranscriptCleaner;
        let mut controller = completed_controller();
        let request = controller.begin_cleanup().unwrap();
        assert_eq!(controller.begin_cleanup().unwrap_err(), CleanupError::Busy);
        assert_eq!(
            controller.restore_original().unwrap_err(),
            CleanupError::Busy
        );
        let cleaner: &dyn TranscriptCleaner = &FakeCleaner(Ok("Move it to Friday.".into()));
        let result = cleaner.clean(&request.original).await;
        let mut clipboard = Vec::new();
        assert!(controller.finish_cleanup(&request, result, |text| {
            clipboard.push(text.to_string());
            Ok(())
        }));
        assert_eq!(clipboard, ["Move it to Friday."]);
        assert_eq!(
            controller.snapshot().transcript.unwrap().text,
            "Move it to Friday."
        );
        // Re-delivering the same result cannot write twice.
        assert!(
            !controller.finish_cleanup(&request, Ok("duplicate".into()), |_| panic!(
                "duplicate write"
            ))
        );
        let next = controller.begin_cleanup().unwrap();
        assert_eq!(next.original, "Um, move it, move it to Friday.");
        controller.finish_cleanup(&next, Err(CleanupError::Timeout), |_| {
            panic!("failed cleanup copied")
        });
        assert_eq!(
            controller.snapshot().transcript.unwrap().text,
            "Move it to Friday."
        );
        let restored = controller.restore_original().unwrap();
        assert_eq!(restored.transcript.unwrap().text, request.original);
        assert!(!restored.cleanup.cleaned);
        assert_eq!(clipboard.len(), 1);
    }

    #[test]
    fn new_recording_invalidates_pending_cleanup_even_if_another_cleanup_has_started() {
        let mut controller = completed_controller();
        let stale = controller.begin_cleanup().unwrap();
        controller.begin_start();
        assert!(!controller.snapshot().cleanup.running);
        controller.finish_start(test_audio_format());
        controller.begin_stop();
        controller.finish_stop(
            TranscriptResult {
                text: "New dictation.".into(),
                provider: "deepgram".into(),
            },
            test_audio_stats(),
        );
        let current = controller.begin_cleanup().unwrap();
        assert!(
            !controller.finish_cleanup(&stale, Ok("stale".into()), |_| panic!(
                "stale clipboard write"
            ))
        );
        assert!(controller.snapshot().cleanup.running);
        assert_eq!(
            controller.snapshot().transcript.unwrap().text,
            "New dictation."
        );
        assert!(
            controller.finish_cleanup(&current, Ok("New cleaned dictation.".into()), |_| Ok(()))
        );
    }

    #[test]
    fn failed_provider_preserves_original_and_failed_copy_keeps_cleaned_result() {
        let mut controller = completed_controller();
        let original = controller.snapshot().transcript.unwrap();
        let request = controller.begin_cleanup().unwrap();
        controller.finish_cleanup(&request, Err(CleanupError::InvalidKey), |_| {
            panic!("failure wrote clipboard")
        });
        assert_eq!(controller.snapshot().transcript, Some(original));
        assert!(!controller.snapshot().cleanup.cleaned);
        let retry = controller.begin_cleanup().unwrap();
        controller.finish_cleanup(&retry, Ok("Move it to Friday.".into()), |_| Err(()));
        let snapshot = controller.snapshot();
        assert!(snapshot.cleanup.cleaned);
        assert_eq!(
            snapshot.cleanup.message,
            "Cleaned, but couldn't copy. Use Copy."
        );
        assert_eq!(snapshot.status, AppStatus::Transcribed);
    }

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
            provider: "soniox".to_string(),
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

    #[test]
    fn paste_failure_keeps_transcript_but_marks_error() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start(test_audio_format());
        controller.begin_stop();
        let audio_stats = test_audio_stats();
        controller.finish_stop(fake_transcript(&audio_stats), audio_stats.clone());

        let snapshot = controller.report_paste_failure(AppError::PasteFailed {
            message: "Could not paste into the focused app.".to_string(),
        });

        assert_eq!(snapshot.status, AppStatus::Error);
        assert_eq!(
            snapshot.transcript.map(|transcript| transcript.text),
            Some(fake_transcript(&audio_stats).text)
        );
        assert_eq!(snapshot.audio_stats, Some(audio_stats));
    }
}
