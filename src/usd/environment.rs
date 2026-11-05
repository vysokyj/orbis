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
    /// Environment Tint, linear (`crate::iray::dome_setup`).
    pub tint: [f64; 3],
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
    /// Iray's ground fog (`crate::iray::ground_fog`): slabs of fog volume
    /// (`ground_fog_slabs`).
    pub ground_fog: Option<crate::iray::GroundFog>,
    /// Dome Mode "Infinite Sphere w/ Ground" (and the finite ones with a
    /// ground): the HDRI's lower half projected onto the ground from this
    /// high above it — Ground Texture Scale, meters (Iray's
    /// `environment_dome_ground_texturescale`).
    pub dome_ground: Option<f64>,
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
    /// Octane's daylight model: 3 Hosek-Wilkie (warms a low sun), 1 Octane
    /// with a white sunset colour (`Tuning::daylight_model`).
    pub model: u8,
}

impl SunSky {
    /// The variant's daylight model and power (`Tuning::daylight_model`,
    /// `daylight_power`, `daylight_extinction`); read from the scene at
    /// Hosek-Wilkie's.
    pub fn tune(&mut self, t: &crate::tuning::Tuning) {
        self.model = t.daylight_model;
        let air_mass = 1.0 / self.sun_dir[1].max(0.05);
        self.power *= t.daylight_power / SUN_SKY_POWER * (-t.daylight_extinction * air_mass).exp();
    }
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

/// A horizontal slab of uniform fog: heights (m) and Iray's extinction
/// (per meter, per channel) there.
#[derive(Debug, Clone, PartialEq)]
pub struct FogSlab {
    pub y0: f64,
    pub y1: f64,
    pub extinction: [f64; 3],
}

/// Iray's ground fog is unbounded; Octane's volumes are boxes. Their half
/// width around the camera and the depth of the uniform part below
/// `height0`, meters.
pub const FOG_RADIUS: f64 = 1000.0;
/// The slabs that stand in for the exponential decay: each `FOG_SLAB_DECAYS`
/// decay lengths thick (at most `FOG_SLABS`), up to where the density is
/// `FOG_END` of the ground's. Fewer, thicker slabs rendered both closer to
/// Iray and faster than many thin ones (Calib-Env-GroundFog: four of two
/// decays floor 1.06 / below the plane 1.00 in 321 s, sixteen of half a
/// decay 1.12 / 0.96 in 450 s); the width doesn't count.
pub const FOG_SLAB_DECAYS: f64 = 2.0;
pub const FOG_SLABS: usize = 16;
pub const FOG_END: f64 = 1e-3;

/// Iray's ground fog as slabs: below `height0` its density `density0`
/// (`FOG_RADIUS` deep), above it the exponential decay (Iray manual 10.6,
/// per channel) as slabs at their mean density — a uniform fog (equal
/// densities) one slab `FOG_RADIUS` high.
pub fn ground_fog_slabs(fog: &crate::iray::GroundFog) -> Vec<FogSlab> {
    let mut out = vec![FogSlab { y0: fog.height0 - FOG_RADIUS, y1: fog.height0, extinction: fog.density0 }];
    let decay = fog.decay();
    if !decay.is_finite() {
        out.push(FogSlab { y0: fog.height0, y1: fog.height0 + FOG_RADIUS, extinction: fog.density0 });
        return out;
    }
    if decay <= 0.0 {
        return out;
    }
    let span = decay * (1.0 / FOG_END).ln();
    let n = ((span / (decay * FOG_SLAB_DECAYS)).ceil() as usize).clamp(1, FOG_SLABS);
    let t = span / n as f64;
    // Each channel's own decay; its mean over [a, b] above height0.
    let channel = |k: usize| {
        let (d0, d1) = (fog.density0[k], fog.density1[k].min(fog.density0[k]));
        let r = if d0 > 0.0 && d1 > 0.0 { (d1 / d0).ln() } else { f64::NEG_INFINITY };
        (d0, if r == 0.0 { f64::INFINITY } else { (fog.height0 - fog.height1) / r })
    };
    for i in 0..n {
        let (a, b) = (i as f64 * t, (i + 1) as f64 * t);
        let extinction = [0, 1, 2].map(|k| {
            let (d0, l) = channel(k);
            if !l.is_finite() { d0 } else if l <= 0.0 { 0.0 } else { d0 * l * ((-a / l).exp() - (-b / l).exp()) / (b - a) }
        });
        out.push(FogSlab { y0: fog.height0 + a, y1: fog.height0 + b, extinction });
    }
    out
}

impl SceneEnvironment {
    /// The lat-long map's U shift that turns Octane's texture environment like
    /// the DAZ dome: `0.25 + (rotation − orientation Y) / 360`, wrapped to
    /// [0, 1) (`Ry(-rot) · Ry(orient y)`; offset and sign
    /// measured in Octane X against the Blender render at 0° and 110°). The
    /// image's transform is a UV transform there — a 3D rotation does
    /// nothing — so a tilted dome takes orientation Y along into
    /// `dome_tilt`.
    pub fn dome_u_shift(&self) -> f64 {
        let oy = if self.dome_tilt().is_some() { 0.0 } else { self.orientation_deg[1] };
        (0.25 + (self.rotation_deg - oy) / 360.0).rem_euclid(1.0)
    }

    /// The rotation (degrees X, Y, Z, Octane order ZXY) of a Spherical
    /// projection that tilts the dome when Dome Orientation X or Z isn't 0:
    /// Iray's `environment_function_orientation` Rz · Rx · Ry as it is —
    /// the projection turns the lookup direction (probe 110: projection Y
    /// +20° = U shift −20°; Calib-L-Mi-Dome-turned sky correlation 0.95).
    pub fn dome_tilt(&self) -> Option<[f64; 3]> {
        let [x, _, z] = self.orientation_deg;
        (x != 0.0 || z != 0.0).then_some(self.orientation_deg)
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
        // What DAZ hands Iray (`crate::iray::environment`, verified against
        // its .mi export).
        let iray = crate::iray::environment(&ch);
        let setup = crate::iray::dome_setup(&ch);
        let dome = match iray {
            crate::iray::Environment::Dome { intensity } => intensity / crate::iray::DOME_INTENSITY_SCALE,
            _ => num("Environment Intensity").unwrap_or(1.0) * num("Environment Map").unwrap_or(1.0),
        };
        return Some(SceneEnvironment {
            map,
            // Intensity × the map channel's own value.
            intensity: dome,
            rotation_deg: setup.rotation_turns * 360.0,
            tint: setup.tint,
            orientation_deg: ["X", "Y", "Z"].map(|a| num(&format!("Dome Orientation {a}")).unwrap_or(0.0)),
            scene_only: num("Environment Mode") == Some(3.0),
            ground_shadow: (setup.ground && num("Environment Mode") != Some(3.0)).then_some(setup.ground_shadow),
            ground_origin_y: (num("Ground Position Mode") == Some(1.0)).then_some(setup.ground_y),
            draw_dome: ch.get("Draw Dome").and_then(Value::as_bool).unwrap_or(true),
            scene_lights: !matches!(num("Environment Mode").map(|m| m as i64), Some(1 | 2)),
            sun_sky: match &iray {
                crate::iray::Environment::SunSky { args, ground_color } => Some(SunSky {
                    sun_dir: steered.unwrap_or_else(|| {
                        sun_direction(
                            num("SS Latitude").unwrap_or(40.76426),
                            num("SS Longitude").unwrap_or(-111.8972),
                            num("SS Day").unwrap_or(2_457_092.0),
                            num("SS Time").unwrap_or(43_200.0),
                            num("SS UTC Offset").unwrap_or(-6.0),
                        )
                    }),
                    turbidity: (2.0 + 2.0 * args["haze"]).clamp(2.0, 10.0),
                    power: SUN_SKY_POWER * args["multiplier"] / 0.10132 * haze_dimming(args["haze"]),
                    sun_intensity: SUN_SKY_SUN_SHARE * args["sun_disk_intensity"],
                    sun_size: args["sun_disk_scale"] / 4.0,
                    model: 3,
                    ground: *ground_color,
                }),
                _ => None,
            },
            ground_fog: crate::iray::ground_fog(&ch),
            dome_ground: { let d = crate::iray::dome_shape(&ch); d.projected_ground().then_some(d.ground_texture_scale * f64::from(CM)) },
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
    /// The colour that maps to white, linear (Iray's `mip_whitepoint`:
    /// `White Point`^2.2 × `White Point Scale`).
    pub white_point: [f64; 3],
    /// 1 = highlights left alone, 0 = fully compressed.
    pub burn_highlights: f64,
    /// Darkens the shadows (0 = off).
    pub crush_blacks: f64,
    pub saturation: f64,
    pub gamma: f64,
    /// Iray's `mip_vignetting`: linear values × cos(θ)^v, θ the ray's angle
    /// to the camera axis (0 = off).
    pub vignetting: f64,
    /// Burn per channel (Iray's default); off, the burn compresses the
    /// luminance and keeps the chromaticity.
    pub burn_per_component: bool,
}

impl Default for ToneMapping {
    fn default() -> Self {
        ToneMapping { exposure_value: 13.0, cm2_factor: 1.0, white_point: [1.0; 3], burn_highlights: 1.0, crush_blacks: 0.0, saturation: 1.0, gamma: 2.2, vignetting: 0.0, burn_per_component: true }
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
        self.after_burn(self.burn(c))
    }

    /// Iray's burn: c (1 + c b) / (1 + c).
    fn burn(&self, c: f64) -> f64 {
        let b = self.burn_highlights.clamp(0.0, 1.0);
        let c = c.max(0.0);
        c * (1.0 + c * b) / (1.0 + c)
    }

    /// Iray's crush and gamma on a burnt value.
    fn after_burn(&self, burnt: f64) -> f64 {
        let crush = self.crush_blacks.max(0.0);
        let crushed = if crush > 0.0 { burnt * burnt / (burnt + CRUSH_KNEE * crush) } else { burnt };
        crushed.powf(1.0 / self.gamma.max(0.1))
    }

    /// Octane's imager vignetting for Iray's cos(θ)^v through a camera of
    /// this horizontal field of view and aspect (w / h). Octane darkens by
    /// 1 − v_o · e, e = √(x² + y²) / √2 on the frame's coordinates (each
    /// −1 … 1, the corner 1; Calib-Tone-Default at v_o 0.25 / 0.5 / 1: slope
    /// 1.00 v_o, power of e 1.00, the same at FOV 40° and 69°) — a cone in
    /// frame space, where Iray's falloff is round and follows the FOV. v_o
    /// is the least-squares fit over the frame (Calib-Tone, 69°, 16:9: Iray
    /// 0.5 → 0.086, 1 → 0.165).
    pub fn octane_vignetting(&self, fov_deg: f64, aspect: f64) -> f64 {
        if self.vignetting <= 0.0 {
            return 0.0;
        }
        let t = (fov_deg.clamp(0.0, 179.0) / 2.0).to_radians().tan();
        let (n, aspect) = (64, aspect.max(1e-3));
        let (mut num, mut den) = (0.0, 0.0);
        for j in 0..n {
            for i in 0..n {
                let (x, y) = ((i as f64 + 0.5) / n as f64 * 2.0 - 1.0, (j as f64 + 0.5) / n as f64 * 2.0 - 1.0);
                let e = (x * x + y * y).sqrt() / std::f64::consts::SQRT_2;
                let cos = 1.0 / (1.0 + (x * x + (y / aspect).powi(2)) * t * t).sqrt();
                num += e * (1.0 - cos.powf(self.vignetting));
                den += e * e;
            }
        }
        (num / den).clamp(0.0, 1.0)
    }

    /// Iray's white point correction as per-channel gains on the linear
    /// colour: k / wp (`white_point`, linear), k = 1 / Σ yᵢ / wpᵢ with the
    /// CIE 1931 RGB luminance weights y — the white point's hue maps to
    /// grey and a white pixel keeps that luminance (#70). Iray over
    /// Calib-Tone-Default, per-pixel medians: k 0.651 (WhiteWarm), 0.531
    /// (WhiteCool), 0.945 (WhiteGreen); this gives 0.651 / 0.531 / 0.944.
    /// The mean of wp (0.646 / 0.646 / 0.750) and its Rec. 709 luminance
    /// (0.674 / 0.579 / 0.917) miss. A grey white point at any scale: 1, 1, 1.
    pub fn white_gains(&self) -> [f64; 3] {
        let lin = self.white_point.map(|c| c.max(1e-6));
        let k = 1.0 / (0..3).map(|i| CIE_RGB_Y[i] / lin[i]).sum::<f64>();
        lin.map(|l| k / l)
    }

    /// Whether the white point is grey (its gains all 1).
    fn neutral_white(&self) -> bool {
        self.white_gains().iter().all(|g| (g - 1.0).abs() < 1e-6)
    }

    /// Iray's tone mapping on a linear colour (after exposure): the white
    /// point's gains (`white_gains`), then the curve per channel, or with
    /// Burn Highlights Per Component off the burn of its Rec. 709
    /// luminance Y as c · burn(Y) / Y, then crush and gamma per channel
    /// (Calib-Tone-BurnComp-off predicted from -on's pixels within 0.44 /
    /// 255 where the two differ by 3.1; Rec. 601 weights do as well, the
    /// mean or the maximum of the channels 0.7 / 2.7).
    pub fn iray_rgb(&self, c: [f64; 3]) -> [f64; 3] {
        let g = self.white_gains();
        let c = [0, 1, 2].map(|i| c[i].max(0.0) * g[i]);
        if !self.luminance_burn() {
            return c.map(|v| self.iray_curve(v));
        }
        let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let k = if y > 0.0 { self.burn(y) / y } else { 1.0 };
        c.map(|v| self.after_burn(v * k))
    }

    /// Whether the burn goes by luminance (per component off and a burn
    /// to do): Octane's imager then needs a 3D LUT.
    fn luminance_burn(&self) -> bool {
        !self.burn_per_component && self.burn_highlights < 1.0
    }

    /// Whether Octane's own imager reproduces this: the calibrated defaults
    /// (Burn 1, Crush 0, Gamma 2.2 — the Gamma 2.2 response) and a grey
    /// white point (Octane's white balance is a chromatic adaptation, not
    /// Iray's per-channel division, #70).
    pub fn is_plain(&self) -> bool {
        self.burn_highlights >= 1.0 && self.crush_blacks <= 0.0 && (self.gamma - 2.2).abs() < 1e-3 && self.neutral_white()
    }

    /// Iray's curve as a 1D `.cube` LUT for Octane's imager: its input is
    /// the Gamma 2.2 response of c / `LUT_HEADROOM` (the exposure is
    /// divided by it, so highlights up to 16× white survive the response's
    /// clip at 1). Octane applies response, gamma, then LUT (`order` 0).
    /// The white point's gains are in it, per channel.
    /// A luminance burn needs the channels together: a 3D LUT of
    /// `LUT_3D_SIZE`³ entries (red fastest, as `.cube` orders them), not
    /// clipped at 1 — a clip inside a cell bends what the interpolation
    /// reads by up to 4 / 255 (48³), the display clips after it.
    pub fn cube_lut(&self) -> String {
        if self.luminance_burn() {
            let n = LUT_3D_SIZE;
            let mut out = format!("TITLE \"ORBIS Iray tone mapping\"\nLUT_3D_SIZE {n}\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\n");
            let lin = |i: usize| (i as f64 / (n - 1) as f64).powf(2.2) * LUT_HEADROOM;
            for b in 0..n {
                for g in 0..n {
                    for r in 0..n {
                        let [r, g, b] = self.iray_rgb([lin(r), lin(g), lin(b)]);
                        let _ = writeln!(out, "{r:.6} {g:.6} {b:.6}");
                    }
                }
            }
            return out;
        }
        let mut out = format!("TITLE \"ORBIS Iray tone mapping\"\nLUT_1D_SIZE {LUT_SIZE}\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\n");
        for i in 0..LUT_SIZE {
            let x = i as f64 / (LUT_SIZE - 1) as f64;
            let [r, g, b] = self.iray_rgb([x.powf(2.2) * LUT_HEADROOM; 3]).map(|v| v.clamp(0.0, 1.0));
            let _ = writeln!(out, "{r:.6} {g:.6} {b:.6}");
        }
        out
    }
}

/// Iray's Crush Blacks pulls each channel down as x² / (x + k · crush):
/// fitted on Calib-Tone's Crush 0.1 / 0.3 against its Crush 0 render (same
/// samples, so the same noise) — 1.4 / 2.4 / 255, Gladiator's mix 1.2;
/// `^(1 + crush)` gave 2.3 / 4.2 / 1.7, a luminance-based crush 1.7 / 2.9.
const CRUSH_KNEE: f64 = 0.3;

/// The luminance Iray's white point keeps: the CIE 1931 RGB weights
/// (Y row of CIE RGB → XYZ). Fitted free on Calib-Tone-WhiteWarm, -Cool and
/// -Green as k = 1 / Σ wᵢ / wpᵢ, the three weights came out 0.177 0.813
/// 0.010, summing to 1.000 (#70).
const CIE_RGB_Y: [f64; 3] = [0.17697, 0.81240, 0.01063];

/// Entries of the tone LUT.
const LUT_SIZE: usize = 4096;

/// Entries per axis of the 3D tone LUT (a luminance burn). The response
/// makes Iray's curve nearly linear in the LUT's input, so trilinear
/// interpolation stays within 0.2 / 255 (`luminance_burn_lut`).
const LUT_3D_SIZE: usize = 33;

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
        // What DAZ hands Iray (`crate::iray::tone`, verified against its
        // .mi export).
        let iray = crate::iray::tone(&ch);
        let p = |k: &str| iray[k];
        // Iray exposes by the camera's shutter, f-number and ISO (∝ ISO /
        // (N² · S)), not by DAZ's "Exposure Value", which is derived from
        // them for show. Stated as the EV that gives the same exposure at
        // ISO 100.
        let (s, n, iso) = (p("mip_camera_shutter"), p("mip_f_number"), p("mip_film_iso"));
        // A scene without those channels (hand-made) keeps its EV.
        let saved = ["Shutter Speed", "Aperture", "Film ISO"].iter().all(|k| ch.contains_key(*k));
        let physical = (saved && s > 0.0 && n > 0.0 && iso > 0.0).then(|| (n * n * s).log2() - (iso / 100.0).log2());
        return ToneMapping {
            exposure_value: physical.unwrap_or_else(|| ch.get("Exposure Value").and_then(Value::as_f64).unwrap_or(13.0)),
            cm2_factor: p("mip_cm2_factor"),
            white_point: crate::iray::white_point(&ch),
            burn_highlights: p("mip_burn_highlights"),
            crush_blacks: p("mip_crush_blacks"),
            saturation: p("mip_saturation"),
            gamma: p("mip_gamma"),
            vignetting: p("mip_vignetting"),
            burn_per_component: crate::iray::burn_per_component(&ch),
        };
    }
    ToneMapping::default()
}
