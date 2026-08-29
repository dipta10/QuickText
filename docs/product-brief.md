# QuickText Product Brief

## Summary

Build QuickText, a desktop speech-to-text app that can be opened quickly, record speech, transcribe it through Soniox, and make the resulting text easy to copy.

QuickText is meant for fast capture rather than long-form audio editing. The core loop should feel instant and predictable.

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

The app should keep running in the background after launch. Closing the main window should hide it to the tray/menu bar, not quit the process. Quitting should be an explicit tray/menu action.

## MVP Requirements

- Desktop app with a small, fast UI.
- Capture-first UI with a separate Settings view.
- One primary control that toggles between start and stop.
- Visible recording state.
- Visible transcription/progress state.
- Transcript display area.
- Copy-to-clipboard button.
- Soniox API key configuration.
- Global shortcut for start/stop.
- Tray/menu bar background mode after launch.
- Basic error handling for missing API key, microphone permission failure, network failure, and provider failure.
- Local privacy-safe support diagnostics with bounded retention, correlated run/session identifiers, and explicit user export.

## Non-Goals for MVP

- Multi-provider support beyond clean internal boundaries.
- Transcript history.
- Speaker diarization UI.
- Long-form audio file import.
- Rich text formatting.
- Team or cloud account features.
- Mobile support.
- Automatic telemetry, remote log upload, and crash-reporting infrastructure.

## Nice-to-Have Later

- Auto-copy transcript after completion.
- Paste transcript into the previously focused app.
- Local transcript history.
- Custom vocabulary or context hints if supported by the provider.
- Streaming partial transcript display while speaking.
- Offline fallback provider.

## UX Principles

- The first action should start listening, not show a setup-heavy screen.
- Normal dictation should happen in Capture; configuration belongs in Settings.
- The primary button should always communicate the next action.
- Recording and processing states should be unmistakable.
- The app should stay compact, but the transcript must remain readable.
- The transcript should look like selectable result text, not an input box.
- Errors should tell the user what to fix without exposing low-level provider details.

## Resolved Product Questions

- First trigger surface: use a global shortcut plus visible UI button for MVP.
- Background behavior: app stays resident in tray/menu bar after launch; closing the window hides it.
- Recording strategy: stream live to Soniox instead of upload-after-stop.
- Auto-copy default: keep manual copy as the MVP default; auto-copy can be added as an option.
- UI after transcription: keep the UI open so the user can inspect and copy the transcript.
- UI information architecture: use a Capture view for recording/transcripts and a Settings view for keybind/API/preferences.

## Open Product Questions

- Should there be a maximum recording duration in MVP?
- What default global shortcut should be least likely to conflict across Linux, macOS, and Windows?
- Should language hints be fixed initially or exposed in settings?
