---
name: docs-first
description: Look things up in the local Octane, NVIDIA Iray/MDL and DAZ docs before guessing or fitting by calibration. Use before tuning a channel, mapping a shader parameter, or explaining Octane/Iray behaviour.
---

# Docs first

Order: 1) `docs/spec/octane-export.md` (what we decided) 2) `docs/daz/` (Iray Uber layer stack: `iray-uber-mdl.md`; `script-api.md`) 3) `docs/octane/` + `docs/octane/manual/` 4) `docs/nvidia/` (Iray, MDL) 5) DAZ's own MDL under `/Applications/DAZ 3D/DAZStudio6 64-bit/DAZStudio.app/Contents/shaders/iray/daz_3d/` (`irayubermaterial.mdl`, `pbr_skin.mdl`).

`docs/octane/manual/` and `docs/nvidia/` are **git-ignored, copyrighted mirrors**: read freely, never copy their text into the repo, never un-ignore. Missing? `python3 scripts/dev/fetch-octane-docs.py docs/octane/manual <cache dir>` / `python3 scripts/dev/fetch-nvidia-docs.py` (needs pypdf).
For wide lookups delegate to the `doc-researcher` agent.
Decided: PBRSkin gets a **direct OpenPBR mapping** (#57) from `pbr_skin.mdl`, no fitted constants; don't resume skin-hue tuning (#19).
