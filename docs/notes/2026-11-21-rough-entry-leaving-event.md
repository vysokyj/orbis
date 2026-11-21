# 2026-11-21 — Rough resin rims: Octane's excess sits on the event that leaves (#213)

Follow-up of #212 (`2026-11-21-rough-resin-entry.md`, § 3): with a rough
outside over a smooth inside (probe `side`), Octane kept 8 / 22 % more than
Beckmann + Smith at rings 0.7 / 0.8 – 0.9 and, under a falloff on the
transmission, kept relatively more ("bent nearer the normal"). Octane only,
no Iray. All probes on Calib-Furnace-Resin2's Faithful export
`--no-denoise`, re-pinned with `orbx-variants.py set=` (Beckmann `brdf` 1,
IOR 1.49, coating 0, albedo 0 set on every sphere), 1024 samples raw,
`furnace-rings.py measure`; roughness pin √α. Model: the Smith tracer
(`diag213` in `src/mdl_layer.rs`, ignored tests). Octane renders: 54 – 190 s
per job of eight spheres.

## 1. Probes

| probe | spheres | isolates |
|---|---|---|
| `brdf` | rough out α 0.707 / smooth in: Beckmann, GGX, GGX EP, STD (rt); Beckmann and GGX with transmission 0; Beckmann and GGX with specular 0 | the entry's reflection alone; the entry's refraction without the Fresnel pick; the BRDF model |
| `side` (repeat) | #212's eight spheres | the repeat: every ring within 0.002 |
| `side-subd2` | the same, the import subdividing every mesh twice (`subdLevel` 2, `usdImpOverrideSubdivLevel`) | the mesh |
| `kinds` | IOR 0.6711 (a bubble: the outside is an inside-kind event) rt / reflection alone / specular 0; reflection alone at α 0.5 / 1, IOR 1.49 and 0.6711 | the inside kind of event |
| `fr2` | both faces α 0.707: flat and seven falloffs (q 0 – 0.75, s 0.5 – 3.1) | the check |
| `refl` | reflection alone at α 0.2 – 0.6; the coating's reflection alone (specular 0, coating 1) at α 0.707 / 0.5 / 0.3 | the reflection table |
| `exit` | smooth outside, rough inside (`s<pin>,0`) at α 0.2 – 0.6; both faces α 0.5 flat / q 0, α 0.3 q 0 | the exit table, checks at other α |

## 2. The rough entry, split

Rough outside α 0.707, smooth inside, rings centre → 0.8 – 0.9:

| | Octane | model |
|---|---|---|
| rt (B1) | 0.999 … 0.945 0.737 0.448 | 0.985 … 0.910 0.679 0.364 |
| reflection alone (B5) | 0.032 … 0.049 0.063 0.083 | 0.031 … 0.036 0.040 0.047 |
| specular 0 (B7) | 1.001 … 0.947 0.720 0.398 | 1.000 … 0.937 0.702 0.372 |

The reflection alone is the model's head-on and 1.74 × at the rim (as #185's
`refl`, which had fake shadows on: they don't matter to it). Taking it out,
the transmitted part is 1.01 … 1.05 1.15 × the model. With those two
measured per ring and nothing else, `side` U4 (q 0) and U6 (q 0.5) are
predicted within 1.5 / 0.7 % on every ring (`print_entry_split`; the
model alone: rim 1.41 / 1.28). **The falloff reading of #212 was the
reflection's share**: the falloff can't reach the reflection, so a ring with
more reflection keeps relatively more under it. No change of direction is
needed.

The model's loss at the rim is almost all light the smooth inside traps:
0.59 of ring 0.8 – 0.9 is still inside after 32 events (a refraction through
a tilted facet past the critical angle, then total reflection for ever on a
perfect sphere). Octane lets some out through the mesh: subdivided twice the
same sphere's rim drops 0.446 → 0.425 (0.737 → 0.728 at 0.7 – 0.8; the
rough-inside spheres move < 0.2 %). A probe artefact of a smooth inside,
not a property of resin.

GGX (rt) sits 1.03 – 1.10 × its own GGX + Smith model, its reflection alone
1.14 × flat: smaller and differently shaped than Beckmann's; not followed
(the export writes Beckmann for resin).

## 3. Readings that don't hold

- Samples on the wrong side of the macro surface (`print_wrong_side`):
  kept as a transmission, swapped for the reflection, a reflection kept
  going in — the rough-entry sphere doesn't move (refraction rarely lands
  outside from outside), the rough-inside one breaks at the rim (0.88).
- Reflections below the horizon mirrored above (`print_reflection_flip`):
  the IOR-50 sphere would read 0.751 at the centre, Octane 0.696 (the model
  0.691). Refuted.
- Path depth (`print_depth`): 16 – 512 events change nothing.
- **Octane's single-event ratios on every event** (`print_event_model`:
  outside reflection, entry refraction, inside reflection from the bubble
  1.23 – 1.37 ×, exit): both-rough spheres overshoot by 10 – 20 % at the rim
  (`fr` flat 0.80 – 0.84, smooth entry / rough inside 0.90).

## 4. The event that leaves

Ratios put only on the event that leaves the sphere — the entry's
reflection by B5's ratio at its cosine, the exit by `side` U3's (smooth
entry, rough inside) at the internal cosine, solved in one pass so U3's own
paths meet U3 — and none on the events inside (`print_final_event_model`,
`print_fr2`):

| `fr2` sphere (α 0.707) | Octane / model, rim | Octane / leaving-event model, every ring |
|---|---|---|
| flat | 1.07 | 0.99 – 1.03 |
| q 0 s 1 | 1.27 | 0.99 – 1.04 |
| q 0 s 3.1 | 1.10 | 0.99 – 1.02 |
| q 0.5 s 1 | 1.13 | 0.99 – 1.03 |
| q 0.25 s 1 | 1.18 | 0.99 – 1.04 |
| q 0 s 0.5 | 1.55 | 0.99 – 1.04 |
| q 0.5 s 3 | 1.08 | 0.99 – 1.02 |
| q 0.75 s 1 | 1.10 | 0.99 – 1.03 |

The bubble agrees: its refraction (into the bubble, the event that doesn't
leave) is the model's (0.96 – 1.01). `fr`'s U8 (#211, q 0.5) read 0.419 at
the rim, `fr2`'s controlled twin 0.376: #211's U8 wasn't pinned like its
neighbours (it is the scene's coated sphere), so its 1.15 is not the model's.

Our reading (not documented by OTOY): Octane's light sampling reaches the
white environment only from the event that leaves — a closed glass without
fake shadows blocks shadow rays from inside — and its rough dielectric,
evaluated there, keeps more than its sampling does; with fake shadows the
shadow rays pass every wall, which is why `OCTANE_SPHERE`'s (Uber glass)
rims are so much brighter than `OCTANE_SPHERE_PLAIN`'s.

## 5. Tables, export, check

`OCTANE_REFLECTION` (α 0.2 – 1, the coating's reflection alone reads the
base's within 0.002 at α 0.5 / 0.707) and `OCTANE_EXIT` (α 0.2 – 0.707) in
`src/mdl_layer.rs`; their ratios over the model are computed at run time
(the reflection weighted by Fresnel instead of picked: the Bernoulli pick
left ring 0 / 1 ±6 – 11 % noisy). `leaving_excess` puts them on the
leaving event of a closed reflect + transmit base without fake shadows
(resin), the Faithful solve only (`rough_transmission_excess`); the
reflection's ratio by the coat's or base's α, linear from 1 at α 0.

Ratios (rim): reflection 1.01 / 0.98 / 1.07 / 1.24 / 1.46 / 1.75 / 2.40 at α
0.2 / 0.3 / 0.4 / 0.5 / 0.6 / 0.707 / 1; exit 1.00 – 1.10, highest head-on
at α 0.5 – 0.707.

The export's model (`octane_trace_c`, 2^16 paths) against Octane
(`print_leaving_excess_check`): `fr2` all eight within 0.95 (ring 0, the
trace's noise) and 0.99 – 1.04; `exit` U6 / U7 (α 0.5 flat / q 0) 0.98 –
1.01, U8 (α 0.3 q 0) 1.00 – 1.02; `OCTANE_SPHERE_PLAIN`'s flat rt rows
(IOR 1.5, measured for #201 on another scene) within 2 % at α 0.25 – 0.7,
5 / 8 % at α 0.85 / 1 (the exit table held past 0.707).

Faithful renders, 400 samples denoised, Octane / Iray (Iray Server 5000
references), disk (rings by area) / rim 0.8 – 0.9, before (0e7092c) →
after:

| sphere | disk | rim |
|---|---|---|
| Furnace-Resin2 U4 (r 0.5) | 1.013 → 0.998 | 1.037 → 1.025 |
| Furnace-Resin2 U5 (r 0.75) | 1.010 → 0.990 | 1.051 → 1.029 |
| Furnace-Resin2 U6 (r 1) | 1.059 → 1.023 | 1.107 → 1.060 |
| Furnace-Resin2 U7 (grey r 0.5) | 1.023 → 1.007 | 1.062 → 1.042 |
| Furnace-Resin R7 (coated grey r 0.5) | 1.047 → 1.014 | 1.185 → 1.133 |
| Furnace-Resin3 U5 / U6 (path length 3) | 0.979 / 1.029 → 0.961 / 0.997 | 1.004 / 1.068 → 0.989 / 1.026 |
| Calib-Resin (lit) R7 | 1.160 → 1.133 | 1.451 → 1.416 |

Every other sphere moves < 1 %. U5's middle rings fall to 0.97 – 0.98
(Resin3's to 0.95 – 0.96): the rim solve now pulls the transmission's
falloff harder (r 0.75: grazing 0.59 → 0.40, skew 1.5 → 2.5, k 0.81 →
0.77). Enhanced, architectural glass (transmit base + coat), the Uber glass
(fake shadows) export byte for byte as before.

## Where it stands

Shipped: Faithful resin solves against Octane's excess on the leaving
event. Left: U6's rim 1.06 (the leaving-event model's own 2 – 4 % at the
rim at α 0.707 plus the falloff's shape); R7's coated rim (#211's cut,
unchanged); architectural glass and the Uber glass still on the per-ring
tables (follow-up issue: their exit through a transmit-only lobe and the
fake-shadow walls need their own probes).

## Process lessons

- Pin every sphere's every pin in a probe (brdf, coating, albedo,
  specular): #211's U8 carried the scene's coat and its 1.15 looked like a
  model failure.
- A probe with one face smooth traps light the mesh lets out: subdivide it
  once to see how much of a ring is the mesh.
- Before blaming a direction, split a ring into what the falloff can reach
  (transmission) and what it can't (the entry's reflection): the reflection
  probe (transmission 0) is one sphere.
- When per-event ratios overshoot, ask which events the renderer's light
  sampling can see; the event that leaves the solid was the only one here.
