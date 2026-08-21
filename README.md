# Speech to Text Desktop App

A snappy cross-platform desktop speech-to-text application using Soniox as the transcription provider.

## Goal

The app should make short dictation fast:

1. Press one button or shortcut to show the UI and start listening.
2. Press the same button again to stop recording.
3. Show the transcribed text immediately after recording ends.
4. Let the user copy the text to the clipboard.

The first version is focused on Linux, macOS, and Windows desktop use.

## Current Status

This repository is in the product definition stage. See:

- [Product brief](docs/product-brief.md)
- [Technical plan](docs/technical-plan.md)
- [Implementation plan](docs/implementation-plan.md)
- [Domain model](docs/domain-model.md)

## Core Experience

- One primary record/stop control.
- Low-latency startup and recording.
- Clear listening, processing, success, and error states.
- Copy-to-clipboard action for the latest transcript.
- Soniox-backed transcription.

## Early Constraints

- Desktop-first, not browser-first.
- Cross-platform support for Linux, macOS, and Windows.
- UI must feel lightweight and quick to open.
- Provider integration should be isolated so Soniox-specific code does not leak through the rest of the app.

## Initial Direction

The implementation plan currently assumes Tauri 2 with a web frontend and Rust backend. Audio should stream to Soniox over its real-time STT WebSocket API so stopping the recording can finalize an active session instead of starting transcription from scratch.
