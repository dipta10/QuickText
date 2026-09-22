# Soniox Transcription Terms Implementation Plan

## Summary

Add one persisted multi-line **Terms** field to the Soniox Settings section. Treat each nonblank trimmed line as one provider-neutral transcription term, snapshot the list when recording starts, and map a non-empty list to Soniox `context.terms`.

## Scope

- One Terms textarea in Settings, with one term per line.
- Empty by default and persisted across restarts.
- Exact editor-text round-trip with a 10,000-character maximum.
- Blank lines ignored and surrounding whitespace trimmed only at the session boundary.
- Applied to the next recording session, never an active one.
- No chips, suggestions, sorting, deduplication, description extraction, general context, or translation terms.

## Data Flow

1. Settings loads the saved raw terms text from a backend query.
2. The user edits and saves the textarea.
3. The backend validates the character maximum and persists the exact editor text.
4. When recording starts, the backend parses nonblank trimmed lines and snapshots the resulting ordered list.
5. The list crosses the provider boundary without Soniox field names.
6. The Soniox provider includes `context.terms` only for a non-empty list and combines it with `context.text` when a description also exists.

## Changes

- Add DOM controls, typed Tauri adapters, load/save wiring, and existing-input styling in the frontend.
- Add backend state, persistence, validation, parsing, and read/write commands.
- Snapshot parsed terms in the shared recording-start path.
- Extend the Soniox context serializer with optional `terms` alongside optional `text`.
- Never log raw editor text, parsed terms, counts, lengths, or derived content.

## Verification

- Unit-test exact editor-text storage, empty parsing, blank-line omission, surrounding-whitespace trimming, the character bound, and missing-file defaults.
- Unit-test Soniox context omission when both fields are empty, terms-only serialization, and combined description-plus-terms serialization.
- Build the frontend and run Rust format, check, and test commands.
- Inspect Settings at the compact window size and verify save, clear, and restart persistence behavior in the desktop app.

## Acceptance Criteria

- Settings contains one Terms textarea labeled as one term per line.
- Saved editor text survives restart exactly.
- Each new recording uses the parsed terms captured at its start.
- Empty or whitespace-only editor text sends no terms.
- Terms reach Soniox only as `context.terms` inside the provider implementation.
- No term content or derived data appears in diagnostics or support exports.

## References

- [ADR 0019](adrs/0019-soniox-transcription-terms.md).
- [Description implementation plan](soniox-transcription-description-implementation-plan.md).
- [Soniox context documentation](https://soniox.com/docs/stt/concepts/context).
