# Deepgram Speech to Text Integration Plan

## Purpose

This plan adds Deepgram as a second selectable speech-to-text provider without changing QuickText's capture-first workflow. Soniox remains the default for existing and new installations. Deepgram is an explicit opt-in that uses Nova 3, English transcription, interim results, and keyterm prompting through Deepgram's v1 streaming API.

The implementation must first make the documented provider boundary real. The current Rust shell stores and constructs `SonioxSession` directly, so adding Deepgram is an architectural extraction followed by a new adapter, not a single provider implementation.

## Decisions

- QuickText supports Soniox and Deepgram, with exactly one active provider for each recording session.
- Backend-managed settings persist the active provider. Soniox is the migration and fresh-install default.
- Provider and session behavior is represented by object-safe Rust traits. The app controller depends on those traits rather than concrete Soniox or Deepgram session types.
- Deepgram uses the v1 live transcription endpoint with the Nova 3 model. Flux is outside this feature.
- Deepgram uses English for this release. QuickText exposes no Deepgram language control yet.
- Soniox keeps its current automatic detection and optional language-hint behavior.
- One shared `Terms and phrases` list maps to Soniox context terms and repeated Deepgram `keyterm` query parameters.
- Deepgram legacy weighted `keywords` are not exposed.
- Provider selection and provider-specific settings are disabled during an active recording. Changes apply to the next session.
- QuickText never falls back automatically from one provider to another.
- The inactive provider receives no audio, transcript, credential, or diagnostic payload.
- Deepgram is not hidden behind an experimental feature flag. Selecting it and saving its key is the opt-in boundary.

## Scope

This feature includes:

- A provider-neutral Rust boundary for provider creation, session lifecycle, transcript updates, errors, and results.
- Backend-owned provider selection and session option snapshots.
- Separate operating-system credential-store entries for Soniox and Deepgram.
- A shared terminology preference with provider-specific mapping and validation.
- A Deepgram Nova 3 streaming adapter.
- Session-correlated partial transcript events.
- Provider-aware Settings and a read-only active-provider label in Capture.
- Offline contract, parser, configuration, migration, and frontend tests.
- A secret-gated Deepgram live transcription fixture in CI.

This feature does not include:

- Deepgram Flux or the v2 streaming protocol.
- Deepgram language selection, multilingual mode, or streaming language detection.
- Weighted Deepgram keywords.
- Automatic fallback, retries through another provider, or sending one recording to multiple providers.
- Provider model selection, diarization, translation, or transcript history.
- A generic frontend form renderer for arbitrary provider-defined schemas.

## Current Architecture Gap

The project documents already describe `TranscriptionProvider`, but production code does not implement that abstraction. `src-tauri/src/lib.rs` imports, stores, constructs, and stops `SonioxSession` directly. Shared partial transcript data also lives in `soniox_provider.rs`. Credentials, missing-key messages, Settings controls, language preferences, live fixtures, and several tests use Soniox-specific names.

The extraction must preserve the behavior established by the current implementation:

- Microphone capture starts immediately after an accepted trigger.
- Audio captured during provider connection is queued and flushed in order.
- Readiness failure terminates capture cleanly.
- Partial updates contain only confirmed text and the current revisable hypothesis.
- Stop finalizes the provider session within a bounded timeout.
- Cancel discards the provider session and publishes no transcript.
- UI, shortcut, tray, and IPC triggers share the same backend toggle path.

## Product Behavior

### Provider selection

Settings presents a transcription provider selector. The backend persists `soniox` or `deepgram`; the frontend never owns the authoritative selection. All start paths read the same saved value.

Existing users migrate to `soniox` without changing their keyring entry, language preferences, or capture behavior. A fresh installation also defaults to Soniox. Selecting Deepgram without a saved key is allowed, but capture is blocked with provider-specific setup guidance until the credential exists.

The selector and provider-specific controls are disabled from the accepted start transition through success, cancellation, or failure. A session uses an immutable snapshot of provider, credential reference, terms, language options, audio format, and timeout settings.

### Capture view

Capture shows a compact read-only label such as `Provider: Soniox` or `Provider: Deepgram`. The label is informational; provider changes remain in Settings.

The record and stop loop does not change. Deepgram confirmed text and revisable text use the same transcript surface, display preferences, and copy behavior as Soniox.

### Settings view

Settings contains:

- A provider selector.
- A shared `Terms and phrases` multiline field with one entry per line.
- A Soniox section with credential status and the existing language preference control.
- A Deepgram section with credential status and an English-only limitation note.

The active provider's credential controls and limitations are emphasized. Stored secret values are never returned to the frontend. Saving a credential performs local non-empty and length checks only; authentication occurs when the next provider connection starts.

### Terms and phrases

QuickText stores a trimmed, ordered, de-duplicated list of non-empty terms and phrases as non-secret settings. The list is portable app data, while each adapter owns its wire representation.

- Soniox maps the list to its context or terminology mechanism supported by the selected Soniox model.
- Deepgram repeats the `keyterm` query parameter once for each entry.
- QuickText never appends weights, joins entries with commas or semicolons, or sends the list through Deepgram's legacy `keywords` parameter.

The active provider validates the list when Settings saves it and every provider validates its session snapshot before connecting. For Deepgram, validation enforces no more than 100 entries and no more than 500 provider tokens across the request. If a list valid for Soniox exceeds Deepgram's limits, selecting or starting Deepgram produces an actionable settings error without deleting any entries.

## Provider Capability Mapping

| QuickText concept | Soniox mapping | Deepgram mapping | Product rule |
|---|---|---|---|
| Active provider | Existing Soniox session. | Nova 3 v1 live session. | Exactly one provider per recording. |
| Confirmed text | Final Soniox tokens. | Concatenated `Results` transcripts where `is_final` is true. | Confirmed text is append-only within a session. |
| Revisable text | Non-final Soniox tokens. | Latest `Results` transcript where `is_final` is false. | Replaces the previous hypothesis. |
| Completion | Existing Soniox finalization frame and completion path. | `CloseStream`, remaining `Results`, then `Metadata` or graceful close. | Stop waits within the shared timeout. |
| Pause detection | Provider internal behavior. | `endpointing=false`; `speech_final` does not control QuickText. | Only the explicit QuickText stop ends capture. |
| Terms and phrases | Provider context terms. | Repeated plain `keyterm` parameters. | One portable user list. |
| Language | Automatic detection or saved non-strict hints. | English default. | No Deepgram language control in this release. |
| Credentials | Soniox keyring entry. | Separate Deepgram keyring entry. | Secrets never enter ordinary settings. |
| Audio | Existing supported PCM mapping. | Raw `linear16` or `linear32` with sample rate and channels. | Adapter normalizes unsupported input encodings. |

## Rust Architecture

### Provider-neutral types

Add a provider-neutral transcription module that owns:

- `ProviderId`, initially `Soniox` and `Deepgram`.
- `ProviderCapabilities`, containing stable capability flags and user-facing availability metadata.
- `TranscriptionOptions`, an immutable session snapshot with provider-neutral audio, terms, and timeout values plus validated provider settings.
- `TranscriptUpdate`, containing recording session ID, provider session ID, `final_text`, and `partial_text`.
- `TranscriptResult`, containing final text, duration, provider, and creation time.
- `AppError` and provider-safe error categories.
- Object-safe `TranscriptionProvider` and `TranscriptionSession` traits.

The exact trait signatures may use boxed futures or a narrowly scoped async-trait dependency, but the public contract must support:

```rust
trait TranscriptionProvider {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> ProviderCapabilities;
    fn validate_options(&self, options: &TranscriptionOptions) -> Result<(), AppError>;
    fn start_session(&self, options: TranscriptionOptions) -> ProviderFuture<Box<dyn TranscriptionSession>>;
}

trait TranscriptionSession {
    fn audio_sender(&self) -> AudioSender;
    fn take_ready_receiver(&mut self) -> ReadyReceiver;
    fn take_update_receiver(&mut self) -> UpdateReceiver;
    fn stop(self: Box<Self>) -> ProviderFuture<TranscriptResult>;
    fn cancel(self: Box<Self>);
}
```

The implementation should follow existing ownership semantics rather than forcing these illustrative aliases verbatim. The critical boundary is that `lib.rs` no longer names `SonioxSession` or `DeepgramSession` in shared state or orchestration.

### Provider registry

A backend provider registry resolves a `ProviderId` to its adapter and capability metadata. It is a typed registry, not a generic JSON form schema. The frontend receives stable provider IDs, labels, credential status, and supported settings needed by the explicit Settings layout.

The registry must not instantiate or contact the inactive provider when a recording starts. It may return static capability descriptions without credentials or network access.

### Session correlation

Generate the recording session ID when the start transition is accepted and a provider session ID for the provider connection attempt. Include both IDs in backend transcript updates. The frontend applies an update only when its recording session ID matches the active capture.

This extends the diagnostic correlation model to the live event path and prevents delayed events from a closing session from altering a later recording.

### Credential service

Store credentials under distinct keyring accounts derived from `ProviderId`. Provider-neutral backend commands should accept a validated provider ID and expose only `configured` status, save, and delete operations. They must never return a stored key.

A credential operation may emit typed diagnostics containing provider ID and success or stable failure category. It must never include the credential, authorization header, or raw keyring error text.

### Error model

Provider adapters map protocol and network failures into stable categories:

- `missing_credentials`.
- `invalid_credentials`.
- `invalid_provider_settings`.
- `network_unavailable`.
- `rate_limited`.
- `provider_unavailable`.
- `provider_timeout`.
- `provider_protocol_failure`.
- `empty_audio`.
- `internal_error`.

An error carries provider ID, a bounded safe diagnostic code, error ID, and support reference. User text is generated from the category. Raw Deepgram or Soniox response bodies do not cross the adapter or diagnostics boundary.

## Deepgram Adapter Contract

### Connection configuration

Open `wss://api.deepgram.com/v1/listen` with `Authorization: Token <API_KEY>`. The initial implementation uses:

- `model=nova-3`.
- `interim_results=true`.
- `endpointing=false`.
- Deepgram's default English language behavior, with no `language` or `detect_language` setting exposed by QuickText.
- `encoding`, `sample_rate`, and `channels` matching the normalized raw audio stream.
- One percent-encoded `keyterm` parameter for each validated term or phrase.

Do not send an empty binary frame to complete the session. Some older Deepgram guidance shows that pattern, but the current control-message contract provides `CloseStream` for processing cached audio and closing the v1 stream.

### Audio formats

Deepgram raw streaming requires an explicit encoding and sample rate. Map signed 16-bit little-endian PCM to `linear16` and 32-bit little-endian floating-point PCM to `linear32`. If capture produces unsigned 16-bit samples, convert them at the Deepgram adapter boundary before transmission. Channel count and sample rate must match the recorder snapshot.

The shared recorder remains provider-neutral. Any conversion should be bounded, ordered, tested with sample fixtures, and performed without persisting audio.

### Readiness and buffering

The session returns its audio sender immediately. The recorder starts and queues chunks while the WebSocket handshake proceeds. The adapter reports ready only after the connection is established and configuration is accepted far enough to stream safely.

If readiness fails, shared orchestration stops the recorder, cancels the session, clears partial text, and surfaces a stable provider error. Buffered audio is dropped in memory and never sent to Soniox as a fallback.

### Interim and final results

For each Deepgram `Results` message:

1. Read the top alternative transcript.
2. If `is_final` is true, append the non-empty transcript to confirmed text and clear the current hypothesis for that finalized time span.
3. If `is_final` is false, replace the current hypothesis with that transcript.
4. Emit the full confirmed text plus current hypothesis in a session-correlated `TranscriptUpdate`.

Do not use `speech_final` as a completion signal. It represents pause endpointing and can remain false while Deepgram has already returned finalized transcript segments. Empty result messages must not erase confirmed text.

The adapter must prevent duplicated text when repeated or overlapping results arrive. Parser tests should cover multiple finalized segments, revised hypotheses, empty alternatives, out-of-order or duplicate spans if the protocol exposes timing overlap, and results that arrive after stop but before completion.

### Stop and completion

On explicit stop:

1. Stop microphone capture and close the audio sender so all queued chunks are transmitted in order.
2. Send the JSON control message `{ "type": "CloseStream" }`.
3. Continue reading and accumulating final `Results` messages.
4. Complete successfully after Deepgram sends the summary `Metadata` and the stream closes gracefully.
5. Apply the existing bounded finalization timeout to the entire operation.

The adapter must not rely on `from_finalize=true`, because Deepgram does not guarantee that flag when little or no buffered audio remains. If timeout or protocol failure occurs, return an error even when some finalized text was received. QuickText must not present a possibly truncated transcript as successful.

### Cancel

Cancel drops queued audio, terminates the connection, stops update forwarding, and returns no transcript. It does not send `CloseStream` or wait for provider completion.

### Keepalive

The adapter supports Deepgram's JSON `KeepAlive` control message for any provider-owned period where the connection remains open without media. Normal microphone capture should continuously supply audio, and stop should immediately begin `CloseStream`; keepalive must not become a second completion mechanism.

## Settings and Persistence

Add backend-managed non-secret settings for:

- `active_provider`, defaulting to `soniox`.
- `terms_and_phrases`, defaulting to an empty list.

Keep existing Soniox language preferences unchanged and Soniox-specific. Do not migrate them into Deepgram settings or infer a Deepgram language from them.

Settings writes must be atomic and validated. Unknown provider IDs fail closed. Missing new fields deserialize to their defaults so existing settings files continue to load.

Provider selection can be changed without a credential. The backend checks the selected provider's credential before starting capture and routes missing-key guidance to the matching Settings section.

## Frontend Changes

Maintain the existing frontend boundaries:

- `src/app-view.ts` creates the provider selector, shared terms field, provider credential sections, English-only note, and Capture provider label.
- `src/tauri.ts` contains typed command and event wrappers for provider settings and credentials.
- `src/app-state.ts` owns frontend state for provider metadata, active session correlation, and provider-specific setup errors.
- `src/settings.ts` remains limited to existing frontend-only non-secret display preferences. Backend transcription settings do not move into `localStorage`.
- `src/main.ts` wires events and renders state without containing provider protocol names or parsing rules.

The frontend may name Soniox and Deepgram as user-selectable services. It must not expose wire parameters such as `is_final`, `speech_final`, `CloseStream`, `keyterm`, model IDs, or endpoint URLs.

## Privacy and Diagnostics

Every recording creates one session for the selected provider. Tests must prove that the inactive adapter is never constructed or sent data.

Diagnostics may record:

- Provider ID.
- Recording and provider session IDs.
- Stable lifecycle events and elapsed timings.
- Audio format metadata and chunk counts.
- Terms mode as enabled or disabled, plus entry and token counts.
- Stable error category and safe code.

Diagnostics must not record:

- Credentials or authorization headers.
- Audio or encoded audio.
- Transcript or partial transcript text.
- Terms or phrases.
- Raw provider messages, URLs containing keyterms, or response bodies.
- Soniox language codes selected by the user.

## Migration and Rollout

On first load after upgrade:

- Treat an absent `active_provider` as `soniox`.
- Preserve the existing Soniox credential entry and language settings.
- Initialize the shared terminology list as empty.
- Create no Deepgram credential or settings entry until the user configures it.

No feature flag is needed. The migration retains current behavior, and Deepgram requires explicit selection plus its own credential.

If a release must be rolled back, the new non-secret fields should be ignored by older builds. Separate keyring accounts prevent an older Soniox-only build from reading or deleting the Deepgram credential.

## Implementation Sequence

### Stage 1 Extract the provider boundary

- Move shared transcript updates, results, options, errors, and provider IDs out of `soniox_provider.rs`.
- Add object-safe provider and session traits plus a registry.
- Adapt Soniox to the traits without changing its connection, buffering, partial transcript, finalization, credential, or language behavior.
- Replace concrete `SonioxSession` ownership in shared backend state.
- Add fake-provider orchestration tests and session-correlated transcript events.

Stage 1 is complete only when Soniox behavior is unchanged and all existing checks pass.

### Stage 2 Add provider settings and credentials

- Add backend persistence and migration for active provider and shared terms.
- Generalize secure credential commands and preserve the existing Soniox keyring account.
- Add provider capability and status commands.
- Add the provider selector, conditional provider sections, shared terms field, missing-key routing, and Capture provider label.
- Disable provider-related controls during an active session.
- Add frontend and backend tests for persistence, migration, validation, and active-session snapshots.

Stage 2 is complete only when selecting Deepgram cannot start a session without its key and inactive providers receive no calls.

### Stage 3 Add Deepgram

- Implement the Nova 3 v1 WebSocket adapter.
- Add audio-format normalization, interim parsing, final aggregation, `CloseStream`, completion, cancellation, and timeout behavior.
- Add Deepgram configuration and parser fixtures.
- Add the secret-gated live fixture and CI condition.
- Update user-facing setup and support documentation.

Stage 3 is complete only when the acceptance criteria below pass on Linux, macOS, and Windows packaging targets.

## Test Plan

### Offline Rust tests

- Provider registry resolves known IDs and rejects unknown IDs.
- Existing settings migrate to Soniox without rewriting credentials or language preferences.
- Credential operations use distinct provider accounts and never return secret values.
- The controller starts exactly one selected provider and never invokes the inactive provider.
- Immediate capture queues audio until ready and handles readiness failure.
- Session IDs reject stale partial and completion events.
- Provider switches and option changes affect the next recording only.
- Terms parsing trims, de-duplicates, preserves order, and retains the user's entries after validation errors.
- Deepgram query construction percent-encodes and repeats keyterms without weights or list separators.
- Deepgram rejects more than 100 entries or more than 500 tokens before connecting.
- Signed 16-bit and 32-bit float audio map correctly; unsigned 16-bit conversion preserves ordering and expected amplitude.
- Interim results replace the hypothesis; final results append once; `speech_final` does not complete the recording.
- Stop sends `CloseStream`, consumes trailing results, and completes on metadata plus graceful close.
- Timeout after partial provider output returns an error rather than a successful truncated transcript.
- Cancel publishes no result and clears pending updates.
- Raw provider errors map to stable app categories and safe diagnostic codes.

### Frontend tests

- Existing users see Soniox selected after migration.
- Provider selection and related controls are disabled during capture.
- Selecting a provider shows its credential status and setup guidance.
- Deepgram displays an English-only note and no language control.
- Capture displays the active provider label.
- A mismatched session ID cannot update the transcript.
- Shared terms validation is actionable and does not erase input.
- Missing Deepgram credentials route to the Deepgram Settings section.

### Live fixtures

Generalize the existing live transcription fixture harness while keeping provider-specific environment variables and assertions:

- `SONIOX_API_KEY` gates the Soniox fixture.
- `DEEPGRAM_API_KEY` gates the Deepgram fixture.
- Neither secret is required for normal contributor checks.
- The Deepgram fixture streams a paced known audio file, observes at least one valid update when available, finalizes through `CloseStream`, and compares normalized final text.
- CI output and diagnostics never print either key, the authorization header, raw audio, or provider payloads.

## Acceptance Criteria

- Existing Soniox capture, partial display, finalization, language preferences, and credentials remain intact.
- Every UI, shortcut, tray, IPC, and CLI trigger uses the persisted provider selection.
- Deepgram starts capture immediately and buffers audio during connection.
- Deepgram confirmed and interim text use the shared transcript event contract.
- Explicit stop reliably drains audio and finalizes through `CloseStream`.
- Shared terms map correctly to Soniox and Deepgram.
- Deepgram is English-only and presents no misleading language control.
- Provider or provider-setting changes cannot affect an active recording.
- Credentials remain separate in operating-system secure storage.
- The inactive provider receives no data.
- Provider failures surface stable, actionable errors without raw protocol content.
- Offline tests run without provider credentials, and each provider has a separately secret-gated live fixture.
- `npm run build` passes.
- `npm test` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml` passes.
- `cargo check --manifest-path src-tauri/Cargo.toml` passes.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passes.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Provider extraction changes working Soniox behavior. | Regression for all existing users. | Land extraction first, use contract tests and fake-provider orchestration tests, and preserve Soniox as the default. |
| Deepgram completion is confused with pause endpointing. | Early or truncated transcripts. | Disable endpointing for product completion, accumulate `is_final` segments, and complete only after `CloseStream` response processing. |
| A late event from an old session updates a new capture. | Stale or mixed transcript text. | Carry recording and provider session IDs and reject mismatches. |
| Portable terms exceed one provider's limits. | Capture cannot start after a provider switch. | Preserve the list, validate for the active provider, and show an actionable error without deleting data. |
| Audio sample encoding does not match Deepgram query parameters. | Empty or corrupted transcription. | Normalize at the adapter edge and test byte-level conversions and query construction together. |
| Provider errors or URLs leak user content. | Credential or terminology exposure in diagnostics. | Persist only typed categories, counts, and safe codes; prohibit raw URLs and payloads. |
| English-only behavior is mistaken for automatic detection. | Poor non-English results. | Show an English-only Deepgram note and expose no detection claim or hidden mapping from Soniox preferences. |

## Deferred Decisions

- Deepgram language selection and multilingual mode.
- Streaming language detection if Deepgram adds a compatible contract.
- Flux and its turn-oriented protocol.
- Model selection.
- Runtime keyterm updates.
- Provider-specific advanced controls.
- Commercial credential brokering or temporary provider tokens.
- Automatic or offline fallback providers.

## Deepgram References

- [Live Audio API](https://developers.deepgram.com/reference/speech-to-text/listen-streaming).
- [Models and Languages Overview](https://developers.deepgram.com/docs/models-languages-overview).
- [Interim Results](https://developers.deepgram.com/docs/interim-results).
- [Endpointing and Interim Results](https://developers.deepgram.com/docs/understand-endpointing-interim-results).
- [Keyterm Prompting](https://developers.deepgram.com/docs/keyterm).
- [Keywords and Keyterms](https://developers.deepgram.com/docs/keywords-vs-search).
- [Encoding](https://developers.deepgram.com/docs/encoding).
- [Close Stream](https://developers.deepgram.com/docs/close-stream).
- [Finalize](https://developers.deepgram.com/docs/finalize).
- [Language Detection](https://developers.deepgram.com/docs/language-detection).
