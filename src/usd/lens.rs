//! Iray's lens options (#39) on Octane's cameras: lens shift and the
//! aperture blades on the thin lens; Iray's distortion models as an OSL
//! camera, which writes them out exactly.

use super::camera::SceneCamera;
use crate::iray::Distortion;

/// The thin lens's `lensShift`: Iray's shift (cm on the film) as a fraction
/// of the image's width (x) and height (y) — Octane's units (probe: 0.1 moves
/// a 1920×1080 frame 192 px across, 108 px up).
pub(super) fn thin_lens_shift(cam: &SceneCamera) -> [f64; 2] {
    let [w, h] = cam.film_cm;
    [cam.lens.shift[0] / w.max(1e-6), cam.lens.shift[1] / h.max(1e-6)]
}

/// The thin lens's bokeh (`bokehSidecount`, `bokehRotation` in degrees,
/// `bokehRoundedness`): Aperture Blades > 2 make a polygon of that many
/// sides, turned by the blade rotation; otherwise Iray's lens is round
/// (roundedness 1; the side count then doesn't matter).
pub(super) fn bokeh(cam: &SceneCamera) -> (i64, f64, f64) {
    let l = &cam.lens;
    if l.blades > 2 { (l.blades, l.blade_rotation.to_degrees(), 0.0) } else { (6, 0.0, 1.0) }
}

/// A float as an OSL literal.
fn f(v: f64) -> String {
    let s = format!("{v:.9}");
    let s = s.trim_end_matches('0');
    if s.ends_with('.') { format!("{s}0") } else { s.to_string() }
}

/// The distortion model's polynomial in `r` (Iray manual 13.3): for the
/// forward models rd = P(ru), for the inverse ones ru = P(rd), for the
/// equidistant fisheye the exit angle θ = P(rd).
fn polynomial(d: Distortion, k: [f64; 5], r: &str) -> Option<String> {
    let [k1, k2, k3, k4, k5] = k.map(f);
    Some(match d {
        Distortion::Poly3 | Distortion::InvPoly3 => format!("{r} * (1.0 - {k1} + {k1} * {r} * {r})"),
        Distortion::Poly5 | Distortion::InvPoly5 => format!("{r} * (1.0 + {k1} * {r} * {r} + {k2} * {r} * {r} * {r} * {r})"),
        Distortion::PtLens | Distortion::InvPtLens => {
            format!("{r} * (1.0 - {k1} - {k2} - {k3} + {k1} * {r} + {k2} * {r} * {r} + {k3} * {r} * {r} * {r})")
        }
        Distortion::Brown | Distortion::InvBrown => {
            format!("{r} * ({k1} + {k2} * {r} + {k3} * {r} * {r} + {k4} * {r} * {r} * {r} + {k5} * {r} * {r} * {r} * {r})")
        }
        Distortion::Equidistant => format!("({k1} + {k2} * {r} + {k3} * {r} * {r} + {k4} * {r} * {r} * {r} + {k5} * {r} * {r} * {r} * {r})"),
        Distortion::None | Distortion::Spherical | Distortion::Cylindrical => return None,
    })
}

/// An OSL camera shader for a camera with Iray lens distortion; `None`
/// without one (the thin lens covers the rest).
///
/// The image point is taken on Iray's film (cm, the lens shift added), its
/// radius normalised as `scale · distance / film width` — the normalisation
/// the manual's equidistant table implies (cropped fisheye: k2 = θ / s for a
/// horizontal field of view θ). The perspective models bend the ray through
/// the undistorted film point (the forward ones solved for it by Newton
/// steps); spherical and cylindrical are full 360° panoramas (spherical
/// 180° high, cylindrical with square pixels). Depth of field samples
/// Octane's lens random numbers over Iray's lens: a disc, or the blade
/// polygon; focus on the plane at the focus distance (perspective) or the
/// sphere (the rest).
pub(super) fn osl_camera(cam: &SceneCamera) -> Option<String> {
    let l = &cam.lens;
    if l.distortion == Distortion::None {
        return Option::None;
    }
    let mut body = String::new();
    let film = format!(
        "    int res[2];\n    getattribute(\"camera:resolution\", res);\n    float W = {};\n    float H = W * res[1] / res[0];\n    float F = {};\n    float xf = (u - 0.5) * W + {};\n    float yf = (v - 0.5) * H + {};\n    float rd = {} * hypot(xf, yf) / W;\n    vector d = vector(xf, yf, -F);\n    int planar = 1;\n",
        f(cam.film_cm[0]),
        f(cam.focal_cm),
        f(l.shift[0]),
        f(l.shift[1]),
        f(l.distortion_scale)
    );
    body.push_str(&film);
    use Distortion::*;
    match l.distortion {
        Poly3 | Poly5 | PtLens | Brown => {
            let p = polynomial(l.distortion, l.k, "ru")?;
            let ph = polynomial(l.distortion, l.k, "(ru + 1e-4)")?;
            body.push_str(&format!(
                "    float ru = rd;\n    for (int i = 0; i < 16; i++) {{\n        float g = {p} - rd;\n        float dg = ({ph} - {p}) / 1e-4;\n        if (dg != 0.0) ru -= g / dg;\n    }}\n    if (rd > 0.0) d = vector(xf * ru / rd, yf * ru / rd, -F);\n"
            ));
        }
        InvPoly3 | InvPoly5 | InvPtLens | InvBrown => {
            let p = polynomial(l.distortion, l.k, "rd")?;
            body.push_str(&format!("    float ru = {p};\n    if (rd > 0.0) d = vector(xf * ru / rd, yf * ru / rd, -F);\n"));
        }
        Equidistant => {
            let p = polynomial(l.distortion, l.k, "rd")?;
            body.push_str(&format!(
                "    float theta = {p};\n    if (theta > {}) tMax = 0.0;\n    float phi = atan2(yf, xf);\n    d = vector(sin(theta) * cos(phi), sin(theta) * sin(phi), -cos(theta));\n    planar = 0;\n",
                f(l.max_fov / 2.0)
            ));
        }
        Spherical => body.push_str(
            "    float lon = (u - 0.5) * M_2PI;\n    float lat = (v - 0.5) * M_PI;\n    d = vector(sin(lon) * cos(lat), sin(lat), -cos(lon) * cos(lat));\n    planar = 0;\n",
        ),
        Cylindrical => body.push_str("    float lon = (u - 0.5) * M_2PI;\n    d = vector(sin(lon), (v - 0.5) * M_2PI * H / W, -cos(lon));\n    planar = 0;\n"),
        None => return Option::None,
    }
    // Depth of field: the lens point (m), the ray through the focus point.
    let radius_m = cam.aperture_cm / 100.0;
    if radius_m > 0.0 {
        let lens = if l.blades > 2 {
            format!(
                "    float a = dof[0] * {n};\n    float i = floor(a);\n    float s = a - i;\n    float q = sqrt(dof[1]);\n    float a0 = {rot} + i * M_2PI / {n};\n    float a1 = a0 + M_2PI / {n};\n    float lx = {r} * q * ((1.0 - s) * cos(a0) + s * cos(a1));\n    float ly = {r} * q * ((1.0 - s) * sin(a0) + s * sin(a1));\n",
                n = f(l.blades as f64),
                rot = f(l.blade_rotation + BLADE_ANGLE0),
                r = f(radius_m)
            )
        } else {
            format!("    float q = {} * sqrt(dof[1]);\n    float lx = q * cos(M_2PI * dof[0]);\n    float ly = q * sin(M_2PI * dof[0]);\n", f(radius_m))
        };
        body.push_str(&format!(
            "    float dof[2];\n    getattribute(\"camera:dofrandom\", dof);\n{lens}    vector fp = planar ? d * ({fd} / F) : normalize(d) * {fd};\n    pos = transform(\"camera\", \"world\", point(lx, ly, 0.0));\n    dir = transform(\"camera\", \"world\", fp - vector(lx, ly, 0.0));\n",
            fd = f(cam.focal_depth)
        ));
    } else {
        body.push_str("    pos = transform(\"camera\", \"world\", point(0.0, 0.0, 0.0));\n    dir = transform(\"camera\", \"world\", d);\n");
    }
    Some(format!(
        "// ORBIS: Iray lens distortion \"{}\" (#39).\nshader OrbisIrayLens(output point pos = 0, output vector dir = 0, output float tMax = 1.0 / 0.0)\n{{\n{body}}}\n",
        l.distortion.mi_name()
    ))
}

/// The angle of the blade polygon's first corner at rotation 0, radians:
/// the thin lens's convention, so the OSL camera's polygon sits as Octane's.
pub(super) const BLADE_ANGLE0: f64 = std::f64::consts::FRAC_PI_2;

/// The OSL camera node for the render target's `camera` pin (Octane's
/// NT_CAM_OSL, 126): the shader code plus the position the thin lens would
/// get. `@CAM_…@` placeholders are filled in like the thin lens's.
pub(super) fn osl_camera_node(code: &str) -> String {
    format!(
        "     <pin name='camera'>
      <node id='100005' type='126'>
       <attr name='shaderCode' type='10'>{}</attr>
       <pin name='pos'>
        <node id='100026' type='6'>
         <attr name='value' type='9'>@CAM_POS@</attr>
        </node>
       </pin>
       <pin name='target'>
        <node id='100027' type='6'>
         <attr name='value' type='9'>@CAM_TARGET@</attr>
        </node>
       </pin>
       <pin name='up'>
        <node id='100028' type='6'>
         <attr name='value' type='9'>@CAM_UP@</attr>
        </node>
       </pin>
      </node>
     </pin>
",
        super::project::xml_escape(code)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iray::Lens;

    fn cam(lens: Lens, aperture_cm: f64) -> SceneCamera {
        // Calib-Tone-Default's camera: 65 mm, film 88.9 × 50 mm.
        SceneCamera { pos: [0.0; 3], target: [0.0, 0.0, -1.0], up: [0.0, 1.0, 0.0], fov_deg: 68.7, focal_depth: 1.5, aperture_cm, film_cm: [8.888889, 5.0], focal_cm: 6.5, lens }
    }

    #[test]
    fn shift_is_a_fraction_of_the_frame() {
        // Calib-Lens-Shift: 5 mm, -3 mm → Iray's 0.5, -0.3 cm on the film.
        let s = thin_lens_shift(&cam(Lens { shift: [0.5, -0.3], ..Lens::default() }, 0.0));
        assert!((s[0] - 0.05625).abs() < 1e-6 && (s[1] + 0.06).abs() < 1e-6, "{s:?}");
        assert_eq!(thin_lens_shift(&cam(Lens::default(), 0.0)), [0.0, 0.0]);
    }

    #[test]
    fn blades_make_a_polygon() {
        assert_eq!(bokeh(&cam(Lens::default(), 8.0)), (6, 0.0, 1.0));
        assert_eq!(bokeh(&cam(Lens { blades: 2, ..Lens::default() }, 8.0)), (6, 0.0, 1.0));
        let (n, rot, round) = bokeh(&cam(Lens { blades: 5, blade_rotation: 20f64.to_radians(), ..Lens::default() }, 8.0));
        assert!(n == 5 && (rot - 20.0).abs() < 1e-9 && round == 0.0);
    }

    #[test]
    fn distortion_goes_through_an_osl_camera() {
        assert!(osl_camera(&cam(Lens::default(), 0.0)).is_none());
        let with = |distortion, k| osl_camera(&cam(Lens { distortion, k, ..Lens::default() }, 0.0)).unwrap();
        let poly3 = with(Distortion::Poly3, [0.3, 0.0, 0.0, 0.0, 0.0]);
        // Forward model: solved for the undistorted radius.
        assert!(poly3.contains("ru * (1.0 - 0.3 + 0.3 * ru * ru) - rd") && poly3.contains("for (int i"), "{poly3}");
        let inv = with(Distortion::InvPoly5, [0.2, 0.1, 0.0, 0.0, 0.0]);
        assert!(inv.contains("float ru = rd * (1.0 + 0.2 * rd * rd + 0.1 * rd * rd * rd * rd)") && !inv.contains("for (int i"), "{inv}");
        let brown = with(Distortion::Brown, [1.0, 0.05, -0.3, 0.1, 0.2]);
        assert!(brown.contains("ru * (1.0 + 0.05 * ru + -0.3 * ru * ru + 0.1 * ru * ru * ru + 0.2 * ru * ru * ru * ru)"), "{brown}");
        let fish = osl_camera(&cam(Lens { distortion: Distortion::Equidistant, k: [0.0, 2.0943951, 0.0, 0.0, 0.0], max_fov: 3.0, ..Lens::default() }, 0.0)).unwrap();
        assert!(fish.contains("float theta = (0.0 + 2.0943951 * rd") && fish.contains("if (theta > 1.5) tMax = 0.0;"), "{fish}");
        assert!(with(Distortion::Spherical, [0.0; 5]).contains("float lat = (v - 0.5) * M_PI;"));
        assert!(with(Distortion::Cylindrical, [0.0; 5]).contains("M_2PI * H / W"));
        // The film, the shift and the scale reach the shader.
        let c = osl_camera(&cam(Lens { distortion: Distortion::Poly3, shift: [0.5, -0.3], distortion_scale: 0.8, ..Lens::default() }, 0.0)).unwrap();
        assert!(c.contains("float W = 8.888889;") && c.contains("float F = 6.5;") && c.contains("+ 0.5;") && c.contains("+ -0.3;") && c.contains("float rd = 0.8 * hypot"), "{c}");
        // Depth of field: Octane's lens random numbers over Iray's lens (m), focus on the plane.
        let dof = osl_camera(&cam(Lens { distortion: Distortion::Poly3, ..Lens::default() }, 8.125)).unwrap();
        assert!(dof.contains("camera:dofrandom") && dof.contains("float q = 0.08125 * sqrt(dof[1]);") && dof.contains("d * (1.5 / F)"), "{dof}");
        let blades = osl_camera(&cam(Lens { distortion: Distortion::Poly3, blades: 5, ..Lens::default() }, 8.125)).unwrap();
        assert!(blades.contains("float a = dof[0] * 5.0;"), "{blades}");
        // The node: Octane's OSL camera with the code escaped.
        let node = osl_camera_node("a < b && c");
        assert!(node.contains("type='126'") && node.contains("<attr name='shaderCode' type='10'>a &lt; b &amp;&amp; c</attr>") && node.contains("@CAM_POS@"));
    }
}
