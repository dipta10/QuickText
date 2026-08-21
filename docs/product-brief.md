# Product Brief

## Summary

Build a desktop speech-to-text app that can be opened quickly, record speech, transcribe it through Soniox, and make the resulting text easy to copy.

The app is meant for fast capture rather than long-form audio editing. The core loop should feel instant and predictable.

## Target Platforms

- Linux
- macOS
- Windows

## Primary User Flow

1. User presses the app button or configured shortcut.
2. The UI appears if it is hidden.
3. Recording starts immediately.
4. User speaks.
5. User presses the same button or shortcut again.
6. Recording stops.
7. The app sends audio to Soniox or finalizes an active Soniox stream.
8. The transcript appears in the UI.
9. User can copy the transcript to the clipboard.

## MVP Requirements

- Desktop app with a small, fast UI.
- One primary control that toggles between start and stop.
- Visible recording state.
- Visible transcription/progress state.
- Transcript display area.
- Copy-to-clipboard button.
- Soniox API key configuration.
- Global shortcut for start/stop.
- Basic error handling for missing API key, microphone permission failure, network failure, and provider failure.

## Non-Goals for MVP

- Multi-provider support beyond clean internal boundaries.
- Transcript history.
- Speaker diarization UI.
- Long-form audio file import.
- Rich text formatting.
- Team or cloud account features.
- Mobile support.

## Nice-to-Have Later

- Auto-copy transcript after completion.
- Paste transcript into the previously focused app.
- Local transcript history.
- Custom vocabulary or context hints if supported by the provider.
- Streaming partial transcript display while speaking.
- Tray/menu bar mode.
- Offline fallback provider.

## UX Principles

- The first action should start listening, not show a setup-heavy screen.
- The primary button should always communicate the next action.
- Recording and processing states should be unmistakable.
- The app should stay compact, but the transcript must remain readable.
- Errors should tell the user what to fix without exposing low-level provider details.

## Resolved Product Questions

- First trigger surface: use a global shortcut plus visible UI button for MVP.
- Recording strategy: stream live to Soniox instead of upload-after-stop.
- Auto-copy default: keep manual copy as the MVP default; auto-copy can be added as an option.
- UI after transcription: keep the UI open so the user can inspect and copy the transcript.

## Open Product Questions

- Should there be a maximum recording duration in MVP?
- What default global shortcut should be least likely to conflict across Linux, macOS, and Windows?
- Should language hints be fixed initially or exposed in settings?
