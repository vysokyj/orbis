//! DAZ (DSON) materials — Iray Uber channels of a scene's material groups,
//! for the Octane export.
//!
//! A scene material (`scene.materials[]`) names a geometry instance, the
//! material groups it covers and a definition (`url` into a
//! `material_library`); its own channels override the definition's. Channels
//! sit both at the top level (`"diffuse": {"channel": …}`) and in
//! `extra[].channels[]`, keyed by Iray Uber labels (`"Glossy Roughness"`,
//! `"Cutout Opacity"`, …).

use std::collections::HashMap;

use serde_json::Value;

/// One material channel: its value and the texture driving it, if any.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Channel {
    pub value: Value,
    /// Content-relative image path (`/Runtime/Textures/…`), percent-decoded.
    pub image: Option<String>,
    /// The gamma DAZ hands Iray for the image (`resolve_gammas`): its
    /// `image_library` `map_gamma`, or the channel's `default_image_gamma`
    /// when the scene lists no entry for it; None is gamma "auto" (a
    /// `map_gamma` of 0, DAZ's automatic).
    pub gamma: Option<f32>,
    /// The channel definition's `default_image_gamma`, where it gives one.
    pub default_gamma: Option<f32>,
    /// DAZ's per-map texture-instance settings (`image_modification`), when
    /// the channel saves them.
    pub image_mod: Option<ImageMod>,
    /// Saved hidden (`"visible": false`). DAZ reads it: a hidden Base
    /// Color Effect leaves the SSS Reflectance Tint out of the diffuse
    /// (Calib-Mi-SSSTint, #52).
    pub hidden: bool,
}

/// DAZ's per-map settings of one channel's image (`image_modification`,
/// #77): what DAZ hands Iray as the `file_texture`'s uvw transform, colour
/// scale / offset and mono source (Calib-Mi-ImageMod, -ImageMod2):
/// - uvw = (uv · material tiles + (material offset u, −v)) · `tiles` +
///   (`tiling_offset` u, −v);
/// - the value = the channel's value × (`scale` · t + `offset`), t the
///   texel, or 1 − t when `invert`; a bump or normal map takes the tiles
///   only;
/// - `mono`: which of the texel's channels a grey channel reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageMod {
    pub tiles: [f32; 2],
    pub tiling_offset: [f32; 2],
    pub scale: f32,
    pub offset: f32,
    pub invert: bool,
    pub mono: Mono,
}

/// `grayscale_mode`: how a grey channel reads a colour texel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mono {
    /// DAZ's default (`mono_average`).
    Average,
    /// One channel (`redchannel` …): DAZ zeroes the other channels' scale
    /// and offset and passes `mono_maximum`.
    Red,
    Green,
    Blue,
    /// `mono_maximum`.
    Maximum,
    /// `mono_alpha` (1 where the image has no alpha).
    Alpha,
}

impl Default for ImageMod {
    fn default() -> Self {
        ImageMod { tiles: [1.0; 2], tiling_offset: [0.0; 2], scale: 1.0, offset: 0.0, invert: false, mono: Mono::Average }
    }
}

impl ImageMod {
    /// From a channel's `image_modification` object; missing fields at
    /// DAZ's defaults.
    pub fn from_dson(v: &Value) -> Self {
        let f = |k: &str, d: f32| v.get(k).and_then(Value::as_f64).map_or(d, |x| x as f32);
        let mono = match v.get("grayscale_mode").and_then(Value::as_str).unwrap_or("average") {
            "redchannel" => Mono::Red,
            "greenchannel" => Mono::Green,
            "bluechannel" => Mono::Blue,
            "maximum" => Mono::Maximum,
            "alpha" => Mono::Alpha,
            _ => Mono::Average,
        };
        ImageMod {
            tiles: [f("horizontal_tiles", 1.0), f("vertical_tiles", 1.0)],
            tiling_offset: [f("horizontal_tiling_offset", 0.0), f("vertical_tiling_offset", 0.0)],
            scale: f("scale", 1.0),
            offset: f("offset", 0.0),
            invert: v.get("invert").and_then(Value::as_bool).unwrap_or(false),
            mono,
        }
    }

    /// Only the UV placement (a bump or normal map: DAZ passes nothing
    /// else).
    pub fn placement(self) -> Self {
        ImageMod { tiles: self.tiles, tiling_offset: self.tiling_offset, ..ImageMod::default() }
    }

    /// The texel's value range: t = 0 maps to `lo`, t = 1 to `hi` (before
    /// the channel's value), None when it is 0 … 1.
    pub fn value_range(self) -> Option<(f32, f32)> {
        let (lo, hi) = if self.invert { (self.offset + self.scale, self.offset) } else { (self.offset, self.offset + self.scale) };
        (lo != 0.0 || hi != 1.0).then_some((lo, hi))
    }

    /// A texel value t (the picked channel, decoded) through scale,
    /// offset and invert.
    pub fn apply(self, t: f32) -> f32 {
        self.scale * if self.invert { 1.0 - t } else { t } + self.offset
    }

    /// The per-map tiles, as a usable divisor (DAZ's 0 counts as 1).
    pub fn tiles_or_one(self) -> [f32; 2] {
        self.tiles.map(|t| if t > 0.0 { t } else { 1.0 })
    }
}

/// A material group's merged channels.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Material {
    pub channels: HashMap<String, Channel>,
    /// On a prop, not a figure: no random-walk skin (grass blades and leaves
    /// are open cards).
    pub prop: bool,
    /// Its surface has an open edge (a card, a leaf, a cloak's hem), not a
    /// closed volume.
    pub open: bool,
    /// On strand hair (curves), not a mesh.
    pub strands: bool,
    /// A DAZ brick shader's MDL materials, as `module::name` (e.g.
    /// `nvidia::metal_examples::cast_iron`): what its graph renders with.
    pub mdl: Vec<String>,
    /// A DAZ brick (Shader Mixer) graph, whatever it names.
    pub brick: bool,
    /// One of DAZ Studio's scripted 3Delight shaders
    /// (`studio/material/daz_shader`: UberSurface, dzGlossy, …): its
    /// definition script, "" where only the scene material names the type.
    pub shader: Option<String>,
    /// The scene material's definition (`url`, percent-decoded): a library
    /// file's material (`/data/…/AoA_Subsurface.dsf#AoA_Subsurface`) or the
    /// scene's own (`#Default`).
    pub url: Option<String>,
    /// Its node's object axes X, Y, Z in world space (unit length), where
    /// the dump gives the node's transform: an Emission Profile lies in
    /// the mesh's object space in Iray, the export's meshes in world space.
    pub frame: Option<[[f64; 3]; 3]>,
    /// An Emission Profile's surface: its face normals in world space
    /// (unit, binned) with their share of its area — what Octane's
    /// Lambertian emitter keeps of the profile's flux.
    pub emit_normals: Option<Vec<([f64; 3], f64)>>,
}

impl Material {
    /// Channels of one DSON material object, top level and `extra` alike.
    /// `images` resolves `"image": "#id"` references into the scene's
    /// `image_library`.
    pub fn from_dson(m: &Value, images: &HashMap<String, String>) -> Self {
        let mut out = Material::default();
        out.merge(m, images);
        out
    }

    /// Overlay another DSON material object's channels onto these.
    pub fn merge(&mut self, m: &Value, images: &HashMap<String, String>) {
        let mut add = |c: &Value| {
            let Some(id) = c.get("id").and_then(Value::as_str) else { return };
            let value = c.get("current_value").or_else(|| c.get("value")).cloned().unwrap_or(Value::Null);
            let image = c
                .get("image_file")
                .and_then(Value::as_str)
                .map(crate::dson::percent_decode)
                .or_else(|| {
                    let r = c.get("image").and_then(Value::as_str)?;
                    images.get(&crate::dson::percent_decode(r.trim_start_matches('#'))).cloned()
                })
                .filter(|s| !s.is_empty());
            let image_mod = c.get("image_modification").filter(|v| v.is_object()).map(ImageMod::from_dson);
            // An override that doesn't say keeps the definition's visibility.
            let hidden = match c.get("visible").and_then(Value::as_bool) {
                Some(v) => !v,
                None => self.channels.get(id).is_some_and(|c| c.hidden),
            };
            // An override without one keeps the definition's.
            let default_gamma = c.get("default_image_gamma").and_then(Value::as_f64).map(|g| g as f32).or_else(|| self.channels.get(id).and_then(|c| c.default_gamma));
            self.channels.insert(id.to_string(), Channel { value, image, gamma: None, default_gamma, image_mod, hidden });
        };
        if let Some(obj) = m.as_object() {
            for v in obj.values() {
                if let Some(c) = v.get("channel") {
                    add(c);
                }
            }
        }
        if let Some(u) = m.get("url").and_then(Value::as_str) {
            self.url = Some(crate::dson::percent_decode(u));
        }
        let mut weighted_brick = false;
        for e in m.get("extra").and_then(Value::as_array).into_iter().flatten() {
            for c in e.get("channels").and_then(Value::as_array).into_iter().flatten() {
                if let Some(c) = c.get("channel") {
                    add(c);
                }
            }
            if e.get("type").and_then(Value::as_str) == Some("studio/material/daz_shader") {
                let definition = e.get("definition").and_then(Value::as_str).unwrap_or("");
                if self.shader.as_deref().is_none_or(str::is_empty) {
                    self.shader = Some(definition.to_string());
                }
            }
            if e.get("type").and_then(Value::as_str) == Some("studio/material/daz_brick") {
                self.brick = true;
                let mut found = Vec::new();
                mdl_callables(e, &mut found);
                for f in found {
                    if !self.mdl.contains(&f) {
                        self.mdl.push(f);
                    }
                }
                // A brick graph on Iray Uber's weighted base (OOT
                // Hairblending's hair) has no "Base Mixing" channel: the
                // brick is the mixing mode.
                weighted_brick |= json_contains(e, "DzMdlIrayUberWeighted");
            }
        }
        if weighted_brick && !self.channels.contains_key("Base Mixing") {
            self.channels.insert("Base Mixing".into(), Channel { value: Value::from(2), ..Channel::default() });
        }
    }

    /// Whether the shader renders with the MDL material `callable`.
    pub fn uses_mdl(&self, callable: &str) -> bool {
        self.mdl.iter().any(|m| m == callable)
    }

    /// A channel by its Iray Uber id — or the counterpart a custom (DAZ
    /// brick) shader names it by, which wins when present: HFGG's terrain
    /// keeps its tiled ground in "Diffuse Texture Map" (20 × 20 by "Map
    /// Horizontal / Vertical Tiles") and only a small preview in `diffuse`,
    /// which rendered the Gladiator's sandy path as a blurred stain.
    pub fn channel(&self, id: &str) -> Option<&Channel> {
        CHANNEL_ALIASES.iter().filter(|(uber, _)| *uber == id).find_map(|(_, alias)| self.channels.get(*alias)).or_else(|| self.channels.get(id))
    }

    /// Whether a feature switch turns this channel off: PBRSkin gates each
    /// layer with an "… Enable" channel, every one but Diffuse off by
    /// default — a weight set under a disabled layer does nothing in Iray
    /// (Calib-PBRSkin's matte sphere, its dual lobe off, rendered a full
    /// white gloss at the lobe's default weight 1).
    fn gated_off(&self, id: &str) -> bool {
        let gate = match id {
            "Metallic Weight" => "Metallicity Enable",
            "Translucency Weight" | "Translucency Color" => "Translucency Enable",
            "Dual Lobe Specular Weight" => "Dual Lobe Specular Enable",
            "Top Coat Weight" => "Top Coat Enable",
            "Bump Strength" | "Normal Map" => "Bump Enable",
            "Metallic Flakes Weight" => "Metallic Flakes Enable",
            "Detail Weight" => "Detail Enable",
            "Makeup Weight" => "Makeup Enable",
            "SSS Amount" => "Sub Surface Enable",
            _ => return false,
        };
        matches!(self.channel(gate).map(|c| &c.value), Some(Value::Bool(false)))
    }

    pub fn float(&self, id: &str) -> Option<f32> {
        if self.gated_off(id) {
            return Some(0.0);
        }
        match &self.channel(id)?.value {
            Value::Number(n) => n.as_f64().map(|f| f as f32),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            // A custom shader's colour-typed map strength (HFGG's "Roughness
            // Map" [1, 1, 1]) read as a scalar: its mean.
            Value::Array(a) if !a.is_empty() => Some(a.iter().filter_map(Value::as_f64).sum::<f64>() as f32 / a.len() as f32),
            _ => None,
        }
    }

    /// A colour channel, linear: DAZ keeps colours as entered and Iray
    /// decodes them (`colour_to_linear`) — passed on as-is, a 0.73 grey floor
    /// rendered 1.5× too bright in Octane (Calib-HDRI).
    pub fn color(&self, id: &str) -> Option<[f32; 3]> {
        let a = self.channel(id)?.value.as_array()?;
        Some([0, 1, 2].map(|i| colour_to_linear(a.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32)))
    }

    pub fn image(&self, id: &str) -> Option<&str> {
        if self.gated_off(id) {
            return None;
        }
        self.channel(id)?.image.as_deref()
    }

    /// DAZ's per-map settings of the channel's image (identity when it
    /// saves none).
    pub fn image_mod(&self, id: &str) -> ImageMod {
        self.channel(id).and_then(|c| c.image_mod).unwrap_or_default()
    }

    /// Base color (Iray `diffuse` / "Base Color") × Diffuse Strength, which
    /// DAZ Studio multiplies into the linear colour it passes to the MDL
    /// (its .mi export): ignored, Calib-Map-Effect's strength 0.5 sphere
    /// rendered 1.52 × Iray (#32).
    pub fn base_color(&self) -> [f32; 3] {
        // A map's factor too: DAZ folds the SSS Reflectance Tint into it
        // (Calib-Mi-SSSTint S8, the Genesis 8 skins).
        if let Some(u) = crate::iray::Uber::of(self) {
            if let Some(c) = u.color("diffuse_color").or_else(|| u.map("diffuse_color").map(|m| m.value)) {
                return c;
            }
        }
        let strength = self.float("Diffuse Strength").unwrap_or(1.0).max(0.0);
        self.color("diffuse").unwrap_or([0.8; 3]).map(|c| c * strength)
    }

    /// Specular roughness, 0 = mirror … 1 = matte. Iray Uber keeps it in
    /// "Glossy Roughness" (metallicity/roughness mode), "Glossiness"
    /// (specular/glossiness mode) or, for dual-lobe skin, "Specular Lobe 1
    /// Roughness".
    pub fn roughness(&self) -> f32 {
        self.roughness_source().1
    }

    /// The channel roughness comes from (its texture drives it too) and the
    /// roughness value; `inverted` when the channel is a glossiness.
    pub fn roughness_source(&self) -> (Option<&'static str>, f32, bool) {
        if self.float("Dual Lobe Specular Weight").unwrap_or(0.0) > 0.0 {
            if let Some(r) = self.float("Specular Lobe 1 Roughness") {
                return (Some("Specular Lobe 1 Roughness"), r, false);
            }
        }
        self.glossy_roughness_source()
    }

    /// [`roughness_source`] of the glossy layer itself, the dual lobe's
    /// left aside: Iray has both in its stack at once (#106).
    pub fn glossy_roughness_source(&self) -> (Option<&'static str>, f32, bool) {
        // Base Mixing 1 (specular / glossiness) takes its gloss from
        // Glossiness, 0 (metallicity / roughness) and 2 (weighted) from
        // Glossy Roughness — both channels are always saved (DAZ's .mi
        // export: Calib-Mixing's weighted spheres at Glossy Roughness 0.5,
        // Glossiness 1 get 0.25; #62). A brick graph on Uber's weighted
        // base keeps reading Glossiness: what DAZ makes of a graph isn't
        // verified (HFGG's terrain).
        let glossiness_mode = match self.float("Base Mixing").unwrap_or(0.0) as i32 {
            1 => true,
            2 => self.brick,
            _ => false,
        };
        if !glossiness_mode {
            if let Some(r) = self.float("Glossy Roughness") {
                return (Some("Glossy Roughness"), r, false);
            }
        }
        if let Some(g) = self.float("Glossiness") {
            return (Some("Glossiness"), 1.0 - g, true);
        }
        if let Some(r) = self.float("Glossy Roughness") {
            return (Some("Glossy Roughness"), r, false);
        }
        (None, 0.5, false)
    }

    pub fn metallic(&self) -> f32 {
        self.float("Metallic Weight").unwrap_or(0.0)
    }

    /// The channel opacity lives in: Iray Uber's "Cutout Opacity", else the
    /// DAZ Default shader's `transparency` ("Opacity Strength") — 0 there
    /// hides a group (the NGV8 pubic hair's white "Hiding" cards, Octane3).
    pub fn opacity_channel(&self) -> &'static str {
        if self.channel("Cutout Opacity").is_none() && self.channel("transparency").is_some() { "transparency" } else { "Cutout Opacity" }
    }

    /// Cutout opacity, 1 = opaque (geometry shells are often 0 outside a mask).
    pub fn opacity(&self) -> f32 {
        self.float(self.opacity_channel()).unwrap_or(1.0)
    }
}

/// A DAZ colour → linear as Iray reads it: a pure gamma 2.2, not the sRGB
/// curve. Calib-Glossy's red (0.45, 0.05, 0.04): decoded as sRGB, Octane
/// rendered the green 2.77 × Iray (sRGB's linear toe: 0.0039 against
/// 0.05^2.2 = 0.0014), with gamma 2.2 0.99 ×; its matte sphere 1.13 → 1.08.
pub fn colour_to_linear(c: f32) -> f32 {
    c.max(0.0).powf(2.2)
}

/// Whether any string in `v` is `needle`.
fn json_contains(v: &Value, needle: &str) -> bool {
    match v {
        Value::String(s) => s == needle,
        Value::Array(a) => a.iter().any(|x| json_contains(x, needle)),
        Value::Object(o) => o.values().any(|x| json_contains(x, needle)),
        _ => false,
    }
}

/// The MDL materials a brick graph names: an "MDL Callable" property
/// (`nvidia::metal_examples::cast_iron`) or a module path with a fragment
/// (`nvidia/metal_examples.mdl#metal_brushed`), as `module::name`.
fn mdl_callables(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => {
            let s = s.split('(').next().unwrap_or(s);
            let callable = if let Some((path, name)) = s.split_once(".mdl#") {
                Some(format!("{}::{name}", path.trim_start_matches('/').replace('/', "::")))
            } else if s.contains("::") && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
                Some(s.trim_start_matches("::").to_string())
            } else {
                None
            };
            out.extend(callable);
        }
        Value::Array(a) => a.iter().for_each(|x| mdl_callables(x, out)),
        Value::Object(o) => o.values().for_each(|x| mdl_callables(x, out)),
        _ => {}
    }
}

/// Iray Uber channel → the channel a custom shader keeps the same input in.
const CHANNEL_ALIASES: &[(&str, &str)] = &[
    ("diffuse", "Diffuse Texture Map"),
    ("Horizontal Tiles", "Map Horizontal Tiles"),
    ("Vertical Tiles", "Map Vertical Tiles"),
    ("Bump Strength", "Bump Map"),
    ("Glossy Roughness", "Roughness Map"),
];

/// `image_library` id → the image a channel uses: the file of a single-map
/// entry, or `lie:<id>` for a layered stack (see `crate::layered_image`) that
/// the exporter has to flatten.
pub fn image_library(doc: &Value) -> HashMap<String, String> {
    doc.get("image_library")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|i| {
            let id = crate::dson::percent_decode(i.get("id")?.as_str()?);
            if crate::layered_image::LayeredImage::from_dson(i).is_stack() {
                return Some((id.clone(), format!("{LAYERED_PREFIX}{id}")));
            }
            let url = i.get("map")?.as_array()?.first()?.get("url")?.as_str()?;
            Some((id, crate::dson::percent_decode(url)))
        })
        .collect()
}

/// The `map_gamma` `image_library` gives each image file it lists (by its
/// path; 0 is DAZ's automatic).
pub fn image_gammas(doc: &Value) -> HashMap<String, f32> {
    doc.get("image_library")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|i| {
            let g = i.get("map_gamma").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let url = i.get("map")?.as_array()?.first()?.get("url")?.as_str()?;
            Some((crate::dson::percent_decode(url), g))
        })
        .collect()
}

/// `image_gammas` with the files the scene's `image_library` doesn't list
/// (Calib-Mi-ImageGamma, #91): DAZ keeps one image per file, and an
/// unlisted one takes the `default_image_gamma` of the first channel that
/// loads it — in the scene's material order, a material's own channels
/// (top level, then `extra`) before its definition's. Iray Uber's defaults
/// are 1 for bump, normal and the float channels, 0 ("auto") for the
/// colours: a file alone on Bump Strength went to Iray at gamma 1, the
/// same file first on the diffuse then on a bump "auto" on both, the other
/// way round 1 on both. The Calib-Skin spheres' cropped maps, saved
/// without entries, went at 1; Calib-Mi-AoA's calib-tiles.png, on the
/// diffuse first, "auto" on its Normal Map too. `def` gives a scene
/// material's definition (its `url`).
pub fn scene_image_gammas(doc: &Value, images: &HashMap<String, String>, mut def: impl FnMut(&str) -> Option<Value>) -> HashMap<String, f32> {
    let mut gammas = image_gammas(doc);
    fn ids(m: &Value) -> Vec<String> {
        let top = m.as_object().into_iter().flat_map(|o| o.values()).filter_map(|v| v.get("channel")?.get("id")?.as_str());
        let extra = m.get("extra").and_then(Value::as_array).into_iter().flatten().filter_map(|e| e.get("channels")?.as_array()).flatten().filter_map(|c| c.get("channel")?.get("id")?.as_str());
        top.chain(extra).map(str::to_string).collect()
    }
    for m in doc.pointer("/scene/materials").and_then(Value::as_array).into_iter().flatten() {
        let d = m.get("url").and_then(Value::as_str).and_then(&mut def);
        let mut mat = d.as_ref().map_or_else(Material::default, |d| Material::from_dson(d, images));
        mat.merge(m, images);
        for id in ids(m).into_iter().chain(d.as_ref().map(ids).unwrap_or_default()) {
            let Some(c) = mat.channels.get(&id) else { continue };
            if let Some(i) = &c.image {
                gammas.entry(i.clone()).or_insert(c.default_gamma.unwrap_or(0.0));
            }
        }
    }
    gammas
}

/// The gamma Iray's image loader gives a canvas under gamma "auto" (MDL
/// `gamma_default`, an image with `map_gamma` 0) by its pixel type,
/// ignoring the file's colour metadata (#90, #93). The rule of the MDL SDK's
/// OpenImageIO plugin (`get_pixel_type_and_gamma`, 2025.0.3 as in DAZ
/// Studio 6's Iray, the same in 2026.0.2):
/// - 8 bits a channel, any channel count: 2.2;
/// - 16 bits: 1.0 for a single channel (debug option
///   `linear_gamma_for_single_uint16_channel`, default on), 2.2 with
///   colour or alpha (grey + alpha, RGB, RGBA);
/// - half or float: 1.0.
///
/// Calib-BumpGamma's 16-bit greyscale height map rendered linear (#91).
/// Every map's "auto" resolves here — colour, data, bump and normal maps
/// and the bakes that read them.
pub fn auto_gamma(color: image::ColorType) -> f32 {
    use image::ColorType::*;
    match color {
        L16 | Rgb32F | Rgba32F => 1.0,
        _ => 2.2,
    }
}

impl Material {
    /// Each channel image's gamma as DAZ hands it Iray, from
    /// `scene_image_gammas`: None for "auto" (0) or an image it doesn't know.
    pub fn resolve_gammas(&mut self, gammas: &HashMap<String, f32>) {
        for c in self.channels.values_mut() {
            c.gamma = c.image.as_ref().and_then(|i| gammas.get(i).copied()).filter(|g| *g > 0.0);
        }
    }
}

impl Material {
    /// The gamma of a channel's image, when its image sets one.
    pub fn image_gamma(&self, id: &str) -> Option<f32> {
        self.channel(id)?.gamma
    }
}

/// Marks a channel image that is a layered stack, not a file.
pub const LAYERED_PREFIX: &str = "lie:";

/// The scene's layered images by `image_library` id.
pub fn layered_images(doc: &Value) -> HashMap<String, crate::layered_image::LayeredImage> {
    doc.get("image_library")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|i| {
            let id = crate::dson::percent_decode(i.get("id")?.as_str()?);
            let img = crate::layered_image::LayeredImage::from_dson(i);
            img.is_stack().then_some((id, img))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_weighted_uber_brick_means_weighted_base_mixing() {
        let def = serde_json::json!({"extra": [{"type": "studio/material/daz_brick", "brick_settings": {"Bricks": {"Brick_3": {"DAZStudioClassName": "DzMdlIrayUberWeighted"}}}}]});
        let m = Material::from_dson(&def, &HashMap::new());
        assert_eq!(m.float("Base Mixing"), Some(2.0));
        let plain = Material::from_dson(&serde_json::json!({"extra": [{"type": "studio/material/daz_brick"}]}), &HashMap::new());
        assert_eq!(plain.float("Base Mixing"), None);
    }

    use super::*;
    use serde_json::json;

    #[test]
    fn custom_shader_channels_stand_in_for_uber_ones() {
        // A DAZ brick terrain shader: its tiled map wins over the preview
        // in `diffuse`, its tiles and roughness map count as Uber's.
        let def = json!({
            "id": "Terrain",
            "diffuse": {"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/preview.png"}},
            "extra": [{"type": "studio_material_channels", "channels": [
                {"channel": {"id": "Diffuse Texture Map", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/sand.png"}},
                {"channel": {"id": "Map Horizontal Tiles", "current_value": 20}},
                {"channel": {"id": "Roughness Map", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/rough.png"}}
            ]}],
        });
        let m = Material::from_dson(&def, &HashMap::new());
        assert_eq!(m.image("diffuse"), Some("/Runtime/Textures/sand.png"));
        assert_eq!(m.float("Horizontal Tiles"), Some(20.0));
        assert_eq!((m.image("Glossy Roughness"), m.float("Glossy Roughness")), (Some("/Runtime/Textures/rough.png"), Some(1.0)));
    }

    #[test]
    fn merges_definition_and_scene_overrides() {
        let def = json!({
            "diffuse": {"channel": {"id": "diffuse", "value": [0.75, 0.75, 0.75], "current_value": [1, 1, 1]}},
            "extra": [{"type": "studio_material_channels", "channels": [
                {"channel": {"id": "Glossy Roughness", "current_value": 0.4}},
                {"channel": {"id": "Cutout Opacity", "current_value": 1, "image": "#mask"}},
            ]}],
        });
        let scene = json!({
            "diffuse": {"channel": {"id": "diffuse", "current_value": [1, 0.5, 0.25], "image_file": "/Runtime/Textures/A%20B/skin.jpg"}},
            "extra": [{"type": "studio_material_channels", "channels": [{"channel": {"id": "Metallic Weight", "current_value": 1}}]}],
        });
        let images = image_library(&json!({"image_library": [{"id": "mask", "map": [{"url": "/Runtime/Textures/m.png"}]}]}));
        let mut m = Material::from_dson(&def, &images);
        m.merge(&scene, &images);
        // Linear: DAZ's colours at gamma 2.2.
        let c = m.base_color();
        assert!((c[0] - 1.0).abs() < 1e-6 && (c[1] - 0.217638).abs() < 1e-5 && (c[2] - 0.047366).abs() < 1e-5, "{c:?}");
        assert_eq!(m.image("diffuse"), Some("/Runtime/Textures/A B/skin.jpg"));
        assert_eq!(m.image("Cutout Opacity"), Some("/Runtime/Textures/m.png"));
        assert_eq!((m.roughness(), m.metallic(), m.opacity()), (0.4, 1.0, 1.0));
        // Specular / glossiness mixing takes Glossiness, whatever Glossy
        // Roughness says.
        let gloss = json!({"extra": [{"type": "studio_material_channels", "channels": [
            {"channel": {"id": "Base Mixing", "current_value": 1}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0.5}},
            {"channel": {"id": "Glossiness", "current_value": 0.8}},
        ]}]});
        let g = Material::from_dson(&gloss, &images);
        assert!((g.roughness() - 0.2).abs() < 1e-6, "{}", g.roughness());
    }

    #[test]
    fn image_modification_per_channel() {
        let m = Material::from_dson(&json!({"extra": [{"channels": [
            {"channel": {"id": "Glossy Roughness", "current_value": 0.7, "image_file": "/r.png",
                "image_modification": {"grayscale_mode": "greenchannel", "scale": 1.5, "offset": -0.1, "invert": true, "horizontal_tiles": 2, "horizontal_tiling_offset": 0, "vertical_tiles": 1, "vertical_tiling_offset": 0.25}}},
            {"channel": {"id": "Glossy Weight", "current_value": 1, "image_file": "/w.png"}},
        ]}]}), &HashMap::new());
        let r = m.image_mod("Glossy Roughness");
        assert_eq!((r.tiles, r.tiling_offset, r.mono, r.invert), ([2.0, 1.0], [0.0, 0.25], Mono::Green, true));
        // DAZ's .mi: 0.7 × (1.5 (1 − t) − 0.1) = 0.98 − 1.05 t.
        assert!((r.apply(0.0) * 0.7 - 0.98).abs() < 1e-6 && (r.apply(1.0) * 0.7 + 0.07).abs() < 1e-6);
        assert_eq!(r.value_range(), Some((1.4, -0.1)));
        assert_eq!(r.placement(), ImageMod { tiles: [2.0, 1.0], tiling_offset: [0.0, 0.25], ..ImageMod::default() });
        // None saved: the identity, no value range.
        assert_eq!(m.image_mod("Glossy Weight"), ImageMod::default());
        assert_eq!(ImageMod::default().value_range(), None);
    }

    #[test]
    fn roughness_by_shading_mode() {
        let m = |chs: Value| Material::from_dson(&json!({"extra": [{"channels": chs}]}), &HashMap::new());
        let skin = m(json!([
            {"channel": {"id": "Dual Lobe Specular Weight", "current_value": 0.25}},
            {"channel": {"id": "Specular Lobe 1 Roughness", "current_value": 0.7}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0.1}},
        ]));
        assert_eq!(skin.roughness(), 0.7);
        let gloss = m(json!([{"channel": {"id": "Glossiness", "current_value": 0.75}}]));
        assert_eq!(gloss.roughness(), 0.25);
        assert_eq!(m(json!([])).roughness(), 0.5);
    }
}
