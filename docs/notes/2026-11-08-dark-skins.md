# 2026-11-08 — Enhanced's dark skins after the walk weight (#84, #88)

#84 (split from #19): Enhanced's dark skins came out bright in G / B
(Monique ~1.2 / 1.3 / 1.4 × Iray), put down to Octane's random walk adding
~0.01 of albedo to a dark colour (probes 65 – 67, measured at subsurface
1). #86 then weighted the walk as Iray's translucency (S = w · Tc, the
walk rendered at S²). This session re-checks the dark skins under every
light against the existing Iray references, and looks at Twosret's backlit
ears (#88, 0.47 × Iray's red).

Octane 256 samples, Enhanced + Final, master 39533bc. Octane / Iray,
linear, R / G / B: skin = the median of Calib-Skin's face and body
regions, ears = the two ear boxes (#86's cmp.py); grey = the grey sphere
left of the figure (the light itself). Work dir
`~/Library/Caches/orbis/calib/s84` (`exp.sh`, `cmp.py`, `octane/e0`,
`octane/x1`).

## Step 1: the walk's weight on the dark skins

The exported `subsurface` (S, the face material): Monique 0.45, Twosret
0.46, Gabriela 0.35 – 0.55, Kayo 0.63; the bodies 0.51 – 0.69. Octane
renders the walk at S² (probe 68), so the walk carries 0.2 – 0.4 of what
it did at S 1, and probe 65's +0.01 floor shrinks with it to ~0.002 –
0.004 of albedo.

## Step 2: front, environment and backlight (e0)

| | front | HDRI | HDRI / grey sphere | backlit ears |
|---|---|---|---|---|
| Monique | 1.00 / 1.00 / 1.07 | 1.11 / 1.00 / 1.18 | 1.03 / 0.94 / 1.12 | 1.20 / 0.50 / 0.00 |
| Twosret | 0.94 / 0.96 / 1.00 | 1.04 / 1.00 / 1.15 | 0.97 / 0.94 / 1.09 | 0.44 / 1.00 / 0.43 |
| Kayo | 0.93 / 1.00 / 1.00 | 1.06 / 1.05 / 1.13 | 1.02 / 0.98 / 1.06 | 1.02 / 0.68 / 0.08 |
| Gabriela | 0.94 / 0.96 / 0.96 | 1.06 / 0.97 / 1.07 | 1.02 / 0.90 / 1.01 | 0.97 / 1.00 / 0.21 |

The grey sphere: front 0.98, HDRI 1.04 – 1.07 in every channel — the
environment itself comes out ~5 % bright in Octane, the same on every
figure. With that divided out the dark skins sit where the light ones and
the G8F skin do (Calib-HDRI 1.07 / 1.06 / 1.08, G8F HDRI 1.04 / 1.05 /
1.11; blue ~+10 % under the HDRI on every skin). Kayo against Gabriela
(alike albedos, #84's +0.016 against −0.001 in blue): 1.06 against 1.01
blue under the HDRI, 1.00 against 0.96 front — within the spread of the
light skins. The ears' G / B are thousandths in both renderers (noise).
Nothing dark-skin specific is left: the excess was S 1.

## Step 3: the w0 variants are another material path

Iray's `-w0` variants (Translucency Weight 0) were meant to split off the
skin's diffuse layer. Octane's export of them:

| w0 | front | HDRI / grey |
|---|---|---|
| Monique | 0.98 / 1.06 / 1.16 | 1.08 / 1.13 / 1.38 |
| Twosret | 0.96 / 1.16 / 1.41 | 1.03 / 1.08 / 1.42 |
| Kayo | 0.96 / 1.01 / 1.02 | 0.97 / 0.99 / 1.04 |
| Gabriela | 0.93 / 1.02 / 1.06 | 0.98 / 1.05 / 1.14 |

But a surface without translucency doesn't take the skin path: the w0
exports have no skin bake and no walk — Universal, with the dual lobe as
uniform values. Twosret's front fits base × 0.88 + 0.034 white (R / G / B
predicted 0.98 / 1.16 / 1.44), a too strong white lobe: #81 (Universal
ignores the Dual Lobe Specular Reflectivity map; it already cites
Monique-front-w0). Not a skin-rule decomposition and not #84; the numbers
go to #81.

## Step 4: Twosret's backlit ears — her padding is grey (#88)

Twosret 8's exported detail mask had orange-red rims around every UV
island, the ears' islands with the widest. Her maps (D and the others)
pad the islands with a flat grey 238, all four corners equal;
`padding_colour` (#86) took only white (≥ 250) or black (≤ 5), so the
32 px low-pass carried the grey into the islands, D / D_low fell under
0.35 along the rims and the mask took them out of the walk — Monique's
#86 case with another colour.

Fix: a padding is any flat colour of the four equal corners. Of the
DAZ Genesis 8 textures on this machine, the new ones it catches besides
Twosret are normal maps (128 / 128 / 255) and make-up overlays (Clara 8.1
120 / 85 / 65, Nishi's MU maps) — none of them a skin bake's input.
Exports of Calib-Skin's other eight figures (`-back`) byte-identical before
and after.

| Twosret | e0 | x1 (any flat padding) |
|---|---|---|
| backlit ears | 0.44 / 1.00 / 0.43 | **1.11** / 0.69 / 0.00 |
| front | 0.94 / 0.96 / 1.00 | 0.93 / 0.96 / 1.00 |
| HDRI / grey | 0.97 / 0.94 / 1.09 | 0.99 / 0.94 / 1.09 |

The ears' red now within ±15 % on eight of nine figures; Nishi (1.27, the
only g ≠ 0 skin) stays in #88.

## Process

- The grey sphere in the frame told the light from the skin in one
  number; the HDRI's ~+5 % is the environment, not the skins.
- Looking at the exported textures (the detail mask) found #88's cause
  before any Iray variant was needed.
- A `-w0` Iray variant splits Iray's material, but Octane's export of it
  changes path; check what the export did before reading a variant as a
  decomposition.
- Rendering 24 Calib-Skin scenes at 256 samples took ~70 min (the HDRI
  ones slowest).
