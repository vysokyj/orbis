//! IES (ANSI/IES LM-63) photometric files — what the export needs of a light
//! profile: its candela table and its power relative to its peak.
//!
//! Iray scales a profile to the light's flux (`df::light_profile_power`,
//! #34); Octane's distribution reads it normalised to a peak of 1 (the image
//! node's `iesPhotometryMode` 1), so the emitter's power is raised by what
//! the peak-normalised profile keeps of a uniform emitter's flux: its mean
//! over the sphere ([`Profile::mean_share`]) for a point light, its
//! cosine-weighted mean over a surface's front ([`Profile::lambert_share`])
//! for an area light or emissive surface, which Octane emits Lambertian ×
//! the distribution.

use anyhow::{bail, Context, Result};

/// A parsed profile: candela by vertical angle θ (0° = the profile's axis,
/// "nadir" for a downlight) and horizontal angle φ, both in degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Vertical angles, ascending (0 – 90, 90 – 180 or 0 – 180).
    pub vertical: Vec<f64>,
    /// Horizontal angles, ascending: one (axially symmetric), 0 – 90 (one
    /// quadrant mirrored), 0 – 180 (mirrored about the 0 – 180 plane) or
    /// 0 – 360.
    pub horizontal: Vec<f64>,
    /// Candela, `candela[h][v]`, multiplier applied.
    pub candela: Vec<Vec<f64>>,
}

impl Profile {
    pub fn parse(text: &str) -> Result<Profile> {
        let mut lines = text.lines();
        // Keywords up to TILT=.
        let tilt = loop {
            let l = lines.next().context("IES: no TILT= line")?.trim();
            if let Some(t) = l.strip_prefix("TILT=") {
                break t.trim().to_string();
            }
        };
        let mut nums = lines.flat_map(|l| l.split(|c: char| c.is_whitespace() || c == ',')).filter(|t| !t.is_empty()).map(|t| t.parse::<f64>());
        let mut next = || -> Result<f64> { nums.next().context("IES: the file ends early")?.context("IES: not a number") };
        if tilt == "INCLUDE" {
            // Lamp-to-luminaire geometry, then the tilt angles and factors.
            next()?;
            let n = next()? as usize;
            for _ in 0..2 * n {
                next()?;
            }
        }
        let (_lamps, _lumens, multiplier) = (next()?, next()?, next()?);
        let (nv, nh) = (next()? as usize, next()? as usize);
        let (_photometric_type, _units, _w, _l, _h) = (next()?, next()?, next()?, next()?, next()?);
        let (ballast, _future, _watts) = (next()?, next()?, next()?);
        if nv == 0 || nh == 0 {
            bail!("IES: no angles");
        }
        let vertical = (0..nv).map(|_| next()).collect::<Result<Vec<_>>>()?;
        let horizontal = (0..nh).map(|_| next()).collect::<Result<Vec<_>>>()?;
        let scale = multiplier * if ballast > 0.0 { ballast } else { 1.0 };
        let candela = (0..nh).map(|_| (0..nv).map(|_| next().map(|c| c * scale)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?;
        Ok(Profile { vertical, horizontal, candela })
    }

    /// Candela towards (θ, φ), degrees: linear in both angles, the
    /// horizontal symmetry unfolded, 0 outside the vertical range.
    pub fn intensity(&self, theta: f64, phi: f64) -> f64 {
        let (v0, v1) = (self.vertical[0], *self.vertical.last().unwrap());
        if theta < v0 - 1e-9 || theta > v1 + 1e-9 {
            return 0.0;
        }
        let phi = phi.rem_euclid(360.0);
        let h_last = *self.horizontal.last().unwrap();
        let phi = if self.horizontal.len() == 1 {
            self.horizontal[0]
        } else if h_last <= 90.0 + 1e-9 {
            // One quadrant, mirrored into the others.
            let p = phi % 180.0;
            if p > 90.0 { 180.0 - p } else { p }
        } else if h_last <= 180.0 + 1e-9 {
            if phi > 180.0 { 360.0 - phi } else { phi }
        } else {
            phi
        };
        let (hi, ht) = bracket(&self.horizontal, phi);
        let (vi, vt) = bracket(&self.vertical, theta);
        let at = |h: usize| {
            let row = &self.candela[h];
            row[vi] * (1.0 - vt) + row[(vi + 1).min(row.len() - 1)] * vt
        };
        at(hi) * (1.0 - ht) + at((hi + 1).min(self.candela.len() - 1)) * ht
    }

    /// The peak candela.
    pub fn max(&self) -> f64 {
        self.candela.iter().flatten().copied().fold(0.0, f64::max)
    }

    /// The flux, lumens: ∫ I dΩ over the sphere.
    pub fn power(&self) -> f64 {
        let (nt, np) = (360, 72);
        let mut sum = 0.0;
        for i in 0..nt {
            let t = (i as f64 + 0.5) * 180.0 / nt as f64;
            let ring: f64 = (0..np).map(|j| self.intensity(t, (j as f64 + 0.5) * 360.0 / np as f64)).sum::<f64>() / np as f64;
            sum += ring * t.to_radians().sin();
        }
        sum * std::f64::consts::PI.powi(2) * 2.0 / nt as f64
    }

    /// The peak-normalised profile's mean over the sphere: the share of a
    /// uniform emitter's flux a distribution read at a peak of 1 keeps.
    pub fn mean_share(&self) -> f64 {
        let max = self.max();
        if max <= 0.0 {
            return 1.0;
        }
        self.power() / (4.0 * std::f64::consts::PI * max)
    }

    /// What a one-sided Lambertian emitter keeps of its flux with the
    /// peak-normalised profile on it: ∫ I(ω) / I_max · max(0, n · ω) dΩ / π
    /// over its front, averaged over `normals` (unit vectors in the
    /// profile's frame — x φ 0°, y φ 90°, z θ 0° — with weights summing to
    /// 1, e.g. a mesh's face normals by area). 1 for a uniform profile.
    pub fn lambert_share(&self, normals: &[([f64; 3], f64)]) -> f64 {
        let max = self.max();
        if max <= 0.0 || normals.is_empty() {
            return 1.0;
        }
        let (nt, np) = (180, 72);
        let step = std::f64::consts::PI / nt as f64 * 2.0 * std::f64::consts::PI / np as f64;
        let mut sums = vec![0.0; normals.len()];
        for i in 0..nt {
            let t = (i as f64 + 0.5) * 180.0 / nt as f64;
            let (st, ct) = t.to_radians().sin_cos();
            for j in 0..np {
                let p = (j as f64 + 0.5) * 360.0 / np as f64;
                let w = self.intensity(t, p) * st * step;
                if w == 0.0 {
                    continue;
                }
                let (sp, cp) = p.to_radians().sin_cos();
                let d = [st * cp, st * sp, ct];
                for (sum, (n, _)) in sums.iter_mut().zip(normals) {
                    *sum += w * (n[0] * d[0] + n[1] * d[1] + n[2] * d[2]).max(0.0);
                }
            }
        }
        sums.iter().zip(normals).map(|(s, (_, wt))| s * wt).sum::<f64>() / (std::f64::consts::PI * max)
    }
}

/// The interval of `xs` (ascending) holding `x`: its start and the fraction.
fn bracket(xs: &[f64], x: f64) -> (usize, f64) {
    if xs.len() == 1 || x <= xs[0] {
        return (0, 0.0);
    }
    for i in 0..xs.len() - 1 {
        if x <= xs[i + 1] {
            let span = xs[i + 1] - xs[i];
            return (i, if span > 0.0 { (x - xs[i]) / span } else { 0.0 });
        }
    }
    (xs.len() - 1, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn lm63(header: &str, v: &[f64], h: &[f64], rows: &[Vec<f64>]) -> String {
        let join = |a: &[f64]| a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ");
        let mut s = format!("IESNA:LM-63-2002\n[TEST] synthetic\nTILT=NONE\n{header} {} {} 1 2 0 0 0\n1.0 1.0 0\n{}\n{}\n", v.len(), h.len(), join(v), join(h));
        for r in rows {
            s += &join(r);
            s.push('\n');
        }
        s
    }

    #[test]
    fn uniform_profile_has_the_sphere_s_flux() {
        let v: Vec<f64> = (0..=18).map(|i| i as f64 * 10.0).collect();
        let p = Profile::parse(&lm63("1 -1 1", &v, &[0.0], &[vec![100.0; v.len()]])).unwrap();
        assert!((p.power() / (400.0 * PI) - 1.0).abs() < 1e-3, "{}", p.power());
        assert!((p.mean_share() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn hemisphere_downlight_keeps_half_and_symmetries_unfold() {
        // Lit to 90° only (a hemisphere): half a uniform sphere's flux.
        let v: Vec<f64> = (0..=9).map(|i| i as f64 * 10.0).collect();
        let p = Profile::parse(&lm63("1 -1 2", &v, &[0.0], &[vec![50.0; v.len()]])).unwrap();
        assert_eq!(p.max(), 100.0, "multiplier applied");
        assert!((p.mean_share() - 0.5).abs() < 0.01, "{}", p.mean_share());
        assert_eq!(p.intensity(120.0, 0.0), 0.0);
        // One quadrant (0 – 90°) mirrored: φ 270 reads like 90, 180 like 0.
        let rows = vec![vec![10.0; v.len()], vec![30.0; v.len()]];
        let q = Profile::parse(&lm63("1 -1 1", &v, &[0.0, 90.0], &rows)).unwrap();
        assert_eq!(q.intensity(20.0, 270.0), 30.0);
        assert_eq!(q.intensity(20.0, 180.0), 10.0);
        assert_eq!(q.intensity(20.0, 45.0), 20.0);
    }

    #[test]
    fn lambert_share_weighs_the_front_by_the_cosine() {
        let v: Vec<f64> = (0..=18).map(|i| i as f64 * 10.0).collect();
        let uniform = Profile::parse(&lm63("1 -1 1", &v, &[0.0], &[vec![100.0; v.len()]])).unwrap();
        assert!((uniform.lambert_share(&[([0.0, 0.0, 1.0], 1.0)]) - 1.0).abs() < 1e-3);
        assert!((uniform.lambert_share(&[([1.0, 0.0, 0.0], 0.5), ([0.0, -1.0, 0.0], 0.5)]) - 1.0).abs() < 1e-3);
        // A hemisphere downlight on a surface facing along its axis keeps
        // it all; facing away, nothing; sideways, half.
        let down: Vec<f64> = v.iter().map(|t| if *t <= 90.0 { 100.0 } else { 0.0 }).collect();
        let p = Profile::parse(&lm63("1 -1 1", &v, &[0.0], &[down])).unwrap();
        assert!((p.lambert_share(&[([0.0, 0.0, 1.0], 1.0)]) - 1.0).abs() < 0.02, "{}", p.lambert_share(&[([0.0, 0.0, 1.0], 1.0)]));
        assert!(p.lambert_share(&[([0.0, 0.0, -1.0], 1.0)]) < 0.02);
        let side = p.lambert_share(&[([1.0, 0.0, 0.0], 1.0)]);
        assert!((side - 0.5).abs() < 0.06, "{side}");
    }

    #[test]
    fn reads_the_repository_s_synthetic_profile() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let p = Profile::parse(&std::fs::read_to_string(root.join("calib/ies/Orbis-Asym.ies")).unwrap()).unwrap();
        assert_eq!((p.vertical.len(), p.horizontal.len()), (19, 5));
        assert_eq!(p.max(), 1000.0);
        assert!((p.intensity(0.0, 180.0) - 100.0).abs() < 1e-9);
        assert!((p.intensity(30.0, 90.0) - 424.3).abs() < 0.1);
        assert!((p.intensity(30.0, 270.0) - 212.1).abs() < 0.1);
        assert_eq!(p.intensity(70.0, 0.0), 0.0);
    }
}
