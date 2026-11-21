# 2026-11-21 — Resin's rough coat, decomposed (#211)

Follow-up of #205 / #207. Goal: R7's rim (Calib-Furnace-Resin, grey r 0.5,
coat 0.95), U6 / U5 / U4 / U7 (Calib-Furnace-Resin2). No Iray (the server
was flaky; the references in git suffice). Octane X, Faithful final; probes
on the Faithful export `--no-denoise`, re-pinned with `orbx-variants.py
set=`, 1024 samples raw, `furnace-rings.py measure` (projected discs).

## 1. Re-measured on HEAD (55a6017, Faithful 400, denoised)

Rim = ring 0.8 – 0.9. Octane / Iray:

| sphere | disk < 0.75 | rim | before #207 (19:21 renders) |
|---|---|---|---|
| R7 grey r 0.5 c 0.95 | 1.012 | 1.185 | 0.992 / 1.162 |
| U4 white r 0.5 | 1.007 | 1.039 | unchanged |
| U5 white r 0.75 | 1.000 | 1.093 | unchanged |
| U6 white r 1 | 1.044 | 1.107 | unchanged |
| U7 grey r 0.5 | 1.012 | 1.062 | unchanged |

#207 raised R7's Iray model (0.972 → 0.993), the solve raised k, R7's rim
went up 2 %. Resin2 (uncoated) did not move.

R7 by component, rings centre → 0.8 – 0.9:

| | rings |
|---|---|
| Octane render / Octane model | 1.04 1.01 1.03 1.03 1.04 1.04 1.05 1.07 1.07 |
| Octane model / Iray model | 1.055 1.05 1.01 1.05 1.00 0.96 0.94 0.93 1.07 |
| Iray model / Iray | 0.975 0.986 1.026 0.967 0.981 0.996 0.98 1.019 1.035 |

## 2. How Universal splits a coating colour below 1 (probe `split`)

Calib-Furnace-Resin2's spheres re-pinned (specular 0, transmission 0 unless
said; coating roughness 0, IOR 1.49):

| sphere | pins | rings centre → 0.8 – 0.9 |
|---|---|---|
| U1 | albedo 0, coating 1 | 0.038 … 0.052 0.083 (F(1.49) per ring 0.039 … 0.055 0.083) |
| U2 | albedo 0, coating 0.5 | 0.019 … 0.027 0.038: half of F |
| U3 | albedo 1, coating 1 | 1.006 … 0.985 |
| U4 | albedo 1, coating 0.5 | 1.006 … 1.007 |
| U5 | albedo 1, coating 0 | 1.004 … 0.987 |
| U6 | albedo 0, coating Falloff 1 → 0.5, skew 2 | 0.038 … 0.065 (0.79 × U1 at the rim: 1 − 0.5 t² at t 0.645) |
| U7 / U8 | white glass r 0.3, coating 1 / 0.5 | 1.002 … 0.994 / 1.003 … 0.994 |

The coating reflects c · F and the base gets **1 − c · F** (a white base
under a coating of 0.5 loses nothing); a Falloff Map on the coating takes
the hit's angle, as on the albedo (#205). So the Smith model's coat pick
w · F is exact with w × the coating's factor.

## 3. The coat alone (probe `dc`, Calib-Furnace-Resin)

R7 and R1 with transmission 0, specular 0 (the coat over black); R3 = R7's
pins (a twin: within 0.003 of R7); R8 = R7's base alone (coating 0).

| | rings centre → 0.8 – 0.9 |
|---|---|
| R7 coat, Octane render | 0.038 0.038 0.038 0.038 0.038 0.040 0.043 0.050 0.068 |
| R7 coat, Octane (Smith) model | 0.036 0.037 0.037 0.037 0.037 0.039 0.042 0.046 0.059 |
| R7 coat, Iray model (legacy lobe + `legacy_loss`) | 0.031 0.030 0.031 0.031 0.032 0.033 0.035 0.040 0.050 |
| R1 coat, Octane render | 0.038 … 0.044 0.053 0.076 |
| R1 coat, Iray model | 0.036 … 0.043 0.051 0.070 |
| R7 base alone (R8) | 0.274 0.269 0.264 0.254 0.242 0.225 0.205 0.182 0.155 |

Octane's rough coat keeps 1.23 – 1.36 × Iray's model at r 0.5 (1.06 –
1.09 at r 0.25). Single reflection, by angle (Iray's model / Octane's,
same curve): r 0.1 0.98 flat; r 0.25 0.976 → 0.94 at 50°; r 0.5 0.848 flat
to 55°, 1 past 75°; r 0.75 0.72; r 1 0.64.

## 4. The coat drawn down by angle (`coat_falloff`), shipped after § 7

Octane's coating as a Falloff Mix, w × the models' ratio (least squares
over θ, by the silhouette's share; ≤ 1), traced into the base solve (the
Smith coat pick × the factor). R7: head 0.841, grazing 1, skew 2.81; R1
0.956 / 1 / 5.9; r ≤ 0.1 none (within 2 %); lobe-only (both under Iray's
curve; architectural's Schlick v Fresnel at the F0's IOR is a mirror coat's
difference, not this). Patch: `~/Library/Caches/orbis/calib/c211/c211-export.patch`
(with a unit test and the diagnostics); `cargo test` passed with it.

The coat alone with it (probe `cf`): R7 0.032 0.032 0.032 0.032 0.032 0.034
0.037 0.044 0.060 v Iray's model 0.031 … 0.040 0.050 (1.03 … 1.10 1.20,
was 1.23 … 1.36); R1 0.97 – 1.04; R8 (r 0.1) unchanged. The rest at R7's
rim is Octane's rough coat over its own Smith model (1.15 at the rim, not
modelled).

The whole sphere doesn't move: the base takes what the coat gives up (1 −
c · F) and the solve raises k (0.638 → 0.646). Faithful 400: R7 rings
0.302 … 0.221 0.205 (was 0.300 … 0.220 0.205), rim 1.185 → 1.185, disk
1.012 → 1.020; Calib-Resin's spheres the same (R7 1.11). The models agree
(R7 Octane model rim 0.192 → 0.193). The change is a component (lit
scenes: the coat's highlight 1 / 0.85 too bright at r 0.5), not the rim.
The scene check of the other affected scenes (Calib-Furnace-ArchGlass and
Calib-ArchGlass: two rough-reflection spheres each, Calib-NVIDIA2 / 3,
Enhanced) never rendered: Octane X quit before the first job and found no
Script menu for 2 minutes (screen locked). Not retried, Octane X left open.

## 5. What holds R7's rim

R7 by path kind (models, at the solved falloff, rim ring):

| | coat | base reflection | through |
|---|---|---|---|
| Iray model | 0.052 | 0.035 | 0.091 |
| Octane model | 0.055 | 0.029 | 0.108 |

The excess is in the paths that pass through: Iray's ring 0.7 – 0.8 →
0.8 – 0.9 drops ×0.60, Octane's ×0.76 with the falloff already at q = 0
(the solver's bound). Iray's cut is `fresnel_layer`'s base weight 1 − w ·
max(F(n·k1), F(|n·k2_refl|)) at the macro normal, which a rough base meets
near grazing (F → 1, the base keeps 0.05); Octane's transmission falloff
can't go below 0 at 90°, and a steeper single curve costs the middle rings
(the solve's optimum: skew 2.87, rings 0.5 – 0.8 4 – 8 % under). Not
physical on Iray's side ("a loss that grows with roughness and isn't the
surface's energy balance", `mdl_layer`): the project's Octane-over-Iray
rule would leave it; a two-map construct (a product of two Falloff Maps)
would draw it. The user's call (#211).

## 6. U4 – U7 (uncoated), by component

Models at today's tables (Octane model / Iray model, rings to 0.8 – 0.9):
U4 0.98 … 0.98 1.01, U5 0.99 … 0.98 0.99, U6 1.00 … 0.98 1.03, U7 0.99 …
0.99 1.01 — the models meet. What is left: Iray's model over Iray (U5
1.06 at the rim, `LEGACY_LOBE`'s r 0.75 row) and Octane's render over its
model under the falloff (U6 1.08 v 1.03 flat, #205's `exp` / `flat`
probes). The flat `OCTANE_SPHERE_PLAIN` excess at IOR 1.5, α 0.7 is 1.02 –
1.06 on every ring, so a per-event law that only selects paths can't lift
it to 1.08 at the rim; the falloff's own reading inside a rough sphere is
the next suspect. Needs probes of its own: follow-up issue.

## 7. The scene check of `coat_falloff`, shipped (21:20 – 21:55)

The user's call (5d305a7): R7's rim stays Octane's, no two-map falloff.
Then the patch's before / after: HEAD and HEAD + patch, Enhanced and
Faithful final, 400 samples denoised, Calib-Furnace-ArchGlass / 2 / 3,
Calib-ArchGlass, Calib-NVIDIA2 / 3, Calib-Resin, Calib-Furnace-Resin /
2 / 3 (2 × 8 min per side). Octane X was idle, screen unlocked.

Which materials get the coat falloff (the exports): R1 / R3 (r 0.25:
0.908 → 0.95, skew 5.9), R7 (0.799 → 0.95, 2.81), R8 (r 0.1: 0.942 →
0.921, 0.55) in Calib-Resin and Calib-Furnace-Resin, U8 in Resin2 / 3,
Calib-NVIDIA2's A7 grey resin. No architectural sphere of any scene (their
reflections are smooth or within 2 %): their exports and renders don't
change (largest move H8 Faithful rim 1.084 → 1.067, the denoiser's: the
noise floor of a rim ring, 1 – 2 %; p99 moves up to 4 % on spheres the
patch doesn't touch).

Octane / Iray, disk < 0.9 R, rim 0.8 – 0.9, p99 (before → after):

| sphere | Enhanced | Faithful |
|---|---|---|
| Calib-Resin R7 | 1.230 → 1.209, 1.549 → 1.501, 2.17 → 2.06 | 1.222 → 1.205, 1.548 → 1.508, 2.15 → 2.05 |
| Calib-Resin R1 | 1.045 → 1.044, 1.027 → 1.025, 1.090 → 1.086 | 1.047 → 1.048, 1.046 → 1.040, 1.088 → 1.071 |
| Calib-Resin R3 | 1.044 → 1.043, 1.047 → 1.039, 1.103 → 1.080 | 1.045 → 1.045, 1.062 → 1.056, 1.092 → 1.081 |
| Calib-NVIDIA2 A7 | 1.080 → 1.079, 1.159 → 1.153, 1.32 → 1.31 | 1.083 → 1.082, 1.173 → 1.164, 1.33 → 1.29 |
| Furnace-Resin R7 | 1.069 → 1.072, 1.199 → 1.194 | 1.062 → 1.066, 1.211 → 1.202 |
| Furnace-Resin R1 | 1.017 → 1.018, 1.016 → 1.012 | 1.018 → 1.021, 1.032 → 1.026 |
| Furnace-Resin R8 | 1.020 → 1.022, 1.031 → 1.034 | 1.022 → 1.024, 1.037 → 1.040 |

Lit scenes lose part of the coat's excess highlight (R7 5 % on the p99,
3 % on the rim); the furnace moves ≤ 0.4 % (the base takes the coat's
share back, § 4). Nothing gets worse past the floor: shipped.

## 8. U4 – U7: how Octane reads the falloff (probe `fr`)

Calib-Furnace-Resin2's Faithful export `--no-denoise`, re-pinned, 1024
raw, 61 s. Uncoated white spheres, specular 1, IOR 1.49, the
transmission a Falloff Mix head 1 → grazing q, skew s. The models:
`furnace-rings.py model --model smith --mode rt --loss` with the falloff
as a per-event factor, one hypothesis per reading.

Smooth (roughness 0), rings centre → 0.8 – 0.9:

| sphere | Octane | through events at their incoming cosine (the model) | every event | outside hits only | camera ray |
|---|---|---|---|---|---|
| U1 q 0, s 1 | 0.939 0.852 0.760 0.672 0.586 0.501 0.419 0.344 0.283 | 0.933 0.847 0.756 0.667 0.581 0.498 0.418 0.343 0.280 | … 0.307 0.220 | … 0.488 0.404 | … 0.254 0.195 |
| U3 q 0.5, s 1 | 0.973 0.929 0.880 0.832 0.782 0.733 0.681 0.630 0.582 | 0.966 0.922 0.874 0.825 0.776 0.727 0.678 0.628 0.582 | … 0.547 | … 0.702 | … 0.501 |
| U2 flat | 1.001 … 0.997 | 1 | | | |

Octane reads the falloff exactly as the model does: a factor on every
transmission event at that event's incoming macro cosine, none on
reflections. The falloff's reading is not what lifts U6.

Rough (α 0.841, U6's), Octane v the Smith model:

| sphere | Octane | model | Octane / model |
|---|---|---|---|
| U4 flat | 0.803 0.795 0.783 0.766 0.746 0.725 0.704 0.679 0.644 | 0.649 … 0.595 0.552 | 1.24 1.23 1.21 1.20 1.18 1.16 1.14 1.14 1.17 |
| U5 q 0, s 1 | 0.666 … 0.234 0.190 | 0.527 … 0.180 0.132 | 1.26 … 1.30 1.44 |
| U7 q 0, s 3.1 | 0.792 … 0.546 0.450 | 0.642 … 0.466 0.367 | |
| U8 q 0.5, s 1 | 0.743 … 0.452 0.419 | 0.587 … 0.354 0.301 | |

What the falloff keeps of the flat sphere, Octane's (falloff / flat)
over the model's: U5 1.02 1.02 1.03 1.03 1.04 1.05 1.08 1.14 1.23; U7 1.00
1.00 1.00 1.00 1.00 1.01 1.01 1.03 1.05; U8 1.02 1.03 1.03 1.04 1.05 1.06
1.07 1.12 1.19. So under a falloff Octane keeps relatively more towards
the rim: its rim paths leave through the inner surface closer to head-on
than the Smith model's (an escaping path has two transmission events, the
entry at the camera ray's cosine in both; the exit's cosine is the
difference). U6's 1.08 v 1.03 flat is this (U7 is U6's shape on white:
1.05 at the rim). Discrete model variants tried, none fits, nothing
fitted: height-correlated Smith G (within 0.5 % of separable), α = the pin²
(0.707) / 0.6 / 0.5 (the flat shape comes closer, the falloff ratio
doesn't), the exit read at max(cos in, cos out). It is the rough
transport inside, not the texture: next a rough exit probe of its own
(#212).

U5's other part, Iray's model over Iray at the rim (1.06): `LEGACY_LOBE`'s
r 0.75 row (grazing 0.629, as high as r 0.7's 0.628 where the trend gives
~0.61) was fitted in #191, before #205 / #207 changed the model it is
read under (`RESIN_LOBE` re-read, the coat's loss, |n·k2_refl|). A refit
of rows 0.6 – 1 under today's model against the same references (no Iray
needed): #212 too.

## Process lessons

- Split a coated sphere into coat alone / base alone / twin in one Octane
  job (three spheres of the same export): the twin proves the re-pin, and
  the two parts are the whole decomposition for 72 s.
- A component brought to Iray's can leave the whole render unchanged when a
  solve sits on top: the solve hands the difference to the next component.
  Check the model first (`print_resin_now`): it predicted the null result
  before the render.
- A smooth sphere is the falloff's own test: its paths are fixed by the
  entry radius, so every reading predicts a different ring profile and one
  render settles it; keep the rough question (transport) apart from the
  texture's.
- Before / after on denoised 400-sample renders: rims move 1 – 2 % and
  p99 up to 4 % on spheres the patch doesn't touch; judge a patch on the
  spheres it changes against those.
- A batch that chains several `octane-render.sh` runs keeps retrying after
  the screen locks; one run per background command, or stop the chain at
  the first failure.
