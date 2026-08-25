# ADR 0010: Optional OS Notifications For The Capture Lifecycle

## Status

Accepted (documentation only; implementation is not scheduled by this ADR).

## Context

QuickText runs resident in the tray/menu bar, and recording is usually triggered while the main window is hidden (global shortcut or companion CLI). When a recording starts, finishes, or fails, the only feedback today lives inside the main window, so a user working in another app has no idea whether dictation started, completed, or silently failed.

OS-level desktop notifications can reach the user regardless of window state. However, notifications fired on every lifecycle event would quickly become noise, so exposure must be opt-in per event type.

This decision covers notification behavior only. It does not change the capture pipeline, state machine, or provider boundary described in [ADR 0003](0003-backend-first-soniox-integration.md), [ADR 0007](0007-immediate-capture-with-provider-buffering.md).

## Decision

Add three independent, opt-in OS notifications, each controlled by its own checkbox in a Notifications section of the Settings view:

1. **Transcription started**: fires when a recording session begins (the trigger is accepted and capture goes live).
2. **Transcription complete**: fires when the final transcript is ready.
3. **Error occurred**: fires when any backend failure surfaces during capture, transcription, or provider connection.

Rules:

- All three checkboxes default to **unchecked**; no notifications fire unless the user opts in.
- Delivery uses OS desktop notifications (`tauri-plugin-notification`), not in-app-only banners, because the window is typically hidden when these events happen.
- If the main window is visible and focused, notifications are suppressed; the inline UI already provides the feedback.
- Clicking a notification shows/focuses the main window. It never toggles recording.
- Notification content is fixed labels ("Transcription started", "Transcription complete", "Recording failed"). Transcripts and error details are never placed inside notifications, keeping dictated content out of the OS notification center.
- Toggles are persisted as ordinary non-secret local settings and take effect immediately for the next event, without restart.
- Notification decisions live in the backend: backend state transitions consult the saved preferences and fire notifications. The frontend renders the checkboxes but never owns firing logic.
- If the OS denies notification permission, the Settings view shows an inline warning; the rest of the app works normally and notifications are skipped.

## Consequences

- Users who trigger recordings from other apps finally get feedback without focusing QuickText.
- Three new settings must round-trip through local settings persistence and reach the backend controller.
- The backend needs access to the notification plugin and a permission/availability check for the Settings warning.
- Suppression logic requires knowing foreground/visibility state at fire time, which the backend already tracks via window show/hide paths.
- Error notifications cover all capture/transcription/connection failures under one toggle; users cannot opt into only some error types for MVP.
- Fixed labels mean localization strings will be needed if the app is ever localized.
- Tests must cover: default-off behavior, immediate toggle effect, suppression when the window is visible, and click-to-show behavior.

## Grilled Decisions

- **In-app toasts or OS notifications?** OS notifications. The window is hidden during typical shortcut-driven use, so in-app banners would miss the moment.
- **Which errors notify?** Any backend failure during capture, transcription, or provider connection, under one shared error toggle.
- **What does clicking a notification do?** Shows/focuses the main window only; it never starts or stops recording.
- **Suppress when the window is visible?** Yes. The inline UI already shows the same information; duplicating it as an OS notification is noise.
- **Where do the checkboxes live?** A dedicated Notifications section in Settings, keeping Capture free of configuration.
- **Does the "started" notification mean mic-live or provider-connected?** Mic-live (recording session start). Waiting for the provider handshake would delay or drop the signal (see [ADR 0007](0007-immediate-capture-with-provider-buffering.md)).
- **Do notifications carry transcript or error text?** No. Fixed labels only, so dictated content never lands in the OS notification history.
- **Behavior when OS permission is denied?** Inline warning in Settings; notifications are silently skipped elsewhere.
- **Are toggles persisted across restarts?** Yes, as non-secret local settings; they apply immediately.

## References

- [ADR 0003](0003-backend-first-soniox-integration.md): backend-owned app controller where notification decisions belong.
- [ADR 0004](0004-capture-settings-ui.md): Capture/Settings split hosting the Notifications section.
- [ADR 0007](0007-immediate-capture-with-provider-buffering.md): recording-start semantics referenced by the started notification.
