# QuickText Glossary

Terms used across the product and ADR docs. Add entries here when an ADR introduces new domain vocabulary.

## Terms

- **Capture**: The user-facing activity of recording speech and producing a transcript; the primary app loop.
- **Recording session**: One start-to-stop dictation cycle. Starts when the trigger is accepted and capture goes live; ends at stop, failure, or the max-duration cap.
- **Notification preference**: An individual opt-in checkbox controlling one notification type (started, complete, error). Defaults to off.
- **Lifecycle notification**: An OS desktop notification tied to a recording-session event: started, transcription complete, or error.
- **Suppress (notification)**: Skip firing an OS notification because equivalent feedback is already visible in the focused main window.
