# Technical Plan

## Recommended Shape

Use a cross-platform desktop shell with a small frontend, a local audio capture layer, and a provider boundary around Soniox.

The architecture should keep the app responsive even while audio capture, network I/O, and transcription finalization are in progress.

## Proposed Components

- Desktop shell: owns windows, tray/menu bar behavior, global shortcut registration, clipboard integration, and OS permissions.
- UI: record/stop control, transcript display, copy action, settings entry point, and status/error states.
- Audio recorder: captures microphone input and emits audio in the format expected by the transcription provider.
- Transcription service: provider-agnostic interface used by the app.
- Soniox client: Soniox-specific authentication, streaming or upload protocol, response parsing, and error mapping.
- Settings store: saves non-secret user preferences locally.
- Credential store: saves the Soniox API key in OS-backed secret storage.

## State Model

The primary UI can be modeled with these states:

- `idle`: ready to start recording.
- `recording`: microphone is active and audio is being captured.
- `stopping`: stop was requested and the app is finalizing audio/transcription.
- `transcribed`: transcript is available.
- `error`: user-facing failure state with a retry path.

## Main Toggle Behavior

The primary button should be the only required control for the core loop.

- In `idle`, pressing it opens/focuses the UI and starts recording.
- In `recording`, pressing it stops recording.
- In `stopping`, pressing it should be disabled or ignored.
- In `transcribed`, pressing it starts a new recording.
- In `error`, pressing it retries from a clean recording state when appropriate.

## Provider Boundary

Define a narrow transcription interface before implementing Soniox details:

```ts
interface TranscriptionProvider {
  startSession(options: TranscriptionOptions): Promise<TranscriptionSession>;
}

interface TranscriptionSession {
  writeAudio(chunk: AudioChunk): Promise<void>;
  stop(): Promise<TranscriptResult>;
  cancel(): Promise<void>;
}
```

This keeps Soniox isolated and leaves room for later provider changes without rewriting UI and audio capture code.

## Audio Strategy

Two implementation options were considered:

- Streaming: send microphone audio to Soniox while recording and finalize on stop.
- Buffered upload: record locally, then send the captured clip after stop.

Use streaming for MVP because it can reduce perceived wait time and may support partial transcripts later. Buffered upload remains the fallback if cross-platform streaming capture becomes unexpectedly expensive.

## Settings

MVP settings should include:

- Soniox API key.
- Optional auto-copy after transcription.
- Optional global shortcut.

The API key should be stored using the operating system's secure credential storage if the chosen desktop framework supports it cleanly.

## Error Cases

Handle these explicitly:

- Missing Soniox API key.
- Invalid Soniox API key.
- Microphone permission denied.
- No microphone device available.
- Network unavailable.
- Provider timeout or rate limit.
- Empty or unintelligible audio.

## Framework Decision

Use Tauri 2 unless an implementation spike proves microphone capture or packaging is not viable across the target platforms.

Candidate directions considered:

- Tauri: small app footprint, Rust backend, web frontend.
- Electron: mature desktop APIs and ecosystem, larger footprint.
- Native per-platform shell: best integration, highest implementation cost.

Tauri is the initial choice because the app needs a fast desktop shell, global shortcuts, clipboard integration, local settings, and a native backend that can own microphone capture and WebSocket streaming. The framework decision is recorded in [ADR 0001](adrs/0001-tauri-desktop-shell.md).

## Soniox Integration Decision

Use Soniox real-time STT over WebSocket for MVP.

Current Soniox docs identify the real-time STT endpoint as:

```text
wss://stt-rt.soniox.com/transcribe-websocket
```

The first configuration should target `stt-rt-v5`, send microphone audio as binary WebSocket frames, and send an empty WebSocket frame to end the stream gracefully. The provider decision is recorded in [ADR 0002](adrs/0002-streaming-soniox-stt.md).

## First Implementation Milestones

1. Scaffold Tauri 2 app.
2. Build static UI shell with record/stop, transcript, copy, and settings states.
3. Add backend app controller and state machine.
4. Add local settings and OS credential storage.
5. Implement microphone capture on one platform.
6. Add Soniox streaming transcription behind the provider interface.
7. Add clipboard copy and global shortcut trigger.
8. Test on Linux, macOS, and Windows.

See [Implementation plan](implementation-plan.md) for the expanded milestone breakdown.
