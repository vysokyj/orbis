# 2026-11-16 — What a dielectric lobe leaves of the diffuse under it (#193)

#193 (from #190): under Calib-Point-UberGloss's light at the camera a grey
base 0.5 under a dielectric lobe (F0 0.04) kept 0.963 × itself at
Beckmann and 0.982 at STD; which of them is Iray's dual lobe curve, per
angle? Decompose before fitting; Iray twins needed, prepared here, the
Octane side rendered.

## 1. Docs: what Iray does

- `add_dual_lobe_specular` (`irayubermaterial.mdl`): the *diffuse* is the
  layer of a `custom_curve_layer` (normal_reflectivity 1 − R W,
  grazing 1 − W, exponent 5) whose base is the white pair of lobes.
- MDL spec 21.6: the base of a curve layer is weighted 1 − w · curve.
- `libbsdf` (`src/mdl_layer.rs`'s notes): the layer is weighted
  w · c(k1 · h), the base 1 − w · max(c(n · k1), c(n · k2)). With the
  diffuse as the layer, its share depends on the half angle between the
  view and the light, **not on the view angle**: under a light at the
  camera (k1 · h = 1) it is flat 1 − R W; in a white furnace (new
  `hdri-lobe.py predict`) at W 1, R 0.04 0.960 head-on to 0.953 at
  0.9 – 0.93 R (disc 0.958); R 0.08 0.920 / 0.913, W 0.5 0.980 / 0.976.
  The lobes themselves get the curve's complement at the macro normal,
  0.04 head-on to 1 at grazing.

## 2. #190's numbers were clipped

Calib-Point-UberGloss at EV 3.4 puts a grey 0.5 Lambert's centre at
1.2 – 1.35 linear: 38 – 56 % of each disc's pixels at 255 in #190's
grey-base renders (`yB`, `yD1`). The 0.963 / 0.982 are means over half
clipped discs. New scenes at EV 6.1 (grey centre ~0.2, brightest pixel
207 of 255 with pin 0.3's lobe on it).

## 3. Scenes (make-mdl-scenes.py, Calib-Furnace-Metal's spheres)

Uber, no gloss, Metallic Weight 0 (the furnace spheres are metals: the
first generation forgot it and every sphere exported as a metal of
albedo 0.22, caught on the export's `metallic` pin before any render was
used); three twins each - grey 0.5 under the dual lobe, the lobe over
black, the grey alone (`-Off`) - so (grey − black) / off is the diffuse's
share per ring (`hdri-lobe.py share`):

- Calib-Point-DualLobeBase-Grey / -Black / -Off: light at the camera,
  EV 6.1; P1 – P8 W 1, Reflectivity 0.5 (F0 0.04), both lobes at the MDL
  roughness whose `phong_pin` is 0.3 / 0.4 / 0.448 (Nishi's) / 0.5 / 0.6
  / 0.7 / 0.8 / 0.9.
- Calib-Point-DualLobeCurve-Grey / -Black: pin 0.448; C1 W 1 R 0.5, C2 R 1,
  C3 R 0.25, C4 R 0, C5 W 0.5, C6 W 0.25, C7 lobe 2 at pin 0.3, C8 lobe 2
  at pin 0.8 ratio 0.5 (the off twin is Base-Off's).
- Calib-Furnace-DualLobeBase-* / -DualLobeCurve-*: the same under the
  uniform dome.

## 4. Octane per model (hdri-lobe variants)

Every sphere replaced by Universal (diffuse 0.5, specular 1, IOR 1.5) at
the label's pin per `brdf`, the lobe over black, OpenPBR's grey alone;
1024 samples, ~65 s a render, denoised (raw within 0.002). Light at the
camera, diffuse share per ring of r / R:

| model | pin | 0 – 0.3 | 0.5 – 0.6 | 0.7 – 0.8 | 0.85 – 0.9 | 0.9 – 0.93 | disc |
|---|---|---|---|---|---|---|---|
| Beckmann | 0.3 | 0.960 | 0.958 | 0.945 | 0.903 | 0.865 | 0.947 |
| Beckmann | 0.448 | 0.959 | 0.957 | 0.941 | 0.888 | 0.840 | 0.943 |
| Beckmann | 0.6 | 0.959 | 0.952 | 0.921 | 0.839 | 0.784 | 0.930 |
| Beckmann | 0.9 | 0.951 | 0.906 | 0.829 | 0.770 | 0.753 | 0.879 |
| GGX | 0.3 | 0.960 | 0.957 | 0.943 | 0.904 | 0.872 | 0.946 |
| GGX | 0.448 | 0.956 | 0.952 | 0.939 | 0.905 | 0.880 | 0.944 |
| GGX | 0.9 | 0.926 | 0.928 | 0.924 | 0.918 | 0.915 | 0.925 |
| STD | 0.3 – 0.9 | 0.960 | 0.961 | 0.960 – 0.963 | 0.959 – 0.966 | 0.958 – 0.966 | 0.960 – 0.962 |

**STD leaves the diffuse 1 − F0 flat at every ring and pin - what
`libbsdf` predicts for Iray's dual lobe under this light.** Beckmann
leaves 1 − F0 head-on and falls toward the rim, the more the rougher
(0.75 at pin 0.9); GGX between (rough pins lower head-on too). #190's
"Beckmann 0.963, STD 0.982" was clipping; head-on all three agree.

In the white furnace (Calib-Furnace-DualLobeBase-Grey's variants) every
model falls toward the rim, STD included:

| model | pin | 0 – 0.3 | 0.5 – 0.6 | 0.7 – 0.8 | 0.85 – 0.9 | 0.9 – 0.93 | disc |
|---|---|---|---|---|---|---|---|
| Beckmann | 0.3 | 0.963 | 0.966 | 0.948 | 0.904 | 0.866 | 0.946 |
| Beckmann | 0.448 | 0.970 | 0.963 | 0.944 | 0.893 | 0.843 | 0.942 |
| Beckmann | 0.9 | 0.945 | 0.908 | 0.831 | 0.780 | 0.766 | 0.865 |
| GGX | 0.448 | 0.961 | 0.955 | 0.942 | 0.902 | 0.876 | 0.940 |
| GGX | 0.9 | 0.920 | 0.924 | 0.918 | 0.913 | 0.913 | 0.920 |
| STD | 0.3 | 0.963 | 0.965 | 0.944 | 0.891 | 0.842 | 0.942 |
| STD | 0.448 | 0.970 | 0.962 | 0.943 | 0.884 | 0.828 | 0.939 |
| STD | 0.9 | 0.948 | 0.907 | 0.854 | 0.823 | 0.815 | 0.882 |
| `libbsdf` (Iray, predicted) | any | 0.960 | 0.959 | 0.958 | 0.955 | 0.953 | 0.958 |

So no Octane model is Iray's curve everywhere (if Iray is `libbsdf`'s):
STD's base is right under a light at the camera and loses 10 – 15 % too
much at the rim in the furnace; Beckmann loses at the rim under both.
STD's share under the coaxial light is flat while it falls in the
furnace: its base weight is not a function of the view angle alone (nor
of k · h alone - that would keep the furnace near 0.95); the Iray twins
say which part is Iray's.

## 5. The export as shipped

Point light, (Grey − Black) / Off of the export (diffuse 0.5 as the
export's albedo 0.218; lobe 1 on `specular` at W × 0.85, lobe 2 the
coating at 0.15): Enhanced = Beckmann at pins ≤ 0.448, GGX energy
preserving above; Faithful the same up to 0.448 and STD above. Pins
≤ 0.448 as Beckmann above (rim 0.84 – 0.87), pins ≥ 0.5 flat 0.962 in
both Looks (GGX EP flat like STD under this light). Furnace, both Looks:
0.93 – 0.96 head-on, 0.81 – 0.87 at 0.9 – 0.93 R.

Calib-Point-DualLobeCurve (Faithful, pin 0.448, Beckmann): head-on the
share is 1 − R W to 0.001 on every sphere - C2 (F0 0.08) 0.920, C3 0.980,
C4 (R 0) 1.000, C5 (W 0.5) 0.980, C6 (W 0.25) 0.990 - the export's W on
the pins and F0 as the IOR give the curve's head-on value; the rim falls
as Beckmann's. C4 (Reflectivity 0) is flat 1.000 with no lobe at all
(lobe / base 0.000): F0 0 is IOR 1, nothing reflects, where Iray's
curve still gives the lobes W (1 − cos)^5 toward the rim - the -Black
twin's C4 shows how much. The second lobe's
roughness / ratio (C7 / C8) moves only C8's rim (lobe 2 at pin 0.8 is
STD in Faithful: flat 0.962).

## 6. Iray (resume, references bbf7014)

The ten twins on the home Iray Server (2000 samples point, 5000 furnace,
denoised). (Grey − Black) / Off per ring:

| set | sphere | 0 – 0.3 | 0.5 – 0.6 | 0.7 – 0.8 | 0.85 – 0.9 | 0.9 – 0.93 | disc |
|---|---|---|---|---|---|---|---|
| Point Base | P1 – P8 (pin 0.3 – 0.9) | 0.956 – 0.961 | 0.961 | 0.961 – 0.963 | 0.961 – 0.965 | 0.960 – 0.965 | 0.960 – 0.962 |
| Furnace Base | P1 – P8 | 0.942 – 0.966 | 0.949 – 0.960 | 0.954 – 0.961 | 0.952 – 0.960 | 0.948 – 0.959 | 0.954 – 0.962 |
| Point Curve | C2 (F0 0.08) | 0.911 | 0.921 | 0.923 | 0.927 | 0.927 | 0.921 |
| Point Curve | C3 / C4 / C5 / C6 | 0.980 / 1.000 / 0.979 / 0.991 | | | | | 0.981 / 1.000 / 0.981 / 0.991 |
| Furnace Curve | C2 | 0.924 | 0.925 | 0.931 | 0.925 | 0.918 | 0.925 |
| Furnace Curve | C4 (R 0) | 1.000 | 1.000 | 0.997 | 0.996 | 0.992 | 0.998 |

**Iray is `libbsdf`'s curve layer**: the diffuse keeps a flat 1 − R W
at every ring, pin, second-lobe setting and light (the furnace's
0.94 – 0.97 head-on is noise on a 5000-sample render; the predicted
0.960 → 0.953 rim fall is within it). Answer to the issue: neither
Beckmann nor STD everywhere. STD is Iray's base under a light near the
view; Beckmann loses 13 – 16 % too much at the rim there. In the furnace
every Octane model loses 10 – 15 % at the rim where Iray loses nothing.
That loss belongs to the BRDF model, and no pin removes it.

The export as shipped against these (shares as in section 5):

- Point: pins ≤ 0.448 (Beckmann, both Looks) 0.84 – 0.87 at the rim, disc
  0.944 – 0.948 vs Iray's 0.961; pins ≥ 0.5 (STD / GGX EP) 0.962 = Iray.
- Furnace: disc 0.886 – 0.937 (Faithful), 0.921 – 0.937 (Enhanced) vs
  0.954 – 0.962; rim 0.81 – 0.87 (Enhanced's GGX EP 0.87 – 0.92).

The lobe alone (-Black twins, export / Iray, disc): point Faithful
0.986 – 1.012, Enhanced 0.89 – 1.06 (GGX EP's rough pins); furnace
Faithful 1.22 – 1.35, Enhanced 0.96 – 1.34, per ring 1.09 head-on to
1.4 – 1.6 at 0.85 – 0.9 R at the sharp pins - with the grey alone at
1.037 × Iray in the furnace (the dome's level, as Calib-Env-DomeMode1's
1.03 – 1.04; 1.007 – 1.019 under the point light), head-on ~1.05 and the
rim the curve's Fresnel shape (#188, kept). Under an environment Octane
moves energy from the base into the lobe at grazing; Iray does not.
Grey net (export / Iray, disc): point 0.99 – 1.01, furnace 1.00 – 1.07.

Per model (the c193 Universal variants' lobes over black, disc / Iray),
sharp pins 0.3 – 0.448: Beckmann point 1.015 – 1.018, furnace 1.29 –
1.36; **STD point 1.013 – 1.015, furnace 1.29 – 1.33**. STD draws the
same sharp lobe as Beckmann and keeps the base under a light near the
view. Faithful at STD for sharp dielectric lobes too is #196 (contested,
to judge on real scenes). C4 (Reflectivity 0): Iray's lobe alone 0.000
under the point light, 0.028 × the base in the furnace (W (1 − cos)^5 at
grazing); the export draws none (#197).

## Where it stands

- #193 answered: Iray's diffuse share is flat 1 − R W (`libbsdf`), no
  Octane model is that everywhere; STD is it under a light near the view.
- No export change here: the Uber skin's Faithful dual lobe already sits
  at STD (#175); sharp dielectric lobes elsewhere at STD in Faithful is
  #196; the R 0 grazing lobe #197.
- dlflat − gloss0 stays #175's metric: it is the dual lobe's net effect
  (lobe minus the base it takes), compared with Iray's same net; under a
  light near the view the Faithful skin's base part is Iray's (STD), so
  the difference there is the lobe's. Under an environment it mixes in
  the rim trade above; split it with -Black twins when that matters.

## Process

- Check a render's clipping before reading a ratio off it: a grey base
  under a lit lobe needs its own exposure (#190's EV was set for a
  lobe over black).
- A generated scene's material channels: start from a scene whose
  spheres are metals and every channel that makes them metals has to be
  set back; read the export's pins before the first render.
