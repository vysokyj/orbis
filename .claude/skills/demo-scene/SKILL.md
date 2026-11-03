---
name: demo-scene
description: Work with the user's hand-made Demo-* DAZ scenes (realistic test scenes with store content) - register them, keep their Iray references in git without the scene, spot stale references. Use when a Demo-* scene is mentioned, added, rendered or compared.
---

# Demo scenes

`Demo-<Name>` is a scene the user built by hand in DAZ to test the export on realistic content (e.g. a gladiator with armour, hair, skin). It wears store content, but a `.duf` only references it; the user decided (2026-11-02) that the **scene source and the renders go in the public repo**.

| In git (`demo/`) | Never in git |
|---|---|
| `scenes/Demo-*.duf` - sanitised copy (`demo.py update` blanks the contributor author/e-mail/website) | the store assets, textures, geometry; dumps, `.mi`, `.usdz`/`.orbx` exports (derived from store geometry/materials) |
| `scenes.toml` manifest (purpose, issue) | |
| `references/iray/*.png` + `renders.tsv` (with `scene_sha256` of the library original) | |

The `Demo-` prefix = the user's consent. A scene without it is private (neither scene nor renders in git). Never rename or copy a private scene to `Demo-` ourselves.

## Rules (git wins)
- **The git copy `demo/scenes/Demo-*.duf` is the master** (the user's rule, 2026-11-02). The library file is just what DAZ opens. Unless the user explicitly says **"update"**, we keep the git version: before using a Demo- scene (queue, render, export, compare) run `python3 scripts/dev/demo.py restore` - it puts the git copy back into the library (a differing file is backed up first to `~/Library/Caches/orbis/demo-backup/`, mtime kept).
- **"update <name>"** from the user = take the library file into git: `python3 scripts/dev/demo.py update [name]` (sanitised), then the reference is stale -> re-render in the next Iray batch.
- **Metadata cleaning is built in**: every copy that enters git is sanitised (`asset_info.contributor` author / e-mail / website blanked); `restore` also removes them from the library file. If more metadata turns up, extend `sanitized()` in `demo.py` - there is no separate skill for it.
- **Our files are ours.** Generators write only `Calib-*`; `demo/` holds only README, `scenes.toml`, `scenes/Demo-*.duf`, reference PNGs and `renders.tsv` (check `git status` before committing: no staged `.duf` outside `calib/scenes` and `demo/scenes`; `.gitignore` ignores other `*.duf`). Claude never edits a `Demo-*.duf` by hand.
- **Freshness.** `demo.py check`: the reference's `scene_sha256` (hash of the git copy) must match; `keep-refs.py --demo` skips a render older than the library file. Never compare a new Octane render with a stale reference: queue a new Iray render first (`iray-queue` skill, part of the next batch).

## Steps
1. Register: `python3 scripts/dev/demo.py add Demo-<Name> "<what it tests>" [issue]` (the user built the scene; it writes the manifest entry and the sanitised copy; commit both). After the user re-saves a scene and asks for it: `demo.py update`.
2. Iray reference: batch it via `iray-queue`; keep: `python3 scripts/dev/keep-refs.py --demo [--renderer NAME] [--queue-log LOG] <render.png>...`, commit with the `.tsv` as renders come.
3. Dump + export + Octane + compare: as in `calib-loop` (small scenes first; big scenes freeze the Mac: half resolution, no render series). Clean the cache after.
4. `python3 scripts/dev/demo.py list` shows each scene with fresh/STALE.
5. Findings -> `lab-note`; rules -> the spec. Mention scenes by manifest name only, never reproduce store product details.
