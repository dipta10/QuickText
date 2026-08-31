# App Version And Build Display Implementation Plan

## Summary

Show the installed QuickText application version and exact release build in Settings so a user can report which distributed artifact they are running.

Place two read-only metadata rows and one copy action in the existing **App** section at the bottom of Settings:

```text
App
Closing the window will hide QuickText when tray mode is connected.

Version                                      0.1.0
Build                           v0.1.0-pre.42

                         Copy build information
```

The application version alone is insufficient because every current rolling release uses `0.1.0`. The Build value must contain the exact GitHub release tag, such as `v0.1.0-pre.42`, which identifies one release across all of its Linux, macOS, and Windows artifacts.

This document plans the change only. It does not authorize or include implementation.

## Goals

- Let users and support identify the exact installed QuickText release without leaving the app.
- Distinguish rolling releases that share the same application version.
- Present the metadata in a quiet, discoverable location outside the Capture workflow.
- Provide a single action that copies the relevant identity in a support-ready format.
- Give every platform artifact from one release the same Build value.
- Make missing metadata non-fatal without silently misidentifying a distributed build.

## Non-Goals

- Implementing the feature in this documentation branch.
- Displaying the source revision, operating system, architecture, package-manager revision, or diagnostics identifiers.
- Adding an About window, modal, system-tray About item, update check, release-notes link, or download link.
- Synchronizing the three application-version declarations currently present in the repository.
- Implementing the broader support diagnostics feature.
- Changing the release tag scheme or retention policy.

## Product Decisions

### Location

Keep the existing **App** section as the final section in Settings. Add the identity rows after the tray/background explanation.

Do not show release metadata in Capture or the global bottom status. It supports troubleshooting rather than dictation and should not compete with recording state or errors.

### Displayed Identity

Show two separately labeled values:

- **Version**: The packaged QuickText application version, for example `0.1.0`.
- **Build**: The exact release tag embedded in a distributed artifact, for example `v0.1.0-pre.42`.

Do not combine the values into one ambiguous version string. Do not prefix the Version value with `v`; preserve that prefix only where it is part of the release tag.

### Development Builds

A local or manual build without a release build identifier displays:

```text
Version                                      0.1.0
Build                                  Development
```

Do not derive a precise-looking development identifier from the current Git commit. A local worktree can contain uncommitted changes, so a commit-derived value could falsely imply reproducibility.

### Copy Action

Add one **Copy build information** action. It copies a self-contained plain-text block:

```text
QuickText
Version: 0.1.0
Build: v0.1.0-pre.42
```

For a local build, the final line is `Build: Development`. If either lookup value is unavailable, copy the visible `Unavailable` value rather than disabling the action.

Show copy success or failure in a small status message local to the App section. Do not replace recording, shortcut, or other Settings status messages.

## Domain Language

- **Application version** identifies the QuickText product version declared by the packaged application, such as `0.1.0`. Multiple releases may share it.
- **Build ID** identifies one distributed QuickText release. It is the exact release tag, such as `v0.1.0-pre.42`, embedded unchanged in every platform artifact attached to that release.
- **Development build** is a local or manual build created without a release Build ID. Its user-facing Build value is `Development` and does not claim to identify a reproducible distributed artifact.

The Build ID is not the GitHub Actions run number by itself, an Arch `pkgrel`, or the source revision. The current release tag contains the run number, but the complete tag is the canonical support identifier because it matches the Releases page directly.

## Data Contract

Expose one backend query named `get_build_info` with a provider-neutral response shape:

```ts
type BuildInfo = {
  version: string;
  buildId: string;
};
```

Example release response:

```json
{
  "version": "0.1.0",
  "buildId": "v0.1.0-pre.42"
}
```

Example local-development response:

```json
{
  "version": "0.1.0",
  "buildId": "Development"
}
```

Return both values from one backend query so the frontend does not need to know how either value is sourced.

## Source Of Truth

### Application Version

Read Version from the running Tauri application's package information in the Rust backend. Do not hard-code `0.1.0` in the frontend or copy it from `package.json` at frontend build time.

The repository currently repeats `0.1.0` in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Aligning or automating those declarations is a separate release-management concern. This UI reports the version exposed by the packaged running application.

### Release Build ID

Embed Build ID at Rust compile time through a narrowly named environment value such as `QUICKTEXT_BUILD_ID`.

- Distributed builds receive the exact release tag, such as `v0.1.0-pre.42`.
- Every platform build in the same release receives the same value.
- Local and manual builds without the environment value compile with the fallback `Development`.
- The application does not inspect Git metadata, filenames, package metadata, or the network at runtime.
- Build ID remains immutable for the lifetime of the installed binary.

The exact mechanism may use Rust's compile-time environment support directly or expose the value through the existing Rust build script. The implementation should choose the smallest approach that still supports validation and tests.

## Release Integrity Contract

The CD workflow must never publish a distributed artifact whose embedded Build value is `Development`.

Before each release build:

1. Set `QUICKTEXT_BUILD_ID` to the workflow's exact `RELEASE_TAG` value.
2. Validate that the build identifier is non-empty and exactly matches `RELEASE_TAG`.
3. Pass the value to every Tauri matrix build and the separate Arch package build.
4. Fail the affected release job before publication if validation or propagation fails.

The Arch package's `pkgrel` remains package-manager metadata. It must not replace the release Build ID. The Arch build path must explicitly export the same `QUICKTEXT_BUILD_ID` used by the Linux, macOS, and Windows Tauri bundle jobs.

One GitHub Release is therefore represented as:

| Artifact | Application Version | Build ID |
|---|---:|---|
| Windows installer. | `0.1.0`. | `v0.1.0-pre.42`. |
| macOS disk image. | `0.1.0`. | `v0.1.0-pre.42`. |
| Debian, RPM, and AppImage packages. | `0.1.0`. | `v0.1.0-pre.42`. |
| Arch package with `pkgrel=42`. | `0.1.0`. | `v0.1.0-pre.42`. |

## Proposed Implementation Boundary

### Rust Backend

- Define a serializable `BuildInfo` response with `version` and `build_id` fields and serialize it with camel-case field names so the frontend receives `buildId`.
- Obtain Version from the running application's package information.
- Resolve Build ID through a small pure function that accepts the optional compile-time value and falls back to `Development` only when the release value was not supplied.
- Register one read-only `get_build_info` Tauri command.
- Keep Build ID independent of recording state, app-controller transitions, settings persistence, and provider behavior.

### `src/tauri.ts`

- Add a typed `BuildInfo` frontend shape.
- Add one `getBuildInfo()` wrapper around the `get_build_info` command.
- Keep the raw command name and Tauri invocation in this side-effect boundary.

### `src/app-view.ts`

- Add semantic label/value markup for Version and Build to the existing **App** section.
- Add a **Copy build information** button and a local status element.
- Add the new elements to the `AppView` interface and DOM lookup returned by `createAppView()`.
- Keep this module limited to template construction and element lookup.

### `src/main.ts`

- Request build information once during application startup after creating the view.
- Populate both metadata rows from the single backend response.
- Format the support-ready text from the values currently displayed.
- Use the existing clipboard boundary for the copy action.
- Render copy success or failure only in the App-section status element.
- Do not add build information to recording state or persisted user settings.

### `src/styles.css`

- Style Version and Build as compact metadata rows consistent with existing Settings typography.
- Use a subdued color for labels and a slightly stronger color for values.
- Keep the values selectable and visually non-interactive.
- Allow long future release tags to wrap without horizontal overflow at the minimum window width.
- Style the copy action consistently with secondary Settings actions.

### CD And Arch Packaging

- Set the release Build ID from `RELEASE_TAG` for every platform matrix build.
- Validate the release Build ID before artifact publication.
- Pass the same value into the Arch container and export it during `makepkg`'s application build.
- Keep the existing release tag scheme, platform package versions, and Arch `pkgrel` behavior unchanged.

## Loading And Failure Behavior

- Load build information once during application startup because it cannot change while the process runs.
- Render neutral placeholders until the backend query completes.
- Replace each placeholder with the returned value on success.
- If the query fails, display `Unavailable` for both values and keep QuickText fully usable.
- Do not surface a global application error, change recording state, or overwrite the bottom status line.
- Do not retry continuously.
- Keep **Copy build information** enabled and copy the visible values, including `Unavailable`.

## Accessibility

- Expose `Version` and `Build` as visible labels rather than relying on row position.
- Keep both values as selectable text.
- Give the copy action an unambiguous visible label.
- Associate the App-section copy status with the action without using an interruptive alert.
- Use a polite status announcement for copy completion or failure.
- Do not announce the initial metadata load through a live region.
- Preserve readable contrast in the default dark theme.

## Test Plan

### Backend Tests

- Verify `BuildInfo` contains the running package version.
- Verify a supplied compile-time release value is returned unchanged.
- Verify a non-release build without that value reports `Development`.
- Verify the command response serializes to the frontend contract.

### Frontend Tests

- Verify Version and Build render from one successful response.
- Verify the loading placeholders are replaced.
- Verify a failed query renders `Unavailable` without changing unrelated status or app state.
- Verify the copy payload exactly matches the displayed release values.
- Verify a development build copies `Build: Development`.
- Verify unavailable values remain copyable.
- Verify success and error feedback stays local to the App section.

### CD And Packaging Checks

- Verify every release job receives a non-empty `QUICKTEXT_BUILD_ID` equal to `RELEASE_TAG`.
- Verify the Linux, macOS, Windows, and Arch binaries from one release all report the same Build ID.
- Verify the Arch Build value is the release tag rather than `pkgrel` alone.
- Verify a simulated missing release Build ID fails before publishing artifacts.

### Repository Checks

- Run `npm run build`.
- Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- Run `cargo fmt --manifest-path src-tauri/Cargo.toml --check`.

### Manual Checks

1. Launch a local build with `npm run tauri dev`.
2. Open Settings and scroll to the final **App** section.
3. Confirm Version matches the packaged development version and Build shows `Development`.
4. Copy build information and confirm the clipboard contains the exact three-line development payload.
5. Confirm copy feedback appears only in the App section.
6. Confirm the rows and action fit at the minimum supported window width without clipping or horizontal scrolling.
7. Confirm returning to Capture and recording behavior are unchanged.
8. Install one CD artifact on each supported platform and confirm Settings reports the exact release tag that contains that artifact.

## Acceptance Criteria

- The bottom **App** section in Settings shows separately labeled Version and Build values.
- Version comes from the running Tauri package information.
- A distributed Build value exactly matches its GitHub release tag.
- Every artifact attached to one release reports the same Build value across Linux, macOS, and Windows.
- A local or manual build without a release Build ID shows `Development`.
- The CD pipeline fails before publication when the release Build ID is missing or does not match `RELEASE_TAG`.
- One **Copy build information** action copies the exact visible values in the agreed three-line format.
- Copy feedback stays local to the App section.
- Missing metadata shows `Unavailable`, remains copyable, and does not interfere with dictation or other status messages.
- Version and Build are not persisted as user settings and are not added to recording state.
- Source revision, platform metadata, update behavior, and diagnostics remain out of scope.
- The layout remains accessible and usable at the minimum supported window size.

## Implementation Sequence

1. Add compile-time Build ID handling and the backend `BuildInfo` query.
2. Add backend contract tests for release, development, and serialization behavior.
3. Propagate and validate `RELEASE_TAG` as the Build ID in every CD packaging path.
4. Add the typed frontend query wrapper.
5. Add the metadata rows, copy action, and local status element to the App section.
6. Load and render build information once during startup with the non-fatal fallback.
7. Format and copy the visible build information through the existing clipboard boundary.
8. Add compact responsive styling and accessibility behavior.
9. Run automated, packaging, and manual verification.

## Grilled Decisions

- **Purpose?** Identify the precise installed release so users can report what they are running.
- **One combined string or two rows?** Two rows. Application Version and Build ID answer different questions.
- **Build ID format?** The exact release tag, not the run number alone or a package-manager revision.
- **Location?** The final App section in Settings, not Capture, a global footer, or a new About surface.
- **Interaction?** One action copies both values in a support-ready block.
- **Development identity?** `Development`, because a commit-derived value can misrepresent a dirty local build.
- **Metadata boundary?** One backend query returns both values so sourcing remains outside the UI.
- **Missing release identity?** Fail CD before publication; never ship a distributed artifact labeled Development.
- **Cross-platform identity?** Every artifact in one release reports the same release tag.
- **Source revision?** Deferred to diagnostics rather than embedded for an unused UI field.
- **Loading?** Once at startup, because the values are immutable for the process lifetime.
- **Failure behavior?** Non-fatal and locally displayed as `Unavailable`.
- **Copy feedback?** Local to the App section so it cannot overwrite recording or settings status.
- **Platform metadata?** Excluded from this focused support identity.

## References

- [ADR 0013: Persist Release History With Bounded Retention](adrs/0013-persist-release-history.md).
- [ADR 0015: Local Privacy-Safe Support Diagnostics](adrs/0015-local-support-diagnostics.md).
- [QuickText glossary](glossary.md).
