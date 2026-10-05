# Deepgram branch review

Reviewed `codex/deepgram-stt-plan` from committed baseline `8c6da57` against `origin/main`. Fetch and fast-forward pull confirmed the baseline was current. The four pre-existing backend edits were stashed separately and excluded from this review and its fixes.

## Fixed findings

- Provider sessions detached their background task. Queued cancellation could not interrupt a blocked connection, audio drain, or socket write. Sessions now own a task guard that aborts on cancellation, drop, or finalization timeout.
- DNS/TCP/TLS/WebSocket readiness had no bounded deadline. Both adapters now apply a 10-second connection deadline; the existing five-second finalization deadline remains.
- The controller observed readiness failure only. Terminal provider failures after readiness now stop microphone capture, clear pending paste delivery, and surface an error for the matching recording session.
- Deepgram accepted any close code after metadata, and Soniox accepted closure without its completion response. Abnormal or premature closure now fails instead of returning potentially truncated text. Unexpected Soniox completion during recording is also a failure.
- Provider responses and parser errors could expose raw payload values through user messages or terminal output. Errors now use fixed guidance; transport errors distinguish authentication, invalid settings, rate limits, and network failures without copying headers or response bodies.
- Provider setting validation and writes could race capture, while related backend commands relied on frontend disabling. The controller lock now protects provider-related settings changes and rejects them throughout an active session. Duplicate start/stop dispatch also rechecks the state under that lock.
- Provider selection and terms files were overwritten directly. Saves now serialize first, write a separate file, and replace the destination atomically. Provider selection becomes authoritative in memory only after its disk save succeeds.
- Invalid persisted provider selection silently retained the Soniox default. An unreadable or unknown saved selection now blocks capture until the user explicitly saves a provider; missing settings still default to Soniox.
- The frontend displayed Soniox as selected when provider settings failed to load, making an explicit Soniox selection unable to fire a change event. Loading and invalid selections now show an unselected provider prompt. Capture remains disabled until a selection is confirmed, and Stop remains available during an existing recording.
- Provider event subscription wrappers could throw synchronously and abort frontend initialization before settings loaded. They now reject asynchronously through the existing error handlers. The idle provider picker remains available during loading so a pending read cannot lock it.
- Native dropdowns could draw a light system background under white app text. Provider and microphone dropdowns now declare a light surface and dark text explicitly. Author CSS also now respects hidden elements so inactive provider sections stay out of view.
- Missing credentials could route to Settings while leaving the named provider's credential section hidden. Frontend selection now follows the backend's missing-key provider. Provider selection errors appear next to the selector, and completed form saves respect current recording state when re-enabling controls.
- Delayed command replies and backend events could rewind the UI to an older session or an earlier recording phase. Frontend state now rejects stale snapshots while allowing a completed transcript to remain available after a paste failure.
- A new recording reply received before its starting event could retain the previous transcript, draft text, or recording clock. Session changes now reset those values independently of the first observed phase; repeated snapshots within the same session preserve live text and its clock.
- Language picker controls could remain enabled when capture started. Each render now updates their disabled state, closes the picker during capture, and blocks summary activation while controls are unavailable.
- The periodic Deepgram idle check could postpone the first keepalive to the service timeout boundary. Keepalives now use a three-second deadline measured from the latest successful audio or keepalive send.
- README and relevant plans now describe provider choice, current task ownership, buffering, and shared terminology. The terms editor remains exact user input; the provider list removes exact duplicates in first occurrence order.

## Validation

All checks passed on the local Linux host:

- `npm test`: 37 tests passed.
- `npm run build`: TypeScript and production Vite build passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked`: 96 tests passed; two live-provider tests ignored.
- `cargo check --manifest-path src-tauri/Cargo.toml --locked`: passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: passed.
- `git diff --check`: passed.
- Browser preview: reproduced the initialization failure before the fix; verified recovery afterward. With a temporary Tauri test fixture, verified choosing either provider while the initial settings read was pending, ignoring the stale read after selection, section visibility, and readable dropdown foreground/background colors. The fixture was removed after verification.

New offline tests exercise real WebSocket framing over in-memory duplex transports: ordered audio draining, trailing final results, metadata followed by graceful close, abnormal close, missing completion metadata, cancellation without finalization, blocked-drain timeout, premature Soniox close, error-content exclusion, task cancellation, settings replacement, invalid-provider recovery, and frontend credential/session isolation, stale snapshot ordering, and exact idle keepalive timing with audio resetting the deadline.

## Remaining verification

- Neither provider API key was available in the environment, so the secret-gated live Soniox and Deepgram fixtures were not run locally.
- Microphone capture, tray interaction, clipboard/paste delivery, and installed bundles were not exercised during this review. macOS and Windows execution and packaging remain unverified here.
- The local keyterm check counts whitespace-delimited words, not Deepgram's proprietary tokens. Deepgram enforces its exact token limit. A locally accepted list can still produce a settings rejection; it is not silently truncated.
- The existing IPv4 connection behavior is preserved. IPv6-only network support is outside this fix.

## Verdict

No remaining code blocker was found in the reviewed paths after these fixes. Recommend merging after CI is green and a live dictation smoke test with each provider confirms final transcripts. The offline checks establish the lifecycle and parsing behavior, but do not certify real service availability or cross-platform runtime behavior.

## Protocol references

The adapter contract was checked against Deepgram's primary documentation:

- [Live Audio API](https://developers.deepgram.com/reference/speech-to-text/listen-streaming).
- [KeepAlive](https://developers.deepgram.com/docs/audio-keep-alive).
- [Close Stream](https://developers.deepgram.com/docs/close-stream).
- [Encoding](https://developers.deepgram.com/docs/encoding).
- [Keyterm Prompting](https://developers.deepgram.com/docs/keyterm).
