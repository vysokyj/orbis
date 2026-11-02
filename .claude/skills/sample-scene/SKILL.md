---
name: sample-scene
description: Work with the user's hand-made Sample-* DAZ scenes (realistic test scenes with store content) - register them, keep their Iray references in git without the scene, spot stale references. Use when a Sample-* scene is mentioned, added, rendered or compared.
---

# Sample scenes

`Sample-<Name>` is a scene the user built by hand in DAZ to test the export on realistic content (e.g. a gladiator with armour, hair, skin). Unlike generated `Calib-*`, it wears store content, so the public repo gets only what is derived:

| In git (`sample/`) | Never in git |
|---|---|
| `scenes.toml` manifest (purpose, issue) | the `.duf`, dumps, `.mi`, `.usdz`/`.orbx` exports |
| `references/iray/*.png` + `renders.tsv` (with `scene_sha256`) | store asset names beyond the product list in the manifest purpose |

The `Sample-` prefix = the user's consent to publish its renders. A scene without the prefix is private (no renders either). Never rename or copy a private scene to `Sample-` ourselves.

## Rules (no overwriting each other)
- **The scene is the user's.** It lives in `My Library/Scenes/Sample-*.duf` (`ORBIS_LIBRARY` overrides the root). Scripts and Claude only read it; the hook blocks edits. `iray-queue.sh` renders a capped copy, the original stays untouched.
- **Our files are ours.** Generators write only `Calib-*`; `sample/` holds only README, `scenes.toml`, reference PNGs and `renders.tsv` (the hook rejects other files and any staged `.duf` outside `calib/scenes`; `.gitignore` ignores `*.duf` there).
- **A re-saved scene stales its reference.** `python3 scripts/dev/sample.py check` compares the scene's sha256 with the one in `renders.tsv`. `keep-refs.py --sample` skips a render older than the scene file. Never compare a new Octane render with a stale reference: queue a new Iray render first (`iray-queue` skill, part of the next batch).

## Steps
1. Register: `python3 scripts/dev/sample.py add Sample-<Name> "<what it tests>" [issue]` (the user built the scene; commit the manifest).
2. Iray reference: batch it via `iray-queue`; keep: `python3 scripts/dev/keep-refs.py --sample [--renderer NAME] [--queue-log LOG] <render.png>...`, commit with the `.tsv` as renders come.
3. Dump + export + Octane + compare: as in `calib-loop` (small scenes first; big scenes freeze the Mac: half resolution, no render series). Clean the cache after.
4. `python3 scripts/dev/sample.py list` shows each scene with fresh/STALE.
5. Findings -> `lab-note`; rules -> the spec. Mention scenes by manifest name only, never reproduce store product details.
