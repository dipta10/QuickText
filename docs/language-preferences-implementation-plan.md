# Language Preferences Implementation Plan

## Goal

Let users optionally choose one or more likely transcription languages. Send
those ISO codes to Soniox when a recording begins. When no language is selected,
omit the Soniox field and retain automatic language detection.

This plan implements [ADR 0017](adrs/0017-language-preferences.md). It does not
include strict language restriction, endpoint controls, a generic provider
settings framework, runtime catalog discovery, retry logic, or notifications.

## User Experience

Add **Transcription languages** directly to the Soniox Settings section.

- The initial value is **Automatic detection**.
- Expanding the control shows a search field and an alphabetical checkbox list.
- Users can select multiple languages.
- The collapsed label shows one language name or the number selected.
- **Clear selections** restores Automatic detection.
- Each change is saved immediately and applies to the next recording.
- The control remains usable without a Soniox API key or network connection.

## Backend

1. Keep the supported Soniox language catalog in the Rust backend as display
   names paired with ISO codes.
2. Expose one command that returns the catalog and current selections.
3. Expose one command that validates, normalizes, persists, and updates the
   current selections.
4. Persist the codes in the platform application configuration directory. An
   absent file or empty list means Automatic detection.
5. Snapshot the current codes at recording start and pass them into the new
   Soniox session.
6. Serialize `language_hints` only when that snapshot is non-empty.

## Frontend

1. Extend the DOM factory with the language picker elements only.
2. Keep loading, filtering, rendering, saving, and error handling in the main
   wiring module, following the existing input-device pattern.
3. Disable the picker during a save and restore the last confirmed selection if
   persistence fails.
4. Show concise status text confirming either saved preferences or Automatic
   detection.

## Validation And Tests

- Reject codes absent from the bundled catalog.
- Normalize codes to lowercase, remove duplicates, and preserve catalog order.
- Verify an empty list omits `language_hints` from the Soniox JSON.
- Verify selected codes serialize as the exact `language_hints` array.
- Verify missing persisted settings start in Automatic detection.
- Run:

  ```bash
  npm run build
  cargo test --manifest-path src-tauri/Cargo.toml
  cargo check --manifest-path src-tauri/Cargo.toml
  cargo fmt --manifest-path src-tauri/Cargo.toml --check
  ```

## Acceptance Criteria

- Settings lists the Soniox-supported languages and supports search and multiple
  selections.
- No selection is visibly described as Automatic detection.
- Selections survive application restart.
- A recording started with selections includes those codes in
  `language_hints`.
- A recording started without selections omits `language_hints`.
- Changing Settings during a recording does not alter that active Soniox
  session.
- Language selection does not require an API key or network request.
- No retry, fallback, strict-mode, endpoint-delay, notification, or generic
  provider-settings behavior is introduced.

