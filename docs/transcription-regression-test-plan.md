# Transcription Regression Test Plan

Feed pre-recorded audio files through the real transcription pipeline (Soniox streaming path) and compare the resulting transcript against an expected reference. This validates the provider integration end-to-end without needing a live microphone during tests.

## Status

Planning. Recordings have not been captured yet.

## Goal

Catch transcription regressions — wrong audio format, broken framing, marker stripping, finalization bugs — by replaying known recordings through `SonioxSession` and asserting on the output.

## Test Approach

1. Capture 2–3 short recordings (10–30 seconds each) of clear speech.
2. Store each recording as a fixture alongside its expected transcript.
3. An integration test starts a `SonioxSession`, streams the fixture bytes as audio chunks, finalizes, and compares the result to the expected transcript.
4. Comparison is done on normalized text, not raw strings (see Comparison Rules).

This is an integration test, not a unit test: it requires network access and a valid Soniox API key. It must be excluded from the default test run.

## Fixture Format

- Recordings must be convertible to the PCM format the pipeline sends (`AudioEncoding::PcmF32LE` or `PcmS16LE` at the device sample rate).
- Store fixtures as standard `.wav` files for easy inspection; the test decodes them to raw PCM chunks before feeding the session.
- Each fixture gets a sibling text file with the expected transcript, e.g. `fixtures/sample-01.wav` and `fixtures/sample-01.expected.txt`.
- A small metadata note (speaker, duration, recording conditions) helps interpret failures later.

### Fixture Location

Fixtures should live in `src-tauri/fixtures/` if they contain no personal data and stay small (< 1 MB total). If recordings contain personal speech content, keep them out of git in a local untracked directory instead, and document the expected layout.

## Comparison Rules

STT output is never byte-stable across runs and model versions, so exact equality will produce false failures. Compare after normalization:

- Lowercase everything.
- Collapse whitespace.
- Strip punctuation.
- Strip stream markers (`<end>`, `<fin>`) — already handled by the pipeline, but assert none survive.

Two pass levels:

- **Strict:** normalized text matches exactly. This is the target for clear, short recordings.
- **Tolerant:** word error rate (WER) below a threshold (start with ≤ 1 word difference). Useful when Soniox model updates change wording slightly.

Start with strict matching; relax per-fixture only when a mismatch is reviewed and accepted.

## Running The Tests

- Gate on the presence of `SONIOX_API_KEY` (or a dedicated `QUICKTEXT_TEST_API_KEY`) in the environment.
- Mark tests `#[ignore]` by default; run explicitly with `cargo test -- --ignored`.
- Never commit the API key; never log it.
- Document the invocation in this file once implemented:

```bash
SONIOX_API_KEY=... cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
```

## Open Questions

- Do the recordings contain personal information? Decides whether fixtures are committed or kept local.
- Which sample rate should fixtures use — capture from the dev machine's default input config, or fix a canonical rate like 16 kHz mono? A fixed canonical rate makes fixtures portable across machines; requires the test to declare its own `AudioFormat` instead of reusing `input_format()`.

## Resolved Decisions

- Chunk pacing must mimic real-time streaming. The session's finalize path drains and discards buffered audio (`soniox_provider.rs`), so blasting all chunks before `stop()` loses audio. The test sleeps for each chunk's playback duration between sends.
- The first fixture was converted from an MP3 recording with: `ffmpeg -i input.mp3 -acodec pcm_s16le sample-01.wav` (48 kHz mono 16-bit).

## Recording Checklist (When Ready)

- Quiet room, single speaker, English.
- One short phrase fixture (~5 s), one sentence fixture (~15 s), one multi-sentence fixture (~30 s).
- Save as WAV, note sample rate and channel count in the metadata.
- Write the expected transcript by hand from listening, not from copying Soniox output, so the test does not just encode current model behavior.

## Deliverables

- Fixture directory with recordings and expected transcripts.
- A Rust integration test (e.g. `src-tauri/tests/transcription_fixture.rs`) using the public `SonioxSession` interface.
- Normalization helper shared between fixtures, ideally pure enough to unit test on its own.
- This document updated with the final invocation command and any decisions made on the open questions.
