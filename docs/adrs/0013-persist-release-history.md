# ADR 0013: Persist Release History With Bounded Retention

## Status

Accepted; amends the rolling-replace policy of ADR 0009.

## Context

ADR 0009 replaced the previous pre-release on every push to `main`. That design kept the releases page tidy, but it destroyed the last known-good build the moment a new one published. If a fresh build broke recording or failed to launch, there was nothing to roll back to except reverting the commit and rebuilding — which only helps if the rebuild succeeds and even then produces new artifacts, not the ones that were known to work.

The app has no auto-updater, so "rolling back" means re-downloading an older installer. That is only possible if older installers still exist.

## Decision

Publish every push to `main` as its own release instead of replacing a single rolling one.

- Tag scheme: `v0.1.0-pre.<run-number>` (for example `v0.1.0-pre.42`), where the run number increments with each merge, giving readable, sortable, chronological tags.
- Each release is a regular release, so the newest build always appears as "Latest" on the repo homepage and at the `releases/latest` permalink.
- A cleanup job runs after publishing and deletes releases beyond the newest 10, along with their tags. Retention is a workflow-level variable (`KEEP_RELEASES`).
- The `prepare-release` deletion job from ADR 0009 is removed entirely; nothing deletes the release that was just published.
- The `concurrency: cd-main` queue is kept: simultaneous merges still publish sequentially so asset uploads never race.
- No curated stable channel yet; every main build is equally downloadable, newest highlighted.

Rollback is now "point people at the previous release link" rather than any git operation.

## Consequences

- A broken merge can no longer destroy the installers that worked; the previous build remains one release away.
- The releases page holds at most 10 near-identical entries; older than that are pruned, so very old builds eventually become unrecoverable by design.
- The hardcoded `v0.1.0` now appears in the tag scheme and the release-notes download table; version bumps must touch both (still an open question from ADR 0009).
- Friends and testers must re-download manually to update; nothing changed there, but the releases page now makes older versions visibly available.
- Each push creates a tag, so the tag list grows alongside releases and is pruned with them.

## Grilled Decisions

- **Rolling replace vs accumulate?** Accumulate. The user rejected replacement outright after realizing a broken build would destroy the last working installers; commit-revert does not restore artifacts.
- **Retention window?** Last 10. "Keep everything" was rejected as unbounded clutter; "keep 3" as too shallow a rollback window for a project merging several times a day.
- **Tag naming?** Run number (`v0.1.0-pre.42`); commit SHA was rejected as unreadable and unsortable at a glance, timestamps as noisy.
- **What shows as "Latest"?** The newest build, as a regular release; all-pre-release was rejected because the homepage sidebar hides pre-releases, which the user had already discovered the hard way.
- **Stable channel now?** Deferred. A tag-triggered curated channel is the long-term shape but adds a manual promotion ritual the project will not perform at MVP stage.
- **Archive-old-assets-instead-of-delete?** Rejected during grilling as complexity without benefit over plain accumulation.

## Open Questions

- Should the tag and notes derive their version from `tauri.conf.json` so bumps propagate without workflow edits?
- Should the retention count differ once a stable channel exists (for example keep 10 dev builds but all stable ones)?
