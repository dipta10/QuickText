# ADR 0011: Provider-Declared Advanced Transcription Settings

## Status

Superseded by [ADR 0017](0017-language-preferences.md). The language-only
feature was implemented without endpoint controls or a generic
provider-declared settings framework.

## Context

Settings currently expose only app-level preferences (API key, shortcut, behavior toggles). Users have no control over how the transcription provider behaves during a session.

Soniox's real-time WebSocket API accepts several tunable parameters. Two are meaningful for a short-dictation app:

- `language_hints`: biases recognition toward expected languages; a major accuracy lever for non-English and mixed-language users.
- `max_endpoint_delay_ms` (500–3000, default 2000): how long Soniox waits for silence before finalizing tokens; directly controls how quickly final text appears.

Other parameters were considered and rejected for this scope:

- `endpoint_sensitivity`, `endpoint_latency_adjustment_level`: v5-only micro-tunables with non-obvious effects; not worth UI surface.
- `language_hints_strict`: only matters for pathological mixed-language cases; revisit if users hit garbage-language output.
- Profanity filtering: does not exist in Soniox's API (it is a Deepgram parameter).
- Model selection, custom context, diarization, translation: no second model exists yet; context is free-form UI surface; diarization and translation are out of MVP scope per product constraints.

Custom context was deferred only for this advanced-settings slice. [ADR 0018](0018-soniox-transcription-description.md) later adds one fixed app-level transcription description without expanding the provider-declared settings schema.

A naive "Soniox section" hard-coded in the frontend would violate the project rule that Soniox-specific details stay out of UI code and behind the provider boundary.

## Decision

Expose provider-specific transcription settings through a **provider-declared schema**:

1. The backend asks the active provider for its setting descriptors (id, type, range/choices, default, label). For Soniox this covers language hints (multi-select from a static language list owned by the backend provider) and endpoint delay (integer range).
2. The Settings view renders a collapsed "Advanced transcription" section that expands on click and renders whatever descriptors arrive, generically. Adding Deepgram later requires zero UI changes.
3. Saved values live in **backend state**, sent via a Tauri command; the provider receives them at session start. Backend validates ranges and choices before persisting.
4. Selecting zero languages means no hints are sent; the provider auto-detects.
5. One "Reset to defaults" action restores provider defaults for all declared settings.
6. Changes apply at the start of the next recording session; in-flight recordings keep prior values.

## Consequences

- The frontend stays provider-agnostic; it renders descriptors and posts values back.
- The provider trait grows a settings-descriptor surface; each provider owns its valid choices (e.g. the static Soniox language list) and validation rules.
- Backend state becomes the source of truth for transcription settings, consistent with ADR 0003's backend-first direction.
- Mid-session reconfiguration stays out of scope.
- Future providers implement the same descriptor contract instead of new UI work.
- The later transcription-description field is not part of this descriptor contract; it is a single provider-neutral session option with a fixed UI control.

## Grilled Decisions

- **How do provider settings reach the UI?** Provider-declared schema; UI renders generically.
- **Which Soniox parameters are exposed?** Language hints and max endpoint delay only.
- **Where do saved settings live?** Backend state, applied at next session start.
- **Endpoint controls: raw or preset?** Raw delay slider only; sensitivity and latency adjustment rejected as noise.
- **Where does the language list come from?** Static list owned by the backend Soniox provider.
- **Is `language_hints_strict` exposed?** No.
- **What does zero selected languages mean?** Auto-detect; no hints sent.
- **Reset affordance?** One reset-to-defaults action for the whole advanced section.
- **Mid-recording changes?** Not applied to the in-flight session; take effect next session.

## References

- [ADR 0018](0018-soniox-transcription-description.md): one persisted transcription description mapped to Soniox context text.
