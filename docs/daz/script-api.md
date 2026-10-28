# DAZ Script API — what ORBIS uses

DAZ Studio scripts (`.dsa`) are QtScript (ECMAScript 3 style: `var`, no
arrow functions, `let`/`const`). Official reference:
<https://docs.daz3d.com/public/software/dazstudio/4/referenceguide/scripting/api_reference/object_index/start>.
Machine-readable signatures: the npm package
[`dazscript-types`](https://www.npmjs.com/package/dazscript-types)
(`npm pack dazscript-types`, `src/types/daz/dz_<class>.d.ts`, MPL-2.0) —
generated from the docs, so undocumented members may be missing.

## Globals

- `getScriptFileName()` — the running script's path (also when run as a
  custom action).
- `startProgress(info, totalSteps, cancellable, showTime)`,
  `stepProgress(n)`, `progressIsCancelled()`, `setProgressInfo(text)`,
  `finishProgress()`; `processEvents()`, `sleep(ms)`, `print(text)`.
- `connect(sender, "signal(args)", function)` — e.g.
  `connect(button, "clicked()", fn)`, `connect(radio, "toggled(bool)", fn)`.
- `MessageBox.information|critical(text, title, button)`.
- `FileDialog.doDirectoryDialog(title, desc, startWith, parent)` → path or
  `""` when cancelled; `doFileDialog(open, title, startWith, filter)`.

## App, scene, content

- `App.longVersionString` (`major.minor.revision.build`), `App.getTempPath()`,
  `App.getContentMgr()`.
- `DzDir.homePath()` is documented (static, 4.9.3.35+) but fails in DAZ
  Studio 6 ("Property 'homePath' … is not a function") — don't count on
  `DzDir` statics.
- `DzContentMgr`: `getNumContentDirectories()` /
  `getContentDirectoryPath(i)`, `getNumBuiltInMDLDirectories()` /
  `getBuiltInMDLDirectoryPath(i)` (the app's `shaders/iray`, root of
  `/resources/…` paths).
- `Scene.getFilename()` (`""` when never saved), `Scene.needsSave()`,
  `Scene.getNumNodes()` / `getNode(i)`.
- `Scene.saveScene(filename)` saves there without asking (and makes it the
  scene's file); `Scene.saveScene()` without a name opens the Save As
  dialog. In DAZ Studio 6 `findAction("DzSaveSceneAction").trigger()` (File >
  Save) asks for a name too, and runs only after the script has returned.
- `Scene.needsSave()` can be true right after opening a scene: UltraScatter
  rebuilds its instance groups on load.
- Rendering from a script (`scripts/dev/daz-spot-variants.dsa`, works in
  DS6): `App.getRenderMgr()`, `getRenderOptions()`, `renderImgToId =
  DzRenderOptions.DirectToFile`, `renderImgFilename`, `doRender(options)`
  (blocks until done).

## Evaluated geometry

Per node (`DzNode`; skip `inherits("DzBone")`): `name`, `getLabel()` (unique
per scene), `className()`, `finalize()`, `getObject()` (`DzObject`, null for
nodes without geometry).

- `DzObject.getCurrentShape()` → `DzShape` (a `DzFacetShape` for meshes);
  `getLODControl()` → the *Resolution Level* enum property (0 = base, 1 =
  high resolution / SubD).
- `DzObject.getCachedGeom()` → the evaluated mesh (`DzFacetMesh`): world
  space, cm, Y up, every morph and the pose applied. At resolution level 0 it
  is the base mesh, vertex-for-vertex the `.dsf` geometry; set the level with
  `setValue(0)`, then `forceCacheUpdate(node, false)` before reading, and
  restore the level afterwards. A geometry shell (`DzGeometryShellNode`)
  copies its host's cache — switch the host too, before the shell's update.
- `getNumVertices()`, `getVertex(i)` → `DzVec3` (`x`, `y`, `z`);
  `getNumMaterialGroups()`, `getMaterialGroup(i).name`; `getNumFacets()`.
- Strand hair (dForce, no facets): `getNumPolylines()`,
  `getPolylineVertexIndices(i)` → Array, `getPolylineMaterialGroupIndex(i)`.
  These are the rendered strands; the library mesh is only their guide.

## Files and processes

- `DzFile(path)`: `open(DzFile.WriteOnly | DzFile.Truncate)`, `write(str)`,
  `close()`. Keep the text ASCII (escape the rest) to stay out of encoding
  questions.
- `DzFileInfo(path)`: `path()` (directory), `exists()`.
- `DzProcess(args)`: `communication = DzProcess.Stdout | DzProcess.Stderr`,
  `start()`, `running` (property; older builds `isRunning()`), `canReadLineStdout()` /
  `readLineStdout()`, `readStderr()`, `normalExit`, `exitStatus`;
  `writeToStdin(str)` / `closeStdin()` (#2).

## Dialogs and settings

- `DzBasicDialog`: `caption`, `addWidget(w)`, `setAcceptButtonText(text)`,
  `exec()` → true on accept. `getWidget().objectName = "<unique>"` makes DAZ
  remember the dialog's size and position.
- Widgets take their parent in the constructor. `DzVGroupBox` /
  `DzVButtonGroup` (`title`) squeeze their contents into scroll areas in DAZ
  Studio 6 — use a `DzLabel` heading (`text` takes rich text, `<b>…</b>`)
  and a `DzWidget` + `DzVBoxLayout` instead; radio buttons sharing a parent
  are exclusive. Size a `DzBasicDialog` to its contents with
  `setFixedSize(getWidget().minimumSizeHint.width, ….height)` before
  `exec()`. Also
  `DzRadioButton`, `DzCheckBox` (`text`, `checked`), `DzLineEdit` (`text`),
  `DzPushButton`, `DzWidget` + `DzHBoxLayout(widget)` (`addWidget(w,
  stretch)`, `margin`) for a row. `enabled` on any widget.
- `DzAppSettings(path)`: `getBoolValue(key, def)`, `getStringValue(key,
  def)`, `setBoolValue`, `setStringValue` — persisted in DAZ Studio's
  preferences (plist on macOS).

## Menu actions

- `MainWindow.getActionMgr()` (null headless): `getNumCustomActions()`,
  `getCustomActionText(i)`, `getCustomActionFile(i)`,
  `addCustomAction(text, desc, scriptPath, isFile, shortcut, iconPath)` →
  GUID, `setCustomActionScript(i, path, isFile)`, `setCustomActionIcon(i,
  path)`, `getCustomActionName(i)` → its GUID. Custom actions persist in
  `customactions.dsx`, the menus that show them in `menus.dsx` — apart: a
  menu reset or edit drops the item and keeps the action (#46).
- `getMenu().findSubMenu("&File")` → `DzActionMenu`: `getNumItems()`,
  `getItem(i).label` (e.g. `"&Export..."`), `getItem(i).action` (the class,
  e.g. `DzExportAction`, or a custom action's GUID), `insertCustomAction(guid,
  index)`.
- A second launch of DAZ Studio with a script, even under the same
  `-instanceName`, starts its own instance; it doesn't hand the script to the
  running one.

## Content Library thumbnails

A `<name>.png` beside a file or folder is its thumbnail in the Content
Library (91×91 is what DAZ's own scripts ship); `Scripts/ORBIS.png` for the
folder, `Scripts/ORBIS/Export to Octane.png` for the script.
