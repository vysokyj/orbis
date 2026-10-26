//! Iray environment (HDRI, ground) and tone mapping.

use super::*;

/// DAZ's Iray environment (the "Environment Options" node).
#[derive(Debug, Clone, PartialEq)]
pub struct SceneEnvironment {
    /// Content-relative HDRI (`.hdr` / `.exr`), when the dome shows one.
    pub map: Option<String>,
    /// Dome brightness: "Environment Intensity" × the "Environment Map" value.
    pub intensity: f64,
    /// Dome rotation about the vertical axis, degrees.
    pub rotation_deg: f64,
    /// Dome orientation (tilt) about X / Y / Z, degrees.
    pub orientation_deg: [f64; 3],
    /// Iray "Scene Only": no dome light at all.
    pub scene_only: bool,
    /// Iray's shadow-catching ground ("Draw Ground"): its shadow intensity.
    pub ground_shadow: Option<f64>,
    /// `Ground Position Mode` Manual: the ground's height (`Ground Origin Y`,
    /// DAZ cm). Auto (`None`) puts it at the scene's lowest point.
    pub ground_origin_y: Option<f64>,
}

impl SceneEnvironment {
    /// The lat-long map's U shift that turns Octane's texture environment like
    /// the DAZ dome: `0.25 + (rotation − orientation Y) / 360`, wrapped to
    /// [0, 1) (`Ry(-rot) · Ry(orient y)`; offset and sign
    /// measured in Octane X against the Blender render at 0° and 110°). The
    /// environment's transform is a UV transform there — a 3D rotation does
    /// nothing — so the X / Z tilt is left out.
    pub fn dome_u_shift(&self) -> f64 {
        (0.25 + (self.rotation_deg - self.orientation_deg[1]) / 360.0).rem_euclid(1.0)
    }
}

/// Where Iray puts its ground, meters: `Ground Origin Y` in Manual mode,
/// else (Auto) the lowest point of the scene's geometry — 0 for an empty one.
pub(super) fn ground_height(env: &SceneEnvironment, meshes: &[Mesh]) -> f64 {
    if let Some(y) = env.ground_origin_y {
        return y * f64::from(CM);
    }
    let lowest = meshes.iter().flat_map(|m| m.points.iter().map(|p| p[1])).fold(f32::INFINITY, f32::min);
    if lowest.is_finite() { f64::from(lowest) } else { 0.0 }
}

/// The scene's Iray environment, if it saved an "Environment Options" node.
pub(super) fn scene_environment(lib: &mut Library) -> Option<SceneEnvironment> {
    let nodes = lib.duf.pointer("/scene/nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    for node in &nodes {
        let def = node.get("url").and_then(Value::as_str).and_then(|u| lib.entry(u, "node_library"));
        let is_env = |v: &Value| v.get("id").and_then(Value::as_str).is_some_and(|i| i.starts_with("Environment Options"));
        if !is_env(node) && !def.as_ref().is_some_and(is_env) {
            continue;
        }
        // Channel values and the map's image, scene node over definition.
        let mut ch: HashMap<String, Value> = def.as_ref().map(node_channels).unwrap_or_default();
        ch.extend(node_channels(node));
        let image = |n: &Value| -> Option<String> {
            n.get("extra")?.as_array()?.iter().flat_map(|e| e.get("channels").and_then(Value::as_array).into_iter().flatten()).find_map(|c| {
                let c = c.get("channel")?;
                (c.get("id")?.as_str()? == "Environment Map").then_some(())?;
                c.get("image_file").or_else(|| c.get("image")).and_then(Value::as_str).map(percent_decode)
            })
        };
        let map = image(node).or_else(|| def.as_ref().and_then(image)).filter(|m| !m.is_empty() && !m.starts_with('#'));
        let num = |k: &str| ch.get(k).and_then(Value::as_f64);
        return Some(SceneEnvironment {
            map,
            // Intensity × the map channel's own value.
            intensity: num("Environment Intensity").unwrap_or(1.0) * num("Environment Map").unwrap_or(1.0),
            rotation_deg: num("Dome Rotation").unwrap_or(0.0),
            orientation_deg: ["X", "Y", "Z"].map(|a| num(&format!("Dome Orientation {a}")).unwrap_or(0.0)),
            scene_only: num("Environment Mode") == Some(3.0),
            ground_shadow: (ch.get("Draw Ground").and_then(Value::as_bool).unwrap_or(true) && num("Environment Mode") != Some(3.0))
                .then(|| num("Ground Shadow Intensity").unwrap_or(1.0)),
            ground_origin_y: (num("Ground Position Mode") == Some(1.0)).then(|| num("Ground Origin Y").unwrap_or(0.0)),
        });
    }
    None
}

/// DAZ's Iray tone mapping (the "Tonemapper Options" node).
#[derive(Debug, Clone, PartialEq)]
pub struct ToneMapping {
    pub exposure_value: f64,
    pub cm2_factor: f64,
    /// The colour that maps to white (`White Point` × `White Point Scale`).
    pub white_point: [f64; 3],
    /// 1 = highlights left alone, 0 = fully compressed.
    pub burn_highlights: f64,
    /// Darkens the shadows (0 = off).
    pub crush_blacks: f64,
    pub saturation: f64,
    pub gamma: f64,
}

impl Default for ToneMapping {
    fn default() -> Self {
        ToneMapping { exposure_value: 13.0, cm2_factor: 1.0, white_point: [1.0; 3], burn_highlights: 1.0, crush_blacks: 0.0, saturation: 1.0, gamma: 2.2 }
    }
}

impl ToneMapping {
    /// Octane imager exposure: `base` (the DAZ default EV 13) scaled by the
    /// stops the scene's EV and cm² factor move it.
    pub fn octane_exposure(&self, base: f64) -> f64 {
        base * (13.0 - self.exposure_value).exp2() * self.cm2_factor
    }

    /// Iray's tone curve on a linear value (after exposure): burn,
    /// crush, gamma — c (1 + c b) / (1 + c), then ^(1 + crush), then
    /// ^(1 / gamma). The burn is the photographic tone mapper's; the crush
    /// a fit (Gladiator, crush 0.07: within 1.7 / 255 of Iray's pixels).
    pub fn iray_curve(&self, c: f64) -> f64 {
        let b = self.burn_highlights.clamp(0.0, 1.0);
        let c = c.max(0.0);
        let burnt = c * (1.0 + c * b) / (1.0 + c);
        burnt.powf(1.0 + self.crush_blacks.max(0.0)).powf(1.0 / self.gamma.max(0.1))
    }

    /// Whether Octane's own imager reproduces this: the calibrated defaults
    /// (Burn 1, Crush 0, Gamma 2.2 — the Gamma 2.2 response).
    pub fn is_plain(&self) -> bool {
        self.burn_highlights >= 1.0 && self.crush_blacks <= 0.0 && (self.gamma - 2.2).abs() < 1e-3
    }

    /// Iray's curve as a 1D `.cube` LUT for Octane's imager: its input is
    /// the Gamma 2.2 response of c / `LUT_HEADROOM` (the exposure is
    /// divided by it, so highlights up to 16× white survive the response's
    /// clip at 1). Octane applies response, gamma, then LUT (`order` 0).
    pub fn cube_lut(&self) -> String {
        let mut out = format!("TITLE \"ORBIS Iray tone mapping\"\nLUT_1D_SIZE {LUT_SIZE}\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\n");
        for i in 0..LUT_SIZE {
            let x = i as f64 / (LUT_SIZE - 1) as f64;
            let v = self.iray_curve(x.powf(2.2) * LUT_HEADROOM).clamp(0.0, 1.0);
            let _ = writeln!(out, "{v:.6} {v:.6} {v:.6}");
        }
        out
    }
}

/// Entries of the tone LUT.
const LUT_SIZE: usize = 4096;

/// How far above white the tone LUT reaches (see [`ToneMapping::cube_lut`]).
pub const LUT_HEADROOM: f64 = 16.0;

/// Where the tone LUT goes in an `.orbx`.
pub const TONE_LUT_ASSET: &str = "assets/tone.cube";

/// The scene's Iray tone mapping; DAZ's defaults when it saved none or
/// tone mapping is off.
pub(super) fn scene_tone_mapping(lib: &mut Library) -> ToneMapping {
    let nodes = lib.duf.pointer("/scene/nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    for node in &nodes {
        let def = node.get("url").and_then(Value::as_str).and_then(|u| lib.entry(u, "node_library"));
        let is_tone = |v: &Value| v.get("id").and_then(Value::as_str).is_some_and(|i| i.starts_with("Tonemapper Options"));
        if !is_tone(node) && !def.as_ref().is_some_and(is_tone) {
            continue;
        }
        let mut ch: HashMap<String, Value> = def.as_ref().map(node_channels).unwrap_or_default();
        ch.extend(node_channels(node));
        if ch.get("Tone Mapping Enable").and_then(Value::as_bool) == Some(false) {
            return ToneMapping::default();
        }
        let num = |k: &str, d: f64| ch.get(k).and_then(Value::as_f64).unwrap_or(d);
        let scale = num("White Point Scale", 1.0);
        let wp = ch.get("White Point").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<_>>());
        let white_point = match wp.as_deref() {
            Some(&[r, g, b]) => [r * scale, g * scale, b * scale],
            _ => [scale; 3],
        };
        return ToneMapping {
            exposure_value: num("Exposure Value", 13.0),
            cm2_factor: num("cm2 Factor", 1.0),
            white_point,
            burn_highlights: num("Burn Highlights", 1.0),
            crush_blacks: num("Crush Blacks", 0.0),
            saturation: num("Saturation", 1.0),
            gamma: num("Gamma", 2.2),
        };
    }
    ToneMapping::default()
}
