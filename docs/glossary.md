# QuickText Glossary

Terms used across the product and ADR docs. Add entries here when an ADR introduces new domain vocabulary.

## Terms

- **Capture**: The user-facing activity of recording speech and producing a transcript; the primary app loop.
- **Recording session**: One start-to-stop dictation cycle. Starts when the trigger is accepted and capture goes live; ends at stop, failure, or the max-duration cap.
- **Notification preference**: An individual opt-in checkbox controlling one notification type (started, complete, error). Defaults to off.
- **Lifecycle notification**: An OS desktop notification tied to a recording-session event: started, transcription complete, or error.
- **Suppress (notification)**: Skip firing an OS notification because equivalent feedback is already visible in the focused main window.
- **Provider setting descriptor**: A backend-declared description of one provider-specific setting (id, type, range or choices, default, label). The Settings UI renders descriptors generically and never learns provider specifics.
- **Advanced transcription settings**: The collapsed Settings section that renders the active provider's setting descriptors; expands on click and includes a reset-to-defaults action.
- **Language hints**: Provider setting biasing recognition toward expected languages. Empty selection means none are sent and the provider auto-detects.
- **Endpoint delay**: How long the provider waits for silence before finalizing transcript tokens (Soniox `max_endpoint_delay_ms`, 500–3000 ms); controls how quickly final text appears after speech stops.
- **Session-applied settings**: Transcription settings take effect when the next recording session starts; an in-flight recording keeps its prior values.
- **Diagnostic log**: A bounded local timeline of low-volume lifecycle, warning, and error events used to investigate app failures; it excludes dictated content, audio, credentials, clipboard content, raw provider traffic, and high-frequency event noise.
- **Run ID**: An opaque identifier created for one QuickText process launch so events from the same run can be correlated without identifying the user or device.
- **Log rotation**: Replacing a full active log with a timestamped archive while enforcing a fixed archive count so logging cannot grow without bound.
- **Paste-to-target**: Opt-in behavior where a finalized transcript is written to the clipboard and pasted into the application focused at trigger time.
- **Headless take**: A recording started by shortcut or CLI/IPC while paste-to-target is enabled; the main window is never shown or focused, and finalize pastes into the target app.
- **Paste target**: The application focused when a headless take starts. If QuickText itself is focused at trigger time, the take is not headless and no paste occurs.
