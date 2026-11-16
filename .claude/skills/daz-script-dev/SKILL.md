---
name: daz-script-dev
description: Write or debug DAZ Studio scripts (.dsa), the ORBIS DAZ script, and run DAZ headless. Use when touching daz/Scripts/ORBIS or scripts/dev/*.dsa, or when a DAZ run fails.
---

# DAZ Studio scripting

- ECMAScript 3-ish QtScript: `var` only, no arrow functions, no `let`/`const`; string literals **ASCII only**. Statics like `DzDir.homePath()` fail in DAZ Studio 6. API notes: `docs/daz/script-api.md`.
- `daz/Scripts/ORBIS/` is a template; `orbis install` (`scripts/install.sh`) fills in `@...@` and copies it to `<library>/Scripts/ORBIS/`. Script and `orbis` share `format_version` - bump both on an incompatible change.
- Headless run (`-headless -noDefaultScene`, no window; `HEADLESS=0` for a script that needs the GUI): `[DONE_FILE=<file>] scripts/dev/daz-script.sh $PWD/scripts/dev/<x>.dsa <args>... --` (absolute script path; **ask first**; killed when DONE_FILE appears since quitting a big scene takes minutes; script ends with `App.delayedExit()`).
- Whole installed export unattended: `scripts/dev/daz-export-batch.sh <scene.duf> <out dir>`. Dump + export: `scripts/dev/daz-dump.sh <out dir> <scene.duf>...`.
- `customactions.dsx` (the user's custom menu items) is shared by every instance, `-instanceName` or not; `daz-script.sh` puts it back after the run (`KEEP_ACTIONS=0` keeps a change). `removeCustomAction` takes the index, never the GUID (#181).
- Errors: DAZ log `~/Library/Application Support/DAZ 3D/Studio6/log.txt`.
- App Nap off for DAZ (`NSAppSleepDisabled`) or it crawls in background. DAZ and Octane X may both stay open and both work at once (2026-11-16): `daz-script.sh` takes the `daz` lock (one headless DAZ at a time), Octane and local CPU Iray the `render` lock.
- `Scene.saveScene()` headless writes no `scene.current_camera`: Iray Server then renders DAZ's default perspective view, not the scene's camera - set it in the .duf afterwards (#36).
- Scene builders (`daz-build-*.dsa`, `make-*.py`): generated scenes use DAZ built-ins/primitives (or free DAZ add-ons, referenced only); copy into `calib/scenes` only after checking that.
