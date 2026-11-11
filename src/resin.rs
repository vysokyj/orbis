//! What Iray's layering of NVIDIA's resin (`resin_polyurethane_matte`)
//! keeps, against what Octane's Universal material keeps, computed from the
//! MDL SDK's `libbsdf` (the reference implementation of MDL's `df::`) — no
//! fitted constant (#113).
//!
//! The MDL: `fresnel_layer(ior, weight, simple_glossy_bsdf(reflect),
//! simple_glossy_bsdf(reflect_transmit, tint))`, a closed solid of that IOR.
//! `libbsdf` (MDL SDK `src/mdl/jit/libbsdf`):
//! - `simple_glossy_bsdf`: a Phong half-vector distribution, exponent
//!   2 / r², with v-cavities masking — `min(G1, G2)` on reflection,
//!   `max(G1 + G2 − 1, 0)` on refraction; reflect / transmit split by
//!   Fresnel at the microfacet.
//! - `fresnel_layer` (`curve_layer_sample`): the coat is picked with
//!   probability w · F(n · k1) and weighted w · F(k · h); the base is
//!   weighted **1 − w · max(F(n · k1), F(n · k2_refl))** — Fresnel at the
//!   *macro* normal, k2_refl the reflection of k1 about the base's
//!   microfacet. Light leaving a rough base from inside meets k2_refl past
//!   the critical angle (F = 1), so the base keeps only 1 − w there: a loss
//!   that grows with roughness and isn't the surface's energy balance.
//!
//! Octane renders the same lobes physically (Beckmann with Smith masking at
//! the Phong pin, the coat w · F(k · h), the base the rest). The export
//! scales the base's tint by k so that a closed sphere of the material in a
//! white furnace — the canonical solid, every entry angle (the whole
//! silhouette) — keeps in Octane what it keeps under `libbsdf`, per colour
//! channel. Both are traced here by Monte Carlo with a fixed seed (the
//! paths' weights and number of tinted bounces are kept, so k solves
//! exactly against the same paths).

/// Sphere paths per model.
const PATHS: usize = 1 << 16;
/// Surface events per path before it counts as lost.
const DEPTH: usize = 32;

#[derive(Clone, Copy)]
enum Model {
    /// Iray: `libbsdf`'s Phong + v-cavities lobes, `fresnel_layer`'s weights.
    Libbsdf,
    /// Octane: Beckmann at α = the Phong pin², Smith masking, physical layering.
    Smith,
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

type V = [f64; 3];

fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: V, s: f64) -> V {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn norm(a: V) -> V {
    let l = dot(a, a).sqrt();
    if l > 0.0 { scale(a, 1.0 / l) } else { a }
}

/// `libbsdf`'s `ior_fresnel`: eta = transmitted / incident IOR, c the
/// cosine on the incident side; 1 past the critical angle.
fn fresnel(eta: f64, c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    let ct2 = 1.0 - (1.0 - c * c) / (eta * eta);
    if ct2 < 0.0 {
        return 1.0;
    }
    let ct = ct2.sqrt();
    let rp = (ct - c * eta) / (ct + c * eta);
    let ro = (c - ct * eta) / (c + ct * eta);
    (0.5 * (rp * rp + ro * ro)).clamp(0.0, 1.0)
}

/// A microfacet normal around n: Phong's half-vector distribution
/// (`hvd_phong_sample`, cos θ = (1 − u)^(1 / (e + 1)), e = 2 / r²) or
/// Beckmann (tan² θ = −α² ln(1 − u)). Either is p(h) = D(h) (n · h).
fn sample_h(model: Model, rough: f64, n: V, rng: &mut Rng) -> V {
    let (u1, u2) = (rng.next(), rng.next());
    let c = match model {
        Model::Libbsdf => {
            let e = 2.0 / rough.max(1e-7).powi(2);
            (1.0 - u1).powf(1.0 / (e + 1.0))
        }
        Model::Smith => 1.0 / (1.0 - rough * rough * (1.0 - u1).ln()).sqrt(),
    };
    let s = (1.0 - c * c).max(0.0).sqrt();
    let phi = 2.0 * std::f64::consts::PI * u2;
    let a = if n[0].abs() > 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let t = norm([a[1] * n[2] - a[2] * n[1], a[2] * n[0] - a[0] * n[2], a[0] * n[1] - a[1] * n[0]]);
    let b = [n[1] * t[2] - n[2] * t[1], n[2] * t[0] - n[0] * t[2], n[0] * t[1] - n[1] * t[0]];
    add(add(scale(t, s * phi.cos()), scale(b, s * phi.sin())), scale(n, c))
}

/// Smith masking for Beckmann (`libbsdf`'s rational approximation).
fn smith_g1(c: f64, alpha: f64) -> f64 {
    let c = c.abs().clamp(1e-9, 1.0);
    let x = c / (alpha * (1.0 - c * c).sqrt() + 1e-12);
    if x < 1.6 { (3.535 * x + 2.181 * x * x) / (1.0 + 2.276 * x + 2.577 * x * x) } else { 1.0 }
}

/// v-cavities masking (`microfacet_mask_v_cavities`).
fn vcav(nh: f64, kh: f64, nk: f64) -> f64 {
    (2.0 * nh * nk / kh.max(1e-12)).min(1.0)
}

/// One microfacet lobe at h: the direction, its weight (sampled h ∝ D (n·h):
/// |k1·h| G / (|n·k1| |n·h|)) or None when it dies, and whether it refracted.
fn lobe(model: Model, rough: f64, n: V, v: V, h: V, eta: f64, transmit: bool, rng: &mut Rng) -> Option<(V, f64, bool)> {
    let vh = dot(v, h);
    let (nv, nh) = (dot(n, v), dot(n, h));
    if vh <= 0.0 || nh <= 0.0 || nv <= 0.0 {
        return None;
    }
    let refract = transmit && rng.next() >= fresnel(eta, vh);
    let l = if refract {
        let ct = (1.0 - (1.0 - vh * vh) / (eta * eta)).max(0.0).sqrt();
        add(scale(v, -1.0 / eta), scale(h, vh / eta - ct))
    } else {
        add(scale(h, 2.0 * vh), scale(v, -1.0))
    };
    let nl = dot(n, l);
    if (refract && nl >= 0.0) || (!refract && nl <= 0.0) {
        return None;
    }
    let g = match model {
        Model::Libbsdf => {
            let (g1, g2) = (vcav(nh, vh, nv), vcav(nh, dot(l, h).abs(), nl.abs()));
            if refract { (g1 + g2 - 1.0).max(0.0) } else { g1.min(g2) }
        }
        Model::Smith => smith_g1(nv, rough) * smith_g1(nl, rough),
    };
    let w = vh * g / (nv * nh);
    (w > 0.0).then_some((l, w, refract))
}

/// Escaping paths of a closed unit sphere in a white furnace, entered over
/// its whole silhouette: (weight without the tint, tinted base events).
fn sphere_paths(model: Model, rough: f64, weight: f64, ior: f64) -> Vec<(f64, i32)> {
    let mut rng = Rng(0x5eed_0113);
    let mut out = Vec::with_capacity(PATHS);
    for _ in 0..PATHS {
        let rr = rng.next().sqrt();
        let phi = 2.0 * std::f64::consts::PI * rng.next();
        let mut p: V = [rr * phi.cos(), rr * phi.sin(), (1.0 - rr * rr).max(0.0).sqrt()];
        let mut d: V = [0.0, 0.0, -1.0];
        let (mut thr, mut m, mut inside) = (1.0, 0, false);
        for _ in 0..DEPTH {
            let n = if inside { scale(p, -1.0) } else { p };
            let v = scale(d, -1.0);
            let eta = if inside { 1.0 / ior } else { ior };
            let nk1 = dot(n, v).clamp(0.0, 1.0);
            let h = sample_h(model, rough, n, &mut rng);
            let ev = match model {
                Model::Libbsdf => {
                    let p_layer = weight * fresnel(eta, nk1);
                    if rng.next() < p_layer {
                        lobe(model, rough, n, v, h, eta, false, &mut rng).map(|(l, w, _)| (l, w * weight * fresnel(eta, dot(v, h)) / p_layer, false, false))
                    } else {
                        lobe(model, rough, n, v, h, eta, true, &mut rng).map(|(l, w, refr)| {
                            let k2_refl = if refr { 2.0 * dot(v, h) * dot(n, h) - nk1 } else { dot(n, l) };
                            let base = 1.0 - weight * fresnel(eta, nk1).max(fresnel(eta, k2_refl));
                            (l, w * base / (1.0 - p_layer), refr, true)
                        })
                    }
                }
                Model::Smith => {
                    if rng.next() < weight * fresnel(eta, dot(v, h)) {
                        lobe(model, rough, n, v, h, eta, false, &mut rng).map(|(l, w, _)| (l, w, false, false))
                    } else {
                        lobe(model, rough, n, v, h, eta, true, &mut rng).map(|(l, w, refr)| (l, w, refr, true))
                    }
                }
            };
            let Some((l, w, refr, tinted)) = ev else { break };
            thr *= w;
            m += tinted as i32;
            if refr {
                inside = !inside;
            }
            if !inside {
                out.push((thr, m));
                break;
            }
            // The next hit inside the unit sphere.
            let l = norm(l);
            p = norm(add(p, scale(l, -2.0 * dot(p, l))));
            d = l;
        }
    }
    out
}

fn albedo(paths: &[(f64, i32)], tint: f64) -> f64 {
    paths.iter().map(|&(w, m)| w * tint.powi(m)).sum::<f64>() / PATHS as f64
}

/// Resin's base tint × this (per channel) = Octane's specular and
/// transmission: a closed sphere then keeps in Octane (Beckmann at α =
/// `alpha`, Smith, physical layering) what it keeps under Iray's `libbsdf`
/// (Phong at MDL roughness `rough`, v-cavities, `fresnel_layer`'s macro-
/// normal weights) with coat weight `weight` and IOR `ior`. At most 1.
pub fn resin_tint_factor(rough: f32, alpha: f32, weight: f32, ior: f32, tint: [f32; 3]) -> [f32; 3] {
    let (rough, alpha, weight, ior) = (rough.max(0.0) as f64, alpha.max(0.0) as f64, weight.clamp(0.0, 1.0) as f64, (ior as f64).max(1.0001));
    let iray = sphere_paths(Model::Libbsdf, rough, weight, ior);
    let octane = sphere_paths(Model::Smith, alpha, weight, ior);
    tint.map(|t| {
        let t = t.clamp(0.0, 1.0) as f64;
        let target = albedo(&iray, t);
        if t <= 0.0 || albedo(&octane, t) <= target {
            return 1.0;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..40 {
            let k = 0.5 * (lo + hi);
            if albedo(&octane, t * k) < target { lo = k } else { hi = k }
        }
        (0.5 * (lo + hi)) as f32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin2(r: f32) -> f32 {
        r / (1.0 + r * r).sqrt()
    }

    #[test]
    fn smooth_or_uncoated_resin_keeps_its_tint() {
        // A near-mirror: both models keep the light.
        let k = resin_tint_factor(0.02, pin2(0.02), 0.95, 1.49, [0.8; 3]);
        assert!(k[0] > 0.98, "{k:?}");
        // No coat: only the lobes' own masking differs (≤ 1 % at r 0.25).
        let k = resin_tint_factor(0.25, pin2(0.25), 0.0, 1.49, [0.8; 3]);
        assert!(k[0] > 0.97, "{k:?}");
    }

    #[test]
    fn rough_coated_resin_loses_light_to_the_layer_weights() {
        // Calib-Furnace-Resin's Iray spheres, against the smooth one: white
        // r 0.25 0.722 / 0.967, grey 0.473 / 0.623, r 0.5 0.257 / 0.623.
        let k25 = resin_tint_factor(0.25, pin2(0.25), 0.95, 1.49, [0.793, 1.0, 0.0]);
        let k50 = resin_tint_factor(0.5, pin2(0.5), 0.95, 1.49, [0.793; 3]);
        assert!((0.8..0.92).contains(&k25[0]), "{k25:?}");
        assert!(k50[0] < k25[0] - 0.05, "{k50:?} {k25:?}");
        assert_eq!(k25[2], 1.0);
        // Deterministic.
        assert_eq!(k25, resin_tint_factor(0.25, pin2(0.25), 0.95, 1.49, [0.793, 1.0, 0.0]));
    }
}
