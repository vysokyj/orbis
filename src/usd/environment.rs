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
    /// "Draw Dome": the dome is seen behind the scene. Off, Iray still lights
    /// with it but shows black there (Calib-L-*, Calib-Dome-Off).
    pub draw_dome: bool,
    /// Whether Iray lights with the scene's lights: not under "Dome Only"
    /// (1) or "Sun-Sky Only" (2) — a 2 000 000 lm point light left the
    /// floor unchanged there (Calib-L-Mode-*: 1.000).
    pub scene_lights: bool,
    /// Iray's procedural sun and sky ("Sun-Sky Only", Environment Mode 2):
    /// Octane's Daylight environment instead of the dome.
    pub sun_sky: Option<SunSky>,
}

/// DAZ's Iray Sun-Sky (the `SS …` channels of "Environment Options") as
/// Octane Daylight environment inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct SunSky {
    /// Unit vector towards the sun, scene axes (Y up, north −Z, east +X).
    pub sun_dir: [f64; 3],
    /// Octane's sky turbidity: 2 + 2 × `SS Haze`, at most 10 (Hosek-Wilkie
    /// renders 14 like 10).
    pub turbidity: f64,
    /// Octane's `power`: `SS Multiplier` relative to DAZ's default 0.10132,
    /// × `SUN_SKY_POWER`, dimmed by the haze (`haze_dimming`).
    pub power: f64,
    /// Octane's `sunIntensity`: `SUN_SKY_SUN_SHARE` × `SS Sun Disk Intensity`.
    pub sun_intensity: f64,
    /// Octane's `sun_size` (× the real sun): `SS Sun Disk Scale` / 4.
    pub sun_size: f64,
    /// `SS Ground Color`, linear.
    pub ground: [f64; 3],
}

/// Octane Daylight (Hosek-Wilkie) `power` at DAZ's default `SS Multiplier`
/// (0.10132), and its `sunIntensity` at `SS Sun Disk Intensity` 1:
/// Calib-L-Mode-SunSky-off (Iray's defaults, 18:00) at power 1 rendered the
/// sky-lit shadow 0.37 × Iray and the sunlit floor 0.53 × — Octane's sun
/// 2.2 × its sky where Iray's is 1.2 ×; at 2.7 / 0.55 the floor 1.02, the
/// shadow 0.96, the ground below the horizon 0.92.
pub const SUN_SKY_POWER: f64 = 2.7;

/// Iray's haze dims its sun and sky where Octane's turbidity brightens:
/// Calib-L-Mode-SunSky-noon-haze4 (haze 4) at turbidity 10 rendered 1.67 ×
/// Iray (floor and shadow alike) — × exp(−0.128 · haze) = 0.60 there. One
/// haze measured; haze 0 at noon and 18:00 sits at 1.01 – 1.02.
pub fn haze_dimming(haze: f64) -> f64 {
    (-0.128 * haze.max(0.0)).exp()
}
pub const SUN_SKY_SUN_SHARE: f64 = 0.55;

/// The sun's direction for DAZ's `SS Latitude` / `SS Longitude` (degrees,
/// east positive), `SS Day` (Julian day number), `SS Time` (seconds since
/// local midnight) and `SS UTC Offset` (hours): the low-precision solar
/// position of the Astronomical Almanac (≈ 0.01°), as a unit vector in
/// scene axes — north −Z, east +X, up +Y (Calib-L-Mode-SunSky-off: 18:00
/// in Salt Lake City casts its shadows east, to +X).
pub fn sun_direction(lat: f64, lon: f64, day: f64, time_s: f64, utc_offset: f64) -> [f64; 3] {
    let jd = day - 0.5 + (time_s / 3600.0 - utc_offset) / 24.0;
    let n = jd - 2_451_545.0;
    let l = (280.460 + 0.985_647_4 * n).rem_euclid(360.0);
    let g = (357.528 + 0.985_600_3 * n).rem_euclid(360.0).to_radians();
    let lambda = (l + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();
    let eps = (23.439 - 0.000_000_4 * n).to_radians();
    let ra = (eps.cos() * lambda.sin()).atan2(lambda.cos());
    let dec = (eps.sin() * lambda.sin()).asin();
    let gmst_h = (18.697_374_558 + 24.065_709_824_419_08 * n).rem_euclid(24.0);
    let h = (gmst_h * 15.0 + lon).to_radians() - ra;
    let phi = lat.to_radians();
    let el = (phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos()).asin();
    // Azimuth from north, clockwise (east 90°).
    let az = (-h.sin()).atan2(dec.tan() * phi.cos() - phi.sin() * h.cos());
    [az.sin() * el.cos(), el.sin(), -az.cos() * el.cos()]
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
        let steered = sun_direction_node(lib, node, def.as_ref(), &nodes);
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
            draw_dome: ch.get("Draw Dome").and_then(Value::as_bool).unwrap_or(true),
            scene_lights: !matches!(num("Environment Mode").map(|m| m as i64), Some(1 | 2)),
            sun_sky: (num("Environment Mode") == Some(2.0)).then(|| {
                let ground = ch.get("SS Ground Color").and_then(Value::as_array);
                SunSky {
                    sun_dir: steered.unwrap_or_else(|| {
                        sun_direction(
                            num("SS Latitude").unwrap_or(40.76426),
                            num("SS Longitude").unwrap_or(-111.8972),
                            num("SS Day").unwrap_or(2_457_092.0),
                            num("SS Time").unwrap_or(43_200.0),
                            num("SS UTC Offset").unwrap_or(-6.0),
                        )
                    }),
                    turbidity: (2.0 + 2.0 * num("SS Haze").unwrap_or(0.0)).clamp(2.0, 10.0),
                    power: SUN_SKY_POWER * num("SS Multiplier").unwrap_or(0.10132) / 0.10132 * haze_dimming(num("SS Haze").unwrap_or(0.0)),
                    sun_intensity: SUN_SKY_SUN_SHARE * num("SS Sun Disk Intensity").unwrap_or(1.0),
                    sun_size: num("SS Sun Disk Scale").unwrap_or(4.0) / 4.0,
                    ground: [0, 1, 2].map(|i| {
                        f64::from(crate::daz_material::colour_to_linear(ground.and_then(|g| g.get(i)).and_then(Value::as_f64).unwrap_or(0.4) as f32))
                    }),
                }
            }),
        });
    }
    None
}

/// The sun's direction from the node `SS Sun Direction` names (a channel
/// whose `node` is `#<scene node id>`), when set: from the origin to the
/// node's position — Iray ignores its rotation (Calib-L-Mode-SunSky-dir: a
/// light at (300, 400, 300) cm turned to point back left cast its shadows
/// back left, from a high front-right sun; at the origin, only turned,
/// Iray rendered night). None for a node at the origin.
fn sun_direction_node(lib: &mut Library, env: &Value, env_def: Option<&Value>, nodes: &[Value]) -> Option<[f64; 3]> {
    let target = |n: &Value| -> Option<String> {
        n.get("extra")?.as_array()?.iter().flat_map(|e| e.get("channels").and_then(Value::as_array).into_iter().flatten()).find_map(|c| {
            let c = c.get("channel")?;
            (c.get("id")?.as_str()? == "SS Sun Direction").then_some(())?;
            c.get("node").and_then(Value::as_str).map(|s| percent_decode(s.trim_start_matches('#')))
        })
    };
    let id = target(env).or_else(|| env_def.and_then(target))?;
    let node = nodes.iter().find(|n| n.get("id").and_then(Value::as_str).map(percent_decode).as_deref() == Some(id.as_str()))?;
    let def = node.get("url").and_then(Value::as_str).and_then(|u| lib.entry(u, "node_library"));
    let p = channel3(node, def.as_ref(), "translation", 0.0);
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    (len > 0.0).then(|| p.map(|c| c / len))
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
    /// crush, gamma — c (1 + c b) / (1 + c), then x² / (x + 0.3 crush),
    /// then ^(1 / gamma). Burn and gamma are the photographic tone mapper's
    /// (Calib-Tone: within 0.5 / 255 of Iray's pixels); the crush is a fit
    /// (`CRUSH_KNEE`).
    pub fn iray_curve(&self, c: f64) -> f64 {
        let b = self.burn_highlights.clamp(0.0, 1.0);
        let c = c.max(0.0);
        let burnt = c * (1.0 + c * b) / (1.0 + c);
        let crush = self.crush_blacks.max(0.0);
        let crushed = if crush > 0.0 { burnt * burnt / (burnt + CRUSH_KNEE * crush) } else { burnt };
        crushed.powf(1.0 / self.gamma.max(0.1))
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

/// Iray's Crush Blacks pulls each channel down as x² / (x + k · crush):
/// fitted on Calib-Tone's Crush 0.1 / 0.3 against its Crush 0 render (same
/// samples, so the same noise) — 1.4 / 2.4 / 255, Gladiator's mix 1.2;
/// `^(1 + crush)` gave 2.3 / 4.2 / 1.7, a luminance-based crush 1.7 / 2.9.
const CRUSH_KNEE: f64 = 0.3;

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
        // Iray exposes by the camera's shutter, f-number and ISO (∝ ISO /
        // (N² · S)); DAZ's "Exposure Value" is derived from them for show,
        // and only in a saved scene is it sure to agree. Stated as the EV
        // that gives the same exposure at ISO 100.
        let physical = match (ch.get("Shutter Speed"), ch.get("Aperture"), ch.get("Film ISO")) {
            (Some(s), Some(n), Some(iso)) => match (s.as_f64(), n.as_f64(), iso.as_f64()) {
                (Some(s), Some(n), Some(iso)) if s > 0.0 && n > 0.0 && iso > 0.0 => Some((n * n * s).log2() - (iso / 100.0).log2()),
                _ => None,
            },
            _ => None,
        };
        return ToneMapping {
            exposure_value: physical.unwrap_or_else(|| num("Exposure Value", 13.0)),
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
