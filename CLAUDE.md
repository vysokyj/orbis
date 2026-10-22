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
cargo run --release -- export <scene.duf> -o <out.orbx|out.usdz> [--preview] [--no-ground]
scripts/package.sh                             # plugin zip → dist/ (git-ignored)
python3 scripts/dev/fetch-octane-docs.py docs/octane/manual <cache dir>   # Octane manual mirror (git-ignored)
```
CI (`.github/workflows/ci.yml`) tests and packages on every push; a `v*` tag
attaches the zip to a GitHub release.

## Layout
| Path | Purpose |
|------|---------|
| `src/main.rs` | `orbis` CLI (`export`); stdout = progress/report read by the DAZ script, logs on stderr |
| `src/usd/` | Scene → meshes (`.duf` + library `.dsf` + `.dbz`), SubD, hair, cameras, lights, environment, USDA/USDZ, `project.ocs`; `export_scene_orbx` / `export_scene_usdz` |
| `src/material.rs` | Iray Uber → Octane material/light/scene nodes in `project.ocs` |
| `src/daz_material.rs` | DSON materials: merged Iray channels |
| `src/layered_image.rs` | DAZ Layered Image Editor stacks → flattened images |
| `src/texture_budget.rs` | Texture memory budget, downscaling |
| `src/orbx.rs` | ORBX container writer/reader (reverse-engineered) |
| `src/dson.rs` | gzip DSON, percent-decoding |
| `src/daz.rs` | DAZ Studio on this machine: content dirs, app bundle, `.dbz` path |
| `assets/octane/*.ocs` | `project.ocs` template + kernels, embedded with `include_str!` |
| `daz/Scripts/ORBIS/` | DAZ Studio script(s), shipped as `Content/Scripts/ORBIS/` with `bin/orbis` |
| `scripts/package.sh` | Builds the plugin zip (universal binary when both Apple targets are installed) |
| `scripts/dev/` | Calibration tools: Octane Lua probes, render comparison, manual mirror |

## Documentation
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
- **Nothing third-party-copyrighted in the repo**: no OTOY manual text, no
  Octane sample files (the ORBX round-trip test reads `cornell.orbx` from the
  installed app and skips without it), no DAZ store content in fixtures —
  tests use synthetic scenes.
- The posed geometry still comes from a `.dbz` beside the scene; our own
  dump from DAZ Studio replaces it (#2). No third-party exporter is part of
  the project.
- The DAZ script is ECMAScript 3-ish QtScript: `var`, no arrow functions or
  `let`/`const`. It is untested in DAZ Studio until #1 is closed.
- Octane X is sandboxed and refuses files opened from outside — never try to
  hand it a file; reveal it and copy the path (`open_in_octane`).
- The binary is ad-hoc signed until #3; a downloaded copy is quarantined.
