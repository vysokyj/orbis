---
name: mi-fixture
description: Add or refresh DAZ .mi (Iray MDL) fixtures that verify src/iray.rs. Use when a DAZ channel -> Iray conversion is in question, or a new shader/scene needs verifying.
---

# .mi fixtures (the only thing that may change `src/iray.rs`)

`src/iray.rs` reproduces what DAZ hands Iray, tested against `calib/references/mi/*.json`. It outranks calibration (#62).

1. Export: `DONE_FILE=<out dir>/export-mi.done scripts/dev/daz-script.sh $PWD/scripts/dev/daz-export-mi.dsa <out dir> <scene.duf>... --` (headless DAZ - ask first).
2. Inspect the MDL arguments: `python3 scripts/dev/mi-args.py [--param <regex>] <scene.mi>...` (Calib-Map-* scenes sweep channels).
3. Fixtures: `python3 scripts/dev/mi-oracle.py --out calib/references/mi <scene.mi>...`
4. Shader defaults: `python3 scripts/dev/daz-defaults.py <definition.dsf> > calib/references/daz-defaults/<Name>.json`.
5. Geometry check: `python3 scripts/dev/mi-geometry.py <scene.mi> <export.usdz>`.
6. Shader preset sweeps: `scripts/dev/daz-build-shader-scene.dsa`, `make-mi-shader-scenes.py`.
7. Update `src/iray.rs`, `cargo test` (the module tests every fixture). Read `docs/daz/iray-uber-mdl.md` and DAZ's `irayubermaterial.mdl` first (see `docs-first`).
