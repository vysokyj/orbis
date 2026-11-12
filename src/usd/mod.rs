//! Native DAZ scene → USDZ export for Octane X.
//!
//! A saved scene is rebuilt from three sources, without Blender: the `.duf`
//! (nodes, geometry/UV/material references), the library `.dsf` assets
//! (topology, UV sets, grafts) and the scene dump DAZ Studio writes through
//! the ORBIS script (final world-space vertices per node, morphs and pose
//! applied). See `docs/spec/octane-export.md` for the formats.
//!
//! Phase 1: geometry — one `Mesh` per node with material-group `GeomSubset`s
//! and indexed `faceVarying` UVs, packed into a `.usdz` — and, for Octane X,
//! into an `.orbx` (Octane's package, see `crate::orbx`) whose `project.ocs`
//! imports it with the scene's camera.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;

use crate::dson::{decompress_scene, percent_decode};

mod camera;
mod dump;
mod environment;
mod instance;
mod lens;
mod library;
mod lights;
mod mesh;
mod project;
mod render_settings;
mod subdivide;
mod usda;
mod usdc;
mod usdz;
mod volume;
#[cfg(test)]
mod tests;

pub use camera::*;
pub use volume::Volume;
use dump::*;
pub use dump::{dump_cache_path, store_dump, DUMP_FORMAT_VERSION};
pub use environment::*;
use instance::*;
use library::*;
pub use lights::*;
use mesh::*;
use project::*;
use subdivide::*;
pub use usda::*;
pub use usdc::Patch;
use usdc::UsdcWriter;
pub use usdz::*;
pub use crate::tuning::{Look, OctaneMode, Variant};

/// Where to look for library assets.
#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    /// DAZ content directories; the dump's own `content_dirs` are searched
    /// after these. With neither, DAZ Studio's preferences (`daz::content_dirs`).
    pub content_dirs: Vec<PathBuf>,
    /// Octane render setup: fast preview or the photoreal path tracer.
    pub octane_mode: OctaneMode,
    /// Leave out the shadow-catching ground even when Iray draws one — it
    /// shows through holes or dips in a floor or terrain at its height.
    pub no_ground: bool,
    /// Iray's look as closely as the export can, or Octane's more plausible
    /// take where the two differ (`Look`).
    pub look: Look,
    /// Leave Octane's AI denoiser off (Final turns it on).
    pub no_denoise: bool,
    /// Told how far the export got.
    pub progress: Progress,
}

impl ExportOptions {
    /// Which of the four variants this export is (`crate::tuning`).
    pub fn variant(&self) -> Variant {
        Variant::new(self.look, self.octane_mode)
    }
}

/// Export progress: a callback taking the percentage done (0–100) and what
/// the export is doing; does nothing by default.
#[derive(Clone, Default)]
pub struct Progress(Option<std::sync::Arc<dyn Fn(u32, &str) + Send + Sync>>);

impl Progress {
    pub fn new(f: impl Fn(u32, &str) + Send + Sync + 'static) -> Progress {
        Progress(Some(std::sync::Arc::new(f)))
    }

    pub fn report(&self, percent: u32, what: &str) {
        if let Some(f) = &self.0 {
            f(percent.min(100), what);
        }
    }
}

impl std::fmt::Debug for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() { "Progress(..)" } else { "Progress(none)" })
    }
}

/// What [`export_scene_usdz`] wrote.
#[derive(Debug, Clone, Default)]
pub struct ExportReport {
    pub meshes: usize,
    pub points: usize,
    pub faces: usize,
    /// Copies drawn through instancing (instance nodes, UltraScatter).
    pub instances: usize,
    /// Nodes left out, with the reason (`label: why`).
    pub skipped: Vec<String>,
    /// Textures packed into an `.orbx`.
    pub textures: usize,
    /// Material textures not found in the content directories.
    pub missing_textures: Vec<String>,
    /// Longest texture edge when the decoded textures exceeded
    /// `texture_budget::BUDGET_BYTES` (`None`: full size) and how many were
    /// scaled down.
    pub texture_limit: Option<u32>,
    pub textures_scaled: usize,
    /// Octane render targets written, one per DAZ camera.
    pub render_targets: usize,
}

/// Convert `scene` (the saved `.duf`) with its `dump` (written by DAZ Studio)
/// into a `.usdz` at `out` — or, named `.usda`, the scene's layer as text
/// (to read; an export packs it binary).
pub fn export_scene_usdz(scene: &Path, dump: &Path, out: &Path, opts: &ExportOptions) -> Result<ExportReport> {
    let (meshes, volumes, mut report, mut lib) = build_scene(scene, dump, opts)?;
    // The .usdz carries meshes only; the volumes go into project.ocs.
    report.skipped.extend(volumes.iter().map(|v| format!("{}: a VDB volume (only in .orbx)", v.label)));
    let lights = lights_iray_uses(&mut lib);
    if out.extension().is_some_and(|e| e.eq_ignore_ascii_case("usda")) {
        let (text, _) = write_usda(&meshes, &lights);
        write_atomically(out, |f| Ok(f.write_all(text.as_bytes())?))?;
        return Ok(report);
    }
    let mut layer = std::io::Cursor::new(Vec::new());
    let mut w = UsdcWriter::new(&mut layer)?;
    write_layer(&mut w, meshes, &lights)?;
    w.finish()?;
    write_atomically(out, |f| write_usdz(f, &[(LAYER, layer.get_ref())]))?;
    Ok(report)
}

/// The USD graph's object layer import in `project.ocs`: none (the
/// template's), or one input per prim for the export to fill.
const OBJECT_LAYERS_HIDDEN: &str = "<attr name='objectLayerImport' type='2'>1</attr>";
const OBJECT_LAYERS_ALL: &str = "<attr name='objectLayerImport' type='2'>3</attr>";

/// Convert `scene` into an Octane package (`.orbx`) at `out`: the scene as an
/// embedded binary USD layer (`.usdc`), imported by a USD graph in `project.ocs`, with a render
/// target wired to it and the scene's DAZ camera. Octane X opens it from its
/// own Open panel or gallery (it refuses files handed over from outside).
pub fn export_scene_orbx(scene: &Path, dump: &Path, out: &Path, opts: &ExportOptions) -> Result<ExportReport> {
    let progress = &opts.progress;
    let (meshes, volumes, mut report, mut lib) = build_scene(scene, dump, opts)?;
    progress.report(35, "writing the USD scene");
    let mut lights = lights_iray_uses(&mut lib);
    let mut cameras = scene_cameras(&mut lib);
    if cameras.is_empty() {
        tracing::info!("USD export: no camera in the scene, framing all meshes");
        cameras.push(("Camera".to_string(), framing_camera(&meshes)));
    }
    // The camera's headlamp, when Iray lights with it (the render camera's).
    let has_lights = !lights.is_empty();
    if let Some(h) = cameras.first().and_then(|(label, cam)| headlamp(&mut lib, label, cam, has_lights)) {
        lights.push(h);
    }
    // What needs the meshes, before they go into the layer one by one: a
    // big scene's (60 M points) held beside a text copy of itself peaked
    // at 23 GB.
    let uv = uv_scales(&meshes);
    let mut environment = scene_environment(&mut lib);
    let ground_y = environment.as_ref().map_or(0.0, |e| ground_height(e, &meshes));
    let name = scene.file_stem().and_then(|s| s.to_str()).unwrap_or("scene");
    let mut scaled = 0;
    write_atomically(out, |f| {
        let mut w = crate::orbx::OrbxWriter::new(f, OCTANE_VERSION)?;
        // The scene: the binary layer, streamed into the package.
        w.begin(&format!("assets/{LAYER}"))?;
        let mut usdc = UsdcWriter::new(&mut w)?;
        let materials = write_layer(&mut usdc, meshes, &lights)?;
        usdc.finish()?;
        progress.report(60, "translating the materials");
        let layered = crate::daz_material::layered_images(&lib.duf);
        let tuning = opts.variant().tuning();
        let (inputs, mut textures) = crate::material::material_inputs_scaled(&materials, &uv, &lib.dirs, layered, 200_000, &tuning);
        if let Some(s) = environment.as_mut().and_then(|e| e.sun_sky.as_mut()) {
            s.tune(&tuning);
        }
        let env_texture = crate::material::environment_texture(environment.as_ref(), &lib.dirs, &mut textures, 190_000);
        let env_power = environment.as_ref().map(|e| if e.scene_only { 0.0 } else { e.intensity }).unwrap_or(1.0);
        let emitters = light_emitters(&lights, &mut |p| textures.ies(&lib.dirs, p));
        let light_inputs = crate::material::light_inputs(&emitters, 180_000);
        // The shadow-catching ground stands in for Iray's ground under an HDRI
        // backdrop; without a map there is nothing for its shadow to sit on.
        let ground = environment.as_ref().filter(|e| e.map.is_some() && !opts.no_ground).and_then(|e| Some((e.ground_shadow?, ground_y)));
        let layers: Vec<(String, &str, bool)> = camera_shown_lights(&lights).into_iter().map(|p| (p, "camera_visibility", true)).collect();
        let iray_ground = ground_y;
        // VDB volumes, each file packed once.
        let mut vdb_files: Vec<(String, PathBuf)> = Vec::new();
        let mut octane_volumes: Vec<_> = volumes
            .iter()
            .map(|v| {
                let asset = match vdb_files.iter().find(|(_, f)| *f == v.file) {
                    Some((a, _)) => a.clone(),
                    None => {
                        let base = v.file.file_name().and_then(|n| n.to_str()).unwrap_or("volume.vdb");
                        let safe: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
                        let a = format!("assets/vdb/{:02}_{safe}", vdb_files.len());
                        vdb_files.push((a.clone(), v.file.clone()));
                        a
                    }
                };
                // VDB units are DAZ's cm: the node's transform, in meters.
                crate::material::OctaneVolume { name: v.label.clone(), asset: Some(asset), grid: v.grid.clone(), transform: v.transform.map(|x| x * f64::from(CM)), scattering: v.scattering, absorption: v.absorption, anisotropy: v.anisotropy }
            })
            .collect();
        // Iray's ground fog: slabs of uniform fog around the render camera.
        let centre = cameras.first().map_or([0.0; 3], |(_, c)| c.pos);
        if let Some(fog) = environment.as_ref().filter(|e| !e.scene_only).and_then(|e| e.ground_fog.as_ref()) {
            for (k, s) in ground_fog_slabs(fog).iter().enumerate() {
                let (min, max) = ([centre[0] - FOG_RADIUS, s.y0, centre[2] - FOG_RADIUS], [centre[0] + FOG_RADIUS, s.y1, centre[2] + FOG_RADIUS]);
                octane_volumes.push(crate::material::OctaneVolume {
                    name: format!("DAZ ground fog {k}"),
                    asset: None,
                    grid: String::new(),
                    transform: crate::material::unit_box(min, max),
                    scattering: [0, 1, 2].map(|c| (fog.albedo[c] * s.extinction[c]) as f32),
                    absorption: [0, 1, 2].map(|c| ((1.0 - fog.albedo[c]) * s.extinction[c]) as f32),
                    anisotropy: fog.anisotropy as f32,
                });
            }
        }
        // Iray's dome ground: the HDRI projected from its virtual camera.
        let dome_ground = environment.as_ref().filter(|e| !e.scene_only && e.draw_dome).and_then(|e| {
            let height = iray_ground;
            let texture = crate::material::projected_environment_texture(e, &lib.dirs, &mut textures, 175_000, [0.0, height + e.dome_ground?, 0.0])?;
            Some(crate::material::DomeGround { texture, height, power: e.intensity })
        });
        let (scene_nodes, mesh_connect) = crate::material::scene_lights(&octane_spots(&lights), ground, dome_ground.as_ref(), &octane_volumes, 170_000, 1439);
        let tone = scene_tone_mapping(&mut lib);
        // Iray lights with a hidden dome (Draw Dome off) but shows black there;
        // "Scene Only" has no dome light to begin with (power 0, black anyway).
        let hide_dome = environment.as_ref().is_some_and(|e| !e.draw_dome && !e.scene_only);
        let ocs = project_ocs(name, LAYER, &cameras, render_size(&lib.duf), &env_texture, &tuning, &tone, hide_dome, environment.as_ref().and_then(|e| e.sun_sky.as_ref()), !opts.no_denoise, &crate::iray::render_options(&render_channels(&lib.duf)))
            .replace(OBJECT_LAYERS_HIDDEN, if layers.is_empty() { OBJECT_LAYERS_HIDDEN } else { OBJECT_LAYERS_ALL })
            .replace("@MATERIAL_INPUTS@", &(inputs + &light_inputs + &crate::material::object_layer_inputs(&layers, 160_000)))
            .replace("@SCENE_NODES@", &scene_nodes)
            .replace("@MESH_CONNECT@", &mesh_connect.to_string())
            .replace("@ENV_POWER@", &format!("{env_power}"));
        report.render_targets = cameras.len();
        // Maps a material only read to bake another (a height map baked into
        // a normal map, #85) stay out of the package and the texture budget.
        let used = |a: &str| !a.starts_with("assets/tex/") || ocs.contains(a);
        textures.files.retain(|(a, _)| used(a));
        textures.generated.retain(|(a, _)| used(a));
        report.textures = textures.files.len() + textures.generated.len();
        report.missing_textures = textures.missing.clone();
        for t in &textures.missing {
            tracing::info!(texture = %t, "Octane export: texture not found");
        }
        // Octane holds textures decoded; scale them down when they'd crowd out
        // the GPU's (unified) memory. Only their headers are read here; each is
        // read (and scaled) as it goes into the package.
        let dims: Vec<_> = textures
            .files
            .iter()
            .filter_map(|(_, f)| crate::texture_budget::file_dimensions(f))
            .chain(textures.generated.iter().filter_map(|(_, d)| crate::texture_budget::dimensions(d)))
            .collect();
        report.texture_limit = crate::texture_budget::choose_limit(&dims, crate::texture_budget::BUDGET_BYTES);
        if let Some(limit) = report.texture_limit {
            tracing::info!(limit, "Octane export: textures scaled down to fit the memory budget");
        }
        let n_files = textures.files.len().max(1);
        progress.report(65, "writing the .orbx");
        // Read and scaled a batch at a time across the cores (decoding and
        // resizing 4K maps one by one took 74 s on a multi-figure scene,
        // #78), written in order.
        let limit = report.texture_limit;
        let batch = std::thread::available_parallelism().map_or(4, |n| n.get()) * 2;
        let mut done = 0;
        for part in textures.files.chunks(batch) {
            let loaded = crate::texture_budget::par_map(part, |(_, file)| {
                let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
                Ok::<_, anyhow::Error>(match limit.map(|l| crate::texture_budget::downscale(&data, l)) {
                    Some(Ok(Some(small))) => (small, true),
                    _ => (data, false),
                })
            });
            for ((asset, _), r) in part.iter().zip(loaded) {
                progress.report(65 + (30 * done / n_files) as u32, &format!("writing textures ({}/{})", done + 1, textures.files.len()));
                done += 1;
                let (data, small) = r?;
                scaled += usize::from(small);
                w.add(asset, &data)?;
            }
        }
        for part in textures.generated.chunks(batch) {
            let small = crate::texture_budget::par_map(part, |(_, data)| match limit.map(|l| crate::texture_budget::downscale(data, l)) {
                Some(Ok(Some(small))) => Some(small),
                _ => None,
            });
            for ((asset, data), small) in part.iter().zip(small) {
                scaled += usize::from(small.is_some());
                w.add(asset, small.as_deref().unwrap_or(data))?;
            }
        }
        for (asset, file) in &vdb_files {
            progress.report(95, "writing the VDB volumes");
            w.begin(asset)?;
            std::io::copy(&mut std::fs::File::open(file).with_context(|| format!("reading {}", file.display()))?, &mut w)?;
        }
        // Iray's tone curve, which the imager's LUT node names (project.ocs).
        if !tone.is_plain() {
            w.add(TONE_LUT_ASSET, tone.cube_lut().as_bytes())?;
        }
        w.add("project.ocs", ocs.as_bytes())?;
        w.finish()
    })?;
    report.textures_scaled = scaled;
    progress.report(100, "done");
    Ok(report)
}

/// The scene's lights, unless its environment mode leaves them out ("Dome
/// Only", "Sun-Sky Only").
fn lights_iray_uses(lib: &mut Library) -> Vec<SceneLight> {
    if scene_environment(lib).is_some_and(|e| !e.scene_lights) {
        tracing::info!("USD export: the environment mode ignores the scene's lights");
        return Vec::new();
    }
    scene_lights(lib)
}

/// The scene layer's file name inside an `.orbx` (under `assets/`) or a
/// `.usdz`. Binary: Octane X imports it as it does text (probed on
/// Calib-Hair, #16), a third of the size and written as it goes.
const LAYER: &str = "scene.usdc";

/// Octane version the `project.ocs` template was saved with (Octane X 2026.6).
const OCTANE_VERSION: &str = "15050000";

/// Read the scene, its dump and library assets into meshes.
fn build_scene(scene: &Path, dump: &Path, opts: &ExportOptions) -> Result<(Vec<Mesh>, Vec<Volume>, ExportReport, Library)> {
    let duf: Value = serde_json::from_str(&decompress_scene(
        &std::fs::read(scene).with_context(|| format!("reading {}", scene.display()))?,
    )?)?;
    opts.progress.report(0, "reading the scene");
    let dump = SceneDump::load(dump).with_context(|| format!("reading the scene dump {}", dump.display()))?;

    let mut dirs = opts.content_dirs.clone();
    if dirs.is_empty() && dump.content_dirs.is_empty() {
        dirs = crate::daz::content_dirs();
    }
    // The installed DAZ Studio's Iray shader folder serves `/resources/…`
    // (default HDRI, brushed-metal maps) even when the dump names an older
    // install.
    let installed_iray = crate::daz::studio_app().map(|a| a.join("Contents/shaders/iray"));
    let extra = dump.content_dirs.iter().chain(&dump.builtin_mdl_dirs).map(PathBuf::from).chain(installed_iray);
    for d in extra {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    let mut lib = Library::new(dirs, duf);
    lib.line_width_scale = opts.variant().tuning().line_width_scale;
    opts.progress.report(5, "building the meshes");
    let (meshes, mut report) = build_meshes(&mut lib, &dump)?;
    let volumes = volume::scene_volumes(&mut lib, &dump, &mut report);
    Ok((meshes, volumes, report, lib))
}

/// Write through `<out>.tmp` and rename, so a failed export leaves no
/// half-written file behind.
fn write_atomically(out: &Path, write: impl FnOnce(&mut std::io::BufWriter<std::fs::File>) -> Result<()>) -> Result<()> {
    let mut tmp = out.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
    let res = write(&mut f).and_then(|_| f.flush().map_err(Into::into));
    drop(f);
    match res {
        Ok(()) => Ok(std::fs::rename(&tmp, out)?),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Where exported files go unless the caller names one: `~/Documents/ORBIS`.
pub fn default_output_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(|| PathBuf::from(".")).join("ORBIS")
}

/// `<usd_root>/<scene stem>.usdz`.
pub fn usdz_output_path(usd_root: &Path, scene_path: &Path) -> PathBuf {
    let stem = scene_path.file_stem().and_then(|s| s.to_str()).unwrap_or("scene");
    usd_root.join(format!("{stem}.usdz"))
}

/// `<usd_root>/<scene stem>.orbx`.
pub fn orbx_output_path(usd_root: &Path, scene_path: &Path) -> PathBuf {
    usdz_output_path(usd_root, scene_path).with_extension("orbx")
}

/// DAZ centimeters → meters.
const CM: f32 = 0.01;
