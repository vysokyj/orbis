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
    hide_dome: bool,
    sun_sky: Option<&SunSky>,
) -> String {
    let v3 = |v: [f64; 3]| format!("{:.6} {:.6} {:.6} 0", v[0], v[1], v[2]);
    // The render target carries the scene's name — Octane offers it as the
    // file name when saving a render; the USD import graph gets a "(DAZ 3D)" suffix so
    // the two don't read alike in the outliner.
    let template = include_str!("../../assets/octane/project.ocs")
        .replace("@SCENE_NAME@", &xml_escape(&format!("{scene_name} (DAZ 3D)")))
        .replace("@USD_FILE@", &xml_escape(usd_file))
        .replace("@RESOLUTION@", &format!("{w} {h}"))
        .replace("@ENV_TEXTURE@", env_texture)
        .replace("@VISIBLE_ENVIRONMENT@", if hide_dome { BLACK_BACKPLATE } else { "" });
    // Iray's Sun-Sky: the render target's environment becomes Octane's
    // Daylight environment.
    let template = match sun_sky {
        Some(s) => {
            let start = template.find("     <pin name='environment'>\n").expect("render target environment");
            let end = start + template[start..].find("\n     </pin>\n").expect("environment pin end") + "\n     </pin>\n".len();
            format!("{}{}{}", &template[..start], daylight_environment(s), &template[end..])
        }
        None => template,
    };
    // Kernel per mode; the imager follows DAZ's Iray tone mapping. Exposure
    // at DAZ's default EV 13: 1.4 under path tracing — measured against the
    // Iray render of Octane2 (HDRI only) with a Lua sweep, 2026-10-21 —
    // and 1 for direct lighting (calibrated on Octane1's lights).
    let (kernel, base_exposure, denoiser) = match mode {
        OctaneMode::Preview => (include_str!("../../assets/octane/kernel_direct_lighting.ocs"), 1.0, "0"),
        OctaneMode::Final => (include_str!("../../assets/octane/kernel_path_tracing.ocs"), 1.4, "1"),
    };
    let [wr, wg, wb] = tone.white_point;
    // Iray's curve beyond the calibrated defaults (burn, crush, gamma) as a
    // custom LUT after the Gamma 2.2 response, the exposure lowered by its
    // headroom; Octane's highlight compression is far weaker than Iray's
    // burn (probe 91: 0.70× at 0.84 where Iray's burn 0 gives 0.54×).
    let (exposure, compression, lut) = if tone.is_plain() {
        (tone.octane_exposure(base_exposure), 0.0, String::new())
    } else {
        let lut = format!(
            "       <pin name='lut'>\n        <node id='100190' type='103'>\n         <attr name='filename' type='11'>{TONE_LUT_ASSET}</attr>\n        </node>\n       </pin>\n"
        );
        (tone.octane_exposure(base_exposure) / LUT_HEADROOM, 0.0, lut)
    };
    let template = template
        .replace("@KERNEL@\n", kernel)
        .replace("@EXPOSURE@", &format!("{:.6} 0 0 0", exposure))
        .replace("@HIGHLIGHT_COMPRESSION@", &format!("{:.6} 0 0 0", compression))
        .replace("@TONE_LUT@", &lut)
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

/// Octane's Daylight environment (14, probe 97) for Iray's Sun-Sky: the sun
/// direction given directly (`sundir`), Hosek-Wilkie with its ground from
/// the horizon down (`groundStartAngle` 0; the default 90 leaves a bright
/// sky below the horizon — 4.9 × Iray's ground under Preetham, which has
/// no ground: Calib-L-Mode-SunSky-off).
fn daylight_environment(s: &SunSky) -> String {
    let float = |id: u32, pin: &str, v: String| {
        format!("       <pin name='{pin}'>\n        <node id='{id}' type='6'>\n         <attr name='value' type='9'>{v}</attr>\n        </node>\n       </pin>\n")
    };
    let [x, y, z] = s.sun_dir;
    let [r, g, b] = s.ground;
    let mut out = String::from("     <pin name='environment'>\n      <node id='100210' type='14'>\n");
    out += &float(100211, "sundir", format!("{x:.6} {y:.6} {z:.6} 0"));
    out += &float(100212, "turbidity", format!("{:.6} 0 0 0", s.turbidity));
    out += &float(100213, "power", format!("{:.6} 0 0 0", s.power));
    out += &float(100214, "sunIntensity", format!("{:.6} 0 0 0", s.sun_intensity));
    out += "       <pin name='model'>\n        <node id='100215' type='57'>\n         <attr name='value' type='2'>3</attr>\n        </node>\n       </pin>\n";
    out += &float(100218, "groundStartAngle", "0 0 0 0".into());
    out += &float(100216, "sun_size", format!("{:.6} 0 0 0", s.sun_size));
    out += &float(100217, "groundColor", format!("{r:.6} {g:.6} {b:.6} 0"));
    out += "      </node>\n     </pin>\n";
    out
}

/// The render target's camera ("visible") environment when DAZ's dome isn't
/// drawn — the pin is `cameraEnvironment` (probe 96) — a black texture
/// environment as the backplate — the camera sees black, the
/// HDRI still lights the scene and shows in reflections, as in Iray.
const BLACK_BACKPLATE: &str = "     <pin name='cameraEnvironment'>
      <node id='100195' type='37'>
       <pin name='texture'>
        <node id='100196' type='33'>
         <attr name='value' type='8'>0 0 0</attr>
        </node>
       </pin>
       <pin name='visibleEnvironmentBackplate'>
        <node id='100197' type='11'>
         <attr name='value' type='1'>1</attr>
        </node>
       </pin>
      </node>
     </pin>
";
