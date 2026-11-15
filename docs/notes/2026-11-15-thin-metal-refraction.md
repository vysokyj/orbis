# 2026-11-15 Thin-walled metal under partial refraction (#164)

#134 left Calib-MetalRefr's X1 - X3 (metal 0.75 / 0.3 under refraction
0.3 / 0.7, thin-walled) at 1.17 - 1.22 x Iray while their closed twins
(-Solid) fit at 1.03 - 1.05, beside metals 1.10 - 1.12 and refractions
1.01 - 1.09. Question: is #129's Mix wrong on thin walls?

Answer: no. The excess is the view through the shell, which Iray blurs
and Octane draws sharp (#136, a known limit). No export change.

Setup: Calib-MetalRefr / -Solid re-exported with the current export
(#163, #140, #117, #161, #162 included: the plain renders came out within
0.01 of #134's), enhanced-final, 400 samples, denoised pass. Variants by
editing project.ocs (scratch script, not kept): Octane only, no new Iray
scenes. Numbers: linear Y within 0.6 R of each projected disc
(`sphere_discs.py`). Octane ~2.5 min per render, 10 renders.

## Iray first

Iray's thin / solid by ring (centre 0 - 0.4 R, 0.4 - 0.7, 0.7 - 0.9) against
Octane's:

| sphere | Iray thin / solid | Octane thin / solid |
|---|---|---|
| R1 / R2 | 0.98 0.83 0.80 / 1.01 0.83 0.79 | 0.94 0.79 0.75 / 1.10 0.83 0.73 |
| M1 / M2 | 1.00 | 1.00 |
| X1 | 1.03 0.88 0.83 | 1.24 0.93 0.83 |
| X2 | 1.07 0.85 0.75 | 1.33 0.92 0.73 |
| X3 | 1.07 0.95 0.89 | 1.24 1.00 0.90 |

The difference sits in the middle of the disc: what a ray sees straight
through. Octane / Iray on the thin scene, by ring: X1 - X3 1.22 - 1.29 in
the centre, 1.02 - 1.12 outside; -g0 (no glossy layer, the refraction
reflects nothing) alike (1.26 - 1.32), so not the refraction's Fresnel.

## Decomposition (Octane variants of the thin scene)

| variant | X1 | X2 | X3 | X4 |
|---|---|---|---|---|
| as exported | 1.194 | 1.222 | 1.174 | 0.814 |
| shell's inside off (metal part black on back faces, Polygon Side) | 1.169 | 1.208 | 1.143 | 0.764 |
| front off (Polygon Side inverted: metal part black on front faces) | 0.904 | 1.191 | 0.824 | 0.706 |
| front reflections only (refraction part's transmission 0) | 0.421 | 0.105 | 0.453 | 0.286 |

- **The inside of the shell** (metal seen through the front wall) is 1 - 3 %
  of the sphere: not the excess.
- **Mix vs one node**: R1 / R2 rebuilt as the refraction Mix (the
  refraction part at transmission 1 and albedo 0, the rest the diffuse
  node without transmission) render 0.96 / 1.09 against the single node's
  1.01 / 1.09, thin; 0.99 / 1.01 against 1.05 / 1.04 closed. Octane's Mix
  splits a thin wall's transmission linearly; the Mix is not it.
- **Transmitted part** T = render - front reflections (Iray's estimated as
  Iray - front / 1.12, the metals' ratio):

  | | X1 | X2 | X3 | X4 |
  |---|---|---|---|---|
  | thin, T Octane / Iray | 1.24 | 1.23 | 1.20 | 0.70 |
  | closed | 1.01 | 1.03 | 0.99 | 1.00 |

  The whole excess is in what passes the shell.

## The blur

Octane renders thin-walled transmission unblurred (#136); Iray spreads it
by the glossy roughness (0.2). The background each sphere sees through
was rendered with all spheres clear (opacity 0) and Gaussian-blurred:
mean(B) / mean(blur B) within 0.6 R:

| σ px | 8 | 16 | 24 | 32 | 48 |
|---|---|---|---|---|---|
| R1 | 1.00 | 0.95 | 0.88 | 0.82 | 0.72 |
| R2 | 1.02 | 1.07 | 1.11 | 1.15 | 1.20 |
| X1 - X3 | 1.03 | 1.10 | 1.18 | 1.24 - 1.25 | 1.34 |
| X4 | 0.99 | 0.83 | 0.65 | 0.53 | 0.40 |

X1 - X3 sit before the floor's back edge: the bright floor fills the lower
two thirds of the disc, the dark backdrop the rest and the surround, so
a blur takes light out of the disc. Octane's render with T scaled by that
loss (Xop + T mean(blur B) / mean(B)):

| σ px | 20 | 24 | 28 | 32 |
|---|---|---|---|---|
| X1 | 1.10 | 1.08 | 1.06 | 1.04 |
| X2 | 1.09 | 1.06 | 1.03 | 1.01 |
| X3 | 1.09 | 1.07 | 1.05 | 1.03 |
| X4 | 1.00 | 1.09 | 1.19 | 1.29 |

At σ 24 (#136 measured R2's edge at 24 - 32 px) X1 - X4 land at 1.06 - 1.09,
beside the metals 1.10 - 1.12 and the closed twins 1.03 - 1.05. R1 / R2
are on other backgrounds (R1 a bright surround, gain under blur): their
fit was the luck of the place. A check, not a rule: the shadow of the
spheres is missing from the clear render, and the real blur is angular,
two walls deep.

#136's "over an even background the blur moves the mean by <= 4 %" was
wrong for X2: its background isn't even. The spec now says so.

-TL (translucency 0.5) shows the same pattern (X1 - X3 1.11 - 1.17 thin,
1.05 - 1.16 closed); not re-split. #172 (rough metals r >= 0.5 under a
camera light) is unrelated: these metals are at r 0.2 under the HDRI.

## Process

- The front-reflection variant (the refraction part's transmission 0)
  split the render into "front" and "through" in one render per scene,
  and the closed twin made the through part's Octane / Iray readable
  without any Iray variant.
- Octane's Polygon Side texture (`NT_TEX_SIDE` = 89, Ray Switch = 173;
  `scripts/dev/octane-probe-nodetypes.lua` lists the ids) as a Mix amount
  switches a part off on back faces: a cheap way to take the inside of a
  shell out of a render.
- Comparing per-sphere ratios across spheres at different places mixes
  the backgrounds in: X vs R said "the Mix overshoots", the same sphere
  thin vs closed said "the view through".
