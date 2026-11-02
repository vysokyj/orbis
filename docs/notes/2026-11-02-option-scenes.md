# 2026-11-02 — option scenes, Demo- scenes, workflow automation

Goal: prepare Iray references ahead of time for options the export does not
cover yet, and make the working procedures repeatable (skills, hooks, agents).
Issues: #35 #37 #38 #43 #65, batch tracking #68. Variant: enhanced-final.

## New scenes (`scripts/dev/make-option-scenes.py`)

40 scenes derived from existing generated scenes (built-ins only, no DAZ):

| Family | Base | Scenes |
|---|---|---|
| `Calib-Tone-*` | Calib-Tone-Default | Vignette05, Vignette1, BurnComp-off/-on, Sat0, Sat15, WhiteWarm, WhiteScale2 |
| `Calib-Env-*` | Calib-Tone-Default | Tint, Rot90, Rot180, OrientX30, Blur, Res128, Intensity2, Ground, GroundMatte, MatteFog, GroundFog, DomeMode1, DomeMode2 |
| `Calib-RS-*` | Calib-Glass | Caustics, PathLen2/4, FireflyOff, Bloom, PixelBox, PixelRadius3, Spectral |
| `Calib-L-Opt-*` | Calib-L-Point-5k | Decay0/1, ShadowInt0/05, ShadowSoft05, ShadowColor, IllumDiffuse/Specular, EmitterOff, CastShadowsOff, Temp3000 |

Channel ids come from the base scenes' own tonemapper / environment / light
nodes and render settings; the generator raises if one is missing.
**Unverified guesses** (settle with the first render or a `.mi` export): the
meaning of `Dome Mode` 1 / 2 and `Illumination` 1 / 2.

## Timings

- Headless DAZ dumps (`daz-dump.sh`, now with `-headless`): 3–4 s per scene,
  40 of 40 ok. DAZ opened every new scene, so they are valid scenes.
- Octane, enhanced-final, 400 samples: about 30 s per scene (40 scenes).
- Iray Server, 5000 samples: 105–112 s for most (Env-*, L-Opt-*), Env-Ground
  175 s, Env-GroundFog 302 s, L-Opt-Temp3000 306 s, **RS-Bloom 1433 s**
  (a low bloom threshold makes Iray slow; expect Caustics and Spectral to be
  slow as well). The stuck Calib-Eye-G9 finally rendered in 1875 s.

## Ratios

Not yet: the Iray references were still rendering when this note was written.
Compare with `calib-compare-all.py` and the `calib-analyst` agent once they
are in, then extend this note.

## Findings and process lessons

- **Headless DAZ without a window** works with `-headless -noDefaultScene`
  started directly (not via `open`); `.mi` export on Calib-Base in 8 s, 0
  on-screen windows (#63).
- **A DAZ menu action keeps an absolute script path.** `Render > ORBIS Iray
  Queue` still pointed at `~/workspace/orbis/...` after the repo moved to
  `~/GitHub/orbis`; DAZ logged "Could not find script file". Stopgap: a
  symlink at the old path. Proper fix: install the queue script under
  `<library>/Scripts/ORBIS/` and point the menu there.
- **Cargo's target dir is not `<repo>/target`** on this machine
  (`/Volumes/Data/Cargo`); scripts now ask `scripts/dev/target-dir.sh`.
  After a repo move, touch the sources: `env!("CARGO_MANIFEST_DIR")` in the
  tests is baked at compile time (7 tests failed on a stale path).
- `--help` used to be taken as an output folder by 20+ scripts (created
  `--help/compare`); every script now prints its usage.
- A broken PreToolUse hook blocks every tool it matches, including the ones
  needed to fix it; fix it with a tool the matcher does not cover. `case`
  patterns cannot come from a variable holding `a|b` — use a function.
- Hooks ask for render runs once per session (marker written by a
  PostToolUse call), not on every call.
- A new scene needs its own dump even when it only changes render settings
  (`orbis export` stops with "no scene dump").

## Where it stands

Iray batch of 41 scenes (40 above + Demo-Gladiator) in the user's queue;
Octane renders of the 40 are ready; references go into git through
`keep-refs.py` as they arrive (`--demo` for Demo-Gladiator). Open: the
comparison per family, the queue-script menu path, #68.
