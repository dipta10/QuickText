# QuickText Glossary

Terms used across the product and ADR docs. Add entries here when an ADR introduces new domain vocabulary.

## Terms

- **Capture**: The user-facing activity of recording speech and producing a transcript; the primary app loop.
- **Recording session**: One accepted dictation attempt. Starts at the backend transition into `starting`, before microphone/provider setup, and ends after finalization, cancellation, or failure reaches a terminal state.
- **Notification preference**: An individual opt-in checkbox controlling one notification type (started, complete, error). Defaults to off.
- **Lifecycle notification**: An OS desktop notification tied to a recording-session event: started, transcription complete, or error.
- **Suppress (notification)**: Skip firing an OS notification because equivalent feedback is already visible in the focused main window.
- **Provider setting descriptor**: A backend-declared description of one provider-specific setting (id, type, range or choices, default, label). The Settings UI renders descriptors generically and never learns provider specifics.
- **Advanced transcription settings**: The collapsed Settings section that renders the active provider's setting descriptors; expands on click and includes a reset-to-defaults action.
- **Language hints**: Provider setting biasing recognition toward expected languages. Empty selection means none are sent and the provider auto-detects.
- **Endpoint delay**: How long the provider waits for silence before finalizing transcript tokens (Soniox `max_endpoint_delay_ms`, 500–3000 ms); controls how quickly final text appears after speech stops.
- **Session-applied settings**: Transcription settings take effect when the next recording session starts; an in-flight recording keeps its prior values.
- **Paste-to-target**: Opt-in behavior where a finalized transcript is written to the clipboard and pasted into the application or field focused when transcription finishes.
- **Headless take**: A recording started by plain CLI/IPC while paste-to-target is enabled; the main window is never shown or focused, and finalize pastes into the target app.
- **Focused paste take**: A recording started by the global shortcut with focus-on-start enabled, or by `quicktext toggle focus`, while paste-to-target is enabled; QuickText shows while recording, then hides after finalization and pastes once the operating system transfers focus.
- **Paste target**: The application or field focused when delivery occurs. QuickText never captures or restores the application focused when recording began. If QuickText itself is focused at trigger time, the take is not paste-enabled.
- **App run**: One lifetime of the resident QuickText process, from process start until exit; all events in that lifetime share one UUID v4 `run_id`.
- **Recording session ID**: UUID v4 correlating one accepted dictation take from the `starting` transition through setup, capture, finalization, cancellation, or failure.
- **Provider session ID**: UUID v4 correlating one provider connection attempt; a retry gets a new provider-session ID while retaining its recording-session ID.
- **Support reference**: Compact `QT-` reference shown with a user-visible error and mapped to a full `error_id` in exported diagnostics.
- **Diagnostics bundle**: User-confirmed ZIP export containing bounded metadata-only logs and a safe technical manifest; QuickText does not upload it.
- **Temporary debug logging**: Non-persistent diagnostics mode that records finer technical timing and lifecycle metadata for at most 30 minutes or until process exit without recording speech content or secrets.
- **Application version**: QuickText product version declared by the packaged application, such as `0.1.0`. Multiple distributed releases may share one application version.
- **Build ID**: Exact release tag embedded in every distributed artifact attached to one QuickText release. Local or manual builds have no release Build ID and display `Development`; the Build ID remains distinct from `source_revision` because the same commit can be built more than once.
- **Source revision**: Exact commit hash used to build a QuickText binary. Settings displays a shortened form while the backend retains the full value; local or manual builds without a supplied revision display `Development`.
- **Development build**: Local or manually compiled QuickText binary created without a release Build ID. Its user-facing Build value is `Development` and does not claim to identify a reproducible distributed release.
