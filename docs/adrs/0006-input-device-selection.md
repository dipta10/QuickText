# ADR 0006: Microphone Input Device Selection

## Status

Accepted for the input-device-selection slice.

## Context

The domain model assigns microphone device selection to `AudioRecorder`, but no mechanism exists for users to choose which microphone records. Today capture uses whatever default device the audio backend picks.

The product requirement is that the user can select an input device in Settings, and that selection governs which microphone a recording session captures from.

## Decision

- Device selection lives in Settings only; Capture stays free of settings controls.
- The picker offers a "System default" entry plus one entry per available input device.
- The "System default" entry displays the resolved OS default device name alongside the label.
- The chosen device is persisted in the settings store as a stable device ID, with the system default used when no explicit choice is saved.
- The backend reads device state at record start. A missing configured device falls back to the OS default for that session.
- Changing the device applies to the next recording session; active recordings keep the device they started with.
- The frontend queries the device list on demand (when Settings opens); no hot-plug event stream for MVP.

## Consequences

- The settings store gains a non-secret `input_device_id` field.
- The backend needs commands to list input devices and to set/clear the selected device.
- Fallback is silent for MVP: when recording starts via global shortcut with the window hidden, the user may not notice a different mic was used. This gap is accepted until notification support exists.
- No hot-plug detection means the Settings device list can go stale while open; it refreshes each time Settings opens.
- Stable-ID persistence may still break across OS driver updates; fallback covers this.

## Grilled Decisions

- **Is this MVP scope?** Yes.
- **Where does the picker live?** Settings only.
- **Is there a system-default option?** Yes, showing the resolved default device name.
- **What happens if the configured device is missing at record start?** Fall back to the OS default; show an inline notice in the Capture view.
- **How are devices identified?** Stable device ID; fall back to system default when the ID is not found.
- **Can the device change mid-recording?** No. Selection applies to the next session.
- **Does selection take effect immediately?** Yes, saving updates backend state so the next toggle uses it.
- **How does the UI get the device list?** On-demand query when Settings opens; no hot-plug events for MVP.
- **Inline notice vs OS notification for fallback?** Inline notice only; the hidden-window miss is accepted until notifications are added.
- **New error category for device failures?** No. Reuse existing categories (`microphone_permission_denied`, `no_microphone_device`) with device context in the message.
