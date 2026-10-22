# DAZ content — library layout, content types, figure compatibility

Reference for the DAZ ecosystem: how a content library is organised and where
a package says what it contains (content type) and which figure it fits.
Frequencies are from a real library (334 DIM manifests, 2026).

Two signal sources, by authority:

1. **`Runtime/Support/*.dsx` (`<ContentDBInstall>`)** — DAZ's own per-asset
   metadata. Authoritative when present.
2. **Content paths** — convention-based folder names; the `data/` tree is
   path-locked and reliable, user-facing folders are not.

## Library layout — three folder classes

| Folder | Role | Reorganizable |
|---|---|---|
| `data/` | geometry, rig, UVs, **morphs** (`.dsf`) — referenced by URL, never user-loaded | **Never** (path-locked) |
| `Runtime/` | Poser files + **all textures** (`Runtime/Textures/…`) + `Runtime/Support/` CMS metadata | No |
| `People`, `Props`, `Environments`, `Poses`, `Materials`, `Light Presets`, … | user-facing `.duf` presets/loaders | Yes (convention only) |

`data/DAZ 3D/Genesis 8/Female/Genesis8Female.dsf` is the **support asset**
(mesh/rig); `People/Genesis 8 Female/….duf` is a **loader** that only
*references* it.

Real distribution:

- **Generation** (`People/Genesis N …`): G8 Female 22.7k, G9 7.0k, G8 Male 6.4k,
  G3 Female 1.9k, G8.1 Female 0.6k files.
- **Content-type folder** (`People/Genesis N/<Type>/…`): Poses 23.8k, Hair 9.8k,
  Characters 2.8k, Expressions 2.4k, Clothing 2.3k, Anatomy 2.2k, Materials 1.0k,
  Props 710, Accessories 536.
- **Geometry root** (`data/DAZ 3D/<figure>/…`): Genesis 8, 9, 3, plus vendor
  folders encoding the generation (`data/goldtassel/GN8GeeGee/…` → "GN8").

## `Runtime/Support/*.dsx` — per-asset metadata

One `<ContentDBInstall>` per product, shipped inside the install zip and
listed in its `Manifest.dsx`:

```xml
<ContentDBInstall VERSION="1.0">
 <Products>
  <Product VALUE="Z Fitness Instructor Couple Poses for Genesis 9 and 8">
   <StoreID VALUE="DAZ 3D"/>
   <GlobalID VALUE="f162c803-…"/>
   <ProductToken VALUE="101559"/>
   <Artists><Artist VALUE="Zeddicuss"/></Artists>
   <Assets>
    <Asset VALUE="People/Genesis 9/Poses/…/Z FITIN 00 Zero G9F.duf">
     <ContentType VALUE="Preset/Pose"/>
     <Categories><Category VALUE="/Default/Poses/By Region/Full Body"/></Categories>
     <Compatibility VALUE="/Genesis 9/Base"/>
    </Asset>
    <Asset VALUE="People/Genesis 8 Female/Poses/…/Z FITIN !Zero G8F.duf">
     <ContentType VALUE="Preset/Pose"/>
     <Compatibility VALUE="/Genesis 8/Female"/>
     <Compatibility VALUE="/Genesis 8.1/Female"/>
```

Real `ContentType` frequencies: `Preset/Pose` 20k, `Preset/Materials/MDL` 5.2k,
`Preset/Pose/Hierarchical`, `Prop` 589, `Preset/Morph/Apply`, `Follower/Hair`
186, `Follower/Attachment`, `Actor/Character` 57, `Preset/Wearables`,
`Preset/Light`, `Scene`.

Real `Compatibility` bases: `/Genesis 8` 13.2k, `/Genesis 8.1` 3.6k,
`/Genesis 9` 1.9k, `/Genesis 3` 278, plus per-product add-on bases.

Third-party content often has no such file at all.

## Figure generation from paths

Most reliable = the `data/` segment:

```
data/DAZ 3D/Genesis 8/Female/Genesis8Female.dsf
data/DAZ 3D/Genesis 8/Female 8_1/Genesis8_1Female.dsf   ← 8.1 uses "Female 8_1"
data/DAZ 3D/Genesis 9/Genesis9.dsf                      ← G9 gender-neutral, no Female/Male
data/DAZ 3D/Genesis 3/Female/Genesis3Female.dsf
```

Definitive marker = the referenced support-asset URL fragment in presets and
morphs: `…/Genesis8Female.dsf#Genesis8Female`, `#Genesis9`,
`#Genesis8_1Female`, `#Genesis3Female`.

Weaker hints: product names ("… for Genesis 8 Female"), manifest file names
(`…Genesis8…`), scene node ids (`Genesis8Male`).

## Content type — file, folder, DSON fields

- Extensions: `.duf` = **DSON User File** (user-facing preset/scene);
  `.dsf` = **DSON Support File** (`data/` asset, referenced by URL);
  `.dhdm` = HD morph delta; `.dsx` = XML (DIM/CMS); `.dse` = encrypted script.
  File formats: [files.md](files.md).
- `asset_info.type` = load hint (the spec defines the field and one example,
  `preset_pose`). Real values: `figure`, `modifier`, `character` /
  `preset_character`, `wearable`, `preset_pose`, `preset_material`,
  `preset_hierarchical_material`, `preset_shape` / `preset_shaping`,
  `preset_light`, `scene`, `scene_subset`. **Hand-editable → a hint, not a
  guarantee.**
- The `content_type` taxonomy (CMS / Support `.dsx`) — top branches: `Actor`
  (`Actor/Character`), `Follower` (`Follower/Hair`, `Follower/Wardrobe/*`,
  `Follower/Attachment`), `Modifier` (`Modifier/Shape`…), `Preset`
  (`Preset/Pose`, `Preset/Materials`, `Preset/Wearables`…), `Prop`, `Light`,
  `Camera`, `Scene`, `Script`, `Set`, `Support`. **The cleanest classifier.**

## Fit target — which figure a wearable conforms to

Readable from the JSON, strongest first:

- `conform_target` on the node instance → URL of the target figure node
  (`…Genesis8Female.dsf#Genesis8Female`).
- Auto-follow morphs → `modifier_library` formulas reference the target
  figure's channels by URL.
- Geograft → `geometry_library[].root_region.graft` + `conform_target`.
- Wearable presets (`asset_info.type: wearable`) hold no geometry — follow the
  referenced base `.dsf` URLs.

## Morphs in `data/…/Morphs/…`

Each morph `.dsf`: `modifier_library[].parent` =
`/data/DAZ 3D/Genesis 8/Female/Genesis8Female.dsf#Genesis8Female` — the
authoritative target figure. `presentation.type` = `Modifier/Shape` etc.

## DIM zip internals

`Manifest.dsx` (`<DAZInstallManifest>`) = file list (`<File VALUE="Content/…">`)
+ `GlobalID`. `Supplement.dsx` (`<ProductSupplement>`) = `ProductName`,
`InstallTypes`, `ProductTags` (software compatibility like `DAZStudio4` —
**not** the figure generation). **No per-file content type or figure in the
manifest** — that lives only in `Runtime/Support/*.dsx` and the
`.duf` / `.dsf` files.

## Sources

- DSON spec: <https://docs.daz3d.com/public/dson_spec/> — `asset_info`,
  `object_definitions`, `content_types`, `file_types`.
- File extensions: <http://docs.daz3d.com/doku.php/public/software/dazstudio/4/referenceguide/tech_articles/file_extensions/start>
- Content Types taxonomy: <http://docs.daz3d.com/doku.php/public/dson_spec/format_description/metadata/content_types/start>
- Conforming internals: <https://forum.daz3d.com/forums/discussion/72837/innards-of-conforming>
- Wearables presets: <https://www.deviantart.com/sickleyield/journal/Tutorial-Clarification-Wearables-Presets-417253297>
- DSX/DIM/CMS: <https://www.daz3d.com/forums/discussion/179136/>
