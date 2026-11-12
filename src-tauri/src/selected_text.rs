//! Selection capture never falls back to unrelated clipboard text.
use tauri::AppHandle;

#[cfg(target_os = "linux")]
pub async fn read(_app: &AppHandle) -> Result<String, String> {
    use std::process::Stdio;
    use tokio::{
        io::AsyncReadExt,
        process::Command,
        time::{timeout, Duration},
    };
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let (program, args): (&str, &[&str]) = if wayland {
        ("wl-paste", &["--primary", "--no-newline"])
    } else {
        ("xclip", &["-o", "-selection", "primary"])
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            format!(
                "Install {} to read selected text, or paste text into QuickText.",
                if wayland { "wl-clipboard" } else { "xclip" }
            )
        })?;
    let mut output = child
        .stdout
        .take()
        .ok_or_else(|| "Could not read selected text.".to_string())?
        .take(20_001);
    let mut bytes = Vec::new();
    timeout(Duration::from_secs(2), output.read_to_end(&mut bytes))
        .await
        .map_err(|_| {
            "Reading the selection timed out. Paste the text into QuickText instead.".to_string()
        })?
        .map_err(|_| {
            "Could not read the selection. Paste the text into QuickText instead.".to_string()
        })?;
    if bytes.len() > 20_000 {
        return Err("Select a shorter passage (up to 5,000 characters).".into());
    }
    let status = timeout(Duration::from_secs(1), child.wait())
        .await
        .map_err(|_| "Reading the selection timed out.".to_string())?
        .map_err(|_| "Could not read selected text.".to_string())?;
    if !status.success() {
        return Err("Select some text first, or paste it into QuickText.".into());
    }
    String::from_utf8(bytes).map_err(|_| "The selection is not readable text.".into())
}

#[cfg(not(target_os = "linux"))]
pub async fn read(app: &AppHandle) -> Result<String, String> {
    // A cancelled reading must not interrupt clipboard restoration halfway through
    // Copy. JoinHandle drop detaches this short, serialized transaction; its
    // result is discarded if its caller has already stopped the reading.
    static CAPTURE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _capture = CAPTURE.lock().await;
        read_with_clipboard(&app).await
    })
    .await
    .map_err(|_| "Could not capture selected text.".to_string())?
}

#[cfg(not(target_os = "linux"))]
async fn read_with_clipboard(app: &AppHandle) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    use tokio::time::{sleep, Duration};
    // Require a text clipboard so this operation cannot destroy an image clipboard.
    let previous = app
        .clipboard()
        .read_text()
        .map_err(|_| "Copy the selection and paste it into QuickText to read it.".to_string())?;
    let marker = format!("quicktext-selection-{:?}", std::time::SystemTime::now());
    struct Restore {
        app: AppHandle,
        previous: String,
        expected: String,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            if self.app.clipboard().read_text().ok().as_deref() == Some(&self.expected) {
                let _ = self.app.clipboard().write_text(&self.previous);
            }
        }
    }
    app.clipboard()
        .write_text(&marker)
        .map_err(|_| "Could not capture selected text.".to_string())?;
    let mut restore = Restore {
        app: app.clone(),
        previous,
        expected: marker.clone(),
    };
    // Allow the global shortcut modifiers to be released before sending Copy.
    sleep(Duration::from_millis(250)).await;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(crate::paste_target::send_copy_keystroke());
    })
    .map_err(|_| "Could not capture the selection.".to_string())?;
    rx.await
        .map_err(|_| "Could not capture the selection.".to_string())??;
    for _ in 0..20 {
        sleep(Duration::from_millis(30)).await;
        if let Ok(text) = app.clipboard().read_text() {
            if text != marker {
                restore.expected = text.clone();
                return Ok(text);
            }
        }
    }
    Err("Select text in another app first. On macOS, allow Accessibility access for QuickText, or paste the text here.".into())
}
