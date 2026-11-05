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

use crate::daz_material::{colour_to_linear, Material};

/// Octane version written into each node's `compatibilityVersion`.
const COMPAT: &str = "15050000";

/// Universal's `diffuseBrdf` Oren-Nayar (enum 57; 0 / 1 render Lambertian,
/// the default; Calib-Diffuse variants, #32).
const DIFFUSE_BRDF_OREN_NAYAR: &str = "2";

/// Octane's `namedColorSpace` values (Lua `octane.namedColorSpace`).
const COLOUR_SPACE_SRGB: u32 = 1;
const COLOUR_SPACE_LINEAR_SRGB: u32 = 2;
/// "Linear sRGB + legacy gamma": the only colour space that reads an
/// image's `gamma` pin (sRGB ignores it).
const COLOUR_SPACE_LEGACY_GAMMA: u32 = 6;

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
    let watts = |lm: f64| lm / 2.0 / DAZ_LUMENS_PER_WATT * EMISSION_POWER_FACTOR;
    // The flux Iray gets (`crate::iray::uber`): a power, or a luminance per
    // `emission_scene_unit_size` (m) squared.
    if let Some(u) = iray(mat).filter(|u| u.bool("emission_enable") == Some(true)) {
        if let (Some(flux), Some(mode), Some(unit)) = (u.color("emission_flux"), u.mode("emission_mode"), u.float("emission_scene_unit_size")) {
            let flux = f64::from(flux[0]);
            return if mode == "intensity_power" { watts(flux) } else { watts(flux) / f64::from(unit * unit) * area_m2 };
        }
    }
    let l = mat.float("Luminance").unwrap_or(1500.0) as f64;
    let per = |factor: f64| watts(l * factor);
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
    /// A packed image's width and height (px) as the scene has it, before
    /// any texture budget scales it down.
    fn size(&self, asset: &str) -> Option<(u32, u32)> {
        if let Some((_, file)) = self.files.iter().find(|(a, _)| a == asset) {
            return crate::texture_budget::file_dimensions(file);
        }
        self.generated.iter().find(|(a, _)| a == asset).and_then(|(_, png)| crate::texture_budget::dimensions(png))
    }
}

impl Textures {
    /// An IES light profile, packed like an image: its package path and the
    /// parsed profile. DAZ keeps the path as picked — content-relative
    /// (`/Runtime/…`) or absolute (DAZ Studio's own
    /// `light_profile_test.ies` in its app bundle).
    pub fn ies(&mut self, dirs: &[PathBuf], daz_path: &str) -> Option<(String, crate::ies::Profile)> {
        let file = dirs.iter().map(|d| d.join(daz_path.trim_start_matches('/'))).find(|p| p.is_file()).or_else(|| Some(PathBuf::from(daz_path)).filter(|p| p.is_absolute() && p.is_file()));
        let Some(file) = file else {
            if !self.missing.iter().any(|m| m == daz_path) {
                self.missing.push(daz_path.to_string());
            }
            return None;
        };
        let profile = match std::fs::read(&file).map_err(anyhow::Error::from).and_then(|b| crate::ies::Profile::parse(&String::from_utf8_lossy(&b))) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(file = %file.display(), "Octane export: IES profile unreadable, left out: {e:#}");
                return None;
            }
        };
        if let Some(a) = self.by_source.get(daz_path) {
            return Some((a.clone(), profile));
        }
        let base = file.file_name().and_then(|n| n.to_str()).unwrap_or("profile.ies");
        let safe: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
        let asset = format!("assets/ies/{:02}_{safe}", self.files.len());
        self.files.push((asset.clone(), file));
        self.by_source.insert(daz_path.to_string(), asset.clone());
        Some((asset, profile))
    }

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
    /// Two colour maps mixed per pixel, `wa` · a + `wb` · b in linear light,
    /// written sRGB-encoded (gamma 2.2) at `a`'s size.
    fn blend(&mut self, dirs: &[PathBuf], a: &str, b: &str, wa: f32, wb: f32) -> Option<String> {
        let key = format!("blend|{a}|{b}|{wa}|{wb}");
        if let Some(asset) = self.by_source.get(&key) {
            return Some(asset.clone());
        }
        let open = |src: &str| dirs.iter().map(|d| d.join(src.trim_start_matches('/'))).find(|p| p.is_file()).and_then(|f| image::open(f).ok()).map(|i| i.to_rgb8());
        let (ia, ib) = (open(a)?, open(b)?);
        let ib = if ib.dimensions() == ia.dimensions() { ib } else { image::imageops::resize(&ib, ia.width(), ia.height(), image::imageops::FilterType::Triangle) };
        let lin = |c: u8| crate::daz_material::colour_to_linear(f32::from(c) / 255.0);
        let out = image::RgbImage::from_fn(ia.width(), ia.height(), |x, y| {
            let (pa, pb) = (ia.get_pixel(x, y).0, ib.get_pixel(x, y).0);
            image::Rgb([0, 1, 2].map(|k| ((wa * lin(pa[k]) + wb * lin(pb[k])).clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8))
        });
        let mut png = Vec::new();
        out.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
        let base = Path::new(a.trim_start_matches('/')).file_stem().and_then(|n| n.to_str()).unwrap_or("map");
        let safe: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        let asset = format!("assets/tex/{:04}_{safe}_blend.png", self.files.len() + self.generated.len());
        self.generated.push((asset.clone(), png));
        self.by_source.insert(key, asset.clone());
        Some(asset)
    }

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

    /// The skin's colour for OpenPBR's random walk, texel by texel, and its
    /// detail mask (see [`skin_albedo`]): from the diffuse map `D` and the
    /// translucency map `Tc` (resampled to `D`'s size; a missing one reads
    /// 1), each also low-passed (down to 32 px and back).
    /// - colour = `a · D + b · Tc · Tc_low` — Tc · Tc_low, not Tc²: squared,
    ///   the map's texture came out at twice its contrast (Calib-Skin);
    /// - × `SKIN_HUE_OPENPBR` to the power of the smooth colour's saturation
    ///   ÷ 0.75 — below 0.75 falling to 0 at `SKIN_HUE_SAT_FLOOR`: fitted on
    ///   saturated skins, a pale grey one takes less of it (Zurie came out
    ///   orange at full strength, 0.89 / 0.75 / 0.64 × Iray at sat ÷ 0.75);
    /// - × (smooth colour / 0.3)^`SKIN_DARK_GAIN`: the random walk brightens
    ///   dark colours against mid ones (Monique ×1.15 over her colour as a
    ///   plain diffuse, Victoria ×0.91); per texel it turned pores and
    ///   wrinkles grainy, from the smooth colour it shifts only the level;
    /// - the mask: D / D_low through a smoothstep 0.35 → 0.70, so marked
    ///   detail (brows, liner) leaves the random walk and pores stay in it.
    ///
    /// Linear, written as sRGB-gamma PNGs.
    fn derived_skin(&mut self, dirs: &[PathBuf], d: Option<&str>, tc: Option<&str>, key: &str, a: [f32; 3], b: [f32; 3]) -> Option<(String, Option<String>)> {
        if let Some(asset) = self.by_source.get(key) {
            return Some((asset.clone(), self.by_source.get(&format!("{key}#detail")).cloned()));
        }
        let open = |src: &str| {
            let rel = src.trim_start_matches('/');
            dirs.iter().map(|dir| dir.join(rel)).find(|p| p.is_file()).and_then(|f| image::open(f).ok()).map(|i| i.to_rgb8())
        };
        let d_img = d.and_then(open);
        let tc_img = tc.and_then(open);
        let (w, h) = d_img.as_ref().or(tc_img.as_ref()).map(|i| i.dimensions())?;
        let tc_img = tc_img.map(|i| if i.dimensions() == (w, h) { i } else { image::imageops::resize(&i, w, h, image::imageops::FilterType::Triangle) });
        let decode = |c: u8| crate::daz_material::colour_to_linear(f32::from(c) / 255.0);
        let encode = |v: f32| (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
        let low = |i: &image::RgbImage| {
            let small = image::imageops::resize(i, 32, 32, image::imageops::FilterType::Triangle);
            image::imageops::resize(&small, w, h, image::imageops::FilterType::Triangle)
        };
        let d_low = d_img.as_ref().map(low);
        let tc_low = tc_img.as_ref().map(low);
        let out = image::RgbImage::from_fn(w, h, |x, y| {
            let dp = d_img.as_ref().map_or([255; 3], |i| i.get_pixel(x, y).0);
            let tp = tc_img.as_ref().map_or([255; 3], |i| i.get_pixel(x, y).0);
            let dl = d_low.as_ref().map_or(dp, |i| i.get_pixel(x, y).0);
            let tl = tc_low.as_ref().map_or(tp, |i| i.get_pixel(x, y).0);
            let colour = |d: [u8; 3], t: [u8; 3], t2: [u8; 3]| [0, 1, 2].map(|k| a[k] * decode(d[k]) + b[k] * decode(t[k]) * decode(t2[k]));
            let (v, vl) = (colour(dp, tp, tl), colour(dl, tl, tl));
            let (lo, hi) = (vl.iter().copied().fold(f32::MAX, f32::min), vl.iter().copied().fold(0.0f32, f32::max));
            let sat = if hi > 0.0 { 1.0 - lo / hi } else { 0.0 };
            let e = if sat >= 0.75 { (sat / 0.75).min(1.3) } else { ((sat - SKIN_HUE_SAT_FLOOR) / (0.75 - SKIN_HUE_SAT_FLOOR)).max(0.0) };
            image::Rgb([0, 1, 2].map(|k| {
                let hue = SKIN_HUE_OPENPBR[k].powf(e);
                encode(v[k] * hue * ((vl[k] * hue).max(1e-4) / 0.3).powf(SKIN_DARK_GAIN))
            }))
        });
        let detail = d_img.as_ref().zip(d_low.as_ref()).map(|(di, dl)| {
            image::RgbImage::from_fn(w, h, |x, y| {
                let (p, q) = (di.get_pixel(x, y).0, dl.get_pixel(x, y).0);
                image::Rgb([0, 1, 2].map(|k| {
                    let t = ((decode(p[k]) / decode(q[k]).max(1e-3) - 0.35) / 0.35).clamp(0.0, 1.0);
                    encode(t * t * (3.0 - 2.0 * t))
                }))
            })
        });
        let base = d.or(tc).and_then(|s| Path::new(s).file_stem()).and_then(|n| n.to_str()).unwrap_or("skin");
        let safe: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        let mut store = |img: &image::RgbImage, suffix: &str, key: String| -> Option<String> {
            let mut png = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
            let asset = format!("assets/tex/{:04}_{safe}_{suffix}.png", self.files.len() + self.generated.len());
            self.generated.push((asset.clone(), png));
            self.by_source.insert(key, asset.clone());
            Some(asset)
        };
        let colour = store(&out, "skin", key.to_string())?;
        let detail = detail.and_then(|img| store(&img, "skin_detail", format!("{key}#detail")));
        Some((colour, detail))
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
            colour: mat.color("diffuse").unwrap_or([0.65; 3]),
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
            colour: mat.color("diffuse").unwrap_or([0.543, 0.53, 0.53]),
            weight: ("resources/grunge_b.jpg".into(), 0.5, 3.0),
            rough_u: rough.clone(),
            rough_v: rough,
            bump: (bump_map(mat, "grunge_d.jpg"), mat.float("Bump Strength").unwrap_or(1.5)),
            tiles: tiles.map(|t| t.unwrap_or(1.0)),
        });
    }
    None
}

/// Octane's `anisotropyTexture` for an MDL lobe of roughness `u` along
/// the tangent and `v` across, both on the GGX curve (`anisotropic_roughness`).
fn mdl_anisotropy(u: f32, v: f32) -> f32 {
    let (u, v) = (ggx_roughness(u.clamp(0.0, 1.0)), ggx_roughness(v.clamp(0.0, 1.0)));
    if (u - v).abs() < 1e-3 || u + v <= 0.0 {
        return 0.0;
    }
    ((v - u) / (v + u)).clamp(-1.0, 1.0)
}

/// Iray Uber / PBRSkin displacement: the map, Octane's height (m) and mid
/// level, and the subdivision level. Iray moves each vertex along its
/// normal by Strength × lerp(Minimum, Maximum, map) cm (Calib-Disp: the
/// white sphere 0..5 cm grew 5 cm, at Strength 0.5 2.5, black −5..5 shrank
/// 5) — Octane by height × (map − mid). Only with a map (Iray needs one).
pub(crate) struct Displacement {
    pub map: String,
    /// The map's gamma: its own `map_gamma`, else DAZ's automatic 2.2.
    pub gamma: f32,
    pub height_m: f32,
    pub mid: f32,
    pub subd: u32,
}

pub(crate) fn displacement(mat: &Material) -> Option<Displacement> {
    let map = mat.image("Displacement Strength")?.to_string();
    let strength = mat.float("Displacement Strength").unwrap_or(1.0);
    let (lo, hi) = (mat.float("Minimum Displacement").unwrap_or(-0.1), mat.float("Maximum Displacement").unwrap_or(0.1));
    let range = hi - lo;
    if strength <= 0.0 || range.abs() < 1e-6 {
        return None;
    }
    // SubD Displacement Level subdivides a SubD figure further for the
    // displacement; Iray ignores it on a plain mesh, and Octane would round
    // a prop's hard edges. Capped: a figure at level 3+ ran to millions of
    // faces per surface.
    let subd = if mat.prop { 0 } else { (mat.float("SubD Displacement Level").unwrap_or(0.0).max(0.0) as u32).min(DISPLACEMENT_SUBD_MAX) };
    let gamma = data_gamma(mat, "Displacement Strength");
    Some(Displacement { map, gamma, height_m: strength * range * 0.01, mid: (-lo / range).clamp(0.0, 1.0), subd })
}

/// The highest Octane subdivision level a displacement asks for, on top of
/// the levels the export refines a figure by (DAZ's SubD Level + Render SubD
/// Level, usually 2): 3 in all, as when the export refined once and this
/// was 2.
const DISPLACEMENT_SUBD_MAX: u32 = 1;

/// DAZ reads a float channel's 8-bit map without an image gamma of its own
/// (`map_gamma` 0, as every map in the user's scenes) at 2.2, though the
/// Uber channels declare `default_image_gamma` 1. Calib-Disp-Map's grey 128
/// moved the sphere by 22 % of its range (L or RGB PNG, JPEG alike), grey
/// 64 by 6 %, the same grey at `map_gamma` 1 by 50 %; Calib-DataGamma's
/// grey opacity, metallic and roughness maps rendered as 0.22, the same
/// map at `map_gamma` 1 as the plain 0.5 (#49). An image's own gamma wins.
const DATA_MAP_GAMMA: f32 = 2.2;

/// Octane bump `power` per unit of Iray's `file_bump_texture` factor
/// (Bump Strength, Top Coat Bump) for a map [`BUMP_REF_WIDTH`] wide, untiled,
/// on a surface of [`BUMP_REF_UV_SCALE`]: fitted on Calib-CoatBump (#61),
/// where ×2 matched Iray's dimples and ridges best (×1 rendered them faint,
/// ×3 too strong). Other maps: [`Ocs::bump_power`].
const BUMP_POWER: f32 = 2.0;

/// The width (px) of Calib-CoatBump's height maps, which `BUMP_POWER` fits.
const BUMP_REF_WIDTH: f32 = 512.0;

/// Calib-CoatBump's spheres' UV scale (m of surface per UV unit,
/// `usd::uv_scales`: 0.282 m² over 0.958 UV units²), which `BUMP_POWER`
/// fits.
const BUMP_REF_UV_SCALE: f64 = 0.542;

/// The gamma Iray reads a float channel's map at ([`DATA_MAP_GAMMA`]).
/// The colour space an image node reads its map in at `gamma`: linear at
/// 1, sRGB at 2.2 (the colour maps' and data maps' default), else linear
/// sRGB + legacy gamma, the only one where Octane applies the `gamma` pin.
fn image_colour_space(gamma: f32) -> u32 {
    if gamma <= 1.0 {
        COLOUR_SPACE_LINEAR_SRGB
    } else if (gamma - 2.2).abs() < 0.01 {
        COLOUR_SPACE_SRGB
    } else {
        COLOUR_SPACE_LEGACY_GAMMA
    }
}

fn data_gamma(mat: &Material, channel: &str) -> f32 {
    mat.image_gamma(channel).unwrap_or(DATA_MAP_GAMMA)
}

/// The MDL metal's bump map: DAZ hangs it on "Bump Strength" (the
/// amount), else `bump_texture`, else the MDL default.
fn bump_map(mat: &Material, default: &str) -> String {
    mat.image("Bump Strength").or(mat.image("bump_texture")).map(str::to_string).unwrap_or_else(|| format!("resources/{default}"))
}

/// The diffuse part of an NVIDIA MDL dielectric.
enum MdlAlbedo {
    Colour([f32; 3]),
    /// A colour map × a factor.
    Map(String, f32),
}

/// The reflection weight of an NVIDIA MDL dielectric's Fresnel layer.
enum MdlGloss {
    Weight(f32),
    /// A grey map (sRGB-decoded, as MDL reads an 8-bit PNG) × a factor.
    Map(String, f32),
    /// A colour map's luminance × a factor (`mono_average`).
    Luma(String, f32),
}

/// NVIDIA's dielectric MDL examples (DAZ's "NVIDIA MDL Examples" concrete,
/// porcelain, granite, …): `df::fresnel_layer(ior, weight, glossy, diffuse)`
/// — a gloss of that weight × Fresnel at `ior` over a diffuse base, as
/// Universal's specular and IOR. The MDL roughness is what the Uber's is
/// after its squaring, so it takes the Uber gloss's GGX curve.
struct MdlDielectric {
    albedo: MdlAlbedo,
    gloss: MdlGloss,
    /// The gloss's tint.
    tint: [f32; 3],
    roughness: f32,
    ior: f32,
    bump: Option<(String, f32)>,
    tiles: [f32; 2],
    /// Resin's base: a tinted glossy reflect + transmit (rough coloured
    /// glass) under the clear coat.
    transmission: Option<[f32; 3]>,
    metallic: bool,
    opacity: f32,
}

/// Resin's tinted base as Universal transmission: its colour × this.
const RESIN_TRANSMISSION: f32 = 0.8;

/// The NVIDIA dielectric a brick shader renders with, its parameters from
/// the shader's channels and the MDL defaults (`nvidia/<module>.mdl`).
fn mdl_dielectric(mat: &Material) -> Option<MdlDielectric> {
    const MODULES: [&str; 8] = ["concrete", "concrete_polished", "asphalt", "ceramic_tiles_small", "granite_black", "porcelain", "resin_polyurethane_matte", "architectural"];
    let module = MODULES.into_iter().find(|m| mat.uses_mdl(&format!("nvidia::{m}::{m}")))?;
    // As for the metals: a map channel the shader leaves out takes the
    // MDL default, one it sets empty is no texture.
    let map = |ch: &str, default: &str| match mat.channel(ch) {
        Some(c) => c.image.clone(),
        None => Some(format!("resources/{default}")),
    };
    let (default_tiles, diffuse, bump_default, bump_amount, rough, weight) = match module {
        "concrete" => (1.0, "concrete_plain.png", "concrete_plain_bump.png", 6.0, 0.35, 0.2),
        "concrete_polished" => (1.0, "concrete_fine_polished.png", "concrete_fine_polished_bump.png", 2.5, 0.15, 0.8),
        "asphalt" => (1.0, "asphalt.png", "asphalt_bump.png", 1.5, 0.25, 0.8),
        "ceramic_tiles_small" => (1.0, "ceramic_tiles_small.png", "ceramic_tiles_small_bump.png", 4.0, 0.0, 1.0),
        "granite_black" => (1.0, "granite_black.png", "", 0.0, 0.0, 1.0),
        "porcelain" => (1.0, "", "", 0.0, 0.1, 1.0),
        "resin_polyurethane_matte" => (1.0, "", "", 0.0, 0.25, 0.95),
        _ => (1.0, "", "", 0.0, 0.0, 0.6),
    };
    let tiles = [mat.float("Horizontal Tiles"), mat.float("Vertical Tiles")].map(|t| t.filter(|t| *t > 0.0).unwrap_or(default_tiles));
    let roughness = mat.float("roughness").unwrap_or(rough);
    let weight = mat.float("glossy_weight").or(mat.float("specular_weight")).unwrap_or(weight);
    let ior = mat.float("Index of Refraction").or(mat.float("coating_ior")).unwrap_or(if module == "ceramic_tiles_small" { 1.4 } else { 1.5 });
    let diffuse_map = map("Diffuse Color", diffuse);
    let colour_map = |scale: f32| diffuse_map.clone().map_or(MdlAlbedo::Colour([0.0; 3]), |m| MdlAlbedo::Map(m, scale));
    let bump_map = (!bump_default.is_empty()).then(|| bump_map(mat, bump_default));
    let bump = bump_map.clone().map(|m| (m, mat.float("Bump Strength").unwrap_or(bump_amount)));
    let mut d = MdlDielectric {
        albedo: colour_map(1.0),
        gloss: MdlGloss::Weight(weight),
        tint: [1.0; 3],
        roughness,
        ior,
        bump,
        tiles,
        transmission: None,
        metallic: false,
        opacity: 1.0,
    };
    match module {
        // The bump map tints the gloss.
        "concrete" => d.gloss = bump_map.map_or(MdlGloss::Weight(weight), |m| MdlGloss::Map(m, weight)),
        // The colour map, scaled by the reflectivity, is both the diffuse
        // tint and (its luminance) the gloss weight.
        "concrete_polished" => {
            d.albedo = colour_map(weight);
            d.gloss = diffuse_map.map_or(MdlGloss::Weight(0.0), |m| MdlGloss::Luma(m, weight));
        }
        "asphalt" | "ceramic_tiles_small" => d.gloss = bump_map.map_or(MdlGloss::Weight(weight), |m| MdlGloss::Map(m, weight)),
        // A perfect mirror varnish.
        "granite_black" => d.roughness = 0.0,
        "porcelain" => d.albedo = MdlAlbedo::Colour(mat.color("diffuse").unwrap_or([0.85; 3])),
        "resin_polyurethane_matte" => {
            d.albedo = MdlAlbedo::Colour([0.0; 3]);
            // At the full colour Octane's sphere came out 1.36 × Iray and
            // redder (g/r 0.30 v 0.37); × 0.85 matched the hue, × 0.6 let
            // too much background through (g/r 0.50) — Calib-NVIDIA.
            d.transmission = Some(mat.color("diffuse").unwrap_or([0.8, 0.104, 0.144]).map(|c| c * RESIN_TRANSMISSION));
        }
        _ => {
            // mental ray's mia_material: diffuse × weight; a reflection of
            // reflectivity × the colour's maximum, tinted by it; without
            // "Use Fresnel Term" Schlick's curve from the 0° reflectivity
            // (grazing 1, exponent 5 — the defaults) = Fresnel at the IOR
            // whose F0 that is.
            let diffuse = mat.color("diffuse").unwrap_or([0.5; 3]).map(|c| c * mat.float("Diffuse Weight").unwrap_or(1.0));
            let refl = mat.color("refl_color").unwrap_or([1.0; 3]);
            let peak = refl.iter().copied().fold(0.0f32, f32::max);
            d.albedo = MdlAlbedo::Colour(diffuse);
            d.gloss = MdlGloss::Weight(mat.float("reflectivity").unwrap_or(0.6) * peak);
            d.tint = refl.map(|c| if peak > 0.0 { c / peak } else { 1.0 });
            d.roughness = architectural_gloss_to_rough(mat.float("refl_gloss").unwrap_or(1.0));
            d.ior = if mat.float("brdf_fresnel").unwrap_or(0.0) > 0.0 {
                mat.float("Index of Refraction").unwrap_or(1.4)
            } else {
                let f0 = mat.float("brdf_0_degree_refl2").unwrap_or(0.2).clamp(0.0, 0.95).sqrt();
                (1.0 + f0) / (1.0 - f0)
            };
            d.metallic = mat.float("refl_is_metal").unwrap_or(0.0) > 0.0;
            let transparency = mat.float("transparency").unwrap_or(0.0);
            if transparency > 0.0 {
                d.transmission = Some(mat.color("refr_color").unwrap_or([1.0; 3]).map(|c| c * transparency));
            }
            d.opacity = mat.float("Cutout Opacity").unwrap_or(1.0);
            d.bump = mat.image("Bump Strength").map(|m| (m.to_string(), mat.float("Bump Strength").unwrap_or(1.0))).filter(|(_, a)| *a > 0.0);
            d.tiles = tiles;
        }
    }
    Some(d)
}

/// mental ray's glossiness → MDL roughness, as Iray's Arch+Design does
/// (`base::architectural_gloss_to_rough`, MDL SDK `base.mdl`): √(2 ·
/// 2^(−4 − 14 g)), a mirror at 1 — 0.35 at glossiness 0, 0.03 at 0.5.
fn architectural_gloss_to_rough(glossiness: f32) -> f32 {
    let g = glossiness.clamp(0.0, 1.0);
    if g >= 1.0 { 0.0 } else { (2.0 * (-4.0 - 14.0 * g).exp2()).sqrt() }
}

/// Octane's texture transform translation for Iray's UV tiles and offsets
/// (Calib-Map-Offset, #65): the offset is in tile units, so it is divided by
/// the tiles, and Octane's U runs against Iray's (V already matches).
fn uv_translation(tiles: [f32; 2], offsets: [f32; 2]) -> (f32, f32) {
    (-offsets[0] / tiles[0], offsets[1] / tiles[1])
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
    /// Writing the surface under a weighted top coat (`weighted_coat_material`):
    /// the mix carries the coat and the displacement.
    in_coat_mix: bool,
    /// Writing the base of an anisotropic Top Coat's Layered material
    /// (`layered_coat_material`): its Universal nodes leave the coating out.
    in_layered_coat: bool,
    /// Writing the surface under metallic flakes (`flake_mix_material`).
    in_flake_mix: bool,
    /// Writing the surface under a backscattering lobe (`backscatter_mix_material`).
    in_backscatter_mix: bool,
    /// Thin-walled glass at IOR 1 (`Tuning::thin_glass_ior_one`).
    thin_glass_ior_one: bool,
    /// `Tuning::max_transmission`.
    max_transmission: f32,
    /// The current material's Iray thin films (`Film::of`): `pin_open`
    /// multiplies its colour pins by their tint.
    film: Option<Film>,
    /// Per open pin, whether `pin_open` wrapped it in the film's Multiply.
    pin_wraps: Vec<bool>,
    /// Writing an Uber gloss as GGX (`gloss_roughness`).
    gloss_ggx: bool,
    /// The current material's UV scale (m per UV unit, `UvScales`).
    uv_scale: Option<f64>,
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

    /// Opens a pin; under a thin film (`Film`) a colour pin gets
    /// Multiply(film tint, what follows), closed again by `pin_close`.
    fn pin_open(&mut self, indent: usize, name: &str) {
        let _ = writeln!(self.out, "{:indent$}<pin name='{name}'>", "");
        let tint = if self.pin_wraps.contains(&true) { None } else { self.film.and_then(|f| f.tint(name)) };
        self.pin_wraps.push(tint.is_some());
        if let Some(t) = tint {
            self.film_tint_open(indent + 1, t);
        }
    }

    fn pin_close(&mut self, indent: usize) {
        if self.pin_wraps.pop() == Some(true) {
            let i = indent + 1;
            let _ = writeln!(self.out, "{:i$} </pin>\n{:i$}</node>", "", "");
        }
        let _ = writeln!(self.out, "{:indent$}</pin>", "");
    }

    /// Iray's (MDL) roughness `r` as Universal's under GGX: r + 1.85 r²,
    /// at most √r (they cross at r 0.36; above, the curve made Calib-Map-
    /// SpecMode's Glossiness 0.3 sphere, r 0.49 → 0.93, 0.53 × Iray, √r
    /// 0.7 0.86 with Iray's spread; Calib-DualLobe2's unsquared 0.6 lobe
    /// 0.77 → 1.03).
    /// Iray's Uber lobes are Phong with exponent 2 / r² (MDL
    /// `simple_glossy_bsdf`), Octane's default BSDF ("Octane") kept a sharp
    /// peak on rough highlights: Calib-DualLobe2's lobes at r 0.36 / 0.81
    /// had p99 / median 2.6 / 2.6 against Iray's 1.3 / 1.5 and rendered
    /// 1.58 / 1.99 × Iray. GGX at r is right for sharp highlights (r 0.09:
    /// Calib-Glossy's 2.5 against Iray's 2.6) but stays sharp above; GGX at
    /// √r matches the rough ones (1.4 / 1.5) but spreads and brightens the
    /// sharp (Calib-Glossy's Reflectivity 1 sphere 1.21 → 1.62). The curve
    /// passes both: 0.09 → 0.105, 0.36 → 0.6, 0.81 → 1. Calib-Glossy's
    /// spheres 1.02 – 1.16 × Iray (were 1.06 – 1.21), roughness 0.6 1.47 →
    /// 1.17; DualLobe2's rough lobes 1.02 – 1.25 (were 1.47 – 1.99), lobe 2
    /// at 0.81 0.83; SpecMode 0.95 – 1.10. Off: Roughness Squared off at 0.6
    /// (clamped at 1: 0.69) and Calib-ColorEffect's C8 (Base Mixing 1, red
    /// Glossy Specular: 1.35 → 1.77, #27). Beckmann stayed sharp (#56).
    fn gloss_roughness(&self, r: f32) -> f32 {
        if self.gloss_ggx { ggx_roughness(r) } else { r }
    }

    /// Universal's `brdf` GGX energy preserving (6; probe 104: 0 Octane,
    /// the default, 1 Beckmann, 2 GGX, 7 STD) while `gloss_ggx`. Plain GGX
    /// lost light on rough lobes, which Iray's Phong keeps: Calib-Glossy's
    /// roughness 0.6 1.17 → 1.01 × Iray, Calib-DualLobe2's r 0.36 lobe 1.25
    /// → 1.08, Calib-Map-SpecMode's Glossiness 0.3 0.41 → 0.53 (#56).
    fn brdf_pin(&mut self, i: usize) {
        self.aniso_brdf_pin(i, false);
    }

    /// `brdf_pin` for a lobe whose anisotropy may saturate: plain GGX (2)
    /// when Iray's long axis runs past roughness 1 (`Anisotropy::saturates`).
    /// Octane's energy-preserving GGX at roughness 1 adds the light of
    /// multiple scattering that Iray's lobe at r / 0.0001 doesn't have:
    /// Calib-Aniso2's sharp anisotropic metal K3 1.52 → 1.08 × Iray, the
    /// weighted coat K4 1.31 → 0.96; not below the cap (Calib-Aniso's
    /// metal at v 0.8 1.12 → 0.86, #23).
    fn aniso_brdf_pin(&mut self, i: usize, saturated: bool) {
        if self.gloss_ggx {
            self.value_pin(i, "brdf", 57, 2, if saturated { "2" } else { "6" });
        }
    }

    /// A Multiply texture (39) of Iray's film tint — a Mix (38) of the
    /// head-on and the grazing tint by a Falloff Map (50, normal vs. eye
    /// ray) — and the pin's own value, left open in `texture2`.
    fn film_tint_open(&mut self, i: usize, t: FilmTint) {
        let id = self.id();
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='39'>", "");
        self.pin_open(i + 1, "texture1");
        let mid = self.id();
        let j = i + 2;
        let _ = writeln!(self.out, "{:j$}<node id='{mid}' type='38'>", "");
        self.pin_open(j + 1, "amount");
        let fid = self.id();
        let k = j + 2;
        let _ = writeln!(self.out, "{:k$}<node id='{fid}' type='50'>", "");
        self.value_pin(k + 1, "normal", 6, 8, "0 0 0");
        self.value_pin(k + 1, "grazing", 6, 8, "1 0 0");
        self.value_pin(k + 1, "falloff index", 6, 8, &format!("{} 0 0", num(FILM_FALLOFF_SKEW)));
        let _ = writeln!(self.out, "{:k$}</node>", "");
        self.pin_close(j + 1);
        // Amount 1 is `texture2`: the other way round, Calib-ThinFilm's
        // spheres took the rim's tint in the middle.
        self.rgb_pin(j + 1, "texture1", t.normal);
        self.rgb_pin(j + 1, "texture2", t.grazing);
        let _ = writeln!(self.out, "{:j$}</node>", "");
        self.pin_close(i + 1);
        let _ = writeln!(self.out, "{:i$} <pin name='texture2'>", "");
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
        // Iray's weighted gloss has no Fresnel either: its colour to the edge.
        self.rgb_pin(j + 1, "edgeTint", gloss_color);
        self.float_pin(j + 1, "metallic", 1.0);
        self.gloss_ggx = true;
        let (rough_ch, rough, inverted) = mat.roughness_source();
        let rough = iray_gloss_roughness(mat).unwrap_or_else(|| iray_roughness(mat, rough, inverted));
        let aniso = Anisotropy::glossy(mat);
        self.aniso_brdf_pin(j + 1, aniso.saturates(rough));
        self.gloss_roughness_pins(j + 1, mat, rough_ch, rough, RoughCurve::Ggx, aniso);
        if let Some(asset) = mat.image("Normal Map").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            self.image_pin(j + 1, "normal", &asset, false, 1.0, 1.0);
        }
        self.base_bump_pin(j + 1, mat, "bump");
        self.round_edges_pin(j + 1, mat);
        // Iray's top coat lies over both parts: on the base alone a dark
        // weighted surface under a 0.8 coat lost half its sheen (Calib-Mixing
        // jamb 0.55 × Iray).
        self.top_coat(j + 1, mat, 0.0);
        self.gloss_ggx = false;
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
        if !self.in_coat_mix {
            self.displacement_pin(i, mat);
        }
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// Iray's metallic flakes (`add_metal_flakes` in DAZ's MDL): a
    /// `weighted_layer` of a glossy BSDF in the flakes' colour over the base
    /// (under the top coat), at Flakes Weight × a procedural flake noise,
    /// each flake with its own normal. Universal has no flakes: a Mix
    /// material (19) of a metal in the flakes' colour at a broad roughness
    /// (the flakes' scattered normals; their own roughness barely shows),
    /// under the same top coat, at `flake_share` (`material1`), and the
    /// surface without them (`material2`) (#30).
    fn flake_mix_material(&mut self, indent: usize, name: &str, mat: &Material, area_m2: f64, skin: SkinShader) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='19' name='{}'>", "", esc(name));
        let i = indent + 1;
        self.float_pin(i, "amount", flake_share(mat));
        self.pin_open(i, "material1");
        let fid = self.id();
        let j = i + 1;
        let _ = writeln!(self.out, "{:j$}<node id='{fid}' type='130' name='{} flakes'>", "", esc(name));
        let _ = writeln!(self.out, "{:j$} <attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        let (colour, _) = color_effect(mat, "Metallic Flakes Color Effect", mat.color("Metallic Flakes Color").unwrap_or([1.0; 3]), 1.0);
        self.rgb_pin(j + 1, "albedo", colour);
        self.rgb_pin(j + 1, "edgeTint", colour);
        self.float_pin(j + 1, "metallic", 1.0);
        self.gloss_ggx = true;
        self.brdf_pin(j + 1);
        self.float_pin(j + 1, "roughness", FLAKE_ROUGHNESS);
        self.top_coat(j + 1, mat, 0.0);
        self.gloss_ggx = false;
        self.round_edges_pin(j + 1, mat);
        let opacity = mat.opacity().clamp(0.0, 1.0);
        if opacity < 1.0 || mat.image(mat.opacity_channel()).is_some() {
            self.float_or_map(j + 1, "opacity", mat, Some(mat.opacity_channel()), opacity);
        }
        let _ = writeln!(self.out, "{:j$}</node>", "");
        self.pin_close(i);
        self.pin_open(i, "material2");
        self.in_flake_mix = true;
        self.universal(i + 1, &format!("{name} base"), mat, area_m2, skin);
        self.in_flake_mix = false;
        self.pin_close(i);
        self.displacement_pin(i, mat);
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// Iray's backscattering (DAZ's MDL): a `weighted_layer` at
    /// Backscattering Weight of `backscattering_glossy_reflection_bsdf` —
    /// light sent back toward where it came from — over diffuse and gloss
    /// (under metal, refraction, flakes and coat), tinted by Glossy Color
    /// while Share Glossy Inputs is on (DAZ doesn't even pass Backscattering
    /// Color then), else by its own colour. MDL's lobe is the
    /// retro-reflective part of a halfway-vector disk model (exponent
    /// 2 / r², no Fresnel, no multiscatter compensation). Octane X has no
    /// retro-reflective lobe (the manual's Retro-Reflective option isn't in
    /// this version): a Mix material (19) of a metal in that colour at full
    /// strength (`material1`, amount = the weight) whose shading normal is
    /// the direction to the viewer (`retro_normal_pin`) — a mirror about it
    /// sends light back where it came from, its lobe around that
    /// direction (Beckmann, `retro_roughness`, `retro_albedo`) —, and the
    /// surface, by the weight as MDL's `weighted_layer` (#29, #73).
    fn backscatter_mix_material(&mut self, indent: usize, name: &str, mat: &Material, area_m2: f64, skin: SkinShader) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='19' name='{}'>", "", esc(name));
        let i = indent + 1;
        let r = iray(mat).and_then(|u| u.float("backscattering_roughness")).unwrap_or_else(|| iray_roughness(mat, mat.float("Backscattering Roughness").unwrap_or(0.0), false));
        self.float_pin(i, "amount", backscatter_weight(mat));
        self.pin_open(i, "material1");
        let bid = self.id();
        let j = i + 1;
        let _ = writeln!(self.out, "{:j$}<node id='{bid}' type='130' name='{} backscatter'>", "", esc(name));
        let _ = writeln!(self.out, "{:j$} <attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        let shared = mat.float("Share Glossy Inputs").unwrap_or(1.0) > 0.0;
        let colour = if shared { mat.color("Glossy Color").unwrap_or([1.0; 3]) } else { mat.color("Backscattering Color").unwrap_or([0.5; 3]) };
        let colour = colour.map(|c| c * retro_albedo(r));
        self.rgb_pin(j + 1, "albedo", colour);
        self.rgb_pin(j + 1, "edgeTint", colour);
        self.float_pin(j + 1, "metallic", 1.0);
        self.gloss_ggx = true;
        // Beckmann: Phong-like tails, single scattering (DAZ leaves the
        // lobe's `multiscatter_tint` at 0).
        self.value_pin(j + 1, "brdf", 57, 2, "1");
        self.retro_normal_pin(j + 1);
        let rough = retro_roughness(r);
        self.float_pin(j + 1, "roughness", rough.clamp(0.0, 1.0));
        self.top_coat(j + 1, mat, 0.0);
        self.gloss_ggx = false;
        self.round_edges_pin(j + 1, mat);
        let opacity = mat.opacity().clamp(0.0, 1.0);
        if opacity < 1.0 || mat.image(mat.opacity_channel()).is_some() {
            self.float_or_map(j + 1, "opacity", mat, Some(mat.opacity_channel()), opacity);
        }
        let _ = writeln!(self.out, "{:j$}</node>", "");
        self.pin_close(i);
        self.pin_open(i, "material2");
        self.in_backscatter_mix = true;
        self.universal(i + 1, &format!("{name} base"), mat, area_m2, skin);
        self.in_backscatter_mix = false;
        self.pin_close(i);
        self.displacement_pin(i, mat);
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// The shading normal turned toward the viewer, so that the mirror
    /// about it sends each ray back where it came from: an OSL texture (117)
    /// in the `normal` pin giving the direction to the viewer (−I) in the
    /// frame Octane reads tangent-space normal maps in — T = dP/du made
    /// perpendicular to N, B = N × T (probe renders 2026-11-05: Ray
    /// Direction's own "tangent" space is another frame and rendered a
    /// distorted mirror; flipping or swapping its channels didn't help).
    /// Without UVs (dP/du ∥ N or zero) the normal stays unperturbed.
    fn retro_normal_pin(&mut self, indent: usize) {
        let id = self.id();
        self.pin_open(indent, "normal");
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='117' name='retro normal'>", "");
        let _ = writeln!(self.out, "{:i$} <attr name='shaderCode' type='10'>{}</attr>", "", esc(RETRO_NORMAL_OSL));
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// Iray's anisotropic Top Coat in Layering Modes 0, 2 and 3 (Universal's
    /// `coating` has no anisotropy): a Layered material (143) of the surface
    /// without its coat (`material1`) and a Specular Layer (139, `Layer 1`)
    /// in the coating's colour × weight, roughness and IOR, with the coat's
    /// anisotropy and rotation as on the gloss (`gloss_roughness_pins`).
    /// Calib-Aniso's A7 (mode 0, anisotropy 1) rendered a sharp isotropic
    /// coat, 1.22 × Iray, where Iray draws a long streak (#23). The pins
    /// (probe of 2026-11-04): `pin_count`, the dynamic `Layer N` pins of
    /// type 33.
    fn layered_coat_material(&mut self, indent: usize, name: &str, mat: &Material, area_m2: f64, skin: SkinShader) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='143' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='pin_count' type='2'>1</attr>", "");
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        self.pin_open(i, "material1");
        self.in_layered_coat = true;
        self.universal(i + 1, &format!("{name} base"), mat, area_m2, skin);
        self.in_layered_coat = false;
        self.pin_close(i);
        // The coat lies over the base's film.
        let film = self.film.take();
        let _ = writeln!(self.out, "{:i$}<pin name='Layer 1' dynamicType='33'>", "");
        let lid = self.id();
        let j = i + 1;
        let _ = writeln!(self.out, "{:j$}<node id='{lid}' type='139' name='{} top coat'>", "", esc(name));
        let _ = writeln!(self.out, "{:j$} <attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        let coat = mat.float("Top Coat Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        let (colour, coat) = color_effect(mat, "Top Coat Color Effect", mat.color("Top Coat Color").unwrap_or([1.0; 3]), coat);
        match mat.image("Top Coat Weight").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.image_pin(j + 1, "specular", &asset, true, 1.0, coat),
            None => self.rgb_pin(j + 1, "specular", colour.map(|c| c * coat)),
        }
        self.gloss_ggx = true;
        let (rough, aniso) = (top_coat_roughness(mat), Anisotropy::top_coat(mat));
        self.aniso_brdf_pin(j + 1, aniso.saturates(rough));
        self.gloss_ggx = false;
        self.gloss_roughness_pins(j + 1, mat, None, rough, RoughCurve::Ggx, aniso);
        self.float_pin(j + 1, "ior", top_coat_ior(mat));
        self.coat_bump_pins(j + 1, mat, "bump", "normal");
        self.round_edges_pin(j + 1, mat);
        let _ = writeln!(self.out, "{:j$}</node>", "");
        let _ = writeln!(self.out, "{:i$}</pin>", "");
        self.film = film;
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// Iray's Top Coat in Layering Mode 1 (Weighted): `weighted_layer` in
    /// DAZ's MDL — the coat's glossy reflection at the coat weight, with no
    /// Fresnel, over the rest at 1 − weight. A Mix material (19) of a
    /// mirror-like metal in the top coat colour and roughness (`material1`,
    /// `amount` = the weight or its map) and the surface without its coat
    /// (`material2`). As Universal's coating (Fresnel by IOR) Calib-TopCoat's
    /// weighted spheres rendered 0.05 – 0.10 × Iray: Iray's at weight 1 is a
    /// chrome mirror, Octane's plain lacquered red (#25).
    fn weighted_coat_material(&mut self, indent: usize, name: &str, mat: &Material, area_m2: f64, skin: SkinShader) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='19' name='{}'>", "", esc(name));
        let i = indent + 1;
        let coat = mat.float("Top Coat Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        let (colour, coat) = color_effect(mat, "Top Coat Color Effect", mat.color("Top Coat Color").unwrap_or([1.0; 3]), coat);
        match mat.image("Top Coat Weight").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.image_pin(i, "amount", &asset, true, 1.0, coat),
            None => self.float_pin(i, "amount", coat),
        }
        // The coat is over the base's film, under its own.
        let film = self.film;
        self.film = film.map(|f| Film { under: FilmTint::NONE, ..f });
        self.pin_open(i, "material1");
        let cid = self.id();
        let j = i + 1;
        let _ = writeln!(self.out, "{:j$}<node id='{cid}' type='130' name='{} top coat'>", "", esc(name));
        let _ = writeln!(self.out, "{:j$} <attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        match mat.image("Top Coat Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.tinted_image_pin(j + 1, "albedo", &asset, 1.0, colour),
            None => self.rgb_pin(j + 1, "albedo", colour),
        }
        // A weighted layer has no Fresnel: the colour to the edge, where
        // Octane's metal goes white (Calib-Map-TopCoat's red weighted coat:
        // G / B 1.57 / 4.29 × Iray).
        self.rgb_pin(j + 1, "edgeTint", colour);
        self.float_pin(j + 1, "metallic", 1.0);
        self.gloss_ggx = true;
        let (rough, aniso) = (top_coat_roughness(mat), Anisotropy::top_coat(mat));
        self.aniso_brdf_pin(j + 1, aniso.saturates(rough));
        self.gloss_ggx = false;
        self.gloss_roughness_pins(j + 1, mat, None, rough, RoughCurve::Ggx, aniso);
        self.coat_bump_pins(j + 1, mat, "bump", "normal");
        self.round_edges_pin(j + 1, mat);
        let opacity = mat.opacity().clamp(0.0, 1.0);
        if opacity < 1.0 || mat.image(mat.opacity_channel()).is_some() {
            self.float_or_map(j + 1, "opacity", mat, Some(mat.opacity_channel()), opacity);
        }
        let _ = writeln!(self.out, "{:j$}</node>", "");
        self.pin_close(i);
        self.film = film;
        self.pin_open(i, "material2");
        self.in_coat_mix = true;
        self.universal(i + 1, &format!("{name} base"), mat, area_m2, skin);
        self.in_coat_mix = false;
        self.pin_close(i);
        self.displacement_pin(i, mat);
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// An NVIDIA MDL metal (`mdl_metal`) as a Universal metal: albedo and
    /// edge tint = colour × weight map (MDL's tinted `simple_glossy_bsdf`
    /// has no Fresnel), each axis's roughness on the Uber gloss's GGX curve
    /// (the same Phong lobe), Octane's roughness the longer one per texel,
    /// anisotropy from the two at a mid-grey texel, the bump map at the
    /// Uber's bump power — all at the shader's tiles.
    fn mdl_metal(&mut self, indent: usize, name: &str, m: &MdlMetal, mat: &Material) {
        self.tiling = (m.tiles != [1.0, 1.0]).then_some([m.tiles[0], m.tiles[1], 0.0, 0.0]);
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='130' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        let (wmap, woff, wscale) = &m.weight;
        let colour = m.colour;
        let key = format!("mdl-metal-albedo|{wmap}|{woff}|{wscale}|{colour:?}");
        // `weighted_layer` clamps its weight to 0..1 (Cast Iron's 0.5 + 3 ·
        // map is mostly 1).
        let derived = if wmap.is_empty() { None } else { self.textures.derived(self.dirs, wmap, &key, 2.2, |l| colour.map(|c| c * (woff + wscale * l).clamp(0.0, 1.0))) };
        match derived {
            Some(asset) => {
                self.image_pin(i, "albedo", &asset, false, 2.2, 1.0);
                self.image_pin(i, "edgeTint", &asset, false, 2.2, 1.0);
            }
            // No texture: Iray's lookup reads 0 and the metal comes out black.
            None => {
                self.rgb_pin(i, "albedo", [0.0; 3]);
                self.rgb_pin(i, "edgeTint", [0.0; 3]);
            }
        }
        self.float_pin(i, "metallic", 1.0);
        let ((umap, uoff, uscale), (vmap, voff, vscale)) = (&m.rough_u, &m.rough_v);
        let (ru, rv) = (uoff + uscale * 0.5, voff + vscale * 0.5);
        // Plain GGX: `metal_generic`'s lobe has no multiple-scattering
        // compensation (MDL spec 21.1.3), and Metal Brushed's broad long
        // axis (0.6 – 0.7 on the curve) came out 1.16 × Iray under the
        // energy-preserving GGX, 1.03 × under plain GGX (the scene's matte
        // level 1.04); Cast Iron 1.11 either way.
        self.value_pin(i, "brdf", 57, 2, "2");
        // Octane's roughness is the long axis, its anisotropy narrows the
        // other (`anisotropic_roughness`, #23).
        let rough = |l: f32| ggx_roughness((uoff + uscale * l).max(voff + vscale * l).clamp(0.0, 1.0));
        let key = format!("mdl-metal-rough-ggx|{umap}|{uoff}|{uscale}|{voff}|{vscale}");
        let derived = if umap.is_empty() { None } else { self.textures.derived(self.dirs, umap, &key, 1.0, |l| [rough(l); 3]) };
        match derived {
            Some(asset) => self.image_pin(i, "roughness", &asset, true, 1.0, 1.0),
            None => self.float_pin(i, "roughness", rough(0.5)),
        }
        // The anisotropy per texel too: Metal Brushed's map runs u 0.04 –
        // 0.18 against v 0.36 – 0.48, and its darkest texels draw Iray's
        // thin streak (at the mid-grey texel's ratio it came out broad).
        let aniso = |l: f32| mdl_anisotropy(uoff + uscale * l, voff + vscale * l);
        if (0..=4).any(|k| aniso(k as f32 / 4.0) != 0.0) {
            let key = format!("mdl-metal-aniso|{umap}|{uoff}|{uscale}|{voff}|{vscale}");
            // Octane's anisotropy runs −1 – 1; the map holds it as is (≥ 0
            // where v is the long axis, as in both NVIDIA metals).
            // One map drives both axes (the presets); with two, the mid-grey ratio.
            let derived = if umap.is_empty() || umap != vmap { None } else { self.textures.derived(self.dirs, umap, &key, 1.0, |l| [aniso(l).max(0.0); 3]) };
            match derived {
                Some(asset) => self.image_pin(i, "anisotropyTexture", &asset, true, 1.0, 1.0),
                None => self.float_pin(i, "anisotropyTexture", mdl_anisotropy(ru, rv)),
            }
        }
        let (bmap, amount) = &m.bump;
        if *amount > 0.0 {
            if let Some(asset) = self.textures.asset(self.dirs, bmap) {
                let p = self.bump_power(&asset, *amount);
                self.image_pin(i, "bump", &asset, true, 1.0, p);
            }
        }
        self.round_edges_pin(i, mat);
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// An NVIDIA MDL dielectric (`mdl_dielectric`) as Universal: albedo,
    /// specular = the gloss weight (map) × tint at the layer's IOR, its
    /// roughness and bump, all at the shader's tiles. Resin's red base is
    /// transmission with the clear gloss as the coating.
    fn mdl_dielectric(&mut self, indent: usize, name: &str, m: &MdlDielectric, mat: &Material) {
        self.tiling = (m.tiles != [1.0, 1.0]).then_some([m.tiles[0], m.tiles[1], 0.0, 0.0]);
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='130' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        match &m.albedo {
            MdlAlbedo::Map(map, scale) => match self.textures.asset(self.dirs, map) {
                Some(asset) => self.image_pin(i, "albedo", &asset, false, 2.2, *scale),
                None => self.rgb_pin(i, "albedo", [0.0; 3]),
            },
            MdlAlbedo::Colour(c) => self.rgb_pin(i, "albedo", *c),
        }
        if m.metallic {
            self.float_pin(i, "metallic", 1.0);
        }
        let (gloss_pin, roughness_pin) = if m.transmission.is_some() { ("coating", "coatingRoughness") } else { ("specular", "roughness") };
        match &m.gloss {
            MdlGloss::Weight(w) => self.rgb_pin(i, gloss_pin, m.tint.map(|c| c * w)),
            MdlGloss::Map(map, w) => match self.textures.asset(self.dirs, map) {
                Some(asset) => self.image_pin(i, gloss_pin, &asset, true, 2.2, *w),
                None => self.rgb_pin(i, gloss_pin, [0.0; 3]),
            },
            MdlGloss::Luma(map, w) => {
                let w = *w;
                match self.textures.derived(self.dirs, map, &format!("mdl-luma|{map}|{w}"), 1.0, |l| [(l * w).clamp(0.0, 1.0); 3]) {
                    Some(asset) => self.image_pin(i, gloss_pin, &asset, true, 1.0, 1.0),
                    None => self.rgb_pin(i, gloss_pin, [0.0; 3]),
                }
            }
        }
        // The MDL's `simple_glossy_bsdf` is the Uber's Phong lobe: GGX
        // energy preserving on the gloss's curve (the Uber's rule). Not on
        // resin's coated rough glass: there the rough-glass rule (Beckmann
        // at √r) came out redder and darker in the middle (g/r 0.28 v
        // Iray's 0.22, centre 0.83 ×) than Octane's default BSDF at r as is
        // (0.24, 0.92 ×; Calib-NVIDIA).
        let curve = if m.transmission.is_some() { RoughCurve::Linear } else { RoughCurve::Ggx };
        if m.transmission.is_none() {
            self.value_pin(i, "brdf", 57, 2, "6");
        }
        let rough = curve.apply(m.roughness.clamp(0.0, 1.0));
        self.float_pin(i, roughness_pin, rough);
        if let Some(t) = m.transmission {
            // The base reflects its tint and lets it through (rough glass).
            self.rgb_pin(i, "specular", t);
            self.float_pin(i, "roughness", rough);
            self.rgb_pin(i, "transmission", t);
            self.float_pin(i, "coatingIor", m.ior);
        }
        self.float_pin(i, "ior", m.ior);
        if let Some((map, amount)) = &m.bump {
            if *amount > 0.0 {
                if let Some(asset) = self.textures.asset(self.dirs, map) {
                    let p = self.bump_power(&asset, *amount);
                    self.image_pin(i, "bump", &asset, true, 1.0, p);
                }
            }
        }
        self.round_edges_pin(i, mat);
        if m.opacity < 1.0 {
            self.float_pin(i, "opacity", m.opacity.max(0.0));
        }
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// Iray's Top Coat: a clear dielectric layer over everything, as
    /// Universal's coating.
    fn top_coat(&mut self, i: usize, mat: &Material, min_roughness: f32) {
        let coat = mat.float("Top Coat Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        if coat <= 0.0 || self.in_coat_mix || self.in_layered_coat {
            return;
        }
        let (colour, coat) = color_effect(mat, "Top Coat Color Effect", mat.color("Top Coat Color").unwrap_or([1.0; 3]), coat);
        // A weight map (data) × the weight, as for the glossy layer:
        // Gabriela 8's face coats only where hers is light (Calib-Skin).
        match mat.image("Top Coat Weight").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.image_pin(i, "coating", &asset, true, 1.0, coat),
            None => self.rgb_pin(i, "coating", colour.map(|c| c * coat)),
        }
        let rough = self.gloss_roughness(top_coat_roughness(mat));
        self.float_pin(i, "coatingRoughness", rough.max(min_roughness).clamp(0.0, 1.0));
        self.float_pin(i, "coatingIor", top_coat_ior(mat));
        self.coat_bump_pins(i, mat, "coatingBump", "coatingNormal");
    }

    /// Octane's image `power` for a height map that Iray reads through
    /// `base::file_bump_texture` at `factor` (#75). Iray's slope is the
    /// height difference of the texels either side (MDL SDK's `base.mdl`),
    /// on unit tangents: the tilt goes with factor / the map's width, the
    /// same at any UV tiling or object size. Octane's goes with power × the
    /// height's slope per meter of surface, whatever the map's resolution
    /// (Calib-BumpRes, Info kernel normals: 256 / 1024 / 4096 px alike,
    /// tiles 2 or the sphere at half size as at twice the power). So the
    /// power scales with the reference width / the map's, the material's
    /// UV scale / the reference's, and 1 / the tiles.
    fn bump_power(&self, asset: &str, factor: f32) -> f32 {
        let width = self.textures.size(asset).map_or(BUMP_REF_WIDTH, |(w, h)| ((w as f32) * (h as f32)).sqrt().max(1.0));
        let tiles = self.tiling.map_or(1.0, |[tu, tv, ..]| (tu * tv).abs().sqrt().max(1e-3));
        let size = self.uv_scale.map_or(1.0, |s| (s / BUMP_REF_UV_SCALE) as f32);
        factor * BUMP_POWER * BUMP_REF_WIDTH / width * size / tiles
    }

    /// The Top Coat's surface detail on the coat's `bump` / `normal` pins
    /// (#61). DAZ's MDL gives the coat its own normal
    /// (`calc_mix_layer_normal`): the Top Coat Bump — a height map, or a
    /// normal map in Top Coat Bump Mode 1 — blended with the base's bump,
    /// else the base's bump or normal map. Octane's coating takes the
    /// plain shading normal unless its own pins are set (Calib-CoatBump:
    /// the base's ridges stayed out of the coat's highlights), so the base
    /// maps go there too. Height maps at `bump_power` of their factor; the
    /// coat's and the base's height maps together as their sum (a Mix
    /// texture at ½ of both at twice the power: Iray's B7 shows both).
    fn coat_bump_pins(&mut self, i: usize, mat: &Material, bump_pin: &str, normal_pin: &str) {
        let asset = |s: &mut Self, id: &str| mat.image(id).map(str::to_string).and_then(|t| s.textures.asset(s.dirs, &t));
        let coat = asset(self, "Top Coat Bump");
        let coat_normal = mat.float("Top Coat Bump Mode").unwrap_or(0.0) as i32 == 1;
        let coat_height = coat.clone().filter(|_| !coat_normal).map(|a| (a, mat.float("Top Coat Bump").unwrap_or(1.0))).filter(|(_, f)| *f > 0.0);
        match (coat_height, self.base_bump(mat)) {
            (Some((c, cf)), Some((b, bf))) => {
                let id = self.id();
                self.pin_open(i, bump_pin);
                let j = i + 1;
                let _ = writeln!(self.out, "{:j$}<node id='{id}' type='38'>", "");
                self.float_pin(j + 1, "amount", 0.5);
                let (cp, bp) = (self.bump_power(&c, cf), self.bump_power(&b, bf));
                self.image_pin(j + 1, "texture1", &c, true, 1.0, 2.0 * cp);
                self.image_pin(j + 1, "texture2", &b, true, 1.0, 2.0 * bp);
                let _ = writeln!(self.out, "{:j$}</node>", "");
                self.pin_close(i);
            }
            (Some((a, f)), None) | (None, Some((a, f))) => {
                let p = self.bump_power(&a, f);
                self.image_pin(i, bump_pin, &a, true, 1.0, p)
            }
            (None, None) => {}
        }
        match (&coat, coat_normal) {
            (Some(a), true) => self.image_pin(i, normal_pin, a, false, 1.0, 1.0),
            _ => {
                if let Some(a) = asset(self, "Normal Map") {
                    self.image_pin(i, normal_pin, &a, false, 1.0, 1.0);
                }
            }
        }
    }

    /// Iray Uber's base height map: Bump Strength's map and its value, the
    /// factor DAZ hands Iray's `file_bump_texture` (none at 0).
    fn base_bump(&mut self, mat: &Material) -> Option<(String, f32)> {
        let f = mat.float("Bump Strength").unwrap_or(1.0);
        mat.image("Bump Strength").filter(|_| f > 0.0).map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)).map(|a| (a, f))
    }

    /// The base's height map on `pin` at `bump_power` of Bump Strength.
    fn base_bump_pin(&mut self, i: usize, mat: &Material, pin: &str) {
        if let Some((a, f)) = self.base_bump(mat) {
            let p = self.bump_power(&a, f);
            self.image_pin(i, pin, &a, true, 1.0, p);
        }
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

    /// A scattering medium (59; probe 105) on `medium`: absorption and
    /// scattering coefficients (1/m per channel) as colours × one `scale`.
    /// The phase stays isotropic (SSS Direction isn't mapped).
    fn scattering_pin(&mut self, indent: usize, sigma_a: [f32; 3], sigma_s: [f32; 3]) {
        let scale = sigma_a.iter().chain(&sigma_s).copied().fold(0.0f32, f32::max).max(1e-6);
        let id = self.id();
        self.pin_open(indent, "medium");
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='59'>", "");
        self.value_pin(i + 1, "scale", 6, 9, &format!("{} 0 0 0", num(scale)));
        self.rgb_pin(i + 1, "absorption", sigma_a.map(|s| s / scale));
        self.value_pin(i + 1, "invertAbsorption", 11, 1, "0");
        self.rgb_pin(i + 1, "scattering", sigma_s.map(|s| s / scale));
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
        // A gamma other than 2.2 (an image's own `map_gamma`, a squared
        // weight map at 4.4) needs the legacy-gamma space: in sRGB Octane
        // ignores the pin, and Calib-Overlay's squared belts rendered
        // unsquared, 4–14 × Iray (#71).
        self.colour_space_pin(i + 1, image_colour_space(gamma));
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
            let (ou, ov) = uv_translation([tu, tv], [ou, ov]);
            let _ = writeln!(self.out, "{:j$}  <attr name='translation' type='8'>{} {} 0</attr>", "", num(ou), num(ov));
            let _ = writeln!(self.out, "{:j$} </node>", "");
            self.pin_close(j);
        }
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// The material's displacement (`displacement`) as an Octane vertex
    /// displacement (97): height map at DAZ's gamma, mid level, subdivision.
    fn displacement_pin(&mut self, indent: usize, mat: &Material) {
        let Some(d) = displacement(mat) else { return };
        let Some(asset) = self.textures.asset(self.dirs, &d.map) else { return };
        let id = self.id();
        self.pin_open(indent, "displacement");
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='97'>", "");
        self.image_pin(i + 1, "texture", &asset, true, d.gamma, 1.0);
        self.value_pin(i + 1, "amount", 6, 9, &format!("{} 0 0 0", num(d.height_m)));
        self.value_pin(i + 1, "black_level", 6, 9, &format!("{} 0 0 0", num(d.mid)));
        self.value_pin(i + 1, "subdLevel", 9, 5, &format!("{} 0 0 0", d.subd));
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// DAZ's Round Corners (`round_corners`) as Octane's Round Edges (137)
    /// in Accurate mode (convex and concave, as Iray's
    /// `state::rounded_corner_normal`), the radius in meters, the roundness
    /// as it is, this object only.
    fn round_edges_pin(&mut self, indent: usize, mat: &Material) {
        let Some(r) = round_corners(mat) else { return };
        let id = self.id();
        self.pin_open(indent, "roundEdges");
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='137'>", "");
        self.value_pin(i + 1, "roundEdgesMode", 57, 2, &ROUND_EDGES_ACCURATE.to_string());
        self.value_pin(i + 1, "roundEdgesRadius", 6, 9, &format!("{} 0 0 0", num(r.radius_m)));
        self.value_pin(i + 1, "roundEdgesCurvatureRoundness", 6, 9, &format!("{} 0 0 0", num(r.roundness)));
        self.value_pin(i + 1, "roundEdgesSampleCount", 9, 5, &format!("{ROUND_EDGES_SAMPLES} 0 0 0"));
        self.value_pin(i + 1, "roundEdgesConsiderOtherObjects", 11, 1, "0");
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// A colour map (`tinted_image_pin`) under OOT Hairblending's overlay:
    /// its `<channel> Overlay Color` blended in where `Overlay Alpha Mask`
    /// is light (Blend Colors, normal mode — the hair's second colour, tips
    /// or streaks), as a Mix texture (38). Without an overlay the map alone.
    /// Only with the mask: Iray Uber's own "Diffuse Overlay Color" is
    /// another layer (#32).
    fn overlaid_pin(&mut self, i: usize, pin: &str, mat: &Material, channel: &str, asset: &str, power: f32, tint: [f32; 3]) {
        let overlay_name = match channel {
            "diffuse" => "Diffuse Overlay Color",
            "Translucency Color" => "Translucency Overlay Color",
            "Glossy Color" => "Glossy Overlay Color",
            _ => "",
        };
        let normal_mode = mat.float("Overlay Mode").unwrap_or(0.0) as i32 == 0;
        let mask = mat.image("Overlay Alpha Mask").filter(|_| normal_mode && !overlay_name.is_empty()).map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t));
        let overlay = mat.image(overlay_name).map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t));
        let (Some(mask), Some(overlay)) = (mask, overlay) else {
            return self.tinted_image_pin(i, pin, asset, power, tint);
        };
        let overlay_tint = mat.color(overlay_name).unwrap_or([1.0; 3]);
        let id = self.id();
        self.pin_open(i, pin);
        let j = i + 1;
        let _ = writeln!(self.out, "{:j$}<node id='{id}' type='38'>", "");
        self.image_pin(j + 1, "amount", &mask, true, 2.2, 1.0);
        self.tinted_image_pin(j + 1, "texture1", asset, power, tint);
        self.tinted_image_pin(j + 1, "texture2", &overlay, power, overlay_tint);
        let _ = writeln!(self.out, "{:j$}</node>", "");
        self.pin_close(i);
    }

    /// Iray Uber's Diffuse Overlay (`diffuse_overlay`) on the albedo: a Mix
    /// texture (38), amount = the weight (its map × the weight, squared as
    /// DAZ squares it), texture1 = the base (`base` writes it), texture2 =
    /// the overlay colour `colour` (linear, × its map). A weight map that
    /// isn't found leaves the base alone; a colour map, the tint.
    fn diffuse_overlay_pin(&mut self, i: usize, ov: &DiffuseOverlay, colour: [f32; 3], base: impl FnOnce(&mut Self, usize)) {
        let weight_map = match &ov.weight_map {
            Some((t, gamma)) => match self.textures.asset(self.dirs, t) {
                Some(asset) => Some((asset, *gamma)),
                None => {
                    let id = self.id();
                    self.pin_open(i, "albedo");
                    let _ = writeln!(self.out, "{:j$}<node id='{id}' type='38'>", "", j = i + 1);
                    self.float_pin(i + 2, "amount", 0.0);
                    base(self, i + 2);
                    self.rgb_pin(i + 2, "texture2", colour);
                    let _ = writeln!(self.out, "{:j$}</node>", "", j = i + 1);
                    return self.pin_close(i);
                }
            },
            None => None,
        };
        let colour_map = ov.colour_map.as_ref().and_then(|t| self.textures.asset(self.dirs, t));
        let id = self.id();
        self.pin_open(i, "albedo");
        let j = i + 1;
        let _ = writeln!(self.out, "{:j$}<node id='{id}' type='38'>", "");
        match weight_map {
            Some((asset, gamma)) => self.image_pin(j + 1, "amount", &asset, true, gamma, ov.weight),
            None => self.float_pin(j + 1, "amount", ov.weight),
        }
        base(self, j + 1);
        match colour_map {
            Some(asset) => self.tinted_image_pin(j + 1, "texture2", &asset, 1.0, colour),
            None => self.rgb_pin(j + 1, "texture2", colour),
        }
        let _ = writeln!(self.out, "{:j$}</node>", "");
        self.pin_close(i);
    }

    /// Whether a custom terrain shader darkens its diffuse map by an AO map
    /// or wet patches (`damp_ao_albedo`).
    fn has_damp_or_ao(&self, mat: &Material) -> bool {
        mat.image("AO Texture Map").is_some() || (mat.image("Damp Map").is_some() && mat.color("Damp Color").is_some())
    }

    /// Howie Farkes's terrain shader (Grassy Grove's HFGG Terrain): the
    /// diffuse map × its AO map, both tiled with the material, darkened
    /// toward `Damp Color` by `Damp Map` (Blend Colors, multiply) — whose
    /// tiler has no frequency of its own, so it spans the terrain once. Its
    /// large wet patches are the ground's texture in Iray; left out,
    /// Gladiator's ground showed only the tiled detail.
    fn damp_ao_albedo(&mut self, i: usize, mat: &Material, diffuse: &str, power: f32, tint: [f32; 3]) {
        let ao = mat.image("AO Texture Map").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t));
        let multiply_mode = mat.float("Damp Blend Mode").unwrap_or(2.0) as i32 == 2;
        let damp = mat.image("Damp Map").filter(|_| multiply_mode).map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t));
        let damp_colour = mat.color("Damp Color").unwrap_or([1.0; 3]);
        // Multiply(diffuse, Multiply(AO, Mix(white, damp colour, damp map))),
        // the parts left out when missing.
        let open_multiply = |w: &mut Self, indent: usize, pin: &str| {
            let id = w.id();
            w.pin_open(indent, pin);
            let _ = writeln!(w.out, "{:j$}<node id='{id}' type='39'>", "", j = indent + 1);
        };
        let close = |w: &mut Self, indent: usize| {
            let _ = writeln!(w.out, "{:j$}</node>", "", j = indent + 1);
            w.pin_close(indent);
        };
        let damp_pin = |w: &mut Self, indent: usize, pin: &str, damp: &str| {
            let id = w.id();
            w.pin_open(indent, pin);
            let j = indent + 1;
            let _ = writeln!(w.out, "{:j$}<node id='{id}' type='38'>", "");
            let tiling = w.tiling.take();
            w.image_pin(j + 1, "amount", damp, true, 2.2, 1.0);
            w.tiling = tiling;
            w.rgb_pin(j + 1, "texture1", [1.0; 3]);
            w.rgb_pin(j + 1, "texture2", damp_colour);
            let _ = writeln!(w.out, "{:j$}</node>", "");
            w.pin_close(indent);
        };
        open_multiply(self, i, "albedo");
        let j = i + 2;
        self.tinted_image_pin(j, "texture1", diffuse, power, tint);
        match (&ao, &damp) {
            (Some(ao), Some(damp)) => {
                open_multiply(self, j, "texture2");
                self.image_pin(j + 2, "texture1", ao, false, 2.2, 1.0);
                damp_pin(self, j + 2, "texture2", damp);
                close(self, j);
            }
            (Some(ao), None) => self.image_pin(j, "texture2", ao, false, 2.2, 1.0),
            (None, Some(damp)) => damp_pin(self, j, "texture2", damp),
            (None, None) => self.rgb_pin(j, "texture2", [1.0; 3]),
        }
        close(self, i);
    }

    /// A colour map scaled by `power` and multiplied by Iray's colour `tint`:
    /// a grey tint folds into the image's power, a coloured one goes through
    /// a Multiply texture (39) — as a scalar, a grey strand map × brown
    /// rendered grey-white pubic hair (Octane3).
    /// An IES profile on the `distribution` pin (see [`IesDistribution`]).
    fn ies_pin(&mut self, indent: usize, d: &IesDistribution) {
        let id = self.id();
        self.pin_open(indent, "distribution");
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='36'>", "");
        let _ = writeln!(self.out, "{:i$} <attr name='filename' type='11'>{}</attr>", "", esc(&d.asset));
        let _ = writeln!(self.out, "{:i$} <attr name='initialColorSpaceSelected' type='1'>1</attr>", "");
        // Normalised to a peak of 1 (Octane's default and its samples').
        let _ = writeln!(self.out, "{:i$} <attr name='iesPhotometryMode' type='2'>1</attr>", "");
        self.colour_space_pin(i + 1, COLOUR_SPACE_LINEAR_SRGB);
        let pid = self.id();
        let j = i + 1;
        self.pin_open(j, "projection");
        let _ = writeln!(self.out, "{:j$} <node id='{pid}' type='77'>", "");
        let [rx, ry, rz] = ies_projection_rotation(&d.frame);
        let tid = self.id();
        let k = j + 1;
        self.pin_open(k, "transform");
        let _ = writeln!(self.out, "{:k$} <node id='{tid}' type='67'>", "");
        let _ = writeln!(self.out, "{:k$}  <attr name='rotationOrder' type='2'>0</attr>", "");
        let _ = writeln!(self.out, "{:k$}  <attr name='rotation' type='8'>{} {} {}</attr>", "", num64(rx), num64(ry), num64(rz));
        let _ = writeln!(self.out, "{:k$}  <attr name='scale' type='8'>1 1 1</attr>", "");
        let _ = writeln!(self.out, "{:k$}  <attr name='translation' type='8'>0 0 0</attr>", "");
        let _ = writeln!(self.out, "{:k$} </node>", "");
        self.pin_close(k);
        // Coordinate space: 1 world, 3 object, 4 normal (probe).
        self.value_pin(k, "positionType", 57, 2, if d.world { "1" } else { "3" });
        let _ = writeln!(self.out, "{:j$} </node>", "");
        self.pin_close(j);
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

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
            Some(asset) => self.image_pin(indent, pin, &asset, true, data_gamma(mat, channel.unwrap_or_default()), value),
            None => self.float_pin(indent, pin, value),
        }
    }

    /// The gloss's roughness pin: `value` (Iray's roughness, through the GGX
    /// curve when `ggx`), or the channel's map as Iray reads it — the map ×
    /// the channel's value, 1 − that for a glossiness, squared (`square_float`,
    /// `gloss_to_rough`: DAZ's .mi export, Calib-Mi-Maps) — baked per pixel
    /// with the same curve. `stretch` is Iray's v / u roughness ratio
    /// (`Anisotropy::stretch`, 1 isotropic): the map then bakes Octane's
    /// roughness of both axes (`anisotropic_roughness`).
    fn roughness_pin(&mut self, indent: usize, mat: &Material, channel: Option<&str>, value: f32, curve: RoughCurve, stretch: f32) {
        let Some((ch, map)) = channel.and_then(|c| Some((c, mat.image(c)?.to_string()))) else {
            return self.float_pin(indent, "roughness", value);
        };
        let inverted = ch.ends_with("Glossiness");
        let v = mat.float(ch).unwrap_or(1.0);
        let squared = mat.float(if inverted { "Glossiness Squared" } else { "Roughness Squared" }).unwrap_or(1.0) > 0.0;
        // `derived` decodes at 2.2; the map's own gamma rescales that.
        let g = data_gamma(mat, ch) / crate::iray::COLOUR_GAMMA;
        let c = curve.key();
        let key = if stretch == 1.0 { format!("roughness|{map}|{v}|{inverted}|{squared}|{c}|{g}") } else { format!("roughness|{map}|{v}|{inverted}|{squared}|{c}|{g}|{stretch}") };
        let asset = self.textures.derived(self.dirs, &map, &key, 1.0, |l| {
            let m = l.max(0.0).powf(g) * v;
            let r = (if inverted { 1.0 - m } else { m }).clamp(0.0, 1.0);
            let r = if squared { r * r } else { r };
            [anisotropic_roughness(r, stretch, curve).0; 3]
        });
        match asset {
            Some(asset) => self.image_pin(indent, "roughness", &asset, true, 1.0, 1.0),
            // An image it can't read: the map as is (a glossiness, its value).
            None => self.float_or_map(indent, "roughness", mat, Some(ch).filter(|_| !inverted), value),
        }
    }

    /// The gloss's roughness for Iray's roughness `r` (squared) and its
    /// anisotropy: `roughness_pin` at Octane's roughness, then
    /// `anisotropyTexture` and `rotation` (both turns: Iray's 1 = 360°,
    /// Octane's slider 0 – 1). Returns the lobe's isotropic equivalent (both
    /// axes' geometric mean, the roughness itself when isotropic), for a
    /// coating at the gloss's roughness.
    fn gloss_roughness_pins(&mut self, indent: usize, mat: &Material, channel: Option<&str>, r: f32, curve: RoughCurve, aniso: Anisotropy) -> f32 {
        let stretch = aniso.stretch();
        let (rough, a) = anisotropic_roughness(r.clamp(0.0, 1.0), stretch, curve);
        self.roughness_pin(indent, mat, channel, rough, curve, stretch);
        if a == 0.0 {
            return rough;
        }
        self.float_pin(indent, "anisotropyTexture", a);
        if let Some(rot) = aniso.octane_rotation() {
            self.float_pin(indent, "rotation", rot);
        }
        // u = v (1 − a) / (1 + a).
        rough * ((1.0 - a) / (1.0 + a)).sqrt()
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
        // Emission Profile (#34): Iray's `measured_edf` in the mesh's object
        // space (`global_distribution`, identity frame), the luminance
        // divided by the profile's power — Octane's distribution in world
        // space along the node's axes, the power raised as for an IES light.
        // Iray's profile axis is the object's −Z, φ 0 its +X, φ 90 its −Y
        // (Calib-L-IES-Emit-Asym, as for lights). Iray emits the profile
        // itself, Octane Lambertian × the profile: the power keeps the flux
        // through the surface's front (`lambert_share` of its face normals).
        let ies = mat.channel("Emission Profile").and_then(|c| c.value.as_str()).filter(|p| !p.is_empty()).map(str::to_string).and_then(|p| self.textures.ies(self.dirs, &p)).map(|(asset, profile)| {
            let frame = mat.frame.map_or([[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]], |[x, y, z]| [x, y.map(|v| -v), z.map(|v| -v)]);
            let share = match &mat.emit_normals {
                Some(normals) => {
                    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
                    let sides: &[f64] = if two_sided { &[1.0, -1.0] } else { &[1.0] };
                    let local: Vec<([f64; 3], f64)> = sides.iter().flat_map(|s| normals.iter().map(move |(n, w)| (frame.map(|a| s * dot(a, *n)), w / sides.len() as f64))).collect();
                    profile.lambert_share(&local)
                }
                None => profile.mean_share(),
            };
            IesDistribution { asset, frame, power_scale: 1.0 / share.max(1e-3), world: true }
        });
        let watts = watts * ies.as_ref().map_or(1.0, |d| d.power_scale);
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
        if let Some(d) = ies {
            self.ies_pin(i + 1, &d);
        }
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// The skin's gloss on OpenPBR as Universal gets it (see `universal`):
    /// Iray's squared roughness; the dual lobe's (or glossy layer's) weight ×
    /// min(2r, 1) of its reflectivity — with the reflectivity's map (dark
    /// under brows and make-up, where a uniform gloss shone blue-grey on the
    /// dark skins; Twosret 1.31 → 1.16 × Iray, Calib-Skin); the Top Coat as
    /// the coating, without OpenPBR's coat darkening (it took the base to
    /// 0.58 – 0.80 of Universal's on Calib-SSS's spheres, off it is 1.00).
    fn skin_gloss(&mut self, i: usize, mat: &Material, rough_ch: Option<&str>, rough: f32, inverted: bool) {
        let glossy = mat.float("Glossy Layered Weight").unwrap_or(0.0).max(mat.float("Glossy Weight").unwrap_or(0.0));
        let coat = mat.float("Top Coat Weight").unwrap_or(0.0) > 0.0;
        // The dual lobe as Universal's (see `DualLobe`): lobe 2 the coating,
        // or under a Top Coat both lobes' roughness blended.
        let dual_lobe = DualLobe::of(mat).filter(|_| glossy <= 0.0);
        let mut rough = iray_gloss_roughness(mat).unwrap_or_else(|| iray_roughness(mat, rough, inverted));
        if let Some(dl) = dual_lobe.filter(|_| coat && rough_ch.and_then(|c| mat.image(c)).is_none()) {
            rough = dl.blended_roughness(rough);
        }
        self.roughness_pin(i, mat, rough_ch, rough.clamp(0.0, 1.0), RoughCurve::Linear, 1.0);
        let lobe1 = dual_lobe.filter(|_| !coat).map_or(1.0, |dl| dl.lobe1);
        let (weight, ch) = match dual_lobe {
            Some(dl) => (dl.share, "Dual Lobe Specular Reflectivity"),
            None => (glossy.min(1.0) * reflectivity_share(glossy_reflectivity(mat)), "Glossy Reflectivity"),
        };
        let lobe = if glossy > 0.0 { glossy } else { mat.float("Dual Lobe Specular Weight").unwrap_or(0.0) }.min(1.0);
        match mat.image(ch).map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.image_pin(i, "specular", &asset, true, data_gamma(mat, ch), lobe * 2.0 * lobe1),
            None => self.float_pin(i, "specular", weight * lobe1),
        }
        match dual_lobe.filter(|dl| !coat && dl.lobe1 < 1.0) {
            Some(dl) => {
                self.rgb_pin(i, "coating", [dl.share * (1.0 - dl.lobe1); 3]);
                self.float_pin(i, "coatingRoughness", dl.lobe2_roughness);
                self.float_pin(i, "coatingIor", dl.ior.unwrap_or(1.5));
                self.float_pin(i, "coatingDarkening", 0.0);
            }
            None => {
                self.top_coat(i, mat, 0.0);
                if coat {
                    self.float_pin(i, "coatingDarkening", 0.0);
                }
            }
        }
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
        // OpenPBR (Enhanced) on Iray's chromatic SSS (`SSS Mode` 1, the
        // modern skins): Iray's skin colour baked per texel, all of it
        // through the random walk, the gloss as Universal's (#19). Mono SSS
        // too with Base Color Effect 0: Calib-Skin's mono variants
        // (Victoria, Monique, Zurie) came out 0.83 – 1.04 × Iray relative to
        // the base render against 0.61 – 1.71 under the older rule (#28). Not the Genesis 8 base skin
        // (mono, w 0.5, Base Color Effect 1, SSS Direction −0.65, a glossy
        // layer): 1.4 – 2.0 × Iray under it on Calib-Eye, 1.0 – 1.4 under
        // the older rule it keeps.
        let mono = mat.float("SSS Mode").unwrap_or(0.0) as i32 != 1;
        let bake = openpbr && (!mono || mat.float("Base Color Effect").unwrap_or(0.0) as i32 == 0);
        let baked = if bake {
            let (d, tc) = (mat.image("diffuse").map(str::to_string), mat.image("Translucency Color").map(str::to_string));
            let (a, b) = skin_albedo(mat, translucency);
            let key = format!("skin:{d:?}:{tc:?}:{a:?}:{b:?}");
            self.textures.derived_skin(self.dirs, d.as_deref(), tc.as_deref(), &key, a, b)
        } else {
            None
        };
        // Otherwise (Standard Surface — Faithful —, or mono SSS): the
        // diffuse map darkened as Iray's translucent layer does
        // (`skin_tint`) — not recalibrated by #19.
        let tc_map = mat.image("Translucency Color").map(str::to_string).and_then(|t| self.textures.mean_covered_colour(self.dirs, &t));
        let mut tint = skin_tint(mat, translucency, tc_map);
        if openpbr {
            // OpenPBR's random walk shows less of the colour, the more so
            // the more subsurface: brought back to Standard Surface's level.
            let lift = openpbr_skin_lift(translucency);
            tint = tint.map(|t| t * lift);
        }
        let colour = [0, 1, 2].map(|k| mat.base_color()[k] * tint[k]);
        let diffuse = mat.image("diffuse").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t));
        let colour_pin = |w: &mut Self, pin: &str| match (&baked, &diffuse) {
            (Some((asset, _)), _) => w.image_pin(i, pin, asset, false, 2.2, 1.0),
            (None, Some(asset)) => w.tinted_image_pin(i, pin, asset, 1.0, colour),
            (None, None) => w.rgb_pin(i, pin, colour),
        };
        colour_pin(self, "baseColor");
        let (rough_ch, rough, inverted) = mat.roughness_source();
        if bake {
            self.skin_gloss(i, mat, rough_ch, rough, inverted);
        } else {
            // Standard Surface takes the slider unsquared; a glossiness map
            // would need inverting: its value only.
            self.float_or_map(i, "roughness", mat, rough_ch.filter(|_| !inverted), rough.clamp(0.0, 1.0));
        }
        match &baked {
            // The diffuse map's marked detail (brows, liner) out of the random
            // walk: under it they bled into blue-grey smudges, as a diffuse
            // they stay dark like Iray's. Elsewhere all of the colour goes
            // through it — at subsurface 0.5 Octane rendered 0.67 – 0.73 of
            // the colour whatever it was (Calib-SSS's spheres).
            Some((_, Some(detail))) => self.image_pin(i, "subsurface", detail, true, 2.2, 1.0),
            Some((_, None)) => self.float_pin(i, "subsurface", 1.0),
            None => self.float_pin(i, "subsurface", translucency),
        }
        // The SSS colour is the colour the skin *shows* (Standard Surface,
        // like Arnold's): the diffuse map, with tan and make-up. Iray's
        // translucency map is a pale, pinkish "sss" map of how light shines
        // through — as subsurface colour it bleached Zelie and wiped her
        // make-up (Octane1 vs Blender, 2026-10-21).
        colour_pin(self, "subsurfaceColor");
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
        self.base_bump_pin(i, mat, "bump");
        self.round_edges_pin(i, mat);
        self.displacement_pin(i, mat);
        let opacity = mat.opacity().clamp(0.0, 1.0);
        if opacity < 1.0 || mat.image(mat.opacity_channel()).is_some() {
            self.float_or_map(i, "opacity", mat, Some(mat.opacity_channel()), opacity);
        }
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// DAZ's dForce hair shader (Blended Dual Lobe Hair, on strands) → an
    /// Octane Hair material (type 147) in albedo mode: the hair colour and
    /// transmission colour, each its map's mean × its tint (strands have no
    /// UVs), mixed by `HAIR_COLOUR_SHARE` / `HAIR_TRANSMISSION_SHARE` — at the
    /// root and at the tip, under 1 − Highlight Weight with the highlight
    /// colour's share over it (`hair_albedo`), blended along each strand as
    /// Iray blends its root and tip lobes (`hair_albedo_pin`); longitudinal
    /// and azimuthal roughness from `base_roughness`; the specular the
    /// glossy layer's weight under 1 − Highlight Weight, the highlight
    /// colour over it; `separation` left out (Octane's `offset` only dimmed
    /// the hair, Calib-Hair-hl).
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
        let (root, root_t) = (colour("Hair Root Color"), colour("Root Transmission Color"));
        let (tip, tip_t) = (colour("Hair Tip Color"), colour("Tip Transmission Color"));
        let (hl_root, hl_tip) = (colour("Highlight Root Color"), colour("Tip Highlight Color"));
        let hw = mat.float("Highlight Weight").unwrap_or(0.11).clamp(0.0, 1.0);
        let albedo = |c: [f32; 3], t: [f32; 3], h: [f32; 3]| hair_albedo(c, t, h, hw);
        self.value_pin(i, "hairMode", 57, 2, "0"); // albedo
        let (bias, gain) = (mat.float("Root To Tip Bias").unwrap_or(0.5), mat.float("Root to Tip Gain").unwrap_or(0.5));
        self.hair_albedo_pin(i, albedo(root, root_t, hl_root), albedo(tip, tip_t, hl_tip), bias, gain);
        let gloss = mat.float("Glossy Layer Weight").unwrap_or(1.0).clamp(0.0, 1.0);
        self.rgb_pin(i, "specular", [0, 1, 2].map(|k| (1.0 - hw) * gloss + hw * 0.5 * (hl_root[k] + hl_tip[k])));
        let rough = mat.float("base_roughness").unwrap_or(0.2).clamp(0.0, 1.0);
        self.float_pin(i, "roughness", rough);
        self.float_pin(i, "roughnessV", rough);
        let _ = writeln!(self.out, "{:indent$}</node>", "");
    }

    /// The Hair `albedo` from `root` to `tip` along each strand: Iray mixes a
    /// root and a tip `dual_lobe_hair` by DAZ's `get_line_root_to_tip_blend`
    /// on the strand's line coordinate (Calib-Mi-Hair2's .mi); Octane gives
    /// each strand a W coordinate, 0 at the root, 1 at the tip. A Gradient
    /// (49) on a W Coordinate texture (104) from the root's to the tip's
    /// albedo, the blend's shape (Root To Tip Bias / Gain other than 0.5)
    /// sampled at `HAIR_BLEND_POINTS` control points; equal ends: the colour.
    fn hair_albedo_pin(&mut self, indent: usize, root: [f32; 3], tip: [f32; 3], bias: f32, gain: f32) {
        if (0..3).all(|k| (root[k] - tip[k]).abs() < 1e-4) {
            return self.rgb_pin(indent, "albedo", root);
        }
        let rgb = |c: [f32; 3]| format!("{} {} {}", num(c[0]), num(c[1]), num(c[2]));
        let linear = (bias - 0.5).abs() < 1e-4 && (gain - 0.5).abs() < 1e-4;
        let points = if linear { Vec::new() } else { blend_points(bias, gain) };
        self.pin_open(indent, "albedo");
        let (i, j) = (indent + 1, indent + 2);
        let id = self.id();
        let _ = writeln!(self.out, "{:i$}<node id='{id}' type='49'>", "");
        let _ = writeln!(self.out, "{:j$}<attr name='controlpoints' type='2'>{}</attr>", "", points.len());
        self.value_pin(j, "gradientInterpolationType", 57, 2, "2"); // linear
        self.value_pin(j, "gradientInterpolationColorSpace", 57, 2, "0"); // physical
        let wid = self.id();
        self.pin_open(j, "input");
        let _ = writeln!(self.out, "{:k$}<node id='{wid}' type='104'/>", "", k = j + 1);
        self.pin_close(j);
        self.value_pin(j, "min", 33, 8, &rgb(root));
        self.value_pin(j, "max", 33, 8, &rgb(tip));
        for (n, t) in points.iter().enumerate() {
            let w = crate::iray::root_to_tip_blend(*t, bias, gain);
            let c = [0, 1, 2].map(|k| root[k] + (tip[k] - root[k]) * w);
            for (pin, dynamic, node, attr, value) in [("Position", 2, 6, 9, format!("{} 0 0 0", num(*t))), ("Value", 5, 33, 8, rgb(c))] {
                let vid = self.id();
                let _ = writeln!(self.out, "{:j$}<pin name='{pin} {}' dynamicType='{dynamic}'>", "", n + 1);
                let _ = writeln!(self.out, "{:k$}<node id='{vid}' type='{node}'>", "", k = j + 1);
                let _ = writeln!(self.out, "{:k$} <attr name='value' type='{attr}'>{value}</attr>", "", k = j + 1);
                let _ = writeln!(self.out, "{:k$}</node>", "", k = j + 1);
                let _ = writeln!(self.out, "{:j$}</pin>", "");
            }
        }
        let _ = writeln!(self.out, "{:i$}</node>", "");
        self.pin_close(indent);
    }

    /// DAZ's OmniHair (`omni_hair`) as Octane's Hair in melanin mode.
    fn omni_hair(&mut self, indent: usize, name: &str, h: &OmniHair) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='147' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        self.value_pin(i, "hairMode", 57, 2, "1"); // melanin + pheomelanin
        self.float_pin(i, "hairMelanin", h.eumelanin);
        self.float_pin(i, "hairPheomelanin", h.pheomelanin);
        self.float_pin(i, "ior", h.ior);
        self.float_pin(i, "roughness", h.roughness);
        self.float_pin(i, "roughnessV", h.azimuthal);
        if h.diffuse > 0.0 {
            self.float_pin(i, "diffuseAmount", h.diffuse);
            self.rgb_pin(i, "diffuse", h.diffuse_colour);
        }
        if h.opacity < 1.0 {
            self.float_pin(i, "opacity", h.opacity.max(0.0));
        }
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
        if let Some(h) = omni_hair(mat) {
            return self.omni_hair(indent, name, &h);
        }
        if mat.channel("Hair Root Color").is_some() {
            return self.hair(indent, name, mat);
        }
        if let Some(metal) = mdl_metal(mat) {
            return self.mdl_metal(indent, name, &metal, mat);
        }
        if let Some(d) = mdl_dielectric(mat) {
            return self.mdl_dielectric(indent, name, &d, mat);
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
        let metal = metallic(mat) >= NEGLIGIBLE_METALLIC || mat.image("Metallic Weight").is_some();
        // Nor is it on a prop: Gladiator's grass (Iray Uber, translucent, not
        // thin-walled) rendered as grey cards.
        // Nor in the eye: Genesis irises, pupils and sclera are translucent
        // too, and the skin's tint (half, reddened) turned blue irises dark
        // green (Calib-Hair, #20).
        if skin != SkinShader::None && translucency > 0.0 && !refractive && !emissive && !cutout && !thin && !metal && !mat.prop && !is_eye(name) && mat.image("diffuse").is_some() {
            return self.standard_surface(indent, name, mat, translucency, skin == SkinShader::OpenPbr);
        }
        if !self.in_layered_coat && !self.in_backscatter_mix && !self.in_flake_mix && !self.in_coat_mix && layered_top_coat(mat) {
            return self.layered_coat_material(indent, name, mat, area_m2, skin);
        }
        if !self.in_backscatter_mix && !self.in_flake_mix && !self.in_coat_mix && backscatter_weight(mat) > 0.0 {
            return self.backscatter_mix_material(indent, name, mat, area_m2, skin);
        }
        if !self.in_flake_mix && !self.in_coat_mix && flake_share(mat) > 0.0 {
            return self.flake_mix_material(indent, name, mat, area_m2, skin);
        }
        if !self.in_coat_mix && weighted_top_coat(mat) {
            return self.weighted_coat_material(indent, name, mat, area_m2, skin);
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
        // Genesis 9's eye (PBRSkin, a closed ball): Iray scatters its
        // translucency — the iris — just under the surface (scattering
        // distance 0.15 mm), so it shows as colour, not light through the
        // ball; as Universal's transmission the whole eye glowed milky.
        let eye_volume = is_eye(name) && crate::iray::is_pbrskin(mat) && translucency > 0.0 && !thin;
        let transmit = if metal || refractive || eye_volume || mat.float("Base Mixing").unwrap_or(0.0) as i32 == 2 { 0.0 } else { translucency.min(self.max_transmission) };
        // Base Color Effect 1 (Scatter & Transmit): Iray reflects 0.4 of
        // that diffuse and passes twice the translucency — Calib-Thin's
        // cards (thin-walled, w 0.3 – 0.65, Translucency Color white /
        // the diffuse / dark, lit from the front or behind) 0.98 – 1.02 ×
        // Iray; at the default shares 1.8 – 2.5 ×. The Genesis EyeSocket's
        // caruncle is such a surface (#20, #45). Base Color Effect 2
        // (Scatter & Transmit Intensity: the same split, the diffuse weight
        // greyed) 0.95 and 1.55: Calib-ColorEffect's C3 1.00 / 1.00 / 0.95 ×
        // Iray, taken as effect 0 0.48 / 0.85 / 0.99 (#27).
        let (diffuse_share, transmit_share) = if transmit > 0.0 {
            match mat.float("Base Color Effect").unwrap_or(0.0) as i32 {
                1 => (SCATTER_TRANSMIT_DIFFUSE, SCATTER_TRANSMIT_TRANSMISSION),
                2 => (SCATTER_TRANSMIT_INTENSITY_DIFFUSE, SCATTER_TRANSMIT_INTENSITY_TRANSMISSION),
                _ => (1.0, 1.0),
            }
        } else {
            (1.0, 1.0)
        };
        // On metal the translucency isn't transmitted (above), yet Iray dims
        // the surface's non-metal part by it: light goes into the volume and
        // only partly comes back. Calib-PBRSkin-MetalTL (PBRSkin metal × its
        // translucency): without it metal 0.3 / a grey metallic map at
        // translucency 0.85 rendered 1.32 / 1.38 × Iray, with all of the
        // translucency's share taken 0.71 / 0.66, metal 0.75 0.53.
        let dimmed = if metal && !refractive { METAL_TRANSLUCENCY_LOSS * translucency } else { 0.0 };
        let diffuse_weight = (1.0 - refraction) * (1.0 - transmit) * (1.0 - dimmed) * diffuse_share;

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
        // Base Mixing 1 with a coloured gloss: see `specular_diffuse_share`.
        let share = if specular_mode && !mirror && weighted.is_none() { specular_diffuse_share(mat, gloss.min(1.0), gloss_color) } else { [1.0; 3] };
        // Under a partial metal albedo and gloss scale (`metal_shares`).
        let metal_w = if mirror || specular_mode || weighted.is_some() || !(metallic(mat) >= NEGLIGIBLE_METALLIC || mat.image("Metallic Weight").is_some()) {
            0.0
        } else {
            metallic(mat)
        };
        let base_keep = metal_mix(mat, metal_w).1;
        let share = share.map(|s| s * base_keep);
        let tint = [0, 1, 2].map(|k| tint[k] * share[k]);
        // The weighted blend scales the diffuse map (not a translucency
        // map standing in for a missing one — the HS CB cap went grey).
        let eye_albedo = if eye_volume {
            match (mat.image("diffuse"), mat.image("Translucency Color")) {
                (Some(d), Some(t)) => self.textures.blend(self.dirs, d, t, 1.0 - translucency, translucency),
                _ => None,
            }
        } else {
            None
        };
        // Iray's diffuse is Oren-Nayar at Diffuse Roughness σ (DAZ's MDL).
        // Universal's Oren-Nayar (`diffuseBrdf` 2) takes the one roughness
        // pin the gloss, metal and transmission use too: only where none
        // of them is there does σ go on it (Calib-Diffuse, σ 0.5 / 1: Lambert
        // 1.26 / 1.49 × Iray, Oren-Nayar 1.02 / 0.97, the smooth sphere
        // 1.08). Elsewhere the Lambertian albedo × the head-on albedo of
        // Iray's Oren-Nayar (`oren_nayar_flat`), the overlay's at its own
        // roughness (#32, #76); under Octane's Oren-Nayar, which stands in
        // for Iray's at σ, the overlay's relative to the base's.
        let glossy = mat.float("Glossy Layered Weight").unwrap_or(0.0).max(mat.float("Glossy Weight").unwrap_or(0.0));
        let dual_lobe = DualLobe::of(mat);
        let no_gloss = weighted.is_some() || (!mirror && refraction == 0.0 && glossy <= 0.0 && dual_lobe.is_none() && mat.image("Glossy Layered Weight").is_none());
        let sigma = diffuse_roughness(mat, false);
        let oren_nayar = sigma > 0.0 && no_gloss && !mirror && !metal && refraction == 0.0 && transmit == 0.0;
        let base_flat = if oren_nayar { 1.0 } else { oren_nayar_flat(sigma) };
        let overlay_flat = oren_nayar_flat(diffuse_roughness(mat, true)) / if oren_nayar { oren_nayar_flat(sigma) } else { 1.0 };
        if mirror {
            self.rgb_pin(i, "albedo", gloss_color);
        } else if let Some(asset) = eye_albedo {
            // The diffuse's share of the colour map and the translucency's of
            // its map, mixed per pixel (sRGB-encoded, as the maps are).
            self.image_pin(i, "albedo", &asset, false, 2.2, 1.0);
        } else {
            // Iray Uber's Diffuse Overlay over the base: flat over a flat
            // base folded into one colour, else a Mix texture (#71).
            let overlay = diffuse_overlay(mat);
            let overlay_colour = |ov: &DiffuseOverlay| [0, 1, 2].map(|k| ov.colour[k] * share[k] * overlay_flat);
            match diffuse.and_then(|t| self.textures.asset(self.dirs, &t)) {
                Some(asset) if self.has_damp_or_ao(mat) => self.damp_ao_albedo(i, mat, &asset, diffuse_weight * mix * base_flat, tint),
                Some(asset) => match overlay.filter(|_| mat.image("Overlay Alpha Mask").is_none()) {
                    Some(ov) => self.diffuse_overlay_pin(i, &ov, overlay_colour(&ov), |w, j| w.tinted_image_pin(j, "texture1", &asset, diffuse_weight * mix * base_flat, tint)),
                    None => self.overlaid_pin(i, "albedo", mat, "diffuse", &asset, diffuse_weight * mix * base_flat, tint),
                },
                None => {
                    let base = [0, 1, 2].map(|k| mat.base_color()[k] * diffuse_weight * share[k] * base_flat);
                    match overlay {
                        Some(ov) if ov.has_map() => self.diffuse_overlay_pin(i, &ov, overlay_colour(&ov), |w, j| w.rgb_pin(j, "texture1", base)),
                        Some(ov) => {
                            let c = overlay_colour(&ov);
                            self.rgb_pin(i, "albedo", [0, 1, 2].map(|k| base[k] * (1.0 - ov.weight) + c[k] * ov.weight))
                        }
                        None => self.rgb_pin(i, "albedo", base),
                    }
                }
            }
        }

        if transmit > 0.0 {
            let tc = mat.color("Translucency Color").unwrap_or([1.0; 3]);
            // An open sheet that isn't thin-walled (a leaf or grass card)
            // passes twice as much in Iray: Calib-Leaf's card 0.48 × Iray at
            // w, 0.81 at min(2w, 1); a closed sphere stays at w (1.25 – 1.38
            // at 2w).
            let share = if mat.open && !thin { (2.0 * transmit).min(1.0) } else { transmit } * transmit_share;
            // A closed solid with Iray's volume under it: the light it passes
            // back out scales with the volume's diffusion reflectance, as on
            // skin (`volume_factor`), × `PROP_VOLUME_LEVEL`. Calib-SSSColor's
            // spheres (Victoria 8's volume, SSS Color grey / blue / deep)
            // 0.84 – 1.05 × Iray, before 1.05 – 2.68; mono 1.40 – 1.54
            // (before 1.40 – 1.69: Iray's mono spheres darker than Rd says,
            // unlike its figures), SSS Reflectance Tint, unread, 1.34 (#28).
            let volume = if !thin && !mat.open && absorption(mat).is_some() { volume_factor(mat).map(|v| v * PROP_VOLUME_LEVEL) } else { [1.0; 3] };
            let tc = [0, 1, 2].map(|k| tc[k] * volume[k]);
            match mat.image("Translucency Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
                Some(asset) => self.overlaid_pin(i, "transmission", mat, "Translucency Color", &asset, share, tc),
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
        // (`glossy`, `dual_lobe`: above.)
        // Iray's Top Coat: a clear dielectric layer over everything.
        let coat = mat.float("Top Coat Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        let mut active_dual_lobe = None;
        if weighted.is_some() {
            self.rgb_pin(i, "specular", [0.0; 3]);
        } else if !mirror && refraction == 0.0 && glossy <= 0.0 && dual_lobe.is_none() && mat.image("Glossy Layered Weight").is_none() {
            self.rgb_pin(i, "specular", [0.0; 3]);
        } else if iray(mat).and_then(|u| u.bool("refraction_reflection_enable")) == Some(false) {
            // Iray's refraction reflects through the glossy layer only: without
            // one DAZ switches its reflection off (`refraction_reflection_enable`,
            // the .mi export) — Calib-Dispersion's glass spheres show no sky
            // nor sun on their surface in Iray.
            self.rgb_pin(i, "specular", [0.0; 3]);
        } else if !mirror && glossy > 0.0 {
            // Iray's Glossy Reflectivity r scales the reflection (r 0.5 is
            // the default, Universal's full specular); Octane matched it at
            // specular × min(2r, 1) — Calib-Glossy, r 0 / 0.25: 2.42 / 1.54
            // × Iray at Universal's default, 1.21 / 1.14 so, like the matte
            // sphere's 1.13 (r 1: see `glossy_excess`).
            let refl = if specular_mode { 1.0 } else { reflectivity_share(glossy_reflectivity(mat)) };
            let weight = glossy.min(1.0) * refl * base_keep;
            // Base Mixing 1: Glossy Specular's hue tints the reflection too
            // (its strength is the IOR, `specular_ior`).
            let hue = if specular_mode { specular_hue(mat) } else { [1.0; 3] };
            // Glossy Color Effect: the hue at the weight × the colour's
            // average (`color_effect`).
            let (gloss_color, weight) = color_effect(mat, "Glossy Color Effect", gloss_color, weight);
            let gloss_color = [0, 1, 2].map(|k| gloss_color[k] * hue[k]);
            // A coloured gloss map tints the reflection: the HS CB braids'
            // brown gloss rendered as a grey sheen with Universal's white
            // default (Octane1, probe 60).
            if let Some(asset) = mat.image("Glossy Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
                self.overlaid_pin(i, "specular", mat, "Glossy Color", &asset, weight, gloss_color);
            } else if let Some(asset) = mat.image("Glossy Layered Weight").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
                // The glossy layer's weight map (data) × its weight: the
                // GSOV4 leather's 0.33 gloss with a seam mask rendered 3.4×
                // Iray's sheen with Universal's full white one (Fetroom).
                self.image_pin(i, "specular", &asset, true, data_gamma(mat, "Glossy Layered Weight"), weight);
            } else if weight < 1.0 || gloss_color != [1.0; 3] || self.film.is_some() {
                // (Universal's default is white; a film tints it.)
                self.rgb_pin(i, "specular", gloss_color.map(|c| c * weight));
            }
        } else if let Some(dl) = dual_lobe.filter(|_| !mirror && refraction == 0.0) {
            active_dual_lobe = Some(dl);
            // The dual lobe specular alone: its reflectivity as the glossy
            // layer's, × its weight; lobe 1's part of it here, lobe 2's the
            // coating below (or all of it, when a Top Coat takes that).
            let weight = if coat > 0.0 { dl.share } else { dl.share * dl.lobe1 };
            if weight < 1.0 || self.film.is_some() {
                self.rgb_pin(i, "specular", [weight; 3]);
            }
        }

        let metallic = if mirror { 1.0 } else if specular_mode { 0.0 } else { metallic(mat) };
        // A trace of metallicity (the GSOV4 jacket's 0.0107) turned Octane's
        // gloss off — black velvet where Iray shows leather (Fetroom).
        if weighted.is_some() {
            // The diffuse part of a weighted mix: its gloss is the mix's
            // other material.
        } else if mirror || (!specular_mode && (metallic >= NEGLIGIBLE_METALLIC || mat.image("Metallic Weight").is_some())) {
            // Iray's metal reflects at w² (`metal_shares`); a map squared
            // too, by twice its gamma (its 1 / k at the channel's value).
            let m = metal_mix(mat, metallic).0;
            // Iray Uber's map is squared with its weight (twice its gamma);
            // PBRSkin's is read as is.
            let squared = !crate::iray::is_pbrskin(mat);
            match (mirror, mat.image("Metallic Weight").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t))) {
                (false, Some(asset)) => self.image_pin(i, "metallic", &asset, true, if squared { 2.0 } else { 1.0 } * data_gamma(mat, "Metallic Weight"), m),
                (false, None) => self.float_pin(i, "metallic", m),
                (true, _) => self.float_pin(i, "metallic", metallic.max(0.0)),
            }
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
        let mut rough = match iray_gloss_roughness(mat).filter(|_| !refraction_rough) {
            Some(r) => r,
            None if refraction_rough => iray(mat).and_then(|u| u.float("refraction_roughness")).unwrap_or_else(|| iray_roughness(mat, rough, inverted)),
            None => iray_roughness(mat, rough, inverted),
        };
        let rough_ch = if refraction_rough { None } else { rough_ch };
        // With dispersion Iray's transmission is perfectly smooth whatever
        // the roughness (DAZ's MDL, `refraction_base`); a mostly refractive
        // body takes that for Universal's one roughness.
        let dispersive = !thin && refraction >= 0.5 && dispersion_abbe(mat).is_some();
        let rough_ch = if dispersive {
            rough = 0.0;
            None
        } else {
            rough_ch
        };
        // A dual lobe under a Top Coat: one lobe at both roughnesses' blend
        // (a roughness map stays lobe 1's).
        if let Some(dl) = active_dual_lobe.filter(|_| coat > 0.0 && rough_ch.and_then(|c| mat.image(c)).is_none()) {
            rough = dl.blended_roughness(rough);
        }
        // A mostly refractive body (not thin-walled: its transmission
        // doesn't blur) refracts through Beckmann at √r, shared roughness or
        // its own (`GLASS_BRDF`); other glosses GGX energy preserving
        // (`gloss_roughness`), a Top Coat over the glass too.
        let glass = !thin && !mirror && refraction >= 0.5 && weighted.is_none();
        self.gloss_ggx = !refraction_rough;
        // Iray's anisotropy on the gloss (and the metal: both take the
        // glossy layer's); a refraction's roughness keeps it isotropic.
        let aniso = if refraction_rough || dispersive { Anisotropy::default() } else { Anisotropy::glossy(mat) };
        let curve = if glass {
            self.value_pin(i, "brdf", 57, 2, GLASS_BRDF);
            RoughCurve::Beckmann
        } else {
            self.aniso_brdf_pin(i, !oren_nayar && aniso.saturates(rough));
            if self.gloss_ggx { RoughCurve::Ggx } else { RoughCurve::Linear }
        };
        let rough = if oren_nayar {
            // No gloss to share it with: the diffuse's σ (see `base_flat`).
            self.value_pin(i, "diffuseBrdf", 57, 2, DIFFUSE_BRDF_OREN_NAYAR);
            self.float_pin(i, "roughness", sigma);
            sigma
        } else {
            self.gloss_roughness_pins(i, mat, rough_ch, rough, curve, aniso)
        };

        // Lobe 2 of a dual lobe specular as the coating (#26): Calib-DualLobe
        // at a ratio of 0.5 / 0.15 / 0 and lobe 2 roughness 0.6 showed one
        // sharp highlight in Octane (p99 / median 10 – 13) where Iray's
        // spread (8.1 / 5.5 / 5.0); now 6.4 / 5.2 / 6.4.
        if let Some(dl) = active_dual_lobe {
            let lobe2 = dl.share * (1.0 - dl.lobe1);
            if coat <= 0.0 && lobe2 > 0.0 {
                self.rgb_pin(i, "coating", [lobe2; 3]);
                let r2 = self.gloss_roughness(dl.lobe2_roughness);
                self.float_pin(i, "coatingRoughness", r2);
                self.float_pin(i, "coatingIor", dl.ior.unwrap_or(1.5));
            }
            // Reflectivity above 0.5 as the IOR (Calib-DualLobe r 1: 0.79 →
            // 1.18 × Iray, its neighbours 1.12 – 1.16).
            if let Some(n) = dl.ior {
                self.float_pin(i, "ior", n);
            }
        }

        // A Glossy Reflectivity above 0.5 reflects more than Universal's full
        // specular, which caps at 1 (Calib-Glossy, r 1: 0.74 × Iray, also
        // at specular 2); the excess 2r − 1 as a clear coating at the gloss's
        // roughness brought it to 1.08, like its neighbours' 1.11 – 1.16.
        // Over a metal only the base's share of it: Iray's gloss is inside
        // the base (1 − w), none at all on a full metal (Calib-Map-Metal's
        // Reflectivity 1 metal: 2.14 × Iray with it).
        let excess = if mirror || specular_mode || glossy <= 0.0 || weighted.is_some() { 0.0 } else { glossy.min(1.0) * glossy_excess(glossy_reflectivity(mat)) * (1.0 - metal_w.clamp(0.0, 1.0)) };
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
        self.gloss_ggx = false;

        // Base Mixing 1 reflects Glossy Specular head-on (F0), as an IOR on
        // the dielectric layer — unless the surface is mostly refraction.
        let spec_ior = (specular_mode && !mirror && glossy > 0.0 && weighted.is_none()).then(|| specular_ior(mat));
        if refraction == 0.0 {
            if let Some(n) = spec_ior {
                self.float_pin(i, "ior", n);
            }
        }

        // Glass-like surfaces (cornea, windows): Iray refraction → transmission,
        // tinted by the refraction's colour (see `own_refraction`).
        if refraction > 0.0 {
            let colour = mat.color(if own_refraction { "Refraction Color" } else { "Glossy Color" }).unwrap_or([1.0; 3]);
            self.rgb_pin(i, "transmission", colour.map(|c| c * refraction));
            let ior = spec_ior.filter(|_| refraction < 0.5).unwrap_or_else(|| mat.float("Refraction Index").unwrap_or(1.5));
            // Iray's thin-walled refraction reflects nothing of
            // its own — Calib-Glass's thin sphere was invisible; only a
            // glossy layer shows. IOR 1 takes Octane's Fresnel reflection away.
            self.float_pin(i, "ior", if thin && self.thin_glass_ior_one { 1.0 } else { ior });
            // Iray's Abbe number spreads the IOR over the spectrum
            // (`base::abbe_number_ior`); Universal's dispersion mode 1 (its
            // default) reads the coefficient as the same Abbe number. A plain
            // float pin: it takes a float value node (6), not a texture (31) —
            // as a texture Octane dropped it on import (probe 112).
            if let Some(abbe) = dispersion_abbe(mat).filter(|_| !thin) {
                self.value_pin(i, "dispersion_coefficient_B", 6, 8, &format!("{} 0 0", num(abbe)));
            }
            if thin {
                // Thin Wall (2): no bending — Iray's thin-walled glass sphere
                // showed the background straight through (Calib-Glass).
                self.value_pin(i, "transmissionType", 57, 2, "2");
            } else if let Some(sigma_s) = scattering(mat) {
                self.scattering_pin(i, absorption(mat).unwrap_or([0.0; 3]), sigma_s);
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
        self.base_bump_pin(i, mat, "bump");
        self.round_edges_pin(i, mat);
        // In a weighted mix the mix material carries it.
        if !self.in_mix && !self.in_coat_mix {
            self.displacement_pin(i, mat);
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

/// Octane Round Edges mode Accurate (enum: 0 off, 1 fast, 2 accurate,
/// 3 convex only, 4 concave only; probe on node 137).
const ROUND_EDGES_ACCURATE: u32 = 2;
/// Round Edges rays per shading point: Octane's default (4 – 16).
const ROUND_EDGES_SAMPLES: u32 = 8;

/// DAZ's Round Corners on a material, as Iray gets them: DAZ wires
/// `state::rounded_corner_normal(radius, across_materials, roundness)` into
/// every Uber material's `geometry_bump`, the channels as they are
/// (Calib-RoundCorners .mi). MDL takes the radius in meters (MDL spec 19.3:
/// world space, whatever `mdl_meters_per_scene_unit`), so DAZ's 1 is 1 m.
/// Iray confirms meters (Calib-RoundCorners R5). `across_materials` off:
/// Octane's Round Edges sees geometry, not materials, so the export cuts
/// that material into a mesh of its own (`split_round_corner_borders`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RoundCorners {
    pub radius_m: f32,
    pub roundness: f32,
    pub across_materials: bool,
}

pub(crate) fn round_corners(mat: &Material) -> Option<RoundCorners> {
    let radius = mat.float("Round Corners Radius").unwrap_or(0.0);
    // Octane's radius pin stops at 100 (m).
    (radius > 0.0).then(|| RoundCorners {
        radius_m: radius.min(100.0),
        roundness: mat.float("Round Corners Roundness").unwrap_or(1.0).clamp(0.0, 1.0),
        across_materials: mat.float("Round Corners Across Materials").map_or(true, |v| v > 0.0),
    })
}

/// Universal's specular share for an Iray reflectivity r (default 0.5 → 1):
/// min(2r, 1). See the glossy layer in `universal`.
fn reflectivity_share(r: Option<f32>) -> f32 {
    (2.0 * r.unwrap_or(0.5)).clamp(0.0, 1.0)
}

/// Iray's dual lobe specular (skin's): two GGX lobes blended by Dual Lobe
/// Specular Ratio, together reflecting the layer's share (its weight ×
/// min(2r, 1) of Dual Lobe Specular Reflectivity, see `reflectivity_share`).
/// On Universal lobe 1 is the `specular`, lobe 2 the `coating` (#26).
#[derive(Debug, Clone, Copy, PartialEq)]
struct DualLobe {
    /// The layer's share of Universal's full specular.
    share: f32,
    /// Lobe 1's part of it.
    lobe1: f32,
    /// Lobe 2's roughness as Octane's `coatingRoughness`.
    lobe2_roughness: f32,
    /// Above reflectivity 0.5: the IOR that reflects Iray's F0 = 0.08 r,
    /// for both lobes.
    ior: Option<f32>,
}

impl DualLobe {
    /// None without a dual lobe layer.
    fn of(mat: &Material) -> Option<Self> {
        let weight = mat.float("Dual Lobe Specular Weight").unwrap_or(0.0).min(1.0);
        if weight <= 0.0 {
            return None;
        }
        let r = mat.float("Dual Lobe Specular Reflectivity");
        // PBRSkin's ratio weighs lobe 2 (default 0.15), whose roughness is
        // lobe 1's × Specular Lobe 2 Roughness Mult (0.4); Iray Uber's
        // weighs lobe 1 (0.85) and lobe 2 has its own roughness (0) — DAZ's
        // MDL (pbr_skin.mdl, irayubermaterial.mdl add_dual_lobe_specular).
        let (lobe1, r2) = match mat.float("Specular Lobe 2 Roughness Mult") {
            // PBRSkin: what DAZ passes (`pbr_material`'s roughness2_weight,
            // roughness, roughness2 — lobe 2's a multiple of lobe 1's).
            Some(mult) => {
                let p = |k: &str, d: f32| pbrskin(mat, k).unwrap_or(d);
                (1.0 - p("pbr_material.roughness2_weight", mat.float("Dual Lobe Specular Ratio").unwrap_or(0.15)), p("pbr_material.roughness", mat.float("Specular Lobe 1 Roughness").unwrap_or(0.6)) * p("pbr_material.roughness2", mult))
            }
            None => (mat.float("Dual Lobe Specular Ratio").unwrap_or(0.85), mat.float("Specular Lobe 2 Roughness").unwrap_or(0.0)),
        };
        // Iray Uber: what DAZ passes — F0 = 0.08 r, lobe 2's roughness
        // squared.
        let u = iray(mat);
        let r = u.as_ref().and_then(|u| u.float("dual_lobe_specular_reflectivity")).or_else(|| pbrskin(mat, "pbr_material.specularlevel")).map(|f0| f0 / crate::iray::REFLECTIVITY_SCALE).or(r);
        let r2_squared = u.as_ref().and_then(|u| u.float("dual_lobe_specular_roughness_2")).unwrap_or(r2 * r2);
        Some(DualLobe {
            share: weight * reflectivity_share(r),
            lobe1: lobe1.clamp(0.0, 1.0),
            lobe2_roughness: (DUAL_LOBE_2_ROUGHNESS * r2_squared).clamp(0.0, 1.0),
            ior: r.filter(|r| *r > 0.5).map(|r| ior_from_f0(crate::iray::REFLECTIVITY_SCALE * r)),
        })
    }

    /// Without a coating to spare (a Top Coat takes it), one lobe at the
    /// ratio's blend of both roughnesses (`lobe1_roughness` as Octane's).
    fn blended_roughness(&self, lobe1_roughness: f32) -> f32 {
        self.lobe1 * lobe1_roughness + (1.0 - self.lobe1) * self.lobe2_roughness
    }
}

/// Octane's `coatingRoughness` for Iray's lobe 2 roughness r2: 2 · r2².
/// Octane's coating is sharper than Iray's lobe at the same squared value:
/// Calib-DualLobe's lobe-2-only sphere (r2 0.6) has Iray's p99 / median 5.0,
/// Octane's coating 9.5 at 0.36, 6.9 at 0.6, 5.7 at 1 (its broadest); r2
/// 0.3 Iray 16, Octane ~0.12 – 0.18. At 2 · r2² Calib-DualLobe's spheres
/// (ratio 0.5 / 0.15 / 0.5 at reflectivity 0.25 / weight 0.5, r2 0.6) came
/// out 6.4 / 5.2 / 7.6 / 9.3 against Iray's 8.1 / 5.5 / 8.1 / 9.0, r2 0.1
/// 9.8 against 10.8. Beyond Octane's reach: r2 0.9 (Iray 3.3, Octane 6.6 at
/// the coating's roughness 1) (#26).
const DUAL_LOBE_2_ROUGHNESS: f32 = 2.0;

/// Iray Base Mixing 1 (specular / glossiness): Glossy Specular is the gloss's
/// head-on reflectance F0, Fresnel to white at grazing, and the diffuse
/// takes what it leaves. Octane's dielectric layer does that by its IOR:
/// n = (1 + √F0) / (1 − √F0) from Glossy Specular's strongest channel, its
/// hue the reflection's tint (`specular_hue`; a grey's is its luminance; a material
/// without the channel: F0 0.04, IOR 1.5). Calib-SpecMode (#52): Glossy
/// Specular 0 / 0.2 / 0.5 / 1, 0.5 at weight 0.5, and Morgan Hair's gloss
/// without and with its top coat rendered 1.26 / 1.13 / 0.74 / 0.35 / 0.84 /
/// 0.55 / 0.55 × Iray at the default IOR; 1.01 – 1.10 so (S1, mode 0: 1.08).
/// Black reflects nothing head-on: floored at F0 0.04 it stayed 1.26.
fn specular_ior(mat: &Material) -> f32 {
    let f0 = specular_f0(mat).clamp(0.0, 0.95);
    let s = f0.sqrt();
    ((1.0 + s) / (1.0 - s)).max(SPECULAR_MIN_IOR)
}

/// Iray's specular-mode curve still rises to 1 at grazing when Glossy
/// Specular is black (DAZ passes it as the curve's head-on value): Calib-
/// Map-SpecMode's black specular sphere had G / B 0.65 / 0.42 × Iray at
/// IOR 1 (no reflection), 0.80 / 0.64 at 1.07 (#52).
const SPECULAR_MIN_IOR: f32 = 1.1;

/// How much of a surface Iray's metallic flakes cover, as the Mix amount
/// of `flake_mix_material`: Flakes Weight × 0.4 × density^1.3. Calib-Flakes
/// (white flakes under a coat over dark blue): Iray's flakes take a sphere
/// from 0.04 to 0.20, at density 0.3 to 0.07 (≈ density^1.3). Without
/// flakes Octane rendered them 0.12 – 0.36 × Iray; at a coverage of 0.5 /
/// 0.4 / 0.3 1.05 – 1.35 / 0.88 – 1.12 / 0.72 – 0.88; Calib-Map-Misc's gold
/// flakes at weight 0.5 0.50 → 0.96 (at 0.35). Size, strength and the
/// flakes' roughness change little in Iray and aren't mapped; nor is the
/// flakes' thin film (#30).
fn flake_share(mat: &Material) -> f32 {
    let w = mat.float("Metallic Flakes Weight").unwrap_or(0.0).clamp(0.0, 1.0);
    if w <= 0.0 {
        return 0.0;
    }
    let density = mat.float("Metallic Flakes Density").unwrap_or(1.0).clamp(0.0, 1.0);
    (w * FLAKE_COVERAGE * density.powf(1.3)).clamp(0.0, 1.0)
}

/// Iray's Backscattering Weight, when it applies (not on refraction, nor
/// specular / weighted mixing — not calibrated there).
fn backscatter_weight(mat: &Material) -> f32 {
    if mat.float("Base Mixing").unwrap_or(0.0) as i32 != 0 || mat.float("Refraction Weight").unwrap_or(0.0) > 0.0 {
        return 0.0;
    }
    mat.float("Backscattering Weight").unwrap_or(0.0).clamp(0.0, 1.0)
}

/// The share of light MDL's backscattering lobe returns: its
/// `1 / max(cos θi, cos θo)` in place of `1 / cos θi` scales every
/// direction by cos θi / cos θo, on average over a Phong lobe of exponent
/// n = 2 / r² the lobe's mean cosine (n + 1) / (n + 2) = (2 + r²) /
/// (2 + 2 r²): 0.996 at r 0.09, 0.90 at 0.49. Octane's Beckmann lobe has
/// no such term (it rendered Calib-Backscatter like a unit-albedo lobe
/// integrated over the HDRI, × 1.00 – 1.05; #73).
fn retro_albedo(r: f32) -> f32 {
    (2.0 + r * r) / (2.0 + 2.0 * r * r)
}

/// Octane's Beckmann roughness for MDL's backscattering lobe at Iray's
/// (squared) roughness r. MDL: a lobe of exponent n = 2 / r² around the
/// retro direction — integrating the HDRI over such a lobe (with the
/// `1 / max(cos)` term) reproduces Iray's Calib-Backscatter spheres at
/// n ≈ 125 – 235 for r 0.09 (247) and ≈ 10 for r 0.49 (8.3), × 1.00 –
/// 1.02. Octane's mirror about the view direction reflects about a
/// Beckmann half vector of α = roughness² (renders fitted the same way:
/// roughness 0.2 / 0.3 / 0.4 → n ≈ 300 / 60 / 19), the reflected
/// direction turning twice as far: n = (2 / α² − 2) / 4. Equal lobes:
/// α = r / √(4 + r²) — roughness 0.21 at r 0.09, 0.49 at r 0.49.
fn retro_roughness(r: f32) -> f32 {
    (r / (4.0 + r * r).sqrt()).sqrt().clamp(0.0, 1.0)
}

/// `retro_normal_pin`'s shader: the direction to the viewer as a
/// tangent-space normal (0..1 encoding).
const RETRO_NORMAL_OSL: &str = "shader OrbisRetroNormal(output color c = color(0.5, 0.5, 1.0)) {
    vector n = normalize(N);
    vector t = dPdu - dot(dPdu, n) * n;
    if (length(t) > 1e-8) {
        t = normalize(t);
        vector b = cross(n, t);
        vector w = -normalize(I);
        c = color(dot(w, t), dot(w, b), dot(w, n)) * 0.5 + 0.5;
    }
}
";

/// See `flake_share`.
const FLAKE_COVERAGE: f32 = 0.4;

/// The flakes' roughness in Octane: their scattered normals, not their
/// own roughness (Calib-Flakes' roughness 0.6 sphere is like the 0.2 one
/// in Iray).
const FLAKE_ROUGHNESS: f32 = 0.5;

/// A layer's colour and weight under Iray's Color Effect (DAZ's MDL,
/// `transmit_color_calc`): 0 (Scatter Only) as they are; 1 and 2 (Scatter
/// & Transmit, … Intensity) the colour's hue at full intensity (over its
/// strongest channel) and the weight × the colour's average — a dark
/// colour thins the layer instead of darkening it (#27).
fn color_effect(mat: &Material, channel: &str, colour: [f32; 3], weight: f32) -> ([f32; 3], f32) {
    let top = max_channel(colour);
    if mat.float(channel).unwrap_or(0.0) as i32 == 0 || top <= 1e-4 {
        return (colour, weight);
    }
    (colour.map(|c| c / top), weight * (colour.iter().sum::<f32>() / 3.0))
}

/// Iray Uber's own Diffuse Overlay: a second diffuse at its weight over
/// the diffuse and translucency, which it replaces by that share
/// (`diffuse_base` in DAZ's MDL); "Weight Squared" squares the weight (the
/// .mi export), the Color Effect as for any layer (on the colour's tint).
/// Missing, Calib-Map-Effect's half blue overlay over red rendered B 0.47 ×
/// Iray (#27).
#[derive(Debug, Clone, PartialEq)]
struct DiffuseOverlay {
    /// Linear; a colour map's tint (its `color_scale`).
    colour: [f32; 3],
    colour_map: Option<String>,
    /// The weight, or a weight map's factor (`power`), squared as DAZ
    /// squares it.
    weight: f32,
    /// The weight map and the gamma to read it at: DAZ squares the map ×
    /// the weight (`square_float` over the map, Calib-Overlay V3), so
    /// twice the map's gamma at the weight² (#71).
    weight_map: Option<(String, f32)>,
}

impl DiffuseOverlay {
    fn has_map(&self) -> bool {
        self.colour_map.is_some() || self.weight_map.is_some()
    }
}

fn diffuse_overlay(mat: &Material) -> Option<DiffuseOverlay> {
    let (colour, colour_map, weight, weight_map) = match iray(mat) {
        Some(u) if u.bool("diffuse_overlay_enable") == Some(true) => {
            let (weight, weight_map) = match u.map("diffuse_overlay_weight") {
                Some(m) => {
                    let gamma = data_gamma(mat, "Diffuse Overlay Weight") * if m.squared { 2.0 } else { 1.0 };
                    (if m.squared { m.scale[0] * m.scale[0] } else { m.scale[0] }, Some((m.image, gamma)))
                }
                None => (u.float("diffuse_overlay_weight").unwrap_or(0.0), None),
            };
            let (colour, colour_map) = match u.map("diffuse_overlay_color") {
                Some(m) => (m.scale, Some(m.image)),
                None => (u.color("diffuse_overlay_color").unwrap_or([0.5; 3]), None),
            };
            (colour, colour_map, weight, weight_map)
        }
        Some(_) => return None,
        // Not Iray Uber: the flat channels only.
        None => {
            let w = mat.float("Diffuse Overlay Weight").unwrap_or(0.0).clamp(0.0, 1.0);
            if mat.image("Diffuse Overlay Weight").is_some() || mat.image("Diffuse Overlay Color").is_some() {
                return None;
            }
            let w = if mat.float("Diffuse Overlay Weight Squared").unwrap_or(0.0) > 0.0 { w * w } else { w };
            (mat.color("Diffuse Overlay Color").unwrap_or([0.5; 3]), None, w, None)
        }
    };
    let (colour, weight) = color_effect(mat, "Diffuse Overlay Color Effect", colour, weight);
    let weight = weight.clamp(0.0, 1.0);
    (weight > 0.0).then_some(DiffuseOverlay { colour, colour_map, weight, weight_map })
}

/// What Universal's albedo keeps per channel under a Base Mixing 1 gloss of
/// weight `w`: Iray leaves the diffuse 1 − w·F0 head-on whatever the
/// gloss's colour (DAZ passes F0 = Glossy Specular's strongest channel as
/// the curve, the hue as a directional tint on the reflection — its .mi
/// export), Octane 1 − w·F0·t per channel of the specular tint t (Glossy
/// Color × the hue), so a red specular let the blue base show through in
/// green and blue: Calib-ColorEffect's C8 (Glossy Specular 0.9 / 0.3 / 0.2
/// over a blue base) 1.77 × Iray, its G / B 1.5 / 2.9 ×; × (1 − w·F0) /
/// (1 − w·F0·t) 1.08, R / G / B 1.05 / 0.82 / 0.94. A grey gloss keeps 1
/// (#27).
fn specular_diffuse_share(mat: &Material, w: f32, gloss_color: [f32; 3]) -> [f32; 3] {
    let f0 = specular_f0(mat).clamp(0.0, 0.95) * w.clamp(0.0, 1.0);
    let hue = specular_hue(mat);
    [0, 1, 2].map(|k| (1.0 - f0) / (1.0 - f0 * hue[k] * gloss_color[k].clamp(0.0, 1.0)).max(1e-3))
}

/// Iray's thin-film tint seen head-on and at grazing angles: what DAZ's
/// `add_thin_film` multiplies the layers under a film by.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FilmTint {
    normal: [f32; 3],
    grazing: [f32; 3],
}

impl FilmTint {
    const NONE: Self = Self { normal: [1.0; 3], grazing: [1.0; 3] };

    /// A film of `thickness` nm at `ior`: `compute_refl` at cos 0.99 and
    /// 0.10, as `add_thin_film` takes them.
    fn of(thickness: f32, ior: f32) -> Self {
        Self { normal: film_reflectance(ior, thickness, 0.99), grazing: film_reflectance(ior, thickness, 0.10) }
    }

    fn times(self, o: Self) -> Self {
        Self { normal: [0, 1, 2].map(|k| self.normal[k] * o.normal[k]), grazing: [0, 1, 2].map(|k| self.grazing[k] * o.grazing[k]) }
    }
}

/// Iray's thin films on an Iray Uber material. DAZ's MDL
/// (`irayubermaterial.mdl`, `add_thin_film`) does not layer a film: it
/// multiplies the whole BSDF under it — diffuse, gloss, metal, refraction
/// — by `df::directional_factor` between an RGB tint head-on and one at
/// grazing angles, (1 − cos θ)^1.1 apart. Base Thin Film is over the base
/// (under the flakes and the coat), Top Coat Thin Film over everything,
/// coat included (only with a coat). Octane's film (`filmwidth`) only dims
/// the specular — Calib-ThinFilm's black spheres went from 1.04 – 1.10 to
/// 0.55 – 0.73 × Iray — so the export multiplies the colour pins by the
/// tint instead (`Ocs::pin_open`). Calib-ThinFilm: Iray's spheres are
/// tinted warm (300 nm), olive (500), purple (800) and blue (500 at IOR
/// 1.3) as the tints predict; Calib-FilmIOR: the head-on tint at IOR 1.91
/// is 0.77 – 0.83, the diffuse Iray leaves 0.77 – 0.81 (#31). Metallic
/// Flakes Thin Film waits for the flakes (#30).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Film {
    /// Over the base: every colour pin but the coating.
    under: FilmTint,
    /// Over the coat: every colour pin.
    coat: FilmTint,
}

impl Film {
    fn of(mat: &Material) -> Option<Self> {
        let film = |thickness: &str, ior: &str| {
            let d = mat.float(thickness).unwrap_or(0.0);
            (d > 0.0).then(|| FilmTint::of(d, mat.float(ior).unwrap_or(1.5)))
        };
        let base = film("Base Thin Film", "Base Thin Film IOR");
        let coat = film("Top Coat Thin Film", "Top Coat Thin Film IOR").filter(|_| mat.float("Top Coat Weight").unwrap_or(0.0) > 0.0);
        (base.is_some() || coat.is_some()).then(|| Self { under: base.unwrap_or(FilmTint::NONE), coat: coat.unwrap_or(FilmTint::NONE) })
    }

    /// The tint of Universal's colour pin `pin`, if it has one.
    fn tint(self, pin: &str) -> Option<FilmTint> {
        let t = match pin {
            "coating" => self.coat,
            "albedo" | "specular" | "transmission" => self.under.times(self.coat),
            _ => return None,
        };
        (t != FilmTint::NONE).then_some(t)
    }
}

/// Octane's Falloff Map maps the angle to (θ / 90°)^skew; 1.92 fits Iray's
/// (1 − cos θ)^1.1 within 0.015, weighted by a sphere's projected area.
const FILM_FALLOFF_SKEW: f32 = 1.92;

/// NVIDIA's `legacy_thin_film::compute_refl` (DAZ's
/// `nvidia/legacy_thin_film.mdl`), which Iray Uber's films use: the
/// interference of a film of `thickness` nm at `ior` over 16 wavelengths,
/// in linear sRGB, at the cosine `kh` to the normal.
fn film_reflectance(ior: f32, thickness: f32, kh: f32) -> [f32; 3] {
    const CIE_XYZ: [[f32; 3]; 16] = [
        [0.02986, 0.00310, 0.13609], [0.20715, 0.02304, 0.99584], [0.36717, 0.06469, 1.89550], [0.28549, 0.13661, 1.67236],
        [0.08233, 0.26856, 0.76653], [0.01723, 0.48621, 0.21889], [0.14400, 0.77341, 0.05886], [0.40957, 0.95850, 0.01280],
        [0.74201, 0.97967, 0.00060], [1.03325, 0.84591, 0.0], [1.08385, 0.62242, 0.0], [0.79203, 0.36749, 0.0],
        [0.38751, 0.16135, 0.0], [0.13401, 0.05298, 0.0], [0.03531, 0.01375, 0.0], [0.00817, 0.00317, 0.0],
    ];
    if thickness <= 0.0 || ior <= 0.0 {
        return [1.0; 3];
    }
    let eta = ior.min(1.0) / ior;
    let cos_refr = (1.0 - (eta * eta - kh * kh * eta * eta)).sqrt();
    let phase_shift = ior * cos_refr * 2.0 * thickness;
    // Fresnel at the film's IOR.
    let n = 1.0 / eta;
    let c = 1.0 - (1.0 - kh * kh) / (n * n);
    let f = if c < 0.0 {
        1.0
    } else {
        let c = c.sqrt();
        let (rp, ro) = ((c - kh * n) / (c + kh * n), (kh - c * n) / (kh + c * n));
        (0.5 * (rp * rp + ro * ro)).clamp(0.0, 1.0)
    };
    let mut xyz = [0.0f32; 3];
    for (i, w) in CIE_XYZ.iter().enumerate() {
        let lambda = 400.0 + i as f32 * 18.75;
        let intensity = (std::f32::consts::TAU * phase_shift / lambda).cos();
        for k in 0..3 {
            xyz[k] += w[k] * intensity / 16.0;
        }
    }
    let rgb = [
        7.4753 * xyz[0] - 3.5459 * xyz[1] - 1.1501 * xyz[2],
        -2.8365 * xyz[0] + 5.4915 * xyz[1] + 0.12149 * xyz[2],
        0.17036 * xyz[0] - 0.62395 * xyz[1] + 3.2329 * xyz[2],
    ];
    let (i0, i1) = (f, 1.0 - f);
    rgb.map(|v| (i0 * i0 + i1 * i1 + 2.0 * i0 * i1 * v).max(0.0))
}

/// Glossy Specular's hue (the colour over its strongest channel, whose F0
/// `specular_ior` takes): it tints a Base Mixing 1 gloss — Morgan Hair's
/// beige specular is Iray's golden sheen. Over the luminance the hue went
/// past 1 and was clipped: Calib-ColorEffect's red specular (0.9, 0.3, 0.2)
/// rendered a white reflection over the blue base, Iray's a red mirror (#27).
fn specular_hue(mat: &Material) -> [f32; 3] {
    if let Some(hue) = iray(mat).and_then(|u| u.color("glossy_directional_normal_color")).filter(|_| mat.float("Base Mixing").unwrap_or(0.0) as i32 == 1) {
        return hue;
    }
    match mat.color("Glossy Specular") {
        Some(c) if max_channel(c) > 1e-3 => c.map(|x| x / max_channel(c)),
        _ => [1.0; 3],
    }
}

/// Base Mixing 1's head-on reflectance: the strongest channel of Glossy
/// Specular, linear — what DAZ passes Iray as `glossy_reflectivity`.
fn specular_f0(mat: &Material) -> f32 {
    iray(mat).and_then(|u| u.float("glossy_reflectivity")).filter(|_| mat.float("Base Mixing").unwrap_or(0.0) as i32 == 1).unwrap_or_else(|| mat.color("Glossy Specular").map_or(0.04, max_channel))
}

/// The largest of a colour's channels.
fn max_channel(c: [f32; 3]) -> f32 {
    c.iter().copied().fold(0.0, f32::max)
}

/// Iray's volume absorption as an absorption coefficient per channel (1/m):
/// light keeps `Transmitted Color` (linear) after `Transmitted Measurement
/// Distance` (cm), so σ = −ln(colour) / distance. Black is Iray's default
/// and means none (Calib-Glass: the clear spheres stayed clear), as does
/// white or a zero distance.
fn absorption(mat: &Material) -> Option<[f32; 3]> {
    let (colour, distance_m) = match iray(mat) {
        // Iray's volume: only inside a closed surface that lets light in.
        Some(u) => (u.color("transmitted_color")?, u.float("transmitted_distance_of_measurement")?),
        None => (mat.color("Transmitted Color")?, mat.float("Transmitted Measurement Distance").unwrap_or(0.0) * CM_PER_M_INV),
    };
    let (lo, hi) = colour.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &c| (lo.min(c), hi.max(c)));
    (distance_m > 0.0 && hi > 0.0 && lo < 1.0).then(|| colour.map(|c| -c.clamp(1e-4, 1.0).ln() / distance_m))
}

/// Iray's volume scattering coefficient per channel (1/m), as DAZ passes
/// it (`add_uber_volume`, the .mi export): chromatic (`SSS Mode` 1) −ln(SSS
/// Color) / `Scattering Measurement Distance`, else `SSS Amount` / that
/// distance (cm; SSS Amount as is), × `REFRACTIVE_SCATTERING_SCALE`. None
/// without SSS. Missing on a refractive volume, Calib-Map-Misc's (SSS
/// Amount 0.3 at 0.5 cm under a blue-grey absorption) rendered 0.67 ×
/// Iray; at the full σs 1.47, × 0.5 / 0.25 / 0.1 1.14 / 0.93 / 0.79 (#28).
fn scattering(mat: &Material) -> Option<[f32; 3]> {
    if let Some(u) = iray(mat) {
        // What DAZ passes: the amount (mono, clamped to 1) or the SSS
        // Color (chromatic, logarithmic), none without SSS Amount.
        if !u.bool("sub_surface_enable")? {
            return None;
        }
        let distance_m = u.float("scattering_distance_of_measurement")?;
        let amount = u.color("sub_surface_scattering_amount")?;
        let log = u.bool("logarithmic_scatter_enable")?;
        let sigma = amount.map(|a| if log { -a.clamp(1e-4, 1.0).ln() } else { a.max(0.0) } / distance_m * REFRACTIVE_SCATTERING_SCALE);
        return sigma.iter().any(|s| *s > 0.0).then_some(sigma);
    }
    let amount = mat.float("SSS Amount")?;
    let distance_m = mat.float("Scattering Measurement Distance").unwrap_or(0.0) * CM_PER_M_INV;
    if distance_m <= 0.0 {
        return None;
    }
    let sigma = if mat.float("SSS Mode").unwrap_or(0.0) as i32 == 1 {
        mat.color("SSS Color")?.map(|c| -c.clamp(1e-4, 1.0).ln() / distance_m)
    } else {
        [amount.max(0.0) / distance_m; 3]
    };
    let sigma = sigma.map(|x| x * REFRACTIVE_SCATTERING_SCALE);
    sigma.iter().any(|s| *s > 0.0).then_some(sigma)
}

/// See `scattering`: one sphere's fit — Octane's medium behind a
/// refractive surface scatters more light back out than Iray's.
const REFRACTIVE_SCATTERING_SCALE: f32 = 0.3;

/// A figure's eye surface by its material name (Genesis 3 / 8 / 9: Irises,
/// Pupils, Sclera; Cornea and EyeMoisture are refractive anyway).
fn is_eye(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    // Genesis 9 keeps the whole eye (sclera, iris, pupil) in one surface per
    // eye, `Eye_Left` / `Eye_Right`: as skin its random walk laid a milky
    // veil over the iris (Gladiator looked blind).
    ["iris", "irises", "pupil", "sclera"].iter().any(|e| n.ends_with(e) || n.ends_with(&format!("{e}s")) || n.contains(&format!("_{e}")))
        || n.ends_with("eye_left")
        || n.ends_with("eye_right")
}

/// Iray's Base Color Effect 1 (Scatter & Transmit) on a translucent
/// surface: its diffuse and its translucency as Universal's albedo and
/// transmission shares. See `universal`.
const SCATTER_TRANSMIT_DIFFUSE: f32 = 0.4;
const SCATTER_TRANSMIT_TRANSMISSION: f32 = 2.0;
/// A translucent prop's volume (`universal`): its transmission × the volume
/// factor × this. Octane's response to it is steep: 0.6 / 0.8 put
/// Calib-SSSColor's chromatic spheres at 0.71 – 0.84 / 1.00 – 1.30 × Iray.
const PROP_VOLUME_LEVEL: f32 = 0.7;
/// Base Color Effect 2 (Scatter & Transmit Intensity): see `universal`.
const SCATTER_TRANSMIT_INTENSITY_DIFFUSE: f32 = 0.95;
const SCATTER_TRANSMIT_INTENSITY_TRANSMISSION: f32 = 1.55;

/// `Ocs::layered_coat_material`: a Top Coat with anisotropy outside
/// Layering Mode 1 (the weighted coat takes it on its own metal node).
fn layered_top_coat(mat: &Material) -> bool {
    let coated = mat.float("Top Coat Weight").unwrap_or(0.0) > 0.0 || mat.image("Top Coat Weight").is_some();
    coated && mat.float("Top Coat Layering Mode").unwrap_or(0.0) as i32 != 1 && Anisotropy::top_coat(mat).amount > 0.0
}

/// Whether Iray's Top Coat is in Layering Mode 1 (Weighted) and on: see
/// `Ocs::weighted_coat_material`.
fn weighted_top_coat(mat: &Material) -> bool {
    mat.float("Top Coat Layering Mode").unwrap_or(0.0) as i32 == 1 && (mat.float("Top Coat Weight").unwrap_or(0.0) > 0.0 || mat.image("Top Coat Weight").is_some())
}

/// The top coat's roughness as Octane takes it: Top Coat Glossiness under
/// Base Mixing 1 and 2, else Top Coat Roughness, as Iray squares it.
fn top_coat_roughness(mat: &Material) -> f32 {
    if let Some(r) = iray(mat).and_then(|u| u.float("top_coat_roughness")) {
        return r.clamp(0.0, 1.0);
    }
    let (r, inv) = match (mat.float("Base Mixing").unwrap_or(0.0) as i32, mat.float("Top Coat Glossiness")) {
        (1 | 2, Some(g)) => (1.0 - g, true),
        _ => (mat.float("Top Coat Roughness").unwrap_or(0.0), false),
    };
    iray_roughness(mat, r, inv).clamp(0.0, 1.0)
}

/// Universal's `coatingIor` for Iray's Top Coat. Layering Mode 0
/// (Reflectivity, the most common: the cornea, 2128 materials in My
/// Library) reflects F0 = 0.08 × Reflectivity head-on (0.5 → 0.04, IOR
/// 1.5): Calib-TopCoat's Reflectivity 1 sphere 0.75 → 1.09 × Iray (0.5:
/// 1.11), and the cornea's 0.3 no longer mirrors the room over the iris
/// (Calib-Eye, #20). DAZ Studio passes neither Top Coat IOR nor the Curve
/// Normal / Grazing / Exponent to the MDL (its .mi export,
/// docs/daz/iray-uber-mdl.md): Fresnel (2) reflects at the MDL's IOR 1.5
/// whatever the channel — Calib-TopCoat's IOR 1.5 and 2 spheres render
/// alike in Iray — and Custom Curve (3) at its defaults, 0 head-on to 1 at
/// grazing by (1 − cos θ)^5 (`CURVE_COAT_IOR`, #25).
fn top_coat_ior(mat: &Material) -> f32 {
    match mat.float("Top Coat Layering Mode").unwrap_or(0.0) as i32 {
        0 => ior_from_f0(
            iray(mat)
                .and_then(|u| u.float("top_coat_curve_strength"))
                // PBRSkin reflects by its own Top Coat Reflectivity.
                .or_else(|| pbrskin(mat, "pbr_add_clearcoat.top_coat_specular_level"))
                .unwrap_or_else(|| 0.08 * mat.float("Reflectivity").unwrap_or(0.5)),
        ),
        3 => CURVE_COAT_IOR,
        _ => 1.5,
    }
}

/// Universal's `coatingIor` for Iray's Custom Curve coat (0 head-on):
/// Calib-TopCoat's curve spheres 2.5 × Iray at the old Top Coat IOR 1.5,
/// 1.52 at 1.22 (Fresnel matching the curve's hemispherical average,
/// 0.048), 1.23 at 1.07, 1.18 at 1 (no coat; the plain coated spheres
/// 1.04 – 1.11). Iray shows no head-on reflection on them, only a rim:
/// 1.07 keeps that rim.
const CURVE_COAT_IOR: f32 = 1.07;

/// The dielectric IOR that reflects `f0` head-on: (1 + √F0) / (1 − √F0).
fn ior_from_f0(f0: f32) -> f32 {
    let s = f0.clamp(0.0, 0.9).sqrt();
    (1.0 + s) / (1.0 - s)
}

/// DAZ's OmniHair ("OmniHair Root-Tip Blend", a brick graph on
/// `OmniSurface::OmniHairBase`): Chiang's hair BSDF — as Octane's Hair is —
/// absorbing by melanin and a base colour. Root and tip are averaged.
struct OmniHair {
    /// Octane's Melanin and Pheomelanin (its melanin mode): OmniHair's
    /// eumelanin and pheomelanin concentrations — as is, the crown of
    /// Calib-OmniHair came out 0.99 (blonde) / 1.01 (brown) × Iray; the
    /// absorption turned into Octane's albedo (Chiang's colour mapping)
    /// rendered it 0.65 / 0.78.
    eumelanin: f32,
    pheomelanin: f32,
    ior: f32,
    /// OmniHair hands Chiang's BSDF its roughness already mapped (`compute_v`,
    /// `compute_s` on its inputs): passed on so (the unmapped 0.2 rendered
    /// the same crown).
    roughness: f32,
    azimuthal: f32,
    /// Its diffuse lobe ("Highlight" in DAZ).
    diffuse: f32,
    diffuse_colour: [f32; 3],
    opacity: f32,
}

/// OmniHairBase's absorption per unit eumelanin and pheomelanin.
const OMNI_EUMELANIN: [f32; 3] = [0.506, 0.841, 1.653];
const OMNI_PHEOMELANIN: [f32; 3] = [0.343, 0.733, 1.924];

/// OmniHairBase's longitudinal roughness mapping (`compute_v`).
fn omni_v(b: f32) -> f32 {
    let t = 0.726 * b + 0.812 * b * b + 3.7 * b.powi(20);
    t * t
}

/// OmniHairBase's azimuthal roughness mapping (`compute_s`).
fn omni_s(b: f32) -> f32 {
    0.265 * b + 1.194 * b * b + 5.372 * b.powi(22)
}

/// Chiang et al.'s colour ↔ absorption factor at azimuthal roughness β:
/// σ = (ln c / f(β))², c = exp(−√σ · f(β)).
fn chiang_factor(b: f32) -> f32 {
    let (b2, b4) = (b * b, b.powi(4));
    5.969 - 0.215 * b + 2.532 * b2 - 10.73 * b2 * b + 5.574 * b4 + 0.245 * b4 * b
}

/// The eumelanin and pheomelanin (≥ 0) whose absorption comes closest to σ.
fn melanin_fit(sigma: [f32; 3]) -> (f32, f32) {
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (e, p) = (OMNI_EUMELANIN, OMNI_PHEOMELANIN);
    let (ee, pp, ep, es, ps) = (dot(e, e), dot(p, p), dot(e, p), dot(e, sigma), dot(p, sigma));
    let det = ee * pp - ep * ep;
    let (a, b) = ((es * pp - ps * ep) / det, (ps * ee - es * ep) / det);
    if a < 0.0 {
        (0.0, (ps / pp).max(0.0))
    } else if b < 0.0 {
        ((es / ee).max(0.0), 0.0)
    } else {
        (a, b)
    }
}

/// The OmniHair a brick shader renders with, from its DAZ channels.
fn omni_hair(mat: &Material) -> Option<OmniHair> {
    mat.channel("Melanin")?;
    mat.channel("Hair Root Color")?;
    let avg = |a: &str, b: &str, d: f32| (mat.float(a).unwrap_or(d) + mat.float(b).or(mat.float(a)).unwrap_or(d)) / 2.0;
    let avg_colour = |a: &str, b: &str| {
        let (x, y) = (mat.color(a).unwrap_or([1.0; 3]), mat.color(b).or(mat.color(a)).unwrap_or([1.0; 3]));
        [0, 1, 2].map(|k| (x[k] + y[k]) / 2.0)
    };
    // "Melanin Presets": white … black, 7 = custom (the slider).
    const PRESETS: [f32; 7] = [0.0, 0.0025, 0.10, 0.175, 0.30, 0.65, 1.0];
    let melanin = match mat.float("Melanin Presets").map(|p| p as usize) {
        Some(p) if p < PRESETS.len() => PRESETS[p],
        _ => mat.float("Melanin").unwrap_or(1.0),
    };
    let redness = mat.float("Melanin Redness").unwrap_or(0.5);
    let m = melanin * melanin * 2.4;
    let (eu, pheo) = (m * (1.0 - redness), m * redness);
    let sigma_m = [0, 1, 2].map(|k| (eu * OMNI_EUMELANIN[k] + pheo * OMNI_PHEOMELANIN[k]).max(0.0));
    let roughness = omni_v(avg("Roughness", "Roughness Tip", 0.1));
    let anisotropic = mat.float("Anisotropic Roughness").unwrap_or(0.0) > 0.0;
    let az_input = avg("Azimuthal Roughness", "Azimuthal Roughness Tip", 0.2);
    let azimuthal = if anisotropic { omni_s(az_input) } else { roughness };
    let base = avg_colour("Hair Root Color", "Hair Tip Color");
    let weight = mat.float("Diffuse Strength").unwrap_or(1.0);
    let (eumelanin, pheomelanin) = if base == [1.0; 3] && weight == 1.0 {
        (eu, pheo)
    } else {
        // The base colour's absorption (× 2.4, as the MDL does), blended
        // toward the melanin's by √melanin — a white base at Diffuse
        // Strength ≥ 0.42 absorbs nothing, which leaves the melanin × √m;
        // a coloured one is fitted as the two pigments (least squares, ≥ 0).
        let f = chiang_factor(if anisotropic { az_input } else { roughness });
        let t = melanin.max(0.0).sqrt();
        let sigma = [0, 1, 2].map(|k| {
            let s = ((base[k] * weight * 2.4).clamp(0.001, 1.0).ln() / f).powi(2);
            s + (sigma_m[k] - s) * t
        });
        melanin_fit(sigma)
    };
    Some(OmniHair {
        eumelanin,
        pheomelanin,
        ior: mat.float("IOR").unwrap_or(1.55).max(1.0 + 1e-5),
        roughness: roughness.clamp(0.0, 1.0),
        azimuthal: azimuthal.clamp(0.0, 1.0),
        diffuse: mat.float("Highlight Weight").unwrap_or(0.0).clamp(0.0, 1.0),
        diffuse_colour: avg_colour("Highlight Root Color", "Tip Highlight Color"),
        opacity: if mat.float("Enable Opacity").unwrap_or(0.0) > 0.0 { mat.float("Cutout Opacity").unwrap_or(1.0) } else { 1.0 },
    })
}

/// Octane Hair albedo from Iray's Blended Dual Lobe hair: its hair colour
/// and, more, its transmission colour (root / tip means, linear) — fitted on
/// the crown of Calib-Hair (blond) and two copies with dark brown hair or
/// transmission colours, strands at `LINE_WIDTH_SCALE`: 0.97 / 1.03 / 1.07 ×
/// Iray in luminance; Octane's Hair is linear in its albedo (× 0.6 → × 0.6).
const HAIR_COLOUR_SHARE: f32 = 0.046;
const HAIR_TRANSMISSION_SHARE: f32 = 0.110;
/// The highlight colour's share of the Hair albedo per unit Highlight
/// Weight: Iray's `dual_lobe_hair` lays its highlight glossy over the rest
/// by a `weighted_layer` (no Fresnel), so Highlight Weight hw takes hw of
/// the light and leaves 1 − hw. Fitted on Calib-Hair-hl (hw 0.8, blue
/// highlight, local): hair median 1.00 / 1.05 / 1.03 × Iray (RGB, linear),
/// mean luminance on par with Calib-Hair's (0.84 vs 0.85 × Iray); 0.25 was
/// 1.48 × in blue (#42).
const HAIR_HIGHLIGHT_SHARE: f32 = 0.15;

/// The Hair albedo at one end of the strand: the hair and transmission
/// colours' shares under 1 − `highlight_weight`, the highlight colour's
/// over it (`HAIR_HIGHLIGHT_SHARE`).
fn hair_albedo(hair: [f32; 3], transmission: [f32; 3], highlight: [f32; 3], highlight_weight: f32) -> [f32; 3] {
    let hw = highlight_weight;
    [0, 1, 2].map(|k| (1.0 - hw) * (HAIR_COLOUR_SHARE * hair[k] + HAIR_TRANSMISSION_SHARE * transmission[k]) + hw * HAIR_HIGHLIGHT_SHARE * highlight[k])
}
/// Segments that sample DAZ's root-to-tip blend into the Hair albedo's
/// Gradient when its bias / gain bend it (`hair_albedo_pin`).
const HAIR_BLEND_POINTS: usize = 16;

/// The inner control points (strand positions 0 – 1) of the Hair albedo's
/// Gradient: spaced evenly along the root-to-tip blend's curve (t, w), so
/// its steep end gets more of them — linear in between, within 1 % of the
/// curve at bias / gain 0.2 – 0.8 (4 % at 0.05 – 0.95; even spacing in t
/// was 9 %).
fn blend_points(bias: f32, gain: f32) -> Vec<f32> {
    const M: usize = 256;
    let curve: Vec<(f32, f32)> = (0..=M).map(|k| k as f32 / M as f32).map(|t| (t, crate::iray::root_to_tip_blend(t, bias, gain))).collect();
    let mut length = vec![0.0f32];
    for w in curve.windows(2) {
        length.push(length.last().unwrap() + (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1));
    }
    let total = *length.last().unwrap();
    let mut out: Vec<f32> = (1..HAIR_BLEND_POINTS).map(|j| curve[length.partition_point(|l| *l < total * j as f32 / HAIR_BLEND_POINTS as f32).min(M)].0).collect();
    out.dedup();
    out
}

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

/// What Iray's skin reflects, as `a · D + b · Tc²` per channel (D the
/// diffuse map × its colour, Tc the translucency map × its colour; see
/// `Textures::derived_skin`): the diffuse layer's (1 − w)^1.3 of the map and
/// the subsurface layer's w · 0.55 · Tc², from the translucency colour alone.
/// Calib-Skin's Iray probes (eight Genesis 8 skins; translucency at w and
/// 0.5, diffuse or translucency map white): the (1 − w)^1.3 fit w 0.9 and
/// 0.5 within 0.94 – 1.03; Tc²'s factor came out 0.45 – 0.65 (red), 0.44 –
/// 0.66 (green), 0.27 – 0.63 (blue) over the figures, and a white diffuse
/// map left it as it was. D · Tc with a factor falling with brightness fit
/// worse (its estimate from map means misled: their padding).
fn skin_albedo(mat: &Material, w: f32) -> ([f32; 3], [f32; 3]) {
    let base = mat.base_color();
    let tc = mat.color("Translucency Color").unwrap_or([1.0; 3]);
    let a = base.map(|x| x * (1.0 - w).max(0.0).powf(SKIN_DIFFUSE_EXPONENT));
    let volume = volume_factor(mat);
    let b = [0, 1, 2].map(|k| w * SKIN_SSS_SHARE * tc[k] * tc[k] * volume[k]);
    (a, b)
}

/// How much more (or less) Iray's subsurface layer reflects with the
/// skin's volume than with the Genesis 8 skins' it was fit on, per channel:
/// (Rd(α) / Rd(α₀))^0.5, Rd Jensen's diffusion reflectance (η 1.4) of the
/// volume's reduced single-scattering albedo α. DAZ's MDL
/// (`add_uber_volume`): σa = −ln(Transmitted Color) / d_t, σs = −ln(SSS
/// Color) / d_s for chromatic SSS (`SSS Mode` 1), SSS Amount / d_s for mono,
/// the colours linear. Calib-Skin's variants (Victoria 8, Monique, Zurie:
/// SSS Color grey / blue / deep, mono at Amount 2 and 0.5) over the
/// subsurface layer's share of the base render: within ~10 % on Victoria
/// and Monique (Zurie, more diffuse, ~20 %); Rd itself overshoots. SSS
/// Reflectance Tint did nothing on them (#28).
fn volume_factor(mat: &Material) -> [f32; 3] {
    let alpha = volume_albedo(mat);
    let reference = volume_albedo_of(SKIN_TRANSMITTED.map(colour_to_linear), SKIN_TRANSMITTED_DISTANCE, Scattering::Chromatic(SKIN_SSS_COLOR.map(colour_to_linear)), SKIN_SCATTERING_DISTANCE, 0.0);
    [0, 1, 2].map(|k| (diffusion_reflectance(alpha[k]) / diffusion_reflectance(reference[k])).sqrt())
}

/// See [`volume_factor`]: the Genesis 8 skins' volume (Victoria 8's,
/// sRGB colours, DAZ cm).
const SKIN_TRANSMITTED: [f32; 3] = [0.9764706, 0.4823529, 0.3529412];
const SKIN_TRANSMITTED_DISTANCE: f32 = 0.125;
const SKIN_SSS_COLOR: [f32; 3] = [0.9764706, 0.6941177, 0.7607843];
const SKIN_SCATTERING_DISTANCE: f32 = 0.015;

enum Scattering {
    /// `SSS Color` (linear): σs = −ln(colour) / d_s.
    Chromatic([f32; 3]),
    /// `SSS Amount`: σs = amount / d_s.
    Mono(f32),
}

/// The skin volume's reduced single-scattering albedo per channel; channels
/// the material leaves out are the Genesis 8 skins'. See [`volume_factor`].
fn volume_albedo(mat: &Material) -> [f32; 3] {
    let linear = |c: [f32; 3]| c.map(colour_to_linear);
    let chromatic = Scattering::Chromatic(mat.color("SSS Color").unwrap_or(linear(SKIN_SSS_COLOR)));
    let scattering = match mat.float("SSS Amount") {
        Some(amount) if mat.float("SSS Mode").unwrap_or(0.0) as i32 != 1 => Scattering::Mono(amount),
        _ => chromatic,
    };
    volume_albedo_of(
        mat.color("Transmitted Color").unwrap_or(linear(SKIN_TRANSMITTED)),
        mat.float("Transmitted Measurement Distance").filter(|d| *d > 0.0).unwrap_or(SKIN_TRANSMITTED_DISTANCE),
        scattering,
        mat.float("Scattering Measurement Distance").filter(|d| *d > 0.0).unwrap_or(SKIN_SCATTERING_DISTANCE),
        mat.float("SSS Direction").unwrap_or(0.0).clamp(-0.95, 0.95),
    )
}

/// Colours linear.
fn volume_albedo_of(transmitted: [f32; 3], d_t: f32, scattering: Scattering, d_s: f32, g: f32) -> [f32; 3] {
    let extinction = |c: f32| -c.clamp(1e-4, 0.9999).ln();
    [0, 1, 2].map(|k| {
        let sigma_a = extinction(transmitted[k]) / d_t;
        let sigma_s = match scattering {
            Scattering::Chromatic(c) => extinction(c[k]),
            Scattering::Mono(amount) => amount.max(0.0),
        } / d_s
            * (1.0 - g);
        sigma_s / (sigma_s + sigma_a).max(1e-6)
    })
}

/// Jensen's diffusion reflectance of a semi-infinite medium (dipole, η 1.4)
/// at reduced albedo `alpha`.
fn diffusion_reflectance(alpha: f32) -> f32 {
    const ETA: f32 = 1.4;
    let fdr = -1.44 / (ETA * ETA) + 0.71 / ETA + 0.668 + 0.0636 * ETA;
    let a = (1.0 + fdr) / (1.0 - fdr);
    let s = (3.0 * (1.0 - alpha.clamp(0.0, 1.0))).sqrt();
    (alpha / 2.0 * (1.0 + (-4.0 / 3.0 * a * s).exp()) * (-s).exp()).max(1e-6)
}

/// See [`skin_albedo`].
const SKIN_DIFFUSE_EXPONENT: f32 = 1.3;
/// See [`skin_albedo`].
const SKIN_SSS_SHARE: f32 = 0.55;
/// Octane / Iray's hue offset on OpenPBR's random walk, undone on the baked
/// colour (`Textures::derived_skin`): with the colour as Iray's, the eight
/// Calib-Skin figures came out 0.86 / 1.20 / 1.48 × Iray per channel.
const SKIN_HUE_OPENPBR: [f32; 3] = [1.16, 0.83, 0.68];
/// See `Textures::derived_skin`: Calib-Skin 0.2 (0.11 left Monique at 1.25).
const SKIN_DARK_GAIN: f32 = 0.2;
/// See `Textures::derived_skin`: the saturation where the hue offset stops.
/// Calib-Skin's eight figures, front light, worst channel's |log| median
/// 0.10 → 0.085 (Zurie 0.89 / 0.75 / 0.64 → 0.81 / 0.83 / 0.80 × Iray,
/// Gabriela's blue 0.88 → 0.97; Angel's 1.06 → 1.24); 0.3 0.102 (#19).
const SKIN_HUE_SAT_FLOOR: f32 = 0.45;

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

/// The parameters DAZ hands Iray for an Iray Uber material, where
/// `crate::iray` has verified them: the Octane side reads those instead of
/// converting the channels itself (#62). None for other shaders and for
/// branches no `.mi` fixture covers — the callers then fall back.
fn iray(mat: &Material) -> Option<crate::iray::Uber> {
    crate::iray::Uber::of(mat)
}

/// Iray Uber's Abbe number when DAZ enables dispersion (`dispersion_enable`:
/// refraction and Abbe > 0, the .mi export), else None.
fn dispersion_abbe(mat: &Material) -> Option<f32> {
    match iray(mat) {
        Some(u) => u.bool("dispersion_enable").filter(|&on| on).and_then(|_| u.float("abbe_number")),
        None => mat.float("Abbe").filter(|_| mat.float("Refraction Weight").unwrap_or(0.0) > 0.0),
    }
    .filter(|&a| a > 0.0)
}

/// Iray Uber's Diffuse Roughness (or Diffuse Overlay Roughness): the
/// Oren-Nayar σ of its `diffuse_reflection_bsdf` (DAZ's MDL), as DAZ hands
/// it over (the .mi export: the value as is). A map is left out.
fn diffuse_roughness(mat: &Material, overlay: bool) -> f32 {
    let (param, channel) = if overlay { ("diffuse_overlay_roughness", "Diffuse Overlay Roughness") } else { ("diffuse_roughness", "Diffuse Roughness") };
    iray(mat).and_then(|u| u.float(param)).or_else(|| mat.float(channel)).unwrap_or(0.0).clamp(0.0, 1.0)
}

/// What Iray's rough diffuse keeps of a Lambertian one head-on: the A
/// term of Fujii's improved Oren-Nayar, which `diffuse_reflection_bsdf`
/// is since MDL 1.10 / Iray 2024.1 (DAZ Studio 6 ships Iray 2025.0):
/// π A = 1 / (1 + (1/2 − 2 / (3π)) σ), σ the MDL roughness as is. Seen
/// head-on (n = v) the B term is zero under any light; elsewhere it only
/// redistributes toward or away from the light. Iray's Calib-Diffuse
/// overlay at σ 1 kept 0.77 of the Lambertian at the sphere's centre
/// (π A 0.777), 0.74 / 0.70 on the middle 60 % / 90 % disc (back-lit);
/// the classic qualitative A, 1 − 0.5 σ² / (σ² + 0.33) = 0.624, left the
/// overlay 0.85 – 0.89 × Iray (#76).
fn oren_nayar_flat(sigma: f32) -> f32 {
    1.0 / (1.0 + (0.5 - 2.0 / (3.0 * std::f32::consts::PI)) * sigma)
}

/// The roughness Iray gets (squared) for the lobe `roughness_source`
/// picks: the dual lobe's lobe 1 when there is one, else the glossy
/// layer's from the channel Base Mixing reads.
fn iray_gloss_roughness(mat: &Material) -> Option<f32> {
    let u = iray(mat)?;
    match mat.roughness_source().0 {
        Some("Specular Lobe 1 Roughness") => u.float("dual_lobe_specular_roughness_1"),
        _ => u.float("glossy_roughness"),
    }
}

/// A PBRSkin argument DAZ hands Iray (`crate::iray::pbrskin`), as a float.
fn pbrskin(mat: &Material, param: &str) -> Option<f32> {
    if !crate::iray::is_pbrskin(mat) {
        return None;
    }
    match crate::iray::pbrskin(mat).get(param)? {
        crate::iray::Arg::Float(f) => Some(*f),
        _ => None,
    }
}

/// Glossy Reflectivity as DAZ's 0–1 slider r, from the F0 = 0.08 r Iray gets.
fn glossy_reflectivity(mat: &Material) -> Option<f32> {
    match iray(mat).and_then(|u| u.float("glossy_reflectivity")).filter(|_| mat.float("Base Mixing").unwrap_or(0.0) as i32 == 0) {
        Some(f0) => Some(f0 / crate::iray::REFLECTIVITY_SCALE),
        None => mat.float("Glossy Reflectivity"),
    }
}

/// Iray's metallic weight (Base Mixing 1 has none).
fn metallic(mat: &Material) -> f32 {
    iray(mat).and_then(|u| u.float("metallic_weight")).unwrap_or_else(|| mat.metallic())
}

/// Iray's roughness on Octane's GGX gloss (`Ocs::gloss_roughness`).
fn ggx_roughness(r: f32) -> f32 {
    (r + GGX_ROUGHNESS_CURVE * r * r).min(r.max(0.0).sqrt()).clamp(0.0, 1.0)
}

/// `Ocs::gloss_roughness`'s curve.
const GGX_ROUGHNESS_CURVE: f32 = 1.85;

/// How Iray's (MDL, squared) roughness `r` becomes Universal's `roughness`
/// pin, for the BSDF model the pin goes with.
#[derive(Clone, Copy, Debug, PartialEq)]
enum RoughCurve {
    /// As is (Octane's default BSDF, the skins' shaders).
    Linear,
    /// `ggx_roughness`: an Uber gloss under GGX energy preserving.
    Ggx,
    /// √r under Beckmann (`brdf` 1): rough glass (`GLASS_BRDF`).
    Beckmann,
}

impl RoughCurve {
    fn apply(self, r: f32) -> f32 {
        match self {
            RoughCurve::Linear => r.clamp(0.0, 1.0),
            RoughCurve::Ggx => ggx_roughness(r),
            RoughCurve::Beckmann => r.clamp(0.0, 1.0).sqrt(),
        }
    }

    /// Its part of a derived roughness map's cache key (the bool it was).
    fn key(self) -> &'static str {
        match self {
            RoughCurve::Linear => "false",
            RoughCurve::Ggx => "true",
            RoughCurve::Beckmann => "beckmann",
        }
    }
}

/// Universal's `brdf` for a refractive body: Beckmann (1; probe 104).
/// Iray's Uber refracts through `simple_glossy_bsdf` (DAZ's MDL
/// `choose_bsdf`, `dfm_glossy`): a Phong distribution of exponent 2 / r²
/// with no multiple-scattering compensation (`multiscatter_tint` left at
/// 0, MDL spec 21.1.3: rough lobes lose albedo). Phong of exponent n is
/// Beckmann at α = √(2 / (n + 2)) ≈ r (Walter et al. 2007), and Octane's
/// roughness pin is √α. Calib-Dispersion-Rough's rough spheres (#72):
/// GGX energy preserving 1.19 / 1.19 × Iray at r 0.36 (the energy
/// Iray's lobe loses comes back), plain GGX 0.86 / 0.85 (its long tail
/// loses more), Beckmann at √r 1.09 / 1.07 with Iray's blur (detail 1.4 /
/// 1.6 vs 1.2 / 1.5; at r 0.09 8.3 / 5.4 vs 8.3 / 5.4).
const GLASS_BRDF: &str = "1";

/// Iray Uber's anisotropy of a glossy lobe (`base::anisotropy_conversion`,
/// MDL SDK's `base.mdl`): roughness_u = r, roughness_v = r / (1 − a)
/// (r / 0.0001 at a = 1), the tangent turned by 2π · rotation about the
/// normal.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Anisotropy {
    amount: f32,
    rotation: f32,
}

impl Anisotropy {
    /// The glossy layer's (the metal's and the refraction's too): Glossy
    /// Anisotropy and Glossy Anisotropy Rotations as DAZ hands them Iray.
    /// Backscattering Anisotropy never reaches Iray: DAZ's .mi export leaves
    /// it out (Calib-Backscatter B8, 0.8), so Iray's backscattering lobe stays
    /// isotropic.
    fn glossy(mat: &Material) -> Self {
        let (a, rot) = match iray(mat) {
            Some(u) => (u.float("glossy_anisotropy").unwrap_or(0.0), u.float("glossy_anisotropy_rotation").unwrap_or(0.0)),
            None => (mat.float("Glossy Anisotropy").unwrap_or(0.0), mat.float("Glossy Anisotropy Rotations").unwrap_or(0.0)),
        };
        Self::of(a, rot)
    }

    /// The Top Coat's: Top Coat Anisotropy and Top Coat Rotations.
    fn top_coat(mat: &Material) -> Self {
        let a = iray(mat).and_then(|u| u.float("top_coat_anisotropy")).unwrap_or_else(|| mat.float("Top Coat Anisotropy").unwrap_or(0.0));
        let rot = iray(mat).and_then(|u| u.float("top_coat_anisotropy_rotation")).unwrap_or_else(|| mat.float("Top Coat Rotations").unwrap_or(0.0));
        Self::of(a, rot)
    }

    fn of(amount: f32, rotation: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            return Self::default();
        }
        Self { amount, rotation }
    }

    /// roughness_v / roughness_u.
    fn stretch(self) -> f32 {
        if self.amount >= 1.0 { 1.0e4 } else { 1.0 / (1.0 - self.amount) }
    }

    /// Whether Iray's long axis r · stretch runs past roughness 1, where
    /// Octane's is capped (`aniso_brdf_pin`).
    fn saturates(self, r: f32) -> bool {
        self.amount > 0.0 && r * self.stretch() > 1.0
    }

    /// Octane's `rotation` (turns, its slider 0 – 1) for Iray's: the other
    /// way round, 1 − rotation. Calib-Aniso2's K1 (0.125) and K2 (0.375)
    /// mirror each other in Octane; as exported each looked like Iray's
    /// other one, at 1 − r both match (#23). None when unrotated.
    fn octane_rotation(self) -> Option<f32> {
        let r = (-self.rotation).rem_euclid(1.0);
        (r > 1e-6 && r < 1.0 - 1e-6).then_some(r)
    }
}

/// Octane's roughness and anisotropy for Iray's roughness r along u and
/// r · `stretch` along v (capped at 1), each axis through the GGX curve when
/// `ggx`: the long axis v and (v − u) / (v + u) — Octane's anisotropy 1
/// stretches the highlight along v ("vertical"). Octane's roughness is the
/// long axis, its anisotropy narrows the other: at the axes' geometric mean
/// Calib-Aniso's streaks came out short and bright (rough 0.1, aniso 1:
/// highlight elongation 1.96 against Iray's 9.9, mean 1.52 × Iray), at v
/// 9.85 and 1.15 × (#23).
fn anisotropic_roughness(r: f32, stretch: f32, curve: RoughCurve) -> (f32, f32) {
    let curve = |x: f32| curve.apply(x);
    if stretch == 1.0 {
        return (curve(r), 0.0);
    }
    let (u, v) = (curve(r), curve((r * stretch).min(1.0)));
    if u + v <= 0.0 {
        return (0.0, 0.0);
    }
    (v, ((v - u) / (v + u)).clamp(-1.0, 1.0))
}

/// Iray's metal at Metallic Weight w (DAZ's MDL, `metallic_layered_base`):
/// a `weighted_layer` at w of a metal coat that is itself at weight w over
/// black — DAZ passes the same w to both (its .mi export) — over the
/// dielectric base: w² metal + (1 − w) base, the rest lost. Universal's
/// `metallic` m blends m metal + (1 − m) base, and its metal takes the
/// albedo's colour too, so both albedo and specular × k with m k = w² and
/// (1 − m) k = 1 − w: k = 1 − w + w², m = w² / k. Scaling only the base
/// (m = w², albedo × 1 / (1 + w)) darkened the metal: Calib-PBRSkin's
/// metals 0.9 / 0.75 went from 1.01 / 1.24 to 0.55 / 0.74 × Iray. At m = w
/// Calib-Map-Metal's red spheres at 0.25 / 0.5 / 0.75 rendered 1.23 / 1.38
/// / 1.39 × Iray (#59). Returns (m, k).
/// Universal's (metallic, base scale) for a material's metal: Iray Uber's
/// `metal_shares`; PBRSkin's metal is one `weighted_layer` at Metallicity
/// (`pbr_skin.mdl`: the base colour's tint over the dual lobe specular, the
/// rest at 1 − w), so Universal's own blend at w, the base unscaled.
fn metal_mix(mat: &Material, w: f32) -> (f32, f32) {
    if crate::iray::is_pbrskin(mat) { (w.clamp(0.0, 1.0), 1.0) } else { metal_shares(w) }
}

fn metal_shares(w: f32) -> (f32, f32) {
    let w = w.clamp(0.0, 1.0);
    let k = 1.0 - w + w * w;
    (w * w / k, k)
}

/// The share of a translucent metal's non-metal diffuse its translucency
/// takes, per unit of Translucency Weight: see `universal`.
const METAL_TRANSLUCENCY_LOSS: f32 = 0.35;

/// Metallic Weight below this (without a map) is left out: see `universal`.
const NEGLIGIBLE_METALLIC: f32 = 0.05;

/// How much brighter OpenPBR gets a mono-SSS skin's colour than Standard
/// Surface's `skin_tint`, by translucency `w`, between the points of
/// `OPENPBR_SKIN_LIFT`. For skins outside `skin_albedo`'s rule (mono SSS
/// with Base Color Effect 1: the Genesis 8 base skin).
fn openpbr_skin_lift(w: f32) -> f32 {
    let w = w.clamp(0.0, 1.0);
    let i = OPENPBR_SKIN_LIFT.windows(2).position(|p| w <= p[1].0).unwrap_or(OPENPBR_SKIN_LIFT.len() - 2);
    let ((w0, l0), (w1, l1)) = (OPENPBR_SKIN_LIFT[i], OPENPBR_SKIN_LIFT[i + 1]);
    l0 + (l1 - l0) * (w - w0) / (w1 - w0)
}

/// See [`openpbr_skin_lift`]: (translucency, lift). Up to w 0.5 OpenPBR
/// rendered the colour darker than Standard Surface, evenly in R, G and B:
/// 0.78 at w 0.5 (Genesis 8 Female, Calib-HDRI), 0.70 at 0.62 (Fleur,
/// Octane4) — 1 / (1 − 0.46 · w) brought it back (#8). Iray darkens this
/// skin far more with w than that (Calib-Skin-Base: w 0.5 a third of w 0,
/// w 0.9 a ninth), so past 0.5 the lift falls: under 1 / (1 − 0.46 · w)
/// w 0.9 came out 1.8 – 2.2 × Iray, w 0 and 0.5 0.83 – 1.08 (#28).
const OPENPBR_SKIN_LIFT: [(f32, f32); 4] = [(0.0, 1.0), (0.5, 1.3), (0.9, 0.45), (1.0, 0.35)];

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
    /// An IES light's profile.
    pub ies: Option<IesDistribution>,
}

/// An IES profile on an Octane emission's `distribution` pin: a greyscale
/// image node reading the `.ies` (peak-normalised, `iesPhotometryMode` 1)
/// through a Spherical projection (#34).
#[derive(Debug, Clone, PartialEq)]
pub struct IesDistribution {
    /// The packed `.ies`.
    pub asset: String,
    /// Iray's profile frame — its φ = 0° axis, φ = 90° axis and θ = 0° axis
    /// (`measured_edf`'s x, y, z) — in the emitter's object space, or in
    /// world space when `world`.
    pub frame: [[f64; 3]; 3],
    /// Multiplies the emitter's power: Iray scales the profile to the
    /// light's flux, Octane's peak-normalised distribution keeps only
    /// [`crate::ies::Profile::mean_share`] of it.
    pub power_scale: f64,
    /// `frame` is in world space (a mesh baked into world space).
    pub world: bool,
}

/// Octane's IES frame: the directions of the profile's φ = 0°, φ = 90° and
/// θ = 0° axes on its Spherical projection at rotation 0.
/// Measured on Orbis-Asym (Calib-L-IES-Asym-Down): the profile's axis
/// along −Y (Octane is Y-up, a downlight's nadir down), φ = 0° along +Z and
/// φ = 90° along −X (right-handed, as Iray's); the projection's `transform`
/// turns that sphere into the coordinate space (rotation X +90° moved the
/// axis from −Y to −Z).
const OCTANE_IES_AXES: [[f64; 3]; 3] = [[0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]];

/// The Spherical projection's rotation (degrees, order XYZ = Rx·Ry·Rz) that
/// turns Octane's IES frame onto `frame`: R · octane_k = frame_k.
fn ies_projection_rotation(frame: &[[f64; 3]; 3]) -> [f64; 3] {
    // R = F · Oᵀ (F's and O's columns the axes; O orthonormal).
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = (0..3).map(|k| frame[k][i] * OCTANE_IES_AXES[k][j]).sum();
        }
    }
    let b = r[0][2].clamp(-1.0, 1.0).asin();
    // At b = ±90° only a ± c is fixed: c = 0, R's middle row is (±sin a, cos a, 0).
    let (a, c) = if b.cos().abs() > 1e-9 { ((-r[1][2]).atan2(r[2][2]), (-r[0][1]).atan2(r[0][0])) } else { ((r[1][0] * r[0][2].signum()).atan2(r[1][1]), 0.0) };
    [a, b, c].map(|x| {
        let d = x.to_degrees();
        if d.abs() < 1e-9 { 0.0 } else { d }
    })
}

/// Material inputs that make Octane's light emitters glow: a black diffuse
/// material with a blackbody (or colour) emission of the light's power.
pub fn light_inputs(lights: &[LightEmitter], first_id: u32) -> String {
    let mut textures = Textures::default();
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs: &[], textures: &mut textures, tiling: None, in_mix: false, in_coat_mix: false, in_layered_coat: false, in_flake_mix: false, in_backscatter_mix: false, thin_glass_ior_one: false, max_transmission: 1.0, film: None, pin_wraps: Vec::new(), gloss_ggx: false, uv_scale: None };
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
        let ies = l.ies.as_ref().map_or(1.0, |d| d.power_scale);
        let watts = l.watts * if l.two_sided { TWO_SIDED_POWER } else { 1.0 } * if blackbody { 1.0 } else { COLOUR_EMISSION_POWER } * ies;
        w.value_pin(8, "power", 6, 8, &format!("{} 0 0", num(watts as f32)));
        w.value_pin(8, "surfaceBrightness", 11, 1, "0");
        if l.two_sided {
            w.value_pin(8, "doubleSided", 11, 1, "1");
        }
        if let Some(d) = &l.ies {
            w.ies_pin(8, d);
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

/// Iray's dome ground (Dome Mode "… w/ Ground"): the HDRI, as
/// `projected_environment_texture` writes it, on the ground at `height`
/// (meters), at the dome's power.
#[derive(Debug, Clone)]
pub struct DomeGround {
    pub texture: String,
    pub height: f64,
    pub power: f64,
}

/// The dome ground's emission power per unit of the dome's (surface
/// brightness on): Calib-Env-DomeMode1's projected ground 1.04 × Iray at 1,
/// as the rest of the frame.
const DOME_GROUND_POWER: f64 = 1.0;

/// The dome ground's box (meters): 2 km wide, 1 mm thick.
const DOME_GROUND_SIZE: [f64; 3] = [2000.0, 0.001, 2000.0];

/// How far (meters) the dome ground sits below Iray's ground height: under
/// the shadow catcher (`GROUND_DROP`).
const DOME_GROUND_DROP: f64 = 0.002;

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
    /// OpenPBR Surface (505) with random walk: chromatic-SSS skins with the
    /// colour baked as Iray's skin shows it (`skin_albedo`, Calib-Skin, #19),
    /// mono ones brightened to Iray's level (`openpbr_skin_lift`).
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
    /// The emitter, meters: a quad width × height (Iray's rectangle), else
    /// a disk of this diameter (both values).
    pub size: [f64; 2],
    pub quad: bool,
    pub euler_deg: [f64; 3],
    pub rotation_order: String,
    pub translation: [f64; 3],
    /// Seen by the camera (Render Emitter).
    pub visible: bool,
}

/// A VDB volume for Octane: Iray's coefficients ([`crate::usd::Volume`])
/// with the `.vdb` packed at `asset`.
#[derive(Debug, Clone)]
pub struct OctaneVolume {
    pub name: String,
    /// The packed `.vdb`; `None` for Octane's Unit Volume (295), a uniform
    /// box placed by `transform` (`unit_box`) — Iray's fog.
    pub asset: Option<String>,
    pub grid: String,
    /// The placement: VDB units → meters, rows of a 3×4 matrix.
    pub transform: [f64; 12],
    pub scattering: [f32; 3],
    pub absorption: [f32; 3],
    /// The phase function's g (Iray's "SSS Direction").
    pub anisotropy: f32,
}

/// Iray's volume coefficients → Octane's: both per meter (Calib-VDB-abs,
/// absorption only, at a converged step: sky 0.80 / horizon 0.36 / below
/// 0.79 of the haze-free render, Iray 0.79 / 0.35 / 0.77).
const VOLUME_COEFFICIENT_SCALE: f32 = 1.0;

/// The volume medium's ray-march step, in world voxels. Octane takes the
/// step percentage of the VDB's own voxel (3000 units in DTAS's haze),
/// not of the placed one (3 m): at the default 100 % it crossed the whole
/// haze in a step or two and rendered it thin and step-dependent (400 %
/// thinner still). Calib-VDB-abs converged at 10 voxels (1 % and 0.1 %
/// alike, 147 s and 837 s for 200 samples; at 4 voxels Calib-VDB-base
/// took over 20 minutes for 400).
const VOLUME_STEP_VOXELS: f64 = 10.0;

/// A Unit Volume's ray-march step (`rayMarchStepPercent`, a fraction):
/// uniform inside, yet Octane's march under-counts the in-scatter at long
/// steps — Calib-Env-GroundFog's floor 1.27 × Iray at 1.0, 1.06 at 0.1
/// (321 s for 128 samples; 0.01 had not finished 256 in 11 minutes).
const UNIT_VOLUME_STEP: f64 = 0.1;

/// The placement (rows of a 3×4 matrix) that stretches Octane's Unit Volume
/// over the box `min`–`max` (meters).
pub fn unit_box(min: [f64; 3], max: [f64; 3]) -> [f64; 12] {
    let s = [0, 1, 2].map(|k| max[k] - min[k]);
    let o = [0, 1, 2].map(|k| min[k] + UNIT_VOLUME_ORIGIN * s[k]);
    [s[0], 0.0, 0.0, o[0], 0.0, s[1], 0.0, o[1], 0.0, 0.0, s[2], o[2]]
}

/// Where Octane's Unit Volume starts, in its own unit: 0.5 for a cube
/// centred on the origin, 0 for one from the origin to (1, 1, 1).
const UNIT_VOLUME_ORIGIN: f64 = 0.5;

/// The volume medium's density pin: Octane's maximum.
const VOLUME_DENSITY_MAX: f32 = 10_000.0;

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
/// VDB volumes (91, [`OctaneVolume`]) in a volume medium (98), and
/// a geometry group (type 3) joining them with the USD import's output node
/// `usd_output`. Returns the XML and the node the render target's mesh pin
/// connects to (the group, or `usd_output` when there is nothing to add).
pub fn scene_lights(spots: &[OctaneSpot], ground: Option<(f64, f64)>, dome_ground: Option<&DomeGround>, volumes: &[OctaneVolume], first_id: u32, usd_output: u32) -> (String, u32) {
    if spots.is_empty() && ground.is_none() && dome_ground.is_none() && volumes.is_empty() {
        return (String::new(), usd_output);
    }
    let mut textures = Textures::default();
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs: &[], textures: &mut textures, tiling: None, in_mix: false, in_coat_mix: false, in_layered_coat: false, in_flake_mix: false, in_backscatter_mix: false, thin_glass_ior_one: false, max_transmission: 1.0, film: None, pin_wraps: Vec::new(), gloss_ggx: false, uv_scale: None };
    let mut ids = Vec::new();
    for (k, l) in spots.iter().enumerate() {
        let id = w.id();
        ids.push(id);
        let _ = writeln!(w.out, "    <node id='{id}' type='402' name='{}' position='{} -450'>", esc(&l.name), 300 + k * 200);
        w.value_pin(5, "analyticLightType", 57, 2, if l.quad { "0" } else { "1" }); // QUAD / DISK
        w.value_pin(5, "analyticLightSpreadAngle", 6, 9, &format!("{} 0 0 0", num(l.spread_deg as f32)));
        // A soft cone, cos θ · cos²(θ · 90° / half): Iray's beam falls off
        // towards the edge (`octane_cone_half` fits the spread to it).
        w.value_pin(5, "analyticLightSpreadCutoffHardness", 6, 9, "0 0 0 0");
        w.value_pin(5, "normalize", 11, 1, "1");
        // Both in the light's XY plane, shining along +Z (probe 111).
        let [sx, sy] = l.size.map(|s| num(s as f32));
        w.value_pin(5, if l.quad { "quadAnalyticLightSize" } else { "diskAnalyticLightSize" }, 6, 9, &format!("{sx} {sy} 0 0"));
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
        if !l.visible {
            // Render Emitter off: an object layer (65) the camera doesn't see.
            let oid = w.id();
            w.pin_open(5, "objectLayer");
            let _ = writeln!(w.out, "      <node id='{oid}' type='65'>");
            w.value_pin(7, "camera_visibility", 11, 1, "0");
            w.out.push_str("      </node>\n");
            w.pin_close(5);
        }
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
    if let Some(g) = dome_ground {
        // The HDRI's lower half on the ground, as seen from Iray's virtual
        // camera: a thin, wide SDF box (296) that emits it and reflects
        // nothing — the infinite plane (110) renders an emitting material
        // black.
        let id = w.id();
        ids.push(id);
        let _ = writeln!(w.out, "    <node id='{id}' type='4' name='DAZ dome ground' position='{} -600'>", 300 + spots.len() * 200);
        let tid = w.id();
        w.pin_open(5, "transform");
        let _ = writeln!(w.out, "      <node id='{tid}' type='67'>");
        let _ = writeln!(w.out, "       <attr name='translation' type='8'>0 {} 0</attr>", num64(g.height - DOME_GROUND_DROP - DOME_GROUND_SIZE[1] / 2.0));
        w.out.push_str("      </node>\n");
        w.pin_close(5);
        let pid = w.id();
        w.pin_open(5, "geometry");
        let _ = writeln!(w.out, "      <node id='{pid}' type='296' name='DAZ dome ground slab'>");
        for (k, v) in ["width", "height", "depth"].iter().zip(DOME_GROUND_SIZE) {
            w.value_pin(7, k, 6, 9, &format!("{} 0 0 0", num64(v)));
        }
        let mid = w.id();
        w.pin_open(7, "material");
        let _ = writeln!(w.out, "        <node id='{mid}' type='17' name='DAZ dome ground'>");
        let _ = writeln!(w.out, "         <attr name='compatibilityVersion' type='2'>{COMPAT}</attr>");
        w.rgb_pin(9, "diffuse", [0.0; 3]);
        let eid = w.id();
        w.pin_open(9, "emission");
        let _ = writeln!(w.out, "          <node id='{eid}' type='54'>");
        w.out.push_str("           <pin name='efficiency or texture'>\n");
        w.out.push_str(&g.texture);
        w.out.push_str("           </pin>\n");
        w.value_pin(11, "power", 6, 8, &format!("{} 0 0", num64(g.power * DOME_GROUND_POWER)));
        w.value_pin(11, "surfaceBrightness", 11, 1, "1");
        w.out.push_str("          </node>\n");
        w.pin_close(9);
        w.out.push_str("        </node>\n");
        w.pin_close(7);
        w.out.push_str("      </node>\n");
        w.pin_close(5);
        w.out.push_str("    </node>\n");
    }
    for (k, v) in volumes.iter().enumerate() {
        let id = w.id();
        ids.push(id);
        let _ = writeln!(w.out, "    <node id='{id}' type='4' name='{}' position='{} -450'>", esc(&v.name), 300 + (spots.len() + 1 + k) * 200);
        let tid = w.id();
        w.pin_open(5, "transform");
        let _ = writeln!(w.out, "      <node id='{tid}' type='67'>");
        let m: Vec<String> = v.transform.iter().map(|x| num64(*x)).collect();
        let _ = writeln!(w.out, "       <attr name='transform' type='13'>{}</attr>", m.join(" "));
        w.out.push_str("      </node>\n");
        w.pin_close(5);
        let gid = w.id();
        w.pin_open(5, "geometry");
        // Octane's coefficients, per meter: grid value × the grid's scale
        // (applied as the file loads) × the medium's density × its colour.
        // The density pin stops at 10⁴: what is beyond goes to the grids.
        let peak = v.scattering.iter().chain(&v.absorption).fold(0.0f32, |a, &b| a.max(b));
        let unit = if peak > 0.0 { peak } else { 1.0 };
        let total = unit * VOLUME_COEFFICIENT_SCALE;
        let density = total.min(VOLUME_DENSITY_MAX);
        match &v.asset {
            Some(asset) => {
                let _ = writeln!(w.out, "      <node id='{gid}' type='91' name='{}'>", esc(&v.name));
                let _ = writeln!(w.out, "       <attr name='filename' type='11'>{}</attr>", esc(asset));
                // Meters: the placement scales the VDB's units (DAZ's cm).
                w.out.push_str("       <attr name='scaleUnitType' type='2'>4</attr>\n");
                for (id, scale) in [("volumeAbsorptionId", "volumeAbsorptionScale"), ("volumeScatterId", "volumeScatteringScale")] {
                    let _ = writeln!(w.out, "       <attr name='{id}' type='10'>{}</attr>", esc(&v.grid));
                    let _ = writeln!(w.out, "       <attr name='{scale}' type='6'>{}</attr>", num(total / density));
                }
                w.out.push_str("       <attr name='volumeEmissionId' type='10'></attr>\n");
            }
            None => {
                let _ = writeln!(w.out, "      <node id='{gid}' type='295' name='{}'>", esc(&v.name));
            }
        }
        let mid = w.id();
        w.pin_open(7, "medium");
        let _ = writeln!(w.out, "        <node id='{mid}' type='98' name='{} medium'>", esc(&v.name));
        w.value_pin(9, "scale", 6, 9, &format!("{} 0 0 0", num(density)));
        // The placement's uniform scale: VDB units → meters.
        let t = &v.transform;
        let det = t[0] * (t[5] * t[10] - t[6] * t[9]) - t[1] * (t[4] * t[10] - t[6] * t[8]) + t[2] * (t[4] * t[9] - t[5] * t[8]);
        let step = if v.asset.is_some() { (VOLUME_STEP_VOXELS * det.abs().cbrt()).min(1.0) } else { UNIT_VOLUME_STEP };
        w.value_pin(9, "rayMarchStepPercent", 6, 9, &format!("{} 0 0 0", num64(step)));
        w.rgb_pin(9, "absorption", v.absorption.map(|c| c / unit));
        w.value_pin(9, "invertAbsorption", 11, 1, "0");
        w.rgb_pin(9, "scattering", v.scattering.map(|c| c / unit));
        let pid = w.id();
        w.pin_open(9, "phase");
        let _ = writeln!(w.out, "          <node id='{pid}' type='60'>");
        w.value_pin(11, "scattering_direction", 6, 8, &format!("{} 0 0", num(v.anisotropy)));
        w.out.push_str("          </node>\n");
        w.pin_close(9);
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
    environment_texture_at(env, dirs, textures, first_id, None)
}

/// The scene's HDRI as `environment_texture` writes it, looked up from
/// `origin` (meters) by a world-space Spherical projection: Iray's dome
/// projected onto its ground (`dome_ground`). None without a map.
pub fn projected_environment_texture(env: &crate::usd::SceneEnvironment, dirs: &[PathBuf], textures: &mut Textures, first_id: u32, origin: [f64; 3]) -> Option<String> {
    env.map.as_ref()?;
    let out = environment_texture_at(Some(env), dirs, textures, first_id, Some(origin));
    out.contains("type='34'").then_some(out)
}

fn environment_texture_at(
    env: Option<&crate::usd::SceneEnvironment>,
    dirs: &[PathBuf],
    textures: &mut Textures,
    first_id: u32,
    origin: Option<[f64; 3]>,
) -> String {
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs, textures, tiling: None, in_mix: false, in_coat_mix: false, in_layered_coat: false, in_flake_mix: false, in_backscatter_mix: false, thin_glass_ior_one: false, max_transmission: 1.0, film: None, pin_wraps: Vec::new(), gloss_ggx: false, uv_scale: None };
    let map = env.and_then(|e| e.map.clone()).and_then(|m| w.textures.asset(dirs, &m));
    match map {
        Some(asset) => {
            // Environment Tint multiplies the map (Iray's
            // `environment_function_tint`, the .mi export).
            let tint = env.map_or([1.0; 3], |e| e.tint).map(|c| c as f32);
            if tint != [1.0; 3] {
                let mid = w.id();
                let _ = writeln!(w.out, "        <node id='{mid}' type='39'>\n         <pin name='texture2'>");
                let cid = w.id();
                let _ = writeln!(w.out, "          <node id='{cid}' type='33'>\n           <attr name='value' type='8'>{} {} {}</attr>\n          </node>", num(tint[0]), num(tint[1]), num(tint[2]));
                w.out.push_str("         </pin>\n         <pin name='texture1'>\n");
            }
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
            // The dome's tilt: a Spherical projection (type 77) in world
            // space turned by Iray's orientation, order ZXY = Rz · Rx · Ry.
            let tilt = env.and_then(|e| e.dome_tilt());
            if tilt.is_some() || origin.is_some() {
                let r = tilt.unwrap_or([0.0; 3]);
                let t = origin.unwrap_or([0.0; 3]);
                let pid = w.id();
                w.pin_open(9, "projection");
                let _ = writeln!(w.out, "          <node id='{pid}' type='77'>");
                let tid = w.id();
                w.pin_open(11, "transform");
                let _ = writeln!(w.out, "            <node id='{tid}' type='67'>");
                w.out.push_str("             <attr name='rotationOrder' type='2'>4</attr>\n");
                let _ = writeln!(w.out, "             <attr name='rotation' type='8'>{} {} {}</attr>", num(r[0] as f32), num(r[1] as f32), num(r[2] as f32));
                // On a surface Octane's Spherical projection looks up the
                // image mirrored to the environment's: X flipped (Calib-Env-DomeMode1).
                w.out.push_str(if origin.is_some() { "             <attr name='scale' type='8'>-1 1 1</attr>\n" } else { "             <attr name='scale' type='8'>1 1 1</attr>\n" });
                let _ = writeln!(w.out, "             <attr name='translation' type='8'>{} {} {}</attr>", num64(t[0]), num64(t[1]), num64(t[2]));
                w.out.push_str("            </node>\n");
                w.pin_close(11);
                w.value_pin(11, "positionType", 57, 2, "1");
                w.out.push_str("          </node>\n");
                w.pin_close(9);
            }
            w.out.push_str("        </node>\n");
            if tint != [1.0; 3] {
                w.out.push_str("         </pin>\n        </node>\n");
            }
        }
        None => {
            let id = w.id();
            let _ = writeln!(w.out, "        <node id='{id}' type='33'>\n         <attr name='value' type='8'>1 1 1</attr>\n        </node>");
        }
    }
    w.out
}

/// Object layer input nodes (type 20017) for the USD graph, which creates
/// one per prim, `/Scene/<prim>_object` (lights `/Scene/Lights/<prim>_object`),
/// once it loads all object layers (`objectLayerImport` 3, probes 100–102):
/// for each `(stem, pin, value)` an object layer (type 65) with that
/// visibility pin set — `shadow_visibility` off for DAZ's `Cast Shadows`
/// off on a mesh, `camera_visibility` on for a light the camera sees (#35).
/// Node ids start at `first_id`.
pub fn object_layer_inputs(layers: &[(String, &str, bool)], first_id: u32) -> String {
    let mut out = String::new();
    for (k, (stem, pin, value)) in layers.iter().enumerate() {
        let id = first_id + 3 * k as u32;
        let _ = writeln!(out, "   <node id='{id}' type='20017' name='/Scene/{}_object' position='{} -450'>", esc(stem), k * 200);
        out.push_str("    <attr name='group' type='10'></attr>\n    <attr name='description' type='10'></attr>\n");
        let _ = writeln!(out, "    <pin name='input'>\n     <node id='{}' type='65' name='Object layer'>", id + 1);
        let _ = writeln!(out, "      <pin name='{pin}'>\n       <node id='{}' type='11'>\n        <attr name='value' type='1'>{}</attr>\n       </node>\n      </pin>", id + 2, u8::from(*value));
        out.push_str("     </node>\n    </pin>\n   </node>\n");
    }
    out
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
    material_inputs_scaled(materials, &UvScales::new(), dirs, layered, first_id, shading)
}

/// Each material's UV scale: meters of surface per UV unit
/// (`usd::uv_scales`), keyed by the material.
pub type UvScales = HashMap<*const Material, f64>;

/// [`material_inputs`] with the materials' UV scales, which set their bump
/// strength ([`Ocs::bump_power`]).
pub fn material_inputs_scaled(
    materials: &[crate::usd::BoundMaterial],
    uv_scales: &UvScales,
    dirs: &[PathBuf],
    layered: HashMap<String, crate::layered_image::LayeredImage>,
    first_id: u32,
    shading: impl Into<Shading>,
) -> (String, Textures) {
    let Shading { skin, thin_glass_ior_one, max_transmission } = shading.into();
    let mut textures = Textures { layered, ..Default::default() };
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs, textures: &mut textures, tiling: None, in_mix: false, in_coat_mix: false, in_layered_coat: false, in_flake_mix: false, in_backscatter_mix: false, thin_glass_ior_one, max_transmission, film: None, pin_wraps: Vec::new(), gloss_ggx: false, uv_scale: None };
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
        w.film = Film::of(mat);
        w.uv_scale = uv_scales.get(&std::rc::Rc::as_ptr(mat)).copied();
        w.universal(5, name, mat, *area, skin);
        w.film = None;
        w.uv_scale = None;
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

    #[test]
    fn ies_projection_turns_octane_s_frame_onto_iray_s() {
        // Rx · Ry · Rz (Octane's rotation order XYZ), degrees.
        let rot = |[a, b, c]: [f64; 3]| {
            let (a, b, c) = (a.to_radians(), b.to_radians(), c.to_radians());
            let rx = [[1.0, 0.0, 0.0], [0.0, a.cos(), -a.sin()], [0.0, a.sin(), a.cos()]];
            let ry = [[b.cos(), 0.0, b.sin()], [0.0, 1.0, 0.0], [-b.sin(), 0.0, b.cos()]];
            let rz = [[c.cos(), -c.sin(), 0.0], [c.sin(), c.cos(), 0.0], [0.0, 0.0, 1.0]];
            let mul = |x: [[f64; 3]; 3], y: [[f64; 3]; 3]| -> [[f64; 3]; 3] { std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| x[i][k] * y[k][j]).sum())) };
            mul(mul(rx, ry), rz)
        };
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let frames = [
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]],
            [[s, s, 0.0], [-s, s, 0.0], [0.0, 0.0, 1.0]],
            [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
            [[0.0, -1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
        ];
        for f in frames {
            let r = rot(ies_projection_rotation(&f));
            for k in 0..3 {
                let v: Vec<f64> = (0..3).map(|i| (0..3).map(|j| r[i][j] * OCTANE_IES_AXES[k][j]).sum()).collect();
                assert!((0..3).all(|i| (v[i] - f[k][i]).abs() < 1e-9), "{f:?}: axis {k} → {v:?}");
            }
        }
        // The probe's: axis down (−Z), φ 0 towards −Y, φ 90 towards −X — X +90°.
        assert_eq!(ies_projection_rotation(&frames[4]), [90.0, 0.0, 0.0]);
    }

    #[test]
    fn emission_profile_becomes_an_ies_distribution() {
        let dir = std::env::temp_dir().join(format!("orbis-ies-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Runtime")).unwrap();
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("calib/ies/Orbis-Asym.ies"), dir.join("Runtime/asym.ies")).unwrap();
        let chs = |profile: &str| {
            json!([
                {"channel": {"id": "Emission Color", "current_value": [1, 1, 1]}},
                {"channel": {"id": "Luminance", "current_value": 5000}},
                {"channel": {"id": "Emission Temperature", "current_value": 6500}},
                {"channel": {"id": "Emission Profile", "current_value": profile}},
            ])
        };
        let mut lamp = (*mat(chs("/Runtime/asym.ies"))).clone();
        // Turned half round X (facing down): its object Z is world −Z.
        lamp.frame = Some([[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]]);
        // A plane facing down (world −Y): in Iray's frame (object X, −Y,
        // −Z = world X, Y, Z) its normal is −y, φ 270.
        lamp.emit_normals = Some(vec![([0.0, -1.0, 0.0], 1.0)]);
        let dirs = [dir.clone()];
        let (x, tex) = material_inputs(&[("Lamp".into(), Rc::new(lamp), 0.25)], &dirs, HashMap::new(), 9000, SkinShader::None);
        let (plain, _) = material_inputs(&[("Lamp".into(), mat(chs("")), 0.25)], &dirs, HashMap::new(), 9000, SkinShader::None);
        let d = &x[x.find("<pin name='distribution'>").expect("a distribution")..];
        assert!(d.contains("<attr name='filename' type='11'>assets/ies/00_asym.ies</attr>") && d.contains("<attr name='iesPhotometryMode' type='2'>1</attr>"), "{d}");
        assert!(d.contains("type='77'") && d.contains("<attr name='value' type='2'>1</attr>"), "world space: {d}");
        assert_eq!(tex.files.len(), 1, "the .ies is packed");
        assert!(!plain.contains("distribution"));
        // The power over the profile's share of a uniform emitter's.
        let power = |x: &str| -> f64 {
            let p = &x[x.find("<pin name='power'>").unwrap()..];
            let v = &p[p.find("type='8'>").unwrap() + 9..];
            v[..v.find(' ').unwrap()].parse().unwrap()
        };
        // The power over what Lambertian × the profile keeps of the flux.
        let share = crate::ies::Profile::parse(&std::fs::read_to_string(dir.join("Runtime/asym.ies")).unwrap()).unwrap().lambert_share(&[([0.0, -1.0, 0.0], 1.0)]);
        assert!((power(&x) / power(&plain) * share - 1.0).abs() < 1e-3, "{} {} {share}", power(&x), power(&plain));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn uv_translation_in_tile_units_with_mirrored_u() {
        assert_eq!(uv_translation([1.0, 1.0], [0.125, 0.125]), (-0.125, 0.125));
        assert_eq!(uv_translation([2.0, 2.0], [0.125, 0.125]), (-0.0625, 0.0625));
        assert_eq!(uv_translation([3.0, 2.0], [0.0, 0.125]), (0.0, 0.0625));
    }

    fn mat(chs: serde_json::Value) -> Rc<Material> {
        Rc::new(Material::from_dson(&json!({"extra": [{"channels": chs}]}), &HashMap::new()))
    }

    #[test]
    fn omni_hair_absorbs_by_melanin() {
        let hair = |melanin: f32| strength(melanin, 1.0);
        fn strength(melanin: f32, w: f32) -> Rc<Material> {
            mat(json!([
                {"channel": {"id": "Diffuse Strength", "current_value": w}},
                {"channel": {"id": "Melanin", "current_value": melanin}},
                {"channel": {"id": "Melanin Redness", "current_value": 0.5}},
                {"channel": {"id": "Hair Root Color", "current_value": [1, 1, 1]}},
                {"channel": {"id": "Hair Tip Color", "current_value": [1, 1, 1]}},
                {"channel": {"id": "Roughness", "current_value": 0.2}},
            ]))
        }
        let blonde = omni_hair(&hair(0.175)).unwrap();
        let brown = omni_hair(&hair(0.65)).unwrap();
        // m² · 2.4, split by the redness; roughness mapped.
        assert!((blonde.eumelanin - 0.175f32.powi(2) * 1.2).abs() < 1e-5 && (brown.pheomelanin - 0.65f32.powi(2) * 1.2).abs() < 1e-5);
        assert!((blonde.roughness - omni_v(0.2)).abs() < 1e-6);
        // A white base at Diffuse Strength 0.8 absorbs nothing: the melanin × √m.
        let d = omni_hair(&strength(0.65, 0.8)).unwrap();
        assert!((d.eumelanin - brown.eumelanin * 0.65f32.sqrt()).abs() < 1e-4, "{} {}", d.eumelanin, brown.eumelanin);
        // The fit recovers a pigment mix.
        let (a, b) = melanin_fit([0, 1, 2].map(|k| 0.3 * OMNI_EUMELANIN[k] + 0.7 * OMNI_PHEOMELANIN[k]));
        assert!((a - 0.3).abs() < 1e-4 && (b - 0.7).abs() < 1e-4);
        // Blended Dual Lobe hair (no Melanin) stays with `hair`.
        assert!(omni_hair(&mat(json!([{"channel": {"id": "Hair Root Color", "current_value": [1, 1, 1]}}]))).is_none());
        let (x, _) = material_inputs(&[("Omni".into(), hair(0.65), 1.0)], &[], HashMap::new(), 9600, SkinShader::None);
        assert!(x.contains("type='147'") && x.contains("<pin name='roughnessV'>"), "{x}");
    }

    #[test]
    fn data_maps_read_at_dazs_automatic_gamma() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("t")).unwrap();
        std::fs::write(dir.path().join("t/o.jpg"), b"x").unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let opacity = |gamma: Option<f32>| {
            let mut m = Material::from_dson(&json!({"extra": [{"channels": [
                {"channel": {"id": "Cutout Opacity", "current_value": 1, "image_file": "/t/o.jpg"}},
            ]}]}), &HashMap::new());
            m.channels.get_mut("Cutout Opacity").unwrap().gamma = gamma;
            let (x, _) = material_inputs(&[("M".into(), Rc::new(m), 1.0)], &dirs, HashMap::new(), 9500, SkinShader::None);
            x
        };
        // No gamma of its own: 2.2, decoded (sRGB space); its own 1: linear.
        let auto = opacity(None);
        assert!(auto.contains("<pin name='opacity'>") && auto.contains("<attr name='value' type='8'>2.2 0 0</attr>"), "{auto}");
        assert!(opacity(Some(1.0)).contains("<attr name='value' type='8'>1 0 0</attr>"));
    }

    #[test]
    fn displacement_moves_by_strength_between_min_and_max() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("t")).unwrap();
        std::fs::write(dir.path().join("t/d.jpg"), b"x").unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let disp = |strength: f32, lo: f32, hi: f32, level: f32| {
            let mut m = Material::from_dson(&json!({"extra": [{"channels": [
                {"channel": {"id": "Displacement Strength", "current_value": strength, "image_file": "/t/d.jpg"}},
                {"channel": {"id": "Minimum Displacement", "current_value": lo}},
                {"channel": {"id": "Maximum Displacement", "current_value": hi}},
                {"channel": {"id": "SubD Displacement Level", "current_value": level}},
            ]}]}), &HashMap::new());
            m.prop = false;
            m
        };
        // 0.5 × (−5 … 5 cm): 5 cm of height about a mid level of 0.5; the
        // subdivision capped.
        let d = displacement(&disp(0.5, -5.0, 5.0, 3.0)).unwrap();
        assert!((d.height_m - 0.05).abs() < 1e-6 && (d.mid - 0.5).abs() < 1e-6 && d.subd == DISPLACEMENT_SUBD_MAX && d.gamma == 2.2);
        // 0 … 10: all outward; a prop isn't subdivided.
        let mut p = disp(1.0, 0.0, 10.0, 1.0);
        p.prop = true;
        let d = displacement(&p).unwrap();
        assert!((d.height_m - 0.1).abs() < 1e-6 && d.mid == 0.0 && d.subd == 0);
        // The image's own gamma wins.
        let mut g = disp(1.0, 0.0, 10.0, 0.0);
        g.channels.get_mut("Displacement Strength").unwrap().gamma = Some(1.0);
        assert_eq!(displacement(&g).unwrap().gamma, 1.0);
        // No strength, no range, no map: nothing.
        assert!(displacement(&disp(0.0, -5.0, 5.0, 0.0)).is_none() && displacement(&disp(1.0, 2.0, 2.0, 0.0)).is_none());
        assert!(displacement(&mat(json!([{"channel": {"id": "Displacement Strength", "current_value": 1}}]))).is_none());
        let (x, _) = material_inputs(&[("Skin".into(), Rc::new(disp(0.5, -5.0, 5.0, 1.0)), 1.0)], &dirs, HashMap::new(), 9400, SkinShader::None);
        assert!(x.contains("<pin name='displacement'>") && x.contains("type='97'"), "{x}");
        assert!(x.contains("<pin name='amount'>") && x.contains("<attr name='value' type='9'>0.05 0 0 0</attr>"), "{x}");
        assert!(x.contains("<pin name='black_level'>") && x.contains("<pin name='subdLevel'>") && x.contains("<attr name='value' type='5'>1 0 0 0</attr>"), "{x}");
    }

    #[test]
    fn nvidia_dielectrics_gloss_by_weight_at_their_ior() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("resources")).unwrap();
        for f in ["concrete_plain.png", "concrete_plain_bump.png"] {
            std::fs::write(dir.path().join("resources").join(f), b"x").unwrap();
        }
        let dirs = vec![dir.path().to_path_buf()];
        let nvidia = |module: &str, chs: serde_json::Value| {
            let mut m = Material::from_dson(&json!({"extra": [{"channels": chs}]}), &HashMap::new());
            m.mdl = vec![format!("nvidia::{module}::{module}")];
            Rc::new(m)
        };
        // Concrete: the bump map tints a 0.141 gloss, at IOR 1.46 and 10 tiles.
        let concrete = nvidia("concrete", json!([
            {"channel": {"id": "Diffuse Color", "image_file": "/resources/concrete_plain.png"}},
            {"channel": {"id": "Bump Strength", "current_value": 3, "image_file": "/resources/concrete_plain_bump.png"}},
            {"channel": {"id": "glossy_weight", "current_value": 0.141}},
            {"channel": {"id": "roughness", "current_value": 0.4356}},
            {"channel": {"id": "Index of Refraction", "current_value": 1.46}},
            {"channel": {"id": "Horizontal Tiles", "current_value": 10}},
        ]));
        let (x, _) = material_inputs(&[("Concrete".into(), concrete, 1.0)], &dirs, HashMap::new(), 9300, SkinShader::None);
        let specular = &x[x.find("<pin name='specular'>").unwrap()..];
        assert!(specular[..specular.find("<pin name='roughness'>").unwrap()].contains("concrete_plain_bump"), "{x}");
        // The roughness on the Uber gloss's GGX curve, energy-preserving GGX.
        assert!(x.contains("0.141") && x.contains("1.46") && x.contains(&format!(">{}<", num(ggx_roughness(0.4356)))) && x.contains("<pin name='bump'>"), "{x}");
        assert!(x.contains("<pin name='brdf'>") && x.contains("type='2'>6<"), "{x}");
        // Porcelain: its colour, a full gloss; resin: its colour let through.
        let porcelain = nvidia("porcelain", json!([{"channel": {"id": "diffuse", "current_value": [1, 1, 1]}}]));
        let (y, _) = material_inputs(&[("Porcelain".into(), porcelain, 1.0)], &dirs, HashMap::new(), 9400, SkinShader::None);
        assert!(y.contains("<pin name='specular'>") && !y.contains("transmission"), "{y}");
        let resin = nvidia("resin_polyurethane_matte", json!([{"channel": {"id": "diffuse", "current_value": [1, 1, 1]}}]));
        let (z, _) = material_inputs(&[("Resin".into(), resin, 1.0)], &dirs, HashMap::new(), 9500, SkinShader::None);
        assert!(z.contains("<pin name='transmission'>") && z.contains("<pin name='coating'>"), "{z}");
        // Resin keeps Octane's default BSDF at the MDL roughness.
        assert!(!z.contains("<pin name='brdf'>") && z.contains(">0.25<"), "{z}");
    }

    /// Arch+Design glossiness → roughness as Iray's
    /// `base::architectural_gloss_to_rough` (MDL SDK `base.mdl`).
    #[test]
    fn architectural_glossiness_is_iray_s_curve() {
        assert_eq!(architectural_gloss_to_rough(1.0), 0.0);
        assert!((architectural_gloss_to_rough(0.0) - 0.125f32.sqrt()).abs() < 1e-6);
        assert!((architectural_gloss_to_rough(0.5) - (2.0f32 * 2f32.powi(-11)).sqrt()).abs() < 1e-6);
    }

    #[test]
    fn oot_overlay_blends_the_second_colour_by_its_mask() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        for f in ["base.png", "over.png", "mask.png"] {
            std::fs::write(dir.path().join("Runtime/Textures").join(f), b"x").unwrap();
        }
        let dirs = vec![dir.path().to_path_buf()];
        let hair = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/base.png"}},
            {"channel": {"id": "Diffuse Overlay Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/over.png"}},
            {"channel": {"id": "Overlay Alpha Mask", "current_value": [0, 0, 0], "image_file": "/Runtime/Textures/mask.png"}},
        ]));
        let (x, _) = material_inputs(&[("Hair".into(), hair, 1.0)], &dirs, HashMap::new(), 9100, SkinShader::None);
        let albedo = &x[x.find("<pin name='albedo'>").unwrap()..];
        let albedo = &albedo[..albedo.find("<pin name='specular'>").unwrap_or(albedo.len())];
        assert!(albedo.contains("type='38'") && albedo.contains("mask.png") && albedo.contains("over.png") && albedo.contains("base.png"), "{albedo}");
        // Without the mask: Iray Uber's own Diffuse Overlay, not this.
        let plain = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/base.png"}},
            {"channel": {"id": "Diffuse Overlay Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/over.png"}},
        ]));
        let (y, _) = material_inputs(&[("Plain".into(), plain, 1.0)], &dirs, HashMap::new(), 9200, SkinShader::None);
        assert!(!y.contains("type='38'"), "{y}");
    }

    #[test]
    fn uber_diffuse_overlay_maps_mix_over_the_base() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        for f in ["base.png", "weight.png", "colour.png"] {
            std::fs::write(dir.path().join("Runtime/Textures").join(f), b"x").unwrap();
        }
        let dirs = vec![dir.path().to_path_buf()];
        let albedo = |x: &str| {
            let a = &x[x.find("<pin name='albedo'>").unwrap()..];
            a[..a.find("<pin name='specular'>").unwrap_or(a.len())].to_string()
        };
        // A weight map at 0.5, squared: DAZ's square_float over map × 0.5,
        // so the map at twice its gamma (2.2) × 0.25; the colour map over a
        // flat base.
        let both = mat(json!([
            {"channel": {"id": "Base Mixing", "current_value": 0}},
            {"channel": {"id": "diffuse", "current_value": [0.5, 0.1, 0.1]}},
            {"channel": {"id": "Diffuse Overlay Weight", "current_value": 0.5, "image_file": "/Runtime/Textures/weight.png"}},
            {"channel": {"id": "Diffuse Overlay Weight Squared", "current_value": true}},
            {"channel": {"id": "Diffuse Overlay Color", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/colour.png"}},
        ]));
        let ov = diffuse_overlay(&both).unwrap();
        assert!((ov.weight - 0.25).abs() < 1e-6 && (ov.weight_map.as_ref().unwrap().1 - 4.4).abs() < 1e-5, "{ov:?}");
        let (x, _) = material_inputs(&[("Both".into(), both, 1.0)], &dirs, HashMap::new(), 9100, SkinShader::None);
        let a = albedo(&x);
        assert!(a.contains("type='38'") && a.contains("weight.png") && a.contains("colour.png"), "{a}");
        let amount = &a[a.find("<pin name='amount'>").unwrap()..a.find("<pin name='texture1'>").unwrap()];
        assert!(amount.contains("weight.png") && amount.contains("4.4 0 0") && amount.contains("0.25"), "{amount}");
        // At 4.4 only the legacy-gamma colour space reads the pin (sRGB ignores it).
        assert!(amount.contains(&format!("<attr name='colorSpace' type='2'>{COLOUR_SPACE_LEGACY_GAMMA}</attr>")), "{amount}");
        // A flat overlay over a diffuse map: the Mix at the weight.
        let over_map = mat(json!([
            {"channel": {"id": "Base Mixing", "current_value": 0}},
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/base.png"}},
            {"channel": {"id": "Diffuse Overlay Weight", "current_value": 0.3}},
            {"channel": {"id": "Diffuse Overlay Color", "current_value": [1, 1, 1]}},
        ]));
        let (y, _) = material_inputs(&[("OverMap".into(), over_map, 1.0)], &dirs, HashMap::new(), 9200, SkinShader::None);
        let a = albedo(&y);
        assert!(a.contains("type='38'") && a.contains("base.png") && a.contains("<attr name='value' type='6'>0.3</attr>"), "{a}");
        // Flat over flat: one colour, no Mix.
        let flat = mat(json!([
            {"channel": {"id": "Base Mixing", "current_value": 0}},
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Diffuse Overlay Weight", "current_value": 0.5}},
        ]));
        let (z, _) = material_inputs(&[("Flat".into(), flat, 1.0)], &dirs, HashMap::new(), 9300, SkinShader::None);
        assert!(!albedo(&z).contains("type='38'"), "{z}");
    }

    #[test]
    fn diffuse_roughness_is_oren_nayar_or_its_flat_term() {
        let dirs: Vec<PathBuf> = Vec::new();
        let pin = |x: &str, name: &str| {
            let p = &x[x.find(&format!("<pin name='{name}'>"))?..];
            let v = &p[p.find("'>").unwrap() + 2..];
            let v = &v[v.find("<attr name='value'").unwrap()..];
            Some(v[v.find('>').unwrap() + 1..v.find("</attr>").unwrap()].to_string())
        };
        // Matte: Universal's Oren-Nayar at σ, the albedo as is.
        let matte = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Diffuse Roughness", "current_value": 0.5}},
        ]));
        let (x, _) = material_inputs(&[("Matte".into(), matte, 1.0)], &dirs, HashMap::new(), 9100, SkinShader::None);
        assert_eq!(pin(&x, "diffuseBrdf").as_deref(), Some(DIFFUSE_BRDF_OREN_NAYAR), "{x}");
        assert_eq!(pin(&x, "roughness").as_deref(), Some("0.5"), "{x}");
        assert_eq!(pin(&x, "albedo").as_deref(), Some("1 1 1"), "{x}");
        // With a gloss on the roughness pin: Lambertian × Iray's head-on
        // Oren-Nayar albedo at σ, the overlay at its own roughness's (#76).
        let glossy = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Diffuse Roughness", "current_value": 1.0}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1.0}},
            {"channel": {"id": "Diffuse Overlay Weight", "current_value": 0.5}},
            {"channel": {"id": "Diffuse Overlay Color", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Diffuse Overlay Roughness", "current_value": 0.5}},
        ]));
        let (y, _) = material_inputs(&[("Glossy".into(), glossy, 1.0)], &dirs, HashMap::new(), 9200, SkinShader::None);
        assert!(pin(&y, "diffuseBrdf").is_none(), "{y}");
        let a = 0.5 * oren_nayar_flat(1.0) + 0.5 * oren_nayar_flat(0.5);
        // Fujii's π A at σ 1 / 0.5 (Iray's overlay kept 0.77 head-on), not
        // the classic 0.624 / 0.784.
        assert!((oren_nayar_flat(1.0) - 0.7765).abs() < 1e-3 && (oren_nayar_flat(0.5) - 0.8742).abs() < 1e-3);
        assert_eq!(oren_nayar_flat(0.0), 1.0);
        let albedo: Vec<f32> = pin(&y, "albedo").unwrap().split(' ').map(|v| v.parse().unwrap()).collect();
        assert!(albedo.iter().all(|v| (v - a).abs() < 1e-4), "{albedo:?} vs {a}");
        // Matte under Octane's Oren-Nayar at σ: an overlay at the same
        // roughness mixes in as is; a smooth one × 1 / (π A(σ)).
        for (sigma0, k) in [(1.0, 1.0), (0.0, 1.0 / oren_nayar_flat(1.0))] {
            let matte = mat(json!([
                {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
                {"channel": {"id": "Diffuse Roughness", "current_value": 1.0}},
                {"channel": {"id": "Diffuse Overlay Weight", "current_value": 0.5}},
                {"channel": {"id": "Diffuse Overlay Color", "current_value": [1, 1, 1]}},
                {"channel": {"id": "Diffuse Overlay Roughness", "current_value": sigma0}},
            ]));
            let (z, _) = material_inputs(&[("MatteOverlay".into(), matte, 1.0)], &dirs, HashMap::new(), 9300, SkinShader::None);
            assert_eq!(pin(&z, "diffuseBrdf").as_deref(), Some(DIFFUSE_BRDF_OREN_NAYAR), "{z}");
            let albedo: Vec<f32> = pin(&z, "albedo").unwrap().split(' ').map(|v| v.parse().unwrap()).collect();
            let want = 0.5 + 0.5 * k;
            assert!(albedo.iter().all(|v| (v - want).abs() < 1e-4), "σ₀ {sigma0}: {albedo:?} vs {want}");
        }
    }

    #[test]
    fn terrain_damp_and_ao_darken_the_tiled_diffuse() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        for f in ["d.png", "ao.png", "damp.png"] {
            std::fs::write(dir.path().join("Runtime/Textures").join(f), b"x").unwrap();
        }
        let dirs = vec![dir.path().to_path_buf()];
        let terrain = mat(json!([
            {"channel": {"id": "Diffuse Texture Map", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/d.png"}},
            {"channel": {"id": "AO Texture Map", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/ao.png"}},
            {"channel": {"id": "Damp Map", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/damp.png"}},
            {"channel": {"id": "Damp Color", "current_value": [0.45, 0.42, 0.36]}},
            {"channel": {"id": "Damp Blend Mode", "current_value": 2}},
            {"channel": {"id": "Map Horizontal Tiles", "current_value": 20}},
            {"channel": {"id": "Map Vertical Tiles", "current_value": 20}},
        ]));
        let (x, _) = material_inputs(&[("Terrain".into(), terrain, 1.0)], &dirs, HashMap::new(), 9000, SkinShader::None);
        let albedo = &x[x.find("<pin name='albedo'>").unwrap()..x.find("<pin name='specular'>").unwrap_or(x.len())];
        // Multiply(diffuse, Multiply(AO, Mix(white, damp colour, damp map))).
        assert_eq!(albedo.matches("type='39'").count(), 2, "{albedo}");
        assert!(albedo.contains("type='38'"), "{albedo}");
        // The diffuse and AO maps tiled 20×, the damp map once.
        let damp = &albedo[albedo.find("damp.png").unwrap()..];
        assert_eq!(albedo[..albedo.find("damp.png").unwrap()].matches("0.05 0.05 1").count(), 2, "{albedo}");
        assert!(!damp[..damp.find("</node>").unwrap()].contains("0.05 0.05 1"), "{albedo}");
    }

    #[test]
    fn openpbr_skin_lift_rises_to_half_translucency_then_falls() {
        assert_eq!(openpbr_skin_lift(0.0), 1.0);
        assert!((openpbr_skin_lift(0.25) - 1.15).abs() < 1e-5);
        assert!((openpbr_skin_lift(0.5) - 1.3).abs() < 1e-5);
        assert!((openpbr_skin_lift(0.9) - 0.45).abs() < 1e-5);
        assert!((openpbr_skin_lift(1.5) - 0.35).abs() < 1e-5, "clamped to w 1");
    }

    #[test]
    fn dual_lobe_second_lobe_as_coating() {
        let dirs: Vec<std::path::PathBuf> = vec![];
        let uber = mat(json!([
            {"channel": {"id": "Dual Lobe Specular Weight", "current_value": 1}},
            {"channel": {"id": "Dual Lobe Specular Reflectivity", "current_value": 0.5}},
            {"channel": {"id": "Specular Lobe 1 Roughness", "current_value": 0.3}},
            {"channel": {"id": "Specular Lobe 2 Roughness", "current_value": 0.6}},
            {"channel": {"id": "Dual Lobe Specular Ratio", "current_value": 0.25}},
        ]));
        let dl = DualLobe::of(&uber).unwrap();
        assert_eq!((dl.share, dl.lobe1, dl.ior), (1.0, 0.25, None));
        assert!((dl.lobe2_roughness - 0.72).abs() < 1e-6, "{dl:?}");
        let (x, _) = material_inputs(&[("Uber".into(), uber.clone(), 1.0)], &dirs, HashMap::new(), 9000, SkinShader::None);
        assert!(x.contains("<pin name='specular'>") && x.contains(">0.25 0.25 0.25<"), "lobe 1's share: {x}");
        assert!(x.contains(">0.75 0.75 0.75<") && x[x.find("<pin name='coatingRoughness'>").unwrap()..][..200].contains("type='6'>0.848528</attr>"), "lobe 2 as the coating (√0.72 below the GGX curve): {x}");

        // PBRSkin: the ratio weighs lobe 2, its roughness lobe 1's × the mult.
        let pbr = mat(json!([
            {"channel": {"id": "Dual Lobe Specular Weight", "current_value": 1}},
            {"channel": {"id": "Specular Lobe 1 Roughness", "current_value": 0.5}},
            {"channel": {"id": "Specular Lobe 2 Roughness Mult", "current_value": 0.4}},
            {"channel": {"id": "Dual Lobe Specular Ratio", "current_value": 0.15}},
            {"channel": {"id": "Dual Lobe Specular Reflectivity", "current_value": 1}},
        ]));
        let dl = DualLobe::of(&pbr).unwrap();
        assert!((dl.lobe1 - 0.85).abs() < 1e-6 && (dl.lobe2_roughness - 0.08).abs() < 1e-6, "{dl:?}");
        assert!((dl.ior.unwrap() - ior_from_f0(0.08)).abs() < 1e-6, "reflectivity 1: F0 0.08");

        // Under a Top Coat the coating is the coat's; one blended lobe.
        let coated = mat(json!([
            {"channel": {"id": "Dual Lobe Specular Weight", "current_value": 1}},
            {"channel": {"id": "Specular Lobe 1 Roughness", "current_value": 0.3}},
            {"channel": {"id": "Specular Lobe 2 Roughness", "current_value": 0.6}},
            {"channel": {"id": "Dual Lobe Specular Ratio", "current_value": 0.25}},
            {"channel": {"id": "Top Coat Weight", "current_value": 1}},
        ]));
        let (x, _) = material_inputs(&[("Coated".into(), coated, 1.0)], &dirs, HashMap::new(), 9000, SkinShader::None);
        assert!(!x.contains(">0.75 0.75 0.75<") && !x.contains("<pin name='specular'>"), "full lobe share: {x}");
        let ggx = |r: f32| (r + GGX_ROUGHNESS_CURVE * r * r).min(r.sqrt()).min(1.0);
        let blended = ggx(0.25 * 0.09 + 0.75 * 0.72);
        assert!(x.contains(&format!(">{}<", num(blended))), "blended roughness {blended} (GGX curve): {x}");
        assert!(DualLobe::of(&mat(json!([]))).is_none());
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
            {"channel": {"id": "Base Color Effect", "current_value": 1}},
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
        // OpenPBR on mono SSS with Base Color Effect 1 (this one): radius the distance, scale the
        // colour; the tint lifted by 1.3 (`openpbr_skin_lift` at w 0.5).
        let (mx, _) = material_inputs(&[("Torso".into(), sss.clone(), 1.0)], &dirs, HashMap::new(), 8050, SkinShader::OpenPbr);
        assert!(mx.contains("type='505' name='Torso'"), "{mx}");
        assert!(mx.contains("<attr name='value' type='8'>0.65 0.52 0.403</attr>"), "lifted tint: {mx}");
        // On chromatic SSS: the colour baked per texel (`skin_albedo`), the
        // same for base and subsurface; the diffuse map's detail mask as the
        // subsurface weight; a gloss. A readable diffuse map to bake from.
        image::RgbImage::from_pixel(8, 8, image::Rgb([200, 150, 120])).save(dir.path().join("Runtime/Textures/skin.jpg")).unwrap();
        let mut chromatic = (*sss).clone();
        chromatic.channels.insert("SSS Mode".into(), crate::daz_material::Channel { value: json!(1), image: None, gamma: None });
        let chromatic = std::rc::Rc::new(chromatic);
        let (ox, _) = material_inputs(&[("Torso".into(), chromatic, 1.0)], &dirs, HashMap::new(), 8100, SkinShader::OpenPbr);
        assert!(ox.contains("type='505' name='Torso'"), "{ox}");
        let radius_pin = &ox[ox.find("<pin name='radius'>").unwrap()..];
        assert!(radius_pin[..120].contains("type='6'>") && ox[ox.find("<pin name='scale'>").unwrap()..].contains("type='33'>"), "{ox}");
        let pin = |name: &str| {
            let p = &ox[ox.find(&format!("<pin name='{name}'>")).unwrap()..];
            p[..p.find("</pin>\n      <pin").unwrap_or(p.len())].to_string()
        };
        assert!(pin("baseColor").contains("_skin.png") && pin("subsurfaceColor").contains("_skin.png"), "{ox}");
        assert!(pin("subsurface").contains("_skin_detail.png"), "{ox}");
        assert!(ox.contains("<pin name='specular'>"), "{ox}");
        let (a, b) = skin_albedo(&sss, 0.5);
        assert!((a[0] - 0.5f32.powf(1.3)).abs() < 1e-6 && (b[0] - 0.5 * 0.55 * volume_factor(&sss)[0]).abs() < 1e-6, "{a:?} {b:?}");
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
        let (tx, _) = material_inputs(&[("Shirt".into(), shirt, 1.0)], &dirs, HashMap::new(), 8700, SkinShader::StandardSurface);
        assert!(tx.contains("type='130'"), "thin-walled fabric stays Universal");
        // With Base Color Effect 1 (Scatter & Transmit) it reflects 0.4 of
        // its 1 − 0.3 diffuse and passes 2 × 0.3.
        let scatter = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.3}},
            {"channel": {"id": "Thin Walled", "current_value": true}},
            {"channel": {"id": "Base Color Effect", "current_value": 1}},
        ]));
        let (ex, _) = material_inputs(&[("Socket".into(), scatter, 1.0)], &dirs, HashMap::new(), 8720, SkinShader::StandardSurface);
        let power = |x: &str| x[x.find("<pin name='albedo'>").unwrap()..].split("<pin name='power'>").nth(1).unwrap().split("</attr>").next().unwrap().rsplit('>').next().unwrap().to_string();
        let transmission = |x: &str| x[x.find("<pin name='transmission'>").unwrap()..].split("</attr>").next().unwrap().rsplit('>').next().unwrap().to_string();
        assert_eq!((power(&tx), power(&ex)), ("0.7".to_string(), "0.28".to_string()), "{ex}");
        assert_eq!((transmission(&tx), transmission(&ex)), ("0.3 0.3 0.3".to_string(), "0.6 0.6 0.6".to_string()), "{ex}");
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
        // Base Mixing 1: Glossy Specular 0.5 (linear 0.218) is F0 → IOR 2.75;
        // black reflects nothing head-on (IOR 1).
        for (gs, ior) in [(0.5, "2.74"), (0.0, "1")] {
            let m = mat(json!([
                {"channel": {"id": "Base Mixing", "current_value": 1}},
                {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
                {"channel": {"id": "Glossy Specular", "current_value": [gs, gs, gs]}},
            ]));
            let (x, _) = material_inputs(&[("Spec".into(), m, 1.0)], &dirs, HashMap::new(), 8920, SkinShader::None);
            let pin = &x[x.find("<pin name='ior'>").expect("F0 as the IOR")..];
            assert!(pin[..pin.len().min(200)].contains(&format!("<attr name='value' type='6'>{ior}")), "{pin}");
        }
        // A thin film tints what is under it, head-on and grazing, as
        // NVIDIA's legacy_thin_film (Calib-ThinFilm's 500 nm: olive head-on,
        // the blue gone at the rim; at IOR 1.3 blue at the rim).
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.002);
        assert!(close(film_reflectance(1.5, 500.0, 0.99), [0.8232, 0.9574, 0.8898]));
        assert!(close(film_reflectance(1.5, 500.0, 0.10), [0.9674, 0.7746, 0.0466]));
        assert!(close(film_reflectance(1.3, 500.0, 0.10), [0.0579, 0.1068, 0.9155]));
        // The coat's film tints the coating and the base, the base's film
        // only the base.
        let filmed = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Reflectivity", "current_value": 0.5}},
            {"channel": {"id": "Base Thin Film", "current_value": 500}},
            {"channel": {"id": "Top Coat Weight", "current_value": 1}},
        ]));
        let (fx, _) = material_inputs(&[("Film".into(), filmed, 1.0)], &dirs, HashMap::new(), 8940, SkinShader::None);
        for pin in ["albedo", "specular"] {
            let p = &fx[fx.find(&format!("<pin name='{pin}'>")).expect(&fx)..];
            assert!(p[..120].contains("type='39'") && p[..1200].contains("type='50'") && p[..1600].contains("0.823211 0.957422 0.889831"), "{p}");
        }
        let coating = &fx[fx.find("<pin name='coating'>").unwrap()..];
        assert!(!coating[..120].contains("type='39'"), "{coating}");
        assert_eq!(fx.matches("<pin").count(), fx.matches("</pin>").count());
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
        // Its own roughness 0.3 (0.09 squared) as Beckmann's √r (#72).
        let pin = |x: &str, p: &str| x[x.find(&format!("<pin name='{p}'>")).unwrap()..].split("</attr>").next().unwrap().rsplit('>').next().unwrap().to_string();
        assert_eq!((pin(&bx, "brdf"), pin(&bx, "roughness")), (GLASS_BRDF.to_string(), num(0.3)), "{bx}");
        // Shared with a rough glossy layer: the same rule on the glossy
        // roughness, the reflection included.
        let frosted = mat(json!([
            {"channel": {"id": "Refraction Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0.6}},
        ]));
        let (fx, _) = material_inputs(&[("Frosted".into(), frosted, 1.0)], &dirs, HashMap::new(), 8985, SkinShader::None);
        assert_eq!((pin(&fx, "brdf"), pin(&fx, "roughness")), (GLASS_BRDF.to_string(), num(0.6)), "{fx}");
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
        let r = num(0.09 + GGX_ROUGHNESS_CURVE * 0.0081);
        assert_eq!((pin("roughness"), pin("coatingRoughness")), (r.clone(), r), "{cx}");
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
    fn colour_effects_split_and_tint() {
        // Base Mixing 1: Glossy Specular's hue over its strongest channel,
        // not clipped — a red specular stays red.
        let red = mat(json!([{"channel": {"id": "Glossy Specular", "current_value": [0.9, 0.3, 0.2]}}]));
        let hue = specular_hue(&red);
        assert!((hue[0] - 1.0).abs() < 1e-6 && hue[1] < 0.2 && hue[2] < 0.1, "{hue:?}");
        let grey = mat(json!([{"channel": {"id": "Glossy Specular", "current_value": [0.5, 0.5, 0.5]}}]));
        assert_eq!(specular_hue(&grey), [1.0; 3]);
        // Under the red gloss the diffuse keeps Iray's 1 − F0 in every
        // channel (Octane's 1 − F0·t compensated); a grey one is left alone.
        let share = specular_diffuse_share(&red, 1.0, [1.0; 3]);
        assert!((share[0] - 1.0).abs() < 1e-6 && share[1] < 0.3 && share[2] < 0.3, "{share:?}");
        assert!(specular_diffuse_share(&grey, 1.0, [1.0; 3]).iter().all(|s| (s - 1.0).abs() < 1e-6));
        // Base Color Effect 2 shares out diffuse and translucency like 1,
        // by its own weights.
        let effect = |e: i32| {
            let m = mat(json!([
                {"channel": {"id": "diffuse", "current_value": [0.5, 0.5, 0.5]}},
                {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
                {"channel": {"id": "Base Color Effect", "current_value": e}},
            ]));
            let (x, _) = material_inputs(&[("Card".into(), m, 1.0)], &[], HashMap::new(), 7000, SkinShader::None);
            let albedo = &x[x.find("<pin name='albedo'>").unwrap()..];
            albedo[albedo.find("type='8'>").unwrap() + 9..].split(' ').next().unwrap().parse::<f32>().unwrap()
        };
        let (e0, e1, e2) = (effect(0), effect(1), effect(2));
        assert!((e1 / e0 - SCATTER_TRANSMIT_DIFFUSE).abs() < 1e-3, "{e0} {e1}");
        assert!((e2 / e0 - SCATTER_TRANSMIT_INTENSITY_DIFFUSE).abs() < 1e-3, "{e0} {e2}");
    }

    #[test]
    fn a_translucent_props_volume_scales_its_transmission() {
        let transmission = |extra: serde_json::Value| {
            let mut ch = vec![json!({"channel": {"id": "diffuse", "current_value": [0.8, 0.8, 0.8]}}), json!({"channel": {"id": "Translucency Weight", "current_value": 0.9}})];
            ch.extend(extra.as_array().unwrap().iter().cloned());
            let (x, _) = material_inputs(&[("Ball".into(), mat(serde_json::Value::Array(ch)), 1.0)], &[], HashMap::new(), 8000, SkinShader::None);
            let t = &x[x.find("<pin name='transmission'>").unwrap()..];
            t[t.find("type='8'>").unwrap() + 9..].split('<').next().unwrap().split(' ').map(|v| v.parse::<f32>().unwrap()).collect::<Vec<_>>()
        };
        // No volume: the translucency as it is.
        assert!((transmission(json!([]))[0] - 0.9).abs() < 1e-4);
        // Victoria 8's volume (the reference): × the level alone.
        let victoria = json!([
            {"channel": {"id": "Transmitted Color", "current_value": [0.9764706, 0.4823529, 0.3529412]}},
            {"channel": {"id": "Transmitted Measurement Distance", "current_value": 0.125}},
            {"channel": {"id": "Scattering Measurement Distance", "current_value": 0.015}},
            {"channel": {"id": "SSS Mode", "current_value": 1}},
            {"channel": {"id": "SSS Color", "current_value": [0.9764706, 0.6941177, 0.7607843]}},
        ]);
        let t = transmission(victoria);
        assert!(t.iter().all(|v| (v - 0.9 * PROP_VOLUME_LEVEL).abs() < 1e-3), "{t:?}");
    }

    /// DAZ's Round Corners (Calib-RoundCorners .mi: the channels as they
    /// are, the radius in meters) → Universal's Round Edges, Accurate; none
    /// at radius 0; also on a weighted mix's parts.
    #[test]
    fn round_corners_are_octane_round_edges() {
        let rc = |extra: serde_json::Value| {
            let mut ch = vec![json!({"channel": {"id": "diffuse", "current_value": [0.5, 0.5, 0.5]}})];
            ch.extend(extra.as_array().unwrap().iter().cloned());
            material_inputs(&[("Box".into(), mat(serde_json::Value::Array(ch)), 1.0)], &[], HashMap::new(), 5000, SkinShader::None).0
        };
        let none = rc(json!([]));
        assert!(!none.contains("roundEdges"), "{none}");
        let x = rc(json!([
            {"channel": {"id": "Round Corners Radius", "current_value": 0.01}},
            {"channel": {"id": "Round Corners Roundness", "current_value": 0.0}},
        ]));
        let r = &x[x.find("<pin name='roundEdges'>").expect("round edges")..];
        assert!(r.contains("type='137'"), "{r}");
        let val = |p: &str| r[r.find(&format!("<pin name='{p}'>")).unwrap()..].split("</attr>").next().unwrap().rsplit('>').next().unwrap().to_string();
        assert_eq!(val("roundEdgesMode"), "2");
        assert_eq!(val("roundEdgesRadius"), "0.01 0 0 0");
        assert_eq!(val("roundEdgesCurvatureRoundness"), "0 0 0 0");
        assert_eq!(val("roundEdgesConsiderOtherObjects"), "0");
        assert_eq!(x.matches("<node ").count(), x.matches("</node>").count());
        // A weighted gloss mix: both its materials round.
        let w = rc(json!([
            {"channel": {"id": "Round Corners Radius", "current_value": 0.02}},
            {"channel": {"id": "Base Mixing", "current_value": 2}},
            {"channel": {"id": "Glossy Weight", "current_value": 0.5}},
        ]));
        assert!(w.contains("type='19'"), "{w}");
        assert_eq!(w.matches("<pin name='roundEdges'>").count(), 2, "{w}");
    }

    #[test]
    fn weighted_top_coat_is_a_mix_with_a_mirror() {
        let coated = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0.8, 0.1, 0.1]}},
            {"channel": {"id": "Top Coat Weight", "current_value": 0.4}},
            {"channel": {"id": "Top Coat Layering Mode", "current_value": 1}},
            {"channel": {"id": "Top Coat Color", "current_value": [1, 0.9, 0.8]}},
            {"channel": {"id": "Top Coat Roughness", "current_value": 0.1}},
        ]));
        let (x, _) = material_inputs(&[("Ball".into(), coated, 1.0)], &[], HashMap::new(), 5000, SkinShader::None);
        assert!(x.contains("type='19' name='Ball'"), "{x}");
        let amount = &x[x.find("<pin name='amount'>").unwrap()..];
        assert!(amount[..200].contains("type='6'>0.4</attr>"), "{amount}");
        // The coat: a GGX metal in the top coat colour (linear), Iray's
        // roughness 0.1² on the GGX curve.
        let coat = &x[x.find("name='Ball top coat'").unwrap()..];
        let coat = &coat[..coat.find("name='Ball base'").unwrap()];
        assert!(coat.contains("<pin name='metallic'>") && coat.contains(&format!("type='6'>{}</attr>", num(0.01 + GGX_ROUGHNESS_CURVE * 1e-4))) && coat.contains("<pin name='brdf'>"), "{coat}");
        // The base keeps no coating of its own.
        let base = &x[x.find("name='Ball base'").unwrap()..];
        assert!(!base.contains("<pin name='coating'>"), "{base}");
        assert_eq!(x.matches("<node ").count(), x.matches("</node>").count());
        // Layering Mode 0 stays Universal's coating.
        let reflectivity = mat(json!([
            {"channel": {"id": "Top Coat Weight", "current_value": 0.4}},
        ]));
        let (y, _) = material_inputs(&[("Ball".into(), reflectivity, 1.0)], &[], HashMap::new(), 6000, SkinShader::None);
        assert!(!y.contains("type='19'") && y.contains("<pin name='coating'>"), "{y}");
    }

    #[test]
    fn bump_power_follows_iray_per_texel_slope() {
        // Iray's file_bump_texture tilts by factor / the map's width,
        // whatever the tiling or the object's size; Octane by power × the
        // slope per meter (#75): power = factor × 2 × 512 / width ×
        // UV scale / 0.542 / tiles.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("t")).unwrap();
        for (f, n) in [("h512.png", 512), ("h2048.png", 2048)] {
            image::GrayImage::new(n, n).save(dir.path().join("t").join(f)).unwrap();
        }
        let dirs = vec![dir.path().to_path_buf()];
        let power = |map: &str, extra: serde_json::Value, scale: Option<f64>| -> f32 {
            let mut ch = vec![json!({"channel": {"id": "Bump Strength", "current_value": 1.5, "image_file": map}})];
            ch.extend(extra.as_array().unwrap().iter().cloned());
            let m = mat(serde_json::Value::Array(ch));
            let scales: UvScales = scale.map(|s| (std::rc::Rc::as_ptr(&m), s)).into_iter().collect();
            let (x, _) = material_inputs_scaled(&[("Ball".into(), m, 1.0)], &scales, &dirs, HashMap::new(), 9000, SkinShader::None);
            let b = &x[x.find("<pin name='bump'>").expect(&x)..];
            let p = &b[b.find("<pin name='power'>").unwrap()..];
            p[p.find("type='6'>").unwrap() + 9..].split('<').next().unwrap().parse().unwrap()
        };
        let close = |a: f32, b: f32| assert!((a - b).abs() < 1e-4, "{a} vs {b}");
        // The reference: 512 px, untiled, Calib-CoatBump's spheres.
        close(power("/t/h512.png", json!([]), Some(BUMP_REF_UV_SCALE)), 3.0);
        // No UV scale known: as the reference.
        close(power("/t/h512.png", json!([]), None), 3.0);
        // 4x the pixels: a quarter of the power.
        close(power("/t/h2048.png", json!([]), Some(BUMP_REF_UV_SCALE)), 0.75);
        // Tiled 2 x 2: half.
        close(power("/t/h512.png", json!([{"channel": {"id": "Horizontal Tiles", "current_value": 2}}, {"channel": {"id": "Vertical Tiles", "current_value": 2}}]), Some(BUMP_REF_UV_SCALE)), 1.5);
        // A surface twice as large per UV unit: twice.
        close(power("/t/h512.png", json!([]), Some(2.0 * BUMP_REF_UV_SCALE)), 6.0);
    }

    #[test]
    fn top_coat_bump_goes_on_the_coats_own_pins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("t")).unwrap();
        for f in ["coat.png", "coat-n.png", "base.png", "base-n.png"] {
            std::fs::write(dir.path().join("t").join(f), b"x").unwrap();
        }
        let dirs = vec![dir.path().to_path_buf()];
        let export = |extra: serde_json::Value| {
            let mut ch = vec![json!({"channel": {"id": "Top Coat Weight", "current_value": 1}})];
            ch.extend(extra.as_array().unwrap().iter().cloned());
            let (x, _) = material_inputs(&[("Ball".into(), mat(serde_json::Value::Array(ch)), 1.0)], &dirs, HashMap::new(), 9000, SkinShader::None);
            x
        };
        // The pin's image file and its power.
        let pin = |x: &str, name: &str| -> Option<(String, String)> {
            let p = &x[x.find(&format!("<pin name='{name}'>"))?..];
            let file = p[p.find("type='11'>").unwrap() + 10..].split('<').next().unwrap().rsplit('/').next().unwrap();
            // Assets are numbered: `0000_coat.png`.
            let file = file.split_once('_').map_or(file, |(_, f)| f).to_string();
            let power = &p[p.find("<pin name='power'>").unwrap()..];
            Some((file, power[power.find("type='6'>").unwrap() + 9..].split('<').next().unwrap().to_string()))
        };
        // A height map at Top Coat Bump × BUMP_POWER as its power.
        let x = export(json!([{"channel": {"id": "Top Coat Bump", "current_value": 3, "image_file": "/t/coat.png"}}]));
        assert_eq!(pin(&x, "coatingBump"), Some(("coat.png".into(), "6".into())), "{x}");
        assert!(pin(&x, "coatingNormal").is_none() && pin(&x, "bump").is_none());
        // Mode 1: a normal map.
        let x = export(json!([
            {"channel": {"id": "Top Coat Bump", "current_value": 1, "image_file": "/t/coat-n.png"}},
            {"channel": {"id": "Top Coat Bump Mode", "current_value": 1}},
        ]));
        assert_eq!(pin(&x, "coatingNormal").map(|p| p.0), Some("coat-n.png".into()), "{x}");
        assert!(pin(&x, "coatingBump").is_none());
        // Without its own, the coat follows the base's maps, as in Iray.
        let x = export(json!([
            {"channel": {"id": "Bump Strength", "current_value": 1, "image_file": "/t/base.png"}},
            {"channel": {"id": "Normal Map", "current_value": 1, "image_file": "/t/base-n.png"}},
        ]));
        assert_eq!(pin(&x, "coatingBump").map(|p| p.0), Some("base.png".into()), "{x}");
        assert_eq!(pin(&x, "coatingNormal").map(|p| p.0), Some("base-n.png".into()), "{x}");
        assert_eq!(pin(&x, "bump"), Some(("base.png".into(), "2".into())));
        // Bump Strength 0: no bump.
        let x = export(json!([{"channel": {"id": "Bump Strength", "current_value": 0, "image_file": "/t/base.png"}}]));
        assert!(pin(&x, "bump").is_none() && pin(&x, "coatingBump").is_none(), "{x}");
        // The coat's and the base's height maps add up (a Mix at ½ of
        // both at twice their power); the base's normal map stays.
        let x = export(json!([
            {"channel": {"id": "Top Coat Bump", "current_value": 1, "image_file": "/t/coat.png"}},
            {"channel": {"id": "Bump Strength", "current_value": 0.5, "image_file": "/t/base.png"}},
            {"channel": {"id": "Normal Map", "current_value": 1, "image_file": "/t/base-n.png"}},
        ]));
        let mix = &x[x.find("<pin name='coatingBump'>").unwrap()..];
        assert!(mix[..120].contains("type='38'") && mix.contains("<attr name='value' type='6'>0.5</attr>"), "{x}");
        assert_eq!(pin(mix, "texture1"), Some(("coat.png".into(), "4".into())), "{x}");
        assert_eq!(pin(mix, "texture2"), Some(("base.png".into(), "2".into())), "{x}");
        assert_eq!(pin(&x, "coatingNormal").map(|p| p.0), Some("base-n.png".into()));
        // A weighted coat (the Mix's metal) and an anisotropic one (the
        // Specular Layer) carry it on their own bump pin.
        for mode in [json!([{"channel": {"id": "Top Coat Layering Mode", "current_value": 1}}]), json!([{"channel": {"id": "Top Coat Anisotropy", "current_value": 1}}])] {
            let mut ch = vec![json!({"channel": {"id": "Top Coat Bump", "current_value": 2, "image_file": "/t/coat.png"}})];
            ch.extend(mode.as_array().unwrap().iter().cloned());
            let x = export(serde_json::Value::Array(ch));
            let coat = &x[x.find("name='Ball top coat'").unwrap()..];
            assert_eq!(pin(coat, "bump"), Some(("coat.png".into(), "4".into())), "{x}");
            assert!(pin(&x, "coatingBump").is_none());
        }
    }

    #[test]
    fn metallic_flakes_are_a_mix_with_a_metal() {
        let flaked = mat(json!([
            {"channel": {"id": "Metallic Flakes Weight", "current_value": 1}},
            {"channel": {"id": "Metallic Flakes Density", "current_value": 1}},
            {"channel": {"id": "Metallic Flakes Color", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Top Coat Weight", "current_value": 1}},
        ]));
        let (x, _) = material_inputs(&[("Paint".into(), flaked, 1.0)], &[], HashMap::new(), 5000, SkinShader::None);
        assert!(x.contains("type='19' name='Paint'") && x.contains("name='Paint flakes'") && x.contains("name='Paint base'"), "{x}");
        let amount = &x[x.find("<pin name='amount'>").unwrap()..];
        assert!(amount[..200].contains(&format!("type='6'>{}</attr>", num(FLAKE_COVERAGE))), "{amount}");
        // The coat lies over both parts.
        assert_eq!(x.matches("<pin name='coating'>").count(), 2, "{x}");
        assert_eq!(x.matches("<node ").count(), x.matches("</node>").count());
    }

    #[test]
    fn fresnel_and_curve_coats_ignore_the_channels_iray_never_gets() {
        let coat = |mode: i32| {
            let m = mat(json!([
                {"channel": {"id": "Top Coat Weight", "current_value": 1}},
                {"channel": {"id": "Top Coat Layering Mode", "current_value": mode}},
                {"channel": {"id": "Top Coat IOR", "current_value": 2.0}},
                {"channel": {"id": "Top Coat Curve Normal", "current_value": 0.3}},
                {"channel": {"id": "Reflectivity", "current_value": 1.0}},
            ]));
            top_coat_ior(&m)
        };
        assert_eq!(coat(2), 1.5);
        assert_eq!(coat(3), CURVE_COAT_IOR);
        assert!((coat(0) - ior_from_f0(0.08)).abs() < 1e-6);
    }

    #[test]
    fn abbe_number_is_universals_dispersion_on_a_smooth_transmission() {
        let glass = |extra: serde_json::Value| {
            let mut chs = vec![
                json!({"channel": {"id": "Refraction Weight", "current_value": 1.0}}),
                json!({"channel": {"id": "Refraction Index", "current_value": 1.5}}),
                json!({"channel": {"id": "Glossy Layered Weight", "current_value": 1.0}}),
                json!({"channel": {"id": "Glossy Roughness", "current_value": 0.4}}),
            ];
            chs.extend(extra.as_array().unwrap().iter().cloned());
            let (x, _) = material_inputs(&[("Glass".into(), mat(json!(chs)), 1.0)], &[], HashMap::new(), 5000, SkinShader::None);
            x
        };
        let pin = |x: &str, name: &str| x.find(&format!("<pin name='{name}'>")).map(|at| {
            let v = &x[at..];
            let v = &v[v.find("type='6'>").unwrap() + 9..];
            v[..v.find('<').unwrap()].parse::<f32>().unwrap()
        });
        // A float value node (6) holding a float3.
        let abbe = |x: &str| x.find("<pin name='dispersion_coefficient_B'>").map(|at| {
            let v = &x[at..];
            let v = &v[v.find("type='8'>").unwrap() + 9..];
            v[..v.find('<').unwrap()].to_string()
        });
        // Abbe 0: no dispersion, the glossy roughness.
        let clear = glass(json!([]));
        assert_eq!(abbe(&clear), None, "{clear}");
        assert!(pin(&clear, "roughness").unwrap() > 0.1, "{clear}");
        // Abbe 20: the coefficient as is, the transmission smooth.
        let flint = glass(json!([{"channel": {"id": "Abbe", "current_value": 20.0}}]));
        assert_eq!(abbe(&flint).as_deref(), Some("20 0 0"), "{flint}");
        assert_eq!(pin(&flint, "roughness").unwrap_or(0.0), 0.0, "{flint}");
        // Thin-walled: no bending, nothing to disperse.
        let thin = glass(json!([
            {"channel": {"id": "Abbe", "current_value": 20.0}},
            {"channel": {"id": "Thin Walled", "current_value": true}},
        ]));
        assert_eq!(abbe(&thin), None, "{thin}");
    }

    #[test]
    fn glossy_anisotropy_is_octanes_long_axis_and_rotation() {
        let ball = |extra: serde_json::Value| {
            let mut chs = vec![
                json!({"channel": {"id": "diffuse", "current_value": [0.5, 0.5, 0.5]}}),
                json!({"channel": {"id": "Glossy Layered Weight", "current_value": 1.0}}),
                json!({"channel": {"id": "Glossy Roughness", "current_value": 0.4}}),
            ];
            chs.extend(extra.as_array().unwrap().iter().cloned());
            let (x, _) = material_inputs(&[("Ball".into(), mat(json!(chs)), 1.0)], &[], HashMap::new(), 5000, SkinShader::None);
            x
        };
        let pin = |x: &str, name: &str| x.find(&format!("<pin name='{name}'>")).map(|at| {
            let v = &x[at..];
            let v = &v[v.find("type='6'>").unwrap() + 9..];
            v[..v.find('<').unwrap()].parse::<f32>().unwrap()
        });
        // Isotropic: no anisotropy pins, GGX's 0.16 + 1.85 · 0.16².
        let iso = ball(json!([]));
        assert_eq!(pin(&iso, "anisotropyTexture"), None, "{iso}");
        assert!((pin(&iso, "roughness").unwrap() - 0.20736).abs() < 1e-5, "{iso}");
        let brdf = |x: &str| x.find("<pin name='brdf'>").map(|at| x[at..].split("type='2'>").nth(1).unwrap().split('<').next().unwrap().to_string());
        // Anisotropy 0.5: v = 0.16 / 0.5 = 0.32 → 0.50944 the roughness,
        // (0.50944 − 0.20736) / (0.50944 + 0.20736) the anisotropy; Iray's
        // rotation 0.125 the other way round, 0.875; energy-preserving GGX
        // below the cap.
        let a = ball(json!([
            {"channel": {"id": "Glossy Anisotropy", "current_value": 0.5}},
            {"channel": {"id": "Glossy Anisotropy Rotations", "current_value": 0.125}},
        ]));
        assert!((pin(&a, "roughness").unwrap() - 0.50944).abs() < 1e-5, "{a}");
        assert!((pin(&a, "anisotropyTexture").unwrap() - 0.421429).abs() < 1e-5, "{a}");
        assert_eq!(pin(&a, "rotation"), Some(0.875));
        assert_eq!(brdf(&a).as_deref(), Some("6"));
        // Anisotropy 1: v past Iray's cap, roughness 1, plain GGX.
        let full = ball(json!([{"channel": {"id": "Glossy Anisotropy", "current_value": 1.0}}]));
        assert_eq!(pin(&full, "roughness"), Some(1.0));
        assert_eq!(pin(&full, "rotation"), None);
        assert_eq!(brdf(&full).as_deref(), Some("2"));
        assert_eq!(brdf(&iso).as_deref(), Some("6"));
        // Backscattering Anisotropy alone never reaches Iray.
        let back = ball(json!([{"channel": {"id": "Backscattering Anisotropy", "current_value": 0.8}}]));
        assert_eq!(pin(&back, "anisotropyTexture"), None);
    }

    #[test]
    fn backscattering_is_a_retro_reflecting_mirror() {
        let ball = |weight: f32, rough: f32| {
            let chs = json!([
                {"channel": {"id": "diffuse", "current_value": [0.45, 0.05, 0.04]}},
                {"channel": {"id": "Backscattering Weight", "current_value": weight}},
                {"channel": {"id": "Backscattering Roughness", "current_value": rough}},
            ]);
            material_inputs(&[("Ball".into(), mat(chs), 1.0)], &[], HashMap::new(), 5000, SkinShader::None).0
        };
        let value = |x: &str, pin: &str, ty: &str| -> String {
            let v = &x[x.find(&format!("<pin name='{pin}'>")).unwrap_or_else(|| panic!("{pin}: {x}"))..];
            v[v.find(&format!("type='{ty}'>")).unwrap() + 9..].split('<').next().unwrap().to_string()
        };
        let x = ball(0.5, 0.3);
        // The Mix by the weight, linear like MDL's weighted_layer.
        assert_eq!(value(&x, "amount", "6").parse::<f32>().unwrap(), 0.5);
        let lobe = &x[x.find("Ball backscatter'>").expect(&x)..];
        let lobe = &lobe[..lobe.find("Ball base'>").unwrap()];
        // Glossy Color (white) × the lobe's mean cosine, Beckmann at the
        // roughness whose lobe equals MDL's exponent 2 / r² (r 0.09), the
        // normal turned toward the viewer by the OSL texture.
        let albedo: Vec<f32> = value(lobe, "albedo", "8").split(' ').map(|v| v.parse().unwrap()).collect();
        assert!(albedo.iter().all(|a| (a - retro_albedo(0.09)).abs() < 1e-4), "{albedo:?}");
        assert_eq!(value(lobe, "brdf", "2"), "1");
        let rough: f32 = value(lobe, "roughness", "6").parse().unwrap();
        assert!((rough - 0.2120).abs() < 1e-3, "{rough}");
        let normal = &lobe[lobe.find("<pin name='normal'>").expect(lobe)..];
        assert!(normal.contains("type='117'") && normal.contains("shaderCode") && normal.contains("-normalize(I)"), "{normal}");
        assert!(normal.contains("cross(n, t)"), "{normal}");
        // Rough (r 0.49): roughness 0.488, 0.90 of the tint.
        let x = ball(1.0, 0.7);
        let lobe = &x[x.find("Ball backscatter'>").expect(&x)..];
        assert!((value(lobe, "roughness", "6").parse::<f32>().unwrap() - 0.4878).abs() < 1e-3, "{lobe}");
        assert!((retro_albedo(0.49) - 0.9032).abs() < 1e-3);
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
            {"channel": {"id": "Highlight Weight", "current_value": 0.0}},
        ]));
        let (hx, tex) = material_inputs(&[("Curls".into(), hair, 0.0)], &[dir.path().to_path_buf()], HashMap::new(), 4000, SkinShader::None);
        assert!(hx.contains("type='147' name='Curls'"), "{hx}");
        // Root 0.5 × (1, 0.218, 0.218) (the colour linear) and tip 0.5,
        // × 0.046, + 0.110 × the (default white) transmission colour, from
        // root to tip along the strand (W), linear at bias / gain 0.5.
        let albedo = &hx[hx.find("<pin name='albedo'>").unwrap()..hx.find("<pin name='specular'>").unwrap()];
        assert!(albedo.contains("type='49'") && albedo.contains("type='104'") && albedo.contains("<attr name='controlpoints' type='2'>0</attr>"), "{albedo}");
        let min = albedo.find("<pin name='min'>").unwrap();
        let max = albedo.find("<pin name='max'>").unwrap();
        assert!(albedo[min..max].contains(">0.133134 0.115035 0.115035<") && albedo[max..].contains(">0.133134 0.133134 0.133134<"), "{albedo}");
        assert!(hx.contains("<pin name='specular'>") && hx.contains("<attr name='value' type='8'>0.22 0.22 0.22</attr>"));
        assert!(hx.contains("<attr name='value' type='6'>0.35</attr>"));
        assert!(tex.files.is_empty(), "the map is averaged, not packed");
        assert_eq!(hx.matches("<node ").count(), hx.matches("</node>").count() + hx.matches("type='104'/>").count());
    }

    #[test]
    fn hair_highlight_weight_lays_its_colour_over_the_rest() {
        // Calib-Hair-hl's settings: Highlight Weight 0.8, a blue highlight
        // (0.2, 0.4, 1 as entered), Glossy Layer Weight 0.25.
        let blue = [0.2f32, 0.4, 1.0].map(crate::daz_material::colour_to_linear);
        let hair = mat(json!([
            {"channel": {"id": "Hair Root Color", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Hair Tip Color", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Highlight Root Color", "current_value": [0.2, 0.4, 1.0]}},
            {"channel": {"id": "Tip Highlight Color", "current_value": [0.2, 0.4, 1.0]}},
            {"channel": {"id": "Highlight Weight", "current_value": 0.8}},
            {"channel": {"id": "Glossy Layer Weight", "current_value": 0.25}},
            {"channel": {"id": "separation", "current_value": 0.2}},
        ]));
        let hx = material_inputs(&[("Hair".into(), hair, 0.0)], &[], HashMap::new(), 4000, SkinShader::None).0;
        // Albedo: 0.2 × the white hair / transmission shares + 0.8 × 0.15 × blue.
        let a = [0, 1, 2].map(|k| 0.2 * (HAIR_COLOUR_SHARE + HAIR_TRANSMISSION_SHARE) + 0.8 * HAIR_HIGHLIGHT_SHARE * blue[k]);
        assert!(hx.contains(&format!(">{} {} {}<", num(a[0]), num(a[1]), num(a[2]))), "{hx}");
        // Specular: 0.2 × the glossy layer's weight + 0.8 × the highlight colour.
        let s = blue.map(|b| 0.2 * 0.25 + 0.8 * b);
        assert!(hx.contains(&format!(">{} {} {}<", num(s[0]), num(s[1]), num(s[2]))), "{hx}");
        // separation is left out (Octane's `offset` only dimmed the hair).
        assert!(!hx.contains("<pin name='offset'>"), "{hx}");
    }

    #[test]
    fn hair_root_to_tip_blend_follows_bias_and_gain() {
        let ocs = |extra: serde_json::Value| {
            let mut chs = vec![
                json!({"channel": {"id": "Hair Root Color", "current_value": [0, 0, 0]}}),
                json!({"channel": {"id": "Root Transmission Color", "current_value": [0, 0, 0]}}),
                json!({"channel": {"id": "Hair Tip Color", "current_value": [1, 1, 1]}}),
                json!({"channel": {"id": "Tip Transmission Color", "current_value": [1, 1, 1]}}),
                json!({"channel": {"id": "Highlight Weight", "current_value": 0.0}}),
            ];
            chs.extend(extra.as_array().unwrap().iter().cloned());
            material_inputs(&[("Hair".into(), mat(serde_json::Value::Array(chs)), 0.0)], &[], HashMap::new(), 4000, SkinShader::None).0
        };
        // Bias 0.2 / gain 0.7 bend the blend: control points along it,
        // each the root-to-tip mix DAZ's MDL gives there.
        let bent = ocs(json!([{"channel": {"id": "Root To Tip Bias", "current_value": 0.2}}, {"channel": {"id": "Root to Tip Gain", "current_value": 0.7}}]));
        assert!(bent.contains("<attr name='controlpoints' type='2'>15</attr>"), "{bent}");
        let at = |t: f32| crate::iray::root_to_tip_blend(t, 0.2, 0.7) * (HAIR_COLOUR_SHARE + HAIR_TRANSMISSION_SHARE);
        let p1 = bent.find("<pin name='Position 1' dynamicType='2'>").unwrap();
        let t1: f32 = bent[p1..].split("type='9'>").nth(1).unwrap().split(' ').next().unwrap().parse().unwrap();
        let v1 = &bent[bent.find("<pin name='Value 1' dynamicType='5'>").unwrap()..];
        assert!(v1.contains(&format!(">{} {} {}<", num(at(t1)), num(at(t1)), num(at(t1)))), "{v1}");
        // Equal ends: a plain colour.
        let flat = material_inputs(&[("Hair".into(), mat(json!([{"channel": {"id": "Hair Root Color", "current_value": [1, 1, 1]}}, {"channel": {"id": "Hair Tip Color", "current_value": [1, 1, 1]}}])), 0.0)], &[], HashMap::new(), 4000, SkinShader::None).0;
        assert!(!flat.contains("type='49'"), "{flat}");
        for x in [&bent, &flat] {
            assert_eq!(x.matches("<node ").count(), x.matches("</node>").count() + x.matches("type='104'/>").count());
        }
    }

    #[test]
    fn skin_volume_scales_the_subsurface_share() {
        // Victoria 8's volume is the reference: no change.
        let victoria = json!([
            {"channel": {"id": "Transmitted Color", "current_value": [0.9764706, 0.4823529, 0.3529412]}},
            {"channel": {"id": "Transmitted Measurement Distance", "current_value": 0.125}},
            {"channel": {"id": "Scattering Measurement Distance", "current_value": 0.015}},
            {"channel": {"id": "SSS Mode", "current_value": 1}},
            {"channel": {"id": "SSS Color", "current_value": [0.9764706, 0.6941177, 0.7607843]}},
            {"channel": {"id": "SSS Amount", "current_value": 2}},
        ]);
        let with = |extra: serde_json::Value| {
            let mut chs = victoria.as_array().unwrap().clone();
            chs.extend(extra.as_array().unwrap().iter().cloned());
            volume_factor(&mat(serde_json::Value::Array(chs)))
        };
        let near = |f: [f32; 3], want: [f32; 3]| f.iter().zip(want).all(|(a, b)| (a - b).abs() < 0.02);
        assert!(near(with(json!([])), [1.0; 3]));
        // Zurie's shorter distances, the same albedo.
        let zurie = with(json!([
            {"channel": {"id": "Transmitted Measurement Distance", "current_value": 0.063}},
            {"channel": {"id": "Scattering Measurement Distance", "current_value": 0.008}},
        ]));
        assert!(near(zurie, [1.0; 3]), "{zurie:?}");
        // A blue SSS Color scatters more red, less blue (Iray 1.69 / 0.92 / 0.52).
        let blue = with(json!([{"channel": {"id": "SSS Color", "current_value": [0.6, 0.75, 0.95]}}]));
        assert!(near(blue, [1.58, 0.93, 0.51]), "{blue:?}");
        // Mono: SSS Amount per distance (Iray 1.71 / 1.21 / 1.35).
        let mono = with(json!([{"channel": {"id": "SSS Mode", "current_value": 0}}]));
        assert!(near(mono, [1.65, 1.24, 1.40]), "{mono:?}");
    }

    /// An anisotropic Top Coat outside Layering Mode 1: a Layered material
    /// of the uncoated surface and a Specular Layer with the coat's
    /// anisotropy and rotation; isotropic, or weighted, it stays as it was.
    #[test]
    fn anisotropic_top_coat_is_a_layered_specular_layer() {
        let ball = |extra: serde_json::Value| {
            let mut chs = vec![
                json!({"channel": {"id": "diffuse", "current_value": [0.5, 0.5, 0.5]}}),
                json!({"channel": {"id": "Top Coat Weight", "current_value": 1.0}}),
                json!({"channel": {"id": "Top Coat Roughness", "current_value": 0.3}}),
            ];
            chs.extend(extra.as_array().unwrap().iter().cloned());
            let (x, _) = material_inputs(&[("Ball".into(), mat(json!(chs)), 1.0)], &[], HashMap::new(), 5000, SkinShader::None);
            x
        };
        let aniso = json!([
            {"channel": {"id": "Top Coat Anisotropy", "current_value": 0.5}},
            {"channel": {"id": "Top Coat Rotations", "current_value": 0.25}},
        ]);
        let x = ball(aniso.clone());
        assert!(x.contains("type='143'") && x.contains("<attr name='pin_count' type='2'>1</attr>"), "{x}");
        let layer = &x[x.find("<pin name='Layer 1' dynamicType='33'>").expect("layer pin")..];
        assert!(layer.contains("type='139'") && layer.contains("<pin name='anisotropyTexture'>") && layer.contains("<pin name='ior'>"), "{layer}");
        let rot = &layer[layer.find("<pin name='rotation'>").unwrap()..];
        assert!(rot.contains("type='6'>0.75<"), "{rot}");
        // The base keeps no coating of its own.
        assert!(!x.contains("<pin name='coating'>"), "{x}");
        // Isotropic: Universal's coating, no Layered material.
        let iso = ball(json!([]));
        assert!(iso.contains("<pin name='coating'>") && !iso.contains("type='143'"), "{iso}");
        // Weighted (mode 1): the Mix with its own anisotropic metal.
        let mut w = aniso.as_array().unwrap().clone();
        w.push(json!({"channel": {"id": "Top Coat Layering Mode", "current_value": 1}}));
        let weighted = ball(json!(w));
        assert!(!weighted.contains("type='143'") && weighted.contains("<pin name='anisotropyTexture'>"), "{weighted}");
    }

    /// NVIDIA Metal Brushed: Octane's roughness is the long axis (not the
    /// axes' geometric mean) on the GGX curve, plain GGX, no Fresnel (edge
    /// tint = albedo).
    #[test]
    fn mdl_metal_roughness_is_the_long_axis() {
        let mut textures = Textures::default();
        let mut w = Ocs { out: String::new(), next_id: 1, dirs: &[], textures: &mut textures, tiling: None, in_mix: false, in_coat_mix: false, in_layered_coat: false, in_flake_mix: false, in_backscatter_mix: false, thin_glass_ior_one: false, max_transmission: 1.0, film: None, pin_wraps: Vec::new(), gloss_ggx: false, uv_scale: None };
        let m = MdlMetal { colour: [0.65; 3], weight: (String::new(), 0.5, 0.5), rough_u: (String::new(), 0.02, 0.4), rough_v: (String::new(), 0.35, 0.325), bump: (String::new(), 0.0), tiles: [1.0, 1.0] };
        w.mdl_metal(0, "Metal", &m, &mat(json!([])));
        let x = w.out;
        // u 0.22, v 0.5125 at mid-grey, each on the GGX curve: roughness v,
        // anisotropy (v − u) / (v + u).
        let (u, v) = (ggx_roughness(0.22), ggx_roughness(0.5125));
        let rough = &x[x.find("<pin name='roughness'>").unwrap()..];
        assert!(rough.contains(&format!("type='6'>{}<", num(v))), "{x}");
        let a = &x[x.find("<pin name='anisotropyTexture'>").unwrap()..];
        let a: f32 = a.split("type='6'>").nth(1).unwrap().split('<').next().unwrap().parse().unwrap();
        assert!((a - (v - u) / (v + u)).abs() < 1e-5, "{a}");
        assert!(x.contains("<pin name='brdf'>") && x.contains("type='2'>2<"), "{x}");
        assert!(x.contains("<pin name='edgeTint'>"), "{x}");
    }

    /// NVIDIA's MDL examples get the channels straight as their MDL
    /// arguments (colours linear), which `mdl_metal` / `mdl_dielectric`
    /// read: against DAZ's .mi export of Calib-NVIDIA. Their definitions
    /// are DAZ's (`/data/DAZ 3D/NVIDIA MDL Examples`): only where installed.
    #[test]
    fn nvidia_examples_match_daz_mi_export() {
        use serde_json::Value;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dirs = crate::daz::content_dirs();
        let read = |p: &std::path::Path| -> Value { serde_json::from_str(&crate::dson::decompress_scene(&std::fs::read(p).unwrap()).unwrap()).unwrap() };
        let doc = read(&root.join("calib/scenes/Calib-NVIDIA.duf"));
        let mi: Value = serde_json::from_str(&std::fs::read_to_string(root.join("calib/references/mi/Calib-NVIDIA.json")).unwrap()).unwrap();
        let images = crate::daz_material::image_library(&doc);
        let mut label = HashMap::new();
        for n in doc.pointer("/scene/nodes").and_then(Value::as_array).into_iter().flatten() {
            for g in n["geometries"].as_array().into_iter().flatten() {
                label.insert(g["id"].as_str().unwrap_or("").to_string(), n["label"].as_str().unwrap_or("").to_string());
            }
        }
        let near = |a: f32, b: f64| (f64::from(a) - b).abs() < 1e-4;
        let (mut checked, mut wrong) = (0, Vec::new());
        for m in doc.pointer("/scene/materials").and_then(Value::as_array).into_iter().flatten() {
            let url = crate::dson::percent_decode(m["url"].as_str().unwrap_or(""));
            let Some((file, _)) = url.split_once('#').filter(|(f, _)| f.contains("NVIDIA MDL Examples")) else { continue };
            let Some(def) = dirs.iter().map(|d| d.join(file.trim_start_matches('/'))).find(|p| p.exists()) else {
                eprintln!("skipped: no {file}");
                return;
            };
            let mut mat = Material::from_dson(&read(&def)["material_library"][0], &images);
            mat.merge(m, &images);
            let node = &label[m["geometry"].as_str().unwrap().rsplit('#').next().unwrap()];
            let args = &mi["materials"][format!("{node}/Default")]["args"]["v"];
            let args = if args["args"]["base"].is_object() { &args["args"]["base"]["args"] } else { &args["args"] };
            let colour = |k: &str| args[k].as_array().map(|a| a.iter().map(|x| x.as_f64().or(x["args"]["v"].as_f64()).unwrap_or(f64::NAN)).collect::<Vec<_>>());
            let tex = |k: &str| args[k]["texture"]["path"].as_str().map(|p| p.rsplit('/').next().unwrap().to_string());
            let name = |p: &str| p.rsplit('/').next().unwrap().to_string();
            let mut check = |what: &str, ok: bool| {
                checked += 1;
                if !ok {
                    wrong.push(format!("{node}: {what}"));
                }
            };
            if let Some(metal) = mdl_metal(&mat) {
                let c = colour("metal_color").unwrap();
                check("metal colour", (0..3).all(|k| near(metal.colour[k], c[k])));
                check("tiling", near(metal.tiles[0], args["tiling"]["args"]["x"].as_f64().unwrap()));
                check("bump map", tex("bump_texture") == Some(name(&metal.bump.0)));
            } else if let Some(d) = mdl_dielectric(&mat) {
                match (&d.albedo, tex("diffuse_texture"), colour("base_color")) {
                    (MdlAlbedo::Map(m, _), Some(t), _) => check("diffuse map", name(m) == t),
                    (_, _, Some(b)) if d.transmission.is_some() => check("resin colour", d.transmission.is_some_and(|t| (0..3).all(|k| near(t[k] / RESIN_TRANSMISSION, b[k])))),
                    (MdlAlbedo::Colour(c), _, Some(b)) => check("base colour", (0..3).all(|k| near(c[k], b[k]))),
                    _ => {
                        // Architectural: diffuse, reflectivity × the reflection colour's peak.
                        let pass = |k: &str| args[k]["args"]["v"].clone();
                        let diffuse: Vec<f64> = pass("diffuse").as_array().unwrap().iter().filter_map(Value::as_f64).collect();
                        check("diffuse", matches!(d.albedo, MdlAlbedo::Colour(c) if (0..3).all(|k| near(c[k], diffuse[k]))));
                        check("reflectivity", matches!(d.gloss, MdlGloss::Weight(w) if near(w, pass("reflectivity").as_f64().unwrap())));
                    }
                }
            } else {
                check("recognised", false);
            }
        }
        wrong.sort();
        assert!(checked > 0 && wrong.is_empty(), "{} of {checked} NVIDIA example values differ from DAZ's .mi:\n{}", wrong.len(), wrong.join("\n"));
    }

    /// A glossiness map as Iray reads it (DAZ's .mi export, Calib-Mi-Maps):
    /// (1 − map × Glossiness)², baked per pixel through the GGX curve.
    #[test]
    fn glossiness_map_becomes_iray_roughness() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        image::GrayImage::from_pixel(2, 2, image::Luma([128])).save(dir.path().join("Runtime/Textures/grey.png")).unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let m = Rc::new(Material::from_dson(
            &json!({"extra": [{"channels": [
                {"channel": {"id": "Base Mixing", "current_value": 1}},
                {"channel": {"id": "Glossy Layered Weight", "current_value": 1.0}},
                {"channel": {"id": "Glossiness", "current_value": 0.6, "image_file": "/Runtime/Textures/grey.png"}},
            ]}]}),
            &HashMap::new(),
        ));
        let (xml, tex) = material_inputs(&[("Gloss".into(), m, 1.0)], &dirs, HashMap::new(), 7000, SkinShader::None);
        let (name, png) = tex.generated.iter().find(|(n, _)| n.ends_with("grey_derived.png")).expect("baked roughness");
        assert!(xml.contains(name.as_str()), "{xml}");
        let px = image::load_from_memory(png).unwrap().to_luma8().get_pixel(0, 0).0[0];
        let l = crate::daz_material::colour_to_linear(128.0 / 255.0);
        let want = ggx_roughness((1.0 - l * 0.6).powi(2));
        assert!((f32::from(px) / 255.0 - want).abs() < 1.5 / 255.0, "{px} vs {want}");
    }

    #[test]
    fn genesis_eyes_are_not_skin() {
        for n in ["Genesis_8_Female_Irises", "Body_Sclera", "Genesis_9_Eyes_Eye_Left", "Genesis_9_Eyes_Eye_Right"] {
            assert!(is_eye(n), "{n}");
        }
        assert!(!is_eye("Genesis_9_Face") && !is_eye("Genesis_9_Eyes_EyeMoisture_Left") && !is_eye("Genesis_9_Eyelashes_Eyelashes_Upper"));
    }
}
