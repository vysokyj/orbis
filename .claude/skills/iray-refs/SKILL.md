---
name: iray-refs
description: Keep Iray reference renders of generated calibration scenes in git via keep-refs.py and renders.tsv. Use after Iray renders finish, or when asked to store/commit references.
---

# Iray references into git

`python3 scripts/dev/keep-refs.py [--renderer NAME] [--queue-log LOG] <render.png>...`
copies renders into `calib/references/iray/` and records samples/renderer/time in `renders.tsv`.

Rules:
- **Only scenes that exist in `calib/scenes`** (generated, DAZ built-ins, primitives and free DAZ add-ons). The script skips everything else - never work around it.
- `Demo-*` scenes (user's hand-made, store content): `keep-refs.py --demo`, into `demo/references/iray` - see the `demo-scene` skill. Only when listed in `demo/scenes.toml`.
- Never commit private scenes (teens, Gladiator, copies like Calib-Hair-Teens), store assets, or scene files with store content. Renders (not data) of Calib-Skin-* / Calib-Hair-* may go in.
- Commit **as renders arrive during a long run**, not at its end.
- References live in `~/Library/Caches/orbis/calib/iray` (5000 samples from Iray Server) and `iray-400` (400 samples); `calib/references/iray` is the git copy.
