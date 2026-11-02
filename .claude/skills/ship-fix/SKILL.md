---
name: ship-fix
description: Finish an ORBIS change - test, commit with the project's message style, push to master, reinstall the binary, check CI, close the issue. Use after a fix or feature is done, or when asked to commit/push/ship/release.
---

# Ship a fix

1. `git pull --rebase` first (the repo may be cloned on two machines).
2. `cargo test` - the **whole suite must pass** before a push. A failure: find the root cause, do not patch the symptom.
3. Commit straight to `master` (no branches, no PRs). Message: English, imperative subject, <= ~72 chars, no trailing dot, bare `Update`/`wip` forbidden; body explains *why*. Existing history uses Conventional-Commit-style prefixes (`fix(material):`); the repo CLAUDE.md says Conventional Commits, the workspace file says no prefix - **repo CLAUDE.md wins**, follow `git log`.
   - No personal data (name, e-mail, address) in files, commit messages or issues. Never touch the git author identity; no `Co-Authored-By`.
   - Reference issues: `refs #N`, close with `Closes #N`.
4. If the export changed: record the rule in `docs/spec/octane-export.md` in the same commit (spec is normative); run `lab-note` if it came from a calibration session.
5. `git push`, then `gh run list -L 1` / `gh run watch`; fix a red CI at once.
6. **If the export changed: `scripts/install.sh`** - the DAZ script exports with `~/Library/Application Support/ORBIS/bin/orbis`, not `target/release`. Tell the user to re-export.
7. `scripts/dev/clean-cache.sh` if a compare loop ran.
8. Found bugs/TODOs on the way that are out of scope? `gh issue create` (English), do not fix on the side.

Never without asking: `git push --force`, history rewrites, branch deletion, `gh repo delete`, `gh release delete`.
A release-worthy atomic result: offer `gh release create` (a `v*` tag makes CI attach the zip).
