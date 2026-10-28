# ORBIS documentation

Start with the project [README](../README.md) (what ORBIS is, install, use).
Then, depending on what you need:

| Document | What it covers | Read it when |
|---|---|---|
| [spec/octane-export.md](spec/octane-export.md) | **Normative.** What the export does with every part of a DAZ scene, and why — with the calibration numbers behind each rule. | You change the export, or wonder why something looks the way it does in Octane. |
| [calibration.md](calibration.md) | How the export is checked against Iray: the principle, the calibration scenes and what each measures, the scripts, the references. | You calibrate a feature, add a test scene or re-check a rule. |
| [notes/](notes/README.md) | Dated lab notebook of calibration sessions: every attempt, the numbers, dead ends, process lessons. Not normative. | You re-open a rule or wonder how it was found. |
| [daz/iray.md](daz/iray.md) | Iray in DAZ Studio: the Uber material's channels, lights, environment, tone mapping. | You need to know what a DAZ channel means. |
| [daz/files.md](daz/files.md) | `.duf` / `.dsf`, how DAZ Studio builds what it renders (subdivision, grafts, shells), headless DAZ Studio. | You read scene files or the scene dump. |
| [daz/content.md](daz/content.md) | Content library layout, content types, figure compatibility. | You resolve assets or metadata. |
| [daz/script-api.md](daz/script-api.md) | The DAZ Script API ORBIS uses (dialogs, geometry, processes). | You change the DAZ Studio script. |
| [octane/octane-x.md](octane/octane-x.md) | Octane X facts: USD import, `project.ocs` rules, node types and pins, kernels, Lua scripting. | You write Octane nodes or a probe. |
| [octane/orbx.md](octane/orbx.md) | The ORBX container (reverse-engineered). | You touch the package writer. |
| [octane/usd.md](octane/usd.md) | The USD subset the export writes. | You touch the USD writer. |
| `octane/manual/` | Local mirror of OTOY's manual (git-ignored, copyrighted). | Before guessing at Octane behaviour. |

The spec decides; `daz/` and `octane/` only describe the other tools. Plans
and open work are [GitHub issues](https://github.com/vysokyj/orbis/issues),
not documents.
