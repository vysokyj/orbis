# Notes — the calibration lab notebook

Dated, detailed notes of calibration sessions: what was tried, in what
order, the numbers, the dead ends and why they were dead, and the process
lessons. **Not normative** — the [spec](../spec/octane-export.md) decides;
a note only records how a rule was found, so it can be re-checked or
re-opened later. Open work stays in
[GitHub issues](https://github.com/vysokyj/orbis/issues).

All ratios are Octane / Iray in linear luminance (sRGB PNG, gamma 2.2,
median over a region) unless a note says otherwise; 1.00 = equal.

| Note | Topics |
|---|---|
| [2026-10-27 – 28](2026-10-27-hair-variants-eyes-thin-walls.md) | strand-hair width and colour, the four variants, the cornea, the caruncle and Base Color Effect 1, Calib-Thin |
| [2026-10-28 skin tones](2026-10-28-skin-tones.md) | Calib-Skin (eight skins), Iray's skin model, Octane's gloss and transfer, Iray Server |
| [2026-10-28 NVIDIA MDL](2026-10-28-nvidia-mdl.md) | NVIDIA's dielectric MDL examples (concrete, porcelain, granite, resin, architectural), the Metal Brushed mismatch |
