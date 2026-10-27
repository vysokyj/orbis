//! DAZ Iray Uber material → Octane Universal material.
//!
//! Writes the `project.ocs` XML for the material inputs Octane creates from
//! the USD's bound materials (`/Scene/Materials/<name>`, node type 20007),
//! each holding a Universal material (type 130). Node and pin formats follow
//! scenes saved by Octane X (see `docs/octane/orbx.md`); pins left out keep
//! Octane's defaults.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::daz_material::Material;

/// Octane version written into each node's `compatibilityVersion`.
const COMPAT: &str = "15050000";

/// Octane's `namedColorSpace` values (Lua `octane.namedColorSpace`).
const COLOUR_SPACE_SRGB: u32 = 1;
const COLOUR_SPACE_LINEAR_SRGB: u32 = 2;

/// Iray works photometrically under a camera exposure (EV 13 by default);
/// physical renderers that take watts (Blender, Octane) match its look at
/// about 15 000 lm per watt for DAZ lights and emission, as Blender renders
/// of DAZ scenes confirm.
pub const DAZ_LUMENS_PER_WATT: f64 = 15_000.0;

/// Power for an emitter that shines from both sides: Iray sends each side
/// the full flux (Calib-L-Rect-50-2sided, -Emit-cd-2sided: the floor below
/// lit 1.000× the one-sided), and so does Octane's `doubleSided` emission —
/// at twice the power the floor came out 1.97–2.03× Iray, at the same 0.99–
/// 1.01×.
pub const TWO_SIDED_POWER: f64 = 1.0;

/// Octane's colour (texture) emission against its blackbody at the same
/// power: a non-photometric light (white, no temperature) lit the floor
/// 0.69× Iray where the photometric ones (blackbody 6500 K) matched.
pub const COLOUR_EMISSION_POWER: f64 = 1.0 / 0.69;

/// Octane's emission against Iray's: a one-sided 50 × 50 cm plane at
/// 5000 cd/m² lit every surface 3.4× Iray without it (#6).
const EMISSION_POWER_FACTOR: f64 = 1.0 / 3.4;

/// Emission power (W) of an Iray emissive surface of `area_m2`:
/// Luminance × its unit factor / 2 / 15 000 × `EMISSION_POWER_FACTOR` —
/// a total for lm and W, per m² for the luminance units.
pub fn iray_emission_watts(mat: &Material, area_m2: f64) -> f64 {
    let l = mat.float("Luminance").unwrap_or(1500.0) as f64;
    let per = |factor: f64| l * factor / 2.0 / DAZ_LUMENS_PER_WATT * EMISSION_POWER_FACTOR;
    match mat.float("Luminance Units").unwrap_or(0.0) as i64 {
        1 => per(1000.0) * area_m2,
        2 => per(10.764) * area_m2,
        3 => per(10_000.0) * area_m2,
        4 => per(1.0),
        5 => per(mat.float("Luminous Efficacy").unwrap_or(15.0) as f64),
        _ => per(1.0) * area_m2,
    }
}

/// Textures referenced by the materials, to pack into the `.orbx`.
#[derive(Debug, Default)]
pub struct Textures {
    /// Content-relative DAZ path → `assets/tex/…` path inside the package.
    by_source: HashMap<String, String>,
    /// (package path, file on disk) in first-use order.
    pub files: Vec<(String, PathBuf)>,
    /// Images the content directories don't have (left out of the material).
    pub missing: Vec<String>,
    /// Layered images (DAZ Layered Image Editor) by id, flattened on use.
    pub layered: HashMap<String, crate::layered_image::LayeredImage>,
    /// Flattened layered images: (package path, PNG bytes).
    pub generated: Vec<(String, Vec<u8>)>,
    /// Mean colours of hair maps by DAZ path (`None`: not found).
    means: HashMap<String, Option<[f32; 3]>>,
}

impl Textures {
    /// The package path for a DAZ image, resolving it once against `dirs`.
    fn asset(&mut self, dirs: &[PathBuf], daz_path: &str) -> Option<String> {
        if let Some(a) = self.by_source.get(daz_path) {
            return Some(a.clone());
        }
        if let Some(id) = daz_path.strip_prefix(crate::daz_material::LAYERED_PREFIX) {
            return self.flatten(dirs, daz_path, id);
        }
        let rel = daz_path.trim_start_matches('/');
        let Some(file) = dirs.iter().map(|d| d.join(rel)).find(|p| p.is_file()) else {
            if !self.missing.iter().any(|m| m == daz_path) {
                self.missing.push(daz_path.to_string());
            }
            return None;
        };
        let base = Path::new(rel).file_name().and_then(|n| n.to_str()).unwrap_or("texture");
        let safe: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
        let asset = format!("assets/tex/{:04}_{safe}", self.files.len());
        self.files.push((asset.clone(), file));
        self.by_source.insert(daz_path.to_string(), asset.clone());
        Some(asset)
    }
}

impl Textures {
    /// A DAZ image's mean colour, linear (sRGB-decoded): strands carry no
    /// UVs, so a hair map can't be mapped onto them.
    fn mean_colour(&mut self, dirs: &[PathBuf], daz_path: &str) -> Option<[f32; 3]> {
        self.mean(dirs, daz_path, false)
    }

    /// The mean colour over the map's painted texels: near-black ones (the
    /// empty space between a figure map's UV islands) left out.
    fn mean_covered_colour(&mut self, dirs: &[PathBuf], daz_path: &str) -> Option<[f32; 3]> {
        self.mean(dirs, daz_path, true)
    }

    fn mean(&mut self, dirs: &[PathBuf], daz_path: &str, covered: bool) -> Option<[f32; 3]> {
        let key = format!("{}{daz_path}", if covered { "covered:" } else { "" });
        if let Some(c) = self.means.get(&key) {
            return *c;
        }
        let rel = daz_path.trim_start_matches('/');
        let mean = dirs.iter().map(|d| d.join(rel)).find(|p| p.is_file()).and_then(|f| {
            let img = image::open(&f).map_err(|e| tracing::warn!(file = %f.display(), error = %e, "Octane export: hair map unreadable")).ok()?;
            let small = img.thumbnail(64, 64).to_rgb8();
            let lin = |c: u8| {
                let c = c as f32 / 255.0;
                if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
            };
            let mut sum = [0.0f32; 3];
            let mut n = 0.0f32;
            for p in small.pixels().filter(|p| !covered || p.0.iter().map(|c| *c as u32).sum::<u32>() >= 30) {
                (0..3).for_each(|k| sum[k] += lin(p[k]));
                n += 1.0;
            }
            Some(sum.map(|s| s / n.max(1.0)))
        });
        self.means.insert(key, mean);
        mean
    }

    /// Flatten a layered image into a PNG asset.
    fn flatten(&mut self, dirs: &[PathBuf], key: &str, id: &str) -> Option<String> {
        let img = self.layered.get(id)?.clone();
        let (pixels, missing) = match crate::layered_image::composite(&img, dirs) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(image = %id, error = %e, "Octane export: layered image not flattened");
                self.missing.push(key.to_string());
                return None;
            }
        };
        for m in missing {
            if !self.missing.contains(&m) {
                self.missing.push(m);
            }
        }
        let mut png = Vec::new();
        if let Err(e) = pixels.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png) {
            tracing::warn!(image = %id, error = %e, "Octane export: layered image not encoded");
            return None;
        }
        let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        let asset = format!("assets/tex/{:04}_{safe}.png", self.files.len() + self.generated.len());
        self.generated.push((asset.clone(), png));
        self.by_source.insert(key.to_string(), asset.clone());
        Some(asset)
    }
}

impl Textures {
    /// An image derived from a map's luminance: each texel's luminance `l`
    /// (decoded at gamma 2.2) → `f(l)` (linear RGB), written as a PNG
    /// encoded at `gamma` — MDL's `file_texture` offsets and scales baked in.
    fn derived(&mut self, dirs: &[PathBuf], src: &str, key: &str, gamma: f32, f: impl Fn(f32) -> [f32; 3]) -> Option<String> {
        if let Some(a) = self.by_source.get(key) {
            return Some(a.clone());
        }
        let rel = src.trim_start_matches('/');
        let Some(file) = dirs.iter().map(|d| d.join(rel)).find(|p| p.is_file()) else {
            if !self.missing.iter().any(|m| m == src) {
                self.missing.push(src.to_string());
            }
            return None;
        };
        let img = image::open(&file).ok()?.to_rgb8();
        let decode = |c: u8| crate::daz_material::colour_to_linear(f32::from(c) / 255.0);
        let out = image::RgbImage::from_fn(img.width(), img.height(), |x, y| {
            let p = img.get_pixel(x, y).0;
            let l = 0.2126 * decode(p[0]) + 0.7152 * decode(p[1]) + 0.0722 * decode(p[2]);
            image::Rgb(f(l).map(|v| (v.clamp(0.0, 1.0).powf(1.0 / gamma) * 255.0 + 0.5) as u8))
        });
        let mut png = Vec::new();
        out.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
        let base = Path::new(rel).file_stem().and_then(|n| n.to_str()).unwrap_or("map");
        let safe: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        let asset = format!("assets/tex/{:04}_{safe}_derived.png", self.files.len() + self.generated.len());
        self.generated.push((asset.clone(), png));
        self.by_source.insert(key.to_string(), asset.clone());
        Some(asset)
    }
}

/// NVIDIA's textured MDL metals (`nvidia::metal_examples`, DAZ's "NVIDIA MDL
/// Examples" shaders): a glossy metal layer over black whose weight and
/// roughness come from grey maps as offset + scale · luminance
/// (`metal_textured`, `metal_textured_anisotropic_uv`).
struct MdlMetal {
    /// Linear, as MDL writes it.
    colour: [f32; 3],
    /// (map, offset, scale) of the reflection weight.
    weight: (String, f32, f32),
    /// (map, offset, scale) of the roughness along u, and along v.
    rough_u: (String, f32, f32),
    rough_v: (String, f32, f32),
    /// Bump map and amount.
    bump: (String, f32),
    tiles: [f32; 2],
}

/// The MDL metal a brick shader renders with, its parameters from the
/// shader's channels and the MDL defaults (`nvidia/metal_examples.mdl`).
fn mdl_metal(mat: &Material) -> Option<MdlMetal> {
    // A texture channel the shader leaves out takes the MDL default; one it
    // sets empty is no texture — Iray then reads 0 (the Metal Brushed
    // preset, its brush textures empty, renders black). Scenes keep the
    // maps as "/resources/…" under DAZ's shader directory.
    let map = |ch: &str, default: &str| match mat.channel(ch) {
        Some(c) => c.image.clone().unwrap_or_default(),
        None => format!("resources/{default}"),
    };
    let tiles = [mat.float("Horizontal Tiles"), mat.float("Vertical Tiles")];
    if mat.uses_mdl("nvidia::metal_examples::metal_brushed") {
        let brushing = mat.float("brushing_strength").unwrap_or(0.5);
        let stretch = mat.float("reflection_stretch").unwrap_or(0.75);
        return Some(MdlMetal {
            colour: [0.65; 3],
            weight: (map("brush_texture", "brushed_full_contrast.jpg"), 0.5, 0.5),
            rough_u: (map("roughness_texture_u", "brushed_full_contrast.jpg"), 0.02, brushing * 0.8),
            rough_v: (map("roughness_texture_v", "brushed_full_contrast.jpg"), stretch * 0.4 + 0.05, stretch * 0.3 + 0.1),
            bump: (bump_map(mat, "brushed_a.jpg"), mat.float("Bump Strength").unwrap_or(0.4)),
            tiles: tiles.map(|t| t.unwrap_or(3.0)),
        });
    }
    if mat.uses_mdl("nvidia::metal_examples::cast_iron") {
        let rough = (map("roughness_texture", "grunge_d.jpg"), 0.06, 0.4);
        return Some(MdlMetal {
            colour: [0.543, 0.53, 0.53],
            weight: ("resources/grunge_b.jpg".into(), 0.5, 3.0),
            rough_u: rough.clone(),
            rough_v: rough,
            bump: (bump_map(mat, "grunge_d.jpg"), mat.float("Bump Strength").unwrap_or(1.5)),
            tiles: tiles.map(|t| t.unwrap_or(1.0)),
        });
    }
    None
}

/// The MDL metal's bump map: DAZ hangs it on "Bump Strength" (the
/// amount), else `bump_texture`, else the MDL default.
fn bump_map(mat: &Material, default: &str) -> String {
    mat.image("Bump Strength").or(mat.image("bump_texture")).map(str::to_string).unwrap_or_else(|| format!("resources/{default}"))
}

/// Builds node XML with unique ids.
struct Ocs<'a> {
    out: String,
    next_id: u32,
    dirs: &'a [PathBuf],
    textures: &'a mut Textures,
    /// The current material's Iray UV tiling (tiles U, V, offset U, V),
    /// applied to every texture it uses.
    tiling: Option<[f32; 4]>,
    /// Writing the diffuse part of Iray's weighted mixing (`weighted_mix_material`).
    in_mix: bool,
    /// Thin-walled glass at IOR 1 (`Tuning::thin_glass_ior_one`).
    thin_glass_ior_one: bool,
    /// `Tuning::max_transmission`.
    max_transmission: f32,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "&apos;").replace('"', "&quot;")
}

/// A value node `<node type><attr name='value' type>…</attr></node>`.
fn num(x: f32) -> String {
    let s = format!("{x:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".into() } else { s.to_string() }
}

/// [`num`] for f64 (positions and angles keep their precision).
fn num64(x: f64) -> String {
    let s = format!("{x:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".into() } else { s.to_string() }
}

impl Ocs<'_> {
    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn pin_open(&mut self, indent: usize, name: &str) {
        let _ = writeln!(self.out, "{:indent$}<pin name='{name}'>", "");
    }

    fn pin_close(&mut self, indent: usize) {
        let _ = writeln!(self.out, "{:indent$}</pin>", "");
    }

    /// `<pin name><node type><attr name='value' type>value</attr></node></pin>`
    fn value_pin(&mut self, indent: usize, pin: &str, node_type: u32, attr_type: u32, value: &str) {
        let id = self.id();
        self.pin_open(indent, pin);
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='{node_type}'>", "");
        let _ = writeln!(self.out, "{:i$} <attr name='value' type='{attr_type}'>{value}</attr>", "");
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// An image texture's `colorSpace` pin (an OCIO colour space node, 163).
    fn colour_space_pin(&mut self, indent: usize, space: u32) {
        let id = self.id();
        self.pin_open(indent, "colorSpace");
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='163'>", "");
        let _ = writeln!(self.out, "{:i$} <attr name='colorSpace' type='2'>{space}</attr>", "");
        let _ = writeln!(self.out, "{:i$} <attr name='ocioColorSpaceName' type='10'></attr>", "");
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// Iray's weighted base mixing (2): a Mix material (19) of a mirror-like
    /// metal in the glossy colour (`material1`, `amount` = the glossy share)
    /// and the surface without its gloss (`material2`). Iray blends a
    /// full-strength glossy reflection by the share (`weighted_gloss`); as a
    /// metallic share of one Universal the reflection took the diffuse
    /// colour — Calib-Mixing's weighted grey sphere (diffuse 1, gloss 0.5)
    /// rendered 0.63 × Iray, the ceiling 0.85.
    fn weighted_mix_material(&mut self, indent: usize, name: &str, mat: &Material, area_m2: f64, skin: SkinShader, share: f32) {
        let tiles = [mat.float("Horizontal Tiles"), mat.float("Vertical Tiles")].map(|t| t.filter(|t| *t > 0.0).unwrap_or(1.0));
        let offsets = [mat.float("Horizontal Offset"), mat.float("Vertical Offset")].map(|o| o.unwrap_or(0.0));
        self.tiling = (tiles != [1.0, 1.0] || offsets != [0.0, 0.0]).then_some([tiles[0], tiles[1], offsets[0], offsets[1]]);
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='19' name='{}'>", "", esc(name));
        let i = indent + 1;
        self.float_pin(i, "amount", share.clamp(0.0, 1.0));
        // The gloss: a metal coloured by the glossy colour, the surface's
        // roughness and detail.
        self.pin_open(i, "material1");
        let gid = self.id();
        let j = i + 1;
        let _ = writeln!(self.out, "{:j$}<node id='{gid}' type='130' name='{} gloss'>", "", esc(name));
        let _ = writeln!(self.out, "{:j$} <attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        let gloss_color = mat.color("Glossy Color").unwrap_or([1.0; 3]);
        match mat.image("Glossy Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.tinted_image_pin(j + 1, "albedo", &asset, 1.0, gloss_color),
            None => self.rgb_pin(j + 1, "albedo", gloss_color),
        }
        self.float_pin(j + 1, "metallic", 1.0);
        let (rough_ch, rough, inverted) = mat.roughness_source();
        let rough_ch = if inverted { None } else { rough_ch };
        let rough = iray_roughness(mat, rough, inverted);
        self.float_or_map(j + 1, "roughness", mat, rough_ch, rough.clamp(0.0, 1.0));
        if let Some(asset) = mat.image("Normal Map").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            self.image_pin(j + 1, "normal", &asset, false, 1.0, 1.0);
        }
        if let Some(asset) = mat.image("Bump Strength").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            self.image_pin(j + 1, "bump", &asset, true, 1.0, 1.0);
        }
        // Iray's top coat lies over both parts: on the base alone a dark
        // weighted surface under a 0.8 coat lost half its sheen (Calib-Mixing
        // jamb 0.55 × Iray).
        self.top_coat(j + 1, mat, 0.0);
        let opacity = mat.opacity().clamp(0.0, 1.0);
        if opacity < 1.0 || mat.image(mat.opacity_channel()).is_some() {
            self.float_or_map(j + 1, "opacity", mat, Some(mat.opacity_channel()), opacity);
        }
        let _ = writeln!(self.out, "{:j$}</node>", "");
        self.pin_close(i);
        // The rest: the surface as Universal, without the gloss.
        self.pin_open(i, "material2");
        self.in_mix = true;
        self.universal(i + 1, &format!("{name} base"), mat, area_m2, skin);
        self.in_mix = false;
        self.pin_close(i);
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// An NVIDIA MDL metal (`mdl_metal`) as a Universal metal: albedo =
    /// colour × weight map, roughness = √(u · v) per texel, anisotropy from
    /// the two at a mid-grey texel, the bump map — all at the shader's tiles.
    fn mdl_metal(&mut self, indent: usize, name: &str, m: &MdlMetal) {
        self.tiling = (m.tiles != [1.0, 1.0]).then_some([m.tiles[0], m.tiles[1], 0.0, 0.0]);
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='130' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        let (wmap, woff, wscale) = &m.weight;
        let colour = m.colour;
        let key = format!("mdl-metal-albedo|{wmap}|{woff}|{wscale}|{colour:?}");
        let derived = if wmap.is_empty() { None } else { self.textures.derived(self.dirs, wmap, &key, 2.2, |l| colour.map(|c| c * (woff + wscale * l).clamp(0.0, 1.0))) };
        match derived {
            Some(asset) => self.image_pin(i, "albedo", &asset, false, 2.2, 1.0),
            // No texture: Iray's lookup reads 0 and the metal comes out black.
            None => self.rgb_pin(i, "albedo", [0.0; 3]),
        }
        self.float_pin(i, "metallic", 1.0);
        let ((umap, uoff, uscale), (_, voff, vscale)) = (&m.rough_u, &m.rough_v);
        let rough = |l: f32| ((uoff + uscale * l).max(0.0) * (voff + vscale * l).max(0.0)).sqrt();
        let key = format!("mdl-metal-rough|{umap}|{uoff}|{uscale}|{voff}|{vscale}");
        let derived = if umap.is_empty() { None } else { self.textures.derived(self.dirs, umap, &key, 1.0, |l| [rough(l); 3]) };
        match derived {
            Some(asset) => self.image_pin(i, "roughness", &asset, true, 1.0, 1.0),
            None => self.float_pin(i, "roughness", rough(0.5)),
        }
        let (ru, rv) = (uoff + uscale * 0.5, voff + vscale * 0.5);
        if (ru - rv).abs() > 1e-3 {
            self.float_pin(i, "anisotropyTexture", ((rv - ru) / (rv + ru)).clamp(-1.0, 1.0));
        }
        let (bmap, amount) = &m.bump;
        if *amount > 0.0 {
            if let Some(asset) = self.textures.asset(self.dirs, bmap) {
                self.image_pin(i, "bump", &asset, true, 1.0, *amount);
            }
        }
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// Iray's Top Coat: a clear dielectric layer over everything, as
    /// Universal's coating.
    fn top_coat(&mut self, i: usize, mat: &Material, min_roughness: f32) {
        let coat = mat.float("Top Coat Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        if coat <= 0.0 {
            return;
        }
        let colour = mat.color("Top Coat Color").unwrap_or([1.0; 3]);
        self.rgb_pin(i, "coating", colour.map(|c| c * coat));
        let (r, inv) = match (mat.float("Base Mixing").unwrap_or(0.0) as i32, mat.float("Top Coat Glossiness")) {
            (1 | 2, Some(g)) => (1.0 - g, true),
            _ => (mat.float("Top Coat Roughness").unwrap_or(0.0), false),
        };
        self.float_pin(i, "coatingRoughness", iray_roughness(mat, r, inv).max(min_roughness).clamp(0.0, 1.0));
        self.float_pin(i, "coatingIor", top_coat_ior(mat));
    }

    /// An absorption medium (58) on `medium`: `sigma` (1/m per channel) as
    /// its colour × `scale`, not inverted.
    fn absorption_pin(&mut self, indent: usize, sigma: [f32; 3]) {
        let scale = sigma.iter().copied().fold(0.0f32, f32::max);
        let id = self.id();
        self.pin_open(indent, "medium");
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='58'>", "");
        self.value_pin(i + 1, "scale", 6, 9, &format!("{} 0 0 0", num(scale)));
        self.rgb_pin(i + 1, "absorption", sigma.map(|s| s / scale));
        self.value_pin(i + 1, "invertAbsorption", 11, 1, "0");
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    fn float_pin(&mut self, indent: usize, pin: &str, v: f32) {
        self.value_pin(indent, pin, 31, 6, &num(v));
    }

    fn rgb_pin(&mut self, indent: usize, pin: &str, c: [f32; 3]) {
        self.value_pin(indent, pin, 33, 8, &format!("{} {} {}", num(c[0]), num(c[1]), num(c[2])));
    }

    /// An image texture pin: RGB (34) or greyscale (36); `gamma` 2.2 for
    /// colour maps, 1 for data; `power` multiplies the texture.
    fn image_pin(&mut self, indent: usize, pin: &str, asset: &str, grey: bool, gamma: f32, power: f32) {
        let id = self.id();
        self.pin_open(indent, pin);
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='{}'>", "", if grey { 36 } else { 34 });
        let _ = writeln!(self.out, "{:i$} <attr name='filename' type='11'>{}</attr>", "", esc(asset));
        // Without it Octane picks the image's colour space itself on load
        // (sRGB for 8-bit files) and overrides the `colorSpace` pin: every
        // data map — roughness, opacity, metallic, bump, normal — was
        // decoded as sRGB (nearly clear sunglasses, Octane1; probe 56).
        let _ = writeln!(self.out, "{:i$} <attr name='initialColorSpaceSelected' type='1'>1</attr>", "");
        self.value_pin(i + 1, "gamma", 6, 8, &format!("{} 0 0", num(gamma)));
        // Octane reads `gamma` only in its "linear sRGB + legacy gamma" colour
        // space; the pin's default is linear sRGB, which left colour maps
        // undecoded (pale, washed-out skin and clothes).
        self.colour_space_pin(i + 1, if gamma > 1.0 { COLOUR_SPACE_SRGB } else { COLOUR_SPACE_LINEAR_SRGB });
        self.float_pin(i + 1, "power", power);
        // Wrap around: DAZ figures lay their surfaces out in UV tiles (UDIM —
        // torso at U 1–2, arms 2–3, …), each tile using its own texture.
        self.value_pin(i + 1, "borderModeU", 57, 2, "0");
        self.value_pin(i + 1, "borderModeV", 57, 2, "0");
        // Iray tiles a material's maps N times: the texture shrinks to 1/N.
        if let Some([tu, tv, ou, ov]) = self.tiling {
            let tid = self.id();
            let j = i + 1;
            self.pin_open(j, "transform");
            let _ = writeln!(self.out, "{:j$} <node id='{tid}' type='67'>", "");
            let _ = writeln!(self.out, "{:j$}  <attr name='rotationOrder' type='2'>2</attr>", "");
            let _ = writeln!(self.out, "{:j$}  <attr name='rotation' type='8'>0 0 0</attr>", "");
            let _ = writeln!(self.out, "{:j$}  <attr name='scale' type='8'>{} {} 1</attr>", "", num(1.0 / tu), num(1.0 / tv));
            let _ = writeln!(self.out, "{:j$}  <attr name='translation' type='8'>{} {} 0</attr>", "", num(ou), num(ov));
            let _ = writeln!(self.out, "{:j$} </node>", "");
            self.pin_close(j);
        }
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// A colour map scaled by `power` and multiplied by Iray's colour `tint`:
    /// a grey tint folds into the image's power, a coloured one goes through
    /// a Multiply texture (39) — as a scalar, a grey strand map × brown
    /// rendered grey-white pubic hair (Octane3).
    fn tinted_image_pin(&mut self, indent: usize, pin: &str, asset: &str, power: f32, tint: [f32; 3]) {
        let (lo, hi) = tint.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &c| (lo.min(c), hi.max(c)));
        if hi - lo < 0.01 {
            return self.image_pin(indent, pin, asset, false, 2.2, power * map_tint(tint));
        }
        let id = self.id();
        self.pin_open(indent, pin);
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='39'>", "");
        self.image_pin(i + 1, "texture1", asset, false, 2.2, power);
        self.rgb_pin(i + 1, "texture2", tint.map(|c| c.clamp(0.0, 1.0)));
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// A float pin driven by `channel`'s texture when it has one (scaled by
    /// `value`), else the constant `value`.
    fn float_or_map(&mut self, indent: usize, pin: &str, mat: &Material, channel: Option<&str>, value: f32) {
        let tex = channel.and_then(|c| mat.image(c)).map(str::to_string);
        match tex.and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.image_pin(indent, pin, &asset, true, 1.0, value),
            None => self.float_pin(indent, pin, value),
        }
    }

    /// Iray emission → an Octane emission: blackbody at the Iray temperature,
    /// or a texture emission tinted by Emission Color when the temperature is
    /// 0, its power from [`iray_emission_watts`].
    fn emission(&mut self, indent: usize, mat: &Material, area_m2: f64) {
        let color = mat.color("Emission Color").unwrap_or([0.0; 3]);
        if color.iter().all(|c| *c <= 0.0) {
            return;
        }
        let two_sided = mat.float("Two Sided Light").unwrap_or(0.0) > 0.0;
        let watts = iray_emission_watts(mat, area_m2) * if two_sided { TWO_SIDED_POWER } else { 1.0 };
        if watts <= 0.0 {
            return;
        }
        let temperature = mat.float("Emission Temperature").unwrap_or(0.0);
        let id = self.id();
        self.pin_open(indent, "emission");
        let i = indent + 1;
        let blackbody = temperature > 0.0;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='{}'>", "", if blackbody { 53 } else { 54 });
        let tint = mat.image("Emission Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t));
        // Iray multiplies the blackbody (or plain) emission by the colour:
        // left out, Fetroom's dark purple trims and door (0.1, 0, 0.1) glowed
        // white at full power — the room twice as bright, the door grey.
        match &tint {
            Some(asset) => self.tinted_image_pin(i + 1, "efficiency or texture", asset, 1.0, color),
            None => self.rgb_pin(i + 1, "efficiency or texture", color),
        }
        self.value_pin(i + 1, "power", 6, 8, &format!("{} 0 0", num(watts as f32)));
        if blackbody {
            self.value_pin(i + 1, "temperature", 6, 8, &format!("{} 0 0", num(temperature)));
            self.value_pin(i + 1, "normalize", 11, 1, "1");
        }
        self.value_pin(i + 1, "surfaceBrightness", 11, 1, "0");
        self.value_pin(i + 1, "doubleSided", 11, 1, if two_sided { "1" } else { "0" });
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// Iray translucency (skin) → Octane Standard Surface (type 277) with its
    /// built-in random-walk subsurface: the diffuse map stays the base colour
    /// and `subsurface` blends diffuse into scattering, per material — no
    /// hand-wired medium (that rendered grey, with a seam at the graft).
    ///
    /// Iray describes the volume by measurement distances (DAZ cm): light
    /// keeps `Transmitted Color` after `Transmitted Measurement Distance`
    /// (absorption σa = −ln T / d) and scatters once per `Scattering
    /// Measurement Distance` (σs, reduced by `SSS Direction` g). The
    /// diffusion length 1/√(3·σa·σs(1−g)) per channel gives the radius colour
    /// (normalised) and the scale (its maximum, in meters).
    ///
    /// Only under path tracing (`Tuning::skin`): it needs that kernel —
    /// direct lighting renders random walk grey.
    fn standard_surface(&mut self, indent: usize, name: &str, mat: &Material, translucency: f32, openpbr: bool) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='{}' name='{}'>", "", if openpbr { 505 } else { 277 }, esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        self.float_pin(i, "base", 1.0);
        // The colour the skin shows: the diffuse map darkened as Iray's
        // translucent layer does (`skin_tint`).
        let tc_map = mat.image("Translucency Color").map(str::to_string).and_then(|t| self.textures.mean_covered_colour(self.dirs, &t));
        let mut tint = skin_tint(mat, translucency, tc_map);
        if openpbr {
            // OpenPBR's random walk shows less of the colour, the more so
            // the more subsurface: brought back to Standard Surface's level.
            let lift = 1.0 / (1.0 - OPENPBR_SKIN_DARKENING * translucency);
            tint = tint.map(|t| t * lift);
        }
        let colour = [0, 1, 2].map(|k| mat.base_color()[k] * tint[k]);
        match mat.image("diffuse").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.tinted_image_pin(i, "baseColor", &asset, 1.0, colour),
            None => self.rgb_pin(i, "baseColor", colour),
        }
        let (rough_ch, rough, inverted) = mat.roughness_source();
        let rough_ch = if inverted { None } else { rough_ch };
        self.float_or_map(i, "roughness", mat, rough_ch, rough.clamp(0.0, 1.0));

        self.float_pin(i, "subsurface", translucency);
        // The SSS colour is the colour the skin *shows* (Standard Surface,
        // like Arnold's): the diffuse map, with tan and make-up. Iray's
        // translucency map is a pale, pinkish "sss" map of how light shines
        // through — as subsurface colour it bleached Zelie and wiped her
        // make-up (Octane1 vs Blender, 2026-10-21).
        match mat.image("diffuse").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.tinted_image_pin(i, "subsurfaceColor", &asset, 1.0, colour),
            None => self.rgb_pin(i, "subsurfaceColor", colour),
        }
        let (radius, scale) = sss_radius(mat);
        if openpbr {
            // OpenPBR: `radius` the distance, `scale` the per-channel colour.
            self.value_pin(i, "radius", 6, 9, &format!("{} 0 0 0", num(scale)));
            self.rgb_pin(i, "scale", radius);
        } else {
            self.rgb_pin(i, "radius", radius);
            self.value_pin(i, "scale", 6, 9, &format!("{} 0 0 0", num(scale)));
        }
        let g = mat.float("SSS Direction").unwrap_or(0.0).clamp(-0.95, 0.95);
        self.value_pin(i, "subsurfaceAnisotropy", 6, 9, &format!("{} 0 0 0", num(g)));

        if let Some(asset) = mat.image("Normal Map").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            self.image_pin(i, "normal", &asset, false, 1.0, 1.0);
        }
        if let Some(asset) = mat.image("Bump Strength").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            self.image_pin(i, "bump", &asset, true, 1.0, 1.0);
        }
        let opacity = mat.opacity().clamp(0.0, 1.0);
        if opacity < 1.0 || mat.image(mat.opacity_channel()).is_some() {
            self.float_or_map(i, "opacity", mat, Some(mat.opacity_channel()), opacity);
        }
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// DAZ's dForce hair shader (Blended Dual Lobe Hair, on strands) → an
    /// Octane Hair material (type 147) in albedo mode: the mean of root and
    /// tip colour and transmission colour, each its map's mean × its tint
    /// (strands have no UVs), mixed by `HAIR_COLOUR_SHARE` /
    /// `HAIR_TRANSMISSION_SHARE`; longitudinal and azimuthal roughness from
    /// `base_roughness`; the specular tinted down to the glossy layer's weight.
    fn hair(&mut self, indent: usize, name: &str, mat: &Material) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='147' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        let mut colour = |ch: &str| -> [f32; 3] {
            let tint = mat.color(ch).unwrap_or([1.0; 3]);
            let map = mat.image(ch).map(str::to_string).and_then(|t| self.textures.mean_colour(self.dirs, &t));
            match map {
                Some(m) => [0, 1, 2].map(|k| m[k] * tint[k]),
                None if mat.image(ch).is_none() => tint,
                None => mat.base_color(),
            }
        };
        let (root, tip) = (colour("Hair Root Color"), colour("Hair Tip Color"));
        let (root_t, tip_t) = (colour("Root Transmission Color"), colour("Tip Transmission Color"));
        self.value_pin(i, "hairMode", 57, 2, "0"); // albedo
        self.rgb_pin(i, "albedo", [0, 1, 2].map(|k| HAIR_COLOUR_SHARE * (root[k] + tip[k]) / 2.0 + HAIR_TRANSMISSION_SHARE * (root_t[k] + tip_t[k]) / 2.0));
        let gloss = mat.float("Glossy Layer Weight").unwrap_or(1.0).clamp(0.0, 1.0);
        self.rgb_pin(i, "specular", [gloss; 3]);
        let rough = mat.float("base_roughness").unwrap_or(0.2).clamp(0.0, 1.0);
        self.float_pin(i, "roughness", rough);
        self.float_pin(i, "roughnessV", rough);
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// Iray's "Weighted" base mixing (`Base Mixing` 2) blends diffuse,
    /// translucency and glossy by their weights; Universal takes only the
    /// diffuse map. The factor that scales it to the blend's brightness:
    /// Σ wᵢ·Yᵢ / (Σ wᵢ · Y_diffuse), Y each layer's mean luminance (map mean
    /// × colour). Mesh hair (HS CB, Octane1) has a dark diffuse map under a
    /// 3× lighter translucency and glossy map — without it the braids came
    /// out grey-brown; ×3 by hand matched Iray (probe 59), this gives 2.8.
    fn weighted_mix(&mut self, mat: &Material) -> f32 {
        if mat.float("Base Mixing").unwrap_or(0.0) as i32 != 2 {
            return 1.0;
        }
        let mut layer = |colour: &str, weight: f32| -> Option<(f32, f32)> {
            let tint = mat.color(colour).unwrap_or([1.0; 3]);
            let mean = match mat.image(colour).map(str::to_string) {
                Some(t) => self.textures.mean_colour(self.dirs, &t)?,
                None => [1.0; 3],
            };
            let y = 0.2126 * mean[0] * tint[0] + 0.7152 * mean[1] * tint[1] + 0.0722 * mean[2] * tint[2];
            (weight > 0.0).then_some((weight, y))
        };
        let Some((wd, yd)) = layer("diffuse", mat.float("Diffuse Weight").unwrap_or(1.0)) else { return 1.0 };
        let mut sum = (wd, wd * yd);
        for (colour, weight) in [("Translucency Color", "Translucency Weight"), ("Glossy Color", "Glossy Weight")] {
            if let Some((w, y)) = layer(colour, mat.float(weight).unwrap_or(0.0)) {
                sum = (sum.0 + w, sum.1 + w * y);
            }
        }
        if yd <= 0.0 { 1.0 } else { sum.1 / (sum.0 * yd) }
    }

    fn universal(&mut self, indent: usize, name: &str, mat: &Material, area_m2: f64, skin: SkinShader) {
        if mat.channel("Hair Root Color").is_some() {
            return self.hair(indent, name, mat);
        }
        if let Some(metal) = mdl_metal(mat) {
            return self.mdl_metal(indent, name, &metal);
        }
        let tiles = [mat.float("Horizontal Tiles"), mat.float("Vertical Tiles")].map(|t| t.filter(|t| *t > 0.0).unwrap_or(1.0));
        let offsets = [mat.float("Horizontal Offset"), mat.float("Vertical Offset")].map(|o| o.unwrap_or(0.0));
        self.tiling = (tiles != [1.0, 1.0] || offsets != [0.0, 0.0]).then_some([tiles[0], tiles[1], offsets[0], offsets[1]]);
        // Skin: Iray translucency on an opaque, non-emissive surface. Hair,
        // lashes and brows are translucent too, but cut-out single-sided
        // cards, and fabrics are thin-walled sheets — random walk needs a
        // closed volume.
        let translucency = mat.float("Translucency Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        let refractive = mat.float("Refraction Weight").unwrap_or(0.0) > 0.0;
        let emissive = mat.color("Emission Color").is_some_and(|c| c.iter().any(|v| *v > 0.0));
        let cutout = mat.opacity() < 1.0 || mat.image(mat.opacity_channel()).is_some();
        // Iray "Thin Walled": clothing — a single-sided sheet, no volume.
        let thin = mat.float("Thin Walled").unwrap_or(0.0) > 0.0;
        // Skin isn't metallic: the Gladio outfit is PBRSkin with translucency
        // and a metallic map throughout — as skin its metal was lost and the
        // open cloak rendered grey instead of red (Gladiator).
        let metal = mat.metallic() >= NEGLIGIBLE_METALLIC || mat.image("Metallic Weight").is_some();
        // Nor is it on a prop: Gladiator's grass (Iray Uber, translucent, not
        // thin-walled) rendered as grey cards.
        // Nor in the eye: Genesis irises, pupils and sclera are translucent
        // too, and the skin's tint (half, reddened) turned blue irises dark
        // green (Calib-Hair, #20).
        if skin != SkinShader::None && translucency > 0.0 && !refractive && !emissive && !cutout && !thin && !metal && !mat.prop && !is_eye(name) && mat.image("diffuse").is_some() {
            return self.standard_surface(indent, name, mat, translucency, skin == SkinShader::OpenPbr);
        }
        if !self.in_mix {
            if let Some(share) = weighted_gloss(mat) {
                return self.weighted_mix_material(indent, name, mat, area_m2, skin, share);
            }
        }
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='130' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");

        // Iray refraction replaces the diffuse base: a fully refractive
        // surface (cornea, eye moisture, glass) has no diffuse left — a white
        // albedo there renders a milky shell over the eye.
        let refraction = mat.float("Refraction Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        // Iray translucency (off the skin path) passes its share of the
        // diffuse through the surface: Universal's diffuse transmission. Left
        // out, Calib-SSS's translucent spheres rendered 1.6 – 3.2 × Iray
        // (Translucency Weight 0.5 / 1), the Gladio cloth sphere 1.72; with it
        // 0.91 – 0.96 and 1.23 (like its neighbours). Not on metal (the
        // transmission swallowed P8's metallic 0.75: 0.23) nor in weighted
        // mixing, whose weights `weighted_mix` already shares out.
        // Nor on a figure's thin-walled skin (the Genesis EyeSocket): with
        // the head behind it nothing comes through, and the corners of the
        // eyes went dark (Calib-Hair, #20).
        let transmit = if metal || refractive || (thin && !mat.prop) || mat.float("Base Mixing").unwrap_or(0.0) as i32 == 2 { 0.0 } else { translucency.min(self.max_transmission) };
        // The Genesis EyeSocket still reflects less of its diffuse: Iray
        // keeps (1 − w) × 0.41 of it at any translucency w (the caruncle:
        // 0.287 at w 0.3, 0.143 at 0.65; Calib-Eye, #20) — at the full
        // diffuse it rendered 4.6 × Iray. Measured on the socket only.
        let thin_skin = thin && !mat.prop && !metal && !refractive && translucency > 0.0 && is_eye_socket(name);
        let diffuse_weight = (1.0 - refraction) * (1.0 - transmit) * if thin_skin { (1.0 - translucency) * THIN_SKIN_DIFFUSE } else { 1.0 };

        // Iray "Base Mixing" 1 (specular/glossiness) ignores Metallic Weight;
        // a black base under a strong white gloss there is a mirror —
        // Octane makes that a metal coloured by the gloss.
        let specular_mode = mat.float("Base Mixing").unwrap_or(0.0) as i32 == 1;
        let gloss = mat.float("Glossy Layered Weight").unwrap_or(0.0);
        let gloss_color = mat.color("Glossy Color").unwrap_or([1.0; 3]);
        let max = |c: [f32; 3]| c.iter().copied().fold(0.0f32, f32::max);
        let mirror = specular_mode && max(mat.base_color()) < 0.05 && gloss > 0.5 && max(gloss_color) > 0.5;

        // Base colour: the diffuse map (scaled by the colour, see `tint`),
        // else the colour. Hair and hair caps often
        // keep a flat dark diffuse and get their colour from a translucency
        // map; without SSS that map is the closer albedo.
        let translucent_map = (mat.float("Translucency Weight").unwrap_or(0.0) > 0.0)
            .then(|| mat.image("Translucency Color"))
            .flatten();
        let diffuse = mat.image("diffuse").or(translucent_map).map(str::to_string);
        // Iray multiplies the diffuse map by the diffuse colour: a grey tint
        // (the vanity's drawers: a pattern map × 0.49) darkens it — left
        // out, the pattern rendered washed-out white. See `tinted_image_pin`.
        // Iray's weighted mixing (Base Mixing 2) without translucency blends
        // the diffuse and a full-strength glossy reflection by their weights
        // — Octane's metallic share does that (`weighted_gloss`); with
        // translucency (mesh hair) the diffuse map is brightened instead.
        let weighted = weighted_gloss(mat);
        let (tint, mix) = match (mat.image("diffuse").is_some(), weighted) {
            (true, None) => (mat.base_color(), self.weighted_mix(mat)),
            (true, Some(_)) => (mat.base_color(), 1.0),
            (false, _) => ([1.0; 3], 1.0),
        };
        // The weighted blend scales the diffuse map (not a translucency
        // map standing in for a missing one — the HS CB cap went grey).
        if mirror {
            self.rgb_pin(i, "albedo", gloss_color);
        } else {
            match diffuse.and_then(|t| self.textures.asset(self.dirs, &t)) {
                Some(asset) => self.tinted_image_pin(i, "albedo", &asset, diffuse_weight * mix, tint),
                None => self.rgb_pin(i, "albedo", mat.base_color().map(|c| c * diffuse_weight)),
            }
        }

        if transmit > 0.0 {
            let tc = mat.color("Translucency Color").unwrap_or([1.0; 3]);
            // An open sheet that isn't thin-walled (a leaf or grass card)
            // passes twice as much in Iray: Calib-Leaf's card 0.48 × Iray at
            // w, 0.81 at min(2w, 1); a closed sphere stays at w (1.25 – 1.38
            // at 2w).
            let share = if mat.open && !thin { (2.0 * transmit).min(1.0) } else { transmit };
            match mat.image("Translucency Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
                Some(asset) => self.tinted_image_pin(i, "transmission", &asset, share, tc),
                None => self.rgb_pin(i, "transmission", tc.map(|c| c * share)),
            }
            // Diffuse (1), or Thin Wall (Diffuse) (3) for a thin-walled sheet:
            // Calib-SSS rendered the two alike on closed spheres.
            self.value_pin(i, "transmissionType", 57, 2, if thin { "3" } else { "1" });
        }

        // Iray gives a surface a dielectric gloss only through its glossy
        // layer (or the dual-lobe specular of skin); at weight 0 it is matte.
        // Universal's default specular is a full white one — at roughness 0
        // over a normal map it made the matte slippers glitter (Octane1).
        let glossy = mat.float("Glossy Layered Weight").unwrap_or(0.0).max(mat.float("Glossy Weight").unwrap_or(0.0));
        let dual_lobe = mat.float("Dual Lobe Specular Weight").unwrap_or(0.0);
        if weighted.is_some() {
            self.rgb_pin(i, "specular", [0.0; 3]);
        } else if !mirror && refraction == 0.0 && glossy <= 0.0 && dual_lobe <= 0.0 && mat.image("Glossy Layered Weight").is_none() {
            self.rgb_pin(i, "specular", [0.0; 3]);
        } else if !mirror && glossy > 0.0 {
            // Iray's Glossy Reflectivity r scales the reflection (r 0.5 is
            // the default, Universal's full specular); Octane matched it at
            // specular × min(2r, 1) — Calib-Glossy, r 0 / 0.25: 2.42 / 1.54
            // × Iray at Universal's default, 1.21 / 1.14 so, like the matte
            // sphere's 1.13 (r 1: see `glossy_excess`).
            let refl = if specular_mode { 1.0 } else { reflectivity_share(mat.float("Glossy Reflectivity")) };
            let weight = glossy.min(1.0) * refl;
            // A coloured gloss map tints the reflection: the HS CB braids'
            // brown gloss rendered as a grey sheen with Universal's white
            // default (Octane1, probe 60).
            if let Some(asset) = mat.image("Glossy Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
                self.tinted_image_pin(i, "specular", &asset, weight, gloss_color);
            } else if let Some(asset) = mat.image("Glossy Layered Weight").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
                // The glossy layer's weight map (data) × its weight: the
                // GSOV4 leather's 0.33 gloss with a seam mask rendered 3.4×
                // Iray's sheen with Universal's full white one (Fetroom).
                self.image_pin(i, "specular", &asset, true, 1.0, weight);
            } else if weight < 1.0 || gloss_color != [1.0; 3] {
                self.rgb_pin(i, "specular", gloss_color.map(|c| c * weight));
            }
        } else if !mirror && refraction == 0.0 && dual_lobe > 0.0 {
            // The dual lobe specular alone: its reflectivity as the glossy
            // layer's, × its weight.
            let weight = dual_lobe.min(1.0) * reflectivity_share(mat.float("Dual Lobe Specular Reflectivity"));
            if weight < 1.0 {
                self.rgb_pin(i, "specular", [weight; 3]);
            }
        }

        let metallic = if mirror { 1.0 } else if specular_mode { 0.0 } else { mat.metallic() };
        // A trace of metallicity (the GSOV4 jacket's 0.0107) turned Octane's
        // gloss off — black velvet where Iray shows leather (Fetroom).
        if weighted.is_some() {
            // The diffuse part of a weighted mix: its gloss is the mix's
            // other material.
        } else if mirror || (!specular_mode && (metallic >= NEGLIGIBLE_METALLIC || mat.image("Metallic Weight").is_some())) {
            let ch = if mirror { None } else { Some("Metallic Weight") };
            self.float_or_map(i, "metallic", mat, ch, metallic.max(0.0));
        }

        // Iray refraction takes the glossy layer's colour and roughness unless
        // "Share Glossy Inputs" is off, then its own (Calib-Glass: Refraction
        // Color and Roughness did nothing while shared; unshared, roughness
        // 0.3 frosted the sphere). Universal has one roughness: a mostly
        // refractive surface gets the refraction's — unless thin-walled,
        // whose straight-through transmission the roughness doesn't blur: it
        // keeps the glossy layer's (the cornea and tear film: their sharp
        // refraction's 0 mirrored the room over the iris, Calib-Eye #20).
        let own_refraction = refraction > 0.0 && mat.float("Share Glossy Inputs").unwrap_or(1.0) == 0.0;
        let refraction_rough = own_refraction && refraction >= 0.5 && !thin;
        let (rough_ch, rough, inverted) = if refraction_rough {
            match (mat.float("Base Mixing").unwrap_or(0.0) as i32, mat.float("Refraction Glossiness")) {
                (1 | 2, Some(g)) => (Some("Refraction Glossiness"), 1.0 - g, true),
                _ => (Some("Refraction Roughness"), mat.float("Refraction Roughness").unwrap_or(0.0), false),
            }
        } else {
            mat.roughness_source()
        };
        // A glossiness map would need inverting; use its value only.
        let rough_ch = if inverted { None } else { rough_ch };
        let mut rough = iray_roughness(mat, rough, inverted);
        // Octane blurs a refraction ~3× more at the same roughness: Iray's
        // Refraction Roughness 0.3 (0.09 squared) matched Octane's 0.02 –
        // 0.03, not 0.09 (frosted) (Calib-Glass R6).
        let rough_ch = if refraction_rough {
            rough *= REFRACTION_ROUGHNESS_SCALE;
            None
        } else {
            rough_ch
        };
        self.float_or_map(i, "roughness", mat, rough_ch, rough.clamp(0.0, 1.0));

        // Iray's Top Coat: a clear dielectric layer over everything.
        let coat = mat.float("Top Coat Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        // A Glossy Reflectivity above 0.5 reflects more than Universal's full
        // specular, which caps at 1 (Calib-Glossy, r 1: 0.74 × Iray, also
        // at specular 2); the excess 2r − 1 as a clear coating at the gloss's
        // roughness brought it to 1.08, like its neighbours' 1.11 – 1.16.
        let excess = if mirror || specular_mode || glossy <= 0.0 || weighted.is_some() { 0.0 } else { glossy.min(1.0) * glossy_excess(mat.float("Glossy Reflectivity")) };
        if coat <= 0.0 && excess > 0.0 {
            self.rgb_pin(i, "coating", gloss_color.map(|c| c * excess));
            self.float_pin(i, "coatingRoughness", rough.clamp(0.0, 1.0));
        }
        // On a thin-walled refractive surface at least the glossy layer's
        // roughness: the cornea's sharp coat (0.0016) mirrored the room over
        // the iris even at its own reflectivity (IOR 1.37), and left out the
        // pupil came out 0.43 × Iray; blurred like its gloss it is Iray's
        // veil over the iris (pupil 0.89, iris 0.77 – 0.83; Calib-Eye, #20).
        self.top_coat(i, mat, if thin && refraction >= 0.5 { rough } else { 0.0 });

        // Glass-like surfaces (cornea, windows): Iray refraction → transmission,
        // tinted by the refraction's colour (see `own_refraction`).
        if refraction > 0.0 {
            let colour = mat.color(if own_refraction { "Refraction Color" } else { "Glossy Color" }).unwrap_or([1.0; 3]);
            self.rgb_pin(i, "transmission", colour.map(|c| c * refraction));
            let ior = mat.float("Refraction Index").unwrap_or(1.5);
            // Iray's thin-walled refraction reflects nothing of
            // its own — Calib-Glass's thin sphere was invisible; only a
            // glossy layer shows. IOR 1 takes Octane's Fresnel reflection away.
            self.float_pin(i, "ior", if thin && self.thin_glass_ior_one { 1.0 } else { ior });
            if thin {
                // Thin Wall (2): no bending — Iray's thin-walled glass sphere
                // showed the background straight through (Calib-Glass).
                self.value_pin(i, "transmissionType", 57, 2, "2");
            } else if let Some(sigma) = absorption(mat) {
                self.absorption_pin(i, sigma);
            }
            // Let light through for shadows (the iris behind a cornea).
            self.value_pin(i, "fake_shadows", 11, 1, "1");
        }

        // Surface detail.
        if let Some(asset) = mat.image("Normal Map").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            self.image_pin(i, "normal", &asset, false, 1.0, 1.0);
        }
        if let Some(asset) = mat.image("Bump Strength").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            self.image_pin(i, "bump", &asset, true, 1.0, 1.0);
        }

        self.emission(i, mat, area_m2);

        // Cutout: geometry shells are mostly transparent outside a mask.
        let opacity = mat.opacity().clamp(0.0, 1.0);
        if opacity < 1.0 || mat.image(mat.opacity_channel()).is_some() {
            self.float_or_map(i, "opacity", mat, Some(mat.opacity_channel()), opacity);
        }
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }
}

/// Universal's specular share for an Iray reflectivity r (default 0.5 → 1):
/// min(2r, 1). See the glossy layer in `universal`.
fn reflectivity_share(r: Option<f32>) -> f32 {
    (2.0 * r.unwrap_or(0.5)).clamp(0.0, 1.0)
}

/// Iray's volume absorption as an absorption coefficient per channel (1/m):
/// light keeps `Transmitted Color` (linear) after `Transmitted Measurement
/// Distance` (cm), so σ = −ln(colour) / distance. Black is Iray's default
/// and means none (Calib-Glass: the clear spheres stayed clear), as does
/// white or a zero distance.
fn absorption(mat: &Material) -> Option<[f32; 3]> {
    let colour = mat.color("Transmitted Color")?;
    let distance_m = mat.float("Transmitted Measurement Distance").unwrap_or(0.0) * CM_PER_M_INV;
    let (lo, hi) = colour.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &c| (lo.min(c), hi.max(c)));
    (distance_m > 0.0 && hi > 0.0 && lo < 1.0).then(|| colour.map(|c| -c.clamp(1e-4, 1.0).ln() / distance_m))
}

/// A figure's eye surface by its material name (Genesis 3 / 8 / 9: Irises,
/// Pupils, Sclera; Cornea and EyeMoisture are refractive anyway).
fn is_eye(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["iris", "irises", "pupil", "sclera"].iter().any(|e| n.ends_with(e) || n.ends_with(&format!("{e}s")) || n.contains(&format!("_{e}")))
}

/// A figure's eye socket by its material name (Genesis 3 / 8 / 9: EyeSocket).
fn is_eye_socket(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with("eyesocket")
}

/// See the roughness in `universal`.
const REFRACTION_ROUGHNESS_SCALE: f32 = 0.3;

/// The diffuse that a figure's thin-walled translucent skin (the EyeSocket)
/// reflects in Iray beyond its 1 − Translucency Weight. See `universal`.
const THIN_SKIN_DIFFUSE: f32 = 0.41;

/// Universal's `coatingIor` for Iray's Top Coat. Layering Mode 0
/// (Reflectivity, the most common: the cornea, 2128 materials in My
/// Library) reflects F0 = 0.08 × Reflectivity head-on (0.5 → 0.04, IOR
/// 1.5): Calib-TopCoat's Reflectivity 1 sphere 0.75 → 1.09 × Iray (0.5:
/// 1.11), and the cornea's 0.3 no longer mirrors the room over the iris
/// (Calib-Eye, #20). The other modes keep Top Coat IOR (#25).
fn top_coat_ior(mat: &Material) -> f32 {
    match mat.float("Top Coat Layering Mode").unwrap_or(0.0) as i32 {
        0 => ior_from_f0(0.08 * mat.float("Reflectivity").unwrap_or(0.5)),
        _ => mat.float("Top Coat IOR").unwrap_or(1.5),
    }
}

/// The dielectric IOR that reflects `f0` head-on: (1 + √F0) / (1 − √F0).
fn ior_from_f0(f0: f32) -> f32 {
    let s = f0.clamp(0.0, 0.9).sqrt();
    (1.0 + s) / (1.0 - s)
}

/// Octane Hair albedo from Iray's Blended Dual Lobe hair: its hair colour
/// and, more, its transmission colour (root / tip means, linear) — fitted on
/// the crown of Calib-Hair (blond) and two copies with dark brown hair or
/// transmission colours, strands at `LINE_WIDTH_SCALE`: 0.97 / 1.03 / 1.07 ×
/// Iray in luminance; Octane's Hair is linear in its albedo (× 0.6 → × 0.6).
const HAIR_COLOUR_SHARE: f32 = 0.046;
const HAIR_TRANSMISSION_SHARE: f32 = 0.110;

/// Meters per centimeter.
const CM_PER_M_INV: f32 = 0.01;

/// The reflectivity beyond Universal's full specular: 2r − 1 for r > 0.5.
fn glossy_excess(r: Option<f32>) -> f32 {
    (2.0 * r.unwrap_or(0.5) - 1.0).clamp(0.0, 1.0)
}

/// The scalar a map is scaled by for a grey Iray colour tint: its mean
/// (white → 1).
fn map_tint(c: [f32; 3]) -> f32 {
    ((c[0] + c[1] + c[2]) / 3.0).clamp(0.0, 1.0)
}

/// Iray's skin reflects only part of its diffuse map: the translucent share
/// `w` shows the map × `Translucency Color` (× its map's mean, `tc_map`), and
/// the whole comes out at half (`SKIN_ALBEDO_FACTOR`) — Calib-HDRI's
/// Genesis 8 Female skin measured (0.228, 0.121, 0.081) of linear albedo
/// against ½ · ((1 − w) · D + w · D · Tc) = (0.225, 0.116, 0.081), while
/// Standard Surface shows the map as is (3.2× Iray). The tint for the map,
/// per channel.
fn skin_tint(mat: &Material, w: f32, tc_map: Option<[f32; 3]>) -> [f32; 3] {
    let tc = mat.color("Translucency Color").unwrap_or([1.0; 3]);
    let tc = [0, 1, 2].map(|k| tc[k] * tc_map.map_or(1.0, |m| m[k]));
    [0, 1, 2].map(|k| SKIN_ALBEDO_FACTOR * ((1.0 - w) + w * tc[k]) * SKIN_HUE[k])
}

/// See [`skin_tint`].
const SKIN_ALBEDO_FACTOR: f32 = 0.5;

/// Octane's random-walk skin comes out bluer than Iray's at the same tint:
/// over nine regions of five figures (Genesis 8 Female, Fleur, Charity,
/// Victoria 8, Paisley; HDRI and Fetroom) green 1.25× and blue 1.6× Iray
/// relative to red. Iray's volume keeps less of them (#8).
const SKIN_HUE: [f32; 3] = [1.0, 0.8, 0.62];

/// Iray's weighted base mixing (2) without translucency: the glossy share
/// of the diffuse + glossy weights. Iray blends a full-strength glossy
/// reflection by it — a weighted sphere (diffuse 1, gloss 0.6) measured
/// 0.625 · its matte twin + 0.375 · the gloss colour × the mirrored sky,
/// where Universal's dielectric specular gave a tenth of its highlight
/// (Calib-Materials, #10). As a metallic share the highlight grew from 19
/// to 68 px (Iray 215) at the same 0.81 of Iray's brightness — neither a
/// white or gloss-coloured specular nor a larger share (0.6) closed that
/// (probe 85).
fn weighted_gloss(mat: &Material) -> Option<f32> {
    if mat.float("Base Mixing").unwrap_or(0.0) as i32 != 2 || mat.float("Translucency Weight").unwrap_or(0.0) > 0.0 {
        return None;
    }
    let gw = mat.float("Glossy Weight").unwrap_or(0.0).max(0.0);
    let dw = mat.float("Diffuse Weight").unwrap_or(1.0).max(0.0);
    (gw > 0.0 && dw + gw > 0.0).then(|| gw / (dw + gw))
}

/// Iray squares the roughness slider (`Roughness Squared`, and `Glossiness
/// Squared` for 1 − glossiness; both on by default); Octane takes it as is —
/// unsquared, Octane's highlights came out broader and dimmer
/// (Calib-Materials, #10).
fn iray_roughness(mat: &Material, r: f32, from_glossiness: bool) -> f32 {
    let channel = if from_glossiness { "Glossiness Squared" } else { "Roughness Squared" };
    if mat.float(channel).unwrap_or(1.0) > 0.0 { r * r } else { r }
}

/// Metallic Weight below this (without a map) is left out: see `universal`.
const NEGLIGIBLE_METALLIC: f32 = 0.05;

/// OpenPBR Surface rendered the same skin colour darker than Standard
/// Surface, evenly in R, G and B: 0.78 at translucency 0.5 (Genesis 8
/// Female, Calib-HDRI), 0.70 at 0.62 (Fleur, Octane4) — 1 − 0.46 · w (#8).
const OPENPBR_SKIN_DARKENING: f32 = 0.46;

/// Subsurface radius colour (max channel 1, softened) and scale (meters)
/// from Iray's volume channels; see [`Ocs::standard_surface`]. The scale is kept to
/// 0.5 mm – 2 cm.
fn sss_radius(mat: &Material) -> ([f32; 3], f32) {
    let d_t = mat.float("Transmitted Measurement Distance").filter(|d| *d > 0.0).unwrap_or(1.0);
    let d_s = mat.float("Scattering Measurement Distance").filter(|d| *d > 0.0).unwrap_or(0.1);
    let g = mat.float("SSS Direction").unwrap_or(0.0).clamp(-0.95, 0.95);
    let t = mat.color("Transmitted Color").unwrap_or([0.8, 0.5, 0.35]);
    let sigma_s = (1.0 - g) / d_s;
    let length_cm = t.map(|c| {
        let sigma_a = (-c.clamp(1e-3, 0.999).ln() / d_t).max(1e-4);
        1.0 / (3.0 * sigma_a * sigma_s).sqrt()
    });
    let max = length_cm.iter().copied().fold(0.0f32, f32::max);
    // Square root of the normalised lengths: strongly diverging channel
    // radii make random walk throw coloured fireflies (OTOY forum; Noemi's
    // 1 / 0.18 / 0.15 speckled), and Iray's thin-slab look is softer anyway.
    (length_cm.map(|l| (l / max).sqrt()), (max * 0.01).clamp(0.0005, 0.02))
}

/// A DAZ light as Octane sees it: Octane turns each UsdLux light into an
/// emitter mesh with a material input `/Scene/Lights/<prim>_light`, but gives
/// it a plain diffuse material — it doesn't glow.
#[derive(Debug, Clone, PartialEq)]
pub struct LightEmitter {
    /// UsdLux prim name under `/Scene/Lights`.
    pub prim: String,
    pub watts: f64,
    pub kelvin: Option<f64>,
    pub color: [f64; 3],
    /// A rect / disc light that shines from both faces ("Two Sided").
    pub two_sided: bool,
}

/// Material inputs that make Octane's light emitters glow: a black diffuse
/// material with a blackbody (or colour) emission of the light's power.
pub fn light_inputs(lights: &[LightEmitter], first_id: u32) -> String {
    let mut textures = Textures::default();
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs: &[], textures: &mut textures, tiling: None, in_mix: false, thin_glass_ior_one: false, max_transmission: 1.0 };
    for (k, l) in lights.iter().enumerate() {
        let id = w.id();
        let _ = writeln!(
            w.out,
            "   <node id='{id}' type='20007' name='/Scene/Lights/{}_light' position='{} -300'>",
            esc(&l.prim),
            k * 200
        );
        w.out.push_str("    <attr name='group' type='10'></attr>\n    <attr name='description' type='10'></attr>\n");
        w.pin_open(4, "input");
        let mid = w.id();
        let _ = writeln!(w.out, "     <node id='{mid}' type='17' name='{}'>", esc(&l.prim));
        let _ = writeln!(w.out, "      <attr name='compatibilityVersion' type='2'>{COMPAT}</attr>");
        w.rgb_pin(6, "diffuse", [0.0; 3]);
        let eid = w.id();
        w.pin_open(6, "emission");
        let blackbody = l.kelvin.is_some_and(|k| k > 0.0);
        let _ = writeln!(w.out, "       <node id='{eid}' type='{}'>", if blackbody { 53 } else { 54 });
        if blackbody {
            w.float_pin(8, "efficiency or texture", 1.0);
            w.value_pin(8, "temperature", 6, 8, &format!("{} 0 0", num(l.kelvin.unwrap_or(6500.0) as f32)));
            w.value_pin(8, "normalize", 11, 1, "1");
        } else {
            w.rgb_pin(8, "efficiency or texture", l.color.map(|c| c as f32));
        }
        let watts = l.watts * if l.two_sided { TWO_SIDED_POWER } else { 1.0 } * if blackbody { 1.0 } else { COLOUR_EMISSION_POWER };
        w.value_pin(8, "power", 6, 8, &format!("{} 0 0", num(watts as f32)));
        w.value_pin(8, "surfaceBrightness", 11, 1, "0");
        if l.two_sided {
            w.value_pin(8, "doubleSided", 11, 1, "1");
        }
        w.out.push_str("       </node>\n");
        w.pin_close(6);
        w.out.push_str("     </node>\n");
        w.pin_close(4);
        w.out.push_str("   </node>\n");
    }
    w.out
}

/// Octane disk power for a DAZ spot's watts (`DAZ_LUMENS_PER_WATT`): DAZ's
/// flux is the beam's, whatever its spread and exponent, and the normalised
/// soft disk keeps its flux too — the same 1500 lm beam measured 0.256
/// (Iray) and 0.29 (Octane at the old 0.00502 W), so 0.0443 at equal flux.
/// The fitted disk is a little narrower and lit the middle of the beam 1.14×
/// Iray at that (Calib-Spot, probe 75); trimmed to the lit middle (#7).
const SPOT_POWER_FACTOR: f64 = 0.039;

/// Octane's shadow catcher darkens a full shadow to black; Iray's ground
/// keeps the sky's fill — 11 % of the lit sand in Octane2's umbra (linear,
/// measured against its Iray render). One HDRI's sun/sky ratio, so a
/// calibration rather than physics.
const GROUND_SHADOW_OPACITY: f64 = 0.89;

/// How far (meters) the shadow-catching ground sits below y = 0: under a
/// room's floor, which then hides it like Iray's environment ground (at the
/// floor's height it z-fought and smeared shadows over it — Octane1).
const GROUND_DROP: f64 = 0.001;

/// How the materials translate: the variant's material fields of
/// `crate::tuning::Tuning`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shading {
    pub skin: SkinShader,
    pub thin_glass_ior_one: bool,
    pub max_transmission: f32,
}

impl Default for Shading {
    fn default() -> Self {
        SkinShader::None.into()
    }
}

impl From<SkinShader> for Shading {
    fn from(skin: SkinShader) -> Self {
        Shading { skin, thin_glass_ior_one: false, max_transmission: 1.0 }
    }
}

impl From<&crate::tuning::Tuning> for Shading {
    fn from(t: &crate::tuning::Tuning) -> Self {
        Shading { skin: t.skin, thin_glass_ior_one: t.thin_glass_ior_one, max_transmission: t.max_transmission }
    }
}

/// How skin (Iray translucency) renders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SkinShader {
    /// No subsurface: a Universal like any surface (preview kernel).
    #[default]
    None,
    /// Standard Surface (277) with random walk — closest to Iray.
    StandardSurface,
    /// OpenPBR Surface (505) with random walk, brightened to Iray's level
    /// (`OPENPBR_SKIN_DARKENING`); looks grittier.
    OpenPbr,
}

/// A DAZ spotlight as an Octane analytic light.
#[derive(Debug, Clone, PartialEq)]
pub struct OctaneSpot {
    pub name: String,
    pub watts: f64,
    pub kelvin: Option<f64>,
    pub color: [f64; 3],
    /// Full cone angle of the Octane disk, degrees (fitted to Iray's beam).
    pub spread_deg: f64,
    /// Emitter disk diameter, meters.
    pub diameter: f64,
    pub euler_deg: [f64; 3],
    pub rotation_order: String,
    pub translation: [f64; 3],
}

/// Octane's rotation order (transform node enum: XYZ, XZY, YXZ, YZX, ZXY,
/// ZYX) for a DAZ one. Octane names the matrix product left to right
/// ("XYZ" = Rx·Ry·Rz, Z applied first — measured with a Lua probe in Octane
/// X); DAZ names the axes in the order they apply ("XYZ" = Rz·Ry·Rx). So
/// the DAZ order reversed is the Octane name.
fn octane_rotation_order(daz: &str) -> u32 {
    let reversed: String = daz.chars().rev().collect();
    ["XYZ", "XZY", "YXZ", "YZX", "ZXY", "ZYX"].iter().position(|o| *o == reversed).unwrap_or(5) as u32
}

/// Nodes for the Scene graph: the spots as analytic disk lights (type 402),
/// Iray's shadow-catching ground (`ground` = its shadow intensity and
/// height, meters) as an infinite plane (110) with a shadow catcher
/// material (145), and
/// a geometry group (type 3) joining them with the USD import's output node
/// `usd_output`. Returns the XML and the node the render target's mesh pin
/// connects to (the group, or `usd_output` when there is nothing to add).
pub fn scene_lights(spots: &[OctaneSpot], ground: Option<(f64, f64)>, first_id: u32, usd_output: u32) -> (String, u32) {
    if spots.is_empty() && ground.is_none() {
        return (String::new(), usd_output);
    }
    let mut textures = Textures::default();
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs: &[], textures: &mut textures, tiling: None, in_mix: false, thin_glass_ior_one: false, max_transmission: 1.0 };
    let mut ids = Vec::new();
    for (k, l) in spots.iter().enumerate() {
        let id = w.id();
        ids.push(id);
        let _ = writeln!(w.out, "    <node id='{id}' type='402' name='{}' position='{} -450'>", esc(&l.name), 300 + k * 200);
        w.value_pin(5, "analyticLightType", 57, 2, "1"); // DISK
        w.value_pin(5, "analyticLightSpreadAngle", 6, 9, &format!("{} 0 0 0", num(l.spread_deg as f32)));
        // A soft cone, cos θ · cos²(θ · 90° / half): Iray's beam falls off
        // towards the edge (`octane_cone_half` fits the spread to it).
        w.value_pin(5, "analyticLightSpreadCutoffHardness", 6, 9, "0 0 0 0");
        w.value_pin(5, "normalize", 11, 1, "1");
        let d = num(l.diameter as f32);
        w.value_pin(5, "diskAnalyticLightSize", 6, 9, &format!("{d} {d} 0 0"));
        let eid = w.id();
        w.pin_open(5, "emission");
        let blackbody = l.kelvin.is_some_and(|k| k > 0.0);
        let _ = writeln!(w.out, "      <node id='{eid}' type='{}'>", if blackbody { 53 } else { 54 });
        if blackbody {
            w.float_pin(7, "efficiency or texture", 1.0);
            w.value_pin(7, "temperature", 6, 8, &format!("{} 0 0", num(l.kelvin.unwrap_or(6500.0) as f32)));
            w.value_pin(7, "normalize", 11, 1, "1");
        } else {
            w.rgb_pin(7, "efficiency or texture", l.color.map(|c| c as f32));
        }
        let watts = l.watts * SPOT_POWER_FACTOR;
        w.value_pin(7, "power", 6, 9, &format!("{} 0 0 0", num(watts as f32)));
        w.out.push_str("      </node>\n");
        w.pin_close(5);
        let tid = w.id();
        w.pin_open(5, "transform");
        let [rx, ry, rz] = l.euler_deg.map(num64);
        let [tx, ty, tz] = l.translation.map(num64);
        let _ = writeln!(w.out, "      <node id='{tid}' type='67'>");
        let _ = writeln!(w.out, "       <attr name='rotationOrder' type='2'>{}</attr>", octane_rotation_order(&l.rotation_order));
        let _ = writeln!(w.out, "       <attr name='rotation' type='8'>{rx} {ry} {rz}</attr>");
        // An analytic disk shines along its +Z, a DAZ spot along -Z.
        w.out.push_str("       <attr name='scale' type='8'>1 1 -1</attr>\n");
        let _ = writeln!(w.out, "       <attr name='translation' type='8'>{tx} {ty} {tz}</attr>");
        w.out.push_str("      </node>\n");
        w.pin_close(5);
        w.out.push_str("    </node>\n");
    }
    if let Some((opacity, height)) = ground {
        // Transparent but for the shadows cast on it, over the HDRI. Iray's
        // ground belongs to the environment, so a room's floor hides it; a
        // plane at the floor's height would z-fight with it instead, so it
        // sits GROUND_DROP below, where a floor at that height covers it.
        let id = w.id();
        ids.push(id);
        let _ = writeln!(w.out, "    <node id='{id}' type='4' name='DAZ ground' position='{} -450'>", 300 + spots.len() * 200);
        let tid = w.id();
        w.pin_open(5, "transform");
        let _ = writeln!(w.out, "      <node id='{tid}' type='67'>");
        let _ = writeln!(w.out, "       <attr name='translation' type='8'>0 {} 0</attr>", num64(height - GROUND_DROP));
        w.out.push_str("      </node>\n");
        w.pin_close(5);
        let pid = w.id();
        w.pin_open(5, "geometry");
        let _ = writeln!(w.out, "      <node id='{pid}' type='110' name='DAZ ground plane'>");
        let cid = w.id();
        w.pin_open(7, "groundColor");
        let _ = writeln!(w.out, "        <node id='{cid}' type='145' name='DAZ ground shadow'>");
        w.float_pin(9, "opacity", (opacity * GROUND_SHADOW_OPACITY) as f32);
        w.out.push_str("        </node>\n");
        w.pin_close(7);
        w.out.push_str("      </node>\n");
        w.pin_close(5);
        w.out.push_str("    </node>\n");
    }
    let gid = w.id();
    let _ = writeln!(w.out, "    <node id='{gid}' type='3' name='DAZ 3D lights' position='0 -450'>");
    let _ = writeln!(w.out, "     <attr name='pin_count' type='2'>{}</attr>", ids.len() + 1);
    w.out.push_str("     <attr name='userInstanceId' type='2'>-1</attr>\n");
    // Dynamic pins must say what they carry (12 = geometry), or Octane drops
    // them and their connections.
    let _ = writeln!(w.out, "     <pin name='Input 1' dynamicType='12' connect='{usd_output}'/>");
    for (k, id) in ids.iter().enumerate() {
        let _ = writeln!(w.out, "     <pin name='Input {}' dynamicType='12' connect='{id}'/>", k + 2);
    }
    w.out.push_str("    </node>\n");
    (w.out, gid)
}

/// The texture of the render target's environment (a texture environment,
/// type 37): the scene's HDRI as a linear image texture — wrapped
/// horizontally and shifted by the dome rotation — else white (Octane's
/// neutral grey sky).
pub fn environment_texture(
    env: Option<&crate::usd::SceneEnvironment>,
    dirs: &[PathBuf],
    textures: &mut Textures,
    first_id: u32,
) -> String {
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs, textures, tiling: None, in_mix: false, thin_glass_ior_one: false, max_transmission: 1.0 };
    let map = env.and_then(|e| e.map.clone()).and_then(|m| w.textures.asset(dirs, &m));
    match map {
        Some(asset) => {
            let id = w.id();
            let _ = writeln!(w.out, "        <node id='{id}' type='34'>");
            let _ = writeln!(w.out, "         <attr name='filename' type='11'>{}</attr>", esc(&asset));
            w.value_pin(9, "gamma", 6, 8, "1 0 0");
            w.value_pin(9, "borderModeU", 57, 2, "0");
            // The dome's rotation: a horizontal shift of the lat-long map.
            let shift = env.map(|e| e.dome_u_shift()).unwrap_or(0.25);
            let tid = w.id();
            w.pin_open(9, "transform");
            let _ = writeln!(w.out, "          <node id='{tid}' type='67'>");
            w.out.push_str("           <attr name='rotationOrder' type='2'>2</attr>\n");
            w.out.push_str("           <attr name='rotation' type='8'>0 0 0</attr>\n");
            w.out.push_str("           <attr name='scale' type='8'>1 1 1</attr>\n");
            let _ = writeln!(w.out, "           <attr name='translation' type='8'>{} 0 0</attr>", num(shift as f32));
            w.out.push_str("          </node>\n");
            w.pin_close(9);
            w.out.push_str("        </node>\n");
        }
        None => {
            let id = w.id();
            let _ = writeln!(w.out, "        <node id='{id}' type='33'>\n         <attr name='value' type='8'>1 1 1</attr>\n        </node>");
        }
    }
    w.out
}

/// Material input nodes (type 20007) for the USD graph, one per bound USD
/// material `(prim name, material)`, plus the textures they use. Node ids
/// start above `first_id`.
pub fn material_inputs(
    materials: &[crate::usd::BoundMaterial],
    dirs: &[PathBuf],
    layered: HashMap<String, crate::layered_image::LayeredImage>,
    first_id: u32,
    shading: impl Into<Shading>,
) -> (String, Textures) {
    let Shading { skin, thin_glass_ior_one, max_transmission } = shading.into();
    let mut textures = Textures { layered, ..Default::default() };
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs, textures: &mut textures, tiling: None, in_mix: false, thin_glass_ior_one, max_transmission };
    for (k, (name, mat, area)) in materials.iter().enumerate() {
        let id = w.id();
        let _ = writeln!(
            w.out,
            "   <node id='{id}' type='20007' name='/Scene/Materials/{}' position='{} -150'>",
            esc(name),
            k * 200
        );
        w.out.push_str("    <attr name='group' type='10'></attr>\n    <attr name='description' type='10'></attr>\n");
        w.pin_open(4, "input");
        w.universal(5, name, mat, *area, skin);
        w.pin_close(4);
        w.out.push_str("   </node>\n");
    }
    (w.out, textures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::rc::Rc;

    fn mat(chs: serde_json::Value) -> Rc<Material> {
        Rc::new(Material::from_dson(&json!({"extra": [{"channels": chs}]}), &HashMap::new()))
    }

    #[test]
    fn maps_iray_channels_to_universal_pins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        for f in ["skin.jpg", "rough.png", "nrm.jpg"] {
            std::fs::write(dir.path().join("Runtime/Textures").join(f), b"x").unwrap();
        }
        let skin = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Dual Lobe Specular Weight", "current_value": 0.25}},
            {"channel": {"id": "Specular Lobe 1 Roughness", "current_value": 0.7, "image_file": "/Runtime/Textures/rough.png"}},
            {"channel": {"id": "Normal Map", "current_value": 1, "image_file": "/Runtime/Textures/nrm.jpg"}},
            {"channel": {"id": "Bump Strength", "current_value": 0.5, "image_file": "/Runtime/Textures/missing.jpg"}},
        ]));
        let cornea = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0.5, 0.25, 0]}},
            {"channel": {"id": "Refraction Weight", "current_value": 1}},
            {"channel": {"id": "Refraction Index", "current_value": 1.38}},
            {"channel": {"id": "Cutout Opacity", "current_value": 0.5}},
        ]));
        let dirs = vec![dir.path().to_path_buf()];
        let cap = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0.44, 0.32, 0.15]}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
            {"channel": {"id": "Translucency Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
        ]));
        let (cx, _) = material_inputs(&[("Cap".into(), cap, 1.0)], &dirs, HashMap::new(), 7000, SkinShader::None);
        assert!(cx.contains("skin.jpg"), "hair colour from the translucency map");
        // Skin (Octane1's Zelie): Standard Surface with random-walk SSS.
        let sss = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
            {"channel": {"id": "Translucency Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/rough.png"}},
            {"channel": {"id": "Transmitted Color", "current_value": [0.7058824, 0.5568628, 0.4666667]}},
            {"channel": {"id": "Transmitted Measurement Distance", "current_value": 10}},
            {"channel": {"id": "Scattering Measurement Distance", "current_value": 0.5}},
            {"channel": {"id": "SSS Direction", "current_value": -0.6}},
        ]));
        let (sx, _) = material_inputs(&[("Torso".into(), sss.clone(), 1.0)], &dirs, HashMap::new(), 8000, SkinShader::StandardSurface);
        assert!(sx.contains("type='277' name='Torso'"), "{sx}");
        assert!(sx.contains("<pin name='baseColor'>") && sx.contains("<pin name='subsurfaceColor'>"));
        // Colour maps decode as sRGB, data maps (the rough.png translucency
        // stand-in is a colour map here) stay linear only with gamma 1.
        let base = &sx[sx.find("<pin name='baseColor'>").unwrap()..];
        let base = &base[..base.find("</pin>\n      <pin").unwrap_or(base.len())];
        assert!(base.contains("<attr name='colorSpace' type='2'>1</attr>"), "{base}");
        // Iray's skin shows ½ · ((1 − w) + w · Tc) of the map × SKIN_HUE:
        // w 0.5 and a white Translucency Color (its map unreadable here).
        assert!(base.contains("<attr name='value' type='8'>0.5 0.4 0.31</attr>"), "skin tint: {base}");
        let ssc = &sx[sx.find("<pin name='subsurfaceColor'>").unwrap()..];
        assert!(ssc[..ssc.find("</pin>\n      <pin").unwrap_or(ssc.len())].contains("<attr name='value' type='8'>0.5 0.4 0.31</attr>"), "{ssc}");
        // Octane keeps the pin's colour space only when told the image's was
        // chosen; every image node says so.
        assert_eq!(sx.matches("<attr name='initialColorSpaceSelected' type='1'>1</attr>").count(), sx.matches("<attr name='filename'").count());
        // OpenPBR: radius the distance, scale the colour; the tint lifted by
        // 1 / (1 − 0.46 · 0.5) to Standard Surface's level.
        let (ox, _) = material_inputs(&[("Torso".into(), sss.clone(), 1.0)], &dirs, HashMap::new(), 8100, SkinShader::OpenPbr);
        assert!(ox.contains("type='505' name='Torso'"), "{ox}");
        let radius_pin = &ox[ox.find("<pin name='radius'>").unwrap()..];
        assert!(radius_pin[..120].contains("type='6'>") && ox[ox.find("<pin name='scale'>").unwrap()..].contains("type='33'>"), "{ox}");
        assert!(ox.contains("<attr name='value' type='8'>0.649351 0.519481 0.402597</attr>"), "lifted tint: {ox}");
        let (radius, scale) = sss_radius(&sss);
        // Transmitted Color (0.706, 0.557, 0.467) is sRGB: linear 0.456, 0.271, 0.184.
        assert!((scale - 0.0115).abs() < 0.0005, "1.15 cm red diffusion length: {scale}");
        assert!(radius[0] == 1.0 && radius[1] < radius[0] && radius[2] < radius[1], "{radius:?}");
        let lashes = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
            {"channel": {"id": "Cutout Opacity", "current_value": 1, "image_file": "/Runtime/Textures/rough.png"}},
        ]));
        let (lx, _) = material_inputs(&[("Lashes".into(), lashes, 1.0)], &dirs, HashMap::new(), 8500, SkinShader::StandardSurface);
        assert!(lx.contains("type='130'"), "cut-out cards stay Universal");
        let shirt = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.3}},
            {"channel": {"id": "Thin Walled", "current_value": true}},
        ]));
        let (tx, _) = material_inputs(&[("Shirt".into(), shirt.clone(), 1.0)], &dirs, HashMap::new(), 8700, SkinShader::StandardSurface);
        assert!(tx.contains("type='130'"), "thin-walled fabric stays Universal");
        // The same as a figure's eye socket reflects (1 − 0.3) × 0.41 of its
        // diffuse map; the shirt all of it.
        let (ex, _) = material_inputs(&[("Genesis_8_Female_EyeSocket".into(), shirt, 1.0)], &dirs, HashMap::new(), 8720, SkinShader::StandardSurface);
        let power = |x: &str| x[x.find("<pin name='albedo'>").unwrap()..].split("<pin name='power'>").nth(1).unwrap().split("</attr>").next().unwrap().rsplit('>').next().unwrap().to_string();
        assert_eq!((power(&tx), power(&ex)), ("1".to_string(), "0.287".to_string()), "{ex}");
        let cloak = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
            {"channel": {"id": "Metallic Weight", "current_value": 0.9, "image_file": "/Runtime/Textures/rough.png"}},
        ]));
        let (kx, _) = material_inputs(&[("Cloak".into(), cloak, 1.0)], &dirs, HashMap::new(), 8750, SkinShader::StandardSurface);
        assert!(kx.contains("type='130'") && kx.contains("<pin name='metallic'>"), "metallic PBRSkin stays Universal: {kx}");
        let mut grass = (*sss).clone();
        grass.prop = true;
        let (gx, _) = material_inputs(&[("Grass".into(), Rc::new(grass), 1.0)], &dirs, HashMap::new(), 8760, SkinShader::StandardSurface);
        assert!(gx.contains("type='130'"), "a prop's translucency stays Universal");
        let bulbs = mat(json!([
            {"channel": {"id": "Emission Color", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Luminance", "current_value": 50000}},
            {"channel": {"id": "Luminance Units", "current_value": 4}},
            {"channel": {"id": "Emission Temperature", "current_value": 7000}},
        ]));
        let glow = mat(json!([
            {"channel": {"id": "Emission Color", "current_value": [1, 0, 0]}},
            {"channel": {"id": "Luminance", "current_value": 100}},
            {"channel": {"id": "Luminance Units", "current_value": 0}},
            {"channel": {"id": "Two Sided Light", "current_value": true}},
        ]));
        let (ex, _) = material_inputs(&[("Bulbs".into(), bulbs, 0.01), ("Glow".into(), glow, 2.0)], &dirs, HashMap::new(), 9000, SkinShader::None);
        assert!(ex.contains("<pin name='emission'>") && ex.contains("type='53'"), "blackbody bulbs");
        // 50 000 lm / 2 / 15 000 lm·W⁻¹ / 3.4.
        assert!(ex.contains("<attr name='value' type='8'>0.490196 0 0</attr>"), "50 000 lm → 0.49 W");
        assert!(ex.contains("<attr name='value' type='8'>7000 0 0</attr>"));
        // 100 cd/m² over 2 m²: 100 / 2 / 15 000 · 2 / 3.4 ≈ 0.002 W, red;
        // its second side gets as much without more power (`TWO_SIDED_POWER`).
        assert!(ex.contains("type='54'") && ex.contains("<attr name='value' type='8'>0.001961 0 0</attr>"), "{ex}");
        // A coloured blackbody: Iray multiplies by the colour (0.1 → linear
        // 0.00631).
        let trim = mat(json!([
            {"channel": {"id": "Emission Color", "current_value": [0.1, 0, 0.1]}},
            {"channel": {"id": "Luminance", "current_value": 1500}},
            {"channel": {"id": "Emission Temperature", "current_value": 6500}},
        ]));
        let (trimx, _) = material_inputs(&[("Trim".into(), trim, 1.0)], &dirs, HashMap::new(), 9100, SkinShader::None);
        assert!(trimx.contains("type='53'") && trimx.contains("<attr name='value' type='8'>0.00631 0 0.00631</attr>"), "{trimx}");
        let mirror = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0, 0, 0]}},
            {"channel": {"id": "Base Mixing", "current_value": 1}},
            {"channel": {"id": "Metallic Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Color", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0}},
        ]));
        // Leather: a 0.33 gloss through a weight map, a trace of metallicity.
        let leather = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0, 0, 0]}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 0.33, "image_file": "/Runtime/Textures/rough.png"}},
            {"channel": {"id": "Metallic Weight", "current_value": 0.0107}},
        ]));
        let (lx, _) = material_inputs(&[("Leather".into(), leather, 1.0)], &dirs, HashMap::new(), 8800, SkinShader::None);
        let spec = &lx[lx.find("<pin name='specular'>").expect("gloss weight map")..];
        assert!(spec.contains("type='36'") && spec[..900].contains("<attr name='value' type='6'>0.33</attr>"), "{spec}");
        assert!(!lx.contains("<pin name='metallic'>"), "a trace of metallicity is left out: {lx}");
        // Glossy Reflectivity 1: full specular plus the excess as a coating.
        let shiny = mat(json!([
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Reflectivity", "current_value": 1}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0.3}},
        ]));
        let (sx, _) = material_inputs(&[("Shiny".into(), shiny, 1.0)], &dirs, HashMap::new(), 8900, SkinShader::None);
        let coat = &sx[sx.find("<pin name='coating'>").expect("excess gloss as a coating")..];
        assert!(coat[..300].contains("1 1 1"), "{coat}");
        assert!(sx.contains("<pin name='coatingRoughness'>"), "{sx}");
        // A translucent prop: half its diffuse passes through.
        let wax = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
            {"channel": {"id": "Translucency Color", "current_value": [1, 1, 1]}},
        ]));
        let (wx, _) = material_inputs(&[("Wax".into(), wax, 1.0)], &dirs, HashMap::new(), 8950, SkinShader::None);
        assert!(wx.contains("<pin name='transmission'>") && wx.contains("<pin name='transmissionType'>"), "{wx}");
        let albedo = &wx[wx.find("<pin name='albedo'>").unwrap()..];
        assert!(albedo[..200].contains("0.5 0.5 0.5"), "{albedo}");
        // Weighted mixing: a Mix of a white metal (a third) and the diffuse.
        let weighted = mat(json!([
            {"channel": {"id": "Base Mixing", "current_value": 2}},
            {"channel": {"id": "Diffuse Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Weight", "current_value": 0.5}},
            {"channel": {"id": "Top Coat Weight", "current_value": 0.8}},
        ]));
        let (mx, _) = material_inputs(&[("Ceiling".into(), weighted, 1.0)], &dirs, HashMap::new(), 8960, SkinShader::None);
        assert!(mx.contains("type='19' name='Ceiling'") && mx.contains("<attr name='value' type='6'>0.333333</attr>"), "{mx}");
        let gloss = &mx[mx.find("<pin name='material1'>").unwrap()..mx.find("<pin name='material2'>").unwrap()];
        assert!(gloss.contains("<pin name='metallic'>") && gloss.contains("<pin name='coating'>"), "{gloss}");
        let base = &mx[mx.find("<pin name='material2'>").unwrap()..];
        assert!(!base.contains("<pin name='metallic'>") && base.contains("<pin name='coating'>"), "{base}");
        // Tinted glass with its own refraction inputs and a blue volume.
        let bottle = mat(json!([
            {"channel": {"id": "Refraction Weight", "current_value": 1}},
            {"channel": {"id": "Share Glossy Inputs", "current_value": false}},
            {"channel": {"id": "Refraction Color", "current_value": [1, 0.5, 1]}},
            {"channel": {"id": "Refraction Roughness", "current_value": 0.3}},
            {"channel": {"id": "Transmitted Color", "current_value": [0.5, 0.5, 1]}},
            {"channel": {"id": "Transmitted Measurement Distance", "current_value": 5}},
        ]));
        let (bx, _) = material_inputs(&[("Bottle".into(), bottle, 1.0)], &dirs, HashMap::new(), 8980, SkinShader::None);
        let tr = &bx[bx.find("<pin name='transmission'>").unwrap()..];
        assert!(tr[..200].contains("1 0.217638 1"), "refraction colour, linear: {tr}");
        assert!(bx.contains("<attr name='value' type='6'>0.027</attr>"), "refraction roughness, squared, × 0.3: {bx}");
        // −ln(0.5^2.2) / 0.05 m = 30.5 per m in R and G, none in B.
        let medium = &bx[bx.find("<pin name='medium'>").expect("absorption medium")..];
        assert!(medium.contains("type='58'") && medium.contains("30.49") && medium.contains("1 1 0"), "{medium}");
        // Thin-walled: no bending, no volume.
        let pane = mat(json!([
            {"channel": {"id": "Refraction Weight", "current_value": 1}},
            {"channel": {"id": "Thin Walled", "current_value": true}},
            {"channel": {"id": "Transmitted Color", "current_value": [0.5, 0.5, 1]}},
        ]));
        let (px, _) = material_inputs(&[("Pane".into(), pane, 1.0)], &dirs, HashMap::new(), 8990, SkinShader::None);
        assert!(px.contains("<pin name='transmissionType'>") && !px.contains("<pin name='medium'>"), "{px}");
        // A cornea: thin-walled, its own sharp refraction, a rough gloss and
        // a sharp Top Coat at Reflectivity 0.3 — the gloss's roughness for
        // both, the coat's IOR from its reflectivity.
        let thin_cornea = mat(json!([
            {"channel": {"id": "Refraction Weight", "current_value": 1}},
            {"channel": {"id": "Thin Walled", "current_value": true}},
            {"channel": {"id": "Share Glossy Inputs", "current_value": false}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0.3}},
            {"channel": {"id": "Top Coat Weight", "current_value": 1}},
            {"channel": {"id": "Top Coat Roughness", "current_value": 0.04}},
            {"channel": {"id": "Reflectivity", "current_value": 0.3}},
        ]));
        let (cx, _) = material_inputs(&[("Cornea".into(), thin_cornea, 1.0)], &dirs, HashMap::new(), 8995, SkinShader::None);
        let pin = |p: &str| cx[cx.find(&format!("<pin name='{p}'>")).unwrap()..].split("</attr>").next().unwrap().rsplit('>').next().unwrap().to_string();
        assert_eq!((pin("roughness"), pin("coatingRoughness")), ("0.09".to_string(), "0.09".to_string()), "{cx}");
        assert!(pin("coatingIor").starts_with("1.36"), "{cx}");
        // On an open card it passes twice the share.
        let leaf = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.25}},
            {"channel": {"id": "Translucency Color", "current_value": [1, 1, 1]}},
        ]));
        let leaf = Rc::new(crate::daz_material::Material { open: true, ..(*leaf).clone() });
        let (fx, _) = material_inputs(&[("Leaf".into(), leaf, 1.0)], &dirs, HashMap::new(), 8970, SkinShader::None);
        let tr = &fx[fx.find("<pin name='transmission'>").unwrap()..];
        assert!(tr[..200].contains("0.5 0.5 0.5"), "{tr}");
        let wall = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Horizontal Tiles", "current_value": 6}},
            {"channel": {"id": "Vertical Tiles", "current_value": 4}},
        ]));
        let drawer = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0.49, 0.49, 0.49], "image_file": "/Runtime/Textures/skin.jpg"}},
        ]));
        let (dx, _) = material_inputs(&[("Drawer".into(), drawer, 1.0)], &dirs, HashMap::new(), 6000, SkinShader::None);
        // A 0.49 grey = linear 0.208.
        assert!(dx.contains("<attr name='value' type='6'>0.208176</attr>") && !dx.contains("type='39'"), "map × grey tint: {dx}");
        let pubic = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0.447, 0.294, 0.145], "image_file": "/Runtime/Textures/skin.jpg"}},
        ]));
        let (px, _) = material_inputs(&[("Pubic".into(), pubic, 1.0)], &dirs, HashMap::new(), 6100, SkinShader::None);
        let albedo = &px[px.find("<pin name='albedo'>").unwrap()..];
        assert!(albedo.contains("type='39'") && albedo.contains("<pin name='texture1'>"), "map × coloured tint: {albedo}");
        assert!(albedo.contains("<pin name='texture2'>") && albedo.contains("<attr name='value' type='8'>0.170089 0.067665 0.014289</attr>"), "{albedo}");
        // DAZ Default shader: "Opacity Strength" 0 hides the group.
        let hiding = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "transparency", "current_value": 0}},
        ]));
        let (hx, _) = material_inputs(&[("Hiding".into(), hiding, 1.0)], &dirs, HashMap::new(), 6200, SkinShader::None);
        let op = &hx[hx.find("<pin name='opacity'>").expect("legacy opacity")..];
        assert!(op.contains("<attr name='value' type='6'>0</attr>"), "{op}");
        let (mx, _) = material_inputs(&[("Mirror".into(), mirror, 1.0), ("Wall".into(), wall, 1.0)], &dirs, HashMap::new(), 5000, SkinShader::None);
        assert!(mx.contains("<attr name='value' type='8'>1 1 1</attr>"), "mirror albedo = gloss colour");
        assert!(mx.contains("<pin name='metallic'>"));
        assert!(mx.contains("<attr name='scale' type='8'>0.166667 0.25 1</attr>"), "{mx}");
        // The wall has no glossy layer: matte (specular 0); the mirror keeps it.
        let wall_xml = &mx[mx.find("name='Wall'").unwrap()..];
        assert!(wall_xml.contains("<pin name='specular'>"), "{wall_xml}");
        assert_eq!(mx.matches("<pin name='specular'>").count(), 1, "only the wall");
        let (xml, tex) = material_inputs(&[("Body_Torso".into(), skin, 1.0), ("Body_Cornea".into(), cornea, 1.0)], &dirs, HashMap::new(), 1000, SkinShader::None);

        assert!(xml.contains("type='20007' name='/Scene/Materials/Body_Torso'"));
        assert!(xml.contains("type='130' name='Body_Torso'"));
        assert!(xml.contains("<attr name='filename' type='11'>assets/tex/0000_skin.jpg</attr>"));
        assert!(xml.contains("<attr name='filename' type='11'>assets/tex/0001_rough.png</attr>"));
        assert!(xml.contains("<pin name='normal'>"));
        assert!(!xml.contains("<pin name='bump'>"), "missing texture is left out");
        assert_eq!(tex.files.len(), 3);
        assert_eq!(tex.missing, ["/Runtime/Textures/missing.jpg"]);
        // Cornea: fully refractive, so no diffuse left; transmission with its
        // IOR, transparent shadows, half opacity.
        assert!(xml.contains("<attr name='value' type='8'>0 0 0</attr>"));
        assert!(xml.contains("<pin name='fake_shadows'>"));
        assert!(xml.contains("<pin name='transmission'>") && xml.contains("<pin name='ior'>") && xml.contains("<attr name='value' type='6'>1.38</attr>"));
        assert!(xml.contains("<pin name='opacity'>"));
        // Ids are unique.
        let ids: Vec<&str> = xml.match_indices("id='").map(|(i, _)| &xml[i + 4..i + 8]).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len());
        // Well-formed (balanced) XML fragment.
        assert_eq!(xml.matches("<node ").count(), xml.matches("</node>").count());
        assert_eq!(xml.matches("<pin ").count(), xml.matches("</pin>").count());
    }

    #[test]
    fn weighted_base_mixing_brightens_the_diffuse_map() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        // sRGB 188 ≈ linear 0.5, 99 ≈ 0.125.
        for (f, v) in [("dark.png", 99u8), ("light.png", 188)] {
            image::RgbImage::from_pixel(4, 4, image::Rgb([v, v, v])).save(dir.path().join("Runtime/Textures").join(f)).unwrap();
        }
        let hair = mat(json!([
            {"channel": {"id": "Base Mixing", "current_value": 2}},
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/dark.png"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
            {"channel": {"id": "Translucency Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/light.png"}},
            {"channel": {"id": "Glossy Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/light.png"}},
            {"channel": {"id": "Cutout Opacity", "current_value": 1, "image_file": "/Runtime/Textures/dark.png"}},
        ]));
        let (x, _) = material_inputs(&[("Braids".into(), hair, 1.0)], &[dir.path().to_path_buf()], HashMap::new(), 3000, SkinShader::None);
        // (1·0.125 + 0.5·0.5 + 1·0.5) / (2.5 · 0.125) ≈ 2.8: the diffuse map's power.
        let albedo = &x[x.find("<pin name='albedo'>").unwrap()..];
        let power = &albedo[albedo.find("<pin name='power'>").unwrap()..];
        let v: f32 = power[power.find("type='6'>").unwrap() + 9..].split('<').next().unwrap().parse().unwrap();
        assert!((v - 2.8).abs() < 0.05, "{v}");
        // Glossy Weight (not Glossy Layered Weight) keeps its gloss, tinted
        // by the glossy map.
        let spec = &x[x.find("<pin name='specular'>").expect("tinted gloss")..];
        assert!(spec[..200].contains("light.png"), "{spec}");
    }

    #[test]
    fn dforce_hair_shader_becomes_an_octane_hair_material() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        // sRGB 188 ≈ linear 0.5: the map's mean, linear.
        image::RgbImage::from_pixel(8, 8, image::Rgb([188, 188, 188])).save(dir.path().join("Runtime/Textures/brown.png")).unwrap();
        let hair = mat(json!([
            {"channel": {"id": "Hair Root Color", "current_value": [1, 0.5, 0.5], "image_file": "/Runtime/Textures/brown.png"}},
            {"channel": {"id": "Hair Tip Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/brown.png"}},
            {"channel": {"id": "Glossy Layer Weight", "current_value": 0.22}},
            {"channel": {"id": "base_roughness", "current_value": 0.35}},
        ]));
        let (hx, tex) = material_inputs(&[("Curls".into(), hair, 0.0)], &[dir.path().to_path_buf()], HashMap::new(), 4000, SkinShader::None);
        assert!(hx.contains("type='147' name='Curls'"), "{hx}");
        // Root 0.5 × (1, 0.218, 0.218) (the colour linear) and tip 0.5,
        // averaged, × 0.046, + 0.110 × the (default white) transmission colour.
        assert!(hx.contains("<attr name='value' type='8'>0.133134 0.124084 0.124084</attr>"), "{hx}");
        assert!(hx.contains("<pin name='specular'>") && hx.contains("<attr name='value' type='8'>0.22 0.22 0.22</attr>"));
        assert!(hx.contains("<attr name='value' type='6'>0.35</attr>"));
        assert!(tex.files.is_empty(), "the map is averaged, not packed");
        assert_eq!(hx.matches("<node ").count(), hx.matches("</node>").count());
    }
}
