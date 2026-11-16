# 2026-11-16 — Rough dielectric lobes split by Look (#162)

Resume of #162 after the user's decision (2026-11-16): both, split by
Look. Faithful takes STD (`brdf` 7) for dielectric Phong lobes past α 0.25
(Iray's highlight shape under small lights), Enhanced keeps GGX energy
preserving (Iray's level under an environment). Metals keep `metal_brdf`
(STD past α 0.25) in both (#172).

Implemented as `Tuning::rough_lobe` (`RoughLobe`, src/tuning.rs; Preview
rows follow their Look), threaded through `Shading` into `phong_brdf` /
`metal_brdf`. #175 (the skin's dual lobe) can read the same field.
Existing Iray references only; no new batch.

Octane faithful-final, 1024 samples, denoised pass; `point-highlight.py`
(peak, run through the peak above 0.3 / 0.5 of it, px),
`calib-compare-all.py` (sphere middle squares), `face-highlight.py`.

## 1. What switches

`brdf` values per export, Enhanced → Faithful: every node at 6 goes to 7,
nothing else moves (Calib-Point-UberGloss: 1×1, 4×6, 3×7 → 1×1, 7×7; the
three metals were STD already). On the G8 figure scenes the switch reaches
the Uber sphere, sclera, teeth, mouth and lashes; the skin itself is
OpenPBR in Faithful and doesn't use `brdf` (#175).

## 2. Faithful on the scenes

| scene | Octane / Iray |
|---|---|
| Calib-Point-UberGloss (means U1 – U8) | 1.03 1.01 1.02 1.07 1.15 1.00 1.02 1.07 |
| — highlights U2 / U3 / U4 | 37×36 29×28 / 53×52 43×42 / 71×71 63×64 v Iray 38×36 29×28 / 53×52 42×42 / 71×72 63×63 |
| Calib-Glossy | 1.08 1.12 1.15 1.11 1.14 1.15 **1.20** 1.11 (G7 1.08 with `brdf7=6`) |
| Calib-DualLobe2 | 1.09 1.10 1.10 1.15 1.11 1.24 1.19 1.24 |
| Calib-NVIDIA2 | 0.99 1.05 1.07 1.09 1.07 1.05 1.00 0.94 |
| Calib-Skin-Victoria-front (private) | spheres 0.99 0.99; face p99.5 0.904 (`brdf7=6`) → 0.901 |

Calib-Skin-Victoria-front: 396 pixels moved by more than 0.005 (scattered,
noise); eyes and mouth boxes 1.076 → 1.074 / 0.935 → 0.935. On a figure
under the front lamp the switch is invisible until the skin's lobe
follows (#175). The rough metals U7 / U8 now come out 1.02 / 1.07 (#172's
STD); the 2026-11-15 numbers (1.74 / 4.15) were before it.

Calib-Point-ArchAniso:

| sphere | Iray | Faithful (STD) |
|---|---|---|
| P1 g 0 iso | 0.018 38×35 29×26 | 0.017 38×36 29×27 |
| P2 g 0 a 4 | 0.020 52×25 40×18 | 0.018 57×26 45×20 |
| P3 g 0.15 iso | 0.082 17×17 13×13 | 0.077 17×18 13×14 |
| P4 g 0.15 a 4 | 0.084 25×12 19×10 | 0.079 25×12 19×10 |
| P5 / P6 g 0.3 | 0.346 9×8 7×6 / 0.331 12×6 9×4 | 0.332 9×8 7×6 / 0.317 12×6 9×4 |
| P7 g 0 a 0.25 | 0.020 25×48 19×38 | 0.018 25×53 19×42 |
| P8 g 0 a 4 rot 0.25 | 0.019 27×48 21×38 | 0.017 26×52 19×41 |

## 3. The anisotropy re-read (#156 / #114 under the new pin)

`arch_aniso_pins` itself went in #114 (each axis at its Phong pin, u = r ·
a^¼, v = r / a^¼); what was left to check is whether Octane's STD reads
`anisotropyTexture` as Beckmann / GGX do (#96: long α = p², short p² (1 −
|a|)²). Probe on Calib-Point-ArchAniso's white lobes (IOR 78, black
albedo; `orbx-variants.py set=`), all eight slots STD, rotation 0.

Isotropic STD ladder (width at half the peak, middle slots):

| pin | 0.5 | 0.6 | 0.6866 | 0.75 | 0.8 | 0.9 |
|---|---|---|---|---|---|---|
| STD | 19 | 29 | 43 | 55 – 57 | 65 | lopsided |
| GGX EP | 15 – 16 | | 33 | | 55 | |

Anisotropic, pin 0.6866 (P2's), a = −0.1 … −0.8 (long axis horizontal):

| a | −0.1 | −0.2 | −0.277 | −0.35 | −0.5 | −0.65 | −0.8 |
|---|---|---|---|---|---|---|---|
| STD long / short | 44 / 31 | 44 / 24 | 43 / 19 | 43 / 15 | 41 / 9 | 38 / 4 | 34 / 1 |
| GGX EP long / short | 36 / 26 | 34 / 19 | 33 / 16 | 33 / 12 | 33 / 7 | 31 / 4 | 27 / 1 |

STD's long axis is the isotropic width at p (43) up to |a| 0.35, its short
axis the isotropic width at p (1 − |a|): a 0.1 → 0.618 (between 29 and
43: 31), 0.277 → 0.496 (19). So STD keeps #96's law and
`octane_anisotropy` stands for Faithful. No change.

The residual is on Iray's side: isotropic STD at P2's u / v pins (r 0.5 /
0.25) draws Iray's isotropic Uber lobes to a pixel (Calib-Point-UberGloss
U3 / U1), but Iray's anisotropic P2 is 40 × 18 at half the peak where
those are ~43 × 19: ~0.9 × on both axes. Either Iray's anisotropic Phong
along an axis is narrower than the isotropic lobe of that exponent, or
mia's conversion at g 0 gives smaller u / v. Filed as #184 (needs our own
MDL module through Iray, as #114's). Enhanced's GGX EP is narrower still
(33 × 16), so the miss doesn't decide the Look.

## 4. Where it stands

- `Tuning::rough_lobe`: Enhanced GGX EP, Faithful STD; spec Look, Variants
  table and the gloss rule say so, with the trade at the value (`RoughLobe`,
  `ENHANCED_ROUGH_LOBE` / `FAITHFUL_ROUGH_LOBE`).
- Enhanced exports unchanged (same `brdf` values; no render needed).
- Calib-Point-ArchAniso can be quoted in Faithful with the ~10 % of #184.
- Next: #175, the skin's dual lobe, the same split on the same field.

Timings: 6 faithful-final scenes + 13 variants at 1024 samples, ~32 min
in all (13:03 – 13:35, ~80 s a render with the batch's start-up); exports
< 1 s from cached dumps.

## Process

- Count the `brdf` values per export before rendering: it shows at once
  which nodes a rule moves (here: all 6 → 7, nothing else), and on figure
  scenes that the skin isn't one of them.
- Eight probe slots in one point-lit scene make a ladder per render;
  compare slot with slot, the projected spheres differ by a few px.
