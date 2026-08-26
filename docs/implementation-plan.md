# QuickText Implementation Plan

## Plan Summary

Build QuickText as a Tauri 2 desktop app with a compact web UI and a Rust backend that owns microphone capture, Soniox streaming, tray/menu bar residency, global shortcut handling, clipboard writes, and local settings.

The core product promise is a fast toggle loop:

1. Trigger app.
2. Show/focus UI.
3. Start recording immediately.
4. Trigger again.
5. Stop recording.
6. Finalize Soniox stream.
7. Show transcript.
8. Copy transcript on demand.

## Current Decisions

- Desktop shell: Tauri 2.
- Frontend: TypeScript UI, likely React plus Vite unless a lighter local pattern is chosen during scaffolding.
- UI structure: Capture view plus Settings view, without adding a frontend framework for MVP.
- Backend: Rust Tauri commands and events.
- Audio capture: Rust backend, using a cross-platform audio crate such as `cpal`.
- Transcription path: Soniox real-time STT WebSocket streaming.
- Clipboard: Tauri clipboard plugin.
- Background mode: app stays resident in tray/menu bar after launch.
- Global shortcut: Tauri global-shortcut plugin.
- Settings: local app settings plus OS credential storage for the Soniox API key.
- Maximum recording duration: 5 minutes for MVP, with configurability deferred.
- Transcript history: out of MVP.

## Architecture

```text
UI
  |
  | Tauri commands/events
  v
App controller/state machine
  |
  +-- Audio recorder
  |     |
  |     v
  |   PCM audio chunks
  |
  +-- Transcription provider interface
  |     |
  |     v
  |   Soniox WebSocket client
  |
  +-- Settings store
  +-- Credential store
  +-- Clipboard service
  +-- Tray/menu service
  +-- Shortcut service
```

## App State Machine

The app should have one source of truth for recording state.

- `idle`: no active recording, ready to start.
- `starting`: UI has requested recording and backend is opening microphone/provider session.
- `recording`: microphone is active and audio is streaming.
- `stopping`: stop requested, audio closed, provider finalizing.
- `transcribed`: latest transcript is ready.
- `error`: user-actionable failure state.

State transitions:

- `idle` -> `starting` when trigger is pressed.
- `starting` -> `recording` when microphone and Soniox session are ready.
- `starting` -> `error` when setup fails.
- `recording` -> `stopping` when trigger is pressed.
- `recording` -> `stopping` automatically when the 5 minute MVP duration limit is reached.
- `stopping` -> `transcribed` when final transcript is available.
- `stopping` -> `error` when finalization fails.
- `transcribed` -> `starting` when trigger is pressed again.
- `error` -> `starting` only when the error is retryable.

## Milestone 0: Repo And Tooling

Deliverables:

- Scaffold Tauri 2 app.
- Add frontend formatting and linting.
- Add Rust formatting and clippy checks.
- Add a basic CI plan, even if CI is not wired yet.
- Add `.env.example` for non-secret configuration only.

Acceptance checks:

- App launches locally.
- `cargo fmt`, frontend format, and type checks can run.
- No Soniox key is committed.

## Milestone 1: Static UI Shell

Deliverables:

- Compact main window.
- Top bar with app identity, status, and Settings/Capture toggle.
- Capture view for record/stop, recording status, transcript display, and copy action.
- Settings view for Soniox API key, shortcut, behavior, and tray/background information.
- Primary record/stop button.
- Status text for idle, recording, stopping, transcribed, and error states.
- Selectable transcript result panel instead of a textarea.
- Copy button.

Acceptance checks:

- UI can be driven with mocked state.
- Opening the app shows Capture by default.
- Keybind and API key controls are only in Settings.
- Button labels and disabled states match the state machine.
- Transcript text can be copied from mocked data.
- Transcript display reads as output, not as an editable form field.

## Milestone 2: Backend App Controller

Deliverables:

- Tauri command to toggle recording.
- Tauri command to copy latest transcript.
- Event stream from backend to frontend for state changes.
- In-memory transcript/session state.
- Guard against double-start and double-stop races.
- Auto-stop active recordings after the 5 minute MVP duration limit.

Acceptance checks:

- Rapid repeated trigger presses do not create overlapping recording sessions.
- A stale max-duration timer cannot stop a newer recording session.
- UI state remains consistent when commands fail.
- Backend unit tests cover core state transitions.

## Milestone 3: Settings And Secrets

Deliverables:

- Store non-secret preferences locally.
- Store Soniox API key in OS credential storage.
- Detect missing API key before recording starts.
- Allow key update and deletion.

Acceptance checks:

- API key is not written to ordinary config files.
- Missing key produces a clear UI error.
- Restarting the app preserves settings.

## Milestone 4: Audio Capture Spike

Deliverables:

- Capture microphone audio in Rust.
- Normalize to the format sent to Soniox.
- Expose microphone permission/device errors.
- Add a local debug path to confirm non-empty audio chunks without sending them to Soniox.

Acceptance checks:

- Linux capture works on the development machine.
- Audio chunks have expected sample rate, channel count, and sample format.
- Stopping capture releases the microphone.

## Milestone 5: Soniox Streaming Provider

Deliverables:

- Implement provider boundary.
- Open Soniox real-time STT WebSocket session.
- Send config with API key, model, audio format, sample rate, and channel count.
- Stream binary audio frames while recording.
- Send an empty frame to finalize.
- Parse tokens into a transcript.
- Map Soniox/network failures to user-facing errors.

Acceptance checks:

- Short recording produces a final transcript.
- Stop finalizes the active stream instead of uploading after the fact.
- Provider errors do not leak raw protocol messages into the UI.

## Milestone 6: Triggering And Clipboard

Deliverables:

- Add tray/menu bar item with show/hide and quit actions.
- Keep the app running in the tray/menu bar after launch.
- Hide the main window instead of quitting when the window is closed.
- Register default global shortcut.
- Show/focus app and start recording from shortcut.
- Stop recording from the same shortcut.
- Copy latest transcript using Tauri clipboard support.
- Add optional auto-copy setting if it does not complicate the core loop.

Acceptance checks:

- Closing the main window leaves the app running in the tray/menu bar.
- Tray/menu show action focuses the main window.
- Tray/menu quit action exits the process explicitly.
- Shortcut works when another app is focused.
- Shortcut works while the main window is hidden.
- Same trigger starts and stops recording.
- Copy action writes exactly the displayed transcript.

## Milestone 7: Cross-Platform Hardening

Deliverables:

- Test microphone permissions on Linux, macOS, and Windows.
- Test shortcut registration conflicts.
- Test packaging basics.
- Add privacy-safe persistent app logs per [ADR 0015](adrs/0015-persistent-local-diagnostic-logging.md): UTC timestamps, stable event names, run/session correlation, OS-standard paths, bounded rotation, and manual log-folder access.
- Document platform-specific setup issues.

Acceptance checks:

- App starts, records, transcribes, and copies on each target OS.
- Permission failures have useful recovery text.
- Logs stay within the documented size/retention bound and logging failures do not block capture.
- Logs never contain the Soniox API key, transcripts, partials, audio, clipboard text, raw provider frames, device identifiers, or target-window details.
- Users can open the log folder and copy its path from Settings; QuickText never uploads logs automatically.

## Milestone 8: Live Partial Transcripts

Deliverables:

- Expose a partial-update channel from the Soniox session, mirroring `audio_sender`.
- Emit a dedicated Tauri event with `{ final_text, partial_text }` per provider message, without throttling.
- Strip stream-boundary markers from partial text.
- Render finals normally and the hypothesis dimmed in the Capture transcript area.
- Add Settings checkboxes: real-time transcript (default on) with a nested unconfirmed-words option (default on) shown only when real-time is enabled; frontend gates rendering so settings stay out of the provider session.
- Clear the partial on stop, error, and cancel; keep copy/auto-copy tied to the final transcript.

Acceptance checks:

- Confirmed words appear while recording, before stop.
- Dimmed partial updates in place without flickering the final text.
- With real-time display off, nothing appears until finalization completes.
- The unconfirmed-words checkbox only appears when real-time display is enabled.
- Stopping hides the partial immediately; final transcript replaces it.
- Error or cancel leaves no stale partial on screen.
- No Soniox protocol details appear in frontend code.

## Grilling Notes

These are the decisions most likely to break the plan if answered casually.

- Trigger model: Is the "button" a global shortcut, a floating button, a tray/menu item, or all three? Current plan starts with global shortcut plus visible UI button.
- Background model: Does closing the window quit the app? No, the app remains resident in the tray/menu bar until explicit quit.
- Credential model: Is this for personal local use only, or will it be distributed to users who should not handle raw Soniox keys? Current plan assumes personal/local key storage. A commercial app likely needs a backend that issues temporary Soniox API keys.
- Streaming complexity: Are partial transcripts required in MVP? Answered: yes, live partials are implemented per [ADR 0006](adrs/0006-streaming-partial-transcripts.md).
- Window behavior: Should the UI hide after copy, after stop, or never automatically? Current plan keeps it visible after transcription.
- UI structure: Do keybind and API key controls belong on the recording screen? No, they belong in Settings so Capture stays focused.
- Recording bounds: What prevents accidental long recordings? Current plan should add a conservative maximum duration before public release.
- Language defaults: Is the app English-only at first, or should language hints be configurable? Current plan starts with English-oriented defaults and leaves language settings for a follow-up.

## Immediate Next Step

Scaffold the Tauri app and implement Milestone 1 with mocked backend state. Do not integrate Soniox first; the UI state machine and app controller should be stable before real audio and network behavior are added.

## References

- Soniox STT WebSocket API: https://soniox.com/docs/api-reference/stt/websocket-api
- Tauri global shortcut plugin: https://v2.tauri.app/plugin/global-shortcut/
- Tauri plugin overview: https://v2.tauri.app/plugin/
