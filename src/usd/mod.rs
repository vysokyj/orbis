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
mod library;
mod lights;
mod mesh;
mod project;
mod subdivide;
mod usda;
mod usdz;
#[cfg(test)]
mod tests;

pub use camera::*;
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
pub use usdz::*;

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
    /// Skin as OpenPBR Surface instead of Standard Surface (Final only).
    pub skin_openpbr: bool,
    /// Told how far the export got.
    pub progress: Progress,
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

/// How the Octane project renders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OctaneMode {
    /// Direct lighting (Octane's fast, biased preview kernel), Universal
    /// skin — what the lights were first calibrated on.
    #[default]
    Preview,
    /// Path tracing, skin with random-walk subsurface scattering (direct
    /// lighting renders that grey) and the denoiser on. Both modes take the
    /// imager from DAZ's tone mapping ([`ToneMapping`]).
    Final,
}

impl OctaneMode {
    pub fn skin_sss(self) -> bool {
        self == OctaneMode::Final
    }
}

fn skin_shader(opts: &ExportOptions) -> crate::material::SkinShader {
    use crate::material::SkinShader;
    match (opts.octane_mode.skin_sss(), opts.skin_openpbr) {
        (false, _) => SkinShader::None,
        (true, false) => SkinShader::StandardSurface,
        (true, true) => SkinShader::OpenPbr,
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
/// into a `.usdz` at `out`.
pub fn export_scene_usdz(scene: &Path, dump: &Path, out: &Path, opts: &ExportOptions) -> Result<ExportReport> {
    let (meshes, report, mut lib) = build_scene(scene, dump, opts)?;
    let (layer, _) = write_usda(&meshes, &lights_iray_uses(&mut lib));
    write_atomically(out, |f| write_usdz(f, &[("scene.usda", layer.as_bytes())]))?;
    Ok(report)
}

/// Convert `scene` into an Octane package (`.orbx`) at `out`: the scene as an
/// embedded `.usdz`, imported by a USD graph in `project.ocs`, with a render
/// target wired to it and the scene's DAZ camera. Octane X opens it from its
/// own Open panel or gallery (it refuses files handed over from outside).
pub fn export_scene_orbx(scene: &Path, dump: &Path, out: &Path, opts: &ExportOptions) -> Result<ExportReport> {
    let progress = &opts.progress;
    let (meshes, mut report, mut lib) = build_scene(scene, dump, opts)?;
    progress.report(35, "writing the USD scene");
    let lights = lights_iray_uses(&mut lib);
    let (layer, materials) = write_usda(&meshes, &lights);
    let mut cameras = scene_cameras(&mut lib);
    if cameras.is_empty() {
        tracing::info!("USD export: no camera in the scene, framing all meshes");
        cameras.push(("Camera".to_string(), framing_camera(&meshes)));
    }
    let name = scene.file_stem().and_then(|s| s.to_str()).unwrap_or("scene");
    progress.report(60, "translating the materials");
    let layered = crate::daz_material::layered_images(&lib.duf);
    let (inputs, mut textures) = crate::material::material_inputs(&materials, &lib.dirs, layered, 200_000, skin_shader(opts));
    let environment = scene_environment(&mut lib);
    let env_texture = crate::material::environment_texture(environment.as_ref(), &lib.dirs, &mut textures, 190_000);
    let env_power = environment.as_ref().map(|e| if e.scene_only { 0.0 } else { e.intensity }).unwrap_or(1.0);
    let emitters = light_emitters(&lights);
    let light_inputs = crate::material::light_inputs(&emitters, 180_000);
    // The shadow-catching ground stands in for Iray's ground under an HDRI
    // backdrop; without a map there is nothing for its shadow to sit on.
    let ground = environment
        .as_ref()
        .filter(|e| e.map.is_some() && !opts.no_ground)
        .and_then(|e| Some((e.ground_shadow?, ground_height(e, &meshes))));
    // The meshes are in the layer now; a big scene's (19.8 M points) held
    // beside it, its package and every texture peaked at 16 GB.
    drop(meshes);
    let (scene_nodes, mesh_connect) = crate::material::scene_lights(&octane_spots(&lights), ground, 170_000, 1439);
    let tone = scene_tone_mapping(&mut lib);
    // Iray lights with a hidden dome (Draw Dome off) but shows black there;
    // "Scene Only" has no dome light to begin with (power 0, black anyway).
    let hide_dome = environment.as_ref().is_some_and(|e| !e.draw_dome && !e.scene_only);
    let ocs = project_ocs(name, USDZ_ASSET, &cameras, render_size(&lib.duf), &env_texture, opts.octane_mode, &tone, hide_dome, environment.as_ref().and_then(|e| e.sun_sky.as_ref()))
        .replace("@MATERIAL_INPUTS@", &(inputs + &light_inputs))
        .replace("@SCENE_NODES@", &scene_nodes)
        .replace("@MESH_CONNECT@", &mesh_connect.to_string())
        .replace("@ENV_POWER@", &format!("{env_power}"));
    report.render_targets = cameras.len();
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
    let mut scaled = 0;
    progress.report(65, "writing the .orbx");
    write_atomically(out, |f| {
        let mut w = crate::orbx::OrbxWriter::new(f, OCTANE_VERSION)?;
        // The scene: the layer streamed into its .usdz inside the package.
        w.begin(&format!("assets/{USDZ_ASSET}"))?;
        write_usdz(&mut w, &[("scene.usda", layer.as_bytes())])?;
        for (k, (asset, file)) in textures.files.iter().enumerate() {
            progress.report(65 + (30 * k / n_files) as u32, &format!("writing textures ({}/{})", k + 1, textures.files.len()));
            let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
            let data = match report.texture_limit.map(|l| crate::texture_budget::downscale(&data, l)) {
                Some(Ok(Some(small))) => {
                    scaled += 1;
                    small
                }
                _ => data,
            };
            w.add(asset, &data)?;
        }
        for (asset, data) in &textures.generated {
            match report.texture_limit.map(|l| crate::texture_budget::downscale(data, l)) {
                Some(Ok(Some(small))) => {
                    scaled += 1;
                    w.add(asset, &small)?;
                }
                _ => w.add(asset, data)?,
            }
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

/// The embedded scene's file name inside an `.orbx`.
const USDZ_ASSET: &str = "scene.usdz";

/// Octane version the `project.ocs` template was saved with (Octane X 2026.6).
const OCTANE_VERSION: &str = "15050000";

/// Read the scene, its dump and library assets into meshes.
fn build_scene(scene: &Path, dump: &Path, opts: &ExportOptions) -> Result<(Vec<Mesh>, ExportReport, Library)> {
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
    opts.progress.report(5, "building the meshes");
    let (meshes, report) = build_meshes(&mut lib, &dump)?;
    Ok((meshes, report, lib))
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
