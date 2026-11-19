# 2026-11-19 Architectural lobe tables re-read on the projected discs

Issue #206 (found in #205: `RESIN_LOBE` was a pre-#146 reading). No render:
the Iray references in git, `scripts/dev/furnace-rings.py measure` (discs
projected from the scene, `sphere_discs.py`, #146) and a small script for
the whole silhouette (r < 0.95) on the same discs.

## Which tables were pre-#146

| Table | Rows | Read | Projected discs (fb8286f, 2026-11-13 21:48) |
|---|---|---|---|
| `ARCH_GLASS_LOBE` | all | 2e784c9, 2026-11-11 | before |
| `ARCH_GLASS_THIN_LOBE` | 0 / 0.2 (H5 / H4) | d6cbc42, 2026-11-13 17:39 | before |
| `ARCH_GLASS_THIN_LOBE` | 0.4 / 0.6 / 0.8 (J4 – J2) | 7231f9b, 2026-11-14 | after |

## Re-read

Closed lobe, disk r < 0.75, white, reflectivity 0:

| refr_gloss | old | G (ArchGlass) | H (ArchGlass2, same material) | new |
|---|---|---|---|---|
| 0 | 0.512 | 0.5078 | 0.5080 (H3) | 0.508 |
| 0.2 | 0.614 | 0.6092 | 0.6094 (H2) | 0.609 |
| 0.4 | 0.938 | 0.9364 | 0.9363 (J6, ArchGlass3) | 0.936 |
| 0.6 | 0.964 | 0.9636 | 0.9637 (H1) | 0.964 |
| 1 | 1.000 | 1.0000 | — | 1.000 |

Two frames agree to 0.0002, so the shift (0.8 % at 0 and 0.2) is the disc
placement, not noise. With `legacy_loss` drawn per event (#191) the lobe
alone needs τ 1.008 / 1.000 / 0.998 at refr_gloss 0 / 0.2 / 0.4 (old
reading: 1.012 / 1.004 / 0.999), so the model now reproduces the measured
lobe nearly by itself.

Thin wall, whole silhouette r < 0.95 (disk r < 0.75):

| refr_gloss | table | r < 0.95 | disk | kept |
|---|---|---|---|---|
| 0 (H5) | 0.386 | 0.389 | 0.387 | yes: one 8-bit level, 0.386 in every ring to 0.8; the 0.389 is only the edge pixels past 0.9 |
| 0.2 (H4) | 0.505 | 0.503 | 0.506 | yes: within 0.4 % (J5, another frame, 0.513: the "frame tie" of #149) |
| 0.4 / 0.6 / 0.8 | 0.915 / 0.960 / 0.965 | 0.915 / 0.960 / 0.965 | | already projected |

## Effect

Exported head-on k (Smith model / with Octane's excess, white):
G5 0.833 → 0.830 / 0.813 → 0.810, G4 0.872 → 0.868 / 0.849 → 0.846,
G3 0.976 → 0.975, G7 (Schlick) 0.871 → 0.868, G8 (Fresnel) 0.663 → 0.661;
grazing and skew move by < 0.01 (≤ 0.4 % on k, below a render check).

Coated spheres (refr_gloss 0.2, reflectivity 0.6), Iray model v Iray
disk: Schlick G7 0.648 v 0.648, Fresnel term G8 0.399 v 0.412 (3 % low).
The spec's earlier "0.657 v 0.655, 0.421 v 0.416" was from the model
before the per-event `legacy_loss`; G8's 3 % is left as it is (its
Octane side was not part of this issue).

## Coordination

#205's prepared patch (`c205-export.patch`: `RESIN_LOBE`, `legacy_loss` on
the coat, Octane's falloff curve) touches other lines and still applies
(`git apply --check`) on top. Its "before" renders of the ArchGlass scenes
were taken before this change, so its scene check sees both.

## Lesson

Every measured table read before fb8286f is suspect by up to ~1 %:
`git log -S` on the table against fb8286f's date settles which rows.
