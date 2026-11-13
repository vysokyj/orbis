//! What Iray's layering of NVIDIA's dielectrics keeps, against what
//! Octane's Universal material keeps, computed from the MDL SDK's `libbsdf`
//! (the reference implementation of MDL's `df::`): resin
//! (`resin_polyurethane_matte`, #113) and architectural glass
//! (`architectural`'s transmit lobe under its reflection, #115).
//!
//! The MDLs: resin is `fresnel_layer(ior, weight, simple_glossy_bsdf
//! (reflect), simple_glossy_bsdf(reflect_transmit, tint))`; architectural
//! `fresnel_layer(refr_ior)` with "Use Fresnel Term", else
//! `custom_curve_layer(r0, r90, exponent)`, of weight reflectivity × the
//! reflection colour's peak, over `weighted_layer(transparency,
//! simple_glossy_bsdf(transmit, refr_color), diffuse)` — closed solids.
//! `libbsdf` (MDL SDK `src/mdl/jit/libbsdf`):
//! - `simple_glossy_bsdf`: a Phong half-vector distribution, exponent
//!   2 / r², with v-cavities masking — `min(G1, G2)` on reflection,
//!   `max(G1 + G2 − 1, 0)` on refraction; reflect / transmit split by
//!   Fresnel at the microfacet; `scatter_transmit` refracts every sample
//!   (no Fresnel), a totally reflected one reflects.
//! - `curve_layer_sample` (both layers): the coat is picked with
//!   probability w · c(n · k1) and weighted w · c(k · h); the base is
//!   weighted **1 − w · max(c(n · k1), c(nk2))** — the curve at the
//!   *macro* normal; under `fresnel_layer` nk2 for a refraction is the
//!   reflection of k1 about the base's microfacet, which light leaving a
//!   rough base from inside meets past the critical angle (c = 1, the base
//!   keeps only 1 − w); `custom_curve_layer` has no IOR and takes |n · k2|.
//!   A loss that grows with roughness and isn't the surface's energy balance.
//!
//! Octane renders the lobes physically (Beckmann with Smith masking at the
//! export's pin, the coating w · F(k · h) at its own IOR, the base the
//! rest). The export scales the base's tint by k so that a closed sphere of
//! the material in a white furnace — the canonical solid, every entry angle
//! (the whole silhouette) — keeps in Octane what it keeps in Iray, per
//! colour channel. Both are traced here by Monte Carlo with a fixed seed
//! (the paths' weights and number of tinted bounces are kept, so k solves
//! exactly against the same paths).
//!
//! Iray's rough transmission through NVIDIA's modules loses more than
//! `libbsdf`'s lobe — the same `simple_glossy_bsdf` through DAZ's Uber
//! doesn't (#116), so it isn't the df and can't be derived from it: the
//! base lobe alone, measured in the furnace (`tint_factor_measured`), sets
//! an extra per-event tint τ on the `libbsdf` model; the layering on top is
//! `libbsdf`'s, and predicts Iray's coated spheres within 1 – 7 %.

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

/// How a lobe scatters: `simple_glossy_bsdf`'s `scatter_mode`.
#[derive(Clone, Copy, PartialEq)]
pub enum Scatter {
    /// Reflect only.
    Reflect,
    /// Reflect / transmit split by Fresnel at the microfacet.
    ReflectTransmit,
    /// Transmit only: `libbsdf` refracts every sample (no Fresnel), a
    /// totally reflected one reflects.
    Transmit,
}

/// The layer's directional curve: `fresnel_layer` (Fresnel at the IOR,
/// past the critical angle 1 from inside) or `custom_curve_layer`
/// (Schlick's r0 + (r90 − r0)(1 − cos)^e, the same from both sides).
#[derive(Clone, Copy)]
pub enum Curve {
    Fresnel(f64),
    Schlick { r0: f64, r90: f64, exponent: f64 },
}

impl Curve {
    fn eval(self, inside: bool, c: f64) -> f64 {
        match self {
            Curve::Fresnel(ior) => fresnel(if inside { 1.0 / ior } else { ior }, c),
            Curve::Schlick { r0, r90, exponent } => r0 + (r90 - r0) * (1.0 - c.clamp(0.0, 1.0)).powf(exponent),
        }
    }
}

/// An NVIDIA dielectric's layer stack on a closed solid (or a thin-walled
/// one): a reflect-only coat (weight, curve, roughness) over a tinted base
/// lobe (its own scatter mode and roughness) of the material's IOR.
/// Roughness is MDL's for `Libbsdf`, Beckmann α for `Smith`.
#[derive(Clone, Copy)]
pub struct Stack {
    pub weight: f64,
    pub curve: Curve,
    pub coat_rough: f64,
    pub base: Scatter,
    pub base_rough: f64,
    pub ior: f64,
    /// MDL's `thin_walled`: a transmission doesn't bend and there is no
    /// inside — both faces are the outside's (the curve and Fresnel at the
    /// IOR from outside, MDL spec 13.1). `libbsdf` transmits the
    /// reflection mirrored through the surface (as rough as the reflection);
    /// Octane's Thin Wall passes the ray straight on, unblurred and
    /// unweighted ("no refraction and no roughness for transmission", #136).
    pub thin: bool,
}

/// One microfacet lobe at h: the direction, its weight (sampled h ∝ D (n·h):
/// |k1·h| G / (|n·k1| |n·h|)) or None when it dies, and whether it refracted.
#[allow(clippy::too_many_arguments)]
fn lobe(model: Model, rough: f64, n: V, v: V, h: V, eta: f64, mode: Scatter, thin: bool, rng: &mut Rng) -> Option<(V, f64, bool)> {
    if thin && mode == Scatter::Transmit && matches!(model, Model::Smith) {
        // Octane's Thin Wall: straight on, unweighted, whatever the facet.
        return Some((scale(v, -1.0), 1.0, true));
    }
    let vh = dot(v, h);
    let (nv, nh) = (dot(n, v), dot(n, h));
    if vh <= 0.0 || nh <= 0.0 || nv <= 0.0 {
        return None;
    }
    // A thin wall's transmission doesn't bend: never total reflection.
    let e = if thin { 1.0 } else { eta };
    let ct2 = 1.0 - (1.0 - vh * vh) / (e * e);
    let refract = match mode {
        Scatter::Reflect => false,
        Scatter::ReflectTransmit => rng.next() >= fresnel(eta, vh),
        Scatter::Transmit => ct2 >= 0.0,
    };
    let reflected = add(scale(h, 2.0 * vh), scale(v, -1.0));
    let l = if !refract {
        reflected
    } else if thin {
        add(reflected, scale(n, -2.0 * dot(n, reflected)))
    } else {
        add(scale(v, -1.0 / eta), scale(h, vh / eta - ct2.max(0.0).sqrt()))
    };
    let nl = dot(n, l);
    if (refract && nl >= 0.0) || (!refract && nl <= 0.0) {
        return None;
    }
    let g = match model {
        Model::Libbsdf => {
            let (g1, g2) = (vcav(nh, vh, nv), vcav(nh, dot(l, h).abs(), nl.abs()));
            if refract && !thin { (g1 + g2 - 1.0).max(0.0) } else { g1.min(g2) }
        }
        Model::Smith => smith_g1(nv, rough) * smith_g1(nl, rough),
    };
    let w = vh * g / (nv * nh);
    (w > 0.0).then_some((l, w, refract))
}

/// Escaping paths of a closed unit sphere in a white furnace, entered over
/// its whole silhouette: (weight without the tint, tinted base events).
/// `Libbsdf` weighs the layers as `curve_layer_sample` does (the coat
/// picked with w · c(n·k1), weighted w · c(k·h); the base 1 − w ·
/// max(c(n·k1), c(nk2)), nk2 the reflection of k1 about the base's
/// microfacet for a refraction under `fresnel_layer`, else |n·k2|);
/// `Smith` physically (the coat w · c(k·h) at its microfacet, the base the
/// rest; Octane's coating is Fresnel at its own IOR).
fn sphere_paths(model: Model, s: &Stack) -> Vec<(f64, i32)> {
    sphere_paths_disk(model, s, 1.0)
}

/// `sphere_paths` entered over the inner disk r < `disk` of the silhouette.
fn sphere_paths_disk(model: Model, s: &Stack, disk: f64) -> Vec<(f64, i32)> {
    let mut rng = Rng(0x5eed_0113);
    let mut out = Vec::with_capacity(PATHS);
    let shared = s.coat_rough == s.base_rough;
    for _ in 0..PATHS {
        let rr = disk * rng.next().sqrt();
        let phi = 2.0 * std::f64::consts::PI * rng.next();
        let mut p: V = [rr * phi.cos(), rr * phi.sin(), (1.0 - rr * rr).max(0.0).sqrt()];
        let mut d: V = [0.0, 0.0, -1.0];
        let (mut thr, mut m, mut inside) = (1.0, 0, false);
        for _ in 0..DEPTH {
            let n = if inside { scale(p, -1.0) } else { p };
            let v = scale(d, -1.0);
            // A thin wall is met from outside on either face.
            let outside = !inside || s.thin;
            let eta = if outside { s.ior } else { 1.0 / s.ior };
            let nk1 = dot(n, v).clamp(0.0, 1.0);
            let h = sample_h(model, s.base_rough, n, &mut rng);
            let hc = if shared { h } else { sample_h(model, s.coat_rough, n, &mut rng) };
            let ev = match model {
                Model::Libbsdf => {
                    let p_layer = s.weight * s.curve.eval(!outside, nk1);
                    if rng.next() < p_layer {
                        lobe(model, s.coat_rough, n, v, hc, eta, Scatter::Reflect, s.thin, &mut rng).map(|(l, w, _)| (l, w * s.weight * s.curve.eval(!outside, dot(v, hc)) / p_layer, false, false))
                    } else {
                        lobe(model, s.base_rough, n, v, h, eta, s.base, s.thin, &mut rng).map(|(l, w, refr)| {
                            let nk2 = match s.curve {
                                Curve::Fresnel(_) if refr => 2.0 * dot(v, h) * dot(n, h) - nk1,
                                _ => dot(n, l).abs(),
                            };
                            let base = 1.0 - s.weight * s.curve.eval(!outside, nk1).max(s.curve.eval(!outside, nk2));
                            (l, w * base / (1.0 - p_layer), refr, true)
                        })
                    }
                }
                Model::Smith => {
                    if rng.next() < s.weight * s.curve.eval(!outside, dot(v, hc)) {
                        lobe(model, s.coat_rough, n, v, hc, eta, Scatter::Reflect, s.thin, &mut rng).map(|(l, w, _)| (l, w, false, false))
                    } else {
                        lobe(model, s.base_rough, n, v, h, eta, s.base, s.thin, &mut rng).map(|(l, w, refr)| (l, w, refr, true))
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

/// What a closed sphere of the stack keeps in a white furnace under Iray
/// (`iray`, MDL roughness) and under Octane (`octane`, Beckmann α) at base
/// tint t: (Iray, Octane).
pub fn furnace_albedo(iray: &Stack, octane: &Stack, tint: f64) -> (f64, f64) {
    (albedo(&sphere_paths(Model::Libbsdf, iray), tint), albedo(&sphere_paths(Model::Smith, octane), tint))
}

/// The base tint × this (per channel) makes the closed sphere keep in
/// Octane (`octane`) what it keeps under Iray's `libbsdf` (`iray`). At most 1.
pub fn tint_factor(iray: &Stack, octane: &Stack, tint: [f32; 3]) -> [f32; 3] {
    let ip = sphere_paths(Model::Libbsdf, iray);
    let op = sphere_paths(Model::Smith, octane);
    tint.map(|t| {
        let t = t.clamp(0.0, 1.0) as f64;
        let target = albedo(&ip, t);
        if t <= 0.0 || albedo(&op, t) <= target {
            return 1.0;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..40 {
            let k = 0.5 * (lo + hi);
            if albedo(&op, t * k) < target { lo = k } else { hi = k }
        }
        (0.5 * (lo + hi)) as f32
    })
}

/// As `tint_factor`, with Iray's rough lobe losing more than `libbsdf`'s:
/// `lobe_albedo` is what Iray's base lobe alone (no coat, white) keeps in
/// the furnace over the sphere's inner disk (r < 0.75 of the silhouette,
/// as measured), and the `libbsdf` model meets it with an extra tint τ on
/// every base event before the stack is traced (#115). A thin-walled
/// sphere's measured profile is flat to the rim (Calib-Furnace-ArchGlass2
/// H4 / H5, #147), so there the disk stands for the whole silhouette and τ
/// is solved on it: at refr_gloss 0 `libbsdf`'s thin model rises towards
/// the rim (0.43 → 0.58) where Iray stays at 0.386, and τ solved on the
/// disk left the whole sphere 1.13 × Iray's.
///
/// `lobe_ior` is the IOR `lobe_albedo` was measured at: τ is solved on the
/// lobe of that IOR (a closed sphere's `libbsdf` lobe itself loses more
/// with the IOR, to total internal reflection and the bent paths), then
/// the material's own stack is traced with it (#148: solved at the
/// material's IOR, τ absorbed `libbsdf`'s own IOR dependence and
/// Calib-Furnace-ArchGlass2's IOR 1.2 / 2 spheres came out 0.89 / 1.32 ×
/// Iray). A thin wall's lobe never bends, so there τ doesn't depend on it.
pub fn tint_factor_measured(iray: &Stack, octane: &Stack, lobe_albedo: f64, lobe_ior: f64, tint: [f32; 3]) -> [f32; 3] {
    let ior = if iray.thin { iray.ior } else { lobe_ior.max(1.0001) };
    let lobe = sphere_paths_disk(Model::Libbsdf, &Stack { weight: 0.0, ior, ..*iray }, if iray.thin { 1.0 } else { 0.75 });
    let tau = if albedo(&lobe, 1.0) <= lobe_albedo { 1.0 } else { solve(|t| albedo(&lobe, t), lobe_albedo) };
    let ip = sphere_paths(Model::Libbsdf, iray);
    let op = sphere_paths(Model::Smith, octane);
    tint.map(|t| {
        let t = t.clamp(0.0, 1.0) as f64;
        let target = albedo(&ip, t * tau);
        if t <= 0.0 || albedo(&op, t) <= target { 1.0 } else { solve(|k| albedo(&op, t * k), target) as f32 }
    })
}

/// k in 0 – 1 with f(k) = target, f increasing.
fn solve(f: impl Fn(f64) -> f64, target: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..40 {
        let k = 0.5 * (lo + hi);
        if f(k) < target { lo = k } else { hi = k }
    }
    0.5 * (lo + hi)
}

/// Linear interpolation in a table of (x, y), x ascending, clamped.
pub fn interp(table: &[(f64, f64)], x: f64) -> f64 {
    let (first, last) = (table[0], table[table.len() - 1]);
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    let i = table.windows(2).position(|w| x <= w[1].0).unwrap();
    let ((x0, y0), (x1, y1)) = (table[i], table[i + 1]);
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

/// Resin's base tint × this (per channel) = Octane's specular and
/// transmission: a closed sphere then keeps in Octane (Beckmann at α =
/// `alpha`, Smith, physical layering) what it keeps under Iray's `libbsdf`
/// (Phong at MDL roughness `rough`, v-cavities, `fresnel_layer`'s macro-
/// normal weights) with coat weight `weight` and IOR `ior`. At most 1.
pub fn resin_tint_factor(rough: f32, alpha: f32, weight: f32, ior: f32, tint: [f32; 3]) -> [f32; 3] {
    let (rough, alpha, weight, ior) = (rough.max(0.0) as f64, alpha.max(0.0) as f64, weight.clamp(0.0, 1.0) as f64, (ior as f64).max(1.0001));
    let stack = |r: f64| Stack { weight, curve: Curve::Fresnel(ior), coat_rough: r, base: Scatter::ReflectTransmit, base_rough: r, ior, thin: false };
    tint_factor(&stack(rough), &stack(alpha), tint)
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

    #[test]
    fn the_glass_layers_follow_their_curve() {
        // Calib-Furnace-ArchGlass: refr_gloss 0.2 (r_t 0.5), reflectivity
        // 0.6 - the Fresnel term's critical angle loses more than Schlick's.
        let sch = Curve::Schlick { r0: 0.2, r90: 1.0, exponent: 5.0 };
        let iray = |curve| Stack { weight: 0.6, curve, coat_rough: 0.0, base: Scatter::Transmit, base_rough: 0.5, ior: 1.5, thin: false };
        let octane = |s: Stack| Stack { curve: Curve::Fresnel(2.618), ..s };
        let ks = tint_factor_measured(&iray(sch), &octane(iray(sch)), 0.614, 1.5, [1.0; 3]);
        let kf = tint_factor_measured(&iray(Curve::Fresnel(1.5)), &octane(iray(Curve::Fresnel(1.5))), 0.614, 1.5, [1.0; 3]);
        assert!((0.8..0.9).contains(&ks[0]) && kf[0] < ks[0] - 0.05, "{ks:?} {kf:?}");
        // The lobe's measured loss only lowers it.
        assert!(tint_factor(&iray(sch), &octane(iray(sch)), [1.0; 3])[0] > ks[0]);
        assert_eq!(interp(&[(0.0, 0.5), (1.0, 1.0)], 0.5), 0.75);
    }

    #[test]
    fn a_thin_wall_does_not_bend() {
        // Octane's Thin Wall passes the light straight on through both
        // walls; `libbsdf`'s near-smooth thin wall keeps it too, its rough
        // one loses to v-cavities masking (no total reflection to trap it).
        let s = |r: f64| Stack { weight: 0.0, curve: Curve::Fresnel(1.5), coat_rough: r, base: Scatter::Transmit, base_rough: r, ior: 1.5, thin: true };
        let (iray, octane) = furnace_albedo(&s(0.02), &s(0.7), 0.8);
        assert!((octane - 0.64).abs() < 1e-9 && (iray - 0.64).abs() < 0.02, "{iray} {octane}");
        assert!(furnace_albedo(&s(0.5), &s(0.5), 1.0).0 < 0.9);
        // A coat on a thin wall reflects off both faces at the IOR from
        // outside; the tint then makes up only what the lobe loses.
        let coated = Stack { weight: 0.6, ..s(0.0) };
        let k = tint_factor_measured(&coated, &coated, 0.505, 1.5, [1.0; 3]);
        assert!((0.6..0.8).contains(&k[0]), "{k:?}");
    }
}
