# Demo scenes

Hand-made DAZ scenes (prefix `Demo-`) for testing the export on realistic
content, beside the generated `Calib-*` scenes. **The scenes themselves are
not here**: they wear DAZ Store content and stay in the user's My Library.
Git holds only what is derived from them:

- `scenes.toml` - the manifest: what each scene is for, issue, products it
  needs (names only).
- `references/iray/*.png` + `renders.tsv` - Iray Server references, with the
  sha256 of the scene each was rendered from.

A `Demo-` name is the user's consent to commit its renders to this public
repo. Scripts only read these scenes; a re-saved scene makes its reference
stale (`python3 scripts/dev/demo.py check`). Skill: `.claude/skills/demo-scene`.
