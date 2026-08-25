# ADR 0014: Promote Releases Through A Develop Channel

## Status

Accepted; not yet implemented. This ADR records the target design only.

## Context

Every merge to `main` currently publishes installers straight to the channel friends use (ADR 0013). That makes `main` releases exactly as reliable as the most recent merge: there is no soak time, no place where a build can be tried before it reaches non-technical users, and no way to publish a work-in-progress integration without either exposing it to friends or not building it at all.

A two-branch promotion model separates those concerns: integration happens on `develop`, and only promoted work reaches `main` and the friends channel.

## Decision

Introduce a `develop` branch and split the release pipeline into a dev channel and a stable channel.

- Branch flow: feature branches target `develop`; `develop` merges into `main` are the promotion step. Direct merges from feature branches to `main` are against the model; the discipline is documented here and can later be enforced with GitHub branch protection (not configured yet).
- **Dev channel** (`push` to `develop`): the CD workflow builds all three platforms and publishes a release tagged `v0.1.0-dev.<run-number>`, marked as a **pre-release**. Pre-releases never appear as "Latest" on the homepage, so dev builds stay invisible to friends while remaining downloadable from the releases page.
- **Stable channel** (`push` to `main`, normally from `develop`): the CD workflow publishes a release tagged `v0.1.0.<run-number>`, as a **regular release**, which becomes the "Latest" download friends see.
- **Retention stays bounded per channel**: the prune step filters releases by tag prefix (`v0.1.0-dev.` vs `v0.1.0.`) and keeps the newest 10 of each. Prefix filtering is essential — a naive "delete beyond newest 10" would let dev churn delete stable releases.
- **CI gates both**: pull requests targeting `develop` and pull requests targeting `main` (the promotion PR) run the test suite; pushes to either branch run it as well. A promotion PR is deliberately re-tested so a bad merge cannot silently break `main`.
- Implementation shape is left open (one workflow with channel conditionals or two workflow files) until implementation begins.

## Consequences

- Friends always see the newest promoted stable build; dev builds exist on the same releases page without disturbing what they download.
- Every feature merge can cost two full three-platform builds (once on `develop`, again on promotion), roughly doubling Actions minutes compared to today.
- The model depends on merge discipline: a feature branch merged directly into `main` bypasses the dev channel and nothing in the current setup prevents it.
- Two release histories mean two prune filters; getting the prefix wrong deletes the wrong channel's history, so the prune logic needs a test before it runs in anger.
- Tag names now encode the channel, making it obvious from a link which channel a build came from.
- The hardcoded `v0.1.0` version prefix now appears in both channels' tag schemes; version bumps must update both (open question carried from ADR 0009).

## Grilled Decisions

- **Two channels at all?** Yes; the user wanted a soak layer so that only deliberately promoted builds reach friends, accepting doubled build minutes.
- **Tag scheme?** `v0.1.0-dev.<run-number>` for dev, `v0.1.0.<run-number>` for stable; keeping "pre" in stable tags was rejected as confusing, and true semver (`v0.1.0`, `v0.1.1`) on main was rejected for now because manual version bumps add promotion ceremony.
- **Retention?** Newest 10 per channel, pruned independently by tag prefix; a single shared prune was rejected because dev churn would evict stable releases.
- **What shows as "Latest"?** Stable releases only, by construction: dev builds are pre-releases, which GitHub never surfaces as Latest — this also preserves the homepage-sidebar visibility the user wanted for friends.
- **Where does CI run?** On PRs to both `develop` and `main`; gating only `develop` was rejected because promotion merges could still break `main` untested.
- **Enforce main-only-from-develop?** Documentation only for now; branch protection is a manual follow-up for the repo owner.

## Open Questions

- One workflow with `if` conditionals on the ref, or two workflow files (`cd-develop.yml`, `cd-main.yml`)?
- Should promotion PRs (`develop` → `main`) be required to have their base up to date, or is a fast-forward-ish merge acceptable?
- When a stable channel exists, should dev retention shrink (for example 5) to keep the releases page shorter?
- Should `workflow_dispatch` remain enabled for manual channel builds during bring-up?
