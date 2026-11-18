# 2026-11-19 — The rough resin rims, decomposed (#205)

Question (#205): after #201, Faithful (400 samples denoised) against the
Iray references, rims (0.8 – 0.9 of the silhouette): coated grey R7 (r 0.5,
coat 0.95, Calib-Furnace-Resin) 1.24, uncoated white U5 / U6 (r 0.75 / 1,
Calib-Furnace-Resin2) 1.12 / 1.13. Decompose before fitting: no Iray
render (the Iray Server didn't answer), Octane probes with known inputs,
the models, the references in git.

## 1. Octane's Falloff Map curve (probe `fo`)

Calib-Furnace-Resin2's Faithful export (`--no-denoise`), every sphere
re-pinned to a Lambertian (`set=` specular 0, transmission 0, coating 0)
whose albedo is `o1,0,<skew>`: 1 facing the ray, 0 at 90°. A convex
Lambertian in the white furnace renders its albedo at the camera ray's
angle, so each ring reads the map directly. Octane 1024 raw, 58 s. Skew 1
gives x = θ / 90° per ring (1 − ring: 0.041 → 0.657; asin of the ring's
mid radius 0.032 → 0.647). Against that x, 1 − ring:

| skew | Octane, centre → rim | x^s | 1 − (1 − x)^(1/s) |
|---|---|---|---|
| 0.5 | 0.078 0.296 0.507 0.700 0.878 | 0.202 0.404 0.550 0.677 0.811 | 0.080 0.296 0.508 0.699 0.882 |
| 1.5 | 0.002 0.061 0.161 0.308 0.533 | 0.008 0.066 0.166 0.310 0.533 | 0.028 0.112 0.213 0.335 0.510 |
| 1.865 | 0.001 0.030 0.103 0.227 0.451 | 0.003 0.034 0.107 0.233 0.457 | |
| 2 | 0.001 0.022 0.087 0.206 0.429 | 0.002 0.027 0.091 0.210 0.432 | |
| 3 | −0.001 0.001 0.024 0.094 0.283 | 0.000 0.004 0.028 0.096 0.284 | |
| 4 | 0.001 0.001 0.008 0.046 0.186 | 0.000 0.001 0.008 0.044 0.186 | |

(every other ring shown). **x^s for s ≥ 1, 1 − (1 − x)^(1/s) below**,
every ring within 0.006 (the ring's own blur): OTOY's manual states the
two branches the other way round. Every solved falloff (resin,
architectural, the legacy metal rim, the film tint) has s ≥ 1, so no
export is wrong; the solver's bounds allow s down to 0.2 and its shape
assumes x^s there (the prepared patch, below, draws Octane's curve).

## 2. Does the excess hold under the Falloff Mix? (probes `exp` / `flat`)

The same export as exported (`exp`), and with U5 / U6's transmission the
flat head-on k (`flat`, the falloff off). Octane 1024 raw, 73 – 74 s.
Render / Octane's model (Smith × `OCTANE_SPHERE_PLAIN`'s excess, the
exported pins), centre → rim:

| sphere | `exp` (falloff) | `flat` |
|---|---|---|
| U5 r 0.75 (α 0.6), q 0.62, s 1.87 | 1.035 1.015 1.016 1.010 1.023 1.011 1.021 1.027 1.041 | 1.032 … 1.029 |
| U6 r 1 (α 0.71), q ≈ 0, s 3.1 | 0.982 1.017 1.021 1.026 1.025 1.017 1.041 1.070 1.078 | 0.980 … 1.031 |

U5: the excess holds under the falloff (1.04 v 1.03 at the rim). U6: not
at the rim — with q ≈ 0 the falloff keeps only the ring's near-head-on
paths, and a per-entry-ring factor measured over all of the ring's paths
is 5 % short for them. The raw render and the denoised Faithful render of
#201 agree (U5 rim 0.349 v 0.348): the denoiser is not in it.

## 3. The Iray side: the model against the references in git

`falloff_rings`' Iray model (`libbsdf` at `legacy_rough` × `legacy_loss`
× τ) over Iray's rings (Iray Server 5000, `furnace-rings.py measure` on
the projected discs):

| sphere | model / Iray, centre → rim |
|---|---|
| U4 white r 0.5 c0 | 0.993 0.976 0.987 0.986 0.982 0.979 0.983 1.003 1.010 |
| U5 white r 0.75 c0 | 1.018 1.018 1.032 1.005 1.001 1.029 1.008 1.058 1.084 |
| U6 white r 1 c0 | 1.059 1.008 0.999 1.020 1.005 1.027 1.022 1.071 1.023 |
| U7 grey r 0.5 c0 | 0.964 0.969 0.977 0.969 0.975 0.968 0.971 0.992 1.001 |
| R5 grey r 0.25 c0 | 0.988 0.991 0.988 0.994 0.995 0.996 0.997 0.997 0.994 |
| R1 grey r 0.25 | 0.980 1.002 0.996 0.986 0.981 0.987 0.999 0.998 0.994 |
| R7 grey r 0.5 | 0.973 0.975 1.014 0.965 0.967 0.993 0.974 1.033 1.080 |

Two findings:

- **`RESIN_LOBE` is a pre-#146 reading.** It was measured on
  2026-11-11 (2e784c9) with the discs found in the image; the projected
  discs (#146, 2026-11-13) read the same reference U1 – U6 as 0.969 /
  0.962 / 0.913 / 0.687 / 0.452 / 0.317 (table 0.971 / 0.963 / 0.920 /
  0.678 / 0.462 / 0.324). At r 0.75 the old value makes τ 1.012 (the
  lobe at τ 1 keeps 0.451 — the new reading), and τ^m on the rim's long
  paths is 2.5 % of U5's 8 %. With the re-read table: U5 0.996 … 1.035
  1.060, U6 1.037 … 1.048 1.000, U7 0.976 … 1.014, U4 1.006 … 1.025, R5 /
  U3 / R1 down ~1 % (0.98 – 0.99).
- **Resin's coat is an `mdl 1.0` lobe too, and loses like one.** The model
  puts `legacy_loss` on base events only. Calib-Furnace-MdlLobe (our own
  `mdl 1.0` reflect lobe, white, reference in git) against the model
  with the loss on every event: r 0.2 / 0.3 / 0.4 / 0.5 within 1.4 / 1.8 /
  2.8 / 3.3 % per ring, without it 2 – 18 % over (r 0.7 / 1 the table
  over-keeps: 1.01 – 1.07 / 1.09 – 1.16). Unit test
  `a_reflect_only_legacy_lobe_loses_the_legacy_loss_too`. With the loss
  on the coat (at the coat's r and the event's incoming cosine) and the
  re-read table: R7 0.967 0.972 1.010 0.954 0.959 0.974 0.953 0.989 1.007
  (rim 1.08 → 1.01), R1 0.971 … 0.977 0.964 (rim 0.99 → 0.96).

## 4. The rims, by component (rim ring 0.8 – 0.9, Faithful)

| sphere | Octane render / its model | Octane model / Iray model | Iray model / Iray | total (render / Iray) |
|---|---|---|---|---|
| U5 | 1.04 (excess holds under the falloff) | 0.99 | 1.084 (τ from the stale disk 1.025; `LEGACY_LOBE`'s r 0.75 rim 1.06) | 1.12 |
| U6 | 1.08 (1.03 flat: the excess under q ≈ 0) | 1.025 (q pinned at 0) | 1.02 | 1.13 |
| R7 | ~1.07 | 1.075 (q pinned at 0) | 1.08 (the coat's missing legacy loss: 1.01 with it) | 1.24 |

R7's Octane model can't come down to Iray's rim with any transmission
falloff: with q = 0 what is left at the rim is reflection — the
coating's (w · F at grazing, physical in Octane) and the base's specular,
which carries the tint × k but no falloff. In Iray the same coat
reflection loses g(r, θ) (§ 3), the base's 1 − w · max(F(n·k1), F(n·k2))
takes the rest. Drawing that in Octane needs a falloff on the coating
(and maybe the specular) — a new construction to probe (how Universal
splits a coating colour below 1 between the reflection and the base, as
#201 did for the specular).

## 5. The export patch, not rendered

Prepared (in `~/Library/Caches/orbis/calib/c205/c205-export.patch`, and
in the issue): `RESIN_LOBE` re-read on the projected discs, `legacy_loss`
on the coat events of a legacy (`mdl 1.0`) `Libbsdf` stack (resin and
architectural glass; the Uber glass has no legacy stack), and the solver's
falloff shape drawn with Octane's curve (`falloff_curve`). `cargo test`
passed with it. The scene check — Faithful and Enhanced of
Calib-Furnace-Resin / -Resin2 / -Resin3 / -ArchGlass / -ArchGlass2 /
-ArchGlass3, Calib-Resin / -ArchGlass — never rendered: Octane X quit
before the first job and had no Script menu within 2 minutes (the screen
was locked). Per the loop's rule not retried; only the test that holds
the reflect-only loss is committed, the export is unchanged.

Expected from the models (not rendered): U5 disk 1.02 → ~1.00, rim 1.12 →
~1.09; U6 disk 1.07 → ~1.04, rim → ~1.11; R7 disk 1.03 → ~1.01, its rim
barely (the Octane side is pinned); R1 / R5 / U3 1 – 2 % down (to
0.99 – 1.00 of Iray in the furnace).

## Process lessons

- A measured table is as old as the measuring tool: `RESIN_LOBE` predates
  the projected discs, and its 2 % at r 0.75 was a quarter of a rim. When
  a residue sits on a sphere, re-read the table's own input with today's
  tool before suspecting the model.
- A Lambertian in the white furnace is a direct probe of any texture of
  the view angle: each ring is the texture at a known angle, no transport
  in between.
- A per-ring excess factor is the average over the ring's paths; a
  falloff that selects paths (q ≈ 0) breaks it. Keep the excess per
  event, or measure it under the falloff, if U6 matters.
- A screen lock between two Octane batches ends the session's renders:
  the first batch (three probes, 00:39) ran, the next (00:55) found no
  Script menu.
