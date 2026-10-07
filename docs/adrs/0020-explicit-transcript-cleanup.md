# ADR 0020: Keep Transcript Cleanup Explicit And Preserve The Original

## Status

Accepted and implemented on the cleanup branch, 2026-10-08. The [implementation plan](../transcript-cleanup-implementation-plan.md) records the selected model, Settings behavior, recovery defaults, and remaining live-validation work.

## Decision

QuickText will offer **Clean & Copy** beside Copy as an explicit rewrite of finalized dictation, rather than changing speech recognition or automatically rewriting every recording. The user chose concise rewriting, a visible cleaned result, an available **Restore original** action, and their own securely stored API key. Cleanup copies once and never automatically pastes.

Use Google Gemini, with the user obtaining their key through Google AI Studio and entering it in a separate Settings section similar to the existing provider credential sections. Clean & Copy requires a saved cleanup key; typing an unsaved key does not enable the action. A missing key leaves ordinary dictation and Copy available. Support free-tier API accounts without requiring billing setup.

Keep the original transcript distinct from its cleaned version: an LLM can change meaning even when instructed to preserve it, so replacing the only retained result would remove the user's way to inspect or recover their words. This boundary also keeps existing transcription and paste-to-target behavior from silently acquiring an extra text-processing dependency or a second delivery action.

## Consequences

- Cleanup is optional and separate from speech recognition; missing cleanup credentials do not block ordinary dictation or Copy.
- Original and cleaned transcripts are distinct concepts for the latest dictation, not a transcript-history feature.
- Cleanup means editing wording while preserving distinct ideas; summarization, translation, and answering transcript-contained questions are outside this feature.
- Google is selected to reduce unfamiliar-provider setup friction. The implementation uses Gemini 3.1 Flash-Lite because the initially selected 2.5 Flash-Lite has restricted access. A narrow `TranscriptCleaner` interface keeps text rewriting separate from speech recognition and permits a future adapter without introducing a provider picker now.

## References

- [Cleanup vocabulary](../../CONTEXT.md).
- [Project glossary](../glossary.md).
- [Capture and Settings boundary](0004-capture-settings-ui.md).
- [Paste-to-target behavior](0012-paste-to-target-dictation.md).
