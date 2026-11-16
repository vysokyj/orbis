# 2026-11-17 — The dual lobe at Reflectivity 0 (#197)

#193 found it: at Dual Lobe Specular Reflectivity 0 Iray still draws a
grazing lobe, the export none. Map it (a minimum IOR, the Schlick curve
on the pins) or leave it? The issue asked to check real content first.
No new render: #193's Iray references and an analytic Fresnel table.

## 1. Docs: what Iray does

`add_dual_lobe_specular` (`irayubermaterial.mdl`): `custom_curve_layer`
with normal_reflectivity 1 − R W, grazing 1 − W, the diffuse its layer,
the two lobes its base. The lobes get the curve's complement,
W (R + (1 − R)(1 − cos)^5) — Schlick at F0 R, scaled by W. At R 0 that is
W (1 − cos)^5: nothing head-on, W at grazing (`libbsdf` takes the cosine
at the less grazing of view and light, `src/mdl_layer.rs`). The export
puts the F0 on the pins as the IOR of F0, so F0 0 is IOR 1 and Fresnel 0
at every angle.

## 2. Iray's lobe at R 0 (#193's references)

Lobe over the base, the -Black twin over Calib-*-DualLobeBase-Off, per
ring of r / R (Iray, home server; base albedo 0.5):

| set | sphere | 0 – 0.3 | 0.5 – 0.6 | 0.7 – 0.8 | 0.8 – 0.85 | 0.85 – 0.9 | 0.9 – 0.93 | disc |
|---|---|---|---|---|---|---|---|---|
| Furnace | C1 (R 0.5) | 0.180 | 0.175 | 0.191 | 0.222 | 0.275 | 0.359 | 0.205 |
| Furnace | C4 (R 0) | 0.000 | 0.000 | 0.012 | 0.039 | 0.096 | 0.192 | 0.028 |
| Point | C4 (R 0) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |

Under the light at the camera a sharp lobe reflects only head-on, where
the curve is 0. In the furnace C4's lobe is half of these as directional
albedo (0.014 over the disc, 0.096 at the rim) — Schlick's (1 − cos)^5
averaged per ring is 0.0113 over the disc, 0.077 at 0.9 – 0.93 R, the
lobe's spread adding the rest.

## 3. Could an IOR draw it?

Fresnel per ring (orthographic view, cos = √(1 − s²)), against Schlick
at F0 0:

| curve | 0 – 0.3 | 0.6 – 0.7 | 0.8 – 0.85 | 0.85 – 0.9 | 0.9 – 0.93 | disc |
|---|---|---|---|---|---|---|
| Schlick F0 0 | 0.0000 | 0.0009 | 0.0161 | 0.0379 | 0.0768 | 0.0113 |
| IOR 1.03 | 0.0002 | 0.0003 | 0.0011 | 0.0022 | 0.0045 | 0.0009 |
| IOR 1.07 | 0.0011 | 0.0017 | 0.0048 | 0.0089 | 0.0170 | 0.0037 |
| IOR 1.10 | 0.0023 | 0.0032 | 0.0085 | 0.0151 | 0.0276 | 0.0064 |
| IOR 1.15 | 0.0049 | 0.0066 | 0.0158 | 0.0263 | 0.0450 | 0.0118 |
| IOR 1.22 | 0.0098 | 0.0127 | 0.0268 | 0.0419 | 0.0672 | 0.0203 |

No IOR has the shape: low IORs rise too late, and the one that matches
the disc (1.15) reflects 0.5 % head-on where Iray reflects nothing and
gives 0.045 of the 0.077 at the rim. The Top Coat's Custom Curve IOR 1.07
(`CURVE_COAT_IOR`, the same curve) would give a third of the disc, a
fifth of the rim. The Schlick curve on the pins needs a Fresnel-free
lobe: an IOR near infinity with a falloff texture on the weight, and
Octane's Falloff Map is a power of the angle, not of 1 − cos — a new
construct for this one case.

## 4. Real content

A scan of both installed libraries (`.duf` / `.dsf`, every material entry
with a Dual Lobe Specular Weight or Reflectivity channel, the saved
value): 233 682 materials, 217 942 of them W 0 R 0.5 (the Uber default).
At W > 0 and R ≤ 0.01: 9 — the four Calib-*-DualLobeCurve scenes, one
store character's mouth at W 1, and four mouths / teeth of another at
W 0.016 (a lobe of 1.6 % at grazing). Nothing a viewer sees.

## Where it stands

- Left, as Glossy Reflectivity 0 (#137): commented at `DualLobe::of`'s F0,
  the spec's dual lobe rule says so. Revisit if real content turns up
  that needs it.
- No export change, no scene generated, no Iray.

## Process

- A content scan settles "is this worth mapping" before any scene or
  render (`json` walk of every material entry, gzip-aware; both
  libraries in ~8 minutes).
