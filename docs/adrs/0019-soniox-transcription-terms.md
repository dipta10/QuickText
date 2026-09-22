# ADR 0019: Supply Explicit Transcription Terms As Soniox Context

## Status

Accepted.

## Context

ADR 0018 added free-form background text through Soniox `context.text`. Soniox separately supports `context.terms` for names, brands, technical vocabulary, and other words whose spelling matters. Putting those words only in a description is less direct and makes a simple vocabulary list awkward to maintain.

QuickText needs one small Terms input in Settings. This does not require a structured context builder, automatic term extraction, or changes to the provider-declared settings schema.

## Decision

Add one multi-line **Terms** field to the Soniox section in Settings.

- The editor accepts one term per line and persists its exact text as ordinary non-secret backend settings data.
- At recording start, QuickText snapshots the saved value, trims surrounding whitespace from each line, ignores blank lines, and preserves the order and duplicates of the remaining terms.
- The editor accepts at most 10,000 characters. The frontend enforces the limit and the backend validates it before saving.
- The provider boundary receives the parsed list. The Soniox provider serializes a non-empty list as `context.terms`.
- When both the parsed terms list and description are empty, the Soniox provider omits `context`. If either has content, it emits one `context` object containing the applicable field or fields.
- Editing or saving terms never changes an in-flight session.
- QuickText does not generate, suggest, deduplicate, sort, or extract terms.

Soniox applies one total context limit of 8,000 tokens, approximately 10,000 characters, across description and terms. QuickText keeps each editor independently bounded and uses the existing provider-error path if their combined payload exceeds Soniox's limit. It does not add a tokenizer or shared live counter.

Terms are user-authored content. Diagnostics must never record the editor text, parsed terms, counts, lengths, excerpts, hashes, or other derived content. Diagnostics exports continue to exclude settings files.

## Consequences

- Users can explicitly provide uncommon spellings without embedding them in prose.
- The settings store gains one raw editor string and two backend read/write commands.
- Tests cover empty parsing, whitespace handling, persistence bounds, session snapshot behavior, and Soniox serialization with terms alone or alongside a description.
- The UI remains a pair of plain textareas instead of becoming a provider-specific context editor.

## Rejected Alternatives

- **Comma-separated input:** Rejected because terms may contain commas and one term per line is easier to scan and edit.
- **Automatic extraction from Description:** Rejected because extraction is ambiguous and would transform user-authored content unexpectedly.
- **Trim the persisted editor value:** Rejected because preserving the editor text avoids surprising changes when Settings is reopened; trimming happens only when creating the provider term list.
- **Deduplicate or sort:** Rejected because the app does not need extra transformations to pass an explicit list to Soniox.
- **Add `context.general` and translation terms now:** Rejected because the request is specifically for transcription terms.

## References

- [ADR 0018](0018-soniox-transcription-description.md): free-form transcription description.
- [Soniox context documentation](https://soniox.com/docs/stt/concepts/context).
- [Soniox WebSocket API](https://soniox.com/docs/api-reference/stt/websocket-api).
