# ADR 0003: Integrate Soniox Behind Backend App Controller

## Status

Accepted for the Soniox implementation slice.

## Context

QuickText's current UI is compact and includes a record button, shortcut configuration, and Soniox API key controls. The frontend still toggles a local `idle`/`recording` state without creating a backend recording session.

Soniox integration adds microphone capture, credential access, WebSocket streaming, provider finalization, and network error handling. If these concerns are wired directly into the UI state, the app will duplicate trigger logic between the button, global shortcut, and tray/menu surfaces.

The existing docs require the Rust backend to own microphone capture, provider streaming, tray/menu bar behavior, shortcut handling, clipboard integration, secret access, and the recording/transcription source of truth.

## Decision

Implement Soniox through a backend-owned app controller before connecting real audio or WebSocket behavior to the UI.

The implementation order is:

1. Add backend app state, app events, and a `toggle_recording` Tauri command.
2. Move the UI button and global shortcut to the same backend toggle command.
3. Add a fake transcription provider so controller behavior can be tested without network access.
4. Add tray/menu bar residency so closing the window hides it instead of quitting.
5. Add secure Soniox credential reads to the start path.
6. Add microphone capture and normalized audio chunks.
7. Add the Soniox real-time WebSocket provider behind the provider interface.
8. Add final transcript and error events for the UI.

The frontend must render backend state events once real recording starts. It may keep local-only state for shortcut capture and form input.

## Consequences

- Button and shortcut behavior share one source of truth.
- Window visibility and app lifetime are separated.
- Race handling for double-start and double-stop lives in one place.
- Soniox protocol details stay out of UI modules.
- Tests can cover controller state transitions with fake audio and fake providers.
- The first backend slice will not produce real transcripts until audio capture and Soniox provider work land.

## Grilled Decisions

- **Where does recording state live?** Backend app controller.
- **Does the UI know Soniox config fields?** No.
- **Does the global shortcut duplicate record/stop logic?** No, it calls the same toggle path.
- **Does closing the window quit the app?** No, it hides the window and keeps the tray/menu bar process running.
- **Do we implement Soniox before fake provider tests?** No.
- **Do we store API keys in local settings?** No, keyring-backed credential storage only.
- **Do we show partial transcripts in MVP?** Not required. The provider boundary should allow it later.
- **What protects against accidental long recordings?** MVP auto-stops an active recording after 5 minutes. A later settings slice can make this configurable.

## Open Questions

- Which raw PCM format should the recorder normalize to for the first Soniox implementation?
- Should language hints default to English-only, empty auto-detection, or a saved setting?
- Should provider finalization have a fixed timeout before returning a user-facing timeout error?
