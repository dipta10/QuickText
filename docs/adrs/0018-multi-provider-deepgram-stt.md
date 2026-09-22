# ADR 0018 Add Deepgram Through a Provider Neutral Transcription Boundary

## Status

Accepted for the Deepgram speech-to-text feature plan.

## Context

QuickText currently transcribes through Soniox. Existing architecture documents describe a provider-neutral transcription interface, but production Rust code stores and constructs `SonioxSession` directly. Credentials, language preferences, Settings controls, error strings, tests, and live fixtures also assume Soniox is the only provider.

Deepgram Nova 3 provides real-time speech-to-text, interim results, and keyterm prompting that can support the same capture loop. Its protocol is not identical to Soniox. Deepgram distinguishes finalized transcript segments (`is_final`) from pause endpointing (`speech_final`), completes a one-shot stream through `CloseStream`, accepts plain repeated keyterms rather than weighted keywords, and cannot preserve Soniox's arbitrary list of non-strict streaming language hints.

Adding Deepgram directly to shared shell code would duplicate capture orchestration and make provider behavior conditional throughout the application. Reducing both services to a false lowest common denominator would hide meaningful validation and finalization differences.

## Decision

Add Deepgram as a second selectable speech-to-text provider after extracting a real provider-neutral Rust boundary.

- Define object-safe `TranscriptionProvider` and `TranscriptionSession` traits for validation, session start, readiness, audio delivery, provider-neutral transcript updates, stop, cancel, results, and application errors.
- Persist one active `ProviderId` in backend-owned settings. Soniox remains the default for existing and fresh installations.
- Resolve the selected adapter through a typed backend registry. Start exactly one provider session for each recording; never send data to the inactive provider.
- Store Soniox and Deepgram credentials under separate operating-system credential-store accounts. The frontend receives configuration status, never secret values.
- Snapshot provider selection and applicable settings at recording start. Provider changes apply to the next session and controls are disabled while a recording is active.
- Keep explicit provider sections in the compact Settings UI. Use typed capability metadata, but do not build the generic form renderer proposed and later superseded by ADR 0011.
- Add one shared `Terms and phrases` list. Soniox maps it to context terms and Deepgram Nova 3 maps it to repeated `keyterm` parameters. Do not expose Deepgram legacy weighted `keywords`.
- Keep Soniox language preferences unchanged. Deepgram uses its default English behavior and exposes no language control in this release.
- Use Deepgram Nova 3 through `wss://api.deepgram.com/v1/listen` with interim results enabled and pause endpointing disabled for product completion.
- Accumulate Deepgram `is_final` segments as confirmed text and expose the latest non-final segment as the revisable hypothesis. Do not treat `speech_final` as session completion.
- On stop, send `CloseStream`, collect remaining results, and complete after summary metadata and graceful closure within the shared timeout. Cancel closes and discards without publishing a transcript.
- Add recording and provider session IDs to live transcript updates so delayed events cannot modify another capture.
- Map provider failures to stable application categories and safe diagnostic codes. Never persist credentials, audio, transcripts, terms, raw URLs, or provider payloads.
- Do not automatically fall back between providers and do not gate Deepgram behind a feature flag. Explicit provider selection and a configured credential are the opt-in boundary.

## Consequences

- Shared capture orchestration no longer depends on `SonioxSession`, and tests can inject fake providers.
- The Soniox adapter must be migrated to the new traits without changing current behavior before Deepgram work begins.
- Backend settings gain an active provider and shared terminology list, while existing Soniox language preferences remain provider-specific.
- Frontend work adds a provider selector, conditional credential sections, terms input, active-provider label, and stale-session event rejection.
- Deepgram can reuse the existing live transcript experience, but it needs its own audio encoding, parsing, finalization, credential, and live-fixture tests.
- English-only Deepgram support is an explicit first-release limitation. Language and multilingual controls require a later decision.
- A terminology list valid for one provider may exceed another provider's limits. QuickText preserves the list and blocks the incompatible provider session with an actionable validation error.
- Existing users remain on Soniox and keep their credential and language preferences after migration.

## Rejected Alternatives

- **Replace Soniox with Deepgram.** Rejected because the feature is provider choice, and replacement would disrupt working users without an architectural benefit.
- **Add Deepgram conditionals directly to `lib.rs`.** Rejected because it duplicates orchestration and leaves the documented provider boundary unrealized.
- **Use an enum facade without provider and session traits.** Rejected because object-safe traits support contract tests, fake providers, and future adapters while keeping the controller independent of concrete session types.
- **Build a fully generic provider-defined settings form.** Rejected for this feature because two explicit compact sections are clearer and avoid a broad frontend form framework.
- **Expose both Deepgram keyterms and weighted keywords.** Rejected because Nova 3 uses keyterms and Deepgram recommends keyterm prompting instead of legacy keywords.
- **Map Soniox language hints into Deepgram silently.** Rejected because the providers' streaming language contracts are incompatible and hidden mapping would misrepresent behavior.
- **Use pause endpointing to stop a recording.** Rejected because QuickText already has an explicit stop action and `speech_final` does not mean all transcript segments are complete.
- **Return accumulated text after finalization timeout.** Rejected because the result may omit the end of the recording while appearing successful.
- **Automatically retry through the other provider.** Rejected because it could send audio to a service the user did not select and conceal credential, availability, or billing failures.

## Implementation Order

1. Extract provider-neutral types and traits, adapt Soniox, and add fake-provider orchestration tests.
2. Add backend provider selection, credential indexing, shared terms, migration, and provider-aware UI.
3. Add the Deepgram adapter, offline contract tests, a secret-gated live fixture, and CI coverage.

Each stage must preserve a working Soniox path and pass the standard frontend and Rust checks before the next stage begins.

## References

- [Deepgram integration plan](../deepgram-stt-integration-plan.md).
- [ADR 0002](0002-streaming-soniox-stt.md).
- [ADR 0003](0003-backend-first-soniox-integration.md).
- [ADR 0006](0006-streaming-partial-transcripts.md).
- [ADR 0007](0007-immediate-capture-with-provider-buffering.md).
- [ADR 0015](0015-local-support-diagnostics.md).
- [ADR 0017](0017-language-preferences.md).
- [Deepgram Live Audio API](https://developers.deepgram.com/reference/speech-to-text/listen-streaming).
- [Deepgram Interim Results](https://developers.deepgram.com/docs/interim-results).
- [Deepgram Keyterm Prompting](https://developers.deepgram.com/docs/keyterm).
- [Deepgram Close Stream](https://developers.deepgram.com/docs/close-stream).
