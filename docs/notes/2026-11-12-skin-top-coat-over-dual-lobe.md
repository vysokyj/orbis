# 2026-11-12 Skin Top Coat over gloss over dual lobe (#138)

Goal: the OpenPBR skin with a Top Coat kept only the gloss of Iray's three
specular layers (Top Coat, glossy layer, dual lobe) - OpenPBR has
`specular` and one `coat`, and `skin_gloss` fell back to the gloss alone
on `specular` with the coat on the coating. Variant enhanced-final, 400
samples, Octane only: the Iray references (400 samples) came with #106.

## Choice
Two options from the issue:
- the dual lobe and gloss blended on `specular`: one lobe for two
  roughnesses and two Fresnel curves - a blend weight would be a fitted
  constant (#57 rule);
- a Layered material (143) over the OpenPBR skin, the Top Coat its
  Specular Layer (139), as `layered_coat_material` already does for Uber
  (#106). The manual's Layered Material page allows any material as the
  base. Taken: the OpenPBR skin keeps the uncoated mapping of #106 (dual
  lobe on `specular`, gloss on the coating, undarkened), which matched
  Iray within 3 % without a coat.

Code: the skin branch of `universal` takes `layered_top_coat` first (same
test as Uber: a coat over a gloss over a dual lobe, or an anisotropic
coat, outside Layering Mode 1); inside it `skin_gloss` sees no coat
(`in_layered_coat`). No new constant.

## Measurement
Private Genesis 8 base skin scene (store skin, not in git),
Calib-Skin-Base-hdri-coat0 / -dlgloss / -dlglosstc (`make-skin-variants.py`),
HDRI. Mean linear luminance per region (skin-compare.py's boxes plus the
face box 380 - 580 x 110 - 330), ratios within one renderer:

| region | Iray dlgloss / coat0 | Iray dlglosstc / coat0 | Octane before | Octane now | now / Iray |
|---|---|---|---|---|---|
| forehead | 1.15 | 1.16 | 1.02 | 1.19 | 1.03 |
| cheek L | 1.29 | 1.30 | 1.01 | 1.34 | 1.03 |
| cheek R | 1.00 | 1.02 | 1.03 | 1.09 | 1.07 |
| nose | 1.29 | 1.31 | 1.01 | 1.35 | 1.03 |
| ear R | 1.38 | 1.39 | 1.02 | 1.44 | 1.03 |
| neck | 1.14 | 1.15 | 1.01 | 1.17 | 1.03 |
| chest | 1.17 | 1.17 | 1.01 | 1.19 | 1.01 |
| face box | 1.34 | 1.35 | 1.02 | 1.39 | 1.03 |

The coated skin now rises as the uncoated one does (Octane dlgloss /
coat0 1.37 on the face box, 1.39 with the coat; Iray 1.34 / 1.35): the
coat adds 1 - 2 % in both. coat0 re-rendered: unchanged (1.000 on the
face box). The face's brightest 3 % (each image's own) rise 1.31 x in
Octane against Iray's 1.38 - the same with and without the coat (dlgloss
1.28), so it is the dual lobe's own highlight, not the layering (#137's
territory: each layer alone off by where it stands).

## Process
- No Iray needed: the decomposition references (coat0, dlgloss) from #106
  already split the coat from the dual lobe.
- `calib-variants.sh` warned of a locked screen but rendered (fresh PNGs
  at 16:53 / 16:54).

## Outcome
#138 closed. Open: #137 (the dual lobe's highlight itself).
