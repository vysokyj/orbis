# 2026-11-16 Normal map strength (#176)

#175 found that the export reads every Uber normal map at strength 1:
`Textures::detail_normal` / `linear_normal_map` took a normal map with no
factor, `detail_normal::Source::Normal` decoded it at 1. `src/iray.rs`
already handed `base_bump` the Normal Map's value as
`tangent_space_normal_texture`'s factor, but no fixture had a value other
than 1, so that was unproven too.

## What DAZ hands Iray (.mi first)

New `.mi`-only scenes over Calib-CoatBump's maps (red base, sharp clear
coat; `make-calib-scenes.py`): Calib-Mi-NormalStrength (N1 – N8),
its -Shown twin (every channel shown), Calib-Mi-NormalStrength2 (Z1 –
Z8), plus Calib-DualLobePores (#175) for its P5. One headless DAZ run each
(`daz-export-mi.dsa`), fixtures by `mi-oracle.py`.

| Sphere | Channel | Iray gets |
|--------|---------|-----------|
| N1 | Normal Map 1 | `base_bump` normal texture, factor 1 (default) |
| N2, P5 | Normal Map 0.5 | factor 0.5 |
| Z6 | Normal Map 0.25 | factor 0.25 |
| Z7 | Normal Map 0.01 | factor 0.01 |
| N4 | Normal Map 2 (saved) | factor 1: the channel's definition says `min` 0, `max` 1, `clamped` |
| N3 | Normal Map 0 | no `base_bump`, `base_bump_enable` false |
| N5 | Normal Map 0.5 + height | factor 0.5 base, the height map as `base_bump2` |
| Z1 | Normal Map 0 + height 1 | the height map as `base_bump` alone (no `base_bump2`) |
| Z2, Z3 | Bump Strength 0 (± Normal Map 0) | nothing |
| N6 | Top Coat Bump 0.5, Mode 1 | `top_coat_bump` normal texture, factor 0.5 |
| N7 | coat normal 0.5 + base normal 0.5 | both factor 0.5 |
| N8, Z4 | Top Coat Bump 0 (Mode 1 / Mode 0) | no `top_coat_bump`, enable false |
| Z5 | coat normal 0 + base normal 1 | base only |
| Z8 | coat normal 0.5, Top Coat Weight 0 | base only (as before) |

Hidden and shown twins identical. Three things `iray.rs` had wrong, all
now in it with the fixtures: a map at 0 passed (as factor 0), the Normal
Map unclamped (N4: ours 2), and the Normal Map at 0 still the base bump
(Z1). The clamp is general: `Channel::range` keeps the definition's
`min` / `max` where it says `clamped` (an override without them keeps
the definition's), and `Material::float` / `Channels::float` hold the
value inside; every other fixture still passes, so no other generated
scene relied on an out-of-range value reaching Iray.

## The export

MDL SDK `base.mdl` (the formula the issue cites): the texel decodes as
t = (c − 0.5) · 2 · factor, then `apply_bump` on the tangents with the
normal's weight |t.z| + 1 − factor: n = normalize(t.x T_u + t.y T_v +
(|t.z| + 1 − factor) N). At 0 the plain normal, at 1 the map. No constant.

- `Source::Normal` takes the factor; `material.rs`'s `Bump` carries a
  `BumpKind` (height or normal, each at its factor) instead of
  `factor: Option<f32>`.
- `normal_factor`: the Normal Map's / Mode-1 Top Coat Bump's value (held
  in range), None at 0 — then the map is left out, as Iray does.
- A normal map alone at strength 1 and gamma 1 still goes in as the file;
  any other strength is baked (`linear_normal_map`), and the bake with a
  height map uses it too. The coat's Octane fallback likewise.
- PBRSkin's Normal Map stays at 1: no PBRSkin `.mi` with a value other
  than 1 yet.

Tests: `detail_normal::normal_map_strength_is_mdls_factor` (the formula at
0, 0.5, 1), `material::normal_map_strength_is_the_factor` (the pin's
baked texel at 1 / 2 / 0.5 / 0, the coat's Mode 1 map at 0.5 / 0), and
the three new fixtures in `iray::uber_matches_daz_mi_export`.

## Octane check (Calib-DualLobePores, enhanced-final, 2000 samples)

Per sphere, linear luminance inside 0.9 R: mean and the RMS of the image
minus a 5 × 5 box blur (texture from the pores), Octane's denoised pass.

| Sphere | before (#175, denoised) | after (denoised) | after (raw) |
|--------|------|------|------|
| P1 smooth | 0.0434 / 0.00156 | 0.0434 / 0.00157 | 0.00158 |
| P4 normal 1 | 0.0434 / 0.00272 | 0.0434 / 0.00280 | 0.00360 |
| P5 normal 0.5 | 0.0433 / **0.00276** | 0.0433 / **0.00236** | 0.00295 |
| P6 normal + height 2 | 0.0430 / 0.00465 | 0.0430 / 0.00461 | 0.00486 |

(mean / high-pass RMS.) Before, P5 rendered as P4 (the bug); now its
texture lies between the smooth P1 and P4, the means unchanged (a normal
map moves no light on average). Every other sphere within noise of #175's
render. Render 2000 samples, ~2 min. No Iray reference yet, so no
Octane / Iray ratio.

## Open

Iray's P4 / P5 (#175's batch) confirm the strength on a render; the
conversion itself is proven by the `.mi`, so it ships without it.
