//! VDB volumes: nodes whose material renders an OpenVDB file in Iray (DAZ's
//! "Simple OpenVDB MDL Volume" — `base::lookup_volume_coefficients` under
//! Iray Uber's inhomogeneous volume). The node's mesh only carries the
//! material; Iray fills the VDB's own extent, placed by the node.

use super::*;

/// A VDB volume as Iray renders it.
#[derive(Debug, Clone)]
pub struct Volume {
    pub label: String,
    /// The `.vdb` on disk.
    pub file: PathBuf,
    /// The density grid (the material's "Volume Channel", else the file's first).
    pub grid: String,
    /// World transform (DAZ's `DzMatrix3` rows, cm): VDB world units are
    /// scene units, cm.
    pub transform: dump::Affine,
    /// Iray's coefficients per unit of grid density: the multipliers ×
    /// "Density Multiplier" (÷ the node's scale when "Density Relative To
    /// Size"), in Iray's inverse scene units.
    pub scattering: [f32; 3],
    pub absorption: [f32; 3],
    /// "SSS Direction": the phase function's g (Iray applies it — 0
    /// rendered Calib-VDB's haze 8–30 % brighter than DTAS's 0.85).
    pub anisotropy: f32,
}

/// Whether a scene material renders a VDB (its "Volume File" is set).
pub(super) fn is_volume(m: &crate::daz_material::Material) -> bool {
    volume_file(m).is_some()
}

fn volume_file(m: &crate::daz_material::Material) -> Option<&str> {
    m.image("Volume File").or_else(|| m.channel("Volume File")?.value.as_str()).filter(|s| !s.is_empty())
}

/// The VDB volumes of the scene's visible nodes; what can't be exported
/// goes into `report.skipped`.
pub(super) fn scene_volumes(lib: &mut Library, dump: &SceneDump, report: &mut ExportReport) -> Vec<Volume> {
    let mut out = Vec::new();
    for n in mesh::geo_nodes(&lib.duf) {
        let mats = mesh::scene_materials(lib, &n.instance_id);
        let Some(m) = mats.values().find(|m| is_volume(m)) else { continue };
        if !n.visible {
            continue;
        }
        let path = volume_file(m).unwrap_or_default();
        let rel = path.trim_start_matches('/');
        let Some(file) = lib.dirs.iter().map(|d| d.join(rel)).find(|p| p.is_file()) else {
            report.skipped.push(format!("{}: VDB volume {path} not found", n.label));
            continue;
        };
        let Some(transform) = dump.nodes.iter().find(|f| f.label == n.label || f.name == n.name).and_then(|f| f.transform) else {
            report.skipped.push(format!("{}: VDB volume not in the scene dump", n.label));
            continue;
        };
        let grids = match crate::vdb::grids(&file) {
            Ok(g) => g,
            Err(e) => {
                report.skipped.push(format!("{}: VDB volume unreadable ({e:#})", n.label));
                continue;
            }
        };
        let named = m.channel("Volume Channel").and_then(|c| c.value.as_str()).filter(|s| !s.is_empty());
        let Some(grid) = named.map(str::to_string).or_else(|| grids.first().map(|g| g.name.clone())) else {
            report.skipped.push(format!("{}: VDB volume without grids", n.label));
            continue;
        };
        // "Density Relative To Size" (MDL: × transform_scale(object,
        // world)) divides by the node's uniform scale in Iray: Calib-VDB's
        // haze (scale 0.1) at density 1000 renders as thick as 10000
        // without it — the optical depth stays the node's at scale 1.
        let t = &transform;
        let det = t[0] * (t[5] * t[10] - t[6] * t[9]) - t[1] * (t[4] * t[10] - t[6] * t[8]) + t[2] * (t[4] * t[9] - t[5] * t[8]);
        let scale = det.abs().cbrt() as f32;
        let relative = m.float("Density Relative To Size").unwrap_or(0.0) > 0.0 && scale > 0.0;
        let density = m.float("Density Multiplier").unwrap_or(1.0) / if relative { scale } else { 1.0 };
        // The multipliers are colours, decoded as Iray decodes them: raw,
        // Calib-VDB-tint's (1, 0.6, 0.3) haze came out too grey and DTAS's
        // absorption 0.09 (0.0086 decoded) too dark.
        let scattering = m.color("Scattering Multiplier").unwrap_or([0.0; 3]).map(|c| c * density);
        let absorption = m.color("Absorption Multiplier").unwrap_or([0.0; 3]).map(|c| c * density);
        if m.float("Emission Enable").unwrap_or(0.0) > 0.0 {
            report.skipped.push(format!("{}: VDB emission (not exported)", n.label));
        }
        let anisotropy = m.float("SSS Direction").unwrap_or(0.0).clamp(-1.0, 1.0);
        out.push(Volume { label: n.label.clone(), file, grid, transform, scattering, absorption, anisotropy });
    }
    out
}
