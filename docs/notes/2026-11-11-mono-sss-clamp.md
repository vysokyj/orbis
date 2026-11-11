# 2026-11-11 Mono SSS Amount clamp (#105)

Goal: `src/iray.rs` clamps a mono SSS Amount to 1 (`sub_surface_scattering_amount`,
confirmed by `calib/references/mi/Calib-SSSColor.json`: the Amount 2 sphere reaches
Iray as `[1, 1, 1]`), but the Octane side (`volume_albedo`, `sss_radius`, and
`scattering`'s fallback without Iray args) read the raw channel: Amount 2 had twice
Iray's σs. The issue asked to decompose again rather than only clamp, since the
square root in `volume_factor` might have been compensating.

Scene: Calib-SSSColor (Uber spheres on Victoria 8's volume, Translucency 0.9; Iray
reference in git), enhanced-final, 400 samples, ~1 min per render. No new scene.

## Sources

- DAZ's `irayubermaterial.mdl`, `add_uber_volume`: σs = amount / d_s for mono
  (`logarithmic_scatter_enable` false), −ln(amount) / d_s for chromatic. The flag
  changes nothing else in the volume (same `anisotropic_vdf`, same σa), so the
  remaining mono-vs-chromatic gap below is not the flag.
- The `.mi` fixture: V5 (Amount 2) `[1, 1, 1]`, V6 (Amount 0.5) `[0.5, 0.5, 0.5]`.

## Components

α (reduced single-scattering albedo), Calib-SSSColor's volume:

| sphere | σs / cm | α |
|---|---|---|
| V2 chromatic grey | 52 | 0.993 / 0.802 / 0.739 |
| V5 mono 2, raw | 133 | 0.997 / 0.912 / 0.878 |
| V5 mono 2, clamped | 67 | 0.995 / 0.838 / 0.783 |
| V6 mono 0.5 | 33 | 0.989 / 0.721 / 0.643 |

Skin (#19's derived A(α), no fit) against Iray's Victoria 8 variants (subsurface
part, variant ÷ base, from the 2026-10-30 note): predicted ÷ Iray

| variant | Iray | unclamped | clamped |
|---|---|---|---|
| mono 2 | 1.71 / 1.21 / 1.35 | 1.11 / 1.18 / 1.31 | 1.05 / 0.90 / 0.96 |
| mono 0.5 | 1.59 / 0.83 / 0.94 | 1.04 / 0.93 / 0.94 | (same) |

The clamp puts mono 2 where mono 0.5 already was. The old root `volume_factor`
matched Iray's mono 2 on Victoria unclamped (1.65 / 1.24 / 1.40) - the
compensation the issue suspected; the skin doesn't use the root since #19, and on
props it is an acknowledged fit (#87), so it isn't re-tuned here.

## Octane, Calib-SSSColor (Octane / Iray per channel, linear)

| sphere | before | after |
|---|---|---|
| V5 mono 2 | 1.26 / 1.53 / 1.59 | 1.13 / 1.29 / 1.31 |
| V6 mono 0.5 | 1.12 / 1.29 / 1.22 | 1.11 / 1.29 / 1.22 |
| chromatic V1 - V4, V7 | 0.70 - 1.27 | unchanged (+-0.02) |

Predicted from the root alone, V5 would go to ~1.24 / 1.33 / 1.35; Octane's response
to the transmission is steeper than linear (`PROP_VOLUME_LEVEL`'s note), so it came
down more. The two mono spheres now agree; both stay above the chromatic ones - the
thickness effect of #87 (a denser volume passes less through the sphere), which
the albedo alone doesn't carry. Not fitted.

## Rule

`mono_sss_amount`: the mono amount clamped to 0 - 1 everywhere σs is read (volume
factor, skin A(α), walk radius, refractive volume's σs fallback). Spec: volume factor
section. The unit test's mono case now expects the clamped value and Amount 2 ==
Amount 1.

## Process

- Baseline render before the code change, the same scene after: one sphere moved,
  the rest unchanged within noise, which is the check that only the mono path changed.
- `calib-variants.sh` needs the library scene name (the dump is keyed by the scene's
  path), not `calib/scenes/<scene>.duf`.

Open: mono spheres 1.1 - 1.3 x Iray against the chromatic ones (#87).
