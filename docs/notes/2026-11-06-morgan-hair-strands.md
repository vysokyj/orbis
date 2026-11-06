# 2026-11-06 — Morgan Hair as strands (#52)

Picks up [2026-10-29](2026-10-29-spec-mode-hair.md). Since then the export
changed a lot: GGX energy preserving (#56), Color Effect (#27), the
specular-mode roughness cap, Fujii Oren-Nayar (#76), the layered anisotropic
Top Coat (#23), Diffuse Overlay (#71), per-map image modifications (#77).
So: re-export the five Calib-Hair-Teens scenes (base, `-nocoat`, `-fx0`,
`-nofilm`, `-nogloss`; the last two needed fresh dumps), render them in
Octane (enhanced-final, 256 samples, full 1280 × 720 — the blonde-only
copy) and compare with the 5000-sample Iray references in the cache.

The scenes and everything derived from them stay local (store hair):
numbers only here.

**Patches** (1280 × 720, Octane / Iray, linear means over pixels that are
hair in both, R / G / B): crown (560 – 720, 70 – 130), hilite (the bright
streak on the left, 490 – 560, 190 – 280), middle (620 – 720, 160 – 260),
lengths (480 – 560, 300 – 370), tips (the purple ends, 580 – 680, 400 –
480). Not the earlier session's patches, so the numbers don't line up with
its tables.

## Not cards: strands

Morgan Hair's geometry is a polyline mesh: 6401 library lines (no facets),
which DAZ renders as 68 773 strands (the dump's render strands — DAZ
computes the count from the guides, the export doesn't widen strands to make
up for it). The export writes them as `BasisCurves` with the Uber material
on them. "Cards seen edge-on" was a wrong picture.

How DAZ hands strands to Iray, from the .mi of Calib-Hair (a dForce hair,
`mi-fig`): not as Iray fibres but as a `trilist` — each strand a thin
tube, `vertex p n t 2 2`. The first UV set steps u by 1/6 between a strand
point's vertices and grows v along the strand: **u runs around the
strand**, so Iray's tangent (the UV's u) lies across it. (MDL's own fibre
shading, where `texture_tangent_u` is zero and the normal is undefined, is
not what Iray sees here.)

## As exported now

| variant | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| base | 2.98 / 3.80 / 3.86 | 0.78 / 0.69 / 0.74 | 1.46 / 1.75 / 1.90 | 1.61 / 1.93 / 2.07 | 1.22 / 1.83 / 1.55 |
| nocoat | 2.31 / 2.72 / 3.59 | 0.79 / 0.73 / 0.73 | 1.28 / 1.43 / 1.79 | 1.42 / 1.58 / 1.99 | 1.14 / 1.57 / 1.54 |
| fx0 | 2.91 / 3.08 / 2.53 | 0.76 / 0.65 / 0.61 | 1.43 / 1.52 / 1.42 | 1.37 / 1.45 / 1.28 | 1.35 / 1.63 / 1.17 |
| nofilm | 2.47 / 2.88 / 3.74 | 0.79 / 0.73 / 0.73 | 1.30 / 1.43 / 1.78 | 1.44 / 1.59 / 1.98 | 1.17 / 1.60 / 1.57 |
| nogloss | 2.34 / 2.78 / 3.12 | 1.54 / 1.83 / 2.14 | 2.16 / 2.63 / 3.02 | 1.70 / 1.99 / 2.33 | 1.54 / 1.87 / 2.10 |

The lighting is right: skin on the shoulder 0.84 / 0.89 / 1.09, the face
0.93 / 0.97 / 1.09. Octane's hair is flat, uniformly bright and grainy;
Iray's has highlight streaks along strand clumps and a darker, warmer
brown between them.

Each renderer against its own base (variant / base):

| | crown | hilite | middle | tips |
|---|---|---|---|---|
| Iray nocoat | 1.50 / 1.97 / 1.51 | 1.09 / 1.28 / 1.31 | 1.27 / 1.66 / 1.37 | 1.23 / 1.57 / 1.38 |
| Octane nocoat | 1.16 / 1.41 / 1.40 | 1.11 / 1.34 / 1.28 | 1.11 / 1.35 / 1.29 | 1.15 / 1.35 / 1.38 |
| Iray fx0 | 1.03 / 1.25 / 1.56 | 1.02 / 1.08 / 1.24 | 1.02 / 1.16 / 1.36 | 0.92 / 1.15 / 1.36 |
| Octane fx0 | 1.01 / 1.02 / 1.02 | 1.00 / 1.01 / 1.02 | 1.00 / 1.01 / 1.02 | 1.02 / 1.02 / 1.02 |
| Iray nofilm | 1.39 / 1.86 / 1.46 | 1.08 / 1.27 / 1.30 | 1.24 / 1.64 / 1.38 | 1.20 / 1.56 / 1.36 |
| Octane nofilm | 1.15 / 1.41 / 1.41 | 1.10 / 1.34 / 1.28 | 1.10 / 1.34 / 1.29 | 1.15 / 1.37 / 1.38 |
| Iray nogloss | 1.36 / 1.46 / 1.56 | 0.55 / 0.42 / 0.45 | 0.75 / 0.74 / 0.82 | 1.08 / 1.21 / 1.13 |
| Octane nogloss | 1.06 / 1.07 / 1.26 | 1.10 / 1.10 / 1.29 | 1.11 / 1.11 / 1.31 | 1.36 / 1.23 / 1.54 |

- Iray's gloss is half of its highlight streak and darkens the crown (its
  Fresnel takes the diffuse); Octane's added 10 % anywhere.
- Iray's film darkens 1.4 / 1.9 / 1.5 on the crown, Octane's 1.15 / 1.4 /
  1.4: close in green and blue, short in red.
- Iray's Color Effects (all 2) darken green and blue by up to 1.25 / 1.56;
  Octane's only rescale the gloss and coat weights (× 1.12, grey).

## Experiments (edited `project.ocs`, base scene)

| try | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| gloss isotropic at the short axis (roughness 0.21) | 2.25 / 2.89 / 3.07 | 0.94 / 0.94 / 1.05 | 1.57 / 2.05 / 2.31 | 1.33 / 1.60 / 1.76 | 1.03 / 1.49 / 1.37 |
| rotation 0.25 (= 0.75: the same axis) | 2.98 / 3.81 / 3.87 | 0.77 / 0.69 / 0.74 | 1.46 / 1.75 / 1.90 | 1.60 / 1.93 / 2.07 | 1.22 / 1.84 / 1.55 |
| **rotation 0** (gloss and coat) | 2.24 / 2.86 / 3.04 | 1.13 / 1.07 / 1.10 | 1.83 / 2.34 / 2.46 | 1.48 / 1.78 / 1.92 | 1.03 / 1.49 / 1.37 |
| coat isotropic too | 2.24 / 2.87 / 3.04 | 0.95 / 0.94 / 1.05 | 1.58 / 2.06 / 2.33 | 1.33 / 1.60 / 1.76 | 1.03 / 1.49 / 1.37 |
| albedo × 0.5 | 1.85 / 2.43 / 2.46 | 0.48 / 0.44 / 0.47 | 0.89 / 1.09 / 1.18 | 0.98 / 1.20 / 1.28 | 0.73 / 1.17 / 0.91 |
| specular 0 | 2.80 / 3.33 / 3.25 | 0.76 / 0.63 / 0.64 | 1.42 / 1.58 / 1.63 | 1.59 / 1.78 / 1.81 | 1.16 / 1.68 / 1.43 |
| transmission 0 | 2.88 / 3.70 / 3.79 | 0.75 / 0.67 / 0.72 | 1.41 / 1.70 / 1.84 | 1.55 / 1.87 / 2.02 | 1.20 / 1.80 / 1.53 |
| coat specular 0 | 2.98 / 3.79 / 3.83 | 0.78 / 0.70 / 0.74 | 1.47 / 1.76 / 1.89 | 1.62 / 1.93 / 2.06 | 1.21 / 1.79 / 1.53 |
| strands 1× DAZ width (export at `LINE_WIDTH_SCALE` 0.001) | 2.78 / 3.55 / 3.61 | 0.75 / 0.67 / 0.72 | 1.38 / 1.65 / 1.81 | 1.53 / 1.83 / 1.96 | 1.16 / 1.74 / 1.47 |
| 1× width, rotation 0, no transmission | 2.01 / 2.59 / 2.76 | 1.08 / 1.00 / 1.03 | 1.74 / 2.16 / 2.30 | 1.36 / 1.65 / 1.79 | 0.96 / 1.39 / 1.28 |
| rotation 0, `fake_shadows` off | 2.20 / 2.82 / 3.00 | 1.12 / 1.06 / 1.09 | 1.80 / 2.30 / 2.43 | 1.45 / 1.76 / 1.90 | 1.02 / 1.47 / 1.36 |
| rotation 0, specular 0 (IOR 5.63 or 1) | 2.79 / 3.32 / 3.23 | 0.76 / 0.63 / 0.64 | 1.43 / 1.59 / 1.65 | 1.60 / 1.79 / 1.82 | 1.16 / 1.67 / 1.42 |
| rotation 0, IOR 1 (specular kept) | 2.79 / 3.31 / 3.22 | 0.76 / 0.63 / 0.64 | 1.43 / 1.59 / 1.64 | 1.60 / 1.78 / 1.82 | 1.15 / 1.66 / 1.42 |

Rotation 0.25 and 0.75 are the same axis (an anisotropic lobe repeats
every half turn), so the first rotation try changed nothing. At 0 Octane
streaks along the strands as Iray does — the tangent's quarter turn above
— and the crown loses 25 %: the sharper lobe across the view takes more
of the diffuse. Rotation 0's crown equals the nogloss ratio (2.24 / 2.86 /
3.04 against 2.34 / 2.78 / 3.12): the gloss now does to the crown what
Iray's does.

Ruled out as the remaining excess: strand width (1× DAZ's: −7 %),
Octane's `fake_shadows` (the 0.106 refraction sets it; −2 %), the
transmission (−3 %), the coat's own reflection (0). Octane's IOR with
specular 0 leaves the diffuse alone, and specular with IOR 1 reflects
nothing: Universal's diffuse is taken only by specular × Fresnel.

**An MDL hypothesis that failed.** MDL's fibre shading has no normal
(`state::normal()` is zero there) and the Uber hands that normal to its
layers, so every Fresnel curve and the film would be at grazing. As
Octane edits — the film at its grazing tint, the albedo × (1 − 0.76)(1 −
0.2) for the gloss and coat at grazing reflectivity 1, rotation 0, no
transmission — the hair went to 0.24 – 1.03 × Iray (crown 0.47 / 0.69 /
0.77, tips 0.24 / 0.41 / 0.33); with Universal's IOR at 100 the middle
1.37 / 1.91 / 1.56. The .mi then showed DAZ tessellates strands, so Iray
has tube normals: not fibres.

## The fix

Iray Uber on strand hair: Octane's rotation 1 − (rotation + 0.25), gloss
and coat (spec, Anisotropy). The export, all variants:

| variant | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| base | 2.24 / 2.86 / 3.04 | 1.13 / 1.07 / 1.10 | 1.83 / 2.34 / 2.46 | 1.48 / 1.78 / 1.92 | 1.04 / 1.49 / 1.37 |
| nocoat | 1.73 / 2.02 / 2.81 | 1.11 / 1.11 / 1.10 | 1.56 / 1.84 / 2.35 | 1.31 / 1.45 / 1.86 | 0.96 / 1.26 / 1.37 |
| fx0 | 2.11 / 2.24 / 1.94 | 1.13 / 1.04 / 0.93 | 1.81 / 2.09 / 1.89 | 1.25 / 1.33 / 1.18 | 1.10 / 1.28 / 1.00 |
| nofilm | 1.85 / 2.14 / 2.93 | 1.11 / 1.11 / 1.10 | 1.58 / 1.85 / 2.32 | 1.33 / 1.47 / 1.86 | 0.99 / 1.29 / 1.39 |
| nogloss | 2.33 / 2.76 / 3.11 | 1.55 / 1.84 / 2.15 | 2.17 / 2.64 / 3.03 | 1.71 / 2.00 / 2.34 | 1.53 / 1.85 / 2.09 |

The highlight streaks are Iray's now. What's left is the base under the
gloss: without any gloss Octane's hair is 1.5 – 3.1 × Iray's (nogloss),
bluest on the crown, and Iray's Color Effects and film take more green and
blue than the export does (fx0 brings Octane closest to Iray in hue). My
reading of `transmit_color_calc` (the effect's tint at full intensity,
its weight the colour's mono) doesn't give a 1.56 blue shift on the
crown, so that is next, not a fitted factor: the effects on a diffuse with
translucency (Base Color Effect 2 at Translucency 0.059 — the export
drops that translucency when the material refracts, here Refraction
Weight 0.106) and on the coat.

## Process

- Octane X, open since two days before, failed the first job of a batch
  with "render engine failure" (#50) and hung; killed and restarted, the
  batch rendered. Each 4-scene batch ≈ 8 min at 256 samples.
- `-nofilm` and `-nogloss` had no dump: `daz-dump.sh` (≈ 50 s each).

## Color Effect and the dropped translucency (second step)

Morgan's Strands material (scene `material_library`): Base Mixing 1,
diffuse 0.855 / 0.812 / 0.729, Translucency Weight 0.059 (Tc 0.80 / 0.76
/ 0.69, Invert Transmission Normal), Base, Glossy, Top Coat and Diffuse
Overlay Color Effect all 2, Refraction Weight 0.106 (IOR 1.79), Thin
Walled. The export dropped the translucency (any refraction > 0 did) and
used fitted Base Color Effect shares.

**DAZ's MDL** (`diffuse_base`, `transmit_color_calc`): with translucency
mixed in, the diffuse is a `weighted_layer` at mono over a
`diffuse_transmission_bsdf` in Tc. Mono = (1 − w) × the colour's average
(effect 1) or, effect 2, the colour greyed by `color_layer_color` first;
the tint is the colour's hue at full intensity. Reading "intensity" as
the strongest channel, the rule reproduces both fitted pairs:

| case | fitted diffuse / transmission | MDL rule |
|---|---|---|
| Calib-Thin green cards, effect 1 (D 0.05 / 0.26 / 0.01, w 0.5) | 0.40 – 0.41 / 2 × | 0.41 / 1.89 × |
| Calib-ColorEffect C3, effect 2 (D 0.07 / 0.22 / 0.61, w 0.5) | 0.95 / 1.55 × | 1.00 / 1.39 × |
| the same, intensity read as the average | — | 0.49 / 1.70 × |

So the shares went, the rule came in (`base_color_effect_shares`), and
effect 2 on Glossy / Top Coat weighs by the strongest channel too. And a
refraction below 0.5 keeps the translucency at 1 − r (the refraction is
a weighted layer over everything); the translucency then holds
Universal's one transmission. Morgan: albedo × 0.941, transmission 0 →
0.30 × Tc.

Check scenes (Octane enhanced-final 256 samples vs the stored Iray refs;
"before" rendered from HEAD's binary in the same session, R G B):

| sphere | before | after |
|---|---|---|
| ColorEffect C1 (effect 0) | 0.87 1.00 1.01 | 0.87 1.00 1.01 |
| ColorEffect C2 (effect 1) | 1.23 1.11 0.82 | 0.90 0.99 0.98 |
| ColorEffect C3 (effect 2) | 1.01 1.00 0.95 | 0.83 0.97 1.00 |
| Map-Effect E1 (effect 1, red, blue Tc) | 0.81 1.12 1.19 | 0.74 1.04 1.07 |
| Map-Effect E2 (effect 2) | 0.95 0.96 0.93 | 0.97 1.04 1.06 |
| Map-Effect E4 (red gloss, effect 2) | 1.09 0.73 2.69 | 1.12 0.76 2.65 |
| CoatEffect E3 / E5 (coat effect 2) | — | 1.08 1.12 1.12 / 1.14 1.14 1.14 (effect 0: 1.05 – 1.16) |
| Calib-Thin front/back A -bce (thirds of the frame) | 0.99 – 1.00 | 0.99 – 1.00 |

Map-Effect E5 (Diffuse Strength, material identical in both exports)
moved 1.08 0.82 1.46 → 1.13 1.09 1.90 in its dim green and blue: noise
or E4's changed reflection next to it, not this rule. C3's red follows
C1's (Octane's red on this blue sphere is low with any effect).

Morgan (patches as above, Octane / Iray):

| variant | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| base | 2.08 / 2.72 / 2.96 | 1.17 / 1.19 / 1.22 | 1.87 / 2.52 / 2.67 | 1.44 / 1.78 / 1.93 | 1.05 / 1.51 / 1.37 |
| fx0 | 1.88 / 2.04 / 1.80 | 1.13 / 1.07 / 0.96 | 1.78 / 2.11 / 1.92 | 1.17 / 1.27 / 1.13 | 1.03 / 1.20 / 0.94 |
| nogloss | 2.21 / 2.63 / 2.97 | 1.47 / 1.75 / 2.05 | 2.05 / 2.51 / 2.88 | 1.61 / 1.90 / 2.22 | 1.54 / 1.87 / 2.05 |

The crown dropped 3 – 7 %; the gap stays. Why it can't close it: under
the strongest-channel reading, effect 2 leaves every reflection as effect
0 has it (tint × weight = colour × weight for gloss, coat and the
diffuse); it only moves 0.059 → 0.34 of the base into a warm
translucency, which adds light. Yet Iray's fx0 is *brighter* in green and
blue (1.25 / 1.56 on the crown) than its base. Octane's fx0 / base is now
0.93 / 0.94 / 0.95: no hue shift. So whatever Iray does with Color
Effect on this material is not in `transmit_color_calc` as read here —
and the 1.5 – 3 × of the nogloss variant isn't a Color Effect matter at
all.

Also ruled out: the Cutout Opacity map (Morgan Hair TR2) — a gradient
along v only (0.12 at one end, 1 from mid-length; mean 0.85), the same
across u, so Iray's u-around-the-tube sampling sees what the export does.

Next (#52): find what makes Iray's strands darker than any one channel
explains — e.g. an Iray render of Morgan's material on a tessellated
strand patch vs a flat card, or the .mi of the Calib-Hair-Teens hair
material compared channel by channel with `src/iray.rs`.

## Channel isolation on the strands (third step)

Four Iray Server renders (400 samples) of Calib-Hair-Teens without gloss,
each with one more channel off on the three Morgan materials, against
Octane enhanced-final at 256 samples. Ratios R / G / B.

Octane / Iray per patch:

| variant | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| nogloss | 2.21 / 2.63 / 2.97 | 1.47 / 1.75 / 2.05 | 2.05 / 2.51 / 2.88 | 1.61 / 1.90 / 2.22 | 1.54 / 1.87 / 2.05 |
| − translucency | 1.36 / 1.42 / 1.34 | 0.84 / 0.86 / 0.83 | 1.25 / 1.33 / 1.26 | 0.93 / 0.95 / 0.92 | 0.89 / 0.98 / 0.84 |
| − refraction | 2.33 / 2.74 / 3.05 | 1.55 / 1.79 / 2.11 | 2.15 / 2.60 / 2.96 | 1.70 / 1.96 / 2.28 | 1.55 / 1.89 / 2.08 |
| − top coat | 2.07 / 2.38 / 3.10 | 1.42 / 1.60 / 2.09 | 1.92 / 2.27 / 2.88 | 1.58 / 1.77 / 2.29 | 1.56 / 1.83 / 2.23 |
| bare (diffuse only) | 1.44 / 1.47 / 1.50 | 0.95 / 0.88 / 0.87 | 1.28 / 1.34 / 1.30 | 0.99 / 0.98 / 0.98 | 0.96 / 1.05 / 0.95 |

Each renderer against its own nogloss render (crown):

| variant | Iray | Octane |
|---|---|---|
| − translucency | 1.71 / 1.94 / 2.32 | 1.05 / 1.05 / 1.05 |
| − refraction | 1.08 / 1.09 / 1.10 | 1.14 / 1.13 / 1.12 |
| − top coat | 1.33 / 1.72 / 1.45 | 1.25 / 1.55 / 1.51 |
| bare | 2.23 / 3.21 / 3.41 | 1.45 / 1.79 / 1.72 |

So:
- The strand geometry is fine: the bare diffuse strands match within a few
  per cent on the lengths, tips and highlight patch (the crown stays
  1.4 ×, also without any other channel).
- Refraction and top coat act alike in both renderers.
- **The translucency is the gap.** A Translucency Weight of 0.059 darkens
  Iray's strands 1.7 / 1.9 / 2.3 × — very nearly 1 / linear Translucency
  Color (1.66 / 1.85 / 2.30) — while in Octane it moves 5 %. By the MDL
  (`diffuse_base`) the diffuse keeps 0.94 of its weight, so this is not
  in the layer weights; flat cards (Calib-Thin) and spheres
  (Calib-ColorEffect C3) with translucency match, so it is specific to
  the strands.

Second batch, to tell a tint from a switch (all without gloss):

| variant | Octane / Iray, lengths | Iray / its nogloss, crown | Octane / its nogloss, crown |
|---|---|---|---|
| Translucency Color white | 1.14 / 1.53 / 1.71 | 1.70 / 1.43 / 1.62 | 1.05 / 1.05 / 1.07 |
| Translucency Weight 0.001 | 1.62 / 1.90 / 2.24 | 1.05 / 1.04 / 1.04 | 1.06 / 1.06 / 1.06 |
| Translucency Weight 0.5 | 1.56 / 1.85 / 2.14 | 0.64 / 0.64 / 0.66 | 0.58 / 0.58 / 0.58 |
| Base Color Effect 0 | 0.87 / 0.91 / 0.89 | 1.64 / 1.85 / 2.21 | 0.95 / 0.96 / 0.97 |

- It is a **switch, not an amount**: a Translucency Weight of 0.001 keeps
  Iray as dark as 0.059, and 0.5 scales both renderers alike (0.64 vs
  0.58) with the gap untouched.
- The switch is **Base Color Effect 2 with translucency on**: Effect 0
  brightens Iray by the same 1.6 – 2.2 × as turning the translucency off,
  and then Octane matches (0.87 – 0.91 on the lengths, the crown's 1.3 ×
  as in the bare variant).
- In Iray, effect 2 thus leaves the strands' diffuse albedo about
  0.61 / 0.54 / 0.45 × what effect 0 gives — close to the diffuse colour
  itself again (linear 0.70 / 0.63 / 0.49 × 0.87), as if the colour were
  applied twice. `transmit_color_calc` as read (tint = colour at full
  intensity, weight × its strongest channel) gives about 1.0. A white
  Translucency Color brightens Iray too, but not by 1 / Tc (1.70 / 1.43
  / 1.62), so the translucency colour is not the factor either.
- The same switch on surfaces matches: Calib-ColorEffect C3 and
  Calib-Map-Effect E2 (thin-walled spheres, effect 2, translucency 0.5)
  are within the noise with the strongest-channel rule. So the doubling
  is specific to strands — or to something in Morgan's material the
  spheres lack (Diffuse Overlay 0.05², effect 2).

Next (#52): a generated scene from free content only — dForce Starter
Essentials strands and a thin-walled plane with Morgan's Uber values,
effect 0 / 2 × translucency 0 / 0.06, overlay on / off — with its `.mi`,
to see what DAZ hands Iray for strand hair, and an Iray batch of it.

## The cause: SSS Reflectance Tint in the diffuse (fourth step)

The `.mi` of the variants (headless DAZ, kept local: store content) shows
it at once: with Base Color Effect 2 DAZ hands Iray Morgan's
`diffuse_color` as 0.306 / 0.259 / 0.172 instead of 0.708 / 0.632 / 0.500
— × 0.431 / 0.410 / 0.344, which is the SSS Reflectance Tint (0.682 /
0.667 / 0.616) to the power 2.2, to the last digit (Under and Mid alike).
Effect 0: the plain colour. So the effect only gates a tint the MDL never
sees, and `transmit_color_calc` was read right after all.

Spheres with the same values didn't get it — not with Morgan's whole
setting either (refraction, coat, overlay, glossy effect, thin film,
abbe, cutout map: Calib-Mi-SSSTint2, discarded). Morgan's whole channel
list copied onto a sphere did, and dropping groups of it (tags, bump,
glossy / coat inputs, line widths) changed nothing. The difference left
was the channel definitions: DAZ's default material (our base scene)
saves Base Color Effect and SSS Reflectance Tint with `"visible": false`,
Morgan's shows them. Making them visible on the spheres: **the tint comes
in exactly when the Base Color Effect channel is shown** (its own
visibility doesn't matter), with translucency, for effect 1 and 2, in
either mixing mode, thin-walled or closed, and into a diffuse map's
`color_scale`. Calib-Mi-SSSTint keeps that as a fixture (S5 the hidden
control). Also from it: glossiness mode without gloss leaves
`glossy_reflectivity` at the default 0.04.

Octane / Iray after the fix (crown / hilite / middle / lengths / tips):

| variant | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| base | 1.22 / 1.53 / 1.48 | 1.02 / 0.96 / 0.92 | 1.55 / 1.98 / 1.92 | 0.97 / 1.14 / 1.11 | 0.64 / 0.94 / 0.64 |
| nogloss | 1.09 / 1.24 / 1.20 | 0.71 / 0.81 / 0.80 | 1.00 / 1.17 / 1.13 | 0.78 / 0.88 / 0.87 | 0.79 / 0.98 / 0.81 |
| (before, base) | 2.08 / 2.72 / 2.96 | 1.17 / 1.19 / 1.22 | 1.87 / 2.52 / 2.67 | 1.44 / 1.78 / 1.93 | 1.05 / 1.51 / 1.37 |

What is left: with gloss the crown and middle stay 1.2 – 2 × (green and
blue most) while the lengths match; the all-effects-0 variant (fx0, gloss
on) is unchanged at 1.9 / 2.0 / 1.8 on the crown against 1.3 without
gloss — so the glossy / top coat layers on strands give Octane too much
on the lit crown. Nogloss now runs 0.8 – 0.9 on the lengths.

Process lesson: `make-calib-scenes.py --force` rewrites *every* generated
scene in My Library; they were restored from `calib/scenes`. Delete the
one scene and rerun without `--force` instead.
