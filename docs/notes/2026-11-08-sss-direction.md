# 2026-11-08 — SSS Direction and the walk's radius (#88)

After #86 and #84 eight of the nine Calib-Skin figures' backlit ears are
within ±15 % of Iray's red. Left: Nishi, 1.27 — the only store skin with
SSS Direction (g −0.5 on the skin, −0.35 on the mouth / teeth). Since #83
the bake carries g (the reduced albedo σs (1 − g) / (σa + σs (1 − g)),
`volume_albedo`) and the walk's own anisotropy is 0.

Octane 256 samples, Enhanced + Final, master f387675. Octane / Iray,
linear, R / G / B, #86's cmp.py (skin = face and body regions, ears = the
two ear boxes); Calib-Eye / -HDRI with #83's cmp.py. Work dir
`~/Library/Caches/orbis/calib/s88` (`exp.sh`, `ORBIS_X88` flags during the
session: `r` reduced radius, `m` detail mask off; removed before the
commit).

## Step 1: what Iray does with g (DAZ's MDL)

`add_uber_volume` (`irayubermaterial.mdl`): `scattering:
df::anisotropic_vdf(directional_bias: SSS Direction)`, with the
scattering coefficient σs unreduced (−ln SSS Color / d_s, or SSS Amount /
d_s). MDL spec: `directional_bias` −1 back, 0 isotropic, 1 forward — a
Henyey-Greenstein phase. So Iray's volume is (σa, σs, HG g).

Our walk: isotropic (`subsurfaceAnisotropy` 0), colour from the reduced
albedo α' = σs (1 − g) / (σa + σs (1 − g)) — the similar medium of
similarity theory (σs' = σs (1 − g), g 0). But `sss_radius` took the
mean free path of the *unreduced* σs, 1 / (σa + σs). The walk was a
medium of the right albedo and the wrong density: for g −0.5 its σt 1 /
1.46 of the similar medium's in red (Nishi's skin σa 0.41, σs 4.4 / cm,
radius 3.19 mm, reduced 2.19 mm). Diffusion: the decay length through a
slab, 1 / √(3 σa σt'), comes out σt' / σt too long when the walk's σa is
(1 − α') / r: more red through an ear. That is the sign of Nishi's
excess, and on g 0 skins nothing changes.

## Step 2: does similarity hold at an ear's thickness? (slab Monte Carlo)

Iray's volume at g isn't exactly the similar isotropic medium in thin
layers (ballistic light sees σt, not σt'). Rather than an Iray batch: a
small Monte Carlo of a backlit slab, η 1.4 Fresnel at both faces, normal
incidence (`scripts/dev/sss-slab-mc.py`). Checked first against Iray's
own Calib-SSSThick slabs (g 0, Victoria 8's red σa 0.043, σs 0.356 /
mm):

| slab | 1 mm | 2 | 3 | 5 | 9 |
|---|---|---|---|---|---|
| Iray | 0.730 | 0.561 | 0.442 | 0.280 | 0.122 |
| MC | 0.741 | 0.585 | 0.464 | 0.299 | 0.125 |

Within 1 – 7 %. Then HG g against the similar isotropic medium:

| T(HG g) / T(isotropic, σs (1 − g)) | 1 mm | 2 | 3 | 5 | 9 |
|---|---|---|---|---|---|
| Nishi's red (σa 0.041, σs 0.436 / mm, g −0.5) | 1.03 | 1.04 | 1.06 | 1.08 | 1.10 |
| G8F base (Victoria's σ, g −0.65) | 1.03 | 1.06 | 1.09 | 1.13 | 1.22 |
| isotropic at the unreduced σs / similar, Nishi | 1.10 | 1.16 | 1.24 | 1.37 | 1.66 |

Similarity is right within 3 – 10 % at an ear's few millimetres; the
unreduced density is 10 – 66 % off. Expected after the fix: Octane a few
percent under Iray in red, where the radius goes.

## Step 3: the variants on Nishi (back)

| | skin (back) | ears |
|---|---|---|
| m0 (master) | 1.00 / 0.88 / 0.77 | 1.24 / 1.00 / 0.14 |
| m (detail mask off) | 1.00 / 0.88 / 0.77 | 1.33 / 1.08 / 0.18 |
| r (reduced radius) | 1.00 / 1.00 / 0.77 | **0.88** / 0.82 / 0.53 |
| rm (both) | 1.00 / 0.88 / 0.77 | 0.93 / 0.85 / 0.50 |

- The mask isn't it: off, the ears gain (1.33 / 0.93 without / with the
  reduced radius), the excess is the walk.
- The reduced radius takes the red to 0.88 — under Iray by about what
  the Monte Carlo says the HG phase passes over the similar medium. The
  back's body G also comes to 1.00 (0.88). Ears' G / B are thousandths in
  both renderers (noise).
- Iray's `-w0` references exist (Nishi back / front / hdri), but Octane
  exports w 0 as Universal (#84, step 3): no decomposition of the walk.
  No whalf / tcwhite was needed: the excess sat in one parameter the MDL
  names.

## Step 4: everything g touches (r)

The G8F base skin has g −0.65 (mono SSS) too: Calib-Skin-Base,
Calib-Eye, Calib-HDRI change with it.

| | before (#86 step 8, #84) | r |
|---|---|---|
| Nishi front | 0.95 / 0.98 / 0.98 | 0.94 / 0.98 / 0.98 |
| Nishi HDRI | 1.06 / 1.03 / 1.08 | 1.07 / 1.03 / 1.08 |
| Nishi backlit ears | 1.24 / 1.00 / 0.14 | **0.88 / 0.82 / 0.53** |
| G8F front | 0.94 / 1.00 / 1.00 | 0.94 / 1.00 / 1.00 |
| G8F HDRI | 1.04 / 1.05 / 1.11 | 1.04 / 1.05 / 1.11 |
| G8F backlit ears | 0.96 / 1.07 / 1.10 | 0.86 / 1.00 / 1.10 |
| Calib-Eye | 1.05 / 1.07 / 1.10 | 1.05 / 1.07 / 1.10 |
| Calib-HDRI | 1.07 / 1.06 / 1.08 | 1.07 / 1.06 / 1.08 |

The G8F ears' red goes 0.96 → 0.86: of it the diffuse layer alone gives
0.58 (#86 df), the walk (w · Tc 0.14) 0.38 → 0.28, ×0.73, as Nishi's
(×0.71). Still within ±15 %, and the Monte Carlo puts this volume's HG
excess over the similar medium at 6 – 13 % — the same few-percent under
Iray as Nishi. Not compensated: it's Iray's phase in a thin layer, which
an isotropic walk can't have, and the anisotropy that could (#83) changes
the albedo the bake gives it.

The other seven figures have g 0: (1 − 0) leaves σs exactly as it was,
their exports byte-identical (Victoria's radius 3.886 mm before and
after).

## Result

Backlit ears' red within ±15 % on all nine Calib-Skin figures (Nishi
0.88, G8F 0.86 the lowest); fronts, HDRI, Calib-Eye and Calib-HDRI
unchanged. No fitted number: the radius follows the same σs (1 − g) as
the colour.

## Process

- The MDL first: one line (`anisotropic_vdf` with σs unreduced) and the
  radius's formula gave the cause before any render; the variants (mask
  off, reduced radius) took one batch of four.
- The Iray side by a Monte Carlo validated on Iray's own g 0 slabs
  instead of an Iray batch: ten seconds, and it says how far similarity
  goes in thin layers.
- A grep for the other skins with g (the G8F base, −0.65) before calling
  the change "Nishi only".
