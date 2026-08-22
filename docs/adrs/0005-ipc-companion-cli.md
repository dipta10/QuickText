# ADR 0005: Expose Toggle And Status Through Local IPC And Companion CLI

## Status

Accepted for the IPC implementation slice.

## Context

The global shortcut plugin relies on X11 key grabs, which do not work for Wayland-native windows on compositors such as Hyprland, Sway, and COSMIC. Registration reports success but the handler never fires when another window has focus. This is compositor security design, not an app bug: Wayland clients cannot grab keys outside their own surfaces.

Compositor-level keybindings are the reliable Wayland mechanism, but they can only launch executables or send signals; they cannot click UI buttons. QuickText therefore needs a way for external processes to trigger app behavior.

QuickText is already tray-resident with a backend-owned app controller, so a local control channel reuses the existing toggle path instead of adding parallel recording logic.

## Decision

Run a local IPC listener inside the resident app process and branch the same binary into CLI mode based on argv.

- The app binds one local socket at startup: Unix domain socket on Linux/macOS, named pipe on Windows.
- Socket location follows platform conventions (for example `$XDG_RUNTIME_DIR/quicktext.sock`) with owner-only permissions.
- The socket bind doubles as single-instance enforcement: if the socket is taken, the second GUI instance exits after forwarding or reporting to the running instance.
- The binary supports subcommands:
  - `quicktext` with no arguments launches the GUI.
  - `quicktext toggle` toggles recording through the same backend path as the button and shortcut.
  - `quicktext toggle focus` additionally manages window visibility: when it starts a recording, the main window is shown and focused; when it stops a recording, the window hides after the transcript is ready. The flag applies per call only.
  - `quicktext status` prints current state without changing it.
- Plain IPC toggles must not steal window focus; showing the window stays a UI-button behavior unless `focus` is requested.
- Wire format is line-delimited JSON with a protocol version field; responses carry the app snapshot.
- CLI output is human-readable text by default and full JSON with `--json`.
- Exit codes are deterministic: 0 success, 1 generic failure, 2 app not running (`toggle` fails fast rather than auto-launching).

The MVP IPC surface is `toggle` and `status` only.

## Consequences

- Compositor bindings like `bind = , F10, exec, quicktext toggle` work regardless of focused window or display server.
- One toggle path serves button, shortcut, and CLI; no duplicated recording logic.
- Single-instance enforcement prevents two processes fighting over the microphone.
- The socket is a new attack surface that must stay owner-only and loopback-only.
- Windows named pipes need their own transport implementation behind the same interface.
- Later commands (`start`, `stop`, transcript fetch) can be added without breaking the wire format.

## Grilled Decisions

- **Transport?** Local socket/named pipe behind one abstraction; D-Bus rejected as Linux-first and heavier; signals rejected as fragile and non-viable on Windows.
- **Toggle when app not running?** Fail fast with exit code 2; auto-launch was rejected as surprising side effects from a keypress.
- **Status output format?** Text by default, `--json` flag returns the snapshot; JSON-only annoys humans, text-only starves scripts.
- **Does IPC toggle focus the window?** Plain toggles do not. `toggle focus` opts into window management: show and focus on start, hide after the transcript is ready on stop, per call only.
- **When does the focused stop hide the window?** After finalization so the transcript is visible and auto-copy can fire; immediate hiding was rejected because manual-copy users would never see the result.
- **Single instance enforced via socket?** Yes.
- **Binary layout?** Same binary, argv branch; a separate CLI binary doubles packaging work for MVP.
- **Commands in MVP?** `toggle` and `status`; `start`/`stop` deferred until a real caller needs them.
- **Wire format?** Line-delimited JSON with version field; plain text rejected because `--json` and error reporting get mushy.
- **Exit codes?** 0/1/2 as above, decided without grilling since scripts require determinism and no real alternative existed.

## Open Questions

- Should the socket accept requests only from the same user via peer credential checks where supported?
- Should `status` include transcript text, or keep transcripts out of IPC until a fetch command exists?
- What timeout should the CLI use before declaring the resident app unresponsive?
