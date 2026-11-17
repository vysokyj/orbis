//! DAZ materials → `UsdPreviewSurface` with their maps, for the `.usdz`
//! viewing export (#199): what Quick Look, Reality Composer or Blender show.
//!
//! Not an Octane path and not calibrated: `UsdPreviewSurface` has one
//! diffuse, one specular lobe, a normal, an opacity and an emission, so
//! subsurface, layering, top coats and hair shading are left out. The maps
//! come from the same `Textures` the Octane export packs (flattened layered
//! images, the OOT Cap's `cap_bake`, the bump and normal maps baked into one
//! normal map by `detail_normal`), the values from what DAZ hands Iray
//! where `crate::iray` has them.

use super::*;

/// A map on a `UsdPreviewSurface` input: a `UsdUVTexture` reading `asset`
/// (its package path), `output` (`rgb`, `r`, `g`, `b`, `a`), decoded as
/// sRGB or raw, its value × `scale` + `bias` (USD's own order: after the
/// decode); `placement` = (scale, translation) on `st` when not identity.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewMap {
    pub asset: String,
    pub output: &'static str,
    pub srgb: bool,
    pub scale: [f32; 4],
    pub bias: [f32; 4],
    pub placement: Option<([f32; 2], [f32; 2])>,
}

/// One material's `UsdPreviewSurface` inputs: each a constant, or a map
/// (which then carries the constant in its `scale`).
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewMaterial {
    pub diffuse: [f32; 3],
    pub diffuse_map: Option<PreviewMap>,
    pub roughness: f32,
    pub roughness_map: Option<PreviewMap>,
    pub metallic: f32,
    pub metallic_map: Option<PreviewMap>,
    pub opacity: f32,
    pub opacity_map: Option<PreviewMap>,
    /// `opacityThreshold`: > 0 renders the opacity as a cutout.
    pub opacity_threshold: f32,
    pub normal_map: Option<PreviewMap>,
    pub emissive: [f32; 3],
    pub emissive_map: Option<PreviewMap>,
    pub occlusion_map: Option<PreviewMap>,
}

impl PreviewMaterial {
    /// Every map it reads.
    pub fn maps(&self) -> impl Iterator<Item = &PreviewMap> {
        [&self.diffuse_map, &self.roughness_map, &self.metallic_map, &self.opacity_map, &self.normal_map, &self.emissive_map, &self.occlusion_map].into_iter().flatten()
    }

    fn maps_mut(&mut self) -> impl Iterator<Item = &mut Option<PreviewMap>> {
        [&mut self.diffuse_map, &mut self.roughness_map, &mut self.metallic_map, &mut self.opacity_map, &mut self.normal_map, &mut self.emissive_map, &mut self.occlusion_map].into_iter()
    }

    /// Point every map at its asset's new name (`rename`); a map whose
    /// asset maps to None is dropped (it can't go into the package).
    pub fn rename_assets(&mut self, rename: impl Fn(&str) -> Option<String>) {
        for slot in self.maps_mut() {
            if let Some(m) = slot.as_mut() {
                match rename(&m.asset) {
                    Some(a) => m.asset = a,
                    None => *slot = None,
                }
            }
        }
    }
}

/// The preview surface of each bound material (in `materials`' order), and
/// the textures they read (`Textures::files` on disk, `generated` made by
/// the export), package paths `assets/tex/…` as the `.orbx` names them.
/// `strands[i]`: material i is bound to strand hair (curves, no UVs), whose
/// colour is its map's mean, as the Octane hair takes it.
pub fn preview_materials(materials: &[crate::usd::BoundMaterial], strands: &[bool], dirs: &[PathBuf], layered: HashMap<String, crate::layered_image::LayeredImage>) -> (Vec<PreviewMaterial>, Textures) {
    let own_gammas = materials.iter().flat_map(|(_, m, _)| m.channels.values()).filter_map(|c| Some((c.image.clone()?, c.gamma?))).collect();
    let mut textures = Textures { layered, own_gammas, ..Default::default() };
    let out = materials.iter().enumerate().map(|(i, (_, mat, _))| preview(&mut textures, dirs, mat, strands.get(i).copied().unwrap_or(false))).collect();
    (out, textures)
}

fn preview(t: &mut Textures, dirs: &[PathBuf], mat: &Material, strand: bool) -> PreviewMaterial {
    let tiles = [mat.float("Horizontal Tiles"), mat.float("Vertical Tiles")].map(|x| x.filter(|x| *x > 0.0).unwrap_or(1.0));
    let offsets = [mat.float("Horizontal Offset"), mat.float("Vertical Offset")].map(|o| o.unwrap_or(0.0));
    // Iray's uvw transform (`uv_transform`): uv · (material tiles · map
    // tiles) + (material offset u · map tiles u + map offset u, −(… v)).
    let place = |m: ImageMod| {
        let pt = m.tiles_or_one();
        let scale = [tiles[0] * pt[0], tiles[1] * pt[1]];
        let shift = [offsets[0] * pt[0] + m.tiling_offset[0], -(offsets[1] * pt[1] + m.tiling_offset[1])];
        (scale != [1.0, 1.0] || shift != [0.0, 0.0]).then_some((scale, shift))
    };
    let srgb = |t: &Textures, asset: &str, own: Option<f32>| t.map_gamma(asset, own) > 1.5;
    // A grey channel's map: the texel t (the picked channel) through the
    // map's value range and × `value`, as scale and bias.
    let grey = |t: &mut Textures, ch: &str, value: f32, inverted: bool| -> Option<PreviewMap> {
        let src = mat.image(ch)?.to_string();
        let asset = t.asset(dirs, &src)?;
        let m = mat.image_mod(ch);
        let (lo, hi) = m.value_range().unwrap_or((0.0, 1.0));
        let (mut s, mut b) = ((hi - lo) * value, lo * value);
        if inverted {
            (s, b) = (-s, 1.0 - b);
        }
        let output = match m.mono {
            Mono::Green => "g",
            Mono::Blue => "b",
            Mono::Alpha => "a",
            Mono::Average | Mono::Red | Mono::Maximum => "r",
        };
        Some(PreviewMap { srgb: srgb(t, &asset, mat.image_gamma(ch)), asset, output, scale: [s, s, s, 1.0], bias: [b, b, b, 0.0], placement: place(m) })
    };
    let colour_map = |asset: String, c: [f32; 3], placement| PreviewMap { asset, output: "rgb", srgb: true, scale: [c[0], c[1], c[2], 1.0], bias: [0.0; 4], placement };

    // Diffuse: the map × the base colour Iray multiplies it by; the OOT
    // Cap's blend baked with the colour in it (`cap_bake`, #141).
    let base = mat.base_color();
    let diffuse_src = mat.image("diffuse").map(str::to_string);
    let mut diffuse = base;
    let diffuse_map = if strand {
        if let Some(mean) = diffuse_src.as_deref().and_then(|d| t.mean_colour(dirs, d)) {
            diffuse = [0, 1, 2].map(|k| base[k] * mean[k]);
        }
        None
    } else if let Some(cap) = cap_blend(mat).and_then(|c| t.cap_bake(dirs, diffuse_src.as_deref(), base, &c)) {
        Some(colour_map(cap, [1.0; 3], place(mat.image_mod("diffuse").placement())))
    } else {
        diffuse_src.as_deref().and_then(|d| t.asset(dirs, d)).map(|a| {
            let gamma = t.map_gamma(&a, mat.image_gamma("diffuse"));
            PreviewMap { srgb: gamma > 1.5, ..colour_map(a, base, place(mat.image_mod("diffuse").placement())) }
        })
    };

    // Roughness: UsdPreviewSurface squares its roughness into the GGX
    // alpha, Iray takes the slider squared (`Roughness Squared` /
    // `Glossiness Squared`, on by default, `iray_roughness`): the slider
    // goes in as it is, 1 − glossiness from a glossiness channel. With the
    // squaring off, the constant's square root; a map then still goes in
    // linear (a look, not a match: such maps are rare).
    let (ch, slider, inverted) = mat.roughness_source();
    let iray_r = iray_roughness_of(mat, ch).unwrap_or_else(|| iray_roughness(mat, slider, inverted));
    // Iray Uber reflects only through its glossy layer, dual lobe, top
    // coat, metal or refraction; UsdPreviewSurface always has its 4 %
    // dielectric specular, at roughness 0 a mirror sheen on a matte
    // surface: fully rough there.
    let refraction = refraction_weight(mat).clamp(0.0, 1.0);
    let glossy = [gloss_weight(mat), mat.float("Dual Lobe Specular Weight").unwrap_or(0.0), mat.float("Top Coat Weight").unwrap_or(0.0), metallic(mat), refraction].iter().any(|w| *w > 0.0)
        || mat.image(gloss_weight_channel(mat)).is_some();
    let roughness = if glossy { iray_r.max(0.0).sqrt().clamp(0.0, 1.0) } else { 1.0 };
    let roughness_map = if strand || !glossy {
        None
    } else {
        ch.and_then(|ch| {
            let value = mat.float(ch).unwrap_or(if inverted { 1.0 - slider } else { slider });
            grey(t, ch, value, inverted)
        })
    };

    let metallic_value = metallic(mat);
    let metallic_map = if strand || !metallic_map(mat) { None } else { grey(t, "Metallic Weight", mat.metallic(), false) };

    // Opacity: a cutout map as a mask at half its value — DAZ's transmapped
    // hair cards and lashes would otherwise need sorting the viewers don't
    // do (blended, a card in front hid the ones behind). A judgment call for
    // viewing (#199); a constant opacity stays blended.
    let opacity_ch = mat.opacity_channel();
    // A refraction (glass, the eyes' cornea and moisture) shows what lies
    // behind it: the preview has no transmission, so it is let through as
    // opacity — opaque, it hid the iris (a look for viewing, #199).
    let opacity = (mat.opacity() * (1.0 - refraction)).clamp(0.0, 1.0);
    let opacity_map = if strand { None } else { grey(t, opacity_ch, opacity, false) };
    let opacity_threshold = if opacity_map.is_some() { 0.5 * opacity } else { 0.0 };

    let normal_map = if strand { None } else { normal(t, dirs, mat).map(|(asset, m)| PreviewMap { asset, output: "rgb", srgb: false, scale: [2.0, 2.0, 2.0, 1.0], bias: [-1.0, -1.0, -1.0, 0.0], placement: place(m) }) };

    // Emission: its colour (and map), no luminance: a viewer shows it unlit.
    let emission = mat.color("Emission Color").unwrap_or([0.0; 3]);
    let emits = emission.iter().any(|c| *c > 0.0);
    let emissive_map = (emits && !strand).then(|| mat.image("Emission Color").map(str::to_string)).flatten().and_then(|e| t.asset(dirs, &e)).map(|a| colour_map(a, emission, place(mat.image_mod("Emission Color").placement())));

    let occlusion_map = if strand { None } else { grey(t, "AO Texture Map", 1.0, false) };

    PreviewMaterial {
        diffuse,
        diffuse_map,
        roughness,
        roughness_map,
        metallic: metallic_value.clamp(0.0, 1.0),
        metallic_map,
        opacity,
        opacity_map,
        opacity_threshold,
        normal_map,
        emissive: if emits { emission } else { [0.0; 3] },
        emissive_map,
        occlusion_map,
    }
}

/// The base surface detail as one tangent-space normal map, as the Octane
/// path bakes it (`surface_detail_pins`): the Normal Map at its strength
/// and the Bump Strength height map blended onto it (`detail_normal`), or
/// either alone; a normal map Iray reads at gamma 1 and full strength as it
/// is. None without either, or when the two are placed differently and
/// can't be baked into one (then the normal map alone).
fn normal(t: &mut Textures, dirs: &[PathBuf], mat: &Material) -> Option<(String, ImageMod)> {
    let nm = mat.image("Normal Map").map(str::to_string).zip(normal_factor(mat, "Normal Map")).and_then(|(src, f)| Some((t.asset(dirs, &src)?, mat.image_mod("Normal Map").placement(), f)));
    let f = mat.float("Bump Strength").unwrap_or(1.0);
    let bump = (f > 0.0).then(|| mat.image("Bump Strength").map(str::to_string)).flatten().and_then(|src| Some((t.asset(dirs, &src)?, f, mat.image_mod("Bump Strength").placement())));
    if let Some((b, f, bm)) = &bump {
        if nm.as_ref().map_or(true, |(_, m, _)| m == bm) {
            let height = Bump { asset: b, kind: BumpKind::Height(*f), gamma: mat.image_gamma("Bump Strength") };
            let baked = match &nm {
                Some((a, _, nf)) => t.detail_normal(Bump { asset: a, kind: BumpKind::Normal(*nf), gamma: mat.image_gamma("Normal Map") }, Some(height)),
                None => t.detail_normal(height, None),
            };
            if let Some(a) = baked {
                return Some((a, *bm));
            }
        }
    }
    let (a, m, f) = nm?;
    let g = t.map_gamma(&a, mat.image_gamma("Normal Map"));
    if g == 1.0 && f == 1.0 {
        return Some((a, m));
    }
    let baked = t.detail_normal(Bump { asset: &a, kind: BumpKind::Normal(f), gamma: Some(g) }, None);
    Some((baked.unwrap_or(a), m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::rc::Rc;

    fn material(channels: serde_json::Value) -> Rc<Material> {
        let m = json!({"extra": [{"channels": channels}]});
        Rc::new(Material::from_dson(&m, &HashMap::new()))
    }

    fn png(dir: &Path, name: &str, colour: image::Rgb<u8>) {
        let p = dir.join("Runtime/Textures").join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        image::RgbImage::from_pixel(8, 8, colour).save(&p).unwrap();
    }

    #[test]
    fn maps_go_on_the_preview_surface_with_their_values() {
        let dir = tempfile::tempdir().unwrap();
        for n in ["d.png", "r.png", "o.png", "n.png", "h.png"] {
            png(dir.path(), n, image::Rgb([128, 128, 255]));
        }
        let mat = material(json!([
            {"channel": {"id": "diffuse", "current_value": [0.5, 0.5, 0.5], "image_file": "/Runtime/Textures/d.png"}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossy Roughness", "current_value": 0.6, "image_file": "/Runtime/Textures/r.png"}},
            {"channel": {"id": "Cutout Opacity", "current_value": 1, "image_file": "/Runtime/Textures/o.png"}},
            {"channel": {"id": "Normal Map", "current_value": 1, "image_file": "/Runtime/Textures/n.png"}},
            {"channel": {"id": "Horizontal Tiles", "current_value": 2}},
        ]));
        let dirs = [dir.path().to_path_buf()];
        let (p, tex) = preview_materials(&[("M".into(), mat.clone(), 1.0)], &[false], &dirs, HashMap::new());
        let p = &p[0];
        let d = p.diffuse_map.as_ref().expect("diffuse map");
        assert!(d.srgb && d.output == "rgb");
        // The base colour (linear) as the map's scale.
        assert!((d.scale[0] - mat.base_color()[0]).abs() < 1e-6 && d.scale[3] == 1.0, "{d:?}");
        assert_eq!(d.placement, Some(([2.0, 1.0], [0.0, 0.0])), "the material's tiles on st");
        let r = p.roughness_map.as_ref().expect("roughness map");
        assert_eq!((r.output, r.scale[0], r.bias[0]), ("r", 0.6, 0.0));
        // An 8-bit grey map: Iray reads it at gamma 2.2 (#49).
        assert!(r.srgb);
        assert!((p.roughness - 0.6).abs() < 1e-6, "the slider, squared by both: {}", p.roughness);
        assert_eq!(p.opacity_threshold, 0.5, "a cutout map is a mask");
        // An 8-bit normal map, gamma 2.2: baked linear, read raw.
        let n = p.normal_map.as_ref().expect("normal map");
        assert!(!n.srgb && n.scale[..3] == [2.0; 3] && n.bias[..3] == [-1.0; 3]);
        assert!(tex.generated.iter().any(|(a, _)| *a == n.asset), "the linear copy is generated");
        assert_eq!(tex.files.len(), 4, "d, r, o, n packed");

        // Height alone: baked into a normal map; a glossiness is inverted.
        let mat = material(json!([
            {"channel": {"id": "Base Mixing", "current_value": 1}},
            {"channel": {"id": "Glossy Layered Weight", "current_value": 1}},
            {"channel": {"id": "Glossiness", "current_value": 0.8, "image_file": "/Runtime/Textures/r.png"}},
            {"channel": {"id": "Bump Strength", "current_value": 0.5, "image_file": "/Runtime/Textures/h.png"}},
        ]));
        let (p, tex) = preview_materials(&[("M".into(), mat, 1.0)], &[false], &dirs, HashMap::new());
        let r = p[0].roughness_map.as_ref().unwrap();
        assert!((r.scale[0] + 0.8).abs() < 1e-6 && (r.bias[0] - 1.0).abs() < 1e-6, "1 − glossiness: {r:?}");
        assert!((p[0].roughness - 0.2).abs() < 1e-5, "{}", p[0].roughness);
        assert!(p[0].normal_map.as_ref().is_some_and(|n| tex.generated.iter().any(|(a, _)| *a == n.asset)));
        assert!(p[0].diffuse_map.is_none());

        // No gloss: fully rough, not the preview's 4 % mirror; a refraction
        // lets what is behind through.
        let mat = material(json!([
            {"channel": {"id": "Glossy Roughness", "current_value": 0.1, "image_file": "/Runtime/Textures/r.png"}},
            {"channel": {"id": "Refraction Weight", "current_value": 0.75}},
        ]));
        let (p, _) = preview_materials(&[("M".into(), mat, 1.0)], &[false], &dirs, HashMap::new());
        assert!(p[0].roughness_map.is_some(), "the refraction reflects");
        assert!((p[0].opacity - 0.25).abs() < 1e-6, "{}", p[0].opacity);
        let mat = material(json!([{"channel": {"id": "Glossy Roughness", "current_value": 0.1, "image_file": "/Runtime/Textures/r.png"}}]));
        let (p, _) = preview_materials(&[("M".into(), mat, 1.0)], &[false], &dirs, HashMap::new());
        assert_eq!((p[0].roughness, p[0].roughness_map.is_none()), (1.0, true));

        // A strand: no maps, the diffuse map's mean in the colour.
        let mat = material(json!([{"channel": {"id": "diffuse", "current_value": [1, 1, 1], "image_file": "/Runtime/Textures/d.png"}}]));
        let (p, _) = preview_materials(&[("H".into(), mat, 0.0)], &[true], &dirs, HashMap::new());
        assert!(p[0].maps().next().is_none());
        assert!(p[0].diffuse[2] > 0.9 && p[0].diffuse[0] < 0.3, "{:?}", p[0].diffuse);
    }
}
