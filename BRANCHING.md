# Git flow for this Rust crate (updated for cargo-release)

Goals
- Support stable (current) releases, plus alpha and beta pre-releases.
- Keep release process reproducible and automatable using cargo-release (and optional artifact builds on tag via CI or goreleaser).
- Keep trunk (main) always releasable (stable), and use develop for integration.

Branches (short)
- main — stable production branch. Only merged PRs that are release-ready.
- develop — integration branch for the next stable release.
- feature/* — short-lived features branched off develop.
- fix/* — bugfix branches off develop (or main for hotfixes).
- hotfix/* — critical fixes made off main, merged back to main and develop.
- alpha/* — long-living or short-living branches for alpha tracks (optional). Merge into develop when ready.
- beta/* — beta release branch for wider testing before stable.

Tagging / versioning (cargo-release)
- Use semantic versioning, with pre-release identifiers for alpha/beta.
  - Stable: v1.2.3
  - Beta:   v1.3.0-beta.1
  - Alpha:  v1.4.0-alpha.2
- cargo-release is the canonical tool to:
  - update version(s) in Cargo.toml,
  - update changelog (if configured),
  - create a git tag (tag-name like v{{version}}),
  - push commits and tags,
  - publish crate(s) to crates.io.
- The crate `version` in Cargo.toml must match the tag before publishing; cargo-release keeps these in sync.
- Tags are canonical triggers for the publish/build-release workflow: `refs/tags/v*`.
- Do not manually change the version in Cargo.toml on main/develop except when preparing a release in the normal documented flow; prefer cargo-release to keep commits and tags consistent.

Typical flow (recommended two-step pattern)
- Day-to-day work:
  - Create feature branches from develop: `feature/awesome-thing`.
  - Open PRs against develop. When reliable and tested, merge to develop.
- Preparing a beta/alpha for the next release:
  - Use cargo-release locally or via the release workflow with a pre-release identifier: e.g. cargo release minor --pre-release beta --execute (or run with the inputs your CI dispatch provides).
  - cargo-release will set version to `x.y.z-beta.N`, commit, tag `vX.Y.Z-beta.N`, and (if configured) publish to crates.io.
  - CI tag workflow runs on the new tag and builds GitHub Release artifacts.
- Releasing stable:
  - Merge develop into main, ensure work is merged and tested (you can leave Cargo.toml version as-is if you plan to bump via cargo-release), then run cargo-release (manual dispatch) with the appropriate level (patch/minor/major).
  - cargo-release will update Cargo.toml to `x.y.z`, commit, tag `vX.Y.Z`, push, and publish to crates.io.
  - The pushed tag triggers CI that builds artifacts and creates a GitHub Release with attached binaries.
- Hotfix:
  - Branch from main -> `hotfix/x.y.z` -> implement fix -> run cargo-release patch (or bump in branch and let cargo-release tag/publish) -> merge into main and develop -> tag and publish.

Workspaces
- For a workspace:
  - You can release individual packages: cargo release <level> --package <name> --execute
  - Or release all packages together via cargo-release workspace mode (workspace = true in .release.toml).
  - The CI artifact builder should build the binaries for each package you want to include in the GitHub Release.
  - If you currently have a single crate but plan to add more in v2, design your .release.toml and CI to support both single-package and workspace releases.

CI and triggers
- Recommended: keep the cargo-release run as a manual GitHub Actions workflow (workflow_dispatch). This avoids accidental loops when cargo-release pushes tags and commits.
- The artifact build workflow MUST be tag-only:
  - on: push: tags: - 'v*'
  - This workflow builds platform artifacts (or runs goreleaser) and creates the GitHub Release.
- If you prefer full automation, ensure workflows ignore commits/tags created by the GitHub Actions bot or otherwise prevent loops (e.g., check commit author or use conditions).
- Use fetch-depth: 0 in checkout steps so cargo-release and CI have full history and tags.

Secrets and permissions
- Add repository secret: CRATES_IO_TOKEN = your crates.io API token
  - Expose to Actions as CARGO_REGISTRY_TOKEN for cargo-release.
- GITHUB_TOKEN is provided by Actions; cargo-release will use it to push tags/commits and create releases (if configured).
- Do not store the raw token in code.

Pre-release and crates.io
- Pre-release crates (alpha/beta) are valid on crates.io and are published the same way. Do not reuse version strings.
- If you wish to avoid publishing pre-releases to crates.io automatically, run cargo-release with --no-publish in CI or adjust publish = false in .release.toml for those runs.

Branch protections & reviews
- Protect main and develop.
- Require PRs and at least one approving review for main (maybe two).
- Require passing CI checks before merging.
- Consider requiring signed commits if desired.
- Decide policy for accepting cargo-release commits/tags:
  - cargo-release will create commits and tags; either allow the CI bot to push those, or have maintainers merge the version bump commit created by cargo-release locally and push tags manually if you want stricter control.

Testing & safety
- Test locally first: cargo release patch --dry-run
- Test CI without publishing: cargo release <level> --execute --no-publish or set publish = false temporarily.
- Use a staging crate or scoped registry if you want to test publishes in a non-production registry.

Notes / examples
- Tags: use `v{{version}}` format (e.g., v1.2.3) and treat tags as the single source of truth for releases.
- Avoid changing the version manually after cargo-release has tagged/published — if you need a hotfix immediately after a release, create a hotfix branch and run cargo-release again.
- If you want richer cross-compile release artifacts later, consider using goreleaser on tag events; cargo-release still creates canonical tags and can publish crates.io, and goreleaser produces binaries and uploads them to the GitHub Release.

This document describes the flow and conventions. See RELEASE_PROCESS.md for step-by-step commands and CI/publish automation in .github/workflows.
