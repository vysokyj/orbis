# 2026-11-11 — Rough transmission: Iray's df against NVIDIA's modules (#116)

Goal: find why Iray's architectural transmit lobe and uncoated resin
(`simple_glossy_bsdf`, scatter_transmit / scatter_reflect_transmit) keep
far less in the white furnace than `libbsdf` predicts (refr_gloss 0.2:
0.614 v 0.99), and derive it so that #115's measured `ARCH_GLASS_LOBE` /
`RESIN_LOBE` can go. Iray Server 5000 samples; no Octane renders.

## Tools

A numpy re-implementation of `src/mdl_layer.rs`'s `libbsdf` tracer (Phong
half-vector sampling, v-cavities, `scatter_transmit` / reflect_transmit /
reflect, thin-walled pseudo-BTDF, IOR, a per-event tint, a cap on the
number of events, anisotropic Phong), output per ring of the entry radius
(10 rings of 0.1) besides the disk r < 0.75. It agrees with the Rust model
on resin (1.000 / 0.999 / 0.973 / 0.824 / 0.655 / 0.531). Iray rings:
sphere / background, gamma 2.2 decode, centres 424 + 153 k, y 561, r 45.

## 1. Analysis before rendering

- **Rough reflection matches.** Calib-Furnace-Metal (white Uber metals,
  `simple_glossy_bsdf` reflect) against `libbsdf`, disk: r 0.05 – 1 0.999 /
  1.000, 0.999 / 1.000, 0.989 / 0.996, 0.964 / 0.972, 0.916 / 0.924, 0.850
  / 0.861, 0.711 / 0.724, 0.548 / 0.565, the rings' shape too. The tracer
  is right for reflection; the excess is transmission or NVIDIA-specific.
- **The architectural rings have a shape no roughness draws.** Iray
  refr_gloss 0.2: 0.717 head-on → 0.358 at the rim (refr_gloss 0: 0.609 →
  0.281); `libbsdf` transmit lobes are flat to r ~0.8 then drop, at any
  roughness and with GGX or Beckmann too. Iray / `libbsdf` at the
  reflection's roughness √(r · r₀) per ring: refr_gloss 0.6 ~0.96 flat,
  0.4 0.96 → 0.85, 0.2 0.72 → 0.42, 0 0.62 → 0.39 — a jump between 0.4
  and 0.2.
- **Blur.** High-pass detail within r < 0.7 relative to the smooth
  sphere: architectural refr_gloss 0.6 (formula r 0.019) keeps 43 %, the
  Uber's own transmit lobe at r 0.09 (Calib-Glass R6) 42 % — Iray's
  architectural roughness looks like √(r · r₀) = 0.082 (the rule the spec
  already has for its reflection), not 0.019. That explains the look, not
  the loss: at refr_gloss 0 both give r 0.354, where `libbsdf` keeps 0.94
  and Iray 0.51. Blur direction (|∂x| / |∂y|) 0.45 / 0.46 v the Uber's
  0.41: no anisotropy (an anisotropic-Phong try with a large r_v fitted
  the thin-walled level but not the solid rings).
- DAZ hands the NVIDIA material wrapped: `pass_material(add_geometry(base:
  nvidia::architectural(...)))`; `add_geometry` (DAZ's Uber module) keeps
  `thin_walled`, `ior`, `volume`, `surface`, `backface` and sets the
  geometry normal to `state::normal()`. Nothing in the MDL or the wrapper
  loses light. Iray's `base::` is compiled into libneuray (only symbol
  names, no source). The Iray docs: the Firefly Filter can cut
  contributions; `iray_max_path_length`; nothing on the NVIDIA modules.
- Calib-Furnace-ArchGlass and -Metal have identical render settings
  (Firefly Filter on, path length unlimited, no guided / caustic sampler)
  and the same 48 × 24 sphere.

## 2. Iray variants (one batch, 4 scenes)

`make-calib-scenes.py`: **Calib-Furnace-UberGlass** — the same df through
DAZ's Uber (Refraction Weight 1, no gloss, own refraction roughness →
scatter_transmit; shared gloss → one reflect_transmit lobe), **-PL3** the
same at Max Path Length 3, **Calib-Furnace-ArchGlass2** (Firefly Filter
off; thin-walled, IOR, refr_color variants), **Calib-Furnace-Resin3**
(Resin2 at Max Path Length 3). Iray 86 / 55 / 70 / 58 s.

### The Uber: `libbsdf` exactly

| sphere | Iray disk | `libbsdf` | PL3 Iray | PL3 `libbsdf` |
|---|---|---|---|---|
| transmit r 0.019 | 1.000 | 1.000 | 1.000 | 1.000 |
| r 0.051 | 1.000 | 1.000 | 1.000 | 1.000 |
| r 0.134 | 0.997 | 0.999 | 0.995 | 0.996 |
| r 0.218 | 0.984 | 0.988 | 0.966 | 0.968 |
| r 0.354 | 0.930 | 0.935 | 0.886 | 0.887 |
| r 0.6 | 0.774 | 0.777 | 0.727 | 0.727 |
| reflect_transmit r 0.25 | 0.967 | 0.975 | 0.899 | 0.901 |
| reflect_transmit r 0.5 | 0.817 | 0.815 | 0.737 | 0.730 |

Per ring within ~0.01 everywhere (r 0.354: 0.981 → 0.738 v 0.986 →
0.724). Iray's rough transmission, multi-bounce share included, *is*
`libbsdf`'s. At the architectural's own MDL roughness the Uber keeps 0.997
(r 0.134) where the architectural keeps 0.614; uncoated resin r 0.25 /
0.5 0.916 / 0.691 where the Uber's identical lobe keeps 0.967 / 0.817.

### Architectural, Firefly Filter off

| sphere | Iray disk (rings head-on → rim) | `libbsdf` (formula r / √(r · r₀)) |
|---|---|---|
| refr_gloss 0.6 | 0.964 (0.996 → 0.905) — was 0.964 | 1.000 / 1.000 |
| 0.2 | 0.617 (0.717 → 0.373) — was 0.614 | 0.999 / 0.989 |
| 0 | 0.513 (0.609 → 0.284) — was 0.512 | 0.934 |
| 0.2 thin-walled | 0.506 flat (0.52 → 0.51) | 0.999 / 0.982 |
| 0 thin-walled | 0.386 flat | 0.892 |
| 0.2 IOR 1.2 | 0.722 (0.817 → 0.437) | 1.000 / 0.998 |
| 0.2 IOR 2.0 | 0.468 (0.524 → 0.305) | 0.994 / 0.964 |
| 0.2 refr_color 0.5 | 0.028 | 0.25 × |

- Firefly Filter: no effect (0.964 / 0.617 / 0.513 v 0.964 / 0.614 /
  0.512).
- refr_color 0.5 is 0.218 linear: 0.218² × 0.617 = 0.029 — two tinted
  events per path, as `libbsdf`'s; light isn't trapped inside.
- Thin-walled (no refraction, no η², no TIR) loses too, a flat
  0.71 per wall at 0.2 — the loss is per surface event and not about
  refraction; in the solid sphere it grows with IOR (head-on 0.82 / 0.72
  / 0.52 at 1.2 / 1.5 / 2).
- A per-sphere roughness fit (`libbsdf`, 40 r values, rings 0 – 0.9):
  r* 0.22 / 0.37 / 0.85 / 1.17 for refr_gloss 0.6 / 0.4 / 0.2 / 0,
  thin 0.77 / 1.05, IOR 1.2 / 2 1.17 / 0.85, rms 0.03 – 0.05 with the
  wrong shape (Iray brighter head-on, thin flat where `libbsdf` rises to
  the rim) and refr_gloss 0.6's r* 0.22 against its blur's 0.08: not a
  roughness.

### Resin at Max Path Length 3

PL3 / full, Iray v `libbsdf`: r 0.02 0.962 / 0.959, 0.25 0.934 / 0.926,
0.5 0.920 / 0.898, 0.75 0.945 / 0.916, 1 0.963 / 0.927. The light that
bounces inside is the share `libbsdf` predicts (slightly less); the
excess is on the paths with two events.

## Conclusion

Iray's `simple_glossy_bsdf` transmission is `libbsdf`'s: the Uber proves
it lobe for lobe, ring for ring, path length included. The excess loss
belongs to NVIDIA's modules as DAZ hands them to Iray (architectural and
resin alike, like `metal_generic`'s √cos θ_v against the Uber's same lobe,
2026-11-10 brushed disk): a per-surface-event loss, present without
refraction, growing with roughness and IOR — not the Firefly Filter, not
light trapped inside, not a roughness remap, nothing in the MDL sources,
DAZ's `add_geometry` wrapper or NVIDIA's docs. It can't be derived from
what we can read, so #115's measurement stays the rule: `ARCH_GLASS_LOBE`
and `RESIN_LOBE` (the lobe alone in the furnace) with `libbsdf`'s layering
on top. No export change. `ARCH_GLASS_ROUGH` (3.7) stays the look's fit;
the blur at refr_gloss 0.6 says √(r · r₀) (0.082 v 3.7 r 0.070), at 0.2
the frosted interior is part of the same unexplained loss.

## Process lessons

- The cheapest decisive variant was the same df through a different
  material (the Uber): one scene split "Iray's lobe" from "NVIDIA's
  module" before any parameter sweep. When an NVIDIA preset disagrees with
  `libbsdf`, render the Uber with the identical df first.
- A tinted variant (refr_color 0.5) counts the events per path; remember
  the DAZ colour is decoded (0.5 → 0.218).
- A started-fresh queue DAZ takes one scene per NVIDIA module, the Uber
  as many as wanted (#94).
