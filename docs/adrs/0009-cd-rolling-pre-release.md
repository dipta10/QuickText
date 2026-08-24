# ADR 0009: Rolling Pre-Release Channel From Main

## Status

Accepted for the initial CD slice.

## Context

The CI pipeline (ADR 0008) verifies pushes but produces nothing installable. Users can only run QuickText from source, and there is no way to hand someone a build to try.

Tauri cannot cross-compile bundles: each platform's installer (deb/AppImage, dmg/app, msi/nsis) must be produced on its native OS runner. All bundles are unsigned for now; signed artifacts require paid certificates and a separate slice.

Every push to `main` is considered shippable-by-default because CI gates merges, so the release channel should track `main`.

## Decision

Add a GitHub Actions workflow (`.github/workflows/cd.yml`) that builds installers on every push to `main` and publishes them as a single rolling pre-release on GitHub Releases.

- A serial **prepare** job first deletes any existing `v0.1.0-pre` release and its tag (`gh release delete --cleanup-tag`), guaranteeing a clean slate.
- A matrix job then builds on `ubuntu-22.04`, `macos-latest`, and `windows-latest`, using `tauri-apps/tauri-action` to bundle and attach artifacts to the recreated `v0.1.0-pre` pre-release.
- Linux builds pin `ubuntu-22.04` so AppImage/deb binaries link against older glibc and run on more distributions.
- The workflow uses `concurrency: cd-main` with `cancel-in-progress: false` so simultaneous merges to `main` publish sequentially instead of racing each other's asset uploads.
- Bundles are unsigned; release notes state this explicitly so OS warnings are not a surprise.
- No version tags are consumed or created besides the fixed rolling tag; real versioned releases remain a future manual flow.
- A `workflow_dispatch` trigger exists temporarily so the pipeline can be exercised from the feature branch before merging; the first manual run publishes branch-built artifacts that the next push to `main` replaces.

The prepare/build split exists because deleting the old release inside the parallel build matrix would race: one platform could delete assets another had just uploaded.

## Consequences

- There is always one current pre-release URL to share, instead of an accumulating pile of per-commit releases.
- Each push to `main` pays three full Tauri builds (~10-15 min with warm caches); frequent merges make this the dominant Actions cost.
- The tag `v0.1.0-pre` is hardcoded against the manifest version `0.1.0`; bumping the app version requires updating the workflow, which is tracked as an open question.
- Unsigned macOS builds require users to bypass Gatekeeper manually; unsigned Windows builds trigger SmartScreen warnings.
- Anyone watching releases gets a notification per main push; watchers who find this noisy can unwatch releases only.
- Failed builds leave no release at all rather than a stale one, since deletion happens before building.

## Grilled Decisions

- **What does CD deliver?** Installers attached to a public GitHub Release; workflow-artifacts-only was rejected because Actions artifact links expire and are awkward to share.
- **Which platforms?** All three despite the testing caveat; the user accepted that macos/windows artifacts ship untested by them until they have hardware.
- **What triggers it?** Every push to `main`; the user explicitly chose continuous delivery over tag-triggered releases.
- **Literal release-per-push?** Rejected during grilling as spam; resolved into a single rolling pre-release that replaces itself, keeping real versioned releases clean and manual.
- **Rolling replace vs accumulate?** Replace; accumulation reintroduces the spam problem the trigger choice created.
- **Why a prepare job?** Deletion must happen exactly once before any builder starts; doing it in the matrix races across platforms.
- **Signing?** Deferred; needs paid certs and secrets plumbing, orthogonal to pipeline mechanics.
- **Version source for the tag?** Hardcoded `v0.1.0-pre` for simplicity now; dynamic extraction from `tauri.conf.json` is the known follow-up.

## Open Questions

- Should the pre-release tag be derived automatically from `tauri.conf.json` so version bumps propagate without editing the workflow?
- Should CD be gated on the CI workflow passing first (workflow_run dependency), or is the merge gate trusted?
- When signing arrives, does the rolling channel get signed too, or only tagged releases?
