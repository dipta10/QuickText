# ADR 0015: Persist Privacy-Safe Local Diagnostic Logs

## Status

Proposed (documentation only; no implementation is included in this ADR).

## Context

QuickText is a tray-resident desktop app, so most failures happen without a useful terminal attached. The Rust backend currently writes a few failures with `eprintln!`, while other backend and frontend failures exist only as transient UI or CLI messages. The release binary also suppresses the console window on Windows. After the app restarts, there is therefore no durable record a user can attach to a support report.

There is no single cross-platform “industry-standard log file” for desktop applications. The established baseline is a combination of conventions:

- use the application framework's logging facility instead of inventing a writer;
- write into the operating system's recommended per-application log directory;
- emit one timestamped, severity-classified event per line with enough context to correlate an operation;
- bound disk usage with rotation and retention;
- exclude credentials, user content, and unnecessary personal data;
- make logging best-effort so a logging failure does not break the product; and
- require an explicit user action before diagnostic data leaves the device.

This matches Tauri's official logging support and the OWASP Logging Cheat Sheet. RFC 3339 supplies an unambiguous timestamp representation.

## Decision

### Logging stack and ownership

Use Tauri 2's first-party `tauri-plugin-log` as the local file sink and Rust's `log` facade as the application logging API.

- The Rust backend is the source of truth for operational events: app lifecycle, state transitions, audio capture, provider connectivity, IPC, tray integration, and credential-store outcomes.
- Replace diagnostic `eprintln!` calls with level-appropriate `log` events. Keep companion-CLI stderr messages that are part of its user-facing output contract; the resident backend should separately record the corresponding IPC event when it receives a request.
- Frontend code sends only frontend-owned diagnostics through `@tauri-apps/plugin-log`: uncaught UI exceptions, unhandled promise rejections, and failed Tauri invocations that are not already fully recorded by the backend.
- Do not mirror every backend event into the frontend logger; duplicate records make support timelines harder to read.
- A focused backend diagnostics module should own logger initialization, file policy, safe value sanitization, and log-directory lookup. Feature modules continue to emit through `log` macros.
- A thin frontend diagnostics adapter should own plugin calls and global error hooks. `src/main.ts` should only install that adapter during composition.

This logging pipeline is for local operational diagnostics, not analytics, telemetry, transcript history, or a tamper-proof audit trail. QuickText will not send logs to a server in this slice.

### Destination and filenames

Use the Tauri `LogDir` target instead of a custom folder. With the current bundle identifier `com.dipta.stt`, the expected directories are:

| Platform | Log directory |
| --- | --- |
| Linux | `$XDG_DATA_HOME/com.dipta.stt/logs`, falling back to `$HOME/.local/share/com.dipta.stt/logs` |
| macOS | `$HOME/Library/Logs/com.dipta.stt` |
| Windows | `%LOCALAPPDATA%\com.dipta.stt\logs` |

The active file is `quicktext.log`. Rotated files keep the plugin's timestamped `quicktext_<rotation-timestamp>.log` naming. QuickText appends to the active file across ordinary restarts; it does not create an unbounded file per launch.

The log directory is private to the current OS user under normal platform permissions. QuickText must not broaden those permissions.

### Record format

Write UTF-8, line-oriented text. Each physical line is one complete event and begins with a UTC RFC 3339 timestamp with millisecond precision. Use a stable format with these logical fields:

```text
2026-08-26T08:12:34.567Z [INFO] [quicktext::capture] event=recording_started run_id=01K... session_id=42 trigger=shortcut
```

Required fields are:

- `timestamp`: UTC RFC 3339 with milliseconds;
- `level`: `ERROR`, `WARN`, `INFO`, `DEBUG`, or `TRACE`;
- `target`: the Rust module or the explicit `quicktext::<area>` target;
- `event`: a stable `snake_case` event name; and
- `message`: optional short human context when the other fields are not sufficient.

Context fields are appended as `key=value` pairs where relevant. Values containing whitespace or delimiters must be quoted and escaped so an event cannot create a fake second line.

Each process launch gets an opaque `run_id`. Each accepted capture gets the backend recording `session_id`; all recording, audio, provider, and finalization events for that take carry it. The first event of a run records the QuickText version, build profile, OS, architecture, and whether the launch was interactive or autostart. Do not repeat stable process metadata on every event.

Do not put a timestamp only in the filename. Every record needs its own timestamp because one file spans many events and may span multiple launches.

### Levels

| Level | QuickText use |
| --- | --- |
| `ERROR` | An operation failed or the app reached an unexpected state: capture failure, provider failure, credential-store failure, IPC listener failure, or panic. |
| `WARN` | QuickText recovered but behavior changed: microphone fallback, ignored invalid state transition, notification/paste fallback, or log-sink degradation. |
| `INFO` | Low-volume lifecycle milestones: app start/quit, logger initialization, recording start/stop, provider connect/finalize outcome, and settings-change outcome. |
| `DEBUG` | Development-only diagnostic detail such as validated audio format, state transition detail, and operation timings. |
| `TRACE` | Very high-volume local investigation only. It is disabled by default and must never include audio chunks, provider frames, or transcript tokens. |

Release builds default to `INFO`; debug builds may default to `DEBUG`. The initial implementation has no user-facing verbosity control. A temporary support mode may be considered later only if real support cases show that the normal timeline is insufficient. The file target is enabled in all builds. A terminal target is additionally enabled during development, but production support must never depend on stdout or stderr.

### Event policy

The default diagnostic log is a low-volume operational timeline, not an error-only dump. Log enough state transitions to reconstruct the core loop without logging what the user dictated.

Initial event families should cover:

- application and logger start, explicit quit, panic, and logging degradation;
- recording start/stop requests and accepted state transitions, trigger kind, maximum-duration stop, and final outcome;
- microphone open/start/stop, safe format metadata, sample/chunk counts, duration, fallback outcome, and categorized errors;
- provider connect/finalize timing and outcome, provider-neutral error category, and a safe provider status/error code when available;
- IPC listener start, already-running detection, request command name, outcome, timeout, and transport error;
- tray, shortcut, autostart, notification, paste, clipboard, and credential-store operation outcomes; and
- frontend bootstrap failure, uncaught exception, unhandled rejection, and failed invocation name/outcome.

Do not log successful high-frequency events such as individual audio chunks, partial-transcript updates, render cycles, timer ticks, or `status` polling. Aggregate them into counts and durations at session boundaries.

Logging an error does not replace the existing `AppError`, UI recovery text, or CLI exit code. User-facing errors remain concise; logs carry safe diagnostic context.

### Privacy and redaction

The default log must be safe enough for a user to review and manually share, but users should still be told to review files before sending them.

Never log:

- the Soniox API key, including a prefix, suffix, fingerprint, hash, or authorization header;
- final or partial transcript text;
- raw or encoded microphone audio, audio chunks, or provider WebSocket frames;
- clipboard contents or text intended for paste-to-target;
- raw provider request/response bodies;
- credential-store secret values;
- window titles, target-application names, or document names;
- usernames, hostnames, persistent installation identifiers, locale, or timezone;
- user home paths, arbitrary filesystem paths, microphone names, or persistent device IDs; or
- raw values from untrusted errors when they have not been reviewed for secrets and user content.

These exclusions apply in every build and at every log level, including `DEBUG`, `TRACE`, and any temporary support mode. Increased verbosity may add operational metadata, never dictated or copied content.

Prefer safe summaries such as `api_key_present=true`, `audio_bytes=184320`, `duration_ms=2150`, `error_type=provider_unavailable`, `provider_code=401`, or `configured_device_found=false`.

Redaction happens at the call site by choosing safe fields; a regex filter is not an adequate primary defense. The diagnostics boundary should additionally sanitize carriage returns, line feeds, delimiters, and any known in-memory secret before writing externally sourced text. Provider-specific code must map raw failures to safe categories before logging them.

The logger must not capture the `AppSnapshot` or `TranscriptResult` with blanket `Debug`/serialization because those values can contain transcript text.

### Rotation and retention

Use size-based bounded rotation:

- maximum active-file size: 5 MiB;
- keep four rotated files plus the active file;
- approximate maximum retained diagnostic data: 25 MiB; and
- append on launch, rotating only when the size limit is reached.

In `tauri-plugin-log`, this maps to `max_file_size(5 * 1024 * 1024)`, `RotationStrategy::KeepSome(4)`, and the append open strategy. Bounded rotation prevents a resident process from filling the user's disk while retaining enough low-volume history to diagnose a failure discovered after restart.

Retention is local and size-based, not a promise to keep a particular number of days. There is no cloud copy or backup controlled by QuickText. A later legal or commercial retention requirement must amend this ADR rather than silently changing the policy.

### User support flow

Add a quiet **Diagnostics** subsection under Settings > App when this ADR is implemented:

- **Open logs folder** opens the platform log directory.
- **Copy logs path** copies the resolved directory path for support instructions.
- Supporting text says that logs contain technical events and may be reviewed before sharing; logs are never uploaded automatically.
- If persistent logging is unavailable, show a non-blocking diagnostics warning without disabling recording or transcription.

The initial Diagnostics UI does not include **Clear logs**. Rotation already bounds retention, and a user who wants immediate deletion can explicitly quit QuickText and remove the files from the opened folder without adding active-file deletion semantics to the app.

When reporting a problem, a user should reproduce it if practical, note the approximate local time, then attach `quicktext.log`. If the problem occurred before a rotation or restart, support may also ask for the newest timestamped rotated file. Timestamps inside every record let support correlate the user's local report even though records use UTC.

If QuickText cannot launch, support documentation should list the platform paths above so the user can reach the files without opening the app.

An **Export diagnostic bundle** action is deferred. Opening the folder is transparent and has fewer privacy and packaging risks; a future exporter may add a manifest and selected logs only after a separate design review.

### Failure behavior

Logging is best-effort and must not become a dependency of the capture loop.

- Failure to create, write, rotate, or flush a log file must not prevent app startup, recording, transcription, copy, or explicit quit.
- Fall back to stderr where a console exists and expose one non-blocking diagnostics warning when possible.
- Avoid recursively logging a failure from the logger itself.
- Flush pending records during orderly quit. Install a panic hook that attempts one sanitized panic event and then preserves the normal panic behavior.
- Never retry a failed write in a tight loop.

The implementation must verify the Tauri plugin's behavior for an unwritable log directory. If the plugin would fail the entire Tauri build/setup path, initialization must wrap or replace that failure path so QuickText continues without the file sink.

## Implementation Outline

Implementation is intentionally deferred. The later slice should:

1. Add compatible `log`, `tauri-plugin-log`, and `@tauri-apps/plugin-log` dependencies and update lockfiles.
2. Add the log plugin before application services emit events, with explicit file size, retention, timestamp, level, and target configuration.
3. Add `src-tauri/src/diagnostics.rs` for initialization, safe sanitization, run context, and log-directory lookup.
4. Add only the Tauri capability permission needed for frontend logging, and include caused schema updates.
5. Add `src/diagnostics.ts` as the frontend plugin adapter and install global error/rejection hooks from `src/main.ts`.
6. Replace existing diagnostic `eprintln!` calls (without removing companion-CLI user output) and instrument low-volume lifecycle boundaries with stable event names and correlation IDs.
7. Add the Diagnostics settings controls without moving log policy or file operations into `src/app-view.ts`.
8. Document the log locations and manual sharing steps in user-facing support/release documentation.

## Acceptance Criteria For The Later Implementation

- A packaged app writes `quicktext.log` in Tauri's recommended log directory on Linux, macOS, and Windows.
- Every record is a single line beginning with a valid UTC RFC 3339 millisecond timestamp and includes level, target, and stable event name.
- Recording and provider events for one take share a session identifier; a new process launch has a new run identifier.
- Release logs include useful `INFO`, `WARN`, and `ERROR` events but exclude `DEBUG` and `TRACE` by default.
- Rotation tests show no more than one 5 MiB active file and four retained archives, allowing for the final record to cross the exact size boundary.
- Restarting appends to the active log instead of erasing the failure that preceded the restart.
- Sentinel tests prove that API keys, transcripts, partials, clipboard text, raw audio/frame data, device identifiers, and target window titles never appear in current or rotated logs.
- Untrusted messages containing CR/LF or delimiters cannot forge a second record.
- Simulated unwritable directory, failed rotation, and full-disk behavior do not stop the core capture loop.
- Frontend exceptions reach the same local file without duplicating backend errors.
- **Open logs folder** and **Copy logs path** resolve the documented platform directory.
- No log or diagnostic bundle leaves the device without an explicit user action.

## Consequences

- Support reports gain a durable, timestamped timeline even when QuickText was hidden or restarted.
- The app gains a small amount of disk I/O and up to approximately 25 MiB of bounded local storage.
- Logging policy becomes part of code review: new features must choose safe event fields rather than dumping internal objects.
- Frontend logging requires a narrowly scoped plugin permission; backend logging does not require exposing general filesystem access to the webview.
- Plain line-oriented files remain readable in a text editor and mechanically searchable, without requiring a local log viewer or a hosted service.
- The files are diagnostic records, not authoritative audit evidence; local users can read, edit, or delete them.

## Alternatives Considered

- **Only stdout/stderr:** rejected because tray/background launches and Windows release builds often have no useful attached console, and output disappears after exit.
- **Platform-native stores (`journald`, macOS unified logging, Windows Event Log):** useful for advanced diagnostics but rejected as the only sink because discovery, retention, permissions, and export differ greatly across platforms. A normal file is easier for users to attach.
- **A custom file writer:** rejected because Tauri already provides path resolution, formatting, filtering, and rotation. Custom code adds failure and security surface without a product advantage.
- **One timestamped file per launch:** rejected because it creates unbounded file counts for an autostart, tray-resident app.
- **Unlimited retention:** rejected because a resident app must not consume disk indefinitely.
- **JSON Lines:** viable for centralized ingestion, but not selected for the first local-support slice. Stable line-oriented text is easier for a user to inspect and matches the first-party plugin. If automated ingestion becomes a real requirement, amend the format deliberately.
- **Automatic crash/log upload:** rejected for this slice because dictated-content software should not transmit diagnostics without explicit consent and a separate privacy design.

## References

- [Tauri 2 logging plugin](https://v2.tauri.app/plugin/logging/): first-party file target, platform log directories, filtering, UTC timestamps, permissions, and rotation.
- [`tauri-plugin-log` API](https://docs.rs/tauri-plugin-log/latest/tauri_plugin_log/): Rust builder and rotation behavior.
- [Rust `log` facade](https://docs.rs/log/latest/log/): severity macros, targets, and structured key-value support.
- [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html): event attributes, data exclusion, injection prevention, bounded-resource and failure testing, and retention principles.
- [RFC 3339](https://www.rfc-editor.org/rfc/rfc3339): unambiguous Internet timestamp format.
