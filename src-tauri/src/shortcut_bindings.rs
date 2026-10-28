use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use tauri_plugin_global_shortcut::Shortcut;

#[derive(Clone, Copy)]
pub enum Target {
    Recording = 0,
    Window = 1,
}

impl Target {
    const ALL: [Self; 2] = [Self::Recording, Self::Window];
}

pub trait Registry {
    fn register(&self, shortcut: Shortcut, target: Target) -> Result<(), String>;
    fn unregister(&self, shortcut: Shortcut) -> Result<(), String>;
}

// None means this slot predates native persistence and may import localStorage.
// Some("") is an explicitly unassigned slot and must never resurrect an old key.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct StoredBindings {
    pub recording: Option<String>,
    pub window: Option<String>,
}

impl StoredBindings {
    fn slot(&self, target: Target) -> &Option<String> {
        match target {
            Target::Recording => &self.recording,
            Target::Window => &self.window,
        }
    }

    fn set(&mut self, target: Target, value: String) {
        *match target {
            Target::Recording => &mut self.recording,
            Target::Window => &mut self.window,
        } = Some(value);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub recording: String,
    pub window: String,
    pub recording_error: Option<String>,
    pub window_error: Option<String>,
}

#[derive(Default)]
pub struct Bindings {
    saved: StoredBindings,
    active: [Option<Shortcut>; 2],
    errors: [Option<String>; 2],
    load_error: Option<String>,
}

impl Bindings {
    pub fn load(&mut self, path: &Path, registry: &impl Registry) -> Result<(), String> {
        let result = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|_| "Could not read saved shortcuts.".to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(StoredBindings::default())
            }
            Err(_) => Err("Could not read saved shortcuts.".to_string()),
        };
        match result {
            Ok(saved) => self.saved = saved,
            Err(error) => {
                self.load_error = Some(error.clone());
                return Err(error);
            }
        }
        for target in Target::ALL {
            self.restore(target, registry);
        }
        Ok(())
    }

    pub fn initialize(
        &mut self,
        path: &Path,
        legacy: StoredBindings,
        registry: &impl Registry,
    ) -> Result<Snapshot, String> {
        self.ensure_loaded()?;
        let mut migrated = self.saved.clone();
        let missing = Target::ALL.map(|target| self.saved.slot(target).is_none());
        for target in Target::ALL {
            if missing[target as usize] {
                migrated.set(target, legacy.slot(target).clone().unwrap_or_default());
            }
        }
        if missing.contains(&true) {
            crate::settings_file::write_json(path, &migrated)?;
            self.saved = migrated;
            for target in Target::ALL {
                if missing[target as usize] {
                    self.restore(target, registry);
                }
            }
        }
        Ok(self.snapshot())
    }

    pub fn set(
        &mut self,
        path: &Path,
        target: Target,
        shortcut: &str,
        registry: &impl Registry,
    ) -> Result<String, String> {
        self.ensure_loaded()?;
        let shortcut = shortcut.trim();
        let parsed = self.parse(target, shortcut)?;
        let previous = self.active[target as usize];
        if parsed != previous {
            if let Some(new) = parsed {
                registry.register(new, target)?;
            }
            if let Some(old) = previous {
                if let Err(error) = registry.unregister(old) {
                    if let Some(new) = parsed {
                        let _ = registry.unregister(new);
                    }
                    return Err(error);
                }
            }
        }
        let mut saved = self.saved.clone();
        saved.set(target, shortcut.to_string());
        if let Err(error) = crate::settings_file::write_json(path, &saved) {
            // Undo the OS change when saving fails; never report a session-only
            // binding as saved. Keep the durable preference intact.
            if parsed != previous {
                if let Some(new) = parsed {
                    let _ = registry.unregister(new);
                }
                if let Some(old) = previous {
                    if registry.register(old, target).is_err() {
                        self.active[target as usize] = None;
                        self.errors[target as usize] = Some(
                            "Could not restore the previous shortcut. Restart QuickText to retry."
                                .into(),
                        );
                    }
                }
            }
            return Err(error);
        }
        self.active[target as usize] = parsed;
        self.saved = saved;
        self.errors[target as usize] = None;
        Ok(shortcut.to_string())
    }

    fn ensure_loaded(&self) -> Result<(), String> {
        match &self.load_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn parse(&self, target: Target, shortcut: &str) -> Result<Option<Shortcut>, String> {
        if shortcut.is_empty() {
            return Ok(None);
        }
        let parsed = shortcut
            .parse::<Shortcut>()
            .map_err(|error| format!("Invalid shortcut: {error}"))?;
        let other = match target {
            Target::Recording => Target::Window,
            Target::Window => Target::Recording,
        };
        if self
            .saved
            .slot(other)
            .as_deref()
            .and_then(|value| value.parse::<Shortcut>().ok())
            == Some(parsed)
        {
            return Err("Recording and window visibility must use different shortcuts.".into());
        }
        Ok(Some(parsed))
    }

    fn restore(&mut self, target: Target, registry: &impl Registry) {
        let shortcut = self.saved.slot(target).as_deref().unwrap_or_default();
        let result = self.parse(target, shortcut).and_then(|parsed| {
            if let Some(parsed) = parsed {
                registry.register(parsed, target)?;
                self.active[target as usize] = Some(parsed);
            }
            Ok(())
        });
        self.errors[target as usize] = result.err();
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            recording: self.saved.recording.clone().unwrap_or_default(),
            window: self.saved.window.clone().unwrap_or_default(),
            recording_error: self.errors[0].clone(),
            window_error: self.errors[1].clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct TestRegistry {
        keys: RefCell<Vec<Shortcut>>,
        fail_register: Cell<bool>,
        fail_unregister: Cell<bool>,
    }

    impl Registry for TestRegistry {
        fn register(&self, shortcut: Shortcut, _: Target) -> Result<(), String> {
            if self.fail_register.get() {
                return Err("Key unavailable".into());
            }
            assert!(!self.keys.borrow().contains(&shortcut));
            self.keys.borrow_mut().push(shortcut);
            Ok(())
        }
        fn unregister(&self, shortcut: Shortcut) -> Result<(), String> {
            if self.fail_unregister.get() {
                return Err("Could not unregister".into());
            }
            self.keys.borrow_mut().retain(|key| *key != shortcut);
            Ok(())
        }
    }

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "quicktext-shortcuts-{}-{}",
                std::process::id(),
                NEXT_TEST.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> PathBuf {
            self.0.join("shortcuts.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn legacy() -> StoredBindings {
        StoredBindings {
            recording: Some("F9".into()),
            window: Some("F10".into()),
        }
    }

    #[test]
    fn migrates_once_and_native_assignments_override_stale_webview_storage() {
        let fixture = Fixture::new();
        let registry = TestRegistry::default();
        let mut bindings = Bindings::default();
        bindings.load(&fixture.path(), &registry).unwrap();
        let snapshot = bindings
            .initialize(&fixture.path(), legacy(), &registry)
            .unwrap();
        assert_eq!(
            (snapshot.recording.as_str(), snapshot.window.as_str()),
            ("F9", "F10")
        );
        bindings
            .set(&fixture.path(), Target::Window, "F12", &registry)
            .unwrap();
        bindings
            .set(&fixture.path(), Target::Recording, "", &registry)
            .unwrap();
        let restarted_registry = TestRegistry::default();
        let mut restarted = Bindings::default();
        restarted
            .load(&fixture.path(), &restarted_registry)
            .unwrap();
        // Restored before the frontend initializes, including a hidden launch.
        assert_eq!(
            *restarted_registry.keys.borrow(),
            vec!["F12".parse::<Shortcut>().unwrap()]
        );
        let snapshot = restarted
            .initialize(&fixture.path(), legacy(), &restarted_registry)
            .unwrap();
        assert_eq!(snapshot.recording, "");
        assert_eq!(snapshot.window, "F12");
    }

    #[test]
    fn registration_failure_preserves_preferences_and_retries_on_next_launch() {
        let fixture = Fixture::new();
        let registry = TestRegistry::default();
        let mut bindings = Bindings::default();
        bindings
            .initialize(&fixture.path(), legacy(), &registry)
            .unwrap();
        let before = fs::read(fixture.path()).unwrap();
        registry.fail_register.set(true);
        assert!(bindings
            .set(&fixture.path(), Target::Window, "F12", &registry)
            .is_err());
        assert_eq!(fs::read(fixture.path()).unwrap(), before);
        let failed_registry = TestRegistry::default();
        failed_registry.fail_register.set(true);
        let mut restarted = Bindings::default();
        restarted.load(&fixture.path(), &failed_registry).unwrap();
        let snapshot = restarted.snapshot();
        assert_eq!(snapshot.window, "F10");
        assert_eq!(snapshot.window_error.as_deref(), Some("Key unavailable"));
        let recovered_registry = TestRegistry::default();
        Bindings::default()
            .load(&fixture.path(), &recovered_registry)
            .unwrap();
        assert_eq!(recovered_registry.keys.borrow().len(), 2);
    }

    #[test]
    fn failed_clear_and_failed_file_write_keep_previous_binding() {
        let fixture = Fixture::new();
        let registry = TestRegistry::default();
        let mut bindings = Bindings::default();
        bindings
            .initialize(&fixture.path(), legacy(), &registry)
            .unwrap();
        let before = fs::read(fixture.path()).unwrap();
        registry.fail_unregister.set(true);
        assert!(bindings
            .set(&fixture.path(), Target::Window, "", &registry)
            .is_err());
        assert_eq!(fs::read(fixture.path()).unwrap(), before);
        registry.fail_unregister.set(false);
        // A directory cannot be replaced by the settings file on any platform.
        let blocked = fixture.0.join("blocked");
        fs::create_dir(&blocked).unwrap();
        for value in ["F12", ""] {
            assert!(bindings
                .set(&blocked, Target::Window, value, &registry)
                .is_err());
            assert_eq!(bindings.snapshot().window, "F10");
            assert_eq!(registry.keys.borrow().len(), 2);
            assert!(registry.keys.borrow().contains(&"F10".parse().unwrap()));
            assert_eq!(fs::read(fixture.path()).unwrap(), before);
        }
    }

    #[test]
    fn rejects_conflicts_and_invalid_keys_without_changing_disk_or_registration() {
        let fixture = Fixture::new();
        let registry = TestRegistry::default();
        let mut bindings = Bindings::default();
        bindings
            .initialize(&fixture.path(), legacy(), &registry)
            .unwrap();
        let before = fs::read(fixture.path()).unwrap();
        for value in ["F9", "not a key"] {
            assert!(bindings
                .set(&fixture.path(), Target::Window, value, &registry)
                .is_err());
            assert_eq!(fs::read(fixture.path()).unwrap(), before);
            assert_eq!(registry.keys.borrow().len(), 2);
        }
    }

    #[test]
    fn unreadable_settings_are_not_overwritten_by_legacy_migration() {
        let fixture = Fixture::new();
        fs::write(fixture.path(), "invalid json").unwrap();
        let registry = TestRegistry::default();
        let mut bindings = Bindings::default();
        assert!(bindings.load(&fixture.path(), &registry).is_err());
        assert!(bindings
            .initialize(&fixture.path(), legacy(), &registry)
            .is_err());
        assert!(bindings
            .set(&fixture.path(), Target::Window, "F10", &registry)
            .is_err());
        assert_eq!(fs::read_to_string(fixture.path()).unwrap(), "invalid json");
        assert!(registry.keys.borrow().is_empty());
    }

    // Spawn fresh native processes so neither a webview nor in-memory state can
    // make the restart assertion pass. This also runs on the Windows CI runner.
    #[test]
    fn preferences_survive_process_exit_replacement_and_clearing() {
        let fixture = Fixture::new();
        for phase in ["save", "restore", "clear", "verify-clear"] {
            assert!(
                Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "shortcut_bindings::tests::process_worker",
                        "--nocapture"
                    ])
                    .env("QUICKTEXT_SHORTCUT_TEST_PATH", fixture.path())
                    .env("QUICKTEXT_SHORTCUT_TEST_PHASE", phase)
                    .status()
                    .unwrap()
                    .success(),
                "phase {phase} failed"
            );
        }
    }

    #[test]
    fn process_worker() {
        let Some(path) = std::env::var_os("QUICKTEXT_SHORTCUT_TEST_PATH") else {
            return;
        };
        let path = PathBuf::from(path);
        let registry = TestRegistry::default();
        let mut bindings = Bindings::default();
        bindings.load(&path, &registry).unwrap();
        match std::env::var("QUICKTEXT_SHORTCUT_TEST_PHASE")
            .unwrap()
            .as_str()
        {
            "save" => {
                bindings
                    .initialize(&path, StoredBindings::default(), &registry)
                    .unwrap();
                bindings
                    .set(&path, Target::Window, "CmdOrCtrl+Shift+F10", &registry)
                    .unwrap();
                // Replace an existing file, rather than only testing first save.
                bindings
                    .set(&path, Target::Window, "CmdOrCtrl+Alt+F11", &registry)
                    .unwrap();
            }
            "restore" | "clear" => {
                assert_eq!(bindings.snapshot().window, "CmdOrCtrl+Alt+F11");
                assert_eq!(
                    *registry.keys.borrow(),
                    vec!["CmdOrCtrl+Alt+F11".parse::<Shortcut>().unwrap()]
                );
                if std::env::var("QUICKTEXT_SHORTCUT_TEST_PHASE").unwrap() == "clear" {
                    bindings.set(&path, Target::Window, "", &registry).unwrap();
                }
            }
            "verify-clear" => {
                let snapshot = bindings.initialize(&path, legacy(), &registry).unwrap();
                assert_eq!(snapshot.window, "");
                assert!(registry.keys.borrow().is_empty());
            }
            _ => panic!("Unknown test phase"),
        }
    }
}
