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
}

/// A material group's merged channels.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Material {
    pub channels: HashMap<String, Channel>,
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
            self.channels.insert(id.to_string(), Channel { value, image });
        };
        if let Some(obj) = m.as_object() {
            for v in obj.values() {
                if let Some(c) = v.get("channel") {
                    add(c);
                }
            }
        }
        for e in m.get("extra").and_then(Value::as_array).into_iter().flatten() {
            for c in e.get("channels").and_then(Value::as_array).into_iter().flatten() {
                if let Some(c) = c.get("channel") {
                    add(c);
                }
            }
        }
    }

    pub fn channel(&self, id: &str) -> Option<&Channel> {
        self.channels.get(id)
    }

    pub fn float(&self, id: &str) -> Option<f32> {
        match &self.channel(id)?.value {
            Value::Number(n) => n.as_f64().map(|f| f as f32),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn color(&self, id: &str) -> Option<[f32; 3]> {
        let a = self.channel(id)?.value.as_array()?;
        Some([0, 1, 2].map(|i| a.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32))
    }

    pub fn image(&self, id: &str) -> Option<&str> {
        self.channel(id)?.image.as_deref()
    }

    /// Base color (Iray `diffuse` / "Base Color").
    pub fn base_color(&self) -> [f32; 3] {
        self.color("diffuse").unwrap_or([0.8; 3])
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
        if let Some(r) = self.float("Glossy Roughness") {
            return (Some("Glossy Roughness"), r, false);
        }
        if let Some(g) = self.float("Glossiness") {
            return (Some("Glossiness"), 1.0 - g, true);
        }
        (None, 0.5, false)
    }

    pub fn metallic(&self) -> f32 {
        self.float("Metallic Weight").unwrap_or(0.0)
    }

    /// Cutout opacity, 1 = opaque (geometry shells are often 0 outside a mask).
    pub fn opacity(&self) -> f32 {
        self.float("Cutout Opacity").unwrap_or(1.0)
    }
}

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
    use super::*;
    use serde_json::json;

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
        assert_eq!(m.base_color(), [1.0, 0.5, 0.25]);
        assert_eq!(m.image("diffuse"), Some("/Runtime/Textures/A B/skin.jpg"));
        assert_eq!(m.image("Cutout Opacity"), Some("/Runtime/Textures/m.png"));
        assert_eq!((m.roughness(), m.metallic(), m.opacity()), (0.4, 1.0, 1.0));
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
