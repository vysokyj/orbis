# ORBIS — CLAUDE.md

## What it is
ORBIS (Octane Render Bridge for Iray Scenes): DAZ Studio scenes → Octane X
(`.orbx`). A Rust crate (`orbis`) with the `orbis` CLI, plus a DAZ Studio
script that runs it; usable as a library by other tools. Open source
(GPL-3.0-or-later). macOS first. The name nods to Comenius's *Orbis
sensualium pictus* (1658).

## Build / test / run
```sh
cargo test                 # unit tests (no DAZ content needed)
cargo build --release      # target/release/orbis
scripts/install.sh         # build + `orbis install` into "My Library"
scripts/package.sh         # plugin zip -> dist/ (git-ignored)
cargo run --release -- export <scene.duf> -o <out.orbx|out.usdz|out.usda> [--preview] [--no-ground] [--look enhanced|faithful] [--no-denoise]   # needs a cached dump
```
Everything else (calibration, DAZ/Octane runs, Iray queue, `.mi` fixtures,
references, lab notes) is a **project skill** in `.claude/skills/` - use it
instead of remembering the commands: `calib-loop`, `ship-fix`, `lab-note`,
`iray-refs`, `demo-scene`, `iray-queue`, `mi-fixture`, `octane-probe`, `daz-script-dev`,
`docs-first`, `progress`, `issue-loop`. Subagents in `.claude/agents/` (`calib-analyst`,
`spec-guard`, `doc-researcher`, `issue-worker`); the
ask-first rules below are kept by Claude itself (no hooks). The scripts are in `scripts/dev/` (each has its usage
in its header). The repo is location-independent: scripts find each other
via their own path; keep it that way (no
absolute home paths).
CI (`.github/workflows/ci.yml`) runs only on a `v*` tag (or by hand): tests,
packages and attaches the zip to a GitHub release. Pushes don't run it, so
never wait for CI - the local `cargo test` before a push is the gate.

## Layout
| Path | Purpose |
|------|---------|
| `src/main.rs` | `orbis` CLI (`export`, `install`); stdout = progress/report read by the DAZ script, logs on stderr |
| `src/install.rs` | `orbis install`: binary → `~/Library/Application Support/ORBIS/bin/`, script (paths filled in) + thumbnails → `<library>/Scripts/ORBIS/` |
| `src/usd/` | Scene → meshes (`.duf` + library `.dsf` + the scene dump, `dump.rs`), SubD, hair, cameras, lights, environment, the USD layer (binary `usdc.rs`, text `usda.rs`), USDZ, `project.ocs`; `export_scene_orbx` / `export_scene_usdz` |
| `src/iray.rs` | DAZ channels → what DAZ hands Iray (materials, lights, camera, tone mapping, environment), verified against DAZ's .mi export (`calib/references/mi`) |
| `src/tuning.rs` | The four export variants (Look × render mode) and every value that differs between them, one table |
| `src/material.rs` | Iray Uber → Octane material/light/scene nodes in `project.ocs` |
| `src/daz_material.rs` | DSON materials: merged Iray channels |
| `src/layered_image.rs` | DAZ Layered Image Editor stacks → flattened images |
| `src/texture_budget.rs` | Texture memory budget, downscaling |
| `src/mdl_layer.rs` | NVIDIA MDL layer stacks (resin, architectural glass): closed sphere in a white furnace traced under `libbsdf` and Octane, the tint factor that matches them |
| `src/detail_normal.rs` | Uber height maps (+ normal map) baked into one tangent-space normal map as MDL's `file_bump_texture` / `blend_normals` |
| `src/bake.rs` | Per-texel baking: exact 8-bit decode / encode tables, parallel rows |
| `src/vdb.rs` | OpenVDB header: grid names |
| `src/ies.rs` | IES (LM-63) light profiles: candela table, power |
| `src/orbx.rs` | ORBX container writer/reader (reverse-engineered) |
| `src/dson.rs` | gzip DSON, percent-decoding |
| `src/daz.rs` | DAZ Studio on this machine: content dirs, app bundle |
| `assets/octane/*.ocs` | `project.ocs` template + kernels, embedded with `include_str!` |
| `daz/Scripts/ORBIS/` | DAZ Studio script (options dialog, scene dump, menu action) + thumbnails; a template — `orbis install` fills in `@…@` |
| `assets/icon/` | Thumbnail sources: `globe.svg` (folder), `orbx.svg` (script) |
| `scripts/package.sh` | Builds the release zip: the (universal) binary + docs |
| `scripts/dev/` | Calibration tools: Octane Lua probes, render comparison, manual mirror |
| `demo/` | `Demo-*` scenes: manifest (`scenes.toml`) + Iray references with the scene's sha256; the scenes stay in My Library |
| `calib/scenes/` | Calibration scenes the scripts generated (Calib-PBRSkin, -Glossy, -Tone-*, -DOF-*, -L-*): copies of `My Library/Scenes`, exposure as tuned; DAZ built-ins, primitives and free DAZ add-ons only |

## Documentation
Map of all docs: `docs/README.md`. Calibration (principle, test scenes,
the loop, references in `calib/references`): `docs/calibration.md`.
- `docs/spec/octane-export.md` — **normative**: what the export does and why
  (calibration numbers included). Code that disagrees is a bug, or fix the spec
  in the same commit. Record confirmed calibration findings right away.
- `docs/notes/` — dated lab notebook: each calibration session in detail
  (tries, dead ends, numbers, timings). Not normative; write one per session.
- `docs/octane/` (Octane X, ORBX, USD) and `docs/daz/` (files, Iray, content)
  — reference about other tools, no Orbis decisions in them.
- `docs/octane/manual/` — local mirror of OTOY's manual. **Git-ignored and
  must stay so** (copyrighted). Read it before guessing at Octane behaviour.
- `docs/nvidia/` — local mirror of NVIDIA's Iray and MDL docs (PDF + text).
  **Git-ignored and must stay so.** With DAZ's own MDL sources
  (`DAZStudio.app/Contents/shaders/iray/daz_3d/irayubermaterial.mdl` etc.)
  it says what Iray does: read them before fitting a channel by calibration
  (Uber's layer stack: `docs/daz/iray-uber-mdl.md`).

## Conventions
- Everything in the repo in English; Conventional Commits; push to `master`.
- `cargo fmt` is not enforced (the code uses long lines); match the surrounding style.
- Plans and to-dos go into GitHub issues, not docs.

## Working agreements (rules the user set for Claude)
- **Ask before rendering.** One short question right before any Octane
  render/probe (`octane-render.sh`, `octane-run.sh`) or headless DAZ run
  (`daz-script.sh`, `daz-dump.sh`, `make-skin-*`): the render lock only
  covers our scripts, not what the user starts by hand (an earlier batch
  interrupted one). "It's free, don't ask" holds for that app for that
  session only. Otherwise Claude may start, quit and kill Octane X / DAZ
  Studio freely; when the user says they work in an app, hands off.
- **No local CPU Iray** (`daz-render.sh`) without asking — it takes the whole
  CPU. Octane may never render alongside it.
- **The issue loop doesn't ask** (skill `issue-loop`, decided 2026-11-11):
  working through issues unattended, headless DAZ and Octane renders /
  probes start without the question above (the render lock, "hands off
  when the user works in an app" and "leave a failed Octane open" still
  hold). Each issue is worked by the `issue-worker` agent (Opus), one at
  a time. An issue that needs Iray is not a call to the user: prepare its
  scenes, write the batch into the issue (`## Iray batch`), label it
  `needs-iray` and take the next issue. The user's "server runs" queues
  every parked batch at once.
- **Iray only on the Iray Server, in big batches** (skill `iray-queue`):
  prepare the whole batch first, then one call to the user to start the
  server; queue the announced batch after their "server runs", never reorder,
  never add unannounced scenes. Octane may render alongside it. Stalled queue
  / "session handler rejected the session" = tell the user.
- **A failed Octane render/probe: leave Octane X open** so the user can read
  its error dialog (#50). Big scenes (the teens scene, 1.2 M faces) in a
  series of full renders froze the Mac — experiment on small Calib-* scenes
  or half resolution.
- **Clean up the cache**: `.orbx` exports in `~/Library/Caches/orbis` filled
  107 GB. After each export → render → compare loop,
  `find ~/Library/Caches/orbis -name '*.orbx' -delete`; keep Iray
  references, dumps, PNG renders and reports (re-export from cached dumps).
- **Reinstall after an export fix**: the DAZ script exports with
  `~/Library/Application Support/ORBIS/bin/orbis`, not `target/release`.
  Run `scripts/install.sh` after committing and tell the user to re-export.
- **Octane probes** run from Octane's sandbox: `scripts/dev/octane-run.sh`
  copies `scripts/dev/octane-probe.lua` to
  `~/Library/Containers/com.otoy.rndrviewer/Data/Documents/ORBIS Scripts/ORBIS probe.lua`;
  `os.getenv("HOME")` there is the container, so probe logs/PNGs land in
  `~/Library/Containers/com.otoy.rndrviewer/Data/Documents/`.
- **Lab notes**: every calibration session gets a dated note in `docs/notes/`
  (all attempts incl. failed ones, ratio tables, timings, process lessons),
  written as findings land and committed with the fix, linked from
  `docs/notes/README.md`. The spec keeps only the winning rule.
- **Octane over Iray**: match Iray's light amounts and what the scene means,
  but where Octane is more plausible (warm low sun, gloss layering, soft spot
  edges) leave it — 100 % Iray compatibility is not the goal (spec, Look).
- **PBRSkin gets a direct OpenPBR mapping** (decided 2026-10-31, #57), not
  the Uber-fitted skin rules (skin_albedo, volume_factor, skin_tint,
  SKIN_HUE). After the Uber base skin (#28): read `pbr_skin.mdl`, map
  parameters directly, no fitted constants, verify on 2–3 G9 figures (±15 %).
  #19 (skin hue) stalled for missing knowledge — don't resume by tuning
  constants.
- **Decompose before fitting** (the user's rule, 2026-11-07): never fit a
  coefficient to a whole render. First split the result into its
  components - Iray variants that switch one part off (w0, tcwhite,
  dwhite, gloss off), Octane probes with a known input (a grey, albedo 0,
  a plain diffuse) - and check each against the MDL / Octane docs. Fit only
  what is still unexplained after that, and say in the code why. #19's
  fitted hue, dark gain, (1 - w)^1.3 and 0.55 share each hid a missing
  component (the volume's albedo, a bounce limit, the diffuse layer, an
  unmapped gloss lobe).
- **Language**: the user writes Czech (informal); everything in the repo is
  English.

## Watch out for
- **Scenes by prefix** (skill `demo-scene`): `Calib-*` = generated from
  DAZ built-ins, scene + renders in git (`calib/`). `Demo-*` = the user's
  hand-made realistic scenes (store content): the **scene source** (the master
  copy in `demo/scenes/`, contributor name blanked; git wins: `demo.py restore`
  before use, `demo.py update` only when the user says "update"), the
  manifest and the Iray reference renders go in git (the user's decision,
  2026-11-02); the prefix is their consent. **Never** the assets themselves
  or what is derived from their geometry/materials: dumps, `.mi`, `.usdz`,
  `.orbx`. Anything else (teens, Calib-Hair-Teens...) is private: neither
  scene nor renders in git; use it freely for local calibration. Claude never
  edits a `Demo-*.duf` by hand. References go in through
  `keep-refs.py` (`--demo` for `Demo-*`), committed as they come during a
  long run; `demo.py check` shows stale references and a library file that differs from git.
- **Verified DAZ → Iray conversions outrank calibration.** `src/iray.rs`
  reproduces DAZ's own `.mi` export (`calib/references/mi/*.json`, test in
  the module). Never change it to fit a render; tune the Octane side
  instead. It changes only with new `.mi` fixtures that prove it (#62).
- `calib/scenes/` holds only generated scenes — never the user's own
  Calib-*.duf (Base, Point, Area, Spot, Distant, Emissive, HDRI, Materials)
  nor Calib-Hair (it wears a DAZ Store hair): copy a new one in with its
  references checked (DAZ built-ins / primitives only, or a free DAZ
  add-on such as dForce Starter Essentials — the scene only references it,
  e.g. Calib-Mi-Hair; decided 2026-11-03).
- Iray references go into git (`calib/references/iray`, with `renders.tsv`)
  as they are rendered, through `keep-refs.py` — only renders of
  `calib/scenes` (see the private-scenes rule above).
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
- Signing: `scripts/sign.sh` (used by `install.sh` and `package.sh`) signs
  with the keychain's Developer ID Application identity (hardened runtime),
  else ad hoc; never put the identity's name or team in the repo. The release
  zip isn't notarized until CI does it (#3), so a downloaded copy is still
  quarantined.
- Automation: never run a DAZ render and an Octane probe at once (Iray takes
  the whole CPU). DAZ Studio and Octane X may both stay open; `daz-render.sh`,
  `daz-script.sh` and `octane-run.sh` take turns through one lock
  (`scripts/dev/render-lock.sh`, `~/Library/Caches/orbis/render.lock`) and
  wait for it — don't wait on `pgrep` for Octane X or DAZ to quit, call the
  scripts (or `render_lock` from them). `octane-run.sh` needs an unlocked
  screen — behind a lock Octane X never leaves its simple mode (no Script
  menu). DAZ renders fine headless; App Nap is off for DAZ
  (`NSAppSleepDisabled`), else it crawls in the background.
