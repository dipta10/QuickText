use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::{AutoLaunchManager, ManagerExt};

pub(crate) const AUTOSTART_ARGUMENT: &str = "--autostart";

trait LaunchOnStartupManager {
    fn enable(&self) -> Result<(), String>;
    fn disable(&self) -> Result<(), String>;
    fn is_enabled(&self) -> Result<bool, String>;
}

impl LaunchOnStartupManager for AutoLaunchManager {
    fn enable(&self) -> Result<(), String> {
        AutoLaunchManager::enable(self).map_err(|error| error.to_string())
    }

    fn disable(&self) -> Result<(), String> {
        AutoLaunchManager::disable(self).map_err(|error| error.to_string())
    }

    fn is_enabled(&self) -> Result<bool, String> {
        AutoLaunchManager::is_enabled(self).map_err(|error| error.to_string())
    }
}

pub(crate) fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_autostart::Builder::new()
        .arg(AUTOSTART_ARGUMENT)
        .build()
}

pub(crate) fn is_autostart_launch(args: &[String]) -> bool {
    matches!(args, [argument] if argument == AUTOSTART_ARGUMENT)
}

pub(crate) fn is_enabled<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("Could not read the launch-on-startup setting: {error}"))
}

pub(crate) fn set_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<bool, String> {
    update_manager(app.autolaunch().inner(), enabled)
        .map_err(|error| format!("Could not update the launch-on-startup setting: {error}"))
}

fn update_manager(manager: &impl LaunchOnStartupManager, enabled: bool) -> Result<bool, String> {
    if enabled {
        manager.enable()?;
    } else {
        manager.disable()?;
    }

    let actual_state = manager.is_enabled()?;
    if actual_state != enabled {
        return Err("the operating system did not apply the requested state".to_string());
    }

    Ok(actual_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeManager {
        enabled: Mutex<bool>,
        fail_updates: bool,
    }

    impl LaunchOnStartupManager for FakeManager {
        fn enable(&self) -> Result<(), String> {
            if self.fail_updates {
                return Err("registration denied".to_string());
            }
            *self.enabled.lock().unwrap() = true;
            Ok(())
        }

        fn disable(&self) -> Result<(), String> {
            if self.fail_updates {
                return Err("registration denied".to_string());
            }
            *self.enabled.lock().unwrap() = false;
            Ok(())
        }

        fn is_enabled(&self) -> Result<bool, String> {
            Ok(*self.enabled.lock().unwrap())
        }
    }

    #[test]
    fn recognizes_only_the_plugin_autostart_argument_as_a_hidden_launch() {
        assert!(is_autostart_launch(&[AUTOSTART_ARGUMENT.to_string()]));
        assert!(!is_autostart_launch(&[]));
        assert!(!is_autostart_launch(&["toggle".to_string()]));
        assert!(!is_autostart_launch(&[
            AUTOSTART_ARGUMENT.to_string(),
            "toggle".to_string(),
        ]));
    }

    #[test]
    fn enables_and_disables_the_operating_system_registration() {
        let manager = FakeManager::default();

        assert_eq!(update_manager(&manager, true), Ok(true));
        assert_eq!(update_manager(&manager, false), Ok(false));
    }

    #[test]
    fn returns_registration_failures_without_changing_state() {
        let manager = FakeManager {
            enabled: Mutex::new(false),
            fail_updates: true,
        };

        assert_eq!(
            update_manager(&manager, true),
            Err("registration denied".to_string())
        );
        assert!(!manager.is_enabled().unwrap());
    }
}
