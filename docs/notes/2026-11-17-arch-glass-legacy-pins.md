# 2026-11-17 — Architectural glass at the legacy Phong pin (#198)

Question (#198, from #191): every `mdl 1.0` lobe, the transmitting ones
included, is a Phong of exponent e (cosᵉ), and resin's base and coat had
moved to `legacy_phong_pin`. NVIDIA's architectural closed glass still
wrote √r for its transmit lobe (r = r_t, `arch_glass_rough`) and for the
coating over it — the pins the roughness rule had been read with (#21,
#117). Does the derived pin match Iray better?

What r_t is: Iray's own roughness, read among the plain `mdl 1.0` lobe's
twins at constant r in the white furnace (2026-11-15 note, A1 – A3 =
G3 – G5). Nothing in it was fitted to an Octane render except the
extrapolation above refr_gloss 0.4 (Calib-NVIDIA3's look at r 0.06 – 0.09,
where √r and the legacy pin differ by < 0.5 %). So the derived Octane pin
is `legacy_phong_pin(r_t)`: α 0.585 / 0.514 / 0.205 at refr_gloss 0 /
0.2 / 0.4 where √r gave 0.72 / 0.60 / 0.21.

## Change

- The pin closure for every NVIDIA dielectric is `legacy_phong_pin`;
  `MdlDielectric::arch_glass_pins` is gone.
- `arch_glass_albedo`'s Octane stack takes the same α
  (`legacy_phong_alpha`) for the lobe and the coating; the Iray side was
  already on `legacy_rough` (#191).
- k (reflectivity 0, refr_gloss 0.6 / 0.4 / 0.2 / 0): 0.98 / 0.97 / 0.85 /
  0.93 → 0.99 / 0.98 / 0.88 / 0.84; 0.2 under reflectivity 0.6 0.85 →
  0.87, with Use Fresnel Term 0.73 → 0.67.

No new scenes, no Iray: the references of Calib-Furnace-ArchGlass / -2 /
-3, Calib-ArchGlass and Calib-NVIDIA3 (Iray Server 5000) were there.
Octane enhanced-final, 400 samples, denoised pass; before = master at
5ad5b3b (√r; ArchGlass / -2 from #191's after-renders, -3 and NVIDIA3
re-rendered at 5ad5b3b), after = this change. Octane ~25 min for the five
scenes (Calib-ArchGlass 270 s).

## Lit: the blur through the glass

`lit.py` (scratch): linear luminance, inside 0.85 R of each projected
disc (`sphere_discs`), a band-pass (Gaussian σ lo − σ hi) of Octane and
Iray; detail = the band's std Octane / Iray, corr = their correlation.

σ 1.5 – 6 px, √r → legacy pin:

| sphere | mean | detail | corr |
|---|---|---|---|
| G3 refr_gloss 0.4 | 1.022 → 1.022 | 0.960 → 0.984 | 0.998 → 0.998 |
| G4 0.2 | 1.057 → 1.050 | 1.005 → 1.021 | 0.941 → 0.966 |
| G5 0 | 1.054 → 1.082 | 1.077 → 1.016 | 0.958 → 0.978 |
| G7 0.2 refl 0.6 | 1.045 → 1.057 | 1.018 → 1.001 | 0.998 → 0.998 |
| G8 0.2 refl 0.6 Fresnel | 1.097 → 1.119 | 0.965 → 0.977 | 0.992 → 0.994 |
| A7 (NVIDIA3) glass 0.2 | 1.088 → 1.080 | 1.024 → 1.012 | 0.934 → 0.968 |
| A8 glass 0.2 refl | 1.073 → 1.085 | 1.017 → 1.009 | 0.998 → 0.998 |

σ 3 – 12: corr G4 0.977 → 0.989, G5 0.984 → 0.995, G8 0.968 → 0.985, A7
0.977 → 0.992. σ 0.7 – 3 (mostly grain on the rough spheres): corr G4 0.67
→ 0.73, G5 0.42 → 0.59, A7 0.56 → 0.68. The smooth spheres (G1, G2, G6,
A1 – A6) are unchanged to 0.003. In every band the rough glass draws the
background through it closer to Iray at the derived pin — the one thing
the pin decides.

## Furnace: levels and ring shape

`furnace-rings.py measure`, Octane / Iray; shape = rms over the rings of
(ring / centre) Octane over Iray.

| sphere | disk | 0.8 – 0.9 | shape rms |
|---|---|---|---|
| G4 refr 0.2 | 1.043 → 1.030 | 1.044 → 1.074 | 0.005 → 0.016 |
| G5 refr 0 | 1.026 → 1.053 | 1.014 → 1.058 | 0.031 → 0.017 |
| G7 0.2 refl 0.6 | 1.014 → 1.023 | 1.031 → 1.056 | 0.015 → 0.012 |
| G8 Fresnel | 0.993 → 1.005 | 0.979 → 1.011 | 0.011 → 0.006 |
| H2 refr 0.2 | 1.048 → 1.033 | 1.044 → 1.079 | 0.006 → 0.016 |
| H3 refr 0 | 1.031 → 1.057 | 1.023 → 1.064 | 0.032 → 0.017 |
| H6 IOR 1.2 | 1.053 → 1.028 | 1.017 → 1.021 | 0.022 → 0.013 |
| H7 IOR 2 | 0.998 → 1.073 | 0.991 → 1.116 | 0.005 → 0.035 |
| H8 colour 0.5 | 1.107 → 1.107 | 1.294 → 1.353 | 0.057 → 0.076 |

Thin walls (H4 / H5, J1 – J8) and the smooth spheres unchanged (Octane's
thin wall doesn't blur, #136). Mean |disk − 1| over the closed rough
spheres 0.037 → 0.045, nearly all of it H7 and G5 / H3.

Decomposed: k is solved per material against the Octane Smith model, so a
furnace disk off 1 is Octane's render over its own model (the Iray side is
the measured `ARCH_GLASS_LOBE`). At the legacy α the model's k falls with
the IOR (0.899 / 0.876 / 0.853 at 1.2 / 1.5 / 2; at √r it was flat, 0.90)
and the renders don't follow: H7 keeps 7 % more. That is #185's finding —
Octane keeps more than the Smith model in rough dielectric events where
facets inside reflect totally, and IOR 2 has the most of them. It was there
before too: at √r the model's flat k left the same IOR slope in the
renders the other way (H6 / H2 / H7 1.05 / 1.05 / 1.00). Whether to fold
that excess in is the user's open call in #185; it doesn't decide the pin.

## Decision

The derived pin: it follows Iray's own lobe (r_t is read on it), it
draws the lit glass closer to Iray on every rough sphere, and its level
residue is #185's. The √r pins and the `arch_glass_pins` switch are
dropped. The test that asserted k flat in the IOR now asserts the new
values (k 0.876 at IOR 1.5, a slope < 0.07 from 1.2 to 2).

## Where it stands

Export changed (architectural glass's pins and its k). The IOR slope of
the Octane model over the render (H7) belongs to #185.

## Process lessons

- The furnace can't judge a pin on its own: k re-solves the level, and
  ring shape barely moves with the lobe width in a uniform dome. The lit
  scene's band-passed detail inside the disc is the measure that reads
  the lobe's width.
- Check what a "swept" value was read against before keeping it: r_t
  came from Iray-only twins, so only the Octane pin had been a choice.
