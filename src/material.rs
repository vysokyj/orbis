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
        if let Some(c) = self.means.get(daz_path) {
            return *c;
        }
        let rel = daz_path.trim_start_matches('/');
        let mean = dirs.iter().map(|d| d.join(rel)).find(|p| p.is_file()).and_then(|f| {
            let img = image::open(&f).map_err(|e| tracing::warn!(file = %f.display(), error = %e, "Octane export: hair map unreadable")).ok()?;
            let small = img.thumbnail(64, 64).to_rgb8();
            let n = (small.width() * small.height()).max(1) as f32;
            let lin = |c: u8| {
                let c = c as f32 / 255.0;
                if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
            };
            let mut sum = [0.0f32; 3];
            for p in small.pixels() {
                (0..3).for_each(|k| sum[k] += lin(p[k]));
            }
            Some(sum.map(|s| s / n))
        });
        self.means.insert(daz_path.to_string(), mean);
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

/// Builds node XML with unique ids.
struct Ocs<'a> {
    out: String,
    next_id: u32,
    dirs: &'a [PathBuf],
    textures: &'a mut Textures,
    /// The current material's Iray UV tiling (tiles U, V, offset U, V),
    /// applied to every texture it uses.
    tiling: Option<[f32; 4]>,
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
        let watts = iray_emission_watts(mat, area_m2);
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
        match (&tint, blackbody) {
            (Some(asset), _) => self.image_pin(i + 1, "efficiency or texture", asset, false, 2.2, 1.0),
            (None, true) => self.float_pin(i + 1, "efficiency or texture", 1.0),
            (None, false) => self.rgb_pin(i + 1, "efficiency or texture", color),
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
    /// Only in `OctaneMode::Final`: it needs the path-tracing kernel —
    /// direct lighting renders random walk grey.
    fn standard_surface(&mut self, indent: usize, name: &str, mat: &Material, translucency: f32) {
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='277' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");
        self.float_pin(i, "base", 1.0);
        match mat.image("diffuse").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
            Some(asset) => self.tinted_image_pin(i, "baseColor", &asset, 1.0, mat.base_color()),
            None => self.rgb_pin(i, "baseColor", mat.base_color()),
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
            Some(asset) => self.image_pin(i, "subsurfaceColor", &asset, false, 2.2, 1.0),
            None => self.rgb_pin(i, "subsurfaceColor", mat.base_color()),
        }
        let (radius, scale) = sss_radius(mat);
        self.rgb_pin(i, "radius", radius);
        self.value_pin(i, "scale", 6, 9, &format!("{} 0 0 0", num(scale)));
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
    /// tip colour, each its map's mean × its tint (strands have no UVs),
    /// squared — Octane's albedo is the colour after many bounces through
    /// the hair volume, DAZ's tints each lobe: as-is, a brunette rendered
    /// blonde, 3–5× too light (Octane2); squared matched its Iray hair;
    /// longitudinal and azimuthal roughness from `base_roughness`; the
    /// specular tinted down to the glossy layer's weight.
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
        self.value_pin(i, "hairMode", 57, 2, "0"); // albedo
        self.rgb_pin(i, "albedo", [0, 1, 2].map(|k| ((root[k] + tip[k]) / 2.0).powi(2)));
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

    fn universal(&mut self, indent: usize, name: &str, mat: &Material, area_m2: f64, skin_sss: bool) {
        if mat.channel("Hair Root Color").is_some() {
            return self.hair(indent, name, mat);
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
        if skin_sss && translucency > 0.0 && !refractive && !emissive && !cutout && !thin && mat.image("diffuse").is_some() {
            return self.standard_surface(indent, name, mat, translucency);
        }
        let id = self.id();
        let _ = writeln!(self.out, "{:indent$}<node id='{id}' type='130' name='{}'>", "", esc(name));
        let i = indent + 1;
        let _ = writeln!(self.out, "{:i$}<attr name='compatibilityVersion' type='2'>{COMPAT}</attr>", "");

        // Iray refraction replaces the diffuse base: a fully refractive
        // surface (cornea, eye moisture, glass) has no diffuse left — a white
        // albedo there renders a milky shell over the eye.
        let refraction = mat.float("Refraction Weight").unwrap_or(0.0).clamp(0.0, 1.0);
        let diffuse_weight = 1.0 - refraction;

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
        let (tint, mix) = if mat.image("diffuse").is_some() { (mat.base_color(), self.weighted_mix(mat)) } else { ([1.0; 3], 1.0) };
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

        // Iray gives a surface a dielectric gloss only through its glossy
        // layer (or the dual-lobe specular of skin); at weight 0 it is matte.
        // Universal's default specular is a full white one — at roughness 0
        // over a normal map it made the matte slippers glitter (Octane1).
        let glossy = mat.float("Glossy Layered Weight").unwrap_or(0.0).max(mat.float("Glossy Weight").unwrap_or(0.0));
        let dual_lobe = mat.float("Dual Lobe Specular Weight").unwrap_or(0.0);
        if !mirror && refraction == 0.0 && glossy <= 0.0 && dual_lobe <= 0.0 && mat.image("Glossy Layered Weight").is_none() {
            self.rgb_pin(i, "specular", [0.0; 3]);
        } else if !mirror && glossy > 0.0 {
            // A coloured gloss map tints the reflection: the HS CB braids'
            // brown gloss rendered as a grey sheen with Universal's white
            // default (Octane1, probe 60).
            if let Some(asset) = mat.image("Glossy Color").map(str::to_string).and_then(|t| self.textures.asset(self.dirs, &t)) {
                self.tinted_image_pin(i, "specular", &asset, glossy.min(1.0), gloss_color);
            }
        }

        let metallic = if mirror { 1.0 } else if specular_mode { 0.0 } else { mat.metallic() };
        if mirror || (!specular_mode && (metallic > 0.0 || mat.image("Metallic Weight").is_some())) {
            let ch = if mirror { None } else { Some("Metallic Weight") };
            self.float_or_map(i, "metallic", mat, ch, metallic.max(0.0));
        }

        let (rough_ch, rough, inverted) = mat.roughness_source();
        // A glossiness map would need inverting; use its value only.
        let rough_ch = if inverted { None } else { rough_ch };
        self.float_or_map(i, "roughness", mat, rough_ch, rough.clamp(0.0, 1.0));

        // Glass-like surfaces (cornea, windows): Iray refraction → transmission.
        if refraction > 0.0 {
            self.float_pin(i, "transmission", refraction);
            let ior = mat.float("Refraction Index").unwrap_or(1.5);
            self.value_pin(i, "index4", 6, 8, &format!("{} 0 0", num(ior)));
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

/// The scalar a map is scaled by for a grey Iray colour tint: its mean
/// (white → 1).
fn map_tint(c: [f32; 3]) -> f32 {
    ((c[0] + c[1] + c[2]) / 3.0).clamp(0.0, 1.0)
}

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
}

/// Material inputs that make Octane's light emitters glow: a black diffuse
/// material with a blackbody (or colour) emission of the light's power.
pub fn light_inputs(lights: &[LightEmitter], first_id: u32) -> String {
    let mut textures = Textures::default();
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs: &[], textures: &mut textures, tiling: None };
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
        w.value_pin(8, "power", 6, 8, &format!("{} 0 0", num(l.watts as f32)));
        w.value_pin(8, "surfaceBrightness", 11, 1, "0");
        w.out.push_str("       </node>\n");
        w.pin_close(6);
        w.out.push_str("     </node>\n");
        w.pin_close(4);
        w.out.push_str("   </node>\n");
    }
    w.out
}

/// Spots came out ~1.33× brighter than the Blender reference on the pool they
/// light (Octane1 test scene, 2026-10-21); an empirical trim.
const SPOT_POWER_FACTOR: f64 = 0.75;

/// Octane's shadow catcher darkens a full shadow to black; Iray's ground
/// keeps the sky's fill — 11 % of the lit sand in Octane2's umbra (linear,
/// measured against its Iray render). One HDRI's sun/sky ratio, so a
/// calibration rather than physics.
const GROUND_SHADOW_OPACITY: f64 = 0.89;

/// How far (meters) the shadow-catching ground sits below y = 0: under a
/// room's floor, which then hides it like Iray's environment ground (at the
/// floor's height it z-fought and smeared shadows over it — Octane1).
const GROUND_DROP: f64 = 0.001;

/// A DAZ spotlight as an Octane analytic light.
#[derive(Debug, Clone, PartialEq)]
pub struct OctaneSpot {
    pub name: String,
    pub watts: f64,
    pub kelvin: Option<f64>,
    pub color: [f64; 3],
    /// Full cone angle, degrees.
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
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs: &[], textures: &mut textures, tiling: None };
    let mut ids = Vec::new();
    for (k, l) in spots.iter().enumerate() {
        let id = w.id();
        ids.push(id);
        let _ = writeln!(w.out, "    <node id='{id}' type='402' name='{}' position='{} -450'>", esc(&l.name), 300 + k * 200);
        w.value_pin(5, "analyticLightType", 57, 2, "1"); // DISK
        w.value_pin(5, "analyticLightSpreadAngle", 6, 9, &format!("{} 0 0 0", num(l.spread_deg as f32)));
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
        // DAZ/Blender spot power is that of a point source the cone cuts from;
        // Octane puts all of it into the cone — scale by the cone's share of
        // the sphere, (1 − cos(half angle)) / 2.
        let cone_share = (1.0 - (l.spread_deg / 2.0).to_radians().cos()) / 2.0;
        let watts = l.watts * cone_share * SPOT_POWER_FACTOR;
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
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs, textures, tiling: None };
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
    skin_sss: bool,
) -> (String, Textures) {
    let mut textures = Textures { layered, ..Default::default() };
    let mut w = Ocs { out: String::new(), next_id: first_id, dirs, textures: &mut textures, tiling: None };
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
        w.universal(5, name, mat, *area, skin_sss);
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
        let (cx, _) = material_inputs(&[("Cap".into(), cap, 1.0)], &dirs, HashMap::new(), 7000, false);
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
        let (sx, _) = material_inputs(&[("Torso".into(), sss.clone(), 1.0)], &dirs, HashMap::new(), 8000, true);
        assert!(sx.contains("type='277' name='Torso'"), "{sx}");
        assert!(sx.contains("<pin name='baseColor'>") && sx.contains("<pin name='subsurfaceColor'>"));
        // Colour maps decode as sRGB, data maps (the rough.png translucency
        // stand-in is a colour map here) stay linear only with gamma 1.
        let base = &sx[sx.find("<pin name='baseColor'>").unwrap()..];
        let base = &base[..base.find("</pin>\n      <pin").unwrap_or(base.len())];
        assert!(base.contains("<attr name='colorSpace' type='2'>1</attr>"), "{base}");
        // Octane keeps the pin's colour space only when told the image's was
        // chosen; every image node says so.
        assert_eq!(sx.matches("<attr name='initialColorSpaceSelected' type='1'>1</attr>").count(), sx.matches("<attr name='filename'").count());
        let (radius, scale) = sss_radius(&sss);
        // Transmitted Color (0.706, 0.557, 0.467) is sRGB: linear 0.456, 0.271, 0.184.
        assert!((scale - 0.0115).abs() < 0.0005, "1.15 cm red diffusion length: {scale}");
        assert!(radius[0] == 1.0 && radius[1] < radius[0] && radius[2] < radius[1], "{radius:?}");
        let lashes = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.5}},
            {"channel": {"id": "Cutout Opacity", "current_value": 1, "image_file": "/Runtime/Textures/rough.png"}},
        ]));
        let (lx, _) = material_inputs(&[("Lashes".into(), lashes, 1.0)], &dirs, HashMap::new(), 8500, true);
        assert!(lx.contains("type='130'"), "cut-out cards stay Universal");
        let shirt = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Translucency Weight", "current_value": 0.3}},
            {"channel": {"id": "Thin Walled", "current_value": true}},
        ]));
        let (tx, _) = material_inputs(&[("Shirt".into(), shirt, 1.0)], &dirs, HashMap::new(), 8700, true);
        assert!(tx.contains("type='130'"), "thin-walled fabric stays Universal");
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
        let (ex, _) = material_inputs(&[("Bulbs".into(), bulbs, 0.01), ("Glow".into(), glow, 2.0)], &dirs, HashMap::new(), 9000, false);
        assert!(ex.contains("<pin name='emission'>") && ex.contains("type='53'"), "blackbody bulbs");
        // 50 000 lm / 2 / 15 000 lm·W⁻¹ / 3.4.
        assert!(ex.contains("<attr name='value' type='8'>0.490196 0 0</attr>"), "50 000 lm → 0.49 W");
        assert!(ex.contains("<attr name='value' type='8'>7000 0 0</attr>"));
        // 100 cd/m² over 2 m²: 100 / 2 / 15 000 · 2 / 3.4 ≈ 0.002 W, red.
        assert!(ex.contains("type='54'") && ex.contains("<attr name='value' type='8'>0.001961 0 0</attr>"), "{ex}");
        let mirror = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0, 0, 0]}},
            {"channel": {"id": "Base Mixing", "current_value": 1}},
            {"channel": {"id": "Metallic Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Color", "current_value": [1, 1, 1]}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0}},
        ]));
        let wall = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/skin.jpg"}},
            {"channel": {"id": "Horizontal Tiles", "current_value": 6}},
            {"channel": {"id": "Vertical Tiles", "current_value": 4}},
        ]));
        let drawer = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0.49, 0.49, 0.49], "image_file": "/Runtime/Textures/skin.jpg"}},
        ]));
        let (dx, _) = material_inputs(&[("Drawer".into(), drawer, 1.0)], &dirs, HashMap::new(), 6000, false);
        // sRGB 0.49 grey = linear 0.205.
        assert!(dx.contains("<attr name='value' type='6'>0.204902</attr>") && !dx.contains("type='39'"), "map × grey tint: {dx}");
        let pubic = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [0.447, 0.294, 0.145], "image_file": "/Runtime/Textures/skin.jpg"}},
        ]));
        let (px, _) = material_inputs(&[("Pubic".into(), pubic, 1.0)], &dirs, HashMap::new(), 6100, false);
        let albedo = &px[px.find("<pin name='albedo'>").unwrap()..];
        assert!(albedo.contains("type='39'") && albedo.contains("<pin name='texture1'>"), "map × coloured tint: {albedo}");
        assert!(albedo.contains("<pin name='texture2'>") && albedo.contains("<attr name='value' type='8'>0.168222 0.070303 0.018478</attr>"), "{albedo}");
        // DAZ Default shader: "Opacity Strength" 0 hides the group.
        let hiding = mat(json!([
            {"channel": {"id": "diffuse", "current_value": [1, 1, 1]}},
            {"channel": {"id": "transparency", "current_value": 0}},
        ]));
        let (hx, _) = material_inputs(&[("Hiding".into(), hiding, 1.0)], &dirs, HashMap::new(), 6200, false);
        let op = &hx[hx.find("<pin name='opacity'>").expect("legacy opacity")..];
        assert!(op.contains("<attr name='value' type='6'>0</attr>"), "{op}");
        let (mx, _) = material_inputs(&[("Mirror".into(), mirror, 1.0), ("Wall".into(), wall, 1.0)], &dirs, HashMap::new(), 5000, false);
        assert!(mx.contains("<attr name='value' type='8'>1 1 1</attr>"), "mirror albedo = gloss colour");
        assert!(mx.contains("<pin name='metallic'>"));
        assert!(mx.contains("<attr name='scale' type='8'>0.166667 0.25 1</attr>"), "{mx}");
        // The wall has no glossy layer: matte (specular 0); the mirror keeps it.
        let wall_xml = &mx[mx.find("name='Wall'").unwrap()..];
        assert!(wall_xml.contains("<pin name='specular'>"), "{wall_xml}");
        assert_eq!(mx.matches("<pin name='specular'>").count(), 1, "only the wall");
        let (xml, tex) = material_inputs(&[("Body_Torso".into(), skin, 1.0), ("Body_Cornea".into(), cornea, 1.0)], &dirs, HashMap::new(), 1000, false);

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
        assert!(xml.contains("<pin name='transmission'>") && xml.contains("<attr name='value' type='8'>1.38 0 0</attr>"));
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
        let (x, _) = material_inputs(&[("Braids".into(), hair, 1.0)], &[dir.path().to_path_buf()], HashMap::new(), 3000, false);
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
        let (hx, tex) = material_inputs(&[("Curls".into(), hair, 0.0)], &[dir.path().to_path_buf()], HashMap::new(), 4000, false);
        assert!(hx.contains("type='147' name='Curls'"), "{hx}");
        // Root 0.5 × (1, 0.214, 0.214) (the colour linear) and tip 0.5,
        // averaged and squared.
        assert!(hx.contains("<attr name='value' type='8'>0.252914 0.093192 0.093192</attr>"), "{hx}");
        assert!(hx.contains("<pin name='specular'>") && hx.contains("<attr name='value' type='8'>0.22 0.22 0.22</attr>"));
        assert!(hx.contains("<attr name='value' type='6'>0.35</attr>"));
        assert!(tex.files.is_empty(), "the map is averaged, not packed");
        assert_eq!(hx.matches("<node ").count(), hx.matches("</node>").count());
    }
}
