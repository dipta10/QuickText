use std::{fs, io::ErrorKind, path::PathBuf};

use crate::settings_file::write_json;

const HISTORY_LIMIT: usize = 50;

#[derive(Clone, Default, serde::Serialize)]
pub struct TranscriptHistorySnapshot {
    entries: Vec<String>,
    warning: Option<&'static str>,
}

pub struct TranscriptHistory {
    path: Option<PathBuf>,
    snapshot: TranscriptHistorySnapshot,
}

impl TranscriptHistory {
    pub fn load(path: Option<PathBuf>) -> Self {
        let entries = path
            .as_ref()
            .ok_or(())
            .and_then(|path| match fs::read(path) {
                Ok(contents) => serde_json::from_slice::<Vec<String>>(&contents).map_err(|_| ()),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
                Err(_) => Err(()),
            });

        match entries {
            Ok(mut entries) => {
                entries.truncate(HISTORY_LIMIT);
                Self {
                    path,
                    snapshot: TranscriptHistorySnapshot {
                        entries,
                        warning: None,
                    },
                }
            }
            // Keep an unreadable archive intact rather than replacing it with an empty one.
            Err(()) => Self {
                path: None,
                snapshot: TranscriptHistorySnapshot {
                    entries: Vec::new(),
                    warning: Some("Could not load saved history. New transcripts will be kept for this session only."),
                },
            },
        }
    }

    pub fn snapshot(&self) -> TranscriptHistorySnapshot {
        self.snapshot.clone()
    }

    pub fn record(&mut self, text: String) {
        if text.trim().is_empty() {
            return;
        }

        self.snapshot.entries.insert(0, text);
        self.snapshot.entries.truncate(HISTORY_LIMIT);

        if let Some(path) = &self.path {
            self.snapshot.warning = write_json(path, &self.snapshot.entries).err().map(|_| {
                "Could not save transcript history. Recent changes may be lost when you quit."
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn test_path() -> PathBuf {
        static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "quicktext-transcript-history-test-{}-{}",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        directory.join("transcript-history.json")
    }

    #[test]
    fn retains_the_newest_fifty_nonblank_transcripts_across_restarts() {
        let path = test_path();
        let mut history = TranscriptHistory::load(Some(path.clone()));
        for index in 0..52 {
            history.record(format!("Transcript {index}"));
        }
        history.record(" \n ".to_string());

        let restored = TranscriptHistory::load(Some(path.clone())).snapshot();
        let expected: Vec<_> = (2..52).rev().map(|i| format!("Transcript {i}")).collect();
        assert_eq!(restored.entries, expected);
        assert!(restored.warning.is_none());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn preserves_an_unreadable_file_and_keeps_new_transcripts_in_memory() {
        let path = test_path();
        fs::write(&path, "invalid JSON").unwrap();
        let mut history = TranscriptHistory::load(Some(path.clone()));
        history.record("New transcript".to_string());

        assert_eq!(history.snapshot().entries, ["New transcript"]);
        assert!(history.snapshot().warning.is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), "invalid JSON");
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
