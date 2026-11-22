# 2026-11-22 — A partial refraction on Iray's fibres (#222)

Found in #220's Iray compare: on untessellated fibres every
Calib-Strand-GlossLobeArc-HDRI column with Pixie Hair01's partial
refraction (0.284) rendered 1.35 - 1.96 x Iray in Faithful + Final, where
the same arcs without refraction (Calib-Strand-LobeArc-HDRI, #216) sit at
1.07 - 1.24 x.

## Docs first

- Iray programmer's manual, Hair Materials and Fiber Geometry: without a
  hair BSDF Iray shades fibres with the regular BSDF, single sided and
  opaque - the transmission is discarded. Nothing there treats a
  refraction differently from a translucency.
- `docs/daz/iray-uber-mdl.md`: Uber's refraction is a `weighted_layer` at
  Refraction Weight over everything, one glossy BSDF in reflect + transmit
  mode at the IOR, its reflection the BSDF's own Fresnel (× Glossy Layered
  Weight when `full_refraction_reflection_enable` is off). So on a fibre
  the refraction's share keeps its Fresnel reflection and loses the rest;
  with `refraction_reflection_enable` off (no gloss) it is black.

The export already dropped the translucency on fibres in Faithful + Final
(#79, `lost_on_fibres`), not the refraction: the Mix's refraction part
(albedo 0, specular 0.3, transmission 1, Thin Wall type, fake shadows) and
the one-node partial refraction (transmission 0.284) still passed light,
and fake shadows let shadow rays through the strands.

## The rule (no constant)

`Tuning::opaque_fibres` (Faithful + Final only, as the translucency was):
on `Material::fibres` a refraction writes no transmission, no
transmission type, medium, dispersion or fake shadows; its specular and
IOR stay (the share's reflection). The translucency rule moves from
`strand_hair == MdlLayers` onto the same field (same variants, no change).
Enhanced keeps the light a strand passes on (Look: real hair passes it,
back-lit hair glows); docs/tuning.md row. Ribbons are untouched (Iray's
ribbons pass transmission: -T2 Octane / Iray 0.65 - 1.08).

No new Iray variant was needed: the manual and the MDL say what the share
holds, and the existing references (b3177d0) decide it - the render below
lands every column on the no-refraction level. (A refraction-1 strand with
and without its reflection, as the issue suggested, would split the
share's reflection from its loss; not needed once the whole column lands
on -LobeArc's level, kept as an option if a fibre refraction drifts
again.)

## Octane (Faithful + Final, 400 samples, denoised; cell sums as #220's note)

Calib-Strand-GlossLobeArc-HDRI, Octane / Iray, rows G / H / I / J:

| column | before | after |
|---|---|---|
| 1 Pixie + gloss (Mix) | 1.87 / 1.85 / 1.44 / 1.42 | 1.15 / 1.17 / 1.17 / 1.17 |
| 2 no lobe (Mix) | 1.64 / 1.64 / 1.35 / 1.41 | 1.10 / 1.13 / 1.11 / 1.15 |
| 3 no translucency (Mix) | 1.39 / 1.48 / 1.41 / 1.39 | 1.18 / 1.20 / 1.18 / 1.16 |
| 4 no transl., no lobe (one node) | 1.96 / 1.74 / 1.36 / 1.37 | 1.11 / 1.13 / 1.12 / 1.14 |

Inside one renderer: 3 / 4 (Mix + lobe against one node) Octane now
1.27 / 1.43 / 1.41 / 1.24, Iray 1.20 / 1.34 / 1.34 / 1.22 (before 0.85 on
G: #220's "split question" was column 4's transmission). Lobe share 1 / 2
1.73 / 1.62 / 1.66 / 1.39, Iray 1.66 / 1.56 / 1.58 / 1.36.

Control, Calib-Strand-LobeArc-HDRI (no refraction): unchanged to 0.01
(1.12 - 1.14 / 1.07 - 1.11 on its opaque columns; columns 3 / 4, the lobe
over black and its glossy twin, 1.34 - 1.44 / 1.18 - 1.30 as before, #216).
The remaining 1.10 - 1.20 is the same level as those opaque arcs: the
fibre base itself, not the refraction.

Calib-Hair-Pixie-T1 (Pixie's Hair01 on its real pile, fibres;
`hair-set-compare.py`, luminance, crown / fringe / side):

| Faithful / Iray | before | after |
|---|---|---|
| base | 0.87 / 0.75 / 1.04 | 0.85 / 0.74 / 1.03 |
| -nogloss (no dual lobe) | 0.75 / 0.70 / 0.77 | 0.73 / 0.69 / 0.75 |
| -coat | 1.21 / 0.93 / 2.11 | 1.20 / 0.93 / 2.08 |
| lobe share (Iray 1.23 / 1.26 / 1.22) | 1.42 / 1.36 / 1.66 | 1.42 / 1.36 / 1.67 |

On the dense pile the refraction's transmission is worth 1 - 2 %: lit from
outside, a pile passes little through. So #222 does not explain #221 (the
side patch's lobe share 1.67 against Iray's 1.22) nor the pile's dark
base (-nogloss 0.69 - 0.75 x); both stay with the pile (#221, #161).
Morgan Hair as fibres (#161: Octane 0.58 - 0.74 x Iray, Strands
refraction 0.106 in Base Mixing 1) can only get a little darker from this
rule, so its gap is not a refraction either; not rendered here.

Timings: the five renders (two arc scenes, three Pixie-T1) 16 min.

## Process

- An Iray rule about *a* BSDF component ("transmission is discarded")
  covers every channel that feeds it; #79 fixed it for the translucency
  only. When applying a documented Iray rule, grep the export for every
  place that writes that pin.
