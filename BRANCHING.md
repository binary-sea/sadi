# Git Workflow for SaDi

## 🎯 Goals

- Support stable releases with semantic versioning
- Keep release process reproducible and automated using `cargo-release`
- Maintain `main` branch always in releasable state
- Ensure high code quality through PR reviews and automated checks
- Align with SaDi's contribution guidelines and issue management

## 🌳 Branch Strategy

### Main Branches
- **`main`** — Stable production branch. All releases are tagged from here.

### Development Branches
- **`feature/description`** — New features (e.g., `feature/async-support`)
- **`fix/description`** — Bug fixes (e.g., `fix/circular-dependency-detection`)
- **`docs/description`** — Documentation updates (e.g., `docs/api-examples`)
- **`hotfix/description`** — Critical fixes for production issues

### Branch Naming Guidelines

Follow the patterns established in our contributing guidelines:
```bash
# Features
git checkout -b feature/container-scoping
git checkout -b feature/conditional-registration

# Bug fixes  
git checkout -b fix/memory-leak-in-factory
git checkout -b fix/thread-safety-issue

# Documentation
git checkout -b docs/contributing-guide
git checkout -b docs/api-reference-update

# Hotfixes (urgent production fixes)
git checkout -b hotfix/security-vulnerability
```

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

## 🔄 Development Workflow

### Daily Development

1. **Stay synchronized with main:**
   ```bash
   git checkout main
   git pull origin main
   ```

2. **Create feature branch:**
   ```bash
   git checkout -b feature/your-awesome-feature
   ```

3. **Development cycle:**
   ```bash
   # Make changes
   git add .
   git commit -m "feat: add awesome feature implementation"
   
   # Ensure quality (run frequently during development)
   cargo test --all
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo fmt
   ```

4. **Push and create PR:**
   ```bash
   git push origin feature/your-awesome-feature
   # Open PR using our PR template on GitHub
   ```

### PR Requirements (Enforced by Template)

**Before opening PR:**
- [ ] All tests pass: `cargo test --all`
- [ ] No clippy warnings: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] Code is formatted: `cargo fmt`
- [ ] Documentation updated for API changes
- [ ] Examples added for new features

**PR must include:**
- Clear motivation and problem description
- Summary of implementation changes
- Test coverage for new functionality
- Documentation updates (inline docs, README if needed)
- Adherence to SaDi's API design principles

### Issue-Driven Development

Follow our issue template workflow:
- **Bug fixes**: Reference the bug report issue number
- **Features**: Reference the feature request issue number
- **Use issue discussions**: For questions and clarifications before coding

### Quality Gates

All PRs must pass:
1. **Automated CI checks** (tests, clippy, formatting)
2. **Code review** focusing on:
   - Type safety and zero-cost abstractions
   - API ergonomics and consistency
   - Test coverage and documentation
   - Alignment with SaDi's design principles
3. **No direct commits to main** except for releases

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

## 🛡️ Branch Protection & Review Policy

### Main Branch Protection
- **Protected branch**: `main` requires PRs for all changes
- **Required reviews**: At least one approving review
- **Required checks**: All CI status checks must pass
- **No force pushes**: Maintain clean history
- **Merge method**: Squash and merge (clean linear history)

### Review Process
1. **Automated checks**: CI runs tests, clippy, formatting
2. **Human review**: Focus on:
   - Code quality and SaDi design principles
   - Test coverage and documentation
   - API consistency and type safety
   - Performance implications
3. **Feedback resolution**: Address all comments before merge
4. **Maintainer approval**: Required before merge

### Exception: Release Commits
- `cargo-release` commits are allowed direct pushes to `main`
- Release tags are created automatically by `cargo-release`
- Manual releases require maintainer privileges

## ✅ Quality Assurance & Testing

### Local Testing (Before PR)
```bash
# Always test locally first
cargo release patch --dry-run        # Test release process
cargo test --all                     # Run all tests
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check                    # Verify formatting
cargo doc --no-deps                  # Ensure docs build
```

### CI Pipeline Testing
- Test CI without publishing: `cargo release <level> --execute --no-publish`
- All PRs trigger full CI pipeline
- Main branch changes trigger release readiness checks

### Integration with Issue Templates

**For Bug Fixes:**
- Reference bug report issue: `Fixes #123`
- Include reproduction case from issue template
- Verify fix addresses all points in bug report

**For Features:**
- Reference feature request issue: `Implements #456`
- Ensure implementation matches proposed API design
- Update documentation with examples from feature request

## 📋 Commit Message Standards

Following our contributing guidelines:
```
type: brief description (50 chars max)

Longer description explaining the what and why.
Reference issues and PRs.

- Specific changes made
- Fixes #123
- Implements #456
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## 📚 Related Documentation

- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Detailed contributor guidelines
- **[PR Template](.github/PULL_REQUEST_TEMPLATE.md)** - Required PR format
- **[Issue Templates](.github/ISSUE_TEMPLATE/)** - Bug reports and feature requests
- **[README.md](README.md)** - Project overview and roadmap

---

**Note**: This workflow emphasizes quality, type safety, and thorough testing in line with SaDi's mission to provide a reliable, zero-cost dependency injection solution for Rust.
