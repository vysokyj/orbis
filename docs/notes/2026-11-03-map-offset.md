# 2026-11-03 Map offsets (#65)

Calib-Mi-Maps' offset 0.5 is a whole checker period and cannot tell the sign
or scale of an offset. New scene Calib-Map-Offset (make-calib-scenes.py): six
spheres with the checker map: none, U 0.125, V 0.125, V -0.375, tiles 2 x 2
with U and V 0.125, tiles 3 x 2 with V 0.125. Iray Server 5000 samples
(116 s), Octane 400 samples (30 s). Compared as correlation of the
high-passed sphere crops, Iray vs Octane.

| Export | 1 none | 2 U | 3 V | 4 V neg | 5 2x2 UV | 6 3x2 V |
|---|---|---|---|---|---|---|
| translation = (ou, ov), as before | 0.95 | 0.36 | 0.95 | 0.97 | 0.02 | 0.20 |
| (ou/tu, ov/tv) | 0.95 | 0.36 | 0.95 | 0.97 | -0.04 | 0.92 |
| (ou*tu, ov*tv) | 0.95 | 0.36 | 0.95 | 0.97 | 0.02 | 0.12 |
| **(-ou/tu, ov/tv)** | 0.98 | 0.98 | 0.98 | 0.98 | 0.87 | 0.96 |

- Untiled V was already right; tiled V needed the division by the tiles;
  U was mirrored (sphere 2 and 5 stayed low until U was negated).
- Process: the first Octane run failed (no Script menu: locked screen);
  Demo-Gladiator in the Iray queue kept failing and blocked it until it was
  moved out of `active/` and the queue script restarted.
