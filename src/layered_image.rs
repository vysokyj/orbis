//! DAZ Layered Image Editor images.
//!
//! A material channel can name an `image_library` entry instead of a file
//! (`"image": "#id"`). Its `map[]` is a stack of layers — image files or flat
//! colours — blended bottom to top; skin with tattoos or make-up is the
//! classic case (base skin + an alpha-blended tattoo PNG). Renderers that
//! only take one file per input need the stack flattened: [`composite`].

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use image::{imageops, Rgba, RgbaImage};
use serde_json::Value;

/// One layer of a layered image.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Content-relative image file, percent-decoded; `None` for a colour layer.
    pub image: Option<String>,
    pub color: Option<[f32; 3]>,
    pub active: bool,
    /// 1 = opaque.
    pub opacity: f32,
    /// `blend_source_over`, `alpha_blend`, `multiply`, `add`, …
    pub operation: String,
    pub invert: bool,
    /// Placement in pixels of the result.
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub mirror: [bool; 2],
}

/// A layered image from a DSON `image_library` entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayeredImage {
    /// Output size (`map_size`), when the entry gives one.
    pub size: Option<[u32; 2]>,
    pub layers: Vec<Layer>,
}

impl LayeredImage {
    pub fn from_dson(entry: &Value) -> Self {
        let f = |v: &Value, k: &str, d: f32| v.get(k).and_then(Value::as_f64).map(|x| x as f32).unwrap_or(d);
        let b = |v: &Value, k: &str, d: bool| v.get(k).and_then(Value::as_bool).unwrap_or(d);
        let layers = entry
            .get("map")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|m| Layer {
                image: m.get("url").and_then(Value::as_str).filter(|u| !u.is_empty()).map(crate::dson::percent_decode),
                color: m.get("color").and_then(Value::as_array).map(|c| {
                    [0, 1, 2].map(|i| c.get(i).and_then(Value::as_f64).unwrap_or(1.0) as f32)
                }),
                active: b(m, "active", true),
                opacity: f(m, "transparency", 1.0),
                operation: m.get("operation").and_then(Value::as_str).unwrap_or("alpha_blend").to_string(),
                invert: b(m, "invert", false),
                offset: [f(m, "xoffset", 0.0), f(m, "yoffset", 0.0)],
                scale: [f(m, "xscale", 1.0), f(m, "yscale", 1.0)],
                mirror: [b(m, "xmirror", false), b(m, "ymirror", false)],
            })
            .collect();
        let size = entry.get("map_size").and_then(Value::as_array).and_then(|s| {
            Some([s.first()?.as_u64()? as u32, s.get(1)?.as_u64()? as u32])
        });
        LayeredImage { size, layers }
    }

    /// Whether this needs compositing (more than one layer, or a colour).
    pub fn is_stack(&self) -> bool {
        self.layers.len() > 1 || self.layers.iter().any(|l| l.image.is_none())
    }
}

fn find(dirs: &[PathBuf], rel: &str) -> Option<PathBuf> {
    let rel = rel.trim_start_matches('/');
    dirs.iter().map(|d| d.join(rel)).find(|p| p.is_file())
}

fn load(path: &Path) -> Result<RgbaImage> {
    Ok(image::open(path).map_err(|e| anyhow!("{}: {e}", path.display()))?.to_rgba8())
}

/// Flatten the stack into one RGBA image. The size is `map_size`, else the
/// first image layer's. Missing layer files are skipped (listed in the
/// returned names).
pub fn composite(img: &LayeredImage, dirs: &[PathBuf]) -> Result<(RgbaImage, Vec<String>)> {
    let mut missing = Vec::new();
    let mut loaded: Vec<Option<RgbaImage>> = Vec::with_capacity(img.layers.len());
    for l in &img.layers {
        loaded.push(match (&l.image, l.active) {
            (Some(rel), true) => match find(dirs, rel) {
                Some(p) => Some(load(&p)?),
                None => {
                    missing.push(rel.clone());
                    None
                }
            },
            _ => None,
        });
    }
    let [nominal_w, _] = img
        .size
        .filter(|[w, h]| *w > 0 && *h > 0)
        .or_else(|| loaded.iter().flatten().next().map(|i| [i.width(), i.height()]))
        .ok_or_else(|| anyhow!("layered image without a size or an image layer"))?;
    let [w, h] = img.size.filter(|[w, h]| *w > 0 && *h > 0).unwrap_or_else(|| loaded.iter().flatten().next().map_or([nominal_w, nominal_w], |i| [i.width(), i.height()]));
    // Prefer the base image's resolution over a small nominal map_size; the
    // layers' pixels scale with it.
    let [w, h] = match loaded.iter().flatten().next() {
        Some(first) if first.width() > w => [first.width(), first.height()],
        _ => [w, h],
    };
    let ratio = w as f32 / nominal_w.max(1) as f32;

    let mut out = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    for (l, layer_img) in img.layers.iter().zip(loaded) {
        if !l.active || l.opacity <= 0.0 {
            continue;
        }
        let layer = match (layer_img, l.color) {
            (Some(mut li), _) => {
                // Layer pixels sit at `offset` at their own size, scaled, as
                // DAZ composes them: Julius 9's 4096 × 2048 iris layer fills
                // the top half of his 4096 × 4096 eye map (DAZ's temp image
                // for Iray) — stretched over all of it, the irises missed
                // the eyes' UVs.
                if l.mirror[0] {
                    imageops::flip_horizontal_in_place(&mut li);
                }
                if l.mirror[1] {
                    imageops::flip_vertical_in_place(&mut li);
                }
                let sw = (li.width() as f32 * ratio * l.scale[0]).round().max(1.0) as u32;
                let sh = (li.height() as f32 * ratio * l.scale[1]).round().max(1.0) as u32;
                if [sw, sh, li.width(), li.height()] != [w, h, w, h] || l.offset != [0.0, 0.0] {
                    let scaled = if [sw, sh] == [li.width(), li.height()] { li } else { imageops::resize(&li, sw, sh, imageops::FilterType::Triangle) };
                    let mut placed = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
                    imageops::overlay(&mut placed, &scaled, (l.offset[0] * ratio) as i64, (l.offset[1] * ratio) as i64);
                    li = placed;
                }
                li
            }
            (None, Some(c)) => {
                let px = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                RgbaImage::from_pixel(w, h, Rgba([px[0], px[1], px[2], 255]))
            }
            (None, None) => continue,
        };
        blend(&mut out, &layer, l);
    }
    // The result is a colour map: fully opaque.
    for p in out.pixels_mut() {
        p[3] = 255;
    }
    Ok((out, missing))
}

/// Blend `layer` onto `out` with the layer's operation and opacity.
fn blend(out: &mut RgbaImage, layer: &RgbaImage, l: &Layer) {
    for (o, s) in out.pixels_mut().zip(layer.pixels()) {
        let mut src = [s[0], s[1], s[2]].map(|v| v as f32 / 255.0);
        if l.invert {
            src = src.map(|v| 1.0 - v);
        }
        let a = s[3] as f32 / 255.0 * l.opacity;
        if a <= 0.0 {
            continue;
        }
        let dst = [o[0], o[1], o[2]].map(|v| v as f32 / 255.0);
        let mixed: [f32; 3] = match l.operation.as_str() {
            "multiply" => [0, 1, 2].map(|i| dst[i] * src[i]),
            "add" | "linear_dodge" => [0, 1, 2].map(|i| (dst[i] + src[i]).min(1.0)),
            "subtract" => [0, 1, 2].map(|i| (dst[i] - src[i]).max(0.0)),
            "screen" => [0, 1, 2].map(|i| 1.0 - (1.0 - dst[i]) * (1.0 - src[i])),
            // blend_source_over, alpha_blend and the rest: normal "over".
            _ => src,
        };
        let da = o[3] as f32 / 255.0;
        for i in 0..3 {
            let v = if da == 0.0 { src[i] } else { dst[i] + (mixed[i] - dst[i]) * a };
            o[i] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        o[3] = ((da + a * (1.0 - da)).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tattoo_layer_blends_over_base_skin() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Runtime/Textures")).unwrap();
        // Base: flat skin tone. Tattoo: transparent except one black pixel.
        RgbaImage::from_pixel(4, 4, Rgba([200, 150, 120, 255])).save(dir.path().join("Runtime/Textures/skin.png")).unwrap();
        let mut tattoo = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 0]));
        tattoo.put_pixel(1, 1, Rgba([0, 0, 0, 255]));
        tattoo.save(dir.path().join("Runtime/Textures/stars%201.png".replace("%20", " "))).unwrap();

        let entry = json!({"id": "lie", "map": [
            {"url": "/Runtime/Textures/skin.png", "operation": "blend_source_over", "transparency": 1},
            {"url": "/Runtime/Textures/stars%201.png", "operation": "alpha_blend", "transparency": 1},
            {"url": "/Runtime/Textures/nope.png", "operation": "alpha_blend"},
        ]});
        let img = LayeredImage::from_dson(&entry);
        assert!(img.is_stack());
        let (out, missing) = composite(&img, &[dir.path().to_path_buf()]).unwrap();
        assert_eq!(missing, ["/Runtime/Textures/nope.png"]);
        assert_eq!(out.get_pixel(0, 0), &Rgba([200, 150, 120, 255]), "base shows through");
        assert_eq!(out.get_pixel(1, 1), &Rgba([0, 0, 0, 255]), "tattoo on top");
    }

    #[test]
    fn colour_layer_with_opacity() {
        let entry = json!({"map_size": [2, 2], "map": [
            {"color": [1, 0, 0], "operation": "blend_source_over"},
            {"color": [0, 0, 1], "operation": "alpha_blend", "transparency": 0.5},
        ]});
        let (out, _) = composite(&LayeredImage::from_dson(&entry), &[]).unwrap();
        assert_eq!(out.get_pixel(0, 0), &Rgba([128, 0, 128, 255]));
    }

    /// Julius 9's eye map against the image DAZ composed of it for Iray
    /// (its temp folder): only where both are on this machine.
    #[test]
    fn layers_of_another_size_sit_at_their_own_size() {
        let lib = std::path::PathBuf::from("/Volumes/Data/DAZ 3D/Studio/My DAZ 3D Library");
        let daz = std::path::Path::new(&std::env::var("HOME").unwrap_or_default())
            .join("Library/Application Support/DAZ 3D/Studio6 [orbisrender]/temp/d3_Eye Color-1_c70935dd-b561-d7d9-38d8-a0bded824185 2.png");
        if !lib.is_dir() || !daz.is_file() {
            eprintln!("skipped: no DAZ content / temp image");
            return;
        }
        let entry = serde_json::json!({"id": "eye", "map_size": [4096, 4096], "map": [
            {"label": "Base", "active": true, "color": [1, 1, 1], "transparency": 1, "operation": "blend_source_over"},
            {"url": "/Runtime/Textures/DAZ/Characters/Genesis9/Base/Eyes/Split/Genesis9_Eyes_Sclera_04.jpg", "active": true, "transparency": 1, "operation": "blend_source_over"},
            {"url": "/Runtime/Textures/Daz/Characters/Genesis9/Julius9/Julius9_Eyes_Iris_D_1001.png", "active": true, "transparency": 1, "operation": "blend_source_over"},
        ]});
        let (ours, missing) = composite(&LayeredImage::from_dson(&entry), &[lib]).unwrap();
        assert!(missing.is_empty(), "{missing:?}");
        let theirs = image::open(&daz).unwrap().to_rgba8();
        let mut diff = 0u64;
        for (a, b) in ours.pixels().step_by(997).zip(theirs.pixels().step_by(997)) {
            diff += (0..3).map(|k| (i32::from(a.0[k]) - i32::from(b.0[k])).unsigned_abs() as u64).sum::<u64>();
        }
        let n = (ours.width() * ours.height() / 997) as u64 * 3;
        assert!(diff / n < 4, "mean channel difference {}", diff / n);
    }
}
