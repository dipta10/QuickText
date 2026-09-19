# ADR 0017: Optional Transcription Language Preferences

## Status

Accepted and implemented. Supersedes [ADR 0011](0011-provider-declared-settings.md).

## Context

Soniox automatically detects supported spoken languages when a real-time
session omits `language_hints`. When the likely languages are known, Soniox
accepts one or more ISO language codes as non-strict hints that bias recognition
without preventing other languages from being detected.

QuickText needs that accuracy control, but it does not currently need the
endpoint-delay controls or generic provider-declared settings framework planned
by ADR 0011. Building those abstractions for one setting would add scope without
improving the current dictation flow.

## Decision

Add a language-only preference to the Soniox section of Settings.

- The default is **Automatic detection**. It is represented by an empty saved
  list and causes QuickText to omit `language_hints` from the Soniox session
  configuration.
- Users may select one or more languages from a searchable checklist. The
  closed control shows the selected language, a selected count, or Automatic
  detection.
- The backend owns the supported-language catalog, validates selected ISO
  codes, persists them as non-secret application settings, and snapshots the
  selection when a recording session starts.
- The initial catalog is the alphabetized language list published by Soniox
  for `stt-rt-v5`. It is bundled with QuickText so Settings works without an API
  key, network access, or a provider metadata request.
- Selected codes are sent unchanged as Soniox `language_hints`. QuickText does
  not enable `language_hints_strict`.
- Changes apply to the next recording. An active session keeps the preferences
  with which it started.
- QuickText adds no retry, fallback, or special provider-error behavior for
  language hints. Soniox remains responsible for interpreting the non-strict
  hints.
- Diagnostics may record automatic-versus-preferred mode and the selection
  count, but must not record the selected language codes.

## Consequences

- Users can improve recognition for expected monolingual or multilingual
  speech while retaining Soniox automatic detection as the zero-configuration
  default.
- The frontend deals only in language names and ISO codes. Soniox field names
  and configuration serialization remain in the backend provider.
- The bundled catalog can drift from Soniox and must be reviewed when the
  Soniox model or documented language list changes.
- QuickText gains one focused settings surface rather than a generic settings
  renderer. Future provider controls should justify their own product and
  architecture decisions when they are actually needed.

## Grilled Decisions

- **One language or several?** Several; Soniox accepts multiple hints and
  QuickText users may dictate multilingual speech.
- **What is the default?** No selected codes, so `language_hints` is omitted and
  Soniox detects languages automatically.
- **Where does the list come from?** A backend-owned snapshot of Soniox's
  published language list.
- **Can Settings work without an API key?** Yes; the catalog is bundled.
- **Where is the preference stored?** Backend-managed, non-secret application
  settings.
- **When do changes apply?** At the next recording-session start.
- **Is strict language restriction enabled?** No.
- **Does QuickText retry or fall back on provider errors?** No.
- **Are language codes written to diagnostics?** No.
- **Why not implement ADR 0011 as written?** Endpoint tuning and a generic
  provider-declared settings schema are outside the requested language-only
  feature.

## References

- [Soniox language hints](https://soniox.com/docs/stt/concepts/language-hints).
- [Soniox supported languages](https://soniox.com/docs/stt/concepts/supported-languages).
- [ADR 0002](0002-streaming-soniox-stt.md): Soniox real-time streaming.
- [ADR 0003](0003-backend-first-soniox-integration.md): backend provider boundary.
- [ADR 0004](0004-capture-settings-ui.md): Settings/Capture separation.

