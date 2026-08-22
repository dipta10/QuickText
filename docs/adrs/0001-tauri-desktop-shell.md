# ADR 0001: Use Tauri For Desktop Shell

## Status

Accepted for initial implementation.

## Context

QuickText needs to run on Linux, macOS, and Windows with a lightweight UI, tray/menu bar residency, global trigger support, clipboard integration, local settings, and access to native microphone capture.

Electron is mature but heavier. Native per-platform shells offer the best integration but multiply implementation work. Tauri gives a small desktop shell with a Rust backend and web frontend.

## Decision

Use Tauri 2 for the initial implementation.

The frontend will own presentation and interaction. The Rust backend will own microphone capture, provider streaming, tray/menu bar behavior, shortcut handling, clipboard integration, and secret access.

## Consequences

- Rust becomes part of the core implementation.
- Audio capture should be implemented in Rust rather than browser APIs.
- Tray/menu bar lifecycle behavior should be implemented in the Tauri shell layer.
- Tauri permissions and plugin setup must be handled deliberately.
- Cross-platform packaging remains simpler than maintaining three native apps.
- If audio capture or key storage becomes painful, the decision should be revisited after a spike rather than after full implementation.

## References

- Tauri global shortcut plugin: https://v2.tauri.app/plugin/global-shortcut/
- Tauri plugin overview: https://v2.tauri.app/plugin/
