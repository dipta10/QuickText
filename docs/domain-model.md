# Domain Model

## Terms

- App trigger: Any user action that toggles the core loop. This can be the primary UI button, a global shortcut, or a tray/menu item.
- Recording session: One attempt to capture microphone audio from start until stop/cancel.
- Transcription session: One provider connection or request that turns audio for a recording session into text.
- Transcript: The final user-visible text produced from a transcription session.
- Partial transcript: Non-final text emitted while audio is still being processed.
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

### AudioRecorder

Owns microphone device selection, permission errors, audio format conversion, and audio chunk emission.

### TranscriptionProvider

Provider-neutral interface used by the app controller.

### SonioxTranscriptionProvider

Concrete transcription provider that opens the Soniox WebSocket, sends config/audio/end frames, and parses token responses.

### TranscriptResult

Final output from a transcription session.

Minimum fields:

- `text`
- `duration_ms`
- `provider`
- `created_at`

### AppSettings

Non-secret user preferences.

Minimum fields:

- `global_shortcut`
- `auto_copy`
- `language_hints`
- `max_recording_seconds`

## Boundaries

- UI must not know Soniox protocol details.
- UI must not read the Soniox API key directly.
- Audio capture must not write provider-specific JSON.
- Soniox client must not own window behavior.
- Clipboard writes must use the transcript currently displayed by app state.

