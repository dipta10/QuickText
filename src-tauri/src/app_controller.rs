use serde::Serialize;

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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub status: AppStatus,
    pub transcript: Option<TranscriptResult>,
    pub error: Option<AppError>,
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
                };
            }
            AppStatus::Starting | AppStatus::Recording | AppStatus::Stopping => {}
        }

        self.snapshot()
    }

    pub fn finish_start(&mut self) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Starting {
            self.snapshot = AppSnapshot {
                status: AppStatus::Recording,
                transcript: None,
                error: None,
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
            };
        }

        self.snapshot()
    }

    pub fn finish_stop(&mut self, transcript: TranscriptResult) -> AppSnapshot {
        if self.snapshot.status == AppStatus::Stopping {
            self.snapshot = AppSnapshot {
                status: AppStatus::Transcribed,
                transcript: Some(transcript),
                error: None,
            };
            self.active_session_id = None;
        }

        self.snapshot()
    }
}

pub fn fake_transcript() -> TranscriptResult {
    TranscriptResult {
        text: "Fake transcript from backend controller. Soniox streaming is next.".to_string(),
        provider: "fake".to_string(),
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
        assert_eq!(controller.finish_start().status, AppStatus::Recording);
    }

    #[test]
    fn ignores_duplicate_start_while_recording() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start();

        assert_eq!(controller.begin_start().status, AppStatus::Recording);
    }

    #[test]
    fn stops_recording_with_transcript() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start();
        assert_eq!(controller.begin_stop().status, AppStatus::Stopping);

        let snapshot = controller.finish_stop(fake_transcript());

        assert_eq!(snapshot.status, AppStatus::Transcribed);
        assert_eq!(
            snapshot.transcript.map(|transcript| transcript.provider),
            Some("fake".to_string())
        );
        assert_eq!(controller.active_session_id(), None);
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
    fn stale_session_id_does_not_match_new_recording() {
        let mut controller = AppController::default();

        controller.begin_start();
        controller.finish_start();
        let first_session_id = controller.active_session_id().unwrap();
        controller.begin_stop();
        controller.finish_stop(fake_transcript());

        controller.begin_start();
        controller.finish_start();

        assert!(!controller.is_recording_session(first_session_id));
        assert!(controller.is_recording_session(2));
    }
}
