# ADR 0007: Capture Immediately And Buffer Audio Until The Provider Connects

## Status

Accepted.

## Context

The start path was strictly sequential: read the API key, probe the microphone, await the Soniox WebSocket connection and configuration, and only then start microphone capture (see [ADR 0002](0002-streaming-soniox-stt.md)). The provider handshake takes hundreds of milliseconds to seconds, and the keyring read adds more.

Everything a user says between pressing the trigger and the connection completing was never captured at all, because capture had not started yet. For short dictation this is the worst possible loss window: it is exactly when users start speaking.

Audio already flows from capture to the provider through an unbounded MPSC channel, so a consumer that attaches late is effectively a buffer for free.

## Decision

Start microphone capture as soon as the trigger is validated and the API key is available; connect to Soniox concurrently in a background task.

Specifics:

- `SonioxSession::start` no longer awaits the network. It creates the audio/command channels immediately, spawns a task that connects and sends the config frame, and returns right away.
- Chunks captured before the connection completes sit in the unbounded audio channel and are flushed to Soniox in order once connected. No separate buffer structure exists.
- `recording` status now means "microphone is live", even while the provider is still connecting.
- On stop during the connecting phase, the session flushes all buffered audio before sending the finalize frames. This requires capture to be stopped first so the buffered flush can observe channel closure; all stop paths already follow that order.
- If the connection fails while still recording, a readiness watcher transitions the app from `recording` to `error` via a new `fail_recording` controller transition: it stops capture, cancels the session, and surfaces a provider-unavailable error.
- If the user stops before the connection outcome is known, a failed connection responds to the queued finalize request with an error instead of leaving stop waiting for a timeout.
- Missing API key and microphone failures still fail in `starting` before anything is live.

## Consequences

- No speech is lost between trigger and "listening"; perceived start latency drops to keyring plus device-open time.
- Provider connection failures now surface after recording has begun, which required the new `fail_recording` transition and watcher; the old clean pre-recording failure path no longer covers this case.
- An offline or stalled connection buffers audio indefinitely until the user stops or the max-duration auto-stop fires; acceptable because recordings are short and bounded by the 5-minute cap.
- Partial transcripts begin only after the connection completes; the UI may briefly show `recording` without partial text.
- Tests must cover the `fail_recording` transition, the queued-finalize error response on connect failure, and flush-before-finalize ordering.

## Grilled Decisions

- **Do we wait for the connection before showing `recording`?** No. The mic is genuinely live; hiding that would mislead users into waiting to speak.
- **Do we add an explicit ring buffer?** No. The unbounded channel already preserves order and absorbs the pre-connect window.
- **What happens if the user stops while still connecting?** Buffered audio flushes, then finalization proceeds; a failed connection answers the queued finalize with an error rather than a timeout.
- **Does the frontend need changes?** No. It renders backend state events; the meaning of `recording` simply becomes more truthful.

## References

- [ADR 0002](0002-streaming-soniox-stt.md): streaming audio to Soniox real-time STT.
- [ADR 0003](0003-backend-first-soniox-integration.md): backend-owned app controller and state machine.
