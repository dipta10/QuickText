# QuickText Domain Model

QuickText is a compact desktop speech-to-text app for fast short dictation.

## Terms

- App trigger: Any user action that toggles the core loop. This can be the primary UI button, a global shortcut, or a tray/menu item.
- IPC command: An external request, such as from the companion CLI, delivered to the resident app over a local socket and dispatched to the same app controller path as UI triggers.
- Companion CLI: The command-line mode of the app binary (`quicktext toggle`, `quicktext status`) that forwards IPC commands to the resident app for compositor bindings and scripts.
- Capture view: The primary UI view for recording, stopping, viewing transcripts, and copying text.
- Settings view: The secondary UI view for keybinds, Soniox API key setup, and app preferences.
- Tray/menu bar resident app: The long-running app process after launch, even when the main window is hidden.
- Recording session: One accepted dictation attempt from the backend start transition through setup, capture, finalization, cancellation, or failure.
- Transcription session: The logical operation that turns one recording session's audio into text; it may contain more than one provider connection attempt if retry or reconnect behavior is used.
- Transcript: The final user-visible text produced from a transcription session.
- Partial transcript: Non-final text emitted while audio is still being processed.
- Hypothesis tokens: The provider's revisable non-final tokens for un-finalized audio; rendered dimmed and replaced as more audio arrives.
- Provider: A service that converts audio into text.
- Soniox client: The provider implementation that speaks Soniox API/protocol details.
- Audio chunk: A small unit of captured microphone data sent to the transcription session.
- Connect buffer: Audio chunks captured before the provider connection completes; held in the transcription session's channel and flushed in order once connected.
- Finalization: The period after stop where audio capture has ended but the provider is still returning final results.
- Copy action: User command that writes the latest transcript to the system clipboard.
- Auto-copy: Optional behavior that copies the transcript immediately when finalization succeeds.
- Credential store: OS-backed secret storage for the Soniox API key.
- Settings store: Local non-secret preferences such as shortcut and auto-copy.
- Input device selection: The user-chosen microphone, persisted as a stable device ID; "System default" tracks the OS default microphone.
- Device fallback: Behavior when the configured input device is missing at record start; capture uses the OS default instead.
- App run: One lifetime of the resident QuickText process, identified by a `run_id` generated at process start.
- Provider session: One provider connection attempt within a transcription session, identified independently so retries remain distinguishable.
- Support reference: A compact user-visible reference for one error event; exported diagnostics contain its full error and session context.
- Diagnostics bundle: A user-exported archive of bounded, metadata-only local logs and a technical manifest.

## Core Entities

### AppController

Owns the state machine and coordinates UI commands, recording, transcription, settings, and clipboard actions.

It should keep recording/transcription behavior independent from window visibility.

### AudioRecorder

Owns microphone device selection, permission errors, audio format conversion, and audio chunk emission.

It captures from the configured input device, or the OS default when none is configured. If the configured device is missing at record start, it falls back to the OS default for that session and reports the fallback so the UI can show a notice. Device changes apply to the next recording session; active recordings keep their starting device.

Minimum responsibilities:

- List available input devices (stable ID plus display label) on demand.
- Resolve the current OS default input device name.
- Start capture from the selected device with permission handling.

### TranscriptionProvider

Provider-neutral interface used by the app controller.

Responsibilities:

- Start a provider session from app-level transcription options.
- Accept normalized audio chunks from the audio recorder.
- Emit provider-agnostic partial updates (`final_text`, `partial_text`) while the session is live.
- Finalize or cancel the active session.
- Return app-level transcript results and app-level errors.

It must not expose provider protocol frames, raw provider response objects, or API key handling to the UI.

### SonioxTranscriptionProvider

Concrete transcription provider that opens the Soniox WebSocket, sends config/audio/end frames, and parses token responses.

Responsibilities:

- Read the Soniox credential through the backend credential service.
- Connect to the Soniox real-time WebSocket endpoint.
- Send the initial Soniox session configuration.
- Stream binary audio frames.
- Send the provider-specific finalization signal.
- Convert token responses into transcript text.
- Map Soniox and network failures into app-level errors.

### TranscriptResult

Final output from a transcription session.

Minimum fields:

- `text`
- `duration_ms`
- `provider`
- `created_at`

### AudioFormat

The normalized audio shape passed from `AudioRecorder` to `TranscriptionProvider`.

Minimum fields:

- `sample_rate`
- `channels`
- `encoding`

For MVP, prefer one explicit raw PCM format so Soniox configuration is deterministic.

### AppError

User-actionable error produced by the backend app controller.

Minimum categories:

- `missing_api_key`
- `invalid_api_key`
- `microphone_permission_denied`
- `no_microphone_device`
- `network_unavailable`
- `provider_unavailable`
- `provider_timeout`
- `empty_audio`
- `internal_error`

UI text should be generated from these categories and should not include raw provider protocol details.

Every surfaced `AppError` also carries one full `error_id` and its derived `support_reference`. Re-rendering or notifying the same error reuses those values; a new failure occurrence receives a new `error_id`.

### AppSettings

Non-secret user preferences.

Minimum fields:

- `global_shortcut`
- `auto_copy`
- `input_device_id` (stable device ID; absent means system default)
- `language_hints`
- `max_recording_seconds`
- `live_transcript` (default on)
- `show_partial_transcript` (default on, only meaningful when `live_transcript` is on)
- `launch_on_startup` (default off; registered with the operating system by the backend)

### TrayMenuService

Owns the tray/menu bar item, show/hide actions, explicit quit action, and window-close-to-hide behavior.

### IpcListener

Owns the local socket/named pipe inside the resident app process.

Responsibilities:

- Bind the platform-local endpoint with owner-only permissions.
- Enforce single-instance by failing to bind when another instance is live.
- Parse versioned line-delimited JSON requests.
- Dispatch `toggle` through the same app-controller trigger path as UI triggers.
- Answer `status` with the current app snapshot without changing state.

It must not own recording logic. When a request asks for window management, it may show, focus, or hide the main window through the shared window helpers and nothing more.

### CompanionCli

The argv-based command-line mode of the app binary.

Responsibilities:

- Branch to GUI launch when no subcommand is given.
- Forward `toggle` and `status` requests to the resident app over IPC.
- Print human-readable status by default and full JSON with `--json`.
- Exit 0 on success, 1 on generic failure, 2 when no resident app is running.

### UiViewState

Frontend-only view state that decides whether Capture or Settings is visible.

It must not own recording state once backend recording is connected. Recording state comes from backend app-state events.

### Logger

Backend-owned application logging boundary. Rust application code depends on this familiar
abstraction and records only typed, allowlisted events through its `debug`, `info`, `warn`, and
`error` methods. It never accepts arbitrary messages or metadata maps.

### SupportLogManager

Backend-owned management surface around the logger's privacy-safe local files.

Responsibilities:

- Generate one UUID v4 `run_id` per resident process launch.
- Assign and propagate recording-session, provider-session, and error identifiers.
- Persist versioned JSON Lines events with UTC timestamps and allowlisted metadata received from the logger.
- Rotate logs by size and enforce bounded retention.
- Serialize writes, stable export snapshots, and close/delete/reopen operations through one writer task.
- Enable non-persistent, time-bounded debug logging without weakening content exclusions.
- Export confirmed diagnostics bundles and delete retained local diagnostics on request.

It must not persist audio, transcripts, partial transcripts, credentials, clipboard contents, raw provider payloads, identifying paths, network identifiers, or microphone names/IDs. It must not upload diagnostics.

### DiagnosticEvent

One structured local diagnostic record.

Minimum fields:

- `schema_version`
- `timestamp_utc`
- `level`
- `component`
- `event`
- `run_id`
- optional `recording_session_id`
- optional `provider_session_id`
- optional `error_id`
- allowlisted `metadata`

### DiagnosticsBundle

A ZIP archive created only after explicit user confirmation. It contains the current and rotated logs plus a manifest with safe technical metadata, including separate immutable `build_id` and `source_revision` fields. Export creates a local file chosen by the user and never transmits it.

## Boundaries

- UI must not know Soniox protocol details.
- UI must not read the Soniox API key directly.
- Audio capture must not write provider-specific JSON.
- Soniox client must not own window behavior.
- Clipboard writes must use the transcript currently displayed by app state.
- Partial transcripts are ephemeral: they must be cleared on stop, error, and cancel, never persisted.
- Global shortcuts, UI button presses, and IPC commands must call the same app-controller trigger path.
- IPC toggles must not show or focus windows unless the request explicitly asks for window management (`focus`).
- Frontend recording state must be derived from backend app-state events once real recording starts.
- Window visibility must not be treated as app lifetime; explicit quit is required to stop the resident process.
- Settings controls must stay out of the Capture view.
- Input device picking belongs to Settings; Capture may only surface device-fallback notices.
- Capture should render transcript text as output, not as an editable input.
- Log writes, retention, export, and deletion are backend responsibilities; the frontend never receives raw retained-log paths.
- Logs must use allowlisted structured events rather than arbitrary console forwarding.
- Recording sessions keep one ID across their lifecycle; each provider connection attempt receives its own ID.
- Normal and debug logs must preserve the same secret and user-content exclusions.
