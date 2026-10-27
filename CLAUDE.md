# ORBIS — CLAUDE.md

## What it is
ORBIS (Octane Render Bridge for Iray Scenes): DAZ Studio scenes → Octane X
(`.orbx`). A Rust crate (`orbis`) with the `orbis` CLI, plus a DAZ Studio
script that runs it; usable as a library by other tools. Open source
(GPL-3.0-or-later). macOS first. The name nods to Comenius's *Orbis
sensualium pictus* (1658).

## Build / test / run
```sh
cargo test                                     # unit tests (no DAZ content needed)
cargo build --release                          # target/release/orbis
cargo run --release -- export <scene.duf> -o <out.orbx|out.usdz> [--preview] [--no-ground] [--look enhanced|faithful] [--no-denoise]   # needs a cached dump
scripts/install.sh                             # build + `orbis install` into "My Library"
scripts/package.sh                             # plugin zip → dist/ (git-ignored)
scripts/dev/render-icons.sh                    # assets/icon/*.svg → committed thumbnails
python3 scripts/dev/fetch-octane-docs.py docs/octane/manual <cache dir>   # Octane manual mirror (git-ignored)
scripts/dev/daz-render.sh <scene.duf> <out.png> [secs] [samples]   # unattended capped Iray render (own DAZ instance)
scripts/dev/octane-run.sh <probe.lua> [scene.orbx]   # unattended Octane X Lua probe (starts/quits Octane X)
scripts/dev/octane-render.sh <out dir> [samples] <scene.orbx>...   # unattended Octane renders of exports, as exported
python3 scripts/dev/make-calib-scenes.py   # Calib-PBRSkin / Calib-Tone-* / Calib-DOF-* from Calib-Materials.duf
python3 scripts/dev/make-light-scenes.py   # Calib-L-* light scenes from Calib-Point/-Area/-Spot/-Distant/-Emissive
python3 scripts/dev/calib-render.py [--auto-ev] <scene.duf>...   # Iray references → ~/Library/Caches/orbis/calib/iray
DONE_FILE=<out>.done scripts/dev/daz-script.sh scripts/dev/daz-build-leaf-scene.dsa <Calib-L-Distant-45.duf> <Iray Uber Base.duf> <out>   # Calib-Leaf (then make-calib-scenes.py tunes its cards)
DONE_FILE=<out>.done scripts/dev/daz-script.sh scripts/dev/daz-build-sundir-scene.dsa <Calib-L-Mode-SunSky-off.duf> <out>   # Calib-L-Mode-SunSky-dir (sun steered by a node)
scripts/dev/daz-dump.sh <out dir> <scene.duf>...   # headless DAZ dump (the ORBIS script's --dump) + orbis export
python3 scripts/dev/orbx-variants.py <in.orbx> <out dir> <hypothesis>...   # material-pin variants of an export (gloss-2r, gloss-ior, dl-ior)
scripts/dev/calib-variants.sh all -- <scene>...   # export (cached dumps) + render + report per variant (Look × mode)
scripts/dev/calib-octane-batch.sh   # Octane side of the calibration: render all exports + variants, report vs Iray
python3 scripts/dev/channel-audit.py > audit.md   # DAZ channels set in My Library's scenes vs. read by the export (#15)
python3 scripts/dev/shader-coverage.py > coverage.md   # every Iray Uber / PBRSkin / DAZ Default channel: default, read or not (#15)
```
CI (`.github/workflows/ci.yml`) tests and packages on every push; a `v*` tag
attaches the zip to a GitHub release.

## Layout
| Path | Purpose |
|------|---------|
| `src/main.rs` | `orbis` CLI (`export`, `install`); stdout = progress/report read by the DAZ script, logs on stderr |
| `src/install.rs` | `orbis install`: binary → `~/Library/Application Support/ORBIS/bin/`, script (paths filled in) + thumbnails → `<library>/Scripts/ORBIS/` |
| `src/usd/` | Scene → meshes (`.duf` + library `.dsf` + the scene dump, `dump.rs`), SubD, hair, cameras, lights, environment, USDA/USDZ, `project.ocs`; `export_scene_orbx` / `export_scene_usdz` |
| `src/tuning.rs` | The four export variants (Look × render mode) and every value that differs between them, one table |
| `src/material.rs` | Iray Uber → Octane material/light/scene nodes in `project.ocs` |
| `src/daz_material.rs` | DSON materials: merged Iray channels |
| `src/layered_image.rs` | DAZ Layered Image Editor stacks → flattened images |
| `src/texture_budget.rs` | Texture memory budget, downscaling |
| `src/orbx.rs` | ORBX container writer/reader (reverse-engineered) |
| `src/dson.rs` | gzip DSON, percent-decoding |
| `src/daz.rs` | DAZ Studio on this machine: content dirs, app bundle |
| `assets/octane/*.ocs` | `project.ocs` template + kernels, embedded with `include_str!` |
| `daz/Scripts/ORBIS/` | DAZ Studio script (options dialog, scene dump, menu action) + thumbnails; a template — `orbis install` fills in `@…@` |
| `assets/icon/` | Thumbnail sources: `globe.svg` (folder), `orbx.svg` (script) |
| `scripts/package.sh` | Builds the release zip: the (universal) binary + docs |
| `scripts/dev/` | Calibration tools: Octane Lua probes, render comparison, manual mirror |
| `calib/scenes/` | Calibration scenes the scripts generated (Calib-PBRSkin, -Glossy, -Tone-*, -DOF-*, -L-*): copies of `My Library/Scenes`, exposure as tuned; DAZ built-ins and primitives only |

## Documentation
Map of all docs: `docs/README.md`. Calibration (principle, test scenes,
the loop, references in `calib/references`): `docs/calibration.md`.
- `docs/spec/octane-export.md` — **normative**: what the export does and why
  (calibration numbers included). Code that disagrees is a bug, or fix the spec
  in the same commit. Record confirmed calibration findings right away.
- `docs/octane/` (Octane X, ORBX, USD) and `docs/daz/` (files, Iray, content)
  — reference about other tools, no Orbis decisions in them.
- `docs/octane/manual/` — local mirror of OTOY's manual. **Git-ignored and
  must stay so** (copyrighted). Read it before guessing at Octane behaviour.

## Conventions
- Everything in the repo in English; Conventional Commits; push to `master`.
- `cargo fmt` is not enforced (the code uses long lines); match the surrounding style.
- Plans and to-dos go into GitHub issues, not docs.

## Watch out for
- `calib/scenes/` holds only generated scenes — never the user's own
  Calib-*.duf (Base, Point, Area, Spot, Distant, Emissive, HDRI, Materials)
  nor Calib-Hair (it wears a DAZ Store hair): copy a new one in with its
  references checked (DAZ built-ins / primitives only).
- **Nothing third-party-copyrighted in the repo**: no OTOY manual text, no
  Octane sample files (the ORBX round-trip test reads `cornell.orbx` from the
  installed app and skips without it), no DAZ store content in fixtures —
  tests use synthetic scenes.
- The posed geometry comes from our own scene dump written by the DAZ
  script (format in the spec). Script and `orbis` share `format_version` —
  bump both on an incompatible change. No third-party exporter (Diffeomorphic
  `.dbz`) is part of the project; streaming instead of a file is #2.
- DAZ Script API: `docs/daz/script-api.md`. Statics like `DzDir.homePath()`
  fail in DAZ Studio 6; keep string literals in the script ASCII. DAZ's log
  (`~/Library/Application Support/DAZ 3D/Studio6/log.txt`) shows script errors.
- The DAZ script is ECMAScript 3-ish QtScript: `var`, no arrow functions or
  `let`/`const`. It is untested in DAZ Studio until #1 is closed.
- Octane X is sandboxed and refuses files opened from outside — don't try to
  launch it with a file; the user opens the `.orbx` from its File → Open.
- The binary is ad-hoc signed until #3; a downloaded copy is quarantined.
- Automation: never run a DAZ render and an Octane probe at once (Iray takes
  the whole CPU; both scripts refuse). `octane-run.sh` needs an unlocked
  screen — behind a lock Octane X never leaves its simple mode (no Script
  menu). DAZ renders fine headless; App Nap is off for DAZ
  (`NSAppSleepDisabled`), else it crawls in the background.
