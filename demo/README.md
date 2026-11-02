# Demo scenes

Hand-made DAZ scenes (prefix `Demo-`) for testing the export on realistic
content, beside the generated `Calib-*` scenes. They wear DAZ Store content
but a `.duf` only references it: the originals stay in the user's My Library
(the master copy), git holds a sanitised copy of the source and what is
rendered from it. Never the assets, dumps, `.mi` or exports:

- `scenes/Demo-*.duf` - the scene source (a copy: contributor name blanked by
  `demo.py sync`; no assets, textures or geometry).
- `scenes.toml` - the manifest: what each scene is for, issue, products it
  needs (names only).
- `references/iray/*.png` + `renders.tsv` - Iray Server references, with the
  sha256 of the scene each was rendered from.

A `Demo-` name is the user's consent to commit its source and renders to this public
repo. Scripts only read these scenes; a re-saved scene makes its reference
stale (`python3 scripts/dev/demo.py check`). Skill: `.claude/skills/demo-scene`.
