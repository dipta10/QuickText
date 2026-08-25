# ADR 0013: Launch On System Startup

## Status

Accepted (documentation only; implementation is not scheduled by this ADR).

## Context

QuickText is designed to stay resident in the tray/menu bar so dictation is always one shortcut away. Today the user must launch the app manually after every login, which breaks that promise for the most common workflow: boot the machine, immediately dictate somewhere.

Desktop platforms provide a standard "launch at login" mechanism, and Tauri exposes it through `tauri-plugin-autostart` (systemd user service or XDG autostart on Linux, Login Item on macOS, registry `Run` key on Windows). This decision covers startup behavior only; it does not change capture, transcription, or window management.

## Decision

Add an opt-in **"Launch on startup"** checkbox in a General section of the Settings view:

- The checkbox defaults to **unchecked**; QuickText never registers itself for autostart without explicit consent.
- Toggling the checkbox calls a dedicated backend command (`set_launch_on_startup`) that enables or disables autostart via `tauri-plugin-autostart`. The frontend never touches the plugin directly, matching the backend-first integration boundary in [ADR 0003](0003-backend-first-soniox-integration.md).
- The preference itself persists as an ordinary non-secret local setting so the Settings view can reflect the saved choice on next launch; the OS registration state remains the source of truth for whether autostart actually happens.
- When enabled, autostart launches the app **hidden** (window not shown); the app appears only in the tray/menu bar, consistent with resident-tray behavior.
- If the platform denies or fails the autostart registration, the command returns an error and the Settings view shows an inline warning; recording and transcription continue to work normally.
- Capability permissions are added for the new command and the autostart plugin per the project rule of updating `src-tauri/gen/schemas/` alongside plugin changes.

## Consequences

- One new settings key must round-trip through local settings persistence and reach the backend controller.
- The backend gains a dependency on `tauri-plugin-autostart` and owns its lifecycle.
- Autostart-hidden behavior means the backend already-visible window show/hide paths are exercised at process start; startup ordering needs a test.
- Tests must cover: default-off, toggle round-trip persistence, backend enable/disable call, and inline warning on failure.
- Flatpak/Snap packaging may restrict autostart mechanisms; the pre-release installers use native packages, so this is out of scope here but worth revisiting if store packaging lands.

## Grilled Decisions

- **Opt-in or opt-out?** Opt-in. Registering autostart silently violates user expectations for a small utility.
- **Frontend or backend owns the plugin?** Backend. The frontend renders the checkbox and invokes a command; keeping OS integration behind the backend controller preserves the provider/plugin boundary.
- **Launch hidden or visible?** Hidden. The whole point is background readiness; showing a window on every boot would be noise.
- **Setting or OS state as source of truth?** Both have a role: the persisted setting drives the UI, while the OS registration decides actual behavior. The backend command keeps them in sync on every toggle.
- **One checkbox or extra options (e.g., start minimized vs. hidden)?** One checkbox for MVP; the app always hides to tray anyway, so there is no meaningful second mode yet.

## References

- [ADR 0003](0003-backend-first-soniox-integration.md): backend-owned app controller where autostart registration belongs.
- [ADR 0004](0004-capture-settings-ui.md): Capture/Settings split hosting the General section.
