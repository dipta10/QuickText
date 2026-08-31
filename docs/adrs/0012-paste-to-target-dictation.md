# ADR 0012: Paste-To-Target Dictation

## Status

Accepted for the paste-to-target feature slice.

## Context

QuickText currently ends every dictation loop inside its own window: recording steals focus (`show_main_window` shows and focuses the window), and the transcript must be manually copied. For system-wide dictation, the transcript should land directly in the app the user was working in.

There is no universal cross-platform API for inserting text into another application's focused field. The pragmatic mechanism is writing the transcript to the clipboard and synthesizing a paste keystroke (Ctrl+V / Cmd+V) — the approach used by comparable dictation tools. Synthetic input has platform constraints: Windows SendInput works out of the box, macOS requires Accessibility permission, Linux works on X11 but may be refused by some Wayland compositors.

A naive implementation conflicts with existing behavior because showing QuickText would steal focus before the paste can be delivered.

## Decision

Add an opt-in Settings checkbox, "paste transcript into the focused app". When enabled:

1. Global-shortcut recordings preserve the configured `focus_on_start` behavior. When QuickText is shown for a shortcut take, it hides after the transcript is ready and before delivery. Plain CLI/IPC recordings remain **headless**. An explicit `quicktext toggle focus` keeps its window contract: show and focus QuickText on start, then hide it after the transcript is ready on stop.
2. The paste target is whichever application or field is focused **when the paste is delivered**. Headless takes paste directly into the current destination. A visible shortcut or focused CLI take hides QuickText, allows the operating system to transfer focus, and then pastes into the destination selected by the operating system. QuickText never captures or restores the application that was focused when recording began.
3. If QuickText's own window is focused at trigger time, the take behaves normally (no self-paste).
4. On finalize, the transcript is written to the clipboard and a paste keystroke is synthesized into the currently focused destination. Visible shortcut and focused CLI takes hide QuickText and briefly wait for focus transfer before synthesizing the keystroke.
5. The clipboard keeps the transcript after pasting (no restore); the user can paste again elsewhere. Clipboard-manager interference is accepted.
6. In-window button and tray-initiated recordings keep current behavior; global-shortcut and CLI/IPC triggers can paste.
7. There is no cancel gesture for headless takes in this slice; a misfired take pastes on finalize.
8. If the paste fails (missing permission, compositor refusal, or input-synthesis failure), the transcript stays in the clipboard and an error message appears in the app UI; an OS error notification is planned once notification surfaces are wired (see ADR 0010).
9. Failed finalization produces no paste, as today.

## Consequences

- Recording supports both a visible shortcut workflow and an uninterrupted headless CLI workflow.
- The backend needs a platform abstraction for synthesizing paste, with per-platform implementations behind one seam.
- macOS builds need an Accessibility-permission check with recovery guidance in Settings.
- Wayland sessions may be unable to synthesize input; the fallback path is the documented behavior there.
- The same shortcut gains behavior conditional on a setting — documentation and UI copy must make the mode explicit.
- Error reporting for failed pastes is deferred to the app UI until notification integration lands.

## Grilled Decisions

- **Is the feature opt-in?** Yes; off by default, gated by a single Settings checkbox.
- **Does triggering still steal focus when the feature is on?** The global shortcut honors `focus_on_start`; if it shows QuickText, the app hides before delivery. Plain CLI/IPC takes do not take focus. `quicktext toggle focus` explicitly shows and focuses QuickText while recording, then hides it before delivery.
- **Which triggers paste?** Global shortcut and companion CLI/IPC toggles. Button and tray takes keep current behavior.
- **What does `quicktext toggle focus` do?** It preserves its original show-on-start and hide-on-stop contract. With paste-to-target enabled, delivery happens after QuickText hides and the operating system transfers focus.
- **How is text delivered?** Clipboard plus synthesized paste keystroke; direct insertion APIs rejected as non-universal.
- **What happens to the user's prior clipboard content?** It is overwritten and not restored; the transcript remains available for re-pasting.
- **What happens on paste failure?** Transcript stays in clipboard; error shown in app UI now, OS notification later.
- **Can a headless take be cancelled?** Not in this slice; no cancel gesture for plain CLI/IPC takes.
- **When is the paste target determined?** At delivery time. QuickText pastes into whatever destination is focused when transcription finishes and never changes focus itself.
