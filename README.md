# QuickText

Fast speech-to-text dictation for your desktop. Press a key, speak, get text.

QuickText is a lightweight desktop app that turns your voice into text in seconds: trigger it with the primary button or a global shortcut, speak, trigger again to stop, and copy the transcript. Transcription streams through [Soniox](https://soniox.com) in real time, so results appear the moment you stop talking.

## Features

- One-key dictation: start and stop with a single action.
- Real-time transcription via Soniox streaming STT.
- Instant copy-to-clipboard for the latest transcript.
- Lives in the system tray; closing the window keeps the app running.
- Capture-first UI with settings (API key, shortcuts) kept out of the way.
- Companion CLI for compositor bindings, scripts, and other apps.
- Cross-platform: Linux, macOS, and Windows.

## Installation

> Prebuilt packages are not published yet. Build from source below.

### Prerequisites

- Node.js and npm
- Rust toolchain
- Tauri 2 [system dependencies](https://tauri.app/start/prerequisites/) for your platform

### Build From Source

```bash
git clone https://github.com/<you>/quicktext.git
cd quicktext
npm install
npm run tauri build
```

The binary is written to `src-tauri/target/release/quicktext`.

## Usage

Launch `quicktext` to start the app. It stays resident in the tray until you quit it explicitly.

1. Press the record button (or your global shortcut) to start listening.
2. Press again to stop.
3. Copy the transcript from the window.

Set your Soniox API key from the in-app Settings view; it is stored in the system keyring, never in plain files.

## CLI On Linux

The same binary doubles as a CLI for driving a running instance over a local socket — ideal for Wayland compositors like Hyprland or Sway, where X11 global shortcuts do not work.

```bash
# Launch the GUI
quicktext

# Toggle recording
quicktext toggle

# Toggle plus window management:
# shown/focused on start, hidden once the transcript is ready
quicktext toggle focus

# Print current state without changing anything
quicktext status

# Machine-readable output
quicktext status --json
```

Exit codes: `0` success, `1` failure, `2` app not running.

Bind it to a key in Hyprland:

```ini
bind = , F10, exec, quicktext toggle focus
```

Or Sway:

```ini
bindsym F10 exec quicktext toggle focus
```

## Documentation

- [Product brief](docs/product-brief.md)
- [Technical plan](docs/technical-plan.md)
- [Architecture decisions](docs/adrs/)
