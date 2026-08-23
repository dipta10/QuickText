# ADR 0007: Recording Notification Sounds

## Status

Accepted.

## Context

Recordings usually start through the global shortcut while the QuickText window is hidden, and the only feedback today is the tray icon changing ([commit dfaf573](https://github.com/anomalyco/opencode)). When the window is not visible, the user cannot tell whether a toggle press actually started or finished a recording without reopening Capture. Audible feedback closes that gap.

The app already has a pattern for non-secret user preferences persisted in `localStorage` and gated in the frontend (auto-copy, live transcript display). Recording state arrives at the frontend exclusively through the `app-state-changed` event carrying the full snapshot.

## Decision

Add opt-in notification sounds for two lifecycle moments, implemented entirely in the frontend:

- **Recording started**: played when the app enters `recording`.
- **Transcription completed**: played when the app enters `transcribed`. Errors stay silent.

Specifics:

- Sounds are **synthesized tones** generated at runtime with the Web Audio API (`OscillatorNode` envelopes). No audio asset files are bundled, avoiding licensing and packaging work.
- One **shared clip palette** (small named tone recipes); the user picks a clip independently for each event from a dropdown.
- Each event has its own enable checkbox (default off) plus a nested clip selector shown only when enabled, mirroring the live-transcript/unconfirmed-words pairing from [ADR 0006](0006-streaming-partial-transcripts.md).
- Playback happens in the webview via `HTMLAudioElement`-free Web Audio synthesis and must work while the window is hidden; this is the feature's primary use case and is called out as a platform risk to verify per OS during hardening ([Milestone 7](../implementation-plan.md)).
- Sound triggering is an **edge detection** on recording-state transitions inside the frontend state-update path, driven by `app-state-changed` snapshots. No backend changes, no new Tauri commands or events. Because the backend owns state truth, auto-stop and shortcut-triggered recordings produce sounds identically.
- Preferences persist in `localStorage` alongside other non-secret settings: enabled flags default to off; selected clips default to the palette's subtle options.
- Playback failures (unsupported context, suspended audio) degrade silently to no sound; they never surface as UI errors.

## Consequences

- The backend stays untouched; the provider boundary and IPC surface do not change.
- If the webview suspends audio while hidden on some platform, sounds silently fail there; verification belongs in cross-platform hardening rather than speculative fallbacks now.
- Transition detection lives in one frontend function so every trigger path (button, shortcut, companion CLI) behaves the same.
- Tone recipes are pure data, keeping the audible behavior unit-testable without audio hardware.

## References

- [ADR 0004](0004-capture-settings-ui.md): capture/settings split these controls belong in Settings.
- [ADR 0006](0006-streaming-partial-transcripts.md): the checkbox-plus-nested-option settings pattern reused here.
