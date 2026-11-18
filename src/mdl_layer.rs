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
//! doesn't (#116): NVIDIA's modules are `mdl 1.0`, and Iray reads the df
//! with an undocumented legacy behaviour below 1.3 (our own 1.0 / 1.3
//! twins, #117: 1.3 is this tracer ring for ring, 1.0 is it times a
//! per-event loss that grows with roughness and towards grazing, at any
//! IOR, with or without refraction). Its shape is derivable: the legacy
//! lobe is a Phong of exponent e = 2 / r², D ∝ cosᵉ, where `libbsdf`'s is
//! cos^(e − 1) — in reflection (#184) and in its transmitting modes too
//! (#191) — so the model traces `libbsdf`'s lobe at `legacy_rough`. The
//! loss on top is not derivable, so it is a measured
//! per-event table on closed solids (`LEGACY_LOBE`), the base lobe alone
//! measured in the furnace scales it (τ, `tint_factor_measured`); the
//! layering on top is `libbsdf`'s, and predicts Iray's coated spheres
//! within 1 – 7 %. Its angle can't be drawn by one k: Octane's
//! transmission gets a Falloff Mix whose shape is solved so the two
//! models' rings meet (`transmission_falloff`).

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
fn sphere_paths(model: Model, s: &Stack) -> Vec<Pw> {
    sphere_paths_disk(model, s, 1.0)
}

/// `sphere_paths` entered over the inner disk r < `disk` of the silhouette.
fn sphere_paths_disk(model: Model, s: &Stack, disk: f64) -> Vec<Pw> {
    trace(model, s, disk, false).paths.iter().map(Pw::of).collect()
}

/// One escaping path: its weight without the tint, its tinted base events,
/// the entry radius (of the silhouette), the incoming cosine |n · k1| of
/// each base event that passed through the surface, and the microfacet's
/// Fresnel F of each refraction through an Octane reflect + transmit base
/// (`Pw::at`).
struct Path {
    w: f64,
    m: i32,
    rr: f64,
    through: Vec<f64>,
    fr: Vec<f64>,
}

/// A path's weight without the tint and what a tint x makes of it
/// (`Pw::at`).
#[derive(Clone)]
struct Pw {
    w: f64,
    m: i32,
    fr: Vec<f64>,
}

impl Pw {
    fn of(p: &Path) -> Pw {
        Pw { w: p.w, m: p.m, fr: p.fr.clone() }
    }

    /// The weight at base tint x: x on every tinted event, and on each
    /// refraction of an Octane reflect + transmit base (`fr`) (1 − F x) /
    /// (1 − F) more — Universal takes its specular F · s off the
    /// transmission, T (1 − F s), where the model's physical split leaves
    /// T (1 − F); resin's specular s is its transmission's tint x (#201).
    fn at(&self, x: f64) -> f64 {
        tinted(self.w, self.m, &self.fr, x)
    }
}

/// `Pw::at` of a weight w, m tinted events and refractions of Fresnel `fr`.
fn tinted(w: f64, m: i32, fr: &[f64], x: f64) -> f64 {
    w * x.powi(m) * fr.iter().map(|f| (1.0 - f * x) / (1.0 - f)).product::<f64>()
}

/// The paths of `sphere_paths_disk`, and how many entered each tenth of the
/// silhouette's radius (the rings `furnace-rings.py` measures).
struct Traced {
    paths: Vec<Path>,
    entered: [usize; 10],
}

/// The sphere's paths; `legacy`: Iray's `mdl 1.0` lobe (`legacy_loss` on
/// every base event of a closed `Libbsdf` stack).
fn trace(model: Model, s: &Stack, disk: f64, legacy: bool) -> Traced {
    trace_n(model, s, disk, legacy, PATHS)
}

/// `trace` with `paths` paths.
fn trace_n(model: Model, s: &Stack, disk: f64, legacy: bool, paths: usize) -> Traced {
    let legacy = legacy && !s.thin && matches!(model, Model::Libbsdf);
    // The legacy lobe's shape: `libbsdf`'s at `legacy_rough` (the coat is
    // the module's `mdl 1.0` lobe too); its loss stays indexed by r.
    let (base_d, coat_d) = if legacy { (legacy_rough(s.base_rough), legacy_rough(s.coat_rough)) } else { (s.base_rough, s.coat_rough) };
    let mut rng = Rng(0x5eed_0113);
    let mut out = Vec::with_capacity(paths);
    let mut entered = [0usize; 10];
    let shared = s.coat_rough == s.base_rough;
    for _ in 0..paths {
        let rr = disk * rng.next().sqrt();
        entered[((rr * 10.0) as usize).min(9)] += 1;
        let phi = 2.0 * std::f64::consts::PI * rng.next();
        let mut p: V = [rr * phi.cos(), rr * phi.sin(), (1.0 - rr * rr).max(0.0).sqrt()];
        let mut d: V = [0.0, 0.0, -1.0];
        let (mut thr, mut m, mut inside) = (1.0, 0, false);
        let mut through = Vec::new();
        let mut fr = Vec::new();
        for _ in 0..DEPTH {
            let n = if inside { scale(p, -1.0) } else { p };
            let v = scale(d, -1.0);
            // A thin wall is met from outside on either face.
            let outside = !inside || s.thin;
            let eta = if outside { s.ior } else { 1.0 / s.ior };
            let nk1 = dot(n, v).clamp(0.0, 1.0);
            let h = sample_h(model, base_d, n, &mut rng);
            let hc = if shared { h } else { sample_h(model, coat_d, n, &mut rng) };
            let ev = match model {
                Model::Libbsdf => {
                    let p_layer = s.weight * s.curve.eval(!outside, nk1);
                    if rng.next() < p_layer {
                        lobe(model, s.coat_rough, n, v, hc, eta, Scatter::Reflect, s.thin, &mut rng).map(|(l, w, _)| (l, w * s.weight * s.curve.eval(!outside, dot(v, hc)) / p_layer, false, 0))
                    } else {
                        lobe(model, s.base_rough, n, v, h, eta, s.base, s.thin, &mut rng).map(|(l, w, refr)| {
                            let nk2 = match s.curve {
                                Curve::Fresnel(_) if refr => 2.0 * dot(v, h) * dot(n, h) - nk1,
                                _ => dot(n, l).abs(),
                            };
                            let base = 1.0 - s.weight * s.curve.eval(!outside, nk1).max(s.curve.eval(!outside, nk2));
                            let g = if legacy { legacy_loss(s.base_rough, nk1) } else { 1.0 };
                            (l, w * base * g / (1.0 - p_layer), refr, 1)
                        })
                    }
                }
                Model::Smith => {
                    if rng.next() < s.weight * s.curve.eval(!outside, dot(v, hc)) {
                        // Octane's Thin Wall met from inside lays the
                        // coating under the base on half the hits: that
                        // reflection passes the transmission twice (#168:
                        // probes with a known coating and k, the inner face's
                        // reflection R (1 + k²) / 2, its transmission k (1 − R)).
                        let under = s.thin && inside && s.base == Scatter::Transmit && rng.next() < 0.5;
                        lobe(model, s.coat_rough, n, v, hc, eta, Scatter::Reflect, s.thin, &mut rng).map(|(l, w, _)| (l, w, false, if under { 2 } else { 0 }))
                    } else {
                        // Octane's base (probes, #201): a total reflection
                        // inside is untinted (Universal's specular reflects
                        // s of it, the transmission lobe the rest), and a
                        // reflect + transmit base refracts T (1 − F s).
                        let vh = dot(v, h);
                        let tir = !s.thin && 1.0 - (1.0 - vh * vh) / (eta * eta) < 0.0;
                        lobe(model, s.base_rough, n, v, h, eta, s.base, s.thin, &mut rng).map(|(l, w, refr)| {
                            if refr && !s.thin && s.base == Scatter::ReflectTransmit {
                                fr.push(fresnel(eta, vh));
                            }
                            (l, w, refr, if tir && !refr { 0 } else { 1 })
                        })
                    }
                }
            };
            let Some((l, w, refr, tinted)) = ev else { break };
            thr *= w;
            m += tinted;
            if refr {
                through.push(nk1);
                inside = !inside;
            }
            if !inside {
                out.push(Path { w: thr, m, rr, through, fr: std::mem::take(&mut fr) });
                break;
            }
            // The next hit inside the unit sphere.
            let l = norm(l);
            p = norm(add(p, scale(l, -2.0 * dot(p, l))));
            d = l;
        }
    }
    Traced { paths: out, entered }
}

/// Iray's `mdl 1.0` `simple_glossy_bsdf` (NVIDIA's modules) loses this on
/// every base event of a closed solid over the cosᵉ lobe it draws
/// (`libbsdf`'s at `legacy_rough`):
/// g(r, θ) = head + (grazing − head) (θ / 90°)^skew at the event's incoming
/// angle, by MDL roughness, linear in r between the rows. **Measured, not
/// derived** — the release notes name only a "legacy behavior" (#117). Each
/// row is fitted to our own `mdl 1.0` modules' furnace rings (centre to
/// 0.8 – 0.9 of the silhouette, Iray Server 5000): Calib-Furnace-
/// MdlTransmit T1 – T4 (IOR 1.5, r 0.05 / 0.1 / 0.2 / 0.4), -MdlTir X1 /
/// X3 (r 0.2 at IOR 1.2 / 2), -MdlLegacy K1 – K4 (r 0.3 / 0.5 / 0.7 / 1),
/// -MdlLegacy2 L1 – L4, L7 (IOR 1.05 / 1.2 / 2 at r 0.6 and 1),
/// Calib-Furnace-Resin2 U3 – U6 (`scatter_reflect_transmit`, r 0.25 / 0.5 /
/// 0.75 / 1), MdlArch A2 (r 0.6; left out of the #191 refit: its r is
/// architectural's equivalent, `ARCH_GLASS_RT`). One row holds every IOR from 1.05 to 2
/// and both scatter modes within 1 – 5 % per ring (r 1, IOR 1.05: 0.88 at
/// the rim), so the loss is the lobe's, not the bending's. A thin wall
/// loses about g² per wall head-on (`ARCH_GLASS_THIN_LOBE` stays measured
/// on its own): not this table. Rows 0.4 – 1 refitted on the cosᵉ lobe
/// (#191): the old rows, fitted on `libbsdf`'s lobe at r, left the
/// spheres' rings up to 4 / 10 / 9 / 10 / 23 % off on it at r 0.5 / 0.6 /
/// 0.7 / 0.75 / 1, the refit 1.5 / 5.8 / 3.0 / 2.6 / 11 % (the r 1 row
/// holds IOR 1.05 – 2 and rt at once, as before 10.5 %); below r 0.4 the
/// two lobes differ by < 2 % in r and the rows stand (worst ring 1 – 1.7 %
/// on either lobe). The new rows lose more: the cosᵉ lobe alone keeps more
/// than `libbsdf`'s at r (disk 0.84 v 0.82 at r 0.5, 0.61 v 0.53 at r 1).
const LEGACY_LOBE: [(f64, f64, f64, f64); 12] = [
    (0.0, 1.0, 1.0, 1.0),
    (0.05, 1.009, 0.971, 0.296),
    (0.1, 0.995, 0.970, 0.460),
    (0.2, 0.986, 0.967, 0.453),
    (0.25, 0.977, 0.956, 1.134),
    (0.3, 0.970, 0.942, 1.720),
    (0.4, 0.955, 0.870, 2.352),
    (0.5, 0.927, 0.789, 2.047),
    (0.6, 0.907, 0.735, 1.522),
    (0.7, 0.885, 0.628, 1.454),
    (0.75, 0.862, 0.629, 1.374),
    (1.0, 0.790, 0.551, 1.209),
];

/// The roughness at which `libbsdf`'s Phong lobe (cos^(e − 1), e = 2 / r²)
/// is Iray's legacy `mdl 1.0` lobe of MDL roughness `r` (cosᵉ): e′ − 1 = e,
/// r′ = r √(2 / (2 + r²)). Derived: point-lit, head-on, every `mdl 1.0`
/// lobe draws the cosᵉ profile and `libbsdf`'s lobe at r′ the same to a
/// pixel — reflect-only (#184), `scatter_reflect_transmit`'s reflection
/// and `scatter_transmit` through a closed sphere (#191,
/// Calib-Point-MdlReflTrans, Calib-Back-MdlTransmit).
pub fn legacy_rough(r: f64) -> f64 {
    r * (2.0 / (2.0 + r * r)).sqrt()
}

/// `LEGACY_LOBE` at MDL roughness `r` and incoming cosine `c`, at most 1
/// (r 0.05's head 1.009 is the 8-bit reference near white).
pub fn legacy_loss(r: f64, c: f64) -> f64 {
    let t = c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2;
    let row = |&(_, head, grazing, skew): &(f64, f64, f64, f64)| head + (grazing - head) * t.powf(skew);
    let r = r.clamp(0.0, 1.0);
    let i = LEGACY_LOBE.windows(2).position(|w| r <= w[1].0).unwrap_or(LEGACY_LOBE.len() - 2);
    let (a, b) = (&LEGACY_LOBE[i], &LEGACY_LOBE[i + 1]);
    let f = (r - a.0) / (b.0 - a.0);
    (row(a) + (row(b) - row(a)) * f).min(1.0)
}

/// What Octane renders of a plain closed rough dielectric sphere in the
/// white furnace, measured (#185): Calib-Furnace-UberGlass's export
/// re-pinned (`orbx-variants.py set=`) to Universal Beckmann at α (the
/// roughness pin √α), transmission white, no falloff, `specular` 0 (`tr`:
/// the transmission lobe alone) and 1 (`rt`: reflect + transmit by
/// Fresnel), at IOR 1.5 and 2; enhanced-final `--no-denoise`, 1024
/// samples, raw pass, `furnace-rings.py measure`: the rings 0 – 0.1 … 0.8 –
/// 0.9 of the silhouette over the dome. **Measured, not derived**: the
/// Smith model (`trace`, `Model::Smith`) follows `tr` within 1 – 3 % up to
/// α 0.7 but Octane's `rt` keeps 1.10 / 1.16 / 1.37 × the model's disk at α
/// 0.6 / 0.7 / 1 (rim 1.24 / 1.34 / 1.71) — its masking is Smith's, its
/// rough dielectric reflection, most of all where facets inside reflect
/// totally, keeps more than F D G, and no single-event model explains all
/// of it (2026-11-17 note). Rows (IOR, α, `tr` rings, `rt` rings).
///
/// That export carries the Uber glass's **`fake_shadows` 1** (#201): this
/// is the table of a material with fake shadows, the rough Uber glass. With
/// them off the same `rt` spheres keep much less towards the rim (α 0.5:
/// rim 0.80 → 0.72, α 1 0.86 → 0.57; `OCTANE_SPHERE_PLAIN`).
const OCTANE_SPHERE: [(f64, f64, [f64; 9], [f64; 9]); 16] = [
    (1.5, 0.25, [1.011, 1.012, 1.013, 1.010, 1.006, 0.999, 0.982, 0.954, 0.907], [1.009, 1.009, 1.008, 1.007, 1.003, 0.995, 0.979, 0.953, 0.911]),
    (1.5, 0.36, [1.013, 1.007, 1.001, 0.994, 0.979, 0.958, 0.926, 0.887, 0.830], [1.007, 1.005, 1.001, 0.992, 0.978, 0.959, 0.932, 0.897, 0.848]),
    (1.5, 0.45, [0.989, 0.984, 0.973, 0.956, 0.935, 0.906, 0.871, 0.828, 0.772], [0.988, 0.982, 0.974, 0.958, 0.940, 0.917, 0.888, 0.855, 0.815]),
    (1.5, 0.5, [0.967, 0.959, 0.946, 0.927, 0.905, 0.875, 0.839, 0.799, 0.744], [0.965, 0.959, 0.948, 0.934, 0.915, 0.892, 0.866, 0.838, 0.803]),
    (1.5, 0.6, [0.895, 0.890, 0.877, 0.859, 0.837, 0.811, 0.781, 0.747, 0.697], [0.899, 0.894, 0.888, 0.876, 0.863, 0.848, 0.833, 0.817, 0.798]),
    (1.5, 0.7, [0.815, 0.809, 0.803, 0.791, 0.773, 0.755, 0.734, 0.707, 0.665], [0.827, 0.829, 0.824, 0.820, 0.816, 0.812, 0.810, 0.812, 0.807]),
    (1.5, 0.85, [0.705, 0.705, 0.703, 0.699, 0.694, 0.688, 0.682, 0.667, 0.636], [0.742, 0.742, 0.746, 0.754, 0.764, 0.779, 0.799, 0.822, 0.833]),
    (1.5, 1.0, [0.628, 0.629, 0.631, 0.633, 0.638, 0.644, 0.647, 0.642, 0.620], [0.685, 0.687, 0.698, 0.712, 0.733, 0.764, 0.800, 0.835, 0.858]),
    (2.0, 0.25, [1.012, 1.009, 1.003, 0.995, 0.983, 0.965, 0.942, 0.912, 0.875], [1.002, 0.999, 0.994, 0.986, 0.976, 0.960, 0.941, 0.917, 0.883]),
    (2.0, 0.36, [0.963, 0.958, 0.950, 0.937, 0.917, 0.895, 0.868, 0.836, 0.795], [0.962, 0.955, 0.945, 0.935, 0.919, 0.901, 0.877, 0.849, 0.814]),
    (2.0, 0.45, [0.893, 0.888, 0.881, 0.867, 0.851, 0.831, 0.804, 0.775, 0.738], [0.901, 0.896, 0.890, 0.880, 0.866, 0.848, 0.829, 0.800, 0.773]),
    (2.0, 0.5, [0.851, 0.844, 0.837, 0.825, 0.810, 0.792, 0.769, 0.743, 0.707], [0.872, 0.864, 0.855, 0.848, 0.834, 0.820, 0.801, 0.781, 0.762]),
    (2.0, 0.6, [0.755, 0.757, 0.750, 0.742, 0.733, 0.722, 0.705, 0.687, 0.661], [0.792, 0.789, 0.786, 0.782, 0.778, 0.769, 0.761, 0.754, 0.744]),
    (2.0, 0.7, [0.673, 0.673, 0.669, 0.667, 0.664, 0.659, 0.652, 0.644, 0.627], [0.726, 0.723, 0.723, 0.725, 0.727, 0.730, 0.734, 0.742, 0.748]),
    (2.0, 0.85, [0.580, 0.577, 0.582, 0.584, 0.588, 0.595, 0.601, 0.601, 0.602], [0.647, 0.649, 0.654, 0.663, 0.675, 0.692, 0.717, 0.741, 0.767]),
    (2.0, 1.0, [0.518, 0.521, 0.527, 0.533, 0.543, 0.556, 0.568, 0.583, 0.593], [0.599, 0.604, 0.612, 0.625, 0.644, 0.673, 0.707, 0.746, 0.786]),
];

/// `OCTANE_SPHERE` without fake shadows (Octane's default, off): what
/// resin's and architectural glass's exports write (#201). Measured the
/// same way on Calib-Furnace-Resin's Faithful export, every sphere
/// re-pinned (Beckmann at √α, transmission white, coating 0, albedo 0,
/// `fake_shadows` 0; 1024 samples raw, `furnace-rings.py measure`). The
/// same spheres with `fake_shadows` 1 reproduce `OCTANE_SPHERE`'s `rt`
/// rows within 1 % (α 0.5 / 0.7 / 1), so the switch is the difference.
const OCTANE_SPHERE_PLAIN: [(f64, f64, [f64; 9], [f64; 9]); 16] = [
    (1.5, 0.25, [1.012, 1.015, 1.016, 1.016, 1.011, 1.000, 0.980, 0.945, 0.892], [1.004, 1.006, 1.007, 1.007, 1.004, 0.993, 0.972, 0.941, 0.891]),
    (1.5, 0.36, [1.023, 1.021, 1.016, 1.002, 0.983, 0.956, 0.916, 0.870, 0.805], [1.015, 1.012, 1.004, 0.991, 0.972, 0.945, 0.909, 0.864, 0.806]),
    (1.5, 0.45, [1.019, 1.011, 0.994, 0.969, 0.938, 0.900, 0.856, 0.804, 0.738], [1.001, 0.992, 0.975, 0.951, 0.922, 0.887, 0.847, 0.801, 0.748]),
    (1.5, 0.5, [1.000, 0.988, 0.968, 0.942, 0.906, 0.866, 0.821, 0.770, 0.704], [0.972, 0.965, 0.946, 0.919, 0.889, 0.854, 0.814, 0.769, 0.721]),
    (1.5, 0.6, [0.928, 0.916, 0.897, 0.870, 0.837, 0.799, 0.757, 0.711, 0.648], [0.896, 0.887, 0.868, 0.845, 0.817, 0.785, 0.752, 0.719, 0.677]),
    (1.5, 0.7, [0.836, 0.828, 0.814, 0.794, 0.769, 0.738, 0.703, 0.662, 0.603], [0.797, 0.795, 0.782, 0.766, 0.746, 0.724, 0.702, 0.681, 0.645]),
    (1.5, 0.85, [0.702, 0.702, 0.697, 0.689, 0.676, 0.660, 0.640, 0.607, 0.549], [0.662, 0.661, 0.661, 0.657, 0.653, 0.649, 0.646, 0.634, 0.604]),
    (1.5, 1.0, [0.609, 0.612, 0.615, 0.617, 0.616, 0.611, 0.597, 0.566, 0.508], [0.570, 0.572, 0.576, 0.582, 0.591, 0.600, 0.606, 0.600, 0.572]),
    (2.0, 0.25, [1.011, 1.005, 1.000, 0.987, 0.970, 0.944, 0.914, 0.877, 0.830], [0.991, 0.989, 0.982, 0.972, 0.955, 0.935, 0.907, 0.876, 0.836]),
    (2.0, 0.36, [0.949, 0.942, 0.928, 0.905, 0.879, 0.846, 0.810, 0.767, 0.715], [0.924, 0.915, 0.905, 0.886, 0.862, 0.833, 0.804, 0.765, 0.721]),
    (2.0, 0.45, [0.846, 0.838, 0.826, 0.805, 0.782, 0.752, 0.717, 0.674, 0.625], [0.823, 0.814, 0.805, 0.788, 0.767, 0.741, 0.709, 0.677, 0.639]),
    (2.0, 0.5, [0.775, 0.772, 0.762, 0.744, 0.722, 0.697, 0.664, 0.625, 0.577], [0.753, 0.752, 0.743, 0.727, 0.708, 0.686, 0.661, 0.630, 0.598]),
    (2.0, 0.6, [0.647, 0.640, 0.633, 0.623, 0.608, 0.590, 0.566, 0.535, 0.495], [0.629, 0.625, 0.620, 0.612, 0.599, 0.586, 0.570, 0.555, 0.532]),
    (2.0, 0.7, [0.526, 0.525, 0.522, 0.517, 0.510, 0.499, 0.484, 0.461, 0.428], [0.514, 0.512, 0.513, 0.509, 0.507, 0.503, 0.499, 0.495, 0.482]),
    (2.0, 0.85, [0.397, 0.397, 0.398, 0.398, 0.398, 0.397, 0.391, 0.380, 0.354], [0.384, 0.384, 0.389, 0.395, 0.401, 0.408, 0.419, 0.428, 0.430]),
    (2.0, 1.0, [0.321, 0.322, 0.325, 0.329, 0.335, 0.338, 0.338, 0.329, 0.307], [0.305, 0.309, 0.314, 0.323, 0.335, 0.352, 0.370, 0.385, 0.391]),
];

/// Octane's render / its Smith model per ring (`OCTANE_SPHERE`; the model
/// traced with 2^18 paths, ring noise ~1 %), over the mean ratio of its
/// IOR's and mode's α 0.25 row on the disk (rings to 0.7): there the model
/// is Octane's within 1 % and what is left (1.01) is the dome's
/// normalisation, not the lobe, so the excess is 1 at α ≤ 0.25. Rows
/// (IOR, α, `tr`, `rt`).
/// `fake_shadows`: the table of a material with fake shadows on
/// (`OCTANE_SPHERE`, the Uber glass) or off (`OCTANE_SPHERE_PLAIN`, resin
/// and architectural glass, #201).
fn octane_excess_rows(fake_shadows: bool) -> &'static [(f64, f64, [f64; 9], [f64; 9])] {
    static ROWS: [std::sync::OnceLock<Vec<(f64, f64, [f64; 9], [f64; 9])>>; 2] = [std::sync::OnceLock::new(), std::sync::OnceLock::new()];
    let table: &[(f64, f64, [f64; 9], [f64; 9])] = if fake_shadows { &OCTANE_SPHERE } else { &OCTANE_SPHERE_PLAIN };
    ROWS[fake_shadows as usize].get_or_init(|| {
        let model = |ior: f64, alpha: f64, weight: f64| {
            let s = Stack { weight, curve: Curve::Fresnel(ior), coat_rough: alpha, base: Scatter::Transmit, base_rough: alpha, ior, thin: false };
            let t = trace_n(Model::Smith, &s, 1.0, false, 1 << 18);
            let mut rings = [0.0; 9];
            for p in &t.paths {
                let j = (p.rr * 10.0) as usize;
                if j < 9 {
                    rings[j] += p.w;
                }
            }
            (0..9).map(|j| rings[j] / t.entered[j].max(1) as f64).collect::<Vec<_>>()
        };
        let raw: Vec<_> = table
            .iter()
            .map(|&(ior, alpha, tr, rt)| {
                let (mt, mr) = (model(ior, alpha, 0.0), model(ior, alpha, 1.0));
                (ior, alpha, std::array::from_fn::<f64, 9, _>(|j| tr[j] / mt[j]), std::array::from_fn::<f64, 9, _>(|j| rt[j] / mr[j]))
            })
            .collect();
        raw.iter()
            .map(|&(ior, alpha, tr, rt)| {
                let base = raw.iter().find(|r| r.0 == ior).unwrap();
                let (bt, br) = (base.2[..7].iter().sum::<f64>() / 7.0, base.3[..7].iter().sum::<f64>() / 7.0);
                (ior, alpha, tr.map(|x| x / bt), rt.map(|x| x / br))
            })
            .collect()
    })
}

/// What Octane renders of the closed `octane` stack over its Smith model,
/// per tenth of the silhouette's radius the path entered (the last for
/// 0.9 – 1): `octane_excess_rows` at the stack's base α, linear in α (1 at
/// α ≤ 0.25) and in the IOR between the measured ones (held outside),
/// mixed between `tr` and `rt` by the reflection's share — 1 for a
/// reflect + transmit base, the coat's weight over a transmit-only one
/// (Universal's `specular` over its transmission, or a coating: one
/// Fresnel reflection on the same surface; the coating's own α is not
/// measured, the base's stands for both). None for a thin wall (Octane's
/// Thin Wall doesn't scatter its transmission, #136) or a reflect-only base.
/// `fake_shadows`: the material's Universal pin (`octane_excess_rows`).
fn octane_excess(octane: &Stack, fake_shadows: bool) -> Option<[f64; 10]> {
    let share = match octane.base {
        _ if octane.thin => return None,
        Scatter::Reflect => return None,
        Scatter::ReflectTransmit => 1.0,
        Scatter::Transmit => octane.weight.clamp(0.0, 1.0),
    };
    let rows = octane_excess_rows(fake_shadows);
    let at_ior = |ior: f64| {
        let mine: Vec<_> = rows.iter().filter(|r| r.0 == ior).collect();
        let alphas: Vec<f64> = std::iter::once(0.0).chain(mine.iter().map(|r| r.1)).collect();
        let a = octane.base_rough.clamp(0.0, *alphas.last().unwrap());
        let i = alphas.windows(2).position(|w| a <= w[1]).unwrap_or(alphas.len() - 2);
        let f = (a - alphas[i]) / (alphas[i + 1] - alphas[i]);
        let ring = |r: Option<&&(f64, f64, [f64; 9], [f64; 9])>, j: usize| r.map_or(1.0, |r| r.2[j] + share * (r.3[j] - r.2[j]));
        let lo = if i == 0 { None } else { mine.get(i - 1) };
        std::array::from_fn::<f64, 9, _>(|j| ring(lo, j) + f * (ring(mine.get(i), j) - ring(lo, j)))
    };
    let mut iors: Vec<f64> = rows.iter().map(|r| r.0).collect();
    iors.dedup();
    let ior = octane.ior.clamp(iors[0], *iors.last().unwrap());
    let i = iors.windows(2).position(|w| ior <= w[1]).unwrap_or(0);
    let rings = if iors.len() == 1 {
        at_ior(iors[0])
    } else {
        let (a, b) = (at_ior(iors[i]), at_ior(iors[i + 1]));
        let f = (ior - iors[i]) / (iors[i + 1] - iors[i]);
        std::array::from_fn(|j| a[j] + f * (b[j] - a[j]))
    };
    Some(std::array::from_fn(|j| rings[j.min(8)]))
}

/// Octane's sphere paths: the Smith model, with `excess` each path's
/// weight × what Octane renders over the model at its entry ring
/// (`octane_excess`, #185; `Tuning::rough_transmission_excess`), measured
/// with or without `fake_shadows` (#201).
fn octane_trace(octane: &Stack, excess: bool, fake_shadows: bool) -> Traced {
    let mut t = trace(Model::Smith, octane, 1.0, false);
    if let Some(e) = excess.then(|| octane_excess(octane, fake_shadows)).flatten() {
        for p in &mut t.paths {
            p.w *= e[((p.rr * 10.0) as usize).min(9)];
        }
    }
    t
}

fn albedo(paths: &[Pw], tint: f64) -> f64 {
    paths.iter().map(|p| p.at(tint)).sum::<f64>() / PATHS as f64
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
    tint_factor_octane(iray, octane, tint, false)
}

/// `tint_factor` against what Octane renders (`excess`: the Smith model ×
/// `octane_excess`, #185) or its Smith model.
pub fn tint_factor_octane(iray: &Stack, octane: &Stack, tint: [f32; 3], excess: bool) -> [f32; 3] {
    let ip = sphere_paths(Model::Libbsdf, iray);
    // The rough Uber glass, the one caller, keeps fake shadows (#201).
    let op: Vec<Pw> = octane_trace(octane, excess, true).paths.iter().map(Pw::of).collect();
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
/// every base event before the stack is traced (#115). For a thin-walled
/// sphere `lobe_albedo` is the whole silhouette and τ is solved on it
/// (#147): at refr_gloss 0 `libbsdf`'s thin model rises towards the rim
/// (0.43 → 0.58) where Iray stays at 0.386 (Calib-Furnace-ArchGlass2 H5,
/// flat), and τ solved on the disk left the whole sphere 1.13 × Iray's; at
/// 0.4 Iray falls to the rim (0.943 → 0.885, Calib-Furnace-ArchGlass3 J4,
/// #149) and Octane's Thin Wall draws one flat k², so the whole sphere is
/// what it can match.
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

/// Octane's transmission for a closed solid of an NVIDIA `mdl 1.0` stack:
/// the base tint × `k` (per channel) head-on, × `k` · `grazing` at 90°,
/// (θ / 90°)^`skew` between — a Falloff Map (normal vs. eye ray) mixing
/// the two, which Octane takes per hit at that hit's incoming ray (inside a
/// solid the refracted ray's, #117 note 2026-11-13 § 3), so a factor on
/// every transmission event as the legacy loss is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Falloff {
    pub k: [f32; 3],
    pub grazing: f32,
    pub skew: f32,
}

/// `tint_factor_measured` with the angle (#117): the Iray model is
/// `libbsdf` × Iray's legacy per-event loss (`legacy_loss`), scaled by τ so
/// the lobe alone keeps the measured `lobe_albedo` on the inner disk at
/// `lobe_ior` (τ ≈ 1: the table was fitted on those spheres); Octane's
/// Smith model gets the Falloff on its transmission events and the shape
/// (k, grazing, skew) is solved at the tint's grey (the channels' mean;
/// white before #201, which left a grey resin's centre 7 % low and its rim
/// 8 % high: Octane's grey dielectric keeps its rim relatively brighter) so
/// its rings (tenths of the radius to 0.9, each by its share of the
/// silhouette) meet Iray's; then k per channel so the whole sphere keeps
/// Iray's at the tint. Why: a
/// constant k (one value for every angle) matched the disk and left the
/// rim 1.13 – 1.32 × Iray at r 0.5 – 1 (Calib-Furnace-Resin2 U4 – U6), 1.17
/// × at architectural's refr_gloss 0.2; with the falloff the models' rings
/// agree within 1 – 3 %. A thin wall: `tint_factor_measured` (Octane's
/// Thin Wall is flat), no falloff.
///
/// `excess`: Octane's side is what Octane renders, the Smith model ×
/// `octane_excess` (#185, `Tuning::rough_transmission_excess`), so the
/// solve also takes back what Octane keeps over its own model.
pub fn transmission_falloff(iray: &Stack, octane: &Stack, lobe_albedo: f64, lobe_ior: f64, tint: [f32; 3], excess: bool) -> Falloff {
    if iray.thin {
        return Falloff { k: tint_factor_measured(iray, octane, lobe_albedo, lobe_ior, tint), grazing: 1.0, skew: 1.0 };
    }
    falloff_rings(iray, octane, lobe_albedo, lobe_ior, tint, excess).0
}

/// `transmission_falloff` of a closed stack, with Iray's white rings and
/// Octane's at the solved falloff, both at the tint's grey (centre to 0.8 – 0.9).
fn falloff_rings(iray: &Stack, octane: &Stack, lobe_albedo: f64, lobe_ior: f64, tint: [f32; 3], excess: bool) -> (Falloff, [f64; 9], [f64; 9]) {
    let lobe = trace(Model::Libbsdf, &Stack { weight: 0.0, ior: lobe_ior.max(1.0001), ..*iray }, 0.75, true);
    let lobe = lobe.paths.iter().map(Pw::of).collect::<Vec<_>>();
    let tau = solve_in(|t| albedo(&lobe, t), lobe_albedo, 1.2);
    let ip = trace(Model::Libbsdf, iray, 1.0, true);
    // Resin and architectural glass, the callers, export no fake shadows (#201).
    let op = octane_trace(octane, excess, false);
    // The shape is solved at the material's own grey (the channels' mean):
    // Octane's grey dielectric keeps its rim relatively brighter than a
    // tinted lobe (T (1 - F s), untinted total reflections, #201), so a
    // shape solved on white leaves a grey resin's centre low and its rim
    // high (R5 0.93 -> 1.08 x Iray). White tints solve as before.
    let grey = (tint.iter().map(|&t| t.clamp(0.0, 1.0) as f64).sum::<f64>() / 3.0).max(0.05);
    // Iray's rings at that grey (t · τ per base event).
    let mut target = [0.0; 9];
    for p in &ip.paths {
        let j = (p.rr * 10.0) as usize;
        if j < 9 {
            target[j] += p.w * (grey * tau).powi(p.m);
        }
    }
    for j in 0..9 {
        target[j] /= ip.entered[j].max(1) as f64;
    }
    // Octane's paths with the falloff's angles as θ / 90°.
    let angles: Vec<Vec<f64>> = op.paths.iter().map(|p| p.through.iter().map(|c| c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2).collect()).collect();
    let shape = |q: f64, s: f64, p: usize| angles[p].iter().map(|t| 1.0 + (q - 1.0) * t.powf(s)).product::<f64>();
    let error = |x: &[f64; 3]| {
        let (k, q, s) = (x[0], x[1], x[2]);
        if !(0.01..=1.5).contains(&k) || !(0.0..=2.0).contains(&q) || !(0.2..=8.0).contains(&s) {
            return f64::INFINITY;
        }
        let mut rings = [0.0; 9];
        for (i, p) in op.paths.iter().enumerate() {
            let j = (p.rr * 10.0) as usize;
            if j < 9 {
                rings[j] += tinted(p.w, p.m, &p.fr, grey * k) * shape(q, s, i);
            }
        }
        (0..9).filter(|&j| target[j] > 0.0).map(|j| op.entered[j] as f64 / PATHS as f64 * (rings[j] / op.entered[j].max(1) as f64 / target[j] - 1.0).powi(2)).sum::<f64>()
    };
    let [_, q, s] = nelder_mead(error, [1.0, 0.7, 2.0]);
    let (q, s) = if (q - 1.0).abs() < 0.02 { (1.0, 1.0) } else { (q, s) };
    let iw = ip.paths.iter().map(Pw::of).collect::<Vec<_>>();
    let ow: Vec<Pw> = op.paths.iter().enumerate().map(|(i, p)| Pw { w: p.w * shape(q, s, i), ..Pw::of(p) }).collect();
    let k = tint.map(|t| {
        let t = t.clamp(0.0, 1.0) as f64;
        let target = albedo(&iw, t * tau);
        if t <= 0.0 { 1.0 } else { solve_in(|k| albedo(&ow, t * k), target, 1.5) as f32 }
    });
    let kw = grey * solve_in(|k| albedo(&ow, grey * k), albedo(&iw, grey * tau), 1.5);
    let mut rings = [0.0; 9];
    for (i, p) in op.paths.iter().enumerate() {
        let j = (p.rr * 10.0) as usize;
        if j < 9 {
            rings[j] += ow[i].at(kw) / op.entered[j].max(1) as f64;
        }
    }
    (Falloff { k, grazing: q as f32, skew: s as f32 }, target, rings)
}

/// Nelder – Mead from `x0` (deterministic, 150 steps).
fn nelder_mead(f: impl Fn(&[f64; 3]) -> f64, x0: [f64; 3]) -> [f64; 3] {
    let mut pts: Vec<[f64; 3]> = vec![x0];
    for (i, step) in [0.1, 0.2, 1.0].into_iter().enumerate() {
        let mut p = x0;
        p[i] -= step;
        pts.push(p);
    }
    let mut vals: Vec<f64> = pts.iter().map(&f).collect();
    let at = |a: &[f64; 3], b: &[f64; 3], t: f64| [0, 1, 2].map(|i| a[i] + t * (b[i] - a[i]));
    for _ in 0..150 {
        let mut idx = [0, 1, 2, 3];
        idx.sort_by(|&a, &b| vals[a].total_cmp(&vals[b]));
        pts = idx.iter().map(|&i| pts[i]).collect();
        vals = idx.iter().map(|&i| vals[i]).collect();
        let c = [0, 1, 2].map(|i| (pts[0][i] + pts[1][i] + pts[2][i]) / 3.0);
        let xr = at(&c, &pts[3], -1.0);
        let fr = f(&xr);
        if fr < vals[0] {
            let xe = at(&c, &pts[3], -2.0);
            let fe = f(&xe);
            (pts[3], vals[3]) = if fe < fr { (xe, fe) } else { (xr, fr) };
        } else if fr < vals[2] {
            (pts[3], vals[3]) = (xr, fr);
        } else {
            let xc = at(&c, &pts[3], 0.5);
            let fc = f(&xc);
            if fc < vals[3] {
                (pts[3], vals[3]) = (xc, fc);
            } else {
                for i in 1..4 {
                    pts[i] = at(&pts[0], &pts[i], 0.5);
                    vals[i] = f(&pts[i]);
                }
            }
        }
    }
    let best = (0..4).min_by(|&a, &b| vals[a].total_cmp(&vals[b])).unwrap();
    pts[best]
}

/// k in 0 – 1 with f(k) = target, f increasing.
fn solve(f: impl Fn(f64) -> f64, target: f64) -> f64 {
    solve_in(f, target, 1.0)
}

/// k in 0 – `hi` with f(k) = target, f increasing.
fn solve_in(f: impl Fn(f64) -> f64, target: f64, hi: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, hi);
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
        crate::material::phong_alpha(r)
    }

    #[test]
    fn smooth_or_uncoated_resin_keeps_its_tint() {
        // A near-mirror: both models keep the light (Octane's grey
        // transmission takes only F s off, #201: 0.97 rather than 0.98).
        let k = resin_tint_factor(0.02, pin2(0.02), 0.95, 1.49, [0.8; 3]);
        assert!(k[0] > 0.965, "{k:?}");
        // No coat: the lobes' own masking differs ≤ 1 % at r 0.25, but a
        // grey base keeps more in Octane - its refractions T (1 - F s),
        // its total reflections untinted (#201) - so k takes ~5 % off.
        let k = resin_tint_factor(0.25, pin2(0.25), 0.0, 1.49, [0.8; 3]);
        assert!((0.93..0.96).contains(&k[0]), "{k:?}");
        assert!(resin_tint_factor(0.25, pin2(0.25), 0.0, 1.49, [1.0; 3])[0] > 0.97);
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
    fn uber_rough_glass_loses_what_v_cavities_lose() {
        // Calib-Furnace-UberGlass (#118): Iray is `libbsdf` per ring; Octane's
        // Smith keeps more the rougher the lobe (disk 1.10 × at r 0.6).
        let s = |base, rough: f64| Stack { weight: 0.0, curve: Curve::Fresnel(1.5), coat_rough: rough, base, base_rough: rough, ior: 1.5, thin: false };
        let alpha = |r: f64| pin2(r as f32) as f64;
        let k = |base, r| tint_factor(&s(base, r), &Stack { base: Scatter::Transmit, weight: if base == Scatter::Transmit { 0.0 } else { 1.0 }, ..s(base, alpha(r)) }, [1.0; 3])[0];
        assert!(k(Scatter::Transmit, 0.051) > 0.999);
        let (k354, k6) = (k(Scatter::Transmit, 0.354), k(Scatter::Transmit, 0.6));
        // (Octane at `phong_alpha`, #162's α: k at r 0.6 → 0.975, k5 → 0.977.)
        assert!((0.98..0.995).contains(&k354) && (0.95..0.98).contains(&k6), "{k354} {k6}");
        let k5 = k(Scatter::ReflectTransmit, 0.5);
        assert!((0.95..0.98).contains(&k5), "{k5}");
    }

    #[test]
    fn the_falloff_draws_the_legacy_rim() {
        // The `mdl 1.0` lobe's loss: none on a mirror, head-on 0.91 at r
        // 0.6 (MdlLegacy2 L1 / L3 / L4 on the cosᵉ lobe, #191), more to the rim.
        assert_eq!(legacy_loss(0.0, 0.3), 1.0);
        assert!((legacy_loss(0.6, 1.0) - 0.907).abs() < 1e-9 && legacy_loss(0.6, 0.5) < 0.86, "{}", legacy_loss(0.6, 0.5));
        // The cosᵉ lobe is `libbsdf`'s one exponent up.
        let r = 0.5;
        assert!(((2.0 / legacy_rough(r).powi(2) - 1.0) - 2.0 / (r * r)).abs() < 1e-9);
        assert!(legacy_loss(1.0, 1.0) < legacy_loss(0.5, 1.0) && legacy_loss(0.5, 1.0) < legacy_loss(0.2, 1.0));
        // Uncoated resin r 0.5 (Calib-Furnace-Resin2 U4): one k left
        // Octane's rim 1.13 × Iray's; with the falloff every ring within 3 %.
        // Octane at the export's α (`legacy_phong_alpha`, #191).
        let alpha = |r: f64| crate::material::legacy_phong_alpha(r as f32) as f64;
        let s = |rough| Stack { weight: 0.0, curve: Curve::Fresnel(1.49), coat_rough: rough, base: Scatter::ReflectTransmit, base_rough: rough, ior: 1.49, thin: false };
        let (f, iray, octane) = falloff_rings(&s(0.5), &s(alpha(0.5)), 0.678, 1.49, [1.0; 3], false);
        assert!(f.grazing < 0.9 && f.k[0] > 0.85, "{f:?}");
        assert!((0..9).all(|j| (octane[j] / iray[j] - 1.0).abs() < 0.03), "{iray:?} {octane:?}");
        // Iray's model is the measured rings (U4 0.783 → 0.517).
        assert!((iray[0] - 0.783).abs() < 0.02 && (iray[8] - 0.517).abs() < 0.02, "{iray:?}");
        // A thin wall keeps one flat k.
        let thin = Stack { thin: true, base: Scatter::Transmit, ..s(0.5) };
        assert_eq!(transmission_falloff(&thin, &thin, 0.5, 1.5, [1.0; 3], true).grazing, 1.0);
    }

    #[test]
    fn a_reflect_only_legacy_lobe_loses_the_legacy_loss_too() {
        // #205: Calib-Furnace-MdlLobe (`mdl 1.0` reflect, white, Iray Server
        // 5000), rings centre -> 0.8 - 0.9. With `legacy_loss` on every
        // event the model is Iray's within 3.5 % per ring up to r 0.5; the
        // cos^e lobe alone keeps 2 - 18 % more. Resin's coat is such a lobe
        // (the export doesn't give it the loss yet, #205).
        let iray: [(f64, [f64; 9]); 3] = [
            (0.3, [0.962, 0.962, 0.962, 0.959, 0.943, 0.941, 0.926, 0.906, 0.877]),
            (0.4, [0.924, 0.924, 0.914, 0.906, 0.898, 0.884, 0.864, 0.840, 0.817]),
            (0.5, [0.834, 0.834, 0.834, 0.818, 0.813, 0.797, 0.780, 0.762, 0.749]),
        ];
        for (r, ir) in iray {
            let st = Stack { weight: 0.0, curve: Curve::Fresnel(1.5), coat_rough: r, base: Scatter::Reflect, base_rough: r, ior: 1.5, thin: false };
            let ratio = |legacy: bool| {
                let t = trace(Model::Libbsdf, &st, 1.0, legacy);
                let mut rings = [0.0; 9];
                for p in &t.paths {
                    let j = (p.rr * 10.0) as usize;
                    if j < 9 {
                        rings[j] += p.w;
                    }
                }
                std::array::from_fn::<f64, 9, _>(|j| rings[j] / t.entered[j] as f64 / ir[j])
            };
            let (with, without) = (ratio(true), ratio(false));
            assert!(with.iter().all(|x| (x - 1.0).abs() < 0.035), "r {r}: {with:?}");
            assert!(without[8] > 1.05, "r {r}: {without:?}");
        }
    }

    #[test]
    fn octane_keeps_more_than_its_smith_model_in_rough_transmission() {
        // #185: the plain spheres' excess, none at α 0.25 (the baseline).
        let s = |weight, alpha, ior| Stack { weight, curve: Curve::Fresnel(ior), coat_rough: alpha, base: Scatter::Transmit, base_rough: alpha, ior, thin: false };
        for fs in [true, false] {
            assert!(octane_excess(&s(1.0, 0.25, 1.5), fs).unwrap()[..7].iter().all(|e| (e - 1.0).abs() < 0.02));
            assert!(octane_excess(&s(1.0, 0.1, 1.5), fs).unwrap().iter().all(|e| (e - 1.0).abs() < 0.01));
        }
        // With fake shadows (the Uber glass) the reflection on: the disk
        // ~1.1 at α 0.6, the rim more.
        let rt = octane_excess(&s(1.0, 0.6, 1.5), true).unwrap();
        assert!((1.05..1.15).contains(&rt[3]) && rt[8] > rt[3] + 0.08, "{rt:?}");
        // Transmission alone barely: within 5 % up to α 0.7.
        let tr = octane_excess(&s(0.0, 0.7, 1.5), true).unwrap();
        assert!(tr[..7].iter().all(|e| (e - 1.0).abs() < 0.05), "{tr:?}");
        // Without fake shadows (resin, architectural glass, #201) the rt
        // sphere is its Smith model's to a few % and flat to the rim.
        let plain = octane_excess(&s(1.0, 0.6, 1.5), false).unwrap();
        assert!(plain[..9].iter().all(|e| (e - 1.0).abs() < 0.06) && plain[8] < rt[8] - 0.08, "{plain:?}");
        // A reflect + transmit base is the rt sphere; a thin wall none.
        let rtb = Stack { weight: 0.0, base: Scatter::ReflectTransmit, ..s(0.0, 0.6, 1.5) };
        assert_eq!(octane_excess(&rtb, true), Some(rt));
        assert_eq!(octane_excess(&Stack { thin: true, ..s(1.0, 0.6, 1.5) }, true), None);
        // Folded in, rough resin's level drops, most towards the rim (the
        // falloff's grazing ratio); off, it is the model's. Resin exports
        // no fake shadows: the plain table's few % (k 0.81 → 0.78, grazing
        // 0.70 → 0.62 at r 0.75), not the Uber glass's 1.2 – 1.5 × rim.
        let alpha = |r: f64| crate::material::legacy_phong_alpha(r as f32) as f64;
        let r = |rough| Stack { weight: 0.0, curve: Curve::Fresnel(1.49), coat_rough: rough, base: Scatter::ReflectTransmit, base_rough: rough, ior: 1.49, thin: false };
        let (off, on) = (transmission_falloff(&r(0.75), &r(alpha(0.75)), 0.462, 1.49, [1.0; 3], false), transmission_falloff(&r(0.75), &r(alpha(0.75)), 0.462, 1.49, [1.0; 3], true));
        assert!(on.k[0] < off.k[0] && (0.03..0.12).contains(&(off.grazing - on.grazing)), "{off:?} {on:?}");
    }

    #[test]
    fn octanes_grey_dielectric_takes_only_its_specular_off_the_transmission() {
        // #201 probes: Calib-Furnace-Resin's spheres re-pinned (Beckmann,
        // no coating, Octane 1024 samples raw), disk r < 0.75 over the
        // dome. Specular = transmission = T keeps T (1 - F T) per
        // refraction, not T (1 - F): smooth, IOR 2, T 0.25 / 0.5 / 0.75 /
        // 0.9 rendered 0.088 / 0.294 / 0.606 / 0.833 (the physical split
        // 0.08 / 0.26 / 0.55 / 0.77).
        let rt = |alpha, ior| Stack { weight: 0.0, curve: Curve::Fresnel(ior), coat_rough: alpha, base: Scatter::ReflectTransmit, base_rough: alpha, ior, thin: false };
        let disk = |s: &Stack, t| albedo(&sphere_paths_disk(Model::Smith, s, 0.75), t);
        for (t, octane) in [(0.25, 0.088), (0.5, 0.294), (0.75, 0.606), (0.9, 0.833)] {
            let m = disk(&rt(0.02, 2.0), t);
            assert!((m / octane - 1.0).abs() < 0.03, "{t}: {m} v {octane}");
        }
        // Rough (α 0.25, IOR 1.49, T 0.5): 0.263, where every total
        // reflection tinted made 0.249 and its rim 25 % darker.
        let m = disk(&rt(0.25, 1.49), 0.5);
        assert!((m / 0.263 - 1.0).abs() < 0.02, "{m}");
        // Transmission alone (specular 0): T per refraction, no Fresnel, a
        // total reflection untinted - α 0.25 disk 0.249 at T 0.5.
        let tr = Stack { base: Scatter::Transmit, ..rt(0.25, 1.49) };
        let m = disk(&tr, 0.5);
        assert!((m / 0.249 - 1.0).abs() < 0.02, "{m}");
        // White, all three are the physical split: 1.
        assert!((disk(&rt(0.02, 2.0), 1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn a_grey_resins_falloff_is_solved_at_its_grey() {
        // #201: R5 (grey 0.79, r 0.25, uncoated) - solved on white its
        // centre rendered 0.93 x Iray and its rim 1.08 x; at its grey the
        // model meets Iray's rings within 3 % (render 1.01 -> 1.02).
        let s = |a: f64| Stack { weight: 0.0, curve: Curve::Fresnel(1.49), coat_rough: a, base: Scatter::ReflectTransmit, base_rough: a, ior: 1.49, thin: false };
        let alpha = crate::material::legacy_phong_alpha(0.25) as f64;
        let (_, target, rings) = falloff_rings(&s(0.25), &s(alpha), 0.920, 1.49, [0.787; 3], true);
        assert!((0..9).all(|j| (rings[j] / target[j] - 1.0).abs() < 0.03), "{target:?} {rings:?}");
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

#[cfg(test)]
mod diag201 {
    use super::*;

    /// Octane's model rings of a closed stack at the exported pins: head-on
    /// colour x, grazing ratio q, skew s (#201 diagnostic).
    fn rings(octane: &Stack, excess: bool, x: f64, q: f64, s: f64) -> [f64; 9] {
        let op = octane_trace(octane, excess, false);
        let mut r = [0.0; 9];
        for p in &op.paths {
            let j = (p.rr * 10.0) as usize;
            if j < 9 {
                let sh: f64 = p.through.iter().map(|c| 1.0 + (q - 1.0) * (c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2).powf(s)).product();
                r[j] += tinted(p.w, p.m, &p.fr, x) * sh;
            }
        }
        for j in 0..9 {
            r[j] /= op.entered[j].max(1) as f64;
        }
        r
    }

    #[test]
    #[ignore]
    fn print_resin_fit() {
        // Iray's target rings v the solved Octane model (falloff_rings), per resin sphere.
        let resin = |r: f64, w: f64, alpha: bool| {
            let a = if alpha { crate::material::legacy_phong_alpha(r as f32) as f64 } else { r };
            Stack { weight: w, curve: Curve::Fresnel(1.49), coat_rough: a, base: Scatter::ReflectTransmit, base_rough: a, ior: 1.49, thin: false }
        };
        let grey = 0.787f32;
        for (lab, r, w, lobe, t) in [("R5", 0.25, 0.0, 0.920, grey), ("R6", 0.25, 0.0, 0.920, 1.0), ("R7", 0.5, 0.95, 0.678, grey), ("U7", 0.5, 0.0, 0.678, grey), ("U4", 0.5, 0.0, 0.678, 1.0), ("U5", 0.75, 0.0, 0.462, 1.0), ("U6", 1.0, 0.0, 0.324, 1.0)] {
            let (f, target, rings) = falloff_rings(&resin(r, w, false), &resin(r, w, true), lobe, 1.49, [t; 3], true);
            let ratio: Vec<String> = (0..9).map(|j| format!("{:.2}", rings[j] / target[j])).collect();
            println!("{lab} {f:?} model/target {}", ratio.join(" "));
        }
    }

    #[test]
    #[ignore]
    fn print_excess_tables() {
        for fs in [true, false] {
            for (ior, a, tr, rt) in octane_excess_rows(fs) {
                let f = |r: &[f64; 9]| r.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" ");
                println!("fs {fs} ior {ior} a {a}: tr {} | rt {}", f(tr), f(rt));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_old_pins() {
        let resin = |r: f64, w: f64| {
            let a = crate::material::legacy_phong_alpha(r as f32) as f64;
            Stack { weight: w, curve: Curve::Fresnel(1.49), coat_rough: a, base: Scatter::ReflectTransmit, base_rough: a, ior: 1.49, thin: false }
        };
        // (label, stack, T0, T90, skew, Octane's render of those pins)
        let cases: Vec<(&str, Stack, f64, f64, f64, [f64; 9])> = vec![
            ("R5 grey r.25 c0", resin(0.25, 0.0), 0.777628, 0.715525, 1.985864, [0.622, 0.622, 0.620, 0.615, 0.613, 0.607, 0.591, 0.570, 0.541]),
            ("R6 white r.25 c0", resin(0.25, 0.0), 0.977262, 0.899217, 1.985864, [0.962, 0.962, 0.962, 0.961, 0.949, 0.937, 0.910, 0.871, 0.816]),
            ("R2 grey r.02", resin(0.02, 0.95), 0.777452, 0.693456, 3.843993, [0.640, 0.640, 0.640, 0.640, 0.640, 0.639, 0.637, 0.636, 0.644]),
            ("R4 white r.02", resin(0.02, 0.95), 0.979612, 0.873775, 3.843993, [0.967, 0.967, 0.967, 0.966, 0.964, 0.962, 0.961, 0.953, 0.945]),
            ("R7 grey r.5", resin(0.5, 0.95), 0.518957, 0.0, 2.750571, [0.315, 0.308, 0.303, 0.293, 0.280, 0.264, 0.246, 0.226, 0.210]),
            ("U7 grey r.5 c0", resin(0.5, 0.0), 0.709779, 0.538316, 2.076511, [0.517, 0.514, 0.503, 0.487, 0.468, 0.444, 0.418, 0.389, 0.357]),
            ("U4 white r.5 c0", resin(0.5, 0.0), 0.890755, 0.675574, 2.076511, [0.802, 0.795, 0.776, 0.751, 0.720, 0.683, 0.638, 0.590, 0.533]),
            ("U5 white r.75 c0", resin(0.75, 0.0), 0.801155, 0.454629, 1.552811, [0.579, 0.566, 0.546, 0.516, 0.483, 0.447, 0.410, 0.374, 0.334]),
            ("U6 white r1 c0", resin(1.0, 0.0), 0.690976, 0.03146, 2.554705, [0.393, 0.390, 0.380, 0.366, 0.347, 0.324, 0.298, 0.266, 0.226]),
        ];
        // sw5: α 0.5, IOR 1.49, flat, s = T (1 - 4) and s = 1 (5 - 8): Octane render.
        let sw5: [(f64, f64, [f64; 9]); 6] = [
            (0.25, 0.25, [0.071, 0.070, 0.069, 0.068, 0.066, 0.065, 0.064, 0.064, 0.067]),
            (0.5, 0.5, [0.262, 0.258, 0.254, 0.248, 0.240, 0.232, 0.225, 0.218, 0.212]),
            (0.75, 0.75, [0.567, 0.561, 0.550, 0.534, 0.517, 0.497, 0.477, 0.455, 0.431]),
            (0.9, 0.9, [0.801, 0.792, 0.777, 0.756, 0.729, 0.700, 0.668, 0.633, 0.594]),
            (0.25, 1.0, [0.098, 0.096, 0.095, 0.094, 0.093, 0.093, 0.095, 0.100, 0.114]),
            (0.5, 1.0, [0.274, 0.270, 0.266, 0.260, 0.253, 0.246, 0.239, 0.235, 0.236]),
        ];
        let plain = Stack { weight: 0.0, curve: Curve::Fresnel(1.49), coat_rough: 0.5, base: Scatter::ReflectTransmit, base_rough: 0.5, ior: 1.49, thin: false };
        let tr = octane_trace(&plain, std::env::var("EXC").is_ok(), false);
        for (t, sp, oct) in sw5 {
            let mut r = [0.0; 9];
            for p in &tr.paths {
                let j = (p.rr * 10.0) as usize;
                if j < 9 {
                    // reflections tinted s, refractions T (1 - F s) / (1 - F): with s = 1 every reflection untinted
                    let refl = p.m - p.fr.len() as i32;
                    let w = p.w * sp.powi(refl) * t.powi(p.fr.len() as i32) * p.fr.iter().map(|f| (1.0 - f * sp) / (1.0 - f)).product::<f64>();
                    r[j] += w;
                }
            }
            let v: Vec<String> = (0..9).map(|j| format!("{:.2}", oct[j] / (r[j] / tr.entered[j].max(1) as f64))).collect();
            println!("sw5 T {t} s {sp}: render/model {}", v.join(" "));
        }
        for (lab, st, t0, t90, sk, oct) in cases {
            let q = t90 / t0;
            let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" ");
            let new = rings(&st, true, t0, q, sk);
            let ratio: Vec<f64> = (0..9).map(|j| oct[j] / new[j]).collect();
            println!("{lab:18} render/new-model {}", fmt(&ratio));
        }
    }
}
