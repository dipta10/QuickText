# ADR 0002: Stream Audio To Soniox Real-Time STT

## Status

Accepted for MVP unless the audio spike exposes a blocking issue.

## Context

The product needs to feel snappy. If the app records locally and uploads only after stop, users wait for the full provider round trip after speaking. Soniox provides a real-time STT WebSocket API that accepts binary audio frames and finalizes the stream when the client sends an empty frame.

Current Soniox documentation identifies the real-time STT endpoint as:

```text
wss://stt-rt.soniox.com/transcribe-websocket
```

It also documents `stt-rt-v5` as the real-time model example.

## Decision

Use Soniox real-time STT WebSocket streaming.

The app will start a Soniox transcription session when recording starts, stream microphone audio as it is captured, and end the stream when the user stops recording.

## Consequences

- Stop can finalize an existing provider session instead of beginning transcription from zero.
- The backend must handle WebSocket lifecycle, binary frames, retries, and finalization.
- The frontend can later show partial transcripts without changing the provider shape.
- MVP does not require partial transcript UI.
- Tests need a fake provider implementation so UI and state-machine tests do not require network access.

## References

- Soniox STT WebSocket API: https://soniox.com/docs/api-reference/stt/websocket-api
