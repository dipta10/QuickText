# QuickText UI Plan

## Design Goal

QuickText should feel like a compact desktop utility, not a settings form with a record button attached.

The main UI should separate the active dictation workflow from configuration. Recording and transcript review belong in the primary Capture view. API keys, shortcut capture, and preferences belong in Settings.

## Product Shape

QuickText has two main views:

- Capture: the default operational view for recording, stopping, reviewing, and copying the transcript.
- Settings: configuration for Soniox API key, global shortcut, tray behavior, language hints, and future preferences.

The app should always open to Capture when triggered for dictation. Settings should never block the core record/stop loop unless a required setup item is missing.

## Primary Window

Recommended initial window size:

- Width: 420-520px.
- Height: 520-680px.
- Resizable: yes, with sensible minimums.
- Default theme: dark.
- Density: compact, but not cramped.

The window should have three stable regions:

- Top bar: app name, current backend status, Settings button.
- Main content: either Capture or Settings.
- Bottom status line: shortcut hint, tray/background status, or current error.

Avoid decorative hero sections, marketing copy, oversized cards, and nested cards.

## Capture View

Capture is the default view and the only view needed for normal dictation.

Required elements:

- Large primary record/stop toggle.
- Short status label: Ready, Listening, Finalizing, Transcript ready, or Error.
- Recording affordance: timer plus subtle activity indicator while listening.
- Transcript display as a readable result surface, not a textarea.
- Copy button in the transcript toolbar.
- Optional "New recording" action after transcript completion if the main toggle behavior is not enough.

Transcript presentation:

- Use a plain text result panel styled like a document excerpt.
- Preserve line breaks.
- Allow text selection.
- Do not use a textarea for the final transcript.
- Show empty state text only before the first transcript.
- Put Copy near the transcript title or bottom action row.

Capture state behavior:

- `idle`: show primary action as Record.
- `starting`: disable toggle and show Starting.
- `recording`: show primary action as Stop, timer, and listening indicator.
- `stopping`: disable toggle and show Finalizing.
- `transcribed`: show transcript, Copy enabled, primary action can start a new recording.
- `error`: show concise recovery text and keep retry action clear.

## Settings View

Settings should be a separate view reached from a gear button or Settings tab in the top bar.

Recommended sections:

- Soniox: API key status, save/update key, delete key.
- Shortcut: current global shortcut, capture-new-shortcut control, conflict/error status, and shortcut behavior checkboxes.
- Behavior: manual copy default, future auto-copy option, max recording duration display.
- App: tray/background explanation and explicit quit note.

Settings rules:

- Do not show raw provider protocol details.
- Do not show stored API key values.
- Do not store API keys in frontend persistence.
- Keep settings controls visually quieter than the Capture record button.
- Use short labels and direct status text.

## Trigger Behavior

The app trigger can come from the global shortcut, tray/menu item, or primary UI button.

Shortcut behavior options (persisted locally, applied by the backend shortcut handler):

- Focus window when recording starts: bring the window to the front when the shortcut starts a recording. Default: on.
- Hide window when recording stops: hide the window to tray after stopping via the shortcut. Default: off.
- Both options only apply to global-shortcut triggers; in-window buttons and IPC/CLI toggles keep their own behavior.

Default trigger behavior:

- If the app is hidden and the trigger is pressed, show/focus the window, switch to Capture, and start recording.
- If Settings is open and the trigger is pressed, switch to Capture and start recording.
- If recording is active and the trigger is pressed, stop recording without changing the transcript view until finalization completes.
- If finalization is active, ignore or disable repeated triggers.
- If the required Soniox API key is missing, show/focus Settings with the Soniox section highlighted and do not start recording.

## Visual Direction

Use a modern restrained desktop style:

- Dark neutral background.
- One clear accent color for recording and focus states.
- High-contrast transcript text.
- Subtle borders, 8px radius maximum, and stable spacing.
- Icon buttons where they are conventional: settings, copy, close/hide, retry.
- Text buttons only for primary record/stop and clear settings actions.

Avoid:

- Putting every control on the first screen.
- Making the transcript look like an editable form field.
- Large gradients or decorative backgrounds.
- A one-color blue/purple dashboard look.
- Layout shifts when status text, shortcut labels, or transcript content changes.

## Information Architecture

```text
QuickText window
  Top bar
    QuickText
    Status chip
    Settings/Capture icon button

  Capture view
    Record/Stop toggle
    Timer and recording state
    Transcript result panel
    Copy action

  Settings view
    Soniox API key
    Global shortcut
    Behavior
    App/tray

  Bottom line
    Shortcut hint or active error
```

## Implementation Plan

1. Extend frontend state with `activeView: "capture" | "settings"`.
2. Refactor `src/app-view.ts` into static view creation for top bar, Capture view, Settings view, and bottom status line.
3. Keep `src/main.ts` as the wiring layer only.
4. Replace the transcript textarea with a selectable transcript result panel.
5. Move keybind and API key controls into Settings.
6. Add a Settings/Capture toggle control in the top bar.
7. Update trigger handling so global shortcut and backend toggle force Capture view before recording.
8. Add empty, recording, finalizing, transcript-ready, and error visual states.
9. Add CSS tokens for spacing, colors, typography, controls, and state indicators.
10. Validate with `npm run build` and the existing Rust checks if backend files change.

## Acceptance Criteria

- Opening the app shows Capture, not settings.
- Capture has one dominant record/stop action.
- Settings contains keybind and Soniox API key controls.
- Pressing the global shortcut while Settings is open switches to Capture and starts recording.
- Pressing the global shortcut while hidden shows Capture and starts recording.
- With "Focus window when recording starts" off, starting via shortcut records without raising or focusing the window.
- With "Hide window when recording stops" on, stopping via shortcut hides the window while the app stays resident.
- Transcript is rendered as readable selectable text, not a textarea.
- Copy action copies the displayed transcript.
- Missing API key sends the user to Settings with a clear setup message.
- UI remains compact at the minimum supported window size.
- No Soniox protocol details appear in UI text.

## Grilled Decisions

- **Should settings live on the first screen?** No. They are necessary but secondary.
- **Should the transcript be a textarea?** No. Final transcript is output, not primary input.
- **Should the trigger respect the currently open Settings view?** No. Dictation trigger should always route to Capture.
- **Should the UI auto-open Capture when recording starts?** Yes. Recording without the Capture view visible makes the app feel ambiguous.
- **Should missing API key fail silently?** No. It should route to Settings and explain the required setup.
- **Should the app use a full navigation sidebar?** No for MVP. A compact top-bar toggle is enough.
- **Should there be a visible tray/background control?** Settings should mention that closing hides to tray and quitting is explicit.

