# ADR 0006: Streaming Partial Transcripts

## Status

Accepted.

## Context

The backend already receives every Soniox token response while recording, but discards all non-final tokens and emits nothing until stop finalization completes (see [ADR 0002](0002-streaming-soniox-stt.md)). The user sees no text until they stop speaking, even though confirmed words are available seconds earlier.

Soniox responses are incremental: each message carries new tokens, where non-final tokens (`is_final: false`) form a revisable hypothesis for un-finalized audio, and final tokens confirm text permanently. Stream-boundary markers (`<end>`, `<fin>`) can appear in either kind and must continue to be stripped.

Today the app has exactly one frontend event, `app-state-changed`, carrying the full `AppSnapshot` and emitted only on state transitions. Token-level updates would make that event high-frequency and force throttling onto status transitions.

## Decision

Show live results during recording as **confirmed finals rendered normally plus the current hypothesis rendered dimmed**, displayed as `final_text + partial_text`.

Specifics:

- The provider session exposes a channel of partial updates, mirroring the existing `audio_sender` pattern. The app controller runs a forwarder task that maps channel messages to a new dedicated Tauri event.
- The event payload carries two provider-agnostic fields: `{ final_text, partial_text }`. The UI composes the display; Soniox protocol details stay behind the provider boundary.
- Partials are emitted per provider message with **no throttling**. Desktop message rate is bounded by speech; if profiling later shows IPC or DOM pressure during fast speech, revisit this before adding speculative buffering.
- On stop, the partial is hidden immediately while status shows finalization; the arriving final transcript replaces everything. This avoids showing a stale hypothesis next to its own final version.
- On error or cancel, the partial is always cleared. Partials are ephemeral UI sugar; only the final transcript or an error state survives abnormal ends.
- Copy and auto-copy behavior stay tied to the final transcript and do not change.

## Consequences

- The provider boundary gains exactly one narrow concept: a stream of `(final_text, partial_text)` updates. UI code stays provider-agnostic.
- `app-state-changed` keeps its coarse meaning; partial traffic never delays status events.
- The frontend gains a second event subscription and a derived display string; transcript rendering remains selectable output text, not an editable input.
- No-throttle emission is the recorded trade-off; it must be revisited only on evidence, not preemptively.
- Tests need coverage for hypothesis composition (marker stripping in partials) and for partial clearing across stop/error/cancel paths.

## References

- [ADR 0002](0002-streaming-soniox-stt.md): streaming audio to Soniox real-time STT.
- [ADR 0003](0003-backend-first-soniox-integration.md): backend-first integration; deferred the partial-UI question answered here.
- Soniox STT WebSocket API: https://soniox.com/docs/api-reference/stt/websocket-api
