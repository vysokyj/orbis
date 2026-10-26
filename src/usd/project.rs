//! Octane `project.ocs` from the bundled template.

use super::*;

/// Escape text for an XML attribute or element.
pub(super) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "&apos;").replace('"', "&quot;")
}

/// The Octane scene: `assets/octane/project.ocs` (saved by Octane X 2026.6:
/// project settings, a USD import graph and a render target in the Scene
/// graph wired to its geometry output) with the scene's values filled in.
/// The Octane project: one render target per camera, all wired to the same
/// geometry and sharing environment and kernel settings. Each target is a
/// copy of the template's, with its node ids moved up by a million per copy;
/// `env_texture` goes into every copy (its ids move along).
pub(super) fn project_ocs(
    scene_name: &str,
    usd_file: &str,
    cams: &[(String, SceneCamera)],
    (w, h): (u32, u32),
    env_texture: &str,
    mode: OctaneMode,
    tone: &ToneMapping,
) -> String {
    let v3 = |v: [f64; 3]| format!("{:.6} {:.6} {:.6} 0", v[0], v[1], v[2]);
    // The render target carries the scene's name — Octane offers it as the
    // file name when saving a render; the USD import graph gets a "(DAZ 3D)" suffix so
    // the two don't read alike in the outliner.
    let template = include_str!("../../assets/octane/project.ocs")
        .replace("@SCENE_NAME@", &xml_escape(&format!("{scene_name} (DAZ 3D)")))
        .replace("@USD_FILE@", &xml_escape(usd_file))
        .replace("@RESOLUTION@", &format!("{w} {h}"))
        .replace("@ENV_TEXTURE@", env_texture);
    // Kernel per mode; the imager follows DAZ's Iray tone mapping. Exposure
    // at DAZ's default EV 13: 1.4 under path tracing — measured against the
    // Iray render of Octane2 (HDRI only) with a Lua sweep, 2026-10-21 —
    // and 1 for direct lighting (calibrated on Octane1's lights).
    let (kernel, base_exposure, denoiser) = match mode {
        OctaneMode::Preview => (include_str!("../../assets/octane/kernel_direct_lighting.ocs"), 1.0, "0"),
        OctaneMode::Final => (include_str!("../../assets/octane/kernel_path_tracing.ocs"), 1.4, "1"),
    };
    let [wr, wg, wb] = tone.white_point;
    let template = template
        .replace("@KERNEL@\n", kernel)
        .replace("@EXPOSURE@", &format!("{:.6} 0 0 0", tone.octane_exposure(base_exposure)))
        .replace("@HIGHLIGHT_COMPRESSION@", &format!("{:.6} 0 0 0", (1.0 - tone.burn_highlights).clamp(0.0, 1.0)))
        .replace("@WHITE_BALANCE@", &format!("{wr:.6} {wg:.6} {wb:.6}"))
        .replace("@SATURATION@", &format!("{:.6} 0 0 0", tone.saturation))
        .replace("@DENOISER@", denoiser);
    let start = template.find("@SCENE_NODES@").expect("template has @SCENE_NODES@") + "@SCENE_NODES@".len();
    let end = template.rfind(" </graph>").expect("template ends with the Scene graph");
    let (head, rt, tail) = (&template[..start], &template[start..end], &template[end..]);
    let mut out = head.to_string();
    for (k, (label, cam)) in cams.iter().enumerate() {
        let name = if cams.len() > 1 { format!("{scene_name} – {label}") } else { scene_name.to_string() };
        let rt = rt
            .replace("@RT_NAME@", &xml_escape(&name))
            .replace("@CAM_POS@", &v3(cam.pos))
            .replace("@CAM_TARGET@", &v3(cam.target))
            .replace("@CAM_UP@", &v3(cam.up))
            .replace("@CAM_FOV@", &format!("{:.6} 0 0 0", cam.fov_deg))
            .replace("@CAM_AUTOFOCUS@", "0")
            .replace("@CAM_FOCAL_DEPTH@", &format!("{:.6} 0 0 0", cam.focal_depth))
            .replace("@CAM_APERTURE@", &format!("{:.6} 0 0 0", cam.aperture_cm))
            .replace("@CAM_FSTOP@", &format!("{:.6} 0 0 0", lens_fstop(cam.aperture_cm)));
        out.push_str(&offset_node_ids(&rt, k as u64 * 1_000_000));
    }
    out + tail
}

/// Add `delta` to every `id='N'` in `s` (node ids; `connect=` is untouched).
pub(super) fn offset_node_ids(s: &str, delta: u64) -> String {
    if delta == 0 {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 64);
    let mut rest = s;
    while let Some(i) = rest.find(" id='") {
        let (before, after) = rest.split_at(i + " id='".len());
        out.push_str(before);
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        match after[..digits].parse::<u64>() {
            Ok(n) => out.push_str(&(n + delta).to_string()),
            Err(_) => out.push_str(&after[..digits]),
        }
        rest = &after[digits..];
    }
    out.push_str(rest);
    out
}

/// The render target lens's focal length (mm, from the template).
const LENS_FOCAL_MM: f64 = 65.0;

/// The f-stop matching an aperture radius (cm) on the template's lens: Octane
/// keeps `fstop` beside `aperture`, and the template's 12 disagreed with every
/// exported aperture. No aperture (DOF off) → a pinhole.
fn lens_fstop(aperture_cm: f64) -> f64 {
    if aperture_cm > 0.0 { LENS_FOCAL_MM / (20.0 * aperture_cm) } else { 1000.0 }
}
