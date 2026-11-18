---
name: iray-refs
description: Keep Iray reference renders of generated calibration scenes in git via keep-refs.py and renders.tsv. Use after Iray renders finish, or when asked to store/commit references.
---

# Iray references into git

`python3 scripts/dev/keep-refs.py [--renderer NAME] [--queue-log LOG] <render.png>...`
copies renders into `calib/references/iray/` and records samples/renderer/time/denoiser in `renders.tsv`. The renderer comes from the capped copy (queue `done/` = Iray Server, else local CPU; `--renderer` only to override); a CPU render is recorded denoiser `off` - the Iray Server applies DAZ's Post Denoiser, the CPU Iray can't (#171).

Rules:
- **Only scenes that exist in `calib/scenes`** (generated, DAZ built-ins, primitives and free DAZ add-ons) **or `calib/scenes-store`** (the user's store content, 2026-11-18). The script skips everything else - never work around it.
- `Demo-*` scenes (user's hand-made, store content): `keep-refs.py --demo`, into `demo/references/iray` - see the `demo-scene` skill. Only when listed in `demo/scenes.toml`.
- Never commit store assets or data derived from them (dumps, `.mi`, `.orbx`, `.usdz`). Scenes with store content go only into `calib/scenes-store` (contributor blanked) and only with the user's yes for a new one; Gladiator stays out.
- Commit **as renders arrive during a long run**, not at its end.
- References live in `~/Library/Caches/orbis/calib/iray` (5000 samples from Iray Server) and `iray-400` (400 samples); `calib/references/iray` is the git copy.
