---
name: mi-fixture
description: Add or refresh DAZ .mi (Iray MDL) fixtures that verify src/iray.rs. Use when a DAZ channel -> Iray conversion is in question, or a new shader/scene needs verifying.
---

# .mi fixtures (the only thing that may change `src/iray.rs`)

`src/iray.rs` reproduces what DAZ hands Iray, tested against `calib/references/mi/*.json`. It outranks calibration (#62).

1. Export: `DONE_FILE=<out dir>/export-mi.done scripts/dev/daz-script.sh $PWD/scripts/dev/daz-export-mi.dsa <out dir> <scene.duf>... --` (headless DAZ - ask first).
   Many scenes per run are fine (#153): the script waits 2 s after each scene for DAZ to release its Iray data, checks that a scene's shader bricks reached the `.mi` (else reopens it, then removes the `.mi`), and `export-mi.done` starts with `FAILED <n>` when a scene still lost them - re-export those alone.
2. Inspect the MDL arguments: `python3 scripts/dev/mi-args.py [--param <regex>] <scene.mi>...` (Calib-Map-* scenes sweep channels).
3. Fixtures: `python3 scripts/dev/mi-oracle.py --out calib/references/mi <scene.mi>...` - it refuses (exit 1) a `.mi` whose surfaces lost their scene's shader brick to an Uber; `python3 scripts/dev/mi-audit.py` checks every fixture that way (#153).
4. Shader defaults: `python3 scripts/dev/daz-defaults.py <definition.dsf> > calib/references/daz-defaults/<Name>.json`.
5. Geometry check: `python3 scripts/dev/mi-geometry.py <scene.mi> <export.usda>` (export with `-o <export.usda>`).
6. Shader preset sweeps: `scripts/dev/daz-build-shader-scene.dsa`, `make-mi-shader-scenes.py`.
7. Update `src/iray.rs`, `cargo test` (the module tests every fixture). Read `docs/daz/iray-uber-mdl.md` and DAZ's `irayubermaterial.mdl` first (see `docs-first`).

## Visibility: always test a channel hidden AND shown
DAZ's export reads each channel's saved UI visibility (`"visible": false`), and our generated scenes inherit DAZ's default material with 77 of 118 Uber channels hidden - product content mostly has them shown. A fixture from generated spheres alone proves only the hidden case (#52: the SSS tint, gloss / backscatter anisotropy, the coat curve). So:
1. **Every new channel sweep gets a `-Shown` twin**: `uber_spheres(...)`, then load the written scene, drop every `visible` key, save as `<name>-Shown.duf` (pattern: `Calib-Mi-Visible` in `make-calib-scenes.py`). Export both `.mi`, diff with `mi-args.py`; both become fixtures.
2. **A difference = a gate**: find what opens it (another channel's value can, e.g. Glossy Roughness > 0 opens hidden gloss anisotropy) with one-change-per-sphere variants, put the proving spheres in the generator (not a scratch scene), and record the gate in `docs/daz/iray-uber-mdl.md` (Channel visibility).
3. **A product material disagrees with a sphere of the same values?** Export the product scene's `.mi` (local only), copy its whole channel list - definitions included - onto a sphere, and bisect groups back to the sphere's until the difference goes; check the `visible` fields first.
4. Only Iray Uber needs the twin: brick shaders (PBRSkin, Blended Dual Lobe Hair, OmniHair) take visibility from their `.dsf` definition, and scenes save none for them - check that with a scene audit before assuming it for a new shader.
5. Never `make-calib-scenes.py --force` (it rewrites every generated scene); delete the one `.duf` and rerun.
