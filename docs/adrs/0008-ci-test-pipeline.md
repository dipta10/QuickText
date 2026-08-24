# ADR 0008: Run Tests In CI Before Any Build Pipeline

## Status

Accepted for the initial CI slice.

## Context

The project has Rust tests across five source files and a TypeScript frontend, but nothing verifies either automatically. Regressions are only caught when a developer runs `cargo test` or `npm run build` locally, and nothing enforces this before merging to `main`.

The app is a Tauri 2 desktop application: `cargo test` compiles the full Tauri stack, which on Linux requires GTK/WebKit system libraries that stock GitHub runners do not have.

Builds and releases are explicitly out of scope for now; the first pipeline slice is verification only.

## Decision

Add a GitHub Actions workflow (`.github/workflows/ci.yml`) that runs verification jobs on pushes to `main` and on pull requests targeting `main`. During bring-up, the `ci/ci-pipeline` branch was temporarily added to the push trigger to validate the workflow itself; it was removed once the pipeline ran green.

- Two independent jobs:
  - **Rust tests**: install Tauri Linux prerequisites via apt, pin stable Rust with `dtolnay/rust-toolchain`, cache Cargo builds with `Swatinem/rust-cache` scoped to `src-tauri`, run `cargo test --locked`.
  - **Frontend typecheck**: `npm ci`, then `npx tsc --noEmit`. This is a check, not a build; no Vite bundle or Tauri artifact is produced.
- A third job, **Soniox integration tests**, runs the `#[ignore]`-gated transcription fixture tests (`cargo test -- --ignored`) against the real Soniox API. It is enabled by the presence of the `SONIOX_API_KEY` repository secret and skips cleanly when the secret is absent. The guard lives at step level because the `secrets` context is not available in job-level `if` expressions; the first step exports an output that every later step conditions on, so a missing secret costs nothing but the checkout.
- Concurrent runs for the same branch are cancelled in favor of the latest push.
- No build, packaging, or release steps exist yet by design.

The split into two jobs keeps a frontend type failure from paying the apt/Rust compile cost and vice versa.

## Consequences

- Merges to `main` cannot bypass tests without an explicit admin override.
- The Rust job pays a one-time apt cost per run; rust-cache keeps recompilation of unchanged dependencies cheap.
- `--locked` fails the build if `Cargo.lock` is out of date, keeping dependency resolution reproducible.
- Frontend behavior beyond types (no JS test runner exists yet) remains unverified until a runner such as Vitest is added.
- Build/packaging CI will be layered on top of this workflow later, likely as additional jobs.

## Grilled Decisions

- **What counts as "tests" today?** Only `cargo test`; no JS test runner exists in `package.json`. `tsc --noEmit` was added as a free non-build check so the frontend is not completely unverified.
- **Triggers?** Pushes to `main` plus PRs targeting `main`; all-branch triggers were rejected as noisy since feature branches already get runs through their PRs.
- **Linux system dependencies?** Installed via apt because `cargo test` compiles Tauri, which links GTK/WebKit; skipping them fails the job at compile time. The first live run also failed on `alsa-sys` because `cpal` needs `libasound2-dev`, which is easy to forget when copying dependency lists from Tauri docs.
- **Cache?** Yes, `Swatinem/rust-cache`; without it every run rebuilds the entire Tauri dependency tree from scratch.
- **Live Soniox tests in CI?** Yes, but only committed fixtures via the existing `#[ignore]` gate, driven by a repository secret. A job-level `if: ${{ secrets... }}` was tried and rejected because GitHub rejects the `secrets` context there; the accepted pattern is a step-level guard output. Skips cleanly when the secret is absent so key-less clones stay green.
- **Why two jobs instead of one?** Independent failure signals and no cross-paying of setup costs; a single job was rejected as conflating unrelated failures.
- **Why no build job yet?** Explicit product decision for this slice; builds arrive later.

## Open Questions

- Which Node version should be pinned long-term, and should it come from `.nvmrc`?
- Should a lint job (`cargo fmt --check`, `cargo clippy`) join the pipeline once formatting policy is settled?
- Should the integration job run on every push or move to a nightly schedule to limit Soniox API usage as fixtures grow?
