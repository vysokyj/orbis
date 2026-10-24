//! DAZ lights → UsdLux prims and Octane emitters / spots.

use super::*;

/// The emitting shape of a light, in meters.
#[derive(Debug, Clone, PartialEq)]
pub enum LightShape {
    /// A point (DAZ point/spot lights, Iray "Light Geometry" Point).
    Point,
    Sphere { radius: f64 },
    Disk { radius: f64 },
    Rect { width: f64, height: f64 },
    /// Parallel rays along the light's -Z (DAZ distant light).
    Distant,
}

/// A DAZ light for UsdLux.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneLight {
    pub label: String,
    /// Local → world (meters), rows = transformed X/Y/Z axes, then translation
    /// (USD row-vector convention).
    pub transform: [[f64; 4]; 4],
    pub shape: LightShape,
    /// Spotlight half angle, degrees.
    pub cone_deg: Option<f64>,
    pub color: [f64; 3],
    pub kelvin: Option<f64>,
    /// Luminous flux, lumens.
    pub lumens: f64,
    /// DAZ Euler rotation (degrees, `rotation_order`) and translation (m) —
    /// for Octane's own light nodes, which take Euler angles.
    pub euler_deg: [f64; 3],
    pub rotation_order: String,
    pub translation: [f64; 3],
}

/// Radius used for DAZ point lights (they have no size; UsdLux treats the
/// sphere as a point but needs it for the intensity).
pub(super) const POINT_RADIUS_M: f64 = 0.01;

/// Flux of a non-photometric DAZ light at 100 % intensity (no physical
/// meaning in DAZ; a bright bulb).
pub(super) const LEGACY_LUMENS: f64 = 15_000.0;

/// The scene's lights that DAZ renders: visible, switched on, illuminating.
pub(super) fn scene_lights(lib: &mut Library) -> Vec<SceneLight> {
    let nodes = lib.duf.pointer("/scene/nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for node in &nodes {
        let Some(def) = node.get("url").and_then(Value::as_str).and_then(|u| lib.entry(u, "node_library")) else { continue };
        if def.get("type").and_then(Value::as_str) != Some("light") {
            continue;
        }
        let mut ch = node_channels(&def);
        ch.extend(node_channels(node));
        let flag = |k: &str| ch.get(k).and_then(Value::as_bool);
        let num = |k: &str| ch.get(k).and_then(Value::as_f64);
        let on = node.get("on").or_else(|| def.get("on")).and_then(Value::as_bool).unwrap_or(true);
        if !on || flag("Visible") == Some(false) || num("Illumination") == Some(0.0) {
            continue;
        }
        let label = node.get("label").or_else(|| def.get("label")).and_then(Value::as_str).unwrap_or("Light").to_string();
        if node.get("parent").is_some() {
            tracing::warn!(light = %label, "USD export: light is parented; its parent's transform is ignored");
        }
        let t = channel3(node, Some(&def), "translation", 0.0).map(|v| v * CM as f64);
        let order = def.get("rotation_order").and_then(Value::as_str).unwrap_or("XYZ");
        let euler_deg = channel3(node, Some(&def), "rotation", 0.0);
        let r = euler(euler_deg, order);
        let mut transform = [[0.0; 4]; 4];
        for i in 0..3 {
            for j in 0..3 {
                transform[i][j] = r[j][i];
            }
            transform[3][i] = t[i];
        }
        transform[3][3] = 1.0;

        let cm = |k: &str| num(k).unwrap_or(10.0) * CM as f64;
        let shape = if def.get("directional").is_some() {
            LightShape::Distant
        } else {
            match num("Light Geometry").unwrap_or(0.0) as i64 {
                1 => LightShape::Rect { width: cm("Width"), height: cm("Height") },
                2 => LightShape::Disk { radius: cm("Height") / 2.0 },
                3 => LightShape::Sphere { radius: cm("Height") / 2.0 },
                _ => LightShape::Point,
            }
        };
        let cone_deg = def.get("spot").map(|spot| {
            num("Spread Angle")
                .map(|a| a / 2.0)
                .or_else(|| spot.get("falloff_angle").and_then(Value::as_f64))
                .unwrap_or(30.0)
        });
        let color = ch
            .get("Color")
            .or_else(|| def.get("color"))
            .and_then(Value::as_array)
            .map(|a| [0, 1, 2].map(|i| crate::daz_material::srgb_to_linear(a.get(i).and_then(Value::as_f64).unwrap_or(1.0) as f32) as f64))
            .unwrap_or([1.0; 3]);
        // Photometric (Iray) lights emit their Flux; Intensity and its scale
        // belong to the non-photometric mode.
        let photometric = flag("Photometric Mode").unwrap_or(false);
        let lumens = if photometric {
            num("Flux").unwrap_or(1500.0)
        } else {
            LEGACY_LUMENS * num("Intensity").unwrap_or(1.0) * num("Intensity Scale").unwrap_or(1.0)
        };
        out.push(SceneLight {
            label,
            transform,
            shape,
            cone_deg,
            color,
            kelvin: if photometric { num("Temperature") } else { None },
            lumens,
            euler_deg,
            rotation_order: order.to_string(),
            translation: t,
        });
    }
    out
}

/// Each light's power on DAZ's scale: watts at `DAZ_LUMENS_PER_WATT`, point
/// lights ×3.
pub(super) fn light_watts(l: &SceneLight) -> f64 {
    let point_factor = if matches!(l.shape, LightShape::Point) && l.cone_deg.is_none() { 3.0 } else { 1.0 };
    point_factor * l.lumens / crate::material::DAZ_LUMENS_PER_WATT
}

/// The UsdLux prim name of each light (as `write_lights` names them).
pub(super) fn light_prim_names(lights: &[SceneLight]) -> Vec<String> {
    let mut taken = HashSet::new();
    lights.iter().map(|l| unique_name(&mut taken, &l.label)).collect()
}

/// Octane emitter materials for the lights (see
/// `octane::material::light_inputs`).
pub(super) fn light_emitters(lights: &[SceneLight]) -> Vec<crate::material::LightEmitter> {
    lights
        .iter()
        .zip(light_prim_names(lights))
        .filter(|(l, _)| l.cone_deg.is_none())
        .map(|(l, prim)| crate::material::LightEmitter { prim, watts: light_watts(l), kelvin: l.kelvin, color: l.color })
        .collect()
}

/// DAZ spotlights as Octane analytic disk lights: UsdLux shaping is lost on
/// import (the emitter shines all around), Octane's own light keeps the cone.
pub(super) fn octane_spots(lights: &[SceneLight]) -> Vec<crate::material::OctaneSpot> {
    lights
        .iter()
        .filter_map(|l| {
            let cone = l.cone_deg?;
            let diameter = match l.shape {
                LightShape::Disk { radius } | LightShape::Sphere { radius } => 2.0 * radius,
                LightShape::Rect { width, height } => width.max(height),
                _ => 2.0 * POINT_RADIUS_M,
            };
            Some(crate::material::OctaneSpot {
                name: l.label.clone(),
                watts: light_watts(l),
                kelvin: l.kelvin,
                color: l.color,
                spread_deg: 2.0 * cone,
                diameter,
                euler_deg: l.euler_deg,
                rotation_order: l.rotation_order.clone(),
                translation: l.translation,
            })
        })
        .collect()
}

/// UsdLux prims for the lights. Intensity is the light's power in watts on
/// the scale DAZ scenes are lit for (`DAZ_LUMENS_PER_WATT`, point lights ×3),
/// with `normalize` so the emitter's size doesn't
/// change it.
pub(super) fn write_lights(o: &mut String, lights: &[SceneLight]) {
    use std::f64::consts::PI;
    if lights.iter().all(|l| l.cone_deg.is_some()) {
        return;
    }
    o.push_str("    def Scope \"Lights\"\n    {\n");
    for (l, name) in lights.iter().zip(light_prim_names(lights)) {
        if l.cone_deg.is_some() {
            continue; // an Octane analytic spot (see `octane_spots`)
        }
        let (kind, area, attrs) = match l.shape {
            LightShape::Point => {
                let r = POINT_RADIUS_M;
                ("SphereLight", 4.0 * PI * r * r, format!("float inputs:radius = {r}\n            bool treatAsPoint = 1\n"))
            }
            LightShape::Sphere { radius } => ("SphereLight", 4.0 * PI * radius * radius, format!("float inputs:radius = {radius}\n")),
            LightShape::Disk { radius } => ("DiskLight", PI * radius * radius, format!("float inputs:radius = {radius}\n")),
            LightShape::Rect { width, height } => (
                "RectLight",
                width * height,
                format!("float inputs:width = {width}\n            float inputs:height = {height}\n"),
            ),
            // Distant: illuminance (lux) — the flux over 1 m².
            LightShape::Distant => ("DistantLight", PI, String::new()),
        };
        let _ = area;
        let watts = light_watts(l);
        let schemas = if l.cone_deg.is_some() { "(\n        prepend apiSchemas = [\"ShapingAPI\"]\n    )" } else { "" };
        let _ = writeln!(o, "        def {kind} \"{name}\" {schemas}\n        {{");
        let _ = writeln!(o, "            custom string userProperties:dazLabel = {}", usd_string(&l.label));
        o.push_str("            ");
        o.push_str(&attrs.replace("\n            ", "\n            "));
        let _ = writeln!(o, "            float inputs:intensity = {watts:.6}\n            bool inputs:normalize = 1");
        let _ = writeln!(o, "            color3f inputs:color = ({}, {}, {})", l.color[0], l.color[1], l.color[2]);
        if let Some(k) = l.kelvin {
            let _ = writeln!(o, "            bool inputs:enableColorTemperature = 1\n            float inputs:colorTemperature = {k}");
        }
        if let Some(cone) = l.cone_deg {
            let _ = writeln!(o, "            float inputs:shaping:cone:angle = {cone}");
        }
        let mut m = l.transform;
        // Octane's USD import emits a rect or disk light along +Z, not
        // UsdLux's (and DAZ's) −Z: turned half round its X axis, it lights
        // where DAZ aims it (Calib-Area lit only the floor behind it).
        if matches!(l.shape, LightShape::Rect { .. } | LightShape::Disk { .. }) {
            for k in 0..3 {
                m[1][k] = -m[1][k];
                m[2][k] = -m[2][k];
            }
        }
        let rows: Vec<String> = m.iter().map(|r| format!("({}, {}, {}, {})", r[0], r[1], r[2], r[3])).collect();
        let _ = writeln!(o, "            matrix4d xformOp:transform = ({})", rows.join(", "));
        o.push_str("            uniform token[] xformOpOrder = [\"xformOp:transform\"]\n        }\n");
    }
    o.push_str("    }\n");
}
