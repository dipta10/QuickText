# ADR 0015: Local Privacy-Safe Support Diagnostics

## Status

Accepted (documentation only; implementation is not scheduled by this ADR).

## Context

QuickText is distributed directly to friends and early testers. When a failure happens on another machine, the current terminal output is usually unavailable and does not provide enough context to correlate app startup, recording, and Soniox activity. Testers need a simple way to collect useful diagnostics and send them to the maintainer.

QuickText processes dictated speech and a Soniox credential, so indiscriminate logging would create a serious privacy and secret-leak risk. Diagnostics must be useful without recording audio, transcript text, credentials, clipboard contents, raw provider payloads, or identifying device data.

The app is a long-running tray/menu bar process. One process launch can contain many recording sessions, and one recording may contain more than one Soniox connection attempt if retry or reconnect behavior is added. A single identifier is therefore not enough to explain the event hierarchy.

## Decision

Add backend-owned, structured local diagnostics with explicit correlation identifiers, bounded retention, and user-controlled export and deletion.

### Correlation Model

Use UUID v4 values for the following identifiers:

- `run_id`: Generated once when the resident QuickText process starts. It covers all UI, tray, shortcut, IPC, recording, and provider activity until that process exits.
- `recording_session_id`: Generated when a start transition is accepted, before microphone or provider setup. It remains stable through setup, capture, stop, finalization, success, cancellation, or failure and is released only at the terminal state.
- `provider_session_id`: Generated for each provider connection attempt. A retry within one recording keeps the recording ID and receives a new provider-session ID.
- `error_id`: Generated when an error is surfaced. The full value is written to diagnostics with the active run, recording, and provider context.

User-visible failures show a compact support reference in the form `QT-7F3A91C2D4E8`, derived from the first 12 hexadecimal characters of `error_id` after removing separators. The reference helps identify the exact error event after the user exports diagnostics; it is not a replacement for the full identifiers in the log.

### Event Format

Persist one JSON object per line. Every event contains:

- Schema version.
- UTC timestamp.
- Level: `info`, `warn`, or `error`; `debug` is permitted only while temporary debug mode is active.
- Component and stable event name.
- `run_id`.
- Optional `recording_session_id`, `provider_session_id`, and `error_id` when that context exists.
- An allowlisted metadata object specific to the event.

Events should describe lifecycle and outcomes rather than arbitrary prose. Initial coverage includes:

- App start, clean shutdown, app version, immutable `build_id`, `source_revision`, OS family/version, and architecture. Release builds derive `build_id` from the distinct release tag or CI run; `source_revision` is the source commit and is not treated as the artifact identity.
- Trigger source and backend state transitions.
- Recording start, stop, cancel, duration, normalized audio format, chunk counts, and device fallback as a boolean outcome.
- Provider connection attempt, connected, finalizing, completed, retry, timeout, and sanitized failure category.
- Settings, credential-store, clipboard, tray, shortcut, IPC, and export operation success or failure without logging their sensitive values.

The backend diagnostics service owns the schema and writes. It accepts typed event variants with bounded fields rather than arbitrary messages or metadata maps. Frontend events, where needed, go through a narrow allowlisted interface. The app must not automatically forward arbitrary browser `console` output into persisted logs.

### Privacy Boundary

Normal and debug diagnostics must never contain:

- Soniox API keys, authorization headers, access tokens, or credential-store values.
- Audio samples, encoded audio, transcript text, partial transcript text, or clipboard contents.
- Raw Soniox request or response payloads.
- Usernames, home-directory paths, hostnames, IP addresses, or stable device identifiers.
- Microphone display names or persisted device IDs.

Provider and system errors must be mapped to stable app-level categories, reviewed error codes, and other bounded typed fields before persistence. Free-form provider, operating-system, dependency, and panic messages are never persisted; sanitizing arbitrary text cannot guarantee the privacy boundary. User-facing error text is generated from app-level categories instead of copied into diagnostics.

Incorrect-transcription investigations are outside this diagnostics channel. A tester may separately provide an example recording or transcript with explicit consent, but QuickText does not capture that content in logs.

### Levels And Debug Mode

Production logging defaults to `info`, `warn`, and `error`.

Settings exposes a temporary debug mode under Support. Enabling it:

- Shows a clear explanation that more detailed technical metadata will be recorded, while the same content and secret exclusions still apply.
- Lasts until the resident process exits or for 30 minutes, whichever happens first.
- Does not persist as an app preference or silently re-enable after restart.
- Adds finer lifecycle and timing events, not audio, transcripts, credentials, or raw payloads.

### Storage And Retention

Write logs to the platform-recommended application log directory resolved through Tauri's path APIs. A single backend writer task owns the active file and serializes event writes, flushes, rotation, export snapshots, and deletion.

Use size-based rotation and retain at most five files of 2 MiB each, for a maximum retained log footprint of approximately 10 MiB. Also remove files older than 14 days. Cleanup runs when diagnostics initialize and after rotation; files cannot be aged out while QuickText is not running and are removed on its next launch. Delete the oldest files first whenever either bound is exceeded.

No diagnostic event is uploaded automatically. QuickText has no remote telemetry or remote crash-reporting service under this decision.

### User Support Workflow

Add a Support section to Settings with:

- **Export diagnostics**: Explains what is and is not included, asks for explicit confirmation, then creates a user-selected ZIP archive.
- **Delete local diagnostics**: Removes retained logs and starts a fresh current log without affecting previously exported archives.
- **Temporary debug logging**: Enables the bounded debug mode described above.

The export contains the current and rotated JSON Lines log files plus a manifest with the diagnostics schema version, QuickText version, immutable `build_id`, `source_revision`, OS family/version, architecture, locale, and export timestamp. The build identifier is required because rolling pre-releases can share the same app version and the same source revision can be rebuilt. The bundle must not include settings files, credentials, audio, transcripts, clipboard data, raw provider payloads, usernames, home paths, device names, or network identifiers.

Export requests are serialized through the writer task. It flushes the active file, snapshots the retained set while new event writes wait, creates the archive from that stable snapshot, then resumes writes. Delete requests close the active file, remove the retained set, and open a new file before writes resume. This avoids copying partial JSON records or deleting an open file differently across platforms.

The user sends the archive voluntarily through a channel of their choice. Export does not upload or transmit it.

### Implementation Boundary

Use a diagnostics-owned writer rather than a process-global logging sink. Process-global sinks can capture dependency records that bypass QuickText's typed allowlist, and plugin-owned file handles do not provide the snapshot/reset lifecycle required by export and deletion. Rust application code logs through typed diagnostics methods. Frontend code does not receive logging permissions, filesystem paths, or direct file access.

Existing `eprintln!` calls that represent backend diagnostics should move to this boundary when implemented. CLI stdout/stderr intended as command output remains CLI output and must not be conflated with persisted diagnostics.

Crash dumps, OS-level crash reporting, Rust panic capture, and automatic log submission are out of scope. This feature covers handled application events, warnings, and errors.

## Consequences

- A tester can export one bounded support bundle without finding platform-specific directories manually.
- Run, recording, provider-attempt, and error identifiers make asynchronous failures traceable across components.
- Metadata-only diagnostics can explain lifecycle, timing, audio configuration, retry, and provider failures but cannot independently diagnose the linguistic cause of an incorrect transcript.
- A centralized typed event schema and rejection of free-form external messages add implementation work but make privacy review and regression testing practical.
- Five 2 MiB files may rotate quickly during temporary debug logging; this is acceptable because the mode is time-bounded and intended for immediate reproduction and export.
- Export and deletion require backend commands, a file-save dialog, and writer-task synchronization, but the frontend remains isolated from raw diagnostic files.
- The support reference becomes user-visible error metadata and should be included anywhere an actionable backend error is rendered. It maps to the full event only while that event remains in the bounded retained set, so the UI should encourage export soon after reproduction.

## Grilled Decisions

- **Local or remote diagnostics?** Local only. Automatic telemetry adds consent, infrastructure, and privacy obligations that are unnecessary for friend distribution.
- **Structured or human-oriented files?** JSON Lines. Stable fields and event names are easier to filter and correlate; the export manifest explains the bundle to humans.
- **Which identifiers exist?** Run, recording session, provider session, and error. They model different lifetimes and should not be collapsed.
- **What happens on a provider retry?** The recording ID remains stable and each provider attempt receives a new provider-session ID.
- **Can logs contain transcripts in debug mode?** No. Debug mode increases timing and lifecycle detail without weakening the content boundary.
- **How are logs bounded?** Five files at 2 MiB each and a 14-day maximum age, oldest first.
- **How does a tester share logs?** Explicit Settings export after a privacy explanation and confirmation; QuickText never uploads the archive.
- **Should users find the raw log folder?** Not for the normal support workflow. Export abstracts platform-specific paths.
- **What does the support reference identify?** One error event, correlated in the log with its run, recording, and provider context.
- **Are crashes covered?** No. Handled events and errors only; panic hooks, dumps, and remote crash reporting are separate future decisions.
- **Why not send typed events through the Tauri logging plugin?** Its `KeepSome` rotation is bounded, but its process-global logger can admit non-allowlisted dependency records and its owned file lifecycle does not provide the synchronized snapshot/reset operations required by export and deletion. A diagnostics-owned writer preserves the privacy boundary.

## Open Questions

- Should a later crash-reporting ADR add a panic hook or OS crash dumps while preserving local-only consent?
- Should future commercial distribution add an opt-in remote support channel, data-processing policy, and automatic expiry for uploaded bundles?

## References

- [ADR 0003](0003-backend-first-soniox-integration.md): backend-owned app controller and provider boundary.
- [ADR 0007](0007-immediate-capture-with-provider-buffering.md): recording and provider connection lifetimes.
- [ADR 0013](0013-persist-release-history.md): distinct rolling pre-releases that require an immutable build identifier.
- [Tauri 2 logging plugin](https://v2.tauri.app/plugin/logging/): evaluated process-global logging alternative.
- [`RotationStrategy::KeepSome`](https://docs.rs/tauri-plugin-log/latest/tauri_plugin_log/enum.RotationStrategy.html): bounded plugin rotation considered but not selected.
