# AGENTS.md

## Project

QuickText is a desktop speech-to-text app for fast short dictation. The MVP target is a compact cross-platform Tauri 2 app for Linux, macOS, and Windows.

Core loop:

1. Trigger the app with the primary button or configured global shortcut.
2. Start recording.
3. Trigger again to stop.
4. Finalize transcription through Soniox real-time STT.
5. Show the transcript and let the user copy it.

Once started, the app should remain resident in the tray/menu bar until the user explicitly quits. Closing the main window should hide it, not terminate the process.

Read the project docs before making architectural changes:

- `docs/product-brief.md`.
- `docs/domain-model.md`.
- `docs/technical-plan.md`.
- `docs/implementation-plan.md`.
- `docs/ui-plan.md`.
- `docs/glossary.md`.
- `docs/adrs/0001-tauri-desktop-shell.md`.
- `docs/adrs/0002-streaming-soniox-stt.md`.
- `docs/adrs/0003-backend-first-soniox-integration.md`.
- `docs/adrs/0004-capture-settings-ui.md`.
- `docs/adrs/0005-ipc-companion-cli.md`.
- `docs/adrs/0006-input-device-selection.md`.
- `docs/adrs/0008-ci-test-pipeline.md`.
- `docs/adrs/0009-cd-rolling-pre-release.md`.
- `docs/adrs/0010-optional-capture-notifications.md`.
- `docs/adrs/0011-provider-declared-settings.md`.
- `docs/adrs/0012-paste-to-target-dictation.md`.
- `docs/adrs/0013-persist-release-history.md`.
- `docs/adrs/0014-launch-on-startup.md`.
- `docs/adrs/0015-persistent-local-diagnostic-logging.md`.

## Current Stack

- Desktop shell: Tauri 2.
- Frontend: Vite plus TypeScript, currently plain DOM code.
- Backend: Rust Tauri commands and events.
- Global shortcuts: `tauri-plugin-global-shortcut`.
- Tray/background mode: app remains resident after launch; window close hides to tray/menu bar.
- Launch on startup: opt-in through `tauri-plugin-autostart`; autostart launches hidden.
- Planned diagnostics: privacy-safe local logs in the platform app-log directory with UTC timestamps, bounded rotation, and manual sharing; see ADR 0015.
- Planned audio capture: Rust backend, likely `cpal`.
- Planned transcription: Soniox real-time WebSocket STT.
- CI: GitHub Actions runs Rust tests and a frontend typecheck on pushes to `main` and PRs targeting `main`.
- CD: every push to `main` publishes unsigned installers for Linux, macOS, and Windows as its own release (`v0.1.0-pre.<run-number>`); the newest 10 releases are kept for rollback; see ADR 0013.

## Local Commands

Use these checks before handing off changes:

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

Run the frontend preview with:

```bash
npm run dev
```

Run the desktop app with:

```bash
npm run tauri dev
```

## Development Notes

- Keep the UI dark mode by default.
- Keep the app compact and task-focused; avoid marketing-page structure.
- Keep the UI Capture-first: recording, transcript review, and copy belong in Capture; keybinds, API keys, and preferences belong in Settings.
- Do not render the final transcript as a textarea; it should be selectable output text.
- Do not leak Soniox-specific protocol details into the UI.
- Do not store API keys in ordinary config files, local storage, or committed files.
- Backend state should become the source of truth for recording/transcription state as the app grows.
- Global shortcuts should emit app events or call app-controller behavior, not duplicate recording logic in the frontend.
- IPC commands and companion CLI calls must go through the same backend toggle path as UI triggers; they must not show or focus windows.
- Tray/menu bar actions should show/hide the same main window and use explicit quit for process exit.
- Use Tauri commands/events for frontend-to-backend communication.
- Add or update Tauri capability permissions when adding new commands or plugins.
- Keep provider integration behind a narrow provider boundary so Soniox can be changed later.
- Never log API keys, transcripts, partial transcripts, raw audio, clipboard content, provider frames, device identifiers, or target-window details.

## Architecture Rules

- Keep `src/main.ts` thin. It should compose the app, wire DOM events, and connect adapters.
- Move reusable behavior into focused modules with small public interfaces.
- Prefer pure functions for parsing, formatting, validation, and state transitions.
- Keep side effects at the edges: DOM updates, `localStorage`, Tauri `invoke`, Tauri `listen`, audio capture, clipboard access, and network calls.
- Keep provider-specific transcription details outside UI code.
- Preserve the provider boundary described in `docs/technical-plan.md`.
- Prefer explicit app states over scattered booleans as recording and transcription behavior grows.
- Test behavior through public module interfaces, especially shortcut formatting and state transitions.
- Do not introduce a frontend framework unless the UI becomes complex enough to justify it.
- Do not add broad abstractions before there are at least two real call sites or a clear project boundary.

## Frontend Boundaries

When splitting frontend code, prefer these module boundaries:

- `src/app-view.ts`: HTML template creation and DOM element lookup only.
- `src/shortcut.ts`: shortcut parsing, validation, and display formatting.
- `src/settings.ts`: non-secret local settings persistence.
- `src/tauri.ts`: Tauri command and event wrappers.
- `src/app-state.ts`: recording and shortcut-capture state transitions.
- `src/main.ts`: DOM setup, rendering, and event wiring.

Keep these responsibilities separate. Do not move HTML templates, shortcut parsing, settings persistence, or raw Tauri command/event names back into `src/main.ts`. Do not put app behavior, state transitions, Tauri calls, or persistence into `src/app-view.ts`; it should remain a DOM factory.

The UI should expose separate Capture and Settings regions. `src/main.ts` may own which view is active, but recording/transcription state should come from backend app-state events once real recording is connected.

## Git And Generated Files

- Create git worktrees inside `worktrees/` (e.g. `git worktree add worktrees/<branch-name> <branch>`); this folder is gitignored so agents can edit within the workspace without permission prompts.
- `node_modules/`, `dist/`, and Rust `target/` output are ignored and should not be committed.
- `package-lock.json` and `src-tauri/Cargo.lock` are committed for reproducible app builds.
- Tauri may update files under `src-tauri/gen/schemas/` when plugins or permissions change; include those updates when they are caused by the change.

## Product Constraints

- MVP is desktop-first, not browser-first.
- MVP should support Linux, macOS, and Windows.
- The primary control should remain a single start/stop action.
- Once launched, the app should run in the background/tray until explicit quit.
- Manual copy is the MVP default; auto-copy can be added later as an option.
- Transcript history, diarization UI, long-form import, and multi-provider UI are out of MVP scope.
