//! Genesis 9's eye as Iray renders it (#66): a closed PBRSkin ball whose
//! translucency sends light into a scattering volume and back out through
//! the same surface.
//!
//! `pbr_skin.mdl`'s `diffuse_translucency_bsdf` is a `weighted_layer`: the
//! diffuse at 1 − t (t = Translucency Weight) over a diffuse transmission
//! at t tinted by Translucency Color. What goes through enters
//! `pbr_add_volume` (absorption `−ln(Transmitted Color) / distance`,
//! scattering `−ln(SSS Color) / distance`, Henyey-Greenstein at SSS
//! Direction), and each time it meets the surface from inside it leaves
//! with t × tint or is reflected back in with (1 − t) × the base colour.
//! Every re-entry starts like the first (Lambertian, a ball looks the same
//! from every point), so with P the share of the entering light that
//! reaches the surface again on the lit side (`escape`, a Monte Carlo of
//! the ball) the eye's albedo is a geometric series (`albedo`).
//!
//! Calib-G9Eye (#66), Octane with this albedo / Iray, sclera: -nogloss
//! (the maps, gloss off) 0.97 – 1.08; -t1 (the map in both crossings, T²)
//! 0.96 – 1.04 G / B (T alone predicted 2.6 – 2.9); -t08-white 0.97 –
//! 1.00 G / B. Red runs 10 – 25 % high where T is 1 or the maps white.
//! The maps alone as albedo had rendered 2.4 – 3.6 × Iray.

use crate::daz_material::Material;

/// Genesis 9's eye ball radius, mm: its `l_eye` bone from the centre
/// (z 7.375 cm) to the cornea's apex (z 8.68 cm).
pub const G9_EYE_RADIUS_MM: f32 = 13.0;

/// The eye's volume as `pbr_add_volume` builds it from the channels, per
/// mm (DAZ's distances are cm). Unsaved channels take PBRSkin.dsf's
/// defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volume {
    pub absorption: [f32; 3],
    pub scattering: [f32; 3],
    pub direction: f32,
}

impl Volume {
    pub fn of(mat: &Material) -> Self {
        let on = |k: &str| mat.float(k).is_none_or(|v| v != 0.0);
        // A colour at a distance → a coefficient (the MDL's log form);
        // black absorbs everything, a distance ≤ 0 nothing.
        let coeff = |c: [f32; 3], cm: f32| c.map(|c| if cm <= 0.0 { 0.0 } else if c <= 0.0 { f32::INFINITY } else { -c.min(1.0).ln() / (cm * 10.0) });
        let absorption = coeff(mat.color("Transmitted Color").unwrap_or([0.0; 3]), mat.float("Transmitted Measurement Distance").unwrap_or(0.12));
        let sss = on("Sub Surface Enable");
        let scattering = if sss { coeff(mat.color("SSS Color").unwrap_or([0.976_470_6f32, 0.694_117_6, 0.760_784_3].map(crate::daz_material::colour_to_linear)), mat.float("Scattering Measurement Distance").unwrap_or(0.015)) } else { [0.0; 3] };
        Volume { absorption, scattering, direction: if sss { mat.float("SSS Direction").unwrap_or(0.0).clamp(-0.99, 0.99) } else { 0.0 } }
    }
}

/// splitmix64: a fixed sequence, so an export is reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 40) as f32 + 0.5) / (1u64 << 24) as f32
    }
}

type V3 = [f32; 3];

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// A direction at cos `mu` from `d`, random azimuth.
fn turn(d: V3, mu: f32, rng: &mut Rng) -> V3 {
    let a = if d[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let u = [d[1] * a[2] - d[2] * a[1], d[2] * a[0] - d[0] * a[2], d[0] * a[1] - d[1] * a[0]];
    let n = dot(u, u).sqrt();
    let u = u.map(|c| c / n);
    let v = [d[1] * u[2] - d[2] * u[1], d[2] * u[0] - d[0] * u[2], d[0] * u[1] - d[1] * u[0]];
    let phi = std::f32::consts::TAU * rng.next();
    let s = (1.0 - mu * mu).max(0.0).sqrt();
    [0, 1, 2].map(|k| d[k] * mu + u[k] * s * phi.cos() + v[k] * s * phi.sin())
}

/// Henyey-Greenstein's cosine at asymmetry `g`.
fn hg(g: f32, rng: &mut Rng) -> f32 {
    if g.abs() < 1e-4 {
        return 2.0 * rng.next() - 1.0;
    }
    let x = (1.0 - g * g) / (1.0 - g + 2.0 * g * rng.next());
    ((1.0 + g * g - x * x) / (2.0 * g)).clamp(-1.0, 1.0)
}

/// The share of light entering a ball of `radius_mm` (Lambertian, at its
/// front pole) that reaches the surface again on the front half, per
/// channel: what a white interface lets out where it is lit and seen.
/// Light that crosses to the back half goes into the socket. Deterministic
/// (fixed seed, `paths` per channel).
pub fn escape(v: &Volume, radius_mm: f32, paths: u32) -> [f32; 3] {
    [0, 1, 2].map(|ch| {
        let (sa, ss) = (v.absorption[ch], v.scattering[ch]);
        if !sa.is_finite() {
            return 0.0;
        }
        let st = sa + ss;
        let albedo = if st > 0.0 { ss / st } else { 0.0 };
        let mut rng = Rng(0x5EED + ch as u64);
        let mut out = 0.0f64;
        for _ in 0..paths {
            let mut p = [0.0, 0.0, radius_mm];
            let mut d = turn([0.0, 0.0, -1.0], rng.next().sqrt(), &mut rng);
            let mut w = 1.0f32;
            loop {
                // Distance to the sphere along d (p is inside or on it).
                let b = dot(p, d);
                let c = dot(p, p) - radius_mm * radius_mm;
                let exit = -b + (b * b - c).max(0.0).sqrt();
                let s = if st > 0.0 { -(rng.next().ln()) / st } else { f32::INFINITY };
                if s >= exit {
                    let at = [0, 1, 2].map(|k| p[k] + d[k] * exit);
                    if at[2] > 0.0 {
                        out += w as f64;
                    }
                    break;
                }
                p = [0, 1, 2].map(|k| p[k] + d[k] * s);
                w *= albedo;
                if w < 1e-3 {
                    if rng.next() < 0.5 {
                        break;
                    }
                    w *= 2.0;
                }
                d = turn(d, hg(v.direction, &mut rng), &mut rng);
            }
        }
        (out / paths as f64) as f32
    })
}

/// The eye's albedo for one channel: the diffuse layer, then the light the
/// translucency lets in (t × tint), that comes back (P) and out again (t ×
/// tint), the diffuse layer reflecting the rest back in ((1 − t) × base)
/// for another round: (1 − t) B + t² T² P / (1 − (1 − t) B P). All linear.
pub fn albedo(t: f32, base: f32, tint: f32, p: f32) -> f32 {
    let r = (1.0 - t) * base;
    r + t * t * tint * tint * p / (1.0 - r * p).max(1e-6)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Calib-G9Eye's eye as DAZ hands it to Iray (its .mi: Transmitted
    /// Color 0.949 / 0.201 / 0.101 at 0.1 cm, SSS Color 0.949 / 0.448 /
    /// 0.548 at 0.015 cm, direction −0.798).
    fn g9() -> Volume {
        let coeff = |c: [f32; 3], mm: f32| c.map(|c| -c.ln() / mm);
        Volume { absorption: coeff([0.948_965, 0.201_096, 0.101_145], 1.0), scattering: coeff([0.948_965, 0.447_871, 0.547_994], 0.15), direction: -0.797_752_8 }
    }

    #[test]
    fn escape_matches_the_reference_monte_carlo() {
        // scripts/dev/eye-volume-mc.py, 200 000 paths: 0.608 / 0.427 / 0.322.
        let p = escape(&g9(), G9_EYE_RADIUS_MM, 20_000);
        for (got, want) in p.iter().zip([0.608, 0.427, 0.322]) {
            assert!((got - want).abs() < 0.015, "{p:?}");
        }
    }

    #[test]
    fn albedo_is_the_ball_series() {
        // White maps at t 0.8: the reference MC's 0.65 / 0.50 / 0.42.
        let p = [0.6103, 0.4268, 0.3215];
        let a = p.map(|p| albedo(0.8, 1.0, 1.0, p));
        for (got, want) in a.iter().zip([0.65, 0.50, 0.42]) {
            assert!((got - want).abs() < 0.01, "{a:?}");
        }
        assert_eq!(albedo(0.0, 0.4, 0.7, 0.5), 0.4, "no translucency: the diffuse");
        assert!((albedo(1.0, 0.4, 0.5, 0.6) - 0.15).abs() < 1e-6, "t 1: T² P");
    }

    #[test]
    fn black_transmitted_colour_returns_nothing() {
        let v = Volume { absorption: [f32::INFINITY, 0.0, 0.0], scattering: [1.0; 3], direction: 0.0 };
        let p = escape(&v, G9_EYE_RADIUS_MM, 2_000);
        assert_eq!(p[0], 0.0);
        assert!(p[1] > 0.9, "no absorption: all of it comes back, {p:?}");
    }
}
