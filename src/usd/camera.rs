//! DAZ cameras → Octane render targets; framing camera and render size.

use super::*;

/// A camera for Octane's thin-lens camera node, in meters.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneCamera {
    pub pos: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    /// Horizontal field of view, degrees.
    pub fov_deg: f64,
    /// Focus distance, meters.
    pub focal_depth: f64,
    /// Lens aperture radius, centimeters (0 = everything sharp).
    pub aperture_cm: f64,
}

/// Three channel values (`x`/`y`/`z`), from the scene node's override or the
/// node definition.
pub(super) fn channel3(node: &Value, def: Option<&Value>, key: &str, default: f64) -> [f64; 3] {
    let arr = node.get(key).or_else(|| def.and_then(|d| d.get(key))).and_then(Value::as_array);
    let mut out = [default; 3];
    for c in arr.into_iter().flatten() {
        let i = match c.get("id").and_then(Value::as_str) {
            Some("x") => 0,
            Some("y") => 1,
            Some("z") => 2,
            _ => continue,
        };
        if let Some(v) = c.get("current_value").or_else(|| c.get("value")).and_then(Value::as_f64) {
            out[i] = v;
        }
    }
    out
}

pub(super) type Mat3 = [[f64; 3]; 3];

pub(super) fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut m = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

pub(super) fn mat_vec(a: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|i| (0..3).map(|k| a[i][k] * v[k]).sum())
}

/// DAZ Euler rotation (degrees). The order string lists the axes in the order
/// they apply: `XYZ` = `Rz · Ry · Rx` (checked against a scene's camera, which
/// only looks at its subject this way round).
pub(super) fn euler(deg: [f64; 3], order: &str) -> Mat3 {
    let mut m: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for axis in order.chars() {
        let (i, a) = match axis {
            'X' => (0, deg[0]),
            'Y' => (1, deg[1]),
            'Z' => (2, deg[2]),
            _ => continue,
        };
        let (s, c) = a.to_radians().sin_cos();
        let r: Mat3 = match i {
            0 => [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]],
            1 => [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]],
            _ => [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
        };
        m = mat_mul(&r, &m);
    }
    m
}

/// DAZ's default camera frame width, millimeters.
pub(super) const FRAME_WIDTH_MM: f64 = 36.0;

/// The scene's current camera (`scene.current_camera`); a scene saved while
/// looking through DAZ's default Perspective View has none, so its first
/// camera node is used instead.
#[cfg(test)]
pub(super) fn scene_camera(lib: &mut Library) -> Option<SceneCamera> {
    scene_cameras(lib).into_iter().next().map(|(_, c)| c)
}

/// Every camera node of the scene with its label, the current camera first,
/// the rest in scene order.
pub(super) fn scene_cameras(lib: &mut Library) -> Vec<(String, SceneCamera)> {
    let nodes = lib.duf.pointer("/scene/nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    let current = ref_id(lib.duf.pointer("/scene/current_camera"));
    let mut cams = Vec::new();
    for node in &nodes {
        let def = node.get("url").and_then(Value::as_str).and_then(|u| lib.entry(u, "node_library"));
        let id = node.get("id").and_then(Value::as_str).map(percent_decode).unwrap_or_default();
        let is_current = current.as_deref() == Some(id.as_str());
        let ty = def.as_ref().and_then(|d| d.get("type")).or_else(|| node.get("type")).and_then(Value::as_str);
        let is_camera = ty == Some("camera");
        if !is_camera && !is_current {
            continue;
        }
        let Some(cam) = camera_from_node(lib, node, def.as_ref()) else { continue };
        let label = node
            .get("label")
            .or_else(|| def.as_ref()?.get("label"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or(id);
        if is_current {
            cams.insert(0, (label, cam));
        } else {
            cams.push((label, cam));
        }
    }
    cams
}

pub(super) fn camera_from_node(lib: &Library, node: &Value, def: Option<&Value>) -> Option<SceneCamera> {
    let def = def.cloned();
    let id = node.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    if node.get("parent").is_some() {
        tracing::warn!(camera = %id, "USD export: camera is parented; its parent's transform is ignored");
    }
    let persp = node.get("perspective").or_else(|| def.as_ref()?.get("perspective"))?.clone();
    let t = channel3(&node, def.as_ref(), "translation", 0.0);
    let r = channel3(&node, def.as_ref(), "rotation", 0.0);
    let order = def
        .as_ref()
        .and_then(|d| d.get("rotation_order"))
        .or_else(|| node.get("rotation_order"))
        .and_then(Value::as_str)
        .unwrap_or("XYZ");
    let rot = euler(r, order);
    let pos = t.map(|v| v * CM as f64);
    let mut def_channels = def.as_ref().map(node_channels).unwrap_or_default();
    def_channels.extend(node_channels(&node));
    let focal_mm = def_channels
        .get("Focal Length")
        .and_then(Value::as_f64)
        .or_else(|| persp.get("focal_length").and_then(Value::as_f64))
        .unwrap_or(65.0);
    // The camera's channels rule (DAZ renders by them); `perspective`
    // mirrors them in a saved scene.
    let num = |ch: &str, key: &str, default: f64| {
        def_channels.get(ch).and_then(Value::as_f64).or_else(|| persp.get(key).and_then(Value::as_f64)).unwrap_or(default)
    };
    let focal_depth = num("Depth of Field", "focal_distance", 300.0) * CM as f64;
    let fwd = mat_vec(&rot, [0.0, 0.0, -1.0]);
    let dof = def_channels.get("DOF").and_then(Value::as_bool).or_else(|| persp.get("depth_of_field").and_then(Value::as_bool)).unwrap_or(false);
    // DAZ spells the f-stop channel "Aperature".
    let fstop = num("Aperature", "fstop", 22.0).max(0.1);
    Some(SceneCamera {
        pos,
        target: [0, 1, 2].map(|i| pos[i] + fwd[i] * focal_depth),
        up: mat_vec(&rot, [0.0, 1.0, 0.0]),
        fov_deg: horizontal_fov(&def_channels, focal_mm, render_size(&lib.duf)),
        focal_depth,
        // Iray's aperture radius is focal length (mm) / (2 · f-number) taken
        // as cm — 10× a real lens's: Gladiator's 65 mm f/6 blurs a trunk at
        // 5.3 m by ~15–20 px (15 px predicted, 1.5 px for a real lens).
        aperture_cm: if dof { focal_mm / (2.0 * fstop) } else { 0.0 },
    })
}

/// Octane's (horizontal) field of view for a DAZ camera. DAZ fits its frame
/// width (36 mm by default) to the image's *vertical* extent — the DSON
/// `yfov` — so a landscape render sees wider: tan(h/2) = tan(v/2) · aspect.
pub(super) fn horizontal_fov(channels: &HashMap<String, Value>, focal_mm: f64, (w, h): (u32, u32)) -> f64 {
    let frame = channels.get("Frame Width").and_then(Value::as_f64).filter(|f| *f > 0.0).unwrap_or(FRAME_WIDTH_MM);
    let half_v = (frame / 2.0 / focal_mm.max(0.1)).atan();
    let aspect = w as f64 / h.max(1) as f64;
    2.0 * (half_v.tan() * aspect).atan().to_degrees()
}

/// A front view (from +Z) that fits every mesh, for scenes without a camera.
pub(super) fn framing_camera(meshes: &[Mesh]) -> SceneCamera {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in meshes.iter().flat_map(|m| &m.points) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] as f64);
            hi[i] = hi[i].max(p[i] as f64);
        }
    }
    if lo[0] > hi[0] {
        (lo, hi) = ([-1.0; 3], [1.0; 3]);
    }
    let center = [0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.0);
    let size = (0..3).map(|i| (hi[i] - lo[i]).powi(2)).sum::<f64>().sqrt().max(0.1);
    let fov_deg = 2.0 * (FRAME_WIDTH_MM / 2.0 / 50.0f64).atan().to_degrees();
    let dist = size / 2.0 / (fov_deg / 2.0).to_radians().tan() * 1.1;
    SceneCamera {
        pos: [center[0], center[1], center[2] + dist],
        target: center,
        up: [0.0, 1.0, 0.0],
        fov_deg,
        focal_depth: dist,
        aperture_cm: 0.0,
    }
}

/// Render size from DAZ's render settings, else 1920×1080.
pub(super) fn render_size(duf: &Value) -> (u32, u32) {
    duf.pointer("/scene/extra")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|e| e.get("type").and_then(Value::as_str) == Some("studio_render_settings"))
        .and_then(|e| e.pointer("/render_options/imageSize")?.as_array().cloned())
        .and_then(|a| Some((a.first()?.as_u64()? as u32, a.get(1)?.as_u64()? as u32)))
        .filter(|&(w, h)| w > 0 && h > 0)
        .unwrap_or((1920, 1080))
}
