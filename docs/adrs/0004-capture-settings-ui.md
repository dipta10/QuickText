# ADR 0004: Split Capture And Settings UI

## Status

Accepted for the UI redesign slice.

## Context

The current UI places the record button, transcript output, shortcut configuration, and Soniox API key controls in one vertical stack. That makes the app feel like a setup form rather than a fast dictation utility.

The product requirement is that the user can trigger QuickText, start recording immediately, stop with the same trigger, then review and copy the transcript. Settings are important, but they are not part of the normal dictation loop.

## Decision

Split the UI into two views:

- Capture: record/stop, recording status, transcript display, and copy action.
- Settings: Soniox API key, global shortcut, behavior preferences, and app/tray information.

Global shortcut, tray trigger, and primary button behavior should route the user to Capture when starting or stopping dictation.

## Consequences

- The first screen becomes focused on dictation.
- API key and shortcut controls no longer compete with the transcript.
- The frontend needs explicit view state.
- Missing API key handling should be designed as a guided Settings state.
- Transcript output can be rendered as selectable display text instead of a textarea.
- `src/app-view.ts` should remain a DOM factory, but its template should expose separate Capture and Settings regions.

## Grilled Decisions

- **Does Capture contain settings controls?** No.
- **Does Settings contain the record button?** No.
- **What happens when the user triggers recording from Settings?** Switch to Capture and start recording.
- **What happens when the user lacks a Soniox API key?** Switch to Settings and show the Soniox setup state.
- **Is a tab/sidebar framework needed?** No. Use simple view state and DOM rendering for MVP.

