# ADR 0018: Supply A Transcription Description As Soniox Context

## Status

Accepted for documentation; implementation is planned separately.

## Context

Short dictation can contain project names, domain language, or background details that are difficult to recognize without context. Soniox accepts optional context with each real-time WebSocket session. Its `context.text` field is intended for longer free-form background text.

QuickText currently has no way to collect that background text. The feature should remain deliberately small: one user-authored description, saved in Settings and supplied to each new Soniox session. It must not grow into a vocabulary editor, structured context builder, or generic provider-settings system.

ADR 0011 deferred custom context from the provider-declared advanced-settings slice because free-form context did not fit the tunable descriptor controls. This decision adds one fixed provider-neutral session option instead. The frontend names the user concept but never constructs Soniox protocol data.

## Decision

Add one multi-line **Description** field to the existing Soniox section in Settings.

- The description defaults to the zero-length string and persists as ordinary non-secret backend settings data.
- The field accepts at most 10,000 characters. The frontend enforces the limit and the backend validates it before saving.
- Saving preserves the exact string, including line breaks and surrounding whitespace. QuickText does not trim, parse, summarize, generate, or otherwise transform it.
- A recording snapshots the saved description when its start transition is accepted. Editing Settings never changes an in-flight session.
- The app controller passes the snapshot through the provider boundary as an app-level transcription description.
- The Soniox provider serializes a non-empty description as `context: { "text": description }` in the initial WebSocket configuration.
- When the description has zero length, the Soniox provider omits `context`. A whitespace-only value is non-empty and is sent unchanged.
- No other context controls, defaults, presets, counters, term extraction, or structured fields are included.

Soniox limits total context to 8,000 tokens, approximately 10,000 characters. The character limit is a simple UI and persistence bound, not a local token estimator. If Soniox rejects an unusual value within that bound because it exceeds the token limit, QuickText uses its existing provider-error path.

The description is user-authored content. Diagnostics may record whether the save or session setup operation succeeded, but must never record the value, its length, excerpts, hashes, tokens, or other derived content. Diagnostics exports continue to exclude settings files.

## Consequences

- Users can give every new dictation session stable background context without re-entering it.
- The settings store gains one string and two backend commands or equivalent read/write operations.
- The provider-neutral session options gain one description string; Soniox-specific JSON remains inside the Soniox provider.
- Empty behavior is deterministic and does not rely on undocumented provider handling of an empty `context.text` value.
- The 10,000-character bound keeps storage and request size predictable but cannot perfectly mirror the provider's token-based limit.
- Tests must cover default empty state, exact persistence, length rejection, session snapshot behavior, empty omission, and exact non-empty Soniox serialization.

## Rejected Alternatives

- **Use `context.general`:** Rejected because the user supplies free-form background text, which Soniox models as `context.text`; structured key-value entry would add UI and parsing.
- **Add the field to the provider-declared advanced schema:** Rejected because one fixed textarea does not justify extending the descriptor system with free-form content controls.
- **Send an empty string to Soniox:** Rejected because `context` is optional and omission expresses the absence of context without depending on empty-string acceptance.
- **Trim or normalize before saving:** Rejected because the field is defined as exact user-authored input.
- **Estimate tokens locally:** Rejected because it adds tokenizer coupling and complexity beyond this feature; the existing provider-error path handles the rare mismatch.

## Grilled Decisions

- **Documentation or implementation now?** Documentation only.
- **Single-line or multi-line?** One plain multi-line Description field.
- **Persist across restarts?** Yes, as ordinary non-secret backend settings data.
- **Empty behavior?** Keep the saved value empty and omit Soniox `context`.
- **Maximum length?** 10,000 characters, enforced by both frontend and backend.
- **Any other behavior?** No parsing, trimming, defaults, generation, or additional context UI.

## References

- [ADR 0003](0003-backend-first-soniox-integration.md): backend-owned app controller and provider boundary.
- [ADR 0004](0004-capture-settings-ui.md): Capture and Settings separation.
- [ADR 0011](0011-provider-declared-settings.md): provider-declared advanced transcription settings.
- [Soniox context documentation](https://soniox.com/docs/stt/concepts/context).
- [Soniox WebSocket API](https://soniox.com/docs/api-reference/stt/websocket-api).
