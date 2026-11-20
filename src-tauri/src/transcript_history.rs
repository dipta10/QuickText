use std::{
    fs,
    io::ErrorKind,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::settings_file::write_json;

const HISTORY_LIMIT: usize = 300;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryEntry {
    text: String,
    recorded_at_ms: Option<u64>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum StoredEntry {
    Dated(HistoryEntry),
    Legacy(String),
}

impl From<StoredEntry> for HistoryEntry {
    fn from(entry: StoredEntry) -> Self {
        match entry {
            StoredEntry::Dated(entry) => entry,
            StoredEntry::Legacy(text) => Self {
                text,
                recorded_at_ms: None,
            },
        }
    }
}

#[derive(Clone, Default, serde::Serialize)]
pub struct TranscriptHistorySnapshot {
    entries: Vec<HistoryEntry>,
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
                Ok(contents) => {
                    serde_json::from_slice::<Vec<StoredEntry>>(&contents).map_err(|_| ())
                }
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
                Err(_) => Err(()),
            });

        match entries {
            Ok(mut entries) => {
                entries.truncate(HISTORY_LIMIT);
                Self {
                    path,
                    snapshot: TranscriptHistorySnapshot {
                        entries: entries.into_iter().map(HistoryEntry::from).collect(),
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

        let recorded_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| duration.as_millis().try_into().ok());
        self.snapshot.entries.insert(
            0,
            HistoryEntry {
                text,
                recorded_at_ms,
            },
        );
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
    fn retains_the_newest_300_nonblank_transcripts_across_restarts() {
        let path = test_path();
        let mut history = TranscriptHistory::load(Some(path.clone()));
        for index in 0..302 {
            history.record(format!("Transcript {index}"));
        }
        history.record(" \n ".to_string());

        let restored = TranscriptHistory::load(Some(path.clone())).snapshot();
        let expected: Vec<_> = (2..302).rev().map(|i| format!("Transcript {i}")).collect();
        let text: Vec<_> = restored.entries.iter().map(|entry| &entry.text).collect();
        assert_eq!(text, expected.iter().collect::<Vec<_>>());
        assert!(restored
            .entries
            .iter()
            .all(|entry| entry.recorded_at_ms.is_some()));
        assert!(restored.warning.is_none());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn preserves_an_unreadable_file_and_keeps_new_transcripts_in_memory() {
        let path = test_path();
        fs::write(&path, "invalid JSON").unwrap();
        let mut history = TranscriptHistory::load(Some(path.clone()));
        history.record("New transcript".to_string());

        assert_eq!(history.snapshot().entries[0].text, "New transcript");
        assert!(history.snapshot().warning.is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), "invalid JSON");
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn retains_legacy_transcripts_without_inventing_timestamps() {
        let path = test_path();
        fs::write(&path, r#"["Legacy transcript"]"#).unwrap();
        let mut history = TranscriptHistory::load(Some(path.clone()));
        history.record("New transcript".into());

        let restored = TranscriptHistory::load(Some(path.clone())).snapshot();
        assert_eq!(restored.entries[0].text, "New transcript");
        assert!(restored.entries[0].recorded_at_ms.is_some());
        assert_eq!(restored.entries[1].text, "Legacy transcript");
        assert!(restored.entries[1].recorded_at_ms.is_none());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
