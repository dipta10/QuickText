# ADR 0020: Keep Transcript Cleanup Explicit And Preserve The Original

## Status

Accepted product boundary; not implemented. Provider availability and recovery defaults remain unresolved in the [implementation plan](../transcript-cleanup-implementation-plan.md).

## Decision

QuickText will offer **Clean & Copy** beside Copy as an explicit rewrite of finalized dictation, rather than changing speech recognition or automatically rewriting every recording. The user chose concise rewriting, a visible cleaned result, an available **Restore original** action, and their own securely stored API key. Cleanup copies once and never automatically pastes.

Keep the original transcript distinct from its cleaned version: an LLM can change meaning even when instructed to preserve it, so replacing the only retained result would remove the user's way to inspect or recover their words. This boundary also keeps existing transcription and paste-to-target behavior from silently acquiring an extra text-processing dependency or a second delivery action.

## Consequences

- Cleanup is optional and separate from speech recognition; missing cleanup credentials do not block ordinary dictation or Copy.
- Original and cleaned transcripts are distinct concepts for the latest dictation, not a transcript-history feature.
- Cleanup means editing wording while preserving distinct ideas; summarization, translation, and answering transcript-contained questions are outside this feature.
- The selected Gemini 2.5 Flash-Lite model needs an access decision before implementation. This ADR does not approve a provider substitution or the proposed recovery defaults.

## References

- [Cleanup vocabulary](../../CONTEXT.md).
- [Project glossary](../glossary.md).
- [Capture and Settings boundary](0004-capture-settings-ui.md).
- [Paste-to-target behavior](0012-paste-to-target-dictation.md).
