# 2026-11-16 — Specular Layer at Beckmann: the lobe is whole, the base is not (#190)

#190 (from #175): with the Uber skin's dual lobe on Specular Layers (139)
at Beckmann, Calib-Skin-Nishi-front's face kept 0.77 × Iray's dual lobe
energy (dlflat − gloss0 summed over the face box; STD 1.05, OpenPBR's GGX
1.03), where Beckmann on Universal keeps sharp dual lobes at Iray's level.
Either Octane's Beckmann inside a Specular Layer loses energy, or it does
under the face's normal map. Decompose first, no constant. No Iray: the
probes compare Octane with Octane (or with the analytic lobe), Nishi's
references from #19 / #175 serve.

## 1. Docs

- Octane manual (Specular Layer, Layered material, BRDF models): a
  Specular Layer has its own `brdf`, `specular`, `transmission`, IOR,
  roughness, `normal` / `bump`; nothing on how a layer's transmission to
  the base depends on the model.
- DAZ's `add_dual_lobe_specular` (`irayubermaterial.mdl`): the *diffuse*
  is the layer of a `custom_curve_layer` over the specular, at
  1 − R W head-on and 1 − W at grazing: what the diffuse loses, the lobe
  gets. So Iray's dlflat − gloss0 is lobe minus the diffuse's loss, and
  so is Octane's.

## 2. Smooth spheres, a black base: the lobe alone

`hdri-lobe.py variants` grew kinds for this (`~/Library/Caches/orbis/calib/c190`):
`l` the dielectric lobe (specular 1, IOR 1.5) as a Specular Layer over a
black Universal in a Layered material (143, `layered_coat_node`), `p` on
OpenPBR's `layer` pin (505, `skin_dual_lobe_layers`), `g` two layers of
specular 0.5 in a Material Layer Group (144) there, `d` Universal itself;
an optional constant normal-map colour; a Calib-Point-UberGloss mode
(one small light at the camera, eight spheres at pins 0.3 / 0.4 / 0.448
(Nishi's) / 0.5 / 0.6 / 0.7 / 0.8 / 1) and `ratio` (disc means of two
renders). 1024 samples, raw pass, 65 – 92 s a render.

Calib-Glossy (HDRI), Octane over the analytic Beckmann lobe (#186's
`measure`), disc < 0.93 R:

| pin | Universal `d` | Layered `l` | OpenPBR layer `p` | furnace `d` / `l` / `p` |
|---|---|---|---|---|
| 0.4 | 1.081 | 1.081 | 1.078 | 0.974 / 0.974 / 0.974 |
| 0.5 | 1.174 | 1.174 | 1.162 | 0.973 / 0.973 / 0.973 |
| 0.6 | 1.396 | 1.395 | 1.360 | 0.972 / 0.972 / 0.972 |
| 1.0 | 2.393 | 2.392 | 2.217 | 1.001 / 1.000 / 1.001 |

GGX (2) and STD (7) the same: `l` = `d` to 0.001, `p` 0.99 – 0.97 × `d`
at the rough pins under the HDRI only (#95's environment defect, a bit
milder on OpenPBR's pin). Group `g` / single `p`: 0.98 at every model
(the top layer's 1 − F over the bottom one). A flat constant normal map
(0.5 0.5 1) changes nothing; a 30° tilted one moves `d` and `p` alike
(`p` / `d` 0.99 – 0.98).

Calib-Point-UberGloss, small light, render over render per sphere (pins
0.3 … 1):

| | 0.3 | 0.4 | 0.448 | 0.5 | 0.6 | 0.7 | 0.8 | 1.0 |
|---|---|---|---|---|---|---|---|---|
| `p` / `d`, Beckmann | 0.999 | 0.998 | 0.999 | 1.001 | 1.000 | 1.000 | 0.998 | 1.000 |
| `p` / `d`, STD | 1.002 | 0.999 | 1.001 | 1.001 | 1.000 | 1.000 | 1.000 | 1.000 |
| `p` / `d`, GGX | 1.000 | 0.999 | 0.997 | 1.002 | 1.000 | 1.000 | 0.998 | 0.998 |
| `p` Beckmann / STD | 0.957 | 1.002 | 1.004 | 1.012 | 1.062 | 1.260 | 1.853 | 4.030 |

**A Specular Layer draws Universal's lobe** at every model, light and
pin; at Nishi's pin 0.448 Beckmann's lobe is STD's (1.004).

## 3. A grey base: what the lobe leaves of it

Upper-case kinds: the base a grey diffuse 0.5 (`B` the base alone). Lobe
+ base over the base alone:

| | 0.3 | 0.4 | 0.448 | 0.5 | 0.6 | 0.7 | 0.8 | 1.0 |
|---|---|---|---|---|---|---|---|---|
| small light, Beckmann layer | 0.959 | 0.962 | 0.963 | 0.961 | 0.952 | 0.945 | 0.957 | 1.026 |
| small light, Beckmann Universal | 0.960 | 0.963 | 0.963 | 0.961 | 0.952 | 0.945 | 0.957 | 1.027 |
| small light, STD layer | 0.974 | 0.979 | 0.982 | 0.983 | 0.984 | 0.987 | 0.994 | 1.004 |

Calib-Glossy (pins 0.05 / 0.4 / 0.5 …): HDRI Beckmann 1.032 / 1.054 /
1.060, STD 1.037 / 1.061 / 1.066; furnace 1.057 / 1.056 / 1.047 against
1.057 / 1.058 / 1.054. Under the head-on light the lobe is a few % and
the base's loss shows: **under a Beckmann dielectric lobe (F0 0.04) the
base keeps ~0.963, under STD ~0.982** - on Universal exactly as on a
Specular Layer. (Iray's curve head-on, R 0.04, W 1: 0.96.)

## 4. Nishi: where the 0.77 is

Faithful exports of the private Nishi twins (dlflat, gloss0, -nn, -nb),
the dual-lobe layers' `brdf` set per lobe (`c190/nishi`), 400 samples,
denoised, ~50 s a render. Face box, dlflat − gloss0, over Iray's:

| layers (lobe 1 / lobe 2) | energy | positive part | negative part | p99.5 (blurred) |
|---|---|---|---|---|
| Iray | 1.000 (256.5) | 312.2 | −55.7 | 1.000 |
| STD / STD (shipped) | 1.048 | 298.6 | −29.7 | 0.894 |
| Beckmann / Beckmann | **0.775** | 303.1 | −104.3 | 0.924 |
| Beckmann / STD | 0.808 | 302.0 | −94.7 | 0.919 |
| STD / Beckmann | 1.007 | 298.9 | −40.5 | 0.899 |
| STD / STD, layers without normal + bump | 1.069 | 306.2 | −32.1 | 1.107 |
| Beckmann / Beckmann, without | 0.803 | 312.4 | −106.4 | 1.146 |

Beckmann − STD over the face: −70; +7 on the dual lobe's top-10 % pixels,
−77 elsewhere (there Beckmann / STD 0.975, of a gloss0 face summing
3677). Without the normal map (dlflatnn) −69, without bump (dlflatnb)
−69. The issue's 0.77 is **the base**: the lobe is there in both models
(positive part 303 against 299), the skin under a Beckmann lobe keeps
2.5 % less, which is a quarter of the dual lobe's energy. Not the layer,
not the normal map.

## Where it stands

- No change to the export: the skin's layers stay at STD (#175's
  choice, now with its reason corrected in the spec and at
  `skin_dual_lobe_layers`); the layered Top Coat's Specular Layer at
  Beckmann (`layered_coat_node`) draws Universal's Beckmann coat, no
  layer loss to fix.
- dlflat − gloss0 mixes the lobe and the base's loss. Iray's negative
  part (−56) lies between STD's (−30) and Beckmann's (−104): which share
  each model leaves the base, per angle, against Iray's curve is #193
  (needs Iray twins of a sphere scene).

## Process

- A black base isolates the lobe, a grey base under a head-on light
  shows what the lobe takes from the base; the face's signed sum hid a
  base effect as a lobe one. Split the sum into positive and negative
  parts first.
- Twin scenes without maps (nn / nb) and a per-lobe model switch settled
  "normal map?" and "which lobe?" in one batch, no Iray.
- APFS is case-insensitive: variant labels `D1` and `d1` are one file.
  The grey-base Calib-Glossy renders overwrote the black ones' PNGs (the
  numbers above were read before); labels now differ by more than case
  (`yD1`).
