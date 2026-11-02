---
name: release
description: Cut an ORBIS release - gates (clean tree, green CI, zero compiler warnings, clippy, full tests), version bump, tag, CI builds and attaches the zip, release notes. Use only when the user runs /release.
disable-model-invocation: true
---

# Release

A `v*` tag makes CI (`.github/workflows/ci.yml`) test, package (`scripts/package.sh`, universal binary + docs) and attach `dist/orbis-<version>-macos.zip` to a GitHub release. Stop at the first failed gate and say which; never skip one.

## 1. Gates
1. On `master`, `git pull --rebase`, clean tree (`git status --short` empty). Otherwise stop.
2. Last CI run on `master` green: `gh run list -L 1` (`gh run watch` if running).
3. **Zero compiler warnings:** `touch src/lib.rs src/main.rs; cargo build --release --all-targets 2>&1 | grep -c '^warning'` must be 0. Fix the cause, never `#[allow]` a warning without a one-line reason.
4. **Clippy:** `cargo clippy --all-targets`.
   - **Errors** (deny-by-default lints) block the release: fix them.
   - Warnings: `cargo clippy --fix --allow-dirty --all-targets` for the machine-applicable ones, `cargo test`, commit as "Fix clippy warnings". The rest: fix when small and obviously safe; otherwise `gh issue create` one issue listing them (English) and carry on. Never reformat code to appease style lints (the code uses long lines; `cargo fmt` is not enforced).
5. **Full `cargo test`** passes (not just the relevant tests). Failure: root cause, no symptom patch. After the repo was moved, touch the sources first (stale `CARGO_MANIFEST_DIR`).
6. `git log <last tag>..HEAD` has nothing alarming (WIP commits like `wip(...)` mean the area is unfinished: ask).

## 2. Version
1. Last tag: `git describe --tags --abbrev=0` (none yet: first release is the version in `Cargo.toml`, 0.1.0).
2. Propose the next semver from the commits (0.x: minor for new export capability or behaviour change, patch for fixes) and **ask the user to confirm** the number.
3. Bump `version` in `Cargo.toml`, `cargo build` to refresh `Cargo.lock`. Do not touch the dump `format_version` here (it changes only with an incompatible dump change, in script and crate together).

## 3. Notes
Release notes in English from `git log <last tag>..HEAD --no-merges`, grouped (New, Fixed, Calibration, Internal), issue numbers as `#N`. No personal data. Mention known limits: the binary is ad-hoc signed until #3, so a downloaded copy is quarantined (`xattr -d com.apple.quarantine`), macOS only.
Write them to a file in the scratchpad, show the user, proceed on their yes.

## 4. Publish
1. `git commit -am "Release vX.Y.Z"`, `git push`, wait for green CI (`gh run watch`).
2. `git tag vX.Y.Z && git push origin vX.Y.Z`, then `gh run watch` for the tag run.
3. `gh release view vX.Y.Z` shows the zip; `gh release edit vX.Y.Z --notes-file <file>`.
4. `scripts/install.sh` so the DAZ script exports with the released build; tell the user to re-export.
5. Report: version, link (`gh release view --json url -q .url`), zip name, anything carried over (clippy issue).

## Never
Delete or move a tag/release (`gh release delete`, `git push --force`, tag rewrites) without the user asking; publish with a red gate; release from a dirty tree or a branch other than `master`.
