# Releasing Herald

This document describes how to cut a new release of Herald.

## Versioning

Herald follows [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking API or configuration changes
- **MINOR** — new features, backward-compatible
- **PATCH** — bug fixes, backward-compatible

While the version is **0.x.y**, minor versions may include breaking changes.

## Pre-Release Checklist

1. **All CI checks pass** on the `main` branch.
2. **CHANGELOG.md** has an `[Unreleased]` section with all user-facing changes.
3. **Cargo.toml versions** are consistent across all workspace members.

## Release Steps

### 1. Update the version

Bump the version in all `Cargo.toml` files:

```bash
# Root workspace
# crates/herald-common/Cargo.toml
# crates/herald-server/Cargo.toml
# (future) crates/herald-cli/Cargo.toml
# (future) crates/herald-web/Cargo.toml
```

### 2. Update CHANGELOG.md

Rename the `[Unreleased]` section to `[X.Y.Z]` with today's date:

```markdown
## [0.2.0] - 2025-03-15
```

Add a new empty `[Unreleased]` section at the top.

### 3. Commit and tag

```bash
git add -A
git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
git push origin main --tags
```

### 4. Automated release

Pushing a `v*` tag triggers the **Release** workflow which:

1. Builds binaries for 5 targets:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu` (cross-compiled)
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-msvc`
2. Packages each binary as `.tar.gz` (unix) or `.zip` (windows) with SHA-256 checksums
3. Extracts release notes from CHANGELOG.md
4. Creates a GitHub Release with all artifacts attached
5. Marks `0.x.y` releases as pre-release automatically

### 5. Verify

- Check the [Releases page](https://github.com/kafkade/herald/releases) for the new release
- Download and verify at least one binary
- Verify the SHA-256 checksum matches

## Hotfix Process

1. Create a branch from the release tag: `git checkout -b hotfix/vX.Y.Z+1 vX.Y.Z`
2. Apply the fix and update CHANGELOG.md
3. Bump the patch version
4. Merge to `main`, tag, and push

## Artifacts

Each release includes:

| File | Description |
|------|-------------|
| `herald-server-vX.Y.Z-<target>.tar.gz` | Binary archive (Linux/macOS) |
| `herald-server-vX.Y.Z-<target>.zip` | Binary archive (Windows) |
| `herald-server-vX.Y.Z-<target>.*.sha256` | SHA-256 checksum |
