use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_WRITE_ID: AtomicU64 = AtomicU64::new(1);

// Serialize before touching the destination, then replace it in one rename.
// A failed write must leave the user's last saved settings intact.
pub fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let contents = serde_json::to_vec_pretty(value)
        .map_err(|_| "Could not serialize transcription settings.".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Could not resolve the settings folder.".to_string())?;
    fs::create_dir_all(parent).map_err(|_| "Could not create the settings folder.".to_string())?;
    let temporary = parent.join(format!(
        ".quicktext-settings-{}-{}.tmp",
        std::process::id(),
        NEXT_WRITE_ID.fetch_add(1, Ordering::Relaxed),
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "Could not save transcription settings.".to_string())?;
    let result = (|| {
        file.write_all(&contents)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|_| "Could not save transcription settings.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomically_replaces_existing_settings_and_removes_temporary_files() {
        let directory = std::env::temp_dir().join(format!(
            "quicktext-settings-test-{}-{}",
            std::process::id(),
            NEXT_WRITE_ID.fetch_add(1, Ordering::Relaxed),
        ));
        let path = directory.join("settings.json");
        write_json(&path, &serde_json::json!({"activeProvider":"soniox"})).unwrap();
        write_json(&path, &serde_json::json!({"activeProvider":"deepgram"})).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap()
                ["activeProvider"],
            "deepgram"
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn serialization_failure_preserves_saved_settings() {
        struct Invalid;
        impl serde::Serialize for Invalid {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("private settings content"))
            }
        }
        let path = std::env::temp_dir().join(format!(
            "quicktext-settings-failure-{}-{}.json",
            std::process::id(),
            NEXT_WRITE_ID.fetch_add(1, Ordering::Relaxed),
        ));
        fs::write(&path, "previous settings").unwrap();
        let error = write_json(&path, &Invalid).unwrap_err();
        assert!(!error.contains("private settings content"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "previous settings");
        fs::remove_file(path).unwrap();
    }
}
