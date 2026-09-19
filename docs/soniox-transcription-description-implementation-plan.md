# Soniox Transcription Description Implementation Plan

## Summary

Add one persisted multi-line **Description** field to the Soniox Settings section. Snapshot its exact saved value when a recording starts and pass a non-empty value through the provider boundary to Soniox as `context.text`. Empty means no Soniox `context` object.

This document plans the change only. It does not include implementation.

## Scope

- One Description textarea in Settings.
- Empty by default and persisted across restarts.
- Exact string round-trip with a 10,000-character maximum.
- Applied to the next recording session, not an active one.
- Non-empty values serialized by the Soniox provider as `context.text`.

No vocabulary editor, structured context builder, automatic suggestions, presets, trimming, parsing, token estimator, or additional Soniox controls are included.

## Data Flow

1. Settings loads the saved description from a backend query.
2. The user edits and saves the textarea.
3. The backend validates the 10,000-character maximum, persists the exact string, and returns the saved value or success state.
4. When recording starts, the app controller snapshots the current description with the other session-applied settings.
5. The snapshot crosses the provider boundary as a provider-neutral description string.
6. The Soniox provider includes `context.text` only when the string has nonzero length.

## Frontend Changes

### `src/app-view.ts`

- Add a labeled multi-line Description control to the existing Soniox Settings section.
- Set the native maximum length to 10,000 characters.
- Expose the textarea and a small local save-status element through `AppView`.
- Keep the module limited to markup construction and DOM lookup.

### `src/tauri.ts`

- Add typed wrappers for the backend description read and save commands.
- Keep raw Tauri command names inside this adapter.

### `src/main.ts`

- Load the saved description when Settings data initializes.
- Save the textarea's exact value without trimming.
- Render save success or failure next to the field without changing capture state.

### `src/styles.css`

- Style the textarea consistently with existing Settings inputs.
- Give it enough height for several lines while preserving the compact window layout.
- Allow vertical resizing only if it does not break the Settings layout; otherwise use a fixed minimum height with internal scrolling.

## Backend Changes

### Settings State

- Add `transcription_description: String` with an empty default.
- Persist it in the existing non-secret application settings location.
- Count characters consistently in the save path and reject values over 10,000 without overwriting the previous value.
- Do not log the description or any derived content.

### Commands

- Add one read command and one save command, or extend an existing cohesive settings command if one already owns equivalent values.
- Return a clear app-level validation error for over-limit input.
- Register command permissions and generated Tauri capability changes when required.

### Session Start

- Read and clone the saved description when the backend accepts a start transition.
- Include that snapshot in the provider-neutral session options.
- Do not let later settings changes mutate the active session.

### Soniox Provider

- Extend the internal config serializer with an optional context object containing `text`.
- Serialize exact non-empty description content.
- Omit `context` when the description length is zero.
- Keep all Soniox field names and JSON shapes inside the provider module.

## Verification

### Backend Unit Tests

- Missing persisted data loads as an empty description.
- Empty, multi-line, and whitespace-surrounded values round-trip exactly.
- A 10,000-character value saves successfully.
- A 10,001-character value fails and preserves the prior saved value.
- Starting a recording snapshots the current description.
- Editing the saved description during recording does not alter the active session.
- Empty description omits `context` from Soniox configuration.
- Non-empty description serializes exactly as `context.text`.

### Frontend Checks

- Settings displays the persisted value after reload.
- The Description control accepts multiple lines and prevents input beyond 10,000 characters.
- Saving sends the exact textarea value without trimming.
- Save status remains local to the Description field.

### Project Checks

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

## Acceptance Criteria

- Settings contains exactly one new Description textarea for transcription context.
- The saved value survives restart and is unchanged by persistence.
- Every new recording uses the description value captured at its start.
- Non-empty text reaches Soniox only as `context.text` inside the provider configuration.
- Empty text produces no `context` object.
- No description content appears in diagnostics or support exports.
- The existing Capture flow, API-key storage, language behavior, and other provider settings remain unchanged.

## References

- [ADR 0018](adrs/0018-soniox-transcription-description.md).
- [Soniox context documentation](https://soniox.com/docs/stt/concepts/context).
- [Soniox WebSocket API](https://soniox.com/docs/api-reference/stt/websocket-api).
