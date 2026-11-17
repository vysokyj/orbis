# Tuning choices

The judgment calls in the export: values where neither "match Iray" nor
"match the physics" decides, so the user picked one look. Each row names
the value, the options weighed, what was chosen and why, and whether it
would make sense as an export option (the DAZ dialog / `orbis export`)
rather than a fixed value. The numbers behind each row are in the spec
(`docs/spec/octane-export.md`) and the lab notes; the code carries the
same reasoning in a comment at the value (`src/tuning.rs` unless noted).

Open choices still waiting for a decision are GitHub issues labelled
`question`; a decided one moves here.

| Choice | Value (Enhanced / Faithful) | Options weighed | Why this one | Option candidate |
|---|---|---|---|---|
| Skin detail contrast s (`skin_detail_contrast`; #47, #179, 2026-11-17) | 0.3 / 0 | 0 (Iray's texture); 0.15 (safe, texture mostly gone at 400 samples); **0.3** (crisper, dark skin a touch ashier, texture jumps in with grain at 400 samples); 0.5 (speckled); split by skin tone (0.3 light / 0.15 dark) | Reads more natural on face close-ups, the ashy tint included | **Yes** - a "skin detail" slider (0 - 0.5); how crisp skin looks also depends on the sample count |
| Final sample count (`FINAL_SAMPLES`; #180) | 500 / 500 | 500 (former default); 2000 (pores 15 - 25 % crisper, 91 s -> 428 s at 960 x 720) | Time over detail by default | Already one (`--samples`, the dialog) |
| Rough dielectric lobe model (`rough_lobe`; #162, #175, #196, 2026-11-16) | GGX energy preserving / STD | GGX EP (Iray's level under an environment, highlight ~0.8 x as wide); STD (Iray's highlight shape, rough gloss 1.2 - 1.5 x under an HDRI) | Neither model has Iray's lobe in both shape and level; Enhanced keeps the level, Faithful the shape | Follows the Look |
| Rough metal energy (`metal_std_spread`, STD `spread` on rough all-metal nodes; #189, 2026-11-17) | default 0.5 / 0.8 | **Faithful only** at 0.8 (Iray's level: furnace 1.01 - 1.05, HDRI 1.00 - 1.03 x; highlight core 2 - 4 px narrower); both Looks; neither (Octane 1.02 - 1.17 x Iray, rising with r) | Iray's single-scatter lobe loses energy a real metal keeps; Enhanced keeps Octane's more plausible level. Judge again on real scenes (armour, Demo-*) | Maybe - a per-export "metal energy" choice if armour looks off |
| Legacy rim on textured `mdl 1.0` metals (`legacy_metal_rim`; #112) | off / on | Octane's flat metal; Iray's rim as a Falloff Mix | A texture lookup changing the BSDF isn't physical | Follows the Look |
| Geometry below Iray's ground (`clip_below_ground`; #69, 2026-11-16) | kept / cut | Keep it visible through the shadow catcher; cut it at the ground's height as Iray's matte ground hides it | Faithful shows what Iray shows; Enhanced keeps the scene whole | Follows the Look |
| Uber dual lobe at Reflectivity 0 (`DualLobe::of`, `src/material.rs`; #197, #137) | no lobe / no lobe | Leave it (IOR 1); an IOR matching the disc mean (lights the core Iray leaves black) | No IOR draws Iray's grazing-only shape; 5 of 233 682 library materials hit it | No |
| GGX dielectric limit (`GGX_DIELECTRIC_MAX_ALPHA`, `src/material.rs`; #186) | α 0.25 / 0.25 | Anywhere between α 0.16 (plain GGX flat v Iray) and 0.36 (plain GGX 1.03 - 1.19) | Beckmann's measured limit, one rule for both | No |
| Skin map means over the white fill (`skin_walk_weight`, `skin_albedo` read `mean_covered_colour` with the old near-black rule, `src/material.rs`; #82, #86, 2026-11-17) | old mean / old mean | **Keep the old mean** (white padding between UV islands counted: Monique's Translucency Color mean 0.62, her front 1.00 / 0.96 / 1.07 x Iray); the UV-covered mean (0.28, front 0.89 / 0.83 / 0.94 x, #86 step 8) with the walk weight S re-decomposed | The rules were calibrated on the old mean and match Iray. **Something is probably missing:** the fill likely stands in for a component the walk-weight rule doesn't model; the honest mean makes it worse. Find it by decomposition (flat-Tc / tcwhite variants of Monique and Twosret), not by refitting - or let the direct PBRSkin mapping (#57) replace it | No |
| Ribbon hair on Octane's round curves (Morgan Hair, Line Tessellation 2; #161, 2026-11-17, open) | Octane's own look / Iray's ribbons | Hold it to the **ribbon** references (what DAZ shows: Octane 0.79 - 1.27 x, cutout along the strand still to fix); to the **fibre** references (Iray at Line Tessellation 1: Octane 0.58 - 0.74 x, a grey deep-pile base left) | **Split by Look:** Faithful follows the ribbons; Enhanced may drift in colour - the user finds Octane's curves more natural as they are. To be tuned on more hair examples, not Morgan alone | Follows the Look |
