# 2026-11-16 — NVIDIA's `mdl 1.0` lobe is a Phong of exponent e (#184)

Question (#184, from #162): under Faithful's STD the anisotropic spheres
of Calib-Point-ArchAniso (P2 / P7 / P8, NVIDIA's architectural, g 0 → r
0.354, a 4 → u 0.5 / v 0.25) came out ~10 % wide on both axes, while
isotropic STD at the u / v pins drew Iray's isotropic Uber lobes
(Calib-Point-UberGloss U3 / U1) to a pixel. Hypotheses in the issue:
Iray's anisotropic Phong narrower along an axis than the isotropic lobe
of that exponent, or `base::anisotropy_conversion` giving smaller u / v.
No new Iray batch: the existing references of #114 and #162 decompose it.

## 1. Decomposition from the references on disk

The three point-lit scenes share the sphere layout: per slot the same
projected half-axes (horizontal 49 / 47.5 / 46.5 / 46 / 46 / 46.5 / 47.5 /
49 px, vertical 45.9 everywhere; `sphere_discs.py`). So vertical widths
compare across slots and scenes, horizontal ones only at equal R. The
comparison #162 made (P2 in slot 2 against U3 in slot 3, and Octane's
isotropic ladder in the middle slots) mixed slots.

Model (scratch `lobe_width.py`): head-on, L ∝ D(θ) G / cos θ with x = R
sin θ, v-cavities G; widths of the run above 0.3 / 0.5 of the peak, for
D ∝ cosᵉ (a Phong of exponent e = 2 / r²) and D ∝ cos^(e − 1) (`libbsdf`,
#162).

| sphere (Iray) | module | measured w × h, 0.3 / 0.5 | cosᵉ | cos^(e − 1) |
|---|---|---|---|---|
| MdlAniso S5 iso r 0.354 (R 46) | ours 1.0 | 35×35 27×27 | 35.5 27.4 | 36.6 28.3 |
| MdlAniso S6 same | ours 1.3 | 37×36 29×28 | | 36.6 28.3 |
| ArchAniso P1 iso r 0.354 (R 49 / 45.9) | NVIDIA 1.0 | 38×35 29×26 | 37.8×35.4 29.2×27.3 | 39.0×36.5 30.1×28.2 |
| UberGloss U2 r 0.354 (R 47.5) | Uber 1.7 | 38×36 29×28 | | 37.8×36.5 29.2×28.2 |
| MdlAniso S3 u 0.4 (R 46.5) | ours 1.0 | 40 31 | 40.4 31.3 | 42.1 32.7 |
| MdlAniso S4 same (R 46) | ours 1.3 | 41 32 | | 41.7 32.4 |
| MdlAniso S7 u 0.5 / v 0.25 via `base::` (R 47.5) | ours 1.0 | 51×25 40×19 | 51.3×25.1 40.3×19.2 | |
| MdlAniso S8 same (R 49) | ours 1.3 | 57×25 45×20 | | 56.3×25.5 44.5×19.5 |
| ArchAniso P2 a 4 (R 47.5) | NVIDIA 1.0 | 52×25 40×18 | 51.3×25.1 40.3×19.2 | |
| ArchAniso P7 a 0.25 (R 47.5) | NVIDIA 1.0 | 25×48 19×38 | 26.0×49.5 19.9×38.9 | |
| UberGloss U3 r 0.5 (R 46.5) | Uber 1.7 | 53×52 42×42 | | 53.5×52.8 42.2×41.7 |

- Every 1.0 lobe is the cosᵉ profile, every 1.3 / Uber lobe the
  cos^(e − 1) one, to about a pixel; isotropic and along either axis
  alike. At r 0.2 the two differ by < 0.5 px (S1 = S2), which is why #114
  read "1.0 renders as 1.3".
- So neither hypothesis: along an axis Iray's lobe is its isotropic lobe
  at that axis's roughness, and the conversion gives u = r · a^¼ (S8
  through `base::` at 1.3 is the cos^(e − 1) profile at u 0.5 / v 0.25).
  The ~10 % was the MDL version — √(e / (e − 1))-ish at e 8 — and it is
  on the isotropic P1 too, only 1 px at r 0.354.
- Peaks (1.0 / 1.3: S5 / S6 1.07, S7 / S8 1.11, S1 / S2 0.95) and sums
  over the disc are slot-dependent (light distance) and noisy; the
  energy of the legacy lobe is #112's matter (`LEGACY_METAL_RIM`, the
  furnace tables), not read here.

## 2. The rule

The reflect-only lobes of NVIDIA's presets (all `mdl 1.0`: metals,
dielectrics' gloss, architectural's reflection, each axis when
anisotropic) take Walter's Beckmann equivalent of a Phong of exponent e,
α = r / √(1 + r²) (`legacy_phong_pin`) — the pin every lobe had before
#162 moved the Uber's to r √(2 / (2 + r²)). Derived, no constant, the same
in both Looks (it maps Iray's D; the Look's choice is the `brdf`). The
transmitting 1.0 lobes (resin's base, architectural's glass and the coat
over them) keep `phong_pin`: `RESIN_LOBE`, `LEGACY_LOBE` and the Falloff
Mixes were fitted on it, and their head-on shape is unmeasured.

## 3. Octane at the rule (1024 samples, denoised)

Calib-Point-ArchAniso, `point-highlight.py`:

| sphere | Iray | Faithful before (#162) | Faithful now | Enhanced now |
|---|---|---|---|---|
| P1 g 0 iso | 0.018 38×35 29×26 | 0.017 38×36 29×27 | 0.018 38×36 29×27 | 0.021 33×30 23×21 |
| P2 g 0 a 4 | 0.020 52×25 40×18 | 0.018 57×26 45×20 | **0.020 51×24 40×18** | 0.024 48×22 33×15 |
| P3 / P4 g 0.15 | 17×17 13×13 / 25×12 19×10 | 17×18 13×14 / 25×12 19×10 | 17×18 13×13 / 25×12 19×10 | same |
| P5 / P6 g 0.3 | 9×8 7×6 / 12×6 9×4 | same | same | same |
| P7 g 0 a 0.25 | 0.020 25×48 19×38 | 0.018 25×53 19×42 | **0.020 25×49 19×38** | 0.024 22×44 15×31 |
| P8 a 4 rot 0.25 | 0.019 27×48 21×38 | 0.017 26×52 19×41 | **0.018 27×50 20×40** | 0.022 23×44 15×31 |

Sphere means (`calib-compare-all.py`), Faithful / Enhanced:

| scene | Faithful | Enhanced |
|---|---|---|
| Calib-Point-ArchAniso | 1.00 1.01 0.97 0.97 0.96 0.96 1.01 1.01 | 1.05 1.07 0.97 0.97 0.96 0.96 1.06 1.07 |
| Calib-NVIDIA2 | 0.99 1.05 1.06 1.09 1.07 1.05 1.11 0.94 | 0.99 1.05 1.06 1.00 1.07 1.05 1.10 0.94 |
| Calib-NVIDIA3 | 1.07 1.06 1.06 1.06 1.03 1.02 1.11 1.11 | same |
| Calib-NVIDIA | 1.01 0.96 1.04 0.97 1.12 1.06 1.01 1.07 | 1.19 1.07 1.03 0.97 1.12 1.07 1.01 1.07 |

Calib-NVIDIA2's export, before / after (Enhanced `project.ocs` diff): only
five roughness / anisotropy values move (0.496 → 0.492, 0.586 → 0.577 at
A4 g 0, A5's pins 0.341 → 0.340, anisotropy −0.292 → −0.291). The HDRI
scenes are within 0.01 of #162's numbers except A7 (resin grey, 1.00 →
1.11), which the change doesn't touch (its pins are equal in the diff):
that is caee575's transmission Falloff Mix, rendered after #162's
numbers. Calib-NVIDIA's N1 1.19 in Enhanced is #112's flat legacy rim
(Faithful draws it).

Timings: 10 renders, 16:38 – 17:00 (~2 min each with start-up); exports
< 1 s.

## Process

- Read the sphere layout before comparing highlight widths across slots:
  the horizontal half-axis runs 46 – 49 px with the slot, the vertical
  doesn't, so a 2 px "excess" can be the slot.
- An analytic profile per candidate D (here two exponents) over every
  sphere on disk settled it without a render; #114's "1.0 = 1.3" held only
  at r 0.2, where the two exponents are 0.5 px apart.
