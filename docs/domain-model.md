# QuickText Domain Model

QuickText is a compact desktop speech-to-text app for fast short dictation.

## Terms

- App trigger: Any user action that toggles the core loop. This can be the primary UI button, a global shortcut, or a tray/menu item.
- IPC command: An external request, such as from the companion CLI, delivered to the resident app over a local socket and dispatched to the same app controller path as UI triggers.
- Companion CLI: The command-line mode of the app binary (`quicktext toggle`, `quicktext status`) that forwards IPC commands to the resident app for compositor bindings and scripts.
- Capture view: The primary UI view for recording, stopping, viewing transcripts, and copying text.
- Settings view: The secondary UI view for keybinds, Soniox API key setup, and app preferences.
- Tray/menu bar resident app: The long-running app process after launch, even when the main window is hidden.
- Recording session: One attempt to capture microphone audio from start until stop/cancel.
- Transcription session: One provider connection or request that turns audio for a recording session into text.
- Transcript: The final user-visible text produced from a transcription session.
- Partial transcript: Non-final text emitted while audio is still being processed.
- Hypothesis tokens: The provider's revisable non-final tokens for un-finalized audio; rendered dimmed and replaced as more audio arrives.
- Provider: A service that converts audio into text.
- Soniox client: The provider implementation that speaks Soniox API/protocol details.
- Audio chunk: A small unit of captured microphone data sent to the transcription session.
- Finalization: The period after stop where audio capture has ended but the provider is still returning final results.
- Copy action: User command that writes the latest transcript to the system clipboard.
- Auto-copy: Optional behavior that copies the transcript immediately when finalization succeeds.
- Credential store: OS-backed secret storage for the Soniox API key.
- Settings store: Local non-secret preferences such as shortcut and auto-copy.

## Core Entities

### AppController

Owns the state machine and coordinates UI commands, recording, transcription, settings, and clipboard actions.

It should keep recording/transcription behavior independent from window visibility.

### AudioRecorder

Owns microphone device selection, permission errors, audio format conversion, and audio chunk emission.

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

### AppSettings

Non-secret user preferences.

Minimum fields:

- `global_shortcut`
- `auto_copy`
- `language_hints`
- `max_recording_seconds`
- `live_transcript` (default on)
- `show_partial_transcript` (default on, only meaningful when `live_transcript` is on)

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
- Capture should render transcript text as output, not as an editable input.
