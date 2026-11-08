# 2026-11-08 — The skin's random-walk weight (#86)

Since #83 Enhanced bakes the Genesis 8 base skin like every skin and sends
all of the colour through OpenPBR's random walk (`subsurface` 1). Front
light came out right; the backlit ears 2.29 / 1.61 / 1.30 × Iray. #86's
candidate: the walk's weight = Iray's subsurface share b / (a + b).

Octane 256 samples, Enhanced + Final. Octane / Iray, linear, R / G / B:
skin = the median of Calib-Skin's face and body regions, ears = the two
ear boxes (the #19 cmp.py); Calib-Eye / -HDRI as in #83. Work dir
`~/Library/Caches/orbis/calib/s86` (`exp.sh`, `cmp.py`, variants m0 = master,
x1, sh, df, x2; probes in `p68/`, `p68m/`).

## Step 1: two derived weights, read as a linear blend

OpenPBR's spec (and Octane's manual: "blends between the diffuse and
subsurface scattering") mixes the two lobes linearly. Base = diffuse part /
(1 − S), subsurface colour = volume part / S, then the same colour for any
S. Tried:

- **x1**: S = (w · Tc)², the share Iray sends through its volume (in and
  out through the translucency), so the walk's colour is the volume's own
  A(α);
- **sh**: S = b / (a + b), #86's candidate;
- **df**: S ≈ 0 — the diffuse layer alone (a decomposition, not a rule).

| backlit ears | m0 (S 1) | x1 | sh | df |
|---|---|---|---|---|
| G8F (Base) | 2.21 / 1.61 / 1.30 | 0.58 / 1.00 / 1.10 | 0.61 / 1.00 / 1.10 | 0.58 / 1.00 / 1.05 |
| Victoria | 0.62 / 0.51 / 0.20 | 0.33 / 0.43 / 0.35 | 0.40 / 0.51 / 0.35 | 0.02 / 0.33 / 0.44 |
| Kayo | 0.58 / 0.39 / 0.47 | 0.18 / 0.60 / 0.66 | 0.33 / 0.60 / 0.76 | 0.04 / 0.60 / 0.82 |
| Monique | 0.92 / 0.29 / 0.27 | 0.08 / 0.42 / 0.51 | 0.28 / 0.37 / 0.51 | 0.03 / 0.42 / 0.64 |

| front | m0 | x1 | sh |
|---|---|---|---|
| G8F | 0.95 / 1.00 / 1.00 | 0.88 / 1.00 / 1.00 | 0.88 / 1.00 / 1.00 |
| Victoria | 0.91 / 0.98 / 1.00 | 0.67 / 0.75 / 0.79 | 0.75 / 0.83 / 0.86 |
| Monique | 1.16 / 1.24 / 1.44 | 0.45 / 0.50 / 0.58 | 0.67 / 0.79 / 0.85 |

What it said:
- df: on the G8F skin the diffuse layer alone gives Iray's ear in G / B
  exactly and 0.58 of its red. The walk has to add ~0.42 of Iray's red;
  at S 1 it added 1.63.
- On the chromatic skins the diffuse alone gives almost nothing (red 0.02 –
  0.09): their ears are the volume.
- Both splits darkened every chromatic front by 15 – 55 %, though the
  colours were split to add up. So the blend isn't linear: a fact about
  Octane to measure, not a constant to fit.

## Step 2: probe 68, how Octane blends (`octane-probe-sssweight.lua`)

Victoria's Enhanced export (front, back), every OpenPBR skin material
painted grey, its own radius. Lobes alone (MODE "lobes"), skin regions,
over S 0 / S 1:

| S | diffuse (base 0.5, sss colour 0) | walk (base 0, sss colour 0.5), face | walk, back ears |
|---|---|---|---|
| 0.10 | 1.000 | 0.010 | 0.005 |
| 0.25 | 0.999 | 0.057 | 0.040 |
| 0.50 | 0.999 | 0.231 | 0.192 |
| 0.75 | 1.000 | 0.537 | 0.485 |

The walk goes as S² (0.010 / 0.062 / 0.250 / 0.562). Through an ear it
goes a little steeper (≈ S^2.3). With a black subsurface colour the base
stays whole at any S. Both lobes at once (MODE "mix", 15 pairs) fit
**(1 − S) · base + S² · walk** whenever the subsurface colour isn't black.
In albedo units: (0.5, 0.5, S 0.5) 0.380 against 0.375, (0.25, 0.5, 0.75)
0.338 / 0.344, (0.5, 0.25, 0.25) 0.383 / 0.383. The walk pays S going in
and again coming out. That also explains #19's "at subsurface 0.5 Octane
rendered 0.67 – 0.73 of the colour" (0.5 + 0.25).

This is Iray's translucency: (1 − w) diffuse, and the volume behind the
translucency layer, w · Tc in and w · Tc out. So S = w · Tc (one crossing),
the walk's colour = the volume's part / S² (in the strongest channel
exactly Iray's A(α), the others less by (Tc / Tc_max)²), the base = the
diffuse part / (1 − S). S is a scalar, so take the strongest channel:
max_c(w · Tc_c · mean Tc map_c). G8F 0.14 (Translucency Color 0.30 in red,
map mean 0.58), Victoria 0.79 – 0.83, Zurie 0.15 – 0.23. No fitted number.

## Step 3: the rule (x2, shipped)

| | m0 (S 1, #83) | x2 |
|---|---|---|
| G8F front | 0.95 / 1.00 / 1.00 | 0.94 / 1.00 / 1.00 |
| G8F HDRI | 1.05 / 1.05 / 1.11 | 1.04 / 1.05 / 1.11 |
| G8F backlit ears | 2.21 / 1.61 / 1.30 | **0.86 / 1.00 / 1.01** |
| Calib-Eye | 1.05 / 1.07 / 1.11 | 1.05 / 1.07 / 1.10 |
| Calib-HDRI | 1.09 / 1.07 / 1.10 | 1.07 / 1.06 / 1.08 |
| Angel front | 0.97 / 1.02 / 1.05 | 1.00 / 1.03 / 1.07 |
| Gabriela front | 0.92 / 0.95 / 0.96 | 0.94 / 0.97 / 0.97 |
| Kayo front | 0.98 / 1.09 / 1.15 | 0.94 / 1.03 / 1.03 |
| Monique front | 1.16 / 1.24 / 1.44 | **0.97 / 1.00 / 1.07** |
| Nishi front | 0.92 / 0.97 / 0.95 | 0.94 / 0.98 / 0.95 |
| Nishi HDRI | 1.03 / 1.03 / 1.08 (#83) | 1.06 / 1.03 / 1.08 |
| Twosret front | 1.03 / 1.11 / 1.30 | **0.91 / 0.95 / 1.06** |
| Victoria front | 0.91 / 0.98 / 1.00 | 0.93 / 0.98 / 0.97 |
| Zurie front | 1.00 / 1.00 / 0.99 | 0.94 / 0.97 / 0.97 |

| backlit ears | m0 | x2 |
|---|---|---|
| Angel | 0.93 / 0.68 / 0.29 | 0.61 / 0.88 / 0.67 |
| Gabriela | 1.03 / 0.49 / 0.00 | 0.53 / 0.79 / 0.51 |
| Kayo | 0.58 / 0.39 / 0.47 | 0.40 / 0.60 / 0.76 |
| Monique | 0.92 / 0.29 / 0.27 | 0.38 / 0.61 / 0.50 |
| Nishi | 0.93 / 0.48 / 0.46 | 0.83 / 0.78 / 0.53 |
| Twosret | 0.64 / 0.35 / 0.43 | 0.32 / 0.86 / 0.57 |
| Victoria | 0.62 / 0.51 / 0.20 | 0.54 / 0.64 / 0.31 |
| Zurie | 1.00 / 0.77 / 0.87 | 0.73 / 0.88 / 0.86 |

- Every front within 0.91 – 1.07. The dark skins (Monique, Twosret) aren't
  lifted any more: #19's "the walk brightens dark colours" was mostly S 1
  sending the whole colour through a lobe that Iray only partly feeds.
- The G8F ears are right; that is #86's case.
- The eight chromatic figures' ears: G and B came closer to Iray, red came
  down from 0.58 – 1.03 to 0.32 – 0.83. S 1 had them nearer by accident:
  far too much walk, at a darker walk colour. Open in #86. Iray must pass
  more red through a thin ear than (w · Tc)² · its volume. To split that,
  Iray's back renders are needed at another w and with a white
  translucency map (Iray batch: needs, in #86).
- The `_skin_sss` maps don't clip (Victoria's red median 180 / 255 ≈ A
  0.46), so the clamp isn't the cause.
- Faithful is unchanged (S 0, the whole colour as a diffuse).

## Process

- Three variants and a decomposition (df) in one batch before deciding.
  The front's drop said "the blend", not "the weight".
- Probe first, then the rule: two probe runs (lobes alone, then pairs)
  gave the model in ~30 min. No number came from fitting the figures.
- `ORBIS_X86` env flags for the variants during the session, removed
  before the commit (the cleaned build exports byte-identical to x2).

## Step 4: Iray's ear at another w and Tc (Iray batch)

`make-skin-variants.py` `whalf` (every material's own Translucency Weight
× 0.5) and `tcwhite` (Translucency Color white, no map), the four `-back`
scenes, Iray Server 5000 samples; Octane x2 (shipped) 256 samples, and
`-w0` from both. The translucent part T = ear − (1 − w) · ear(w0), red:

| | T(w) / T(w/2) Iray | Octane | T(tcwhite) / T Iray | Octane |
|---|---|---|---|---|
| Victoria (w 0.9) | 2.50 | 2.05 | 1.42 | 1.72 |
| Kayo (0.9) | 2.88 | 2.81 | 4.06 | 5.00 |
| Monique (0.9) | 3.14 | 4.05 | 11.25 | 11.78 |
| Zurie (0.65) | 4.06 | 4.23 | 4.39 | 5.46 |

Octane follows Iray's scaling in w and Tc (Iray's own exponent in w runs
1.3 – 2.1, lower on the lighter skins). The rule's weight is right. What
is short is a constant per figure: Octane's translucent part over Iray's,
red, at w/2, w and tcwhite:

| | whalf | base | tcwhite |
|---|---|---|---|
| Victoria | 0.63 | 0.52 | 0.63 |
| Kayo | 0.38 | 0.37 | 0.46 |
| Monique | 0.28 | 0.36 | 0.37 |
| Zurie | 0.54 | 0.56 | 0.69 |

(G / B: the parts are a few thousandths, the ratios noise.) The deficit
follows the figure, not w or Tc: lighter skins lose less (Victoria) than
darker ones (Monique), though their volumes (SSS Color, Transmitted Color,
distance 0.125) are the same. So it is the walk's colour: `skin_albedo`
derives it from what the skin reflects, which a darker skin lowers, and a
random walk's transmission through a thin ear falls much faster with its
albedo than its reflectance does. Iray's volume is set by its own
coefficients, the same for these figures. The next step is the walk's
transmission at Iray's volume albedo (α from SSS / Transmitted Color),
not a fit on the ears. Open in #86.

## Step 5: the walk's albedo and mean free path in an ear (probes 69, 70)

Probe 69 (`octane-probe-walkalbedo.lua`): every skin material as the walk
alone (base 0, subsurface 1) at a grey colour A, the export's radius;
Victoria and Monique, front (face, reflectance) and back (ears,
transmission), over A 1:

| A | 0.30 | 0.50 | 0.70 | 0.85 | 0.95 |
|---|---|---|---|---|---|
| Victoria front | 0.36 | 0.60 | 0.83 | 0.96 | 0.99 |
| Victoria ears | 0.21 | 0.44 | 0.75 | 0.92 | 0.95 |
| Monique front | 0.37 | 0.60 | 0.83 | 0.96 | 0.99 |
| Monique ears | 0.21 | 0.46 | 0.79 | 0.96 | 1.05 |

Reflectance ≈ A (Octane's albedo inversion holds), an ear's transmission
≈ A^1.5 — the same on both figures. So a darker walk colour does cost an
ear more than a face, but it doesn't explain step 4: at tcwhite the walk's
colour is the same on Victoria, Kayo and Monique (A(α) of one volume; the
1 / (1 − k · D) term is a few percent), yet Octane's translucent part is
0.63 / 0.46 / 0.37 of Iray's. The Tc map hypothesis (Tc² in the albedo
instead of the weight) is out.

Probe 70 (`octane-probe-mfp.lua`): the export as is, the radius (Iray's
mean free path 1 / (σa + σs), red 2.56 mm) × 0.5 / 1 / 2, ears O / I red:
Victoria 0.23 / 0.53 / 1.20, Monique 0.16 / 0.34 / 0.98. The ear goes
about as mfp^1.2 – 1.5 in Octane. Iray's ears differ less between figures
at tcwhite (red 0.101 / 0.093 / 0.088) than Octane's (0.063 / 0.043 /
0.033): the same volume through ears of different shape (the figures'
morphs) loses more in Octane. Octane's walk, at the radius we give it,
behaves as a denser medium than Iray's volume, more sensitive to
thickness. Whether that is Octane's reading of the radius (a mean free
path, or a diffuse length it rescales by the albedo) the manual doesn't
say; it calls it "the distance light can scatter below the surface".

Next decomposition: one slab of known thickness with a uniform volume (no
maps, Iray's skin volume) in both renderers, transmission against
thickness. That needs a new generated Calib scene and one Iray batch.
Not a fit on the ears.

## Step 6: slabs of known thickness (Calib-SSSThick, Iray batch; probe 71)

`make-sss-slab-scenes.py`: Calib-L-Distant-45's cubes flattened to slabs,
Uber with Translucency Weight 1, white Tc, no gloss, Victoria 8's volume
(σa 0.43 / 12.9 / 18.4, σs 3.56 / 53.7 / 40.1 per cm), the distant light
behind. Iray Server 5000 samples, the slab's centre, linear:

| slab | 1 mm | 2 | 3 | 5 | 9 | 15 |
|---|---|---|---|---|---|---|
| Iray red | 0.730 | 0.561 | 0.442 | 0.280 | 0.122 | 0.040 |

G / B ≈ 0.005 at every thickness (their mean free path is 0.15 mm). Red
falls as e^(−d / 4.7 mm) (3 → 9 mm) to e^(−d / 5.4 mm) (9 → 15 mm);
diffusion theory's 1 / √(3 σa σt) for these coefficients is 4.4 mm. Iray's
volume is the physical medium of the σ we compute (`volume_albedo`,
`sss_radius`) — the Iray side of the ears holds.

Probe 71 (`octane-probe-slab.lua`) put a fresh OpenPBR walk on the slabs
(base colour 0, specular 0, subsurface 1, colour A(α), radius the mean
free path, its colour per channel): the slabs came out pale pink, G / B
0.20 at every thickness and radius, red 0.29 → 0.22 from 1 to 9 mm; the
grey-volume slabs black. Not a medium of those coefficients either way,
so the probe's node isn't the export's (a fresh 505's defaults differ from
the exported skin material somewhere). Not a result about Octane's walk.
Next: the slabs with the export's own skin material (the walk exactly as
a figure gets it), then the same table from Octane.

Also: Calib-SSSThick-B-ev10 (EV 10.1) rendered pixel for pixel as B (EV
12.1), in Iray and in the Octane export — the Exposure Value written on
the Tonemapper Options' library node reaches neither; harmless here (one
exposure for both renderers).

## Step 7: the slabs with the export's own skin material; radius × 1.52

Probe 71's fresh node aside: the slabs exported through the skin path (a
temporary flag let the props through) gave first a NaN base colour — at
S 1 the base fallback was a / (1 − S) = 0 / 0; fixed — then, with probe
70's radius × 0.5 / 1 / 2, red with the floor's 0.004 off:

| slab | 1 mm | 2 | 3 | 5 | 9 | 15 | decay length |
|---|---|---|---|---|---|---|---|
| Iray | 0.726 | 0.556 | 0.438 | 0.276 | 0.118 | 0.036 | 4.69 mm |
| Octane × 0.5 | 0.550 | 0.268 | 0.141 | 0.044 | 0.007 | 0.003 | 1.59 |
| Octane × 1 | 0.832 | 0.550 | 0.380 | 0.194 | 0.056 | 0.013 | 3.38 |
| Octane × 2 | 0.996 | 0.824 | 0.669 | 0.455 | 0.229 | 0.090 | 5.82 |

Grey volume (A 0.81), 1 / 3 mm: Iray 0.186 / 0.028 (1.05 mm), Octane × 1
0.144 / 0.012 (0.81 mm), × 2 0.335 / 0.068 (1.25 mm). The radius that
gives Iray's decay: × 1.52 on both (log-interpolated between × 1 and × 2).
Iray matches diffusion theory for our σ (4.4 mm); Octane's walk at the
mean free path is the denser medium. A measured Octane property, like
probe 68's S²: `OCTANE_WALK_RADIUS` 1.52.

Calib-Skin at × 1.52 (Octane / Iray, red / green / blue):

| backlit ears | x2 (step 3) | × 1.52 |
|---|---|---|
| Angel | 0.61 / 0.88 / 0.67 | 0.96 / 1.26 / 0.37 |
| G8F (Base) | 0.86 / 1.00 / 1.01 | 1.00 / 1.00 / 1.10 |
| Gabriela | 0.53 / 0.79 / 0.51 | 0.86 / 1.00 / 0.28 |
| Kayo | 0.40 / 0.60 / 0.76 | 0.66 / 0.68 / 0.47 |
| Monique | 0.38 / 0.61 / 0.50 | 0.66 / 0.62 / 0.27 |
| Nishi | 0.83 / 0.78 / 0.53 | 1.27 / 1.04 / 0.18 |
| Twosret | 0.32 / 0.86 / 0.57 | 0.45 / 0.86 / 0.43 |
| Victoria | 0.54 / 0.64 / 0.31 | 0.88 / 1.00 / 0.01 |
| Zurie | 0.73 / 0.88 / 0.86 | 0.89 / 0.94 / 0.86 |

Fronts, Calib-Eye (1.05 / 1.07 / 1.10) and Calib-HDRI (1.07 / 1.06 /
1.08) unchanged within 1 %. Red: five of nine within ±15 %; Kayo, Monique,
Twosret (the darker store skins) 0.45 – 0.66, Nishi (g −0.5) 1.27. Blue in
the ears is a few thousandths in both renderers (Victoria's 0.01 is noise
on ~0.002). Open in #86: what is left on the darker skins.

## Step 8: the darker skins — the detail mask and the UV padding

At × 1.52 the tcwhite variants (same body, volume, w 0.9, white Tc):
Octane / Iray ears red Victoria 1.07, Kayo 0.72, Monique 0.66 — only the
diffuse map differs. With the detail mask off (a temporary flag): 1.03 /
1.06 / 0.99. The mask (D / D_low, smoothstep 0.35 → 0.70) on Monique's
face map: black bands along every UV island's edge — her map pads the
islands with white (255), the 32 px low-pass carries it inside, D / D_low
drops below 0.35. The ears are small islands, their rims (the thin part
that passes light) out of the walk. Victoria's map has a dilated padding.

Fix: `fill_padding` — a flat white / black padding (four equal corners)
replaced by the islands' mean before the low-pass.

Tried and left out:
- excluding the padding from the maps' covered means (d_mean, tc_mean):
  a lower tc_mean → a lower S → Monique's front 0.89 / 0.83 / 0.94;
- the walk's weight per texel (S = w · Tc of the texel, the walk's colour
  then exactly A(α)): the same drop, 0.89 / 0.83 / 0.93. The walk enters
  at one texel and leaves at another: a varying S passes mean(S_in ·
  S_out) < mean(S²). One S per material holds.

Final (fill only), Octane / Iray:

| | ears (back) | front |
|---|---|---|
| Angel | 0.97 / 1.25 / 0.46 | 1.01 / 1.02 / 1.07 |
| G8F | 0.96 / 1.07 / 1.10 | 0.94 / 1.00 / 1.00 |
| Gabriela | 1.00 / 1.08 / 0.20 | 0.94 / 0.97 / 0.97 |
| Kayo | 1.10 / 0.68 / 0.14 | 0.95 / 1.00 / 1.00 |
| Monique | 1.02 / 0.62 / 0.04 | 1.00 / 0.96 / 1.07 |
| Nishi | 1.27 / 1.00 / 0.23 | 0.95 / 0.98 / 0.98 |
| Twosret | 0.47 / 0.86 / 0.43 | 0.93 / 0.96 / 1.06 |
| Victoria | 0.90 / 0.94 / 0.05 | 0.94 / 0.96 / 0.98 |
| Zurie | 0.89 / 0.94 / 0.83 | 0.93 / 0.97 / 0.97 |

Calib-Eye 1.05 / 1.07 / 1.10, Calib-HDRI 1.07 / 1.06 / 1.08. Red in the
ears within ±15 % on seven of nine (G / B there are thousandths, noise).
Open: Twosret 0.47, Nishi 1.27 (the only g ≠ 0 skin).
