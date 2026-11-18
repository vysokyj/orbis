# 2026-11-18 — Octane's grey dielectric, and fake shadows in #185's table (#201)

Question (#201): after #185, grey / tinted rough resin and architectural
glass stay 5 – 7 % above Iray in Faithful (Calib-Furnace-Resin R5 / R7,
-Resin2 U7, -ArchGlass2 H8), rims to 1.3 ×, while the white spheres at the
same roughness are within 1 – 4 %. Decompose before fitting: Octane probes
with known inputs, no Iray, no new constant.

## 0. Where the residue sits (Faithful, 400 samples denoised, vs Iray)

Disk (r < 0.75) and rim (0.8 – 0.9) Octane / Iray, `furnace-rings.py
measure` on #185's `f185-after` renders:

| sphere | α (`legacy_phong_alpha`) | disk | rim | white twin | its disk / rim |
|---|---|---|---|---|---|
| R2 grey r 0.02, coat 0.95 | 0.02 | 1.03 | 1.04 | R4 | 1.00 / 1.01 |
| R5 grey r 0.25, coat 0 | 0.24 | 1.07 | 1.20 | R6 | 1.02 / 1.02 |
| R7 grey r 0.5 | 0.45 | 1.05 | 1.21 | — | |
| U7 grey r 0.5, coat 0 | 0.45 | 1.06 | 1.17 | U4 | 1.01 / 1.03 |

R5 sits at α 0.24, below where #185's counter starts (1 at α ≤ 0.25), so
the grey excess is not the counter's. Iray against its own model
(`libbsdf` × the legacy loss × τ at the grey) is within 1 % ring for ring on
R5 (0.597 → 0.452 v 0.590 → 0.449): the Iray side is right, Octane keeps
more than its Smith model when the colour is below 1.

H8 (architectural, colour 0.5 sRGB) renders at 0.03 of the dome: ~23 / 255
in the PNG, where one 8-bit step is ~10 % — its disk 1.07 and rim 1.30 are
at the measurement's floor and are not used below.

## 1. The pipeline is linear

Calib-Furnace-Resin's Faithful export (`--no-denoise`), all eight spheres
re-pinned to a plain Lambertian (`set=` specular 0, transmission 0,
albedo 0.1 … 1), Octane 1024 raw: 0.099 / 0.247 / 0.498 / 0.600 / 0.701 /
0.795 / 0.903 / 1.002 of the dome. A grey sphere reads as its albedo.

## 2. Probes: grey dielectric spheres

The same export, every sphere Beckmann at a known α, no coating, no
falloff, IOR 1.49 unless said (`set=` pins; 1024 samples raw, 57 – 86 s
each). Model: `trace(Model::Smith)` (2^18 paths) with the tint on every
base event (`all`, the export's model), on refractions only (`refr`), or
the rule of § 3 (`layer`). Rings centre → 0.8 – 0.9 of the silhouette.

`rt5` (specular = transmission = 0.5) and the white twins (= 1):

| α | Octane 0.5 | `all` | `layer` | Octane / `layer` | white Octane / model |
|---|---|---|---|---|---|
| 0.02 | 0.266 → 0.283 | 0.254 → 0.260 | 0.263 → 0.280 | 1.01 → 1.01 | 1.00 |
| 0.25 | 0.266 → 0.251 | 0.253 → 0.192 | 0.263 → 0.254 | 1.01 → 0.99 | 1.01 → 0.99 |
| 0.5 | 0.262 → 0.211 | 0.235 → 0.155 | 0.252 → 0.201 | 1.04 → 1.05 | 1.04 → 1.02 |
| 0.7 | 0.218 → 0.201 | 0.192 → 0.141 | 0.208 → 0.174 | 1.05 → 1.16 | 1.05 → 1.06 |

`tr5` (specular 0, transmission 0.5): α 0.02 0.249 flat — T², no Fresnel
at all; α 0.25 0.249 → 0.222 against `refr` 0.250 → 0.226 and `all` 0.250
→ 0.182: a total reflection inside is **untinted**.

`s1t5` (specular 1, transmission 0.5) minus `rt5` = 0.017 – 0.019 at every
α: the outside reflection carries the specular colour (F · Δs = 0.019).

Tint sweep at α 0.02 (head-on, no rim effects), T = 0.25 / 0.5 / 0.75 /
0.9, specular = T and specular = 1:

| IOR | s | Octane | physical split F s + (1 − F)² T² / (1 − F s) |
|---|---|---|---|
| 1.49 | T | 0.073 / 0.266 / 0.582 / 0.817 | 0.068 / 0.255 / 0.564 / 0.810 |
| 1.49 | 1 | 0.098 / 0.284 / 0.582 / 0.818 | 0.098 / 0.279 / 0.579 / 0.817 |
| 2 | T | 0.088 / 0.294 / 0.601 / 0.835 | 0.079 / 0.265 / 0.568 / 0.811 |
| 2 | 1 | 0.167 / 0.335 / 0.611 / 0.835 | 0.167 / 0.333 / 0.611 / 0.831 |

## 3. The rule: Universal takes F s off the transmission

All of § 2 is one rule (derived from the smooth spheres, then held against
the rough ones): Universal's specular layer reflects F · s and passes 1 − F
· s; the transmission lobe under it refracts T of that — **T (1 − F s)**,
where a tinted reflect + transmit lobe (Iray's, and the export's model)
gives T (1 − F). At s = 1 they agree, so a white sphere can't tell; at s =
T < 1 Octane keeps more, most where F is large (inside, near the critical
angle — the rim, the rough lobe). A total reflection is untinted (the
specular reflects s of it, the transmission lobe the rest). The smooth IOR
2 sweep at s = T: 0.0886 / 0.292 / 0.599 / 0.829 predicted v 0.088 / 0.294 /
0.601 / 0.835; s = 1: 0.1666 / 0.333 / 0.611 / 0.831 v 0.167 / 0.335 / 0.611 /
0.835. At α 0.5 (`sw5`, no excess applied) every T is Octane / model 1.03
± 0.02 flat from centre to rim; at α 0.25 1.00 ± 0.01.

Resin's export sets specular = transmission = its tint × k, so the rule
applies to every resin (and to k itself: k < 1 is a grey too); for a
transmit-only base (Uber rough glass, architectural: specular 0 or a
coating) only the untinted total reflection.

The rule in the solve's Octane model (`trace`, `Model::Smith`): per base
event the tint x as before, a total reflection inside untinted, each
refraction of a reflect + transmit base × (1 − F x) / (1 − F) (F at its
microfacet). Unit-tested against the probes (smooth IOR 2 within 1 %,
rough α 0.25 0.263, tr α 0.25 0.249). Held against #185's Faithful renders
at their own (old) pins, Octane render / new model per ring:

| sphere | centre → rim | old model |
|---|---|---|
| R5 grey r 0.25 c0 | 1.02 → 0.99 | ≈ Octane / Iray: 1.04 → 1.20 |
| R2 grey r 0.02 | 1.02 → 0.99 | 1.00 → 1.04 |
| R7 grey r 0.5 | 1.05 → 0.98 | 1.12 → 1.21 |
| U7 grey r 0.5 c0 | 1.03 → 0.94 | 1.04 → 1.17 |
| R6 white r 0.25 c0 | 1.02 → 0.98 | 1.00 → 1.02 |
| U4 white r 0.5 c0 | 1.03 → 0.93 | 1.02 → 1.03 |
| U5 white r 0.75 c0 | 1.03 → 0.89 | 1.08 → 1.07 |
| U6 white r 1 c0 | 0.94 → 0.88 | 1.05 → 1.05 |

The grey residue is explained. But the rough whites' rims now come out
**below** the model by 7 – 12 %: they had matched only because two errors
cancelled — the old model's missing (1 − F k) / untinted total reflection
at k < 1, and #185's excess at the rim.

## 4. #185's excess was measured with fake shadows on

`rt5`'s white twins (no excess folded in) are Octane / Smith 1.04 → 1.02
at α 0.5 and 1.05 → 1.06 at α 0.7 — flat — where `OCTANE_SPHERE` has 1.04
→ 1.12 and 1.08 → 1.32 for the same plain spheres in the same layout
(Calib-Furnace-UberGlass and -Resin share Calib-NVIDIA2's spheres and
camera). The one difference in the material: Calib-Furnace-UberGlass's
export, which #185 re-pinned, carries the Uber glass's **`fake_shadows`
1**; resin and architectural exports carry none (Octane's default, off).
#104 had seen fake shadows take light in straight past the refraction and
the inner Fresnel in a scattering volume; here the rough rt lobe's direct
light samples do the same towards the rim. So the table is the Uber
glass's (fake shadows on) and over-corrects resin and architectural glass
at the rim — not rendered yet to prove (below).

Nothing exported changed in this session: shipping the rule alone would
drop the rough white resin rims to ~0.9 × Iray until the excess is
measured without fake shadows.

## Octane quit; what is prepared

The check of the scene set in both Looks (calib-variants, Faithful +
Enhanced of Calib-Furnace-Resin / -Resin2 / -ArchGlass / -ArchGlass2 /
-UberGlass, Calib-Resin / -ArchGlass) never rendered: Octane X quit
before the first job and did not come back with its Script menu within 2
minutes (screen unlocked). Per the loop's rule not retried; #201 waits
(`question`). Calib-Furnace-Resin3 has no cached dump.

Prepared (in `~/Library/Caches/orbis/calib/c201/`, regenerable): five
variants of Calib-Furnace-Resin's Faithful `--no-denoise` export, each
sphere re-pinned with `set=` (`b`, a bool pin, new in `orbx-variants.py`):
- `fs`: white rt, α 0.25 / 0.5 / 0.7 / 1, IOR 1.5, spheres 1 – 4
  `fake_shadows` 0, 5 – 8 the same at 1 — the cause, side by side;
- `xrt` / `xtr` / `xrt2` / `xtr2`: white, α 0.25 / 0.36 / 0.45 / 0.5 /
  0.6 / 0.7 / 0.85 / 1, specular 1 / 0, IOR 1.5 / 2, `fake_shadows` 0 —
  `OCTANE_SPHERE` again without fake shadows.

## 5. Resumed (the user: Octane is free): fake shadows confirmed

The five variants, Octane 1024 raw, 55 – 76 s each. `fs`, disk / centre
→ rim per α, fake shadows 0 (spheres 1 – 4) v 1 (5 – 8), and
`OCTANE_SPHERE`'s `rt` row:

| α | `fake_shadows` 0 | 1 | `OCTANE_SPHERE` rt |
|---|---|---|---|
| 0.25 | 0.988, 1.004 → 0.891 | 0.991, 1.008 → 0.907 | 1.009 → 0.911 |
| 0.5 | 0.864, 0.974 → 0.719 | 0.898, 0.964 → 0.803 | 0.965 → 0.803 |
| 0.7 | 0.733, 0.799 → 0.645 | 0.816, 0.831 → 0.806 | 0.827 → 0.807 |
| 1 | 0.594, 0.571 → 0.571 | 0.757, 0.683 → 0.858 | 0.685 → 0.858 |

Fake shadows on reproduce the table to 1 %; off, the rim loses 10 – 33 %.
#185's table is the Uber glass's.

## 6. The excess without fake shadows

`xrt` / `xtr` / `xrt2` / `xtr2` became `OCTANE_SPHERE_PLAIN` (same
layout, IOR, α grid and normalisation as `OCTANE_SPHERE`).
`octane_excess` reads it for resin and architectural glass
(`transmission_falloff`), the old one for the Uber glass
(`tint_factor_octane`, which exports fake shadows unless it scatters — a
scattering rough Uber glass is not measured). Octane / Smith model over
the α 0.25 row, centre → 0.8 – 0.9:

| IOR, α | `tr` plain | `rt` plain | `rt` fake shadows |
|---|---|---|---|
| 1.5, 0.5 | 1.04 → 0.97 | 1.05 → 1.01 | 1.04 → 1.12 |
| 1.5, 0.7 | 1.04 → 0.95 | 1.04 → 1.06 | 1.08 → 1.32 |
| 1.5, 1 | 1.05 → 0.95 | 1.06 → 1.14 | 1.28 → 1.71 |
| 2, 0.5 | 0.95 → 0.88 | 0.95 → 0.92 | 1.08 → 1.16 |
| 2, 1 | 0.81 → 0.78 | 0.90 → 0.98 | 1.74 → 1.94 |

Without fake shadows Octane is its Smith model within a few % at IOR 1.5
and keeps *less* at IOR 2 — most of #185's "excess" was the fake
shadows' light.

With the § 3 rule and the plain table in the model, #185's Faithful
renders at their own (old) pins, Octane / model per ring (`diag201::
print_old_pins`): R5 / R6 / R2 / R4 1.00 – 1.02 everywhere, R7 1.00 →
1.06, U7 / U4 1.00 → 1.03, U5 1.01 → 1.05, U6 0.98 → 1.10 (were R5 1.04
→ 1.20, U4 1.03 → 0.93 with the rule alone). Both errors are gone.

## 7. Calib-Resin cube2 (+27 %) is not a material

`calib-compare.py regions` boxes are Calib-Base's layout (cubes beside a
figure). In Calib-Resin, `cube2` (x 910 – 1010, y 490 – 590) holds the
right edge of R4, the dark HDRI between the spheres, the floor's horizon
and the left rim of R5 (19 % sphere pixels). Its **median** sits where
the bimodal histogram crosses classes, so a few % shift moves it from one
class to the other: 1.27 – 1.29 by the median, 1.064 by the mean of the
same box (off the spheres 1.043, on R4 1.06, on R5's rim 1.25). Measured
per sphere instead (`sphere_discs`, f185-after Faithful, disk r < 0.75 /
rim 0.8 – 0.95): R1 1.09 / 1.12, R2 1.04 / 1.09, R3 1.07 / 1.11, R4 1.03
/ 1.06, R5 1.10 / 1.27, R6 1.04 / 1.06, R7 1.15 / 1.70, R8 1.03 / 1.12 —
the grey and rough spheres high, as in the furnace (R5, R7). The lit
scene's spheres are compared per disc from now on; the cube boxes don't
apply to Calib-Resin / -ArchGlass.

## 8. The scene check (both Looks, 400 samples denoised, vs Iray)

v1 = the § 3 rule + the plain table; the falloff's shape still solved on
white. Furnace disk / rim (0.8 – 0.9), Faithful: R5 1.07 / 1.20 → 0.96 /
1.08 (centre 0.93), U7 1.06 / 1.17 → 0.97 / 1.08, H8 1.07 / 1.29 → 0.96 /
1.18 — the grey's level right, its shape not: the centre low, the rim
high. Cause: the shape (grazing, skew) was solved on the white sphere,
then k per channel; with Octane's grey split the white and the grey
rings differ in shape (the grey's total reflections and T (1 − F s) keep
the rim relatively brighter), so a white-solved shape leaves a grey rim
high. v2 solves the shape at the tint's grey (channel mean) — derived,
no constant: the model meets Iray's rings within 1 – 3 % for R5 / U7 /
U4 / U5 / U6 (R7 0.95 – 1.08, its grazing pinned at 0).

| sphere | before (#185, f185-after) | v1 | v2 |
|---|---|---|---|
| R5 grey r 0.25 c0 | 1.067 / 1.20 | 0.961 / 1.08 | 1.023 / 1.02 |
| R7 grey r 0.5 | 1.051 / 1.21 | 1.035 / 1.24 | 1.031 / 1.24 |
| U7 grey r 0.5 c0 | 1.061 / 1.17 | 0.974 / 1.08 | 1.000 / 1.05 |
| H8 colour 0.5 | 1.071 / 1.29 | 0.964 / 1.18 | 1.000 / 1.12 |
| R2 grey r 0.02 | 1.026 / 1.04 | 0.981 / 1.01 | 1.003 / 1.01 |
| U4 white r 0.5 c0 | 1.009 / 1.03 | 0.994 / 1.02 | 0.994 / 1.02 |
| U5 white r 0.75 c0 | 1.022 / 1.07 | 1.022 / 1.12 | 1.020 / 1.12 |
| U6 white r 1 c0 | 1.041 / 1.05 | 1.066 / 1.13 | 1.066 / 1.13 |
| Resin3 7 | 0.940 / 0.98 (no before) | | 0.965 / 0.95 |

Enhanced (no excess) moves the same way and its rough whites come down
with the rule: U5 / U6 1.100 / 1.26, 1.170 / 1.33 → 1.040 / 1.15, 1.085 /
1.18; R5 1.050 / 1.18 → 1.028 / 1.01; H8 1.107 / 1.35 → 1.000 / 1.06.
Uber glass (fake-shadow table, unchanged path): within 0.01 of before.
Architectural G-spheres within 0.01 – 0.02.

Lit, per sphere (disk / rim 0.8 – 0.95), Faithful before → v2:
Calib-Resin R1 1.09 / 1.12 → 1.06 / 1.09, R2 1.04 / 1.09 → 1.02 / 1.05,
R5 1.10 / 1.27 → 1.05 / 1.07, R8 1.03 / 1.12 → 1.01 / 1.10, R7 1.15 /
1.70 → 1.14 / 1.71; Calib-ArchGlass G4 1.04 / 1.05 → 1.03 / 1.03, G7
1.03 / 0.98 → 1.04 / 1.02, G8 1.15 / 0.93 unchanged (floor 1.03 in both).

Left (follow-up issue): the coated grey R7 (r 0.5, coat 0.95) — its
falloff fit can't reach Iray's rings (grazing pinned at 0, rim 1.24 in
the furnace, 1.70 lit); the rough uncoated whites U5 / U6 rims 1.12 /
1.13 over Iray (the plain table's rt rows are measured without a falloff
on the transmission). Neither is in #201's grey residue.

The first scene check printed "Octane X quit before the probe was done"
but all eight scenes of both variants rendered (Octane relaunched by
itself); the second ran clean.

## Process lessons

- A shape solved on white and a level solved per channel are only
  separable when the model is linear in the tint per event. Octane's
  T (1 − F s) is not: solve the shape at the material's own colour.
- `calib-compare.py regions` boxes are Calib-Base's; on another layout a
  region's median of a mixed box (background, floor, two sphere edges)
  is no measurement. Use the projected discs.

- A white furnace sphere can't separate a reflection from a transmission
  split: any energy-conserving split keeps 1. Probe the grey (and two
  IORs: the Fresnel's size tells the split from a tint exponent).
- A smooth sphere head-on is an analytic series (F s + c² / (1 − F r)):
  fitting c and r there named the rule before any rough sphere was traced.
- A table measured on one export's re-pinned spheres carries every pin
  the re-pin didn't touch (here `fake_shadows`). Diff the whole material
  node against the export it is applied to.
- zsh doesn't split an unquoted `$P`: a scene list in one variable reached
  `calib-variants.sh` as one argument ("export FAILED (no cached dump?)").
