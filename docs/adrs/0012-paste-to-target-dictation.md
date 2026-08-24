# ADR 0012: Paste-To-Target Dictation

## Status

Accepted for the paste-to-target feature slice.

## Context

QuickText currently ends every dictation loop inside its own window: recording steals focus (`show_main_window` shows and focuses the window), and the transcript must be manually copied. For system-wide dictation, the transcript should land directly in the app the user was working in.

There is no universal cross-platform API for inserting text into another application's focused field. The pragmatic mechanism is writing the transcript to the clipboard and synthesizing a paste keystroke (Ctrl+V / Cmd+V) — the approach used by comparable dictation tools. Synthetic input has platform constraints: Windows SendInput works out of the box, macOS requires Accessibility permission, Linux works on X11 but may be refused by some Wayland compositors.

A naive implementation conflicts with existing behavior: triggering currently steals focus, so "the previously focused app" would be lost.

## Decision

Add an opt-in Settings checkbox, "paste transcript into the previous app". When enabled:

1. Shortcut-triggered and CLI/IPC-triggered recordings run **headless**: the main window is never shown or focused, regardless of `focus_on_start` / `hide_on_stop` (those settings are bypassed at runtime for headless takes and remain untouched in Settings).
2. The paste target is the application focused **at trigger time**.
3. If QuickText's own window is focused at trigger time, the take behaves normally (no self-paste).
4. On finalize, the transcript is written to the clipboard and a paste keystroke is synthesized into the target.
5. The clipboard keeps the transcript after pasting (no restore); the user can paste again elsewhere. Clipboard-manager interference is accepted.
6. In-window button and tray-initiated recordings keep current behavior; only headless-capable triggers paste.
7. There is no cancel gesture for headless takes in this slice; a misfired take pastes on finalize.
8. If the paste fails (target gone, missing permission, compositor refusal), the transcript stays in the clipboard and an error message appears in the app UI; an OS error notification is planned once notification surfaces are wired (see ADR 0010).
9. Failed finalization produces no paste, as today.

## Consequences

- Recording becomes usable without any visual interruption — the core "dictate anywhere" loop.
- The backend needs a platform abstraction for capturing the focused app and synthesizing paste, with per-platform implementations behind one seam.
- macOS builds need an Accessibility-permission check with recovery guidance in Settings.
- Wayland sessions may be unable to synthesize input; the fallback path is the documented behavior there.
- The same shortcut gains behavior conditional on a setting — documentation and UI copy must make the mode explicit.
- Error reporting for failed pastes is deferred to the app UI until notification integration lands.

## Grilled Decisions

- **Is the feature opt-in?** Yes; off by default, gated by a single Settings checkbox.
- **Does triggering still steal focus when the feature is on?** No; headless takes never show or focus the window.
- **Which triggers paste?** Global shortcut and companion CLI/IPC toggles. Button and tray takes keep current behavior.
- **How is text delivered?** Clipboard plus synthesized paste keystroke; direct insertion APIs rejected as non-universal.
- **What happens to the user's prior clipboard content?** It is overwritten and not restored; the transcript remains available for re-pasting.
- **What happens on paste failure?** Transcript stays in clipboard; error shown in app UI now, OS notification later.
- **Can a headless take be cancelled?** Not in this slice; no cancel gesture.
- **When is the paste target determined?** At trigger time, not finalize time.
