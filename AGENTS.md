# AGENTS.md

## Project

This is a desktop speech-to-text app for fast short dictation. The MVP target is a compact cross-platform Tauri 2 app for Linux, macOS, and Windows.

Core loop:

1. Trigger the app with the primary button or configured global shortcut.
2. Start recording.
3. Trigger again to stop.
4. Finalize transcription through Soniox real-time STT.
5. Show the transcript and let the user copy it.

Read the project docs before making architectural changes:

- `docs/product-brief.md`.
- `docs/domain-model.md`.
- `docs/technical-plan.md`.
- `docs/implementation-plan.md`.
- `docs/adrs/0001-tauri-desktop-shell.md`.
- `docs/adrs/0002-streaming-soniox-stt.md`.

## Current Stack

- Desktop shell: Tauri 2.
- Frontend: Vite plus TypeScript, currently plain DOM code.
- Backend: Rust Tauri commands and events.
- Global shortcuts: `tauri-plugin-global-shortcut`.
- Planned audio capture: Rust backend, likely `cpal`.
- Planned transcription: Soniox real-time WebSocket STT.

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
- Do not leak Soniox-specific protocol details into the UI.
- Do not store API keys in ordinary config files, local storage, or committed files.
- Backend state should become the source of truth for recording/transcription state as the app grows.
- Global shortcuts should emit app events or call app-controller behavior, not duplicate recording logic in the frontend.
- Use Tauri commands/events for frontend-to-backend communication.
- Add or update Tauri capability permissions when adding new commands or plugins.
- Keep provider integration behind a narrow provider boundary so Soniox can be changed later.

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

- `src/shortcut.ts`: shortcut parsing, validation, and display formatting.
- `src/settings.ts`: non-secret local settings persistence.
- `src/tauri.ts`: Tauri command and event wrappers.
- `src/app-state.ts`: recording and shortcut-capture state transitions.
- `src/main.ts`: DOM setup, rendering, and event wiring.

## Git And Generated Files

- `node_modules/`, `dist/`, and Rust `target/` output are ignored and should not be committed.
- `package-lock.json` and `src-tauri/Cargo.lock` are committed for reproducible app builds.
- Tauri may update files under `src-tauri/gen/schemas/` when plugins or permissions change; include those updates when they are caused by the change.

## Product Constraints

- MVP is desktop-first, not browser-first.
- MVP should support Linux, macOS, and Windows.
- The primary control should remain a single start/stop action.
- Manual copy is the MVP default; auto-copy can be added later as an option.
- Transcript history, diarization UI, long-form import, and multi-provider UI are out of MVP scope.
