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
//!   reflection of k1 about the base's microfacet (its |cosine|: one below
//!   the macro surface counts mirrored, as Iray renders it, #207), which
//!   light leaving a rough base from inside meets past the critical angle
//!   (c = 1, the base keeps only 1 − w); `custom_curve_layer` has no IOR
//!   and takes |n · k2|.
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
/// max(c(n·k1), c(nk2)), nk2 the |cosine| of k1's reflection about the
/// base's microfacet for a refraction under `fresnel_layer`, else |n·k2|);
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
/// (`Pw::at`). For the diagnostics (#212): the incoming cosine of each base
/// event that took `legacy_loss` (a refit reweights the paths by it), the
/// microfacet's |v·h| of each refraction, and the number of events.
struct Path {
    w: f64,
    m: i32,
    rr: f64,
    through: Vec<f64>,
    fr: Vec<f64>,
    #[cfg_attr(not(test), allow(dead_code))]
    lost: Vec<f64>,
    #[cfg_attr(not(test), allow(dead_code))]
    through_h: Vec<f64>,
    #[cfg_attr(not(test), allow(dead_code))]
    events: usize,
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
    trace_c(model, s, disk, legacy, paths, &|_| 1.0)
}

/// `trace_n` with Octane's coating colour × `coat(n · k1)` (`Smith` only):
/// a coating below 1 is picked that much less and the base takes the rest,
/// 1 − c · F (#211: a white Lambertian under a coating of 0.5 stays 1.006
/// in the furnace, as under 1 and 0; a coating of 0.5 over black reflects
/// half of F).
fn trace_c(model: Model, s: &Stack, disk: f64, legacy: bool, paths: usize, coat: &dyn Fn(f64) -> f64) -> Traced {
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
        let mut lost = Vec::new();
        let mut through_h = Vec::new();
        let mut events = 0;
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
                        // The coat is the module's `mdl 1.0` `simple_glossy_bsdf` too:
                        // a reflect-only legacy lobe loses `legacy_loss` as
                        // the base does (#205: Calib-Furnace-MdlLobe's 1.0
                        // reflect spheres, r 0.2 - 0.5, within 1 - 3 % per
                        // ring with it, 2 - 18 % over without).
                        let g = if legacy { legacy_loss(s.coat_rough, nk1) } else { 1.0 };
                        lobe(model, s.coat_rough, n, v, hc, eta, Scatter::Reflect, s.thin, &mut rng).map(|(l, w, _)| (l, w * s.weight * s.curve.eval(!outside, dot(v, hc)) / p_layer * g, false, 0))
                    } else {
                        lobe(model, s.base_rough, n, v, h, eta, s.base, s.thin, &mut rng).map(|(l, w, refr)| {
                            let nk2 = match s.curve {
                                // |nk2|: a reflection about the microfacet that points
                                // below the macro surface counts at its mirrored cosine.
                                // `libbsdf` master would saturate it to c = 1 (the base
                                // keeps 1 - w); Iray doesn't (#207: Calib-Furnace-
                                // MdlFresnel, every Fresnel sphere at 1.0 and 1.3 and
                                // r 0.3 - 1 within 1 % of the Schlick / lobe controls
                                // with it, 3 - 7 % under without; G8 0.396 -> 0.408 v
                                // 0.412, resin R7 0.972 -> 0.993). Measured, not fitted.
                                Curve::Fresnel(_) if refr => (2.0 * dot(v, h) * dot(n, h) - nk1).abs(),
                                _ => dot(n, l).abs(),
                            };
                            let base = 1.0 - s.weight * s.curve.eval(!outside, nk1).max(s.curve.eval(!outside, nk2));
                            let g = if legacy { legacy_loss(s.base_rough, nk1) } else { 1.0 };
                            (l, w * base * g / (1.0 - p_layer), refr, 1)
                        })
                    }
                }
                Model::Smith => {
                    if rng.next() < s.weight * coat(nk1) * s.curve.eval(!outside, dot(v, hc)) {
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
            if legacy && tinted == 1 && matches!(model, Model::Libbsdf) {
                lost.push(nk1);
            }
            thr *= w;
            events += 1;
            m += tinted;
            if refr {
                through.push(nk1);
                through_h.push(dot(v, h).abs());
                inside = !inside;
            }
            if !inside {
                out.push(Path { w: thr, m, rr, through, fr: std::mem::take(&mut fr), lost: std::mem::take(&mut lost), through_h: std::mem::take(&mut through_h), events });
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
/// Rows 0.7 / 0.75 refitted again under today's trace (#212, `diag212`:
/// the same rings, each event's cosine reweighted, Nelder – Mead): #191's
/// rows left K3 2.0 % and U5 4.6 % over Iray at the rim, the refit 0.7 /
/// 0.5 % on every ring (summed squares 0.0017 → 0.00014, 0.0029 →
/// 0.00004); rows 0.6 and 1 moved within their IOR spread (−5 %, −4 %, the
/// worst ring either way) and stand.
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
    (0.7, 0.875, 0.597, 1.659),
    (0.75, 0.860, 0.545, 1.719),
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
    octane_trace_c(octane, excess, fake_shadows, &|_| 1.0)
}

/// `octane_trace` with the coating × `coat(n · k1)` (`trace_c`).
fn octane_trace_c(octane: &Stack, excess: bool, fake_shadows: bool, coat: &dyn Fn(f64) -> f64) -> Traced {
    let mut t = trace_c(Model::Smith, octane, 1.0, false, PATHS, coat);
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
    /// The coating × `coat` (head-on, grazing, skew; a Falloff Map as the
    /// transmission's): what Iray's `mdl 1.0` coat keeps over Octane's
    /// (`coat_falloff`); (1, 1, 1) for none.
    pub coat: CoatFalloff,
}

/// The coating's factor, head-on → grazing as (θ / 90°)^skew
/// (`falloff_curve`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoatFalloff {
    pub head: f32,
    pub grazing: f32,
    pub skew: f32,
}

impl CoatFalloff {
    pub const NONE: CoatFalloff = CoatFalloff { head: 1.0, grazing: 1.0, skew: 1.0 };

    /// The factor at incoming cosine c.
    pub fn at(&self, c: f64) -> f64 {
        let t = c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2;
        self.head as f64 + (self.grazing - self.head) as f64 * falloff_curve(t, self.skew as f64)
    }

    pub fn is_none(&self) -> bool {
        *self == CoatFalloff::NONE
    }
}

/// One reflection off the coat at incoming cosine `c`, outside, white:
/// what it keeps (w · c(k · h) × the lobe; Iray's `mdl 1.0` lobe at
/// `legacy_rough` × `legacy_loss` when `legacy`).
fn coat_albedo(model: Model, s: &Stack, c: f64, legacy: bool, n: usize) -> f64 {
    let mut rng = Rng(0x5eed_0211);
    let nn: V = [0.0, 0.0, 1.0];
    let v: V = [(1.0 - c * c).max(0.0).sqrt(), 0.0, c];
    let d = if legacy { legacy_rough(s.coat_rough) } else { s.coat_rough };
    let g = if legacy { legacy_loss(s.coat_rough, c) } else { 1.0 };
    let mut sum = 0.0;
    for _ in 0..n {
        let h = sample_h(model, d, nn, &mut rng);
        if let Some((_, w, _)) = lobe(model, s.coat_rough, nn, v, h, s.ior, Scatter::Reflect, false, &mut rng) {
            sum += w * s.curve.eval(false, dot(v, h)) * g;
        }
    }
    s.weight * sum / n as f64
}

/// Octane's coating × this (by the hit's angle) keeps what Iray's coat of
/// a closed `mdl 1.0` stack keeps, one reflection at a time (#211). The
/// coat is the module's legacy lobe: `libbsdf` at `legacy_rough` ×
/// `legacy_loss` (#205: Calib-Furnace-MdlLobe within 1 - 3 % per ring);
/// Octane's is Beckmann + Smith at the export's α, w · F. Their ratio is
/// computed per incoming angle and drawn as a Falloff Map (least squares
/// over θ, each angle by its share of a sphere's silhouette, sin θ cos θ;
/// at most 1). Derived from the two models, not fitted to a render: at r
/// 0.5 Octane's coat keeps 1 / 0.85 of Iray's head-on (Calib-Furnace-Resin
/// R7's coat alone: 0.038 v Iray's model 0.031), the ratio near 1 only past
/// θ ≈ 75°.
pub fn coat_falloff(iray: &Stack, octane: &Stack) -> CoatFalloff {
    if iray.thin || iray.weight <= 0.0 || octane.weight <= 0.0 {
        return CoatFalloff::NONE;
    }
    // The lobes only: both under Iray's curve. Architectural's Schlick
    // curve against Octane's Fresnel at the F0's IOR is a mirror coat's
    // difference, not the legacy lobe's (within 4 %, G7 already 0.648 v
    // 0.648, #206).
    let lobe_only = Stack { curve: iray.curve, ..*octane };
    const N: usize = 24;
    let pts: Vec<(f64, f64, f64)> = (0..N)
        .map(|i| {
            let t = (i as f64 + 0.5) / N as f64;
            let th = t * std::f64::consts::FRAC_PI_2;
            let c = th.cos();
            let ratio = (coat_albedo(Model::Libbsdf, iray, c, true, 1 << 14) / coat_albedo(Model::Smith, &lobe_only, c, false, 1 << 14).max(1e-12)).min(1.0);
            (t, ratio, th.sin() * c)
        })
        .collect();
    let err = |x: &[f64; 3]| {
        let (h, g, k) = (x[0], x[1], x[2]);
        if !(0.0..=1.0).contains(&h) || !(0.0..=1.0).contains(&g) || !(0.2..=8.0).contains(&k) {
            return f64::INFINITY;
        }
        pts.iter().map(|&(t, r, w)| w * (h + (g - h) * falloff_curve(t, k) - r).powi(2)).sum()
    };
    let [h, g, k] = nelder_mead(err, [pts[0].1, pts[N - 1].1, 2.0]);
    // Within 2 % of 1 is a fraction of a percent of a reflection of a few
    // percent: no node (r ≤ 0.1, where the table's own rows are within 2 %).
    if (h - 1.0).abs() < 0.02 && (g - 1.0).abs() < 0.02 {
        return CoatFalloff::NONE;
    }
    CoatFalloff { head: h as f32, grazing: g as f32, skew: k as f32 }
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
        return Falloff { k: tint_factor_measured(iray, octane, lobe_albedo, lobe_ior, tint), grazing: 1.0, skew: 1.0, coat: CoatFalloff::NONE };
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
    // Octane's coating drawn down to Iray's legacy coat first (#211), so the
    // base's shape and k solve under the coat the export writes.
    let coat = coat_falloff(iray, octane);
    // Resin and architectural glass, the callers, export no fake shadows (#201).
    let op = octane_trace_c(octane, excess, false, &|c| coat.at(c));
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
    let shape = |q: f64, s: f64, p: usize| angles[p].iter().map(|&t| 1.0 + (q - 1.0) * falloff_curve(t, s)).product::<f64>();
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
    (Falloff { k, grazing: q as f32, skew: s as f32, coat }, target, rings)
}

/// Octane's Falloff Map (normal vs. eye ray) at the angle t = θ / 90° and
/// skew s: t^s for s ≥ 1, 1 − (1 − t)^(1 / s) below (#205: a Lambertian
/// sphere's albedo under the map in the white furnace, skews 0.5 – 4,
/// every ring within 0.006 of these; OTOY's manual has the two branches the
/// other way round).
pub fn falloff_curve(t: f64, s: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if s >= 1.0 { t.powf(s) } else { 1.0 - (1.0 - t).powf(1.0 / s) }
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
        let ks = tint_factor_measured(&iray(sch), &octane(iray(sch)), 0.609, 1.5, [1.0; 3]);
        let kf = tint_factor_measured(&iray(Curve::Fresnel(1.5)), &octane(iray(Curve::Fresnel(1.5))), 0.609, 1.5, [1.0; 3]);
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
    fn octanes_rough_coat_is_drawn_down_to_irays_legacy_coat() {
        // #211: Octane's coat over Iray's `mdl 1.0` one, one reflection by
        // angle, from the two models (Calib-Furnace-Resin R7's coat alone:
        // Octane 0.038 head-on v Iray's model 0.031 before, 0.032 after).
        let resin = |r: f64, w: f64, alpha: bool| {
            let a = if alpha { crate::material::legacy_phong_alpha(r as f32) as f64 } else { r };
            Stack { weight: w, curve: Curve::Fresnel(1.49), coat_rough: a, base: Scatter::ReflectTransmit, base_rough: a, ior: 1.49, thin: false }
        };
        let c = coat_falloff(&resin(0.5, 0.95, false), &resin(0.5, 0.95, true));
        assert!((0.80..0.88).contains(&c.head) && c.grazing > 0.95, "{c:?}");
        // Head-on the factor, rising to 1 towards grazing.
        assert!(c.at(1.0) < c.at(0.3) && c.at(0.3) <= 1.0);
        // A smooth coat, an uncoated resin, a thin wall: none.
        assert!(coat_falloff(&resin(0.02, 0.95, false), &resin(0.02, 0.95, true)).is_none());
        assert!(coat_falloff(&resin(0.5, 0.0, false), &resin(0.5, 0.0, true)).is_none());
        assert!(coat_falloff(&Stack { thin: true, ..resin(0.5, 0.95, false) }, &resin(0.5, 0.95, true)).is_none());
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
                let sh: f64 = p.through.iter().map(|c| 1.0 + (q - 1.0) * falloff_curve(c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2, s)).product();
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

#[cfg(test)]
mod diag207 {
    use super::*;

    /// How the base's layer weight 1 − w · c(·) is taken (#207 hypotheses).
    #[derive(Clone, Copy, Debug)]
    enum Wt {
        /// `curve_layer_sample` as modelled: max(c(n·k1), c(nk2)), nk2 the
        /// reflection of k1 about the microfacet for a refraction.
        Max,
        /// c(n·k1) only (energy-conserving: the coat's pick probability).
        K1,
        /// max(c(n·k1), c(|n·k2|)), k2 the refracted direction itself.
        K2Out,
        /// nk2 on the refraction, but the curve at nk2 taken from the side
        /// k2 leaves to (outside's IOR for an exit).
        Nk2Other,
        /// max(c(n·k1), c(|n·k2|)), k2 the refracted direction, the curve at
        /// k1's side (`libbsdf` up to 2020: no reflected nk2).
        K2Same,
        /// `Max` with nk2 mirrored when the reflection about the microfacet
        /// points below the macro surface (c(|nk2|)).
        AbsNk2,
        /// `Max`, but no nk2 term when that reflection points below it.
        PosNk2,
    }

    /// Where `legacy_loss` is drawn (#207 hypotheses).
    #[derive(Clone, Copy, Debug)]
    enum Lg {
        /// Every base event at its incoming angle (the model).
        All,
        /// Refractions only (a total reflection inside loses nothing).
        Refr,
        /// Every base event at the head-on value.
        Head,
        /// Exits at their angle, entries head-on, total reflections nothing.
        Exit,
        /// The same g per event, but as the death of the samples with the
        /// widest microfacets (u of the h sample above g), not a factor.
        Tail,
    }

    /// The libbsdf Iray model of a closed coated transmit stack, `legacy`
    /// as `trace_n`, with the base's weight by `wt`; rings + disk at
    /// base factor tau.
    fn rings(s: &Stack, wt: Wt, lg: Lg, tau: f64, paths: usize) -> ([f64; 9], f64) {
        rings_t(s, wt, lg, tau, 1.0, false, paths)
    }

    /// `rings` at base tint `t` (every base event), `coat_loss`: the coat
    /// loses `legacy_loss` at its roughness (#205's patch).
    #[allow(clippy::too_many_arguments)]
    fn rings_t(s: &Stack, wt: Wt, lg: Lg, tau: f64, t: f64, coat_loss: bool, paths: usize) -> ([f64; 9], f64) {
        rings_v(s, wt, lg, tau, t, coat_loss, true, paths)
    }

    /// `rings_t`; `legacy` false: the `mdl 1.3` lobe (`libbsdf` at r, no loss).
    #[allow(clippy::too_many_arguments)]
    fn rings_v(s: &Stack, wt: Wt, lg: Lg, tau: f64, t: f64, coat_loss: bool, legacy: bool, paths: usize) -> ([f64; 9], f64) {
        let shared = s.coat_rough == s.base_rough;
        let (base_d, coat_d) = if legacy { (legacy_rough(s.base_rough), legacy_rough(s.coat_rough)) } else { (s.base_rough, s.coat_rough) };
        let mut rng = Rng(0x5eed_0113);
        let (mut r, mut e) = ([0.0; 9], [0usize; 9]);
        let (mut disk, mut nd) = (0.0, 0usize);
        for _ in 0..paths {
            let rr = rng.next().sqrt();
            let j = ((rr * 10.0) as usize).min(9);
            if j < 9 {
                e[j] += 1;
            }
            if rr < 0.75 {
                nd += 1;
            }
            let phi = 2.0 * std::f64::consts::PI * rng.next();
            let mut p: V = [rr * phi.cos(), rr * phi.sin(), (1.0 - rr * rr).max(0.0).sqrt()];
            let mut d: V = [0.0, 0.0, -1.0];
            let (mut thr, mut inside) = (1.0, false);
            for _ in 0..DEPTH {
                let n = if inside { scale(p, -1.0) } else { p };
                let v = scale(d, -1.0);
                let eta = if !inside { s.ior } else { 1.0 / s.ior };
                let nk1 = dot(n, v).clamp(0.0, 1.0);
                let u_peek = Rng(rng.0).next();
                let h = sample_h(Model::Libbsdf, base_d, n, &mut rng);
                let hc = if shared { h } else { sample_h(Model::Libbsdf, coat_d, n, &mut rng) };
                let p_layer = s.weight * s.curve.eval(inside, nk1);
                let ev = if rng.next() < p_layer {
                    lobe(Model::Libbsdf, s.coat_rough, n, v, hc, eta, Scatter::Reflect, false, &mut rng).map(|(l, w, _)| (l, w * s.weight * s.curve.eval(inside, dot(v, hc)) / p_layer * if coat_loss { legacy_loss(s.coat_rough, nk1) } else { 1.0 }, false))
                } else {
                    lobe(Model::Libbsdf, s.base_rough, n, v, h, eta, s.base, false, &mut rng).map(|(l, w, refr)| {
                        let c1 = s.curve.eval(inside, nk1);
                        let nk2m = 2.0 * dot(v, h) * dot(n, h) - nk1;
                        let c2 = match (wt, s.curve) {
                            (Wt::K1, _) => 0.0,
                            (Wt::Max, Curve::Fresnel(_)) if refr => s.curve.eval(inside, nk2m),
                            (Wt::Nk2Other, Curve::Fresnel(_)) if refr => s.curve.eval(!inside, nk2m),
                            (Wt::K2Out, _) if refr => s.curve.eval(!inside, dot(n, l).abs()),
                            (Wt::K2Same, _) => s.curve.eval(inside, dot(n, l).abs()),
                            (Wt::AbsNk2, Curve::Fresnel(_)) if refr => s.curve.eval(inside, nk2m.abs()),
                            (Wt::PosNk2, Curve::Fresnel(_)) if refr => if nk2m > 0.0 { s.curve.eval(inside, nk2m) } else { 0.0 },
                            _ => s.curve.eval(inside, dot(n, l).abs()),
                        };
                        let base = 1.0 - s.weight * c1.max(c2);
                        let g = if !legacy { 1.0 } else { match lg {
                            Lg::All => legacy_loss(s.base_rough, nk1),
                            Lg::Refr => if refr { legacy_loss(s.base_rough, nk1) } else { 1.0 },
                            Lg::Head => legacy_loss(s.base_rough, 1.0),
                            Lg::Tail => if u_peek > legacy_loss(s.base_rough, nk1) { 0.0 } else { 1.0 },
                            Lg::Exit => if refr && inside { legacy_loss(s.base_rough, nk1) } else if refr { legacy_loss(s.base_rough, 1.0) } else { 1.0 },
                        } };
                        (l, w * base * g * tau * t / (1.0 - p_layer), refr)
                    })
                };
                let Some((l, w, refr)) = ev else { break };
                thr *= w;
                if refr {
                    inside = !inside;
                }
                if !inside {
                    if j < 9 {
                        r[j] += thr;
                    }
                    if rr < 0.75 {
                        disk += thr;
                    }
                    break;
                }
                let l = norm(l);
                p = norm(add(p, scale(l, -2.0 * dot(p, l))));
                d = l;
            }
        }
        for j in 0..9 {
            r[j] /= e[j].max(1) as f64;
        }
        (r, disk / nd.max(1) as f64)
    }

    #[test]
    #[ignore]
    fn print_resin_tail() {
        // Calib-Furnace-Resin / -Resin2, Iray Server 5000, projected discs.
        let lobe = [(0.02, 0.969), (0.1, 0.962), (0.25, 0.913), (0.5, 0.687), (0.75, 0.452), (1.0, 0.317)];
        let cases: [(&str, f64, f64, f64, f64, [f64; 9]); 9] = [
            ("R1 grey r.25 c.95", 0.25, 0.95, 0.787, 0.472, [0.538, 0.534, 0.522, 0.507, 0.489, 0.465, 0.438, 0.408, 0.357]),
            ("R3 white r.25 c.95", 0.25, 0.95, 1.0, 0.721, [0.834, 0.823, 0.804, 0.780, 0.750, 0.712, 0.666, 0.608, 0.508]),
            ("R7 grey r.5 c.95", 0.5, 0.95, 0.787, 0.256, [0.281, 0.281, 0.272, 0.271, 0.264, 0.257, 0.245, 0.216, 0.173]),
            ("R8 grey r.1 c.95", 0.1, 0.95, 0.787, 0.602, [0.639, 0.626, 0.624, 0.624, 0.618, 0.606, 0.584, 0.550, 0.502]),
            ("R6 white r.25 c0", 0.25, 0.0, 1.0, 0.912, [0.959, 0.943, 0.943, 0.938, 0.924, 0.915, 0.892, 0.852, 0.799]),
            ("U4 white r.5 c0", 0.5, 0.0, 1.0, 0.687, [0.783, 0.778, 0.763, 0.742, 0.715, 0.679, 0.636, 0.583, 0.517]),
            ("U5 white r.75 c0", 0.75, 0.0, 1.0, 0.452, [0.538, 0.535, 0.521, 0.503, 0.476, 0.445, 0.407, 0.363, 0.312]),
            ("U6 white r1 c0", 1.0, 0.0, 1.0, 0.317, [0.376, 0.375, 0.367, 0.353, 0.334, 0.311, 0.284, 0.252, 0.215]),
            ("U7 grey r.5 c0", 0.5, 0.0, 0.787, 0.425, [0.497, 0.486, 0.477, 0.464, 0.443, 0.419, 0.389, 0.352, 0.306]),
        ];
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" ");
        for (wt, lg) in [(Wt::Max, Lg::All), (Wt::Max, Lg::Tail), (Wt::AbsNk2, Lg::All)] {
            for coat_loss in [false, true] {
                for (lab, r, w, t, idisk, ir) in cases {
                    let st = |w| Stack { weight: w, curve: Curve::Fresnel(1.49), coat_rough: r, base: Scatter::ReflectTransmit, base_rough: r, ior: 1.49, thin: false };
                    let tau = solve_in(|x| rings(&st(0.0), wt, lg, x, 1 << 15).1, interp(&lobe, r), 1.5);
                    let (m, disk) = rings_t(&st(w), wt, lg, tau, t, coat_loss, PATHS);
                    let ratio: Vec<f64> = (0..9).map(|j| m[j] / ir[j]).collect();
                    println!("{wt:?} {lg:?} coat_loss {coat_loss} {lab:20} tau {tau:.3} disk {disk:.3} v {idisk:.3} ({:.3})  {}", disk / idisk, fmt(&ratio));
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn print_mdl_fresnel_predictions() {
        // Calib-Furnace-MdlFresnel's spheres: (label, weight, curve, r, mdl 1.0).
        let sch = Curve::Schlick { r0: 0.04, r90: 1.0, exponent: 5.0 };
        let fr = Curve::Fresnel(1.5);
        let cases = [("F1 1.0 w.6 r.6", 0.6, fr, 0.6, true), ("F2 1.3 w.6 r.6", 0.6, fr, 0.6, false), ("F3 1.0 w1 r.6", 1.0, fr, 0.6, true), ("F4 1.3 w1 r.6", 1.0, fr, 0.6, false),
            ("F5 1.0 w.6 r.3", 0.6, fr, 0.3, true), ("F6 1.0 w.6 r1", 0.6, fr, 1.0, true), ("F7 1.0 w0 r.6", 0.0, fr, 0.6, true), ("F8 1.0 schlick .04 w.6 r.6", 0.6, sch, 0.6, true)];
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        for (lab, w, curve, r, legacy) in cases {
            let st = |w| Stack { weight: w, curve, coat_rough: 0.0, base: Scatter::Transmit, base_rough: r, ior: 1.5, thin: false };
            let lobe = rings_v(&st(0.0), Wt::Max, Lg::All, 1.0, 1.0, false, legacy, 1 << 15).1;
            let tau = solve_in(|x| rings_v(&st(0.0), Wt::Max, Lg::Tail, x, 1.0, false, legacy, 1 << 15).1, lobe, 1.5);
            let (a, da) = rings_v(&st(w), Wt::Max, Lg::All, 1.0, 1.0, false, legacy, PATHS);
            let (b, db) = rings_v(&st(w), Wt::Max, Lg::Tail, tau, 1.0, false, legacy, PATHS);
            println!("{lab:28} factor disk {da:.3} [{}] | tail disk {db:.3} [{}]", fmt(&a), fmt(&b));
        }
    }

    #[test]
    #[ignore]
    fn print_mdl_fresnel_vs_iray() {
        // Calib-Furnace-MdlFresnel, Iray Server 5000, projected discs (0d43a45).
        let sch = Curve::Schlick { r0: 0.04, r90: 1.0, exponent: 5.0 };
        let fr = Curve::Fresnel(1.5);
        let cases = [
            ("F1 1.0 w.6 r.6", 0.6, fr, 0.6, true, 0.404, [0.458, 0.447, 0.441, 0.429, 0.415, 0.396, 0.378, 0.371, 0.363]),
            ("F2 1.3 w.6 r.6", 0.6, fr, 0.6, false, 0.492, [0.511, 0.511, 0.505, 0.497, 0.492, 0.484, 0.483, 0.500, 0.520]),
            ("F3 1.0 w1 r.6", 1.0, fr, 0.6, true, 0.277, [0.300, 0.300, 0.300, 0.291, 0.289, 0.281, 0.264, 0.223, 0.171]),
            ("F4 1.3 w1 r.6", 1.0, fr, 0.6, false, 0.313, [0.321, 0.321, 0.321, 0.321, 0.321, 0.321, 0.311, 0.260, 0.194]),
            ("F5 1.0 w.6 r.3", 0.6, fr, 0.3, true, 0.723, [0.817, 0.804, 0.791, 0.774, 0.749, 0.718, 0.676, 0.628, 0.594]),
            ("F6 1.0 w.6 r1", 0.6, fr, 1.0, true, 0.216, [0.241, 0.235, 0.234, 0.226, 0.219, 0.210, 0.207, 0.205, 0.208]),
            ("F7 1.0 w0 r.6", 0.0, fr, 0.6, true, 0.600, [0.701, 0.695, 0.679, 0.658, 0.628, 0.592, 0.547, 0.492, 0.423]),
            ("F8 1.0 schlick w.6 r.6", 0.6, sch, 0.6, true, 0.602, [0.710, 0.700, 0.683, 0.662, 0.633, 0.595, 0.547, 0.492, 0.429]),
        ];
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" ");
        for wt in [Wt::Max, Wt::AbsNk2, Wt::PosNk2] {
            for (lab, w, curve, r, legacy, idisk, ir) in cases {
                let st = Stack { weight: w, curve, coat_rough: 0.0, base: Scatter::Transmit, base_rough: r, ior: 1.5, thin: false };
                let (m, disk) = rings_v(&st, wt, Lg::All, 1.0, 1.0, false, legacy, PATHS);
                let ratio: Vec<f64> = (0..9).map(|j| m[j] / ir[j]).collect();
                println!("{wt:?} {lab:24} disk {disk:.3} v {idisk:.3} ({:.3})  {}", disk / idisk, fmt(&ratio));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_g8_decomposition() {
        // Calib-Furnace-ArchGlass, Iray Server 5000, projected discs (#206).
        let iray: [(&str, f64, [f64; 9]); 3] = [
            ("G4", 0.609, [0.717, 0.703, 0.689, 0.667, 0.638, 0.601, 0.556, 0.500, 0.431]),
            ("G7", 0.648, [0.749, 0.735, 0.723, 0.704, 0.676, 0.642, 0.597, 0.545, 0.484]),
            ("G8", 0.412, [0.459, 0.459, 0.448, 0.438, 0.423, 0.404, 0.387, 0.376, 0.374]),
        ];
        let st = |w: f64, curve| Stack { weight: w, curve, coat_rough: 0.0, base: Scatter::Transmit, base_rough: 0.6, ior: 1.5, thin: false };
        let sch = Curve::Schlick { r0: 0.2, r90: 1.0, exponent: 5.0 };
        let fr = Curve::Fresnel(1.5);
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        for (wt, lg) in [(Wt::Max, Lg::All), (Wt::Max, Lg::Tail), (Wt::AbsNk2, Lg::All)] {
            let tau = solve_in(|t| rings(&st(0.0, fr), wt, lg, t, 1 << 15).1, 0.609, 1.5);
            for (lab, stack) in [("G4", st(0.0, fr)), ("G7", st(0.6, sch)), ("G8", st(0.6, fr))] {
                let (_, idisk, ir) = iray.iter().find(|x| x.0 == lab).unwrap();
                let (r, disk) = rings(&stack, wt, lg, tau, PATHS);
                let ratio: Vec<f64> = (0..9).map(|j| r[j] / ir[j]).collect();
                println!("{wt:?} {lg:?} tau {tau:.3} {lab}: disk {disk:.3} v {idisk:.3}  model/Iray {}", fmt(&ratio));
            }
        }
    }
}

#[cfg(test)]
mod diag211 {
    use super::*;

    /// Per-ring mean of `f` over a trace's escaping paths (tenths of the
    /// silhouette to 0.9, by the paths that entered each).
    fn rings_of(t: &Traced, f: &dyn Fn(&Path) -> f64) -> [f64; 9] {
        let mut r = [0.0; 9];
        for p in &t.paths {
            let j = (p.rr * 10.0) as usize;
            if j < 9 {
                r[j] += f(p);
            }
        }
        std::array::from_fn(|j| r[j] / t.entered[j].max(1) as f64)
    }

    /// A resin stack at MDL roughness r (Iray) or the export's α (Octane).
    fn resin(r: f64, w: f64, alpha: bool) -> Stack {
        let a = if alpha { crate::material::legacy_phong_alpha(r as f32) as f64 } else { r };
        Stack { weight: w, curve: Curve::Fresnel(1.49), coat_rough: a, base: Scatter::ReflectTransmit, base_rough: a, ior: 1.49, thin: false }
    }

    fn fmt(a: &[f64]) -> String {
        a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ")
    }

    #[test]
    #[ignore]
    fn print_coat_alone() {
        // The coat's single reflection (base tint 0) on the closed sphere:
        // Iray's model v Octane's Smith model with and without `coat_falloff`
        // (Calib-Furnace-Resin's coat-alone probes, #211).
        for (lab, r) in [("R8", 0.1), ("R1", 0.25), ("R7", 0.5)] {
            let (iray, octane) = (resin(r, 0.95, false), resin(r, 0.95, true));
            let cf = coat_falloff(&iray, &octane);
            let ip = trace_n(Model::Libbsdf, &iray, 1.0, true, 1 << 21);
            let op = trace_n(Model::Smith, &octane, 1.0, false, 1 << 21);
            let oc = trace_c(Model::Smith, &octane, 1.0, false, 1 << 21, &|c| cf.at(c));
            let coat = |p: &Path| if p.m == 0 { p.w } else { 0.0 };
            println!("{lab} {cf:?}");
            println!("{lab} Iray model          {}", fmt(&rings_of(&ip, &coat)));
            println!("{lab} Octane model        {}", fmt(&rings_of(&op, &coat)));
            println!("{lab} Octane model, falloff {}", fmt(&rings_of(&oc, &coat)));
        }
    }

    #[test]
    #[ignore]
    fn print_coat_ratio() {
        // Iray's coat / Octane's, one reflection, by θ / 90° (0 .. 1).
        for r in [0.1, 0.25, 0.5, 0.75, 1.0] {
            let (iray, octane) = (resin(r, 1.0, false), resin(r, 1.0, true));
            let line: Vec<String> = (0..=18)
                .map(|i| {
                    let c = (i as f64 / 18.0 * 0.999 * std::f64::consts::FRAC_PI_2).cos();
                    format!("{:.3}", coat_albedo(Model::Libbsdf, &iray, c, true, 1 << 18) / coat_albedo(Model::Smith, &octane, c, false, 1 << 18))
                })
                .collect();
            println!("r {r}: {} -> {:?}", line.join(" "), coat_falloff(&iray, &octane));
        }
    }

    #[test]
    #[ignore]
    fn print_resin_now() {
        // falloff_rings at today's tables, per sphere (grey 0.787 or white),
        // and R7's rings by path kind (coat alone, base reflection, through).
        let lobe = [(0.02, 0.969), (0.1, 0.962), (0.25, 0.913), (0.5, 0.687), (0.75, 0.452), (1.0, 0.317)];
        let grey = 0.787;
        for (lab, r, w, t) in [("R1", 0.25, 0.95, grey), ("R7", 0.5, 0.95, grey), ("U4", 0.5, 0.0, 1.0), ("U5", 0.75, 0.0, 1.0), ("U6", 1.0, 0.0, 1.0), ("U7", 0.5, 0.0, grey)] {
            let (iray, octane) = (resin(r, w, false), resin(r, w, true));
            let (f, target, rings) = falloff_rings(&iray, &octane, interp(&lobe, r), 1.49, [t as f32; 3], true);
            println!("{lab} {f:?}");
            println!("{lab} Iray model   {}", fmt(&target));
            println!("{lab} Octane model {}", fmt(&rings));
            if lab != "R7" {
                continue;
            }
            let lp = trace(Model::Libbsdf, &Stack { weight: 0.0, ..iray }, 0.75, true).paths.iter().map(Pw::of).collect::<Vec<_>>();
            let tau = solve_in(|x| albedo(&lp, x), interp(&lobe, r), 1.2);
            let ip = trace(Model::Libbsdf, &iray, 1.0, true);
            let op = octane_trace_c(&octane, true, false, &|c| f.coat.at(c));
            let (k, q, s) = (f.k[0] as f64, f.grazing as f64, f.skew as f64);
            let shape = |p: &Path| p.through.iter().map(|c| 1.0 + (q - 1.0) * falloff_curve(c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2, s)).product::<f64>();
            let iw = |p: &Path| p.w * (t * tau).powi(p.m);
            let ow = |p: &Path| tinted(p.w, p.m, &p.fr, t * k) * shape(p);
            println!("R7 Iray   coat      {}", fmt(&rings_of(&ip, &|p| if p.m == 0 { iw(p) } else { 0.0 })));
            println!("R7 Iray   base refl {}", fmt(&rings_of(&ip, &|p| if p.m > 0 && p.through.is_empty() { iw(p) } else { 0.0 })));
            println!("R7 Iray   through   {}", fmt(&rings_of(&ip, &|p| if !p.through.is_empty() { iw(p) } else { 0.0 })));
            println!("R7 Octane coat      {}", fmt(&rings_of(&op, &|p| if p.m == 0 && p.through.is_empty() { ow(p) } else { 0.0 })));
            println!("R7 Octane base refl {}", fmt(&rings_of(&op, &|p| if p.m > 0 && p.through.is_empty() { ow(p) } else { 0.0 })));
            println!("R7 Octane through   {}", fmt(&rings_of(&op, &|p| if !p.through.is_empty() { ow(p) } else { 0.0 })));
        }
    }
}

#[cfg(test)]
mod diag212 {
    use super::*;

    /// Iray Server 5000 rings (centre -> 0.8 - 0.9, `furnace-rings.py
    /// measure` on the references in git) of the closed `mdl 1.0` spheres
    /// at r >= 0.6: (label, r, IOR, scatter, rings).
    fn spheres() -> Vec<(&'static str, f64, f64, Scatter, [f64; 9])> {
        use Scatter::{ReflectTransmit as Rt, Transmit as T};
        vec![
            ("L1", 0.6, 1.05, T, [0.834, 0.820, 0.804, 0.797, 0.780, 0.760, 0.730, 0.688, 0.625]),
            ("L3", 0.6, 1.2, T, [0.817, 0.801, 0.785, 0.765, 0.740, 0.704, 0.659, 0.599, 0.520]),
            ("L4", 0.6, 2.0, T, [0.524, 0.517, 0.508, 0.493, 0.476, 0.452, 0.422, 0.385, 0.340]),
            ("K3", 0.7, 1.5, T, [0.609, 0.603, 0.588, 0.568, 0.540, 0.506, 0.463, 0.412, 0.348]),
            ("U5", 0.75, 1.49, Rt, [0.538, 0.535, 0.521, 0.503, 0.476, 0.445, 0.407, 0.363, 0.312]),
            ("K4", 1.0, 1.5, T, [0.404, 0.398, 0.389, 0.374, 0.354, 0.329, 0.298, 0.261, 0.216]),
            ("L2", 1.0, 1.05, T, [0.638, 0.623, 0.605, 0.585, 0.555, 0.521, 0.478, 0.427, 0.366]),
            ("L7", 1.0, 2.0, T, [0.244, 0.244, 0.237, 0.231, 0.222, 0.209, 0.193, 0.173, 0.148]),
            ("U6", 1.0, 1.49, Rt, [0.376, 0.375, 0.367, 0.353, 0.334, 0.311, 0.284, 0.252, 0.215]),
        ]
    }

    fn rings(t: &Traced, r: f64, g: &dyn Fn(f64) -> f64) -> [f64; 9] {
        let mut out = [0.0; 9];
        for p in &t.paths {
            let j = (p.rr * 10.0) as usize;
            if j < 9 {
                out[j] += p.w * p.lost.iter().map(|&c| g(c) / legacy_loss(r, c)).product::<f64>();
            }
        }
        std::array::from_fn(|j| out[j] / t.entered[j].max(1) as f64)
    }

    #[test]
    #[ignore]
    fn print_legacy_refit() {
        let sp = spheres();
        let traced: Vec<Traced> = sp
            .iter()
            .map(|&(_, r, ior, base, _)| trace_n(Model::Libbsdf, &Stack { weight: 0.0, curve: Curve::Fresnel(ior), coat_rough: r, base, base_rough: r, ior, thin: false }, 1.0, true, 1 << 19))
            .collect();
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        for row in [0.6, 0.7, 0.75, 1.0] {
            let idx: Vec<usize> = (0..sp.len()).filter(|&i| sp[i].1 == row).collect();
            let g = |x: &[f64; 3], c: f64| {
                let t = c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2;
                (x[0] + (x[1] - x[0]) * t.powf(x[2])).min(1.0)
            };
            let err = |x: &[f64; 3]| {
                if !(0.3..=1.05).contains(&x[0]) || !(0.0..=1.2).contains(&x[1]) || !(0.2..=8.0).contains(&x[2]) {
                    return f64::INFINITY;
                }
                idx.iter().map(|&i| { let m = rings(&traced[i], row, &|c| g(x, c)); (0..9).map(|j| (m[j] / sp[i].4[j] - 1.0).powi(2)).sum::<f64>() }).sum::<f64>()
            };
            let cur = LEGACY_LOBE.iter().find(|w| w.0 == row).unwrap();
            let x0 = [cur.1, cur.2, cur.3];
            let x = nelder_mead(&err, x0);
            let x = nelder_mead(&err, x);
            println!("row {row}: now {x0:?} err {:.5} -> refit [{:.3}, {:.3}, {:.3}] err {:.5}", err(&x0), x[0], x[1], x[2], err(&x));
            for &i in &idx {
                let now = rings(&traced[i], row, &|c| legacy_loss(row, c));
                let new = rings(&traced[i], row, &|c| g(&x, c));
                let q = |m: &[f64; 9]| m.iter().zip(&sp[i].4).map(|(a, b)| a / b).collect::<Vec<_>>();
                println!("  {} now / Iray   {}", sp[i].0, fmt(&q(&now)));
                println!("  {} refit / Iray {}", sp[i].0, fmt(&q(&new)));
            }
        }
    }
}

#[cfg(test)]
mod diag212_exit {
    use super::*;

    #[test]
    #[ignore]
    fn print_falloff_readings() {
        // Probe `fr` (#211): Calib-Furnace-Resin2 re-pinned, Universal rt,
        // specular 1, IOR 1.49, roughness 0.840896, 1024 raw. Octane's
        // rings: U4 flat, U5 q 0 s 1, U7 q 0 s 3.1, U8 q 0.5 s 1.
        let oct: [(&str, f64, f64, [f64; 9]); 4] = [
            ("U4", 1.0, 1.0, [0.803, 0.795, 0.783, 0.766, 0.746, 0.725, 0.704, 0.679, 0.644]),
            ("U5", 0.0, 1.0, [0.666, 0.614, 0.550, 0.480, 0.411, 0.345, 0.285, 0.234, 0.190]),
            ("U7", 0.0, 3.1, [0.792, 0.785, 0.770, 0.745, 0.711, 0.668, 0.614, 0.546, 0.450]),
            ("U8", 0.5, 1.0, [0.743, 0.714, 0.674, 0.630, 0.582, 0.534, 0.490, 0.452, 0.419]),
        ];
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        // α 0.707 is the sphere's (the pin 0.840896 is √α); 0.841 is #211's reading.
        for alpha in [0.841, 0.707] {
            let s = Stack { weight: 0.0, curve: Curve::Fresnel(1.49), coat_rough: alpha, base: Scatter::ReflectTransmit, base_rough: alpha, ior: 1.49, thin: false };
            let t = trace_n(Model::Smith, &s, 1.0, false, 1 << 20);
            let f = |q: f64, sk: f64, c: f64| 1.0 + (q - 1.0) * falloff_curve(c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2, sk);
            type Rd = fn(&Path, usize) -> f64;
            let readings: [(&str, Rd); 5] = [
                ("macro in", |p, i| p.through[i]),
                ("micro v.h", |p, i| p.through_h[i]),
                ("entry macro, exit micro", |p, i| if i == 0 { p.through[i] } else { p.through_h[i] }),
                ("entry micro, exit macro", |p, i| if i == 0 { p.through_h[i] } else { p.through[i] }),
                ("max(macro, micro)", |p, i| p.through[i].max(p.through_h[i])),
            ];
            let rings = |w: &dyn Fn(&Path) -> f64| {
                let mut r = [0.0; 9];
                for p in &t.paths {
                    let j = (p.rr * 10.0) as usize;
                    if j < 9 {
                        r[j] += w(p);
                    }
                }
                std::array::from_fn::<f64, 9, _>(|j| r[j] / t.entered[j].max(1) as f64)
            };
            let flat = rings(&|p| p.w);
            println!("alpha {alpha}: flat model {}  Octane / model {}", fmt(&flat), fmt(&std::array::from_fn::<f64, 9, _>(|j| oct[0].3[j] / flat[j])));
            for (name, rd) in readings {
                println!("  reading: {name}");
                for &(lab, q, sk, o) in &oct[1..] {
                    let m = rings(&|p| p.w * (0..p.through.len()).map(|i| f(q, sk, rd(p, i))).product::<f64>());
                    let ratio: Vec<f64> = (0..9).map(|j| (o[j] / oct[0].3[j]) / (m[j] / flat[j])).collect();
                    println!("    {lab} Octane keeps / model keeps {}", fmt(&ratio));
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn print_excess_by_path_kind() {
        // Per ring, Octane = a · (paths that never pass the surface) + b ·
        // (paths that pass it) under each falloff (macro incoming cosine),
        // least squares over U4 / U5 / U7 / U8 (probe `fr`).
        let oct: [(f64, f64, [f64; 9]); 4] = [
            (1.0, 1.0, [0.803, 0.795, 0.783, 0.766, 0.746, 0.725, 0.704, 0.679, 0.644]),
            (0.0, 1.0, [0.666, 0.614, 0.550, 0.480, 0.411, 0.345, 0.285, 0.234, 0.190]),
            (0.0, 3.1, [0.792, 0.785, 0.770, 0.745, 0.711, 0.668, 0.614, 0.546, 0.450]),
            (0.5, 1.0, [0.743, 0.714, 0.674, 0.630, 0.582, 0.534, 0.490, 0.452, 0.419]),
        ];
        // α 0.707 is the sphere's (the pin 0.840896 is √α); 0.841 is #211's reading.
        for alpha in [0.841, 0.707] {
            let s = Stack { weight: 0.0, curve: Curve::Fresnel(1.49), coat_rough: alpha, base: Scatter::ReflectTransmit, base_rough: alpha, ior: 1.49, thin: false };
            let t = trace_n(Model::Smith, &s, 1.0, false, 1 << 20);
            let f = |q: f64, sk: f64, c: f64| 1.0 + (q - 1.0) * falloff_curve(c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2, sk);
            println!("alpha {alpha}: ring  R  T  a  b  worst fit");
            for j in 0..9 {
                let n = t.entered[j].max(1) as f64;
                let cols: Vec<(f64, f64)> = oct
                    .iter()
                    .map(|&(q, sk, _)| {
                        let (mut r, mut tr) = (0.0, 0.0);
                        for p in t.paths.iter().filter(|p| (p.rr * 10.0) as usize == j) {
                            if p.through.is_empty() {
                                r += p.w;
                            } else {
                                tr += p.w * p.through.iter().map(|&c| f(q, sk, c)).product::<f64>();
                            }
                        }
                        (r / n, tr / n)
                    })
                    .collect();
                // 2x2 normal equations.
                let (mut srr, mut srt, mut stt, mut sry, mut sty) = (0.0, 0.0, 0.0, 0.0, 0.0);
                for (k, &(r, tr)) in cols.iter().enumerate() {
                    let y = oct[k].2[j];
                    srr += r * r; srt += r * tr; stt += tr * tr; sry += r * y; sty += tr * y;
                }
                let det = srr * stt - srt * srt;
                let a = (sry * stt - sty * srt) / det;
                let b = (srr * sty - srt * sry) / det;
                let worst = cols.iter().enumerate().map(|(k, &(r, tr))| ((a * r + b * tr) / oct[k].2[j] - 1.0).abs()).fold(0.0, f64::max);
                println!("  {j}  {:.3} {:.3}  {a:.3} {b:.3}  {worst:.4}", cols[0].0, cols[0].1);
            }
        }
    }

    #[test]
    #[ignore]
    fn print_excess_three_kinds() {
        // As `print_excess_by_path_kind`, with the paths that pass split into
        // the direct ones (in, out: two events) and those that reflect inside.
        let oct: [(f64, f64, [f64; 9]); 4] = [
            (1.0, 1.0, [0.803, 0.795, 0.783, 0.766, 0.746, 0.725, 0.704, 0.679, 0.644]),
            (0.0, 1.0, [0.666, 0.614, 0.550, 0.480, 0.411, 0.345, 0.285, 0.234, 0.190]),
            (0.0, 3.1, [0.792, 0.785, 0.770, 0.745, 0.711, 0.668, 0.614, 0.546, 0.450]),
            (0.5, 1.0, [0.743, 0.714, 0.674, 0.630, 0.582, 0.534, 0.490, 0.452, 0.419]),
        ];
        // α 0.707 is the sphere's (the pin 0.840896 is √α); 0.841 is #211's reading.
        for alpha in [0.841, 0.707] {
            let s = Stack { weight: 0.0, curve: Curve::Fresnel(1.49), coat_rough: alpha, base: Scatter::ReflectTransmit, base_rough: alpha, ior: 1.49, thin: false };
            let t = trace_n(Model::Smith, &s, 1.0, false, 1 << 20);
            let f = |q: f64, sk: f64, c: f64| 1.0 + (q - 1.0) * falloff_curve(c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2, sk);
            println!("alpha {alpha}: ring  R D B (flat)  a b c  worst");
            for j in 0..9 {
                let n = t.entered[j].max(1) as f64;
                let x: Vec<[f64; 3]> = oct
                    .iter()
                    .map(|&(q, sk, _)| {
                        let mut v = [0.0; 3];
                        for p in t.paths.iter().filter(|p| (p.rr * 10.0) as usize == j) {
                            let k = if p.through.is_empty() { 0 } else if p.events == 2 { 1 } else { 2 };
                            v[k] += p.w * p.through.iter().map(|&c| f(q, sk, c)).product::<f64>() / n;
                        }
                        v
                    })
                    .collect();
                // 3 unknowns, 4 equations: normal equations, Cramer.
                let mut m = [[0.0; 3]; 3];
                let mut rhs = [0.0; 3];
                for (k, xk) in x.iter().enumerate() {
                    for a in 0..3 {
                        for b in 0..3 {
                            m[a][b] += xk[a] * xk[b];
                        }
                        rhs[a] += xk[a] * oct[k].2[j];
                    }
                }
                let det3 = |m: &[[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
                let d = det3(&m);
                let sol: [f64; 3] = std::array::from_fn(|c| {
                    let mut mc = m;
                    for r in 0..3 {
                        mc[r][c] = rhs[r];
                    }
                    det3(&mc) / d
                });
                let worst = x.iter().enumerate().map(|(k, xk)| ((0..3).map(|a| sol[a] * xk[a]).sum::<f64>() / oct[k].2[j] - 1.0).abs()).fold(0.0, f64::max);
                println!("  {j}  {:.3} {:.3} {:.3}  {:.3} {:.3} {:.3}  {worst:.4}", x[0][0], x[0][1], x[0][2], sol[0], sol[1], sol[2]);
            }
        }
    }

    /// Octane's Smith model of a closed rt sphere (IOR 1.49, specular 1)
    /// whose outside hits have α `a_out` and inside hits `a_in`, the
    /// transmission × the falloff (q, s) at each refraction's incoming macro
    /// cosine: rings centre -> 0.8 - 0.9.
    pub(super) fn side_rings(a_out: f64, a_in: f64, q: f64, sk: f64, paths: usize) -> [f64; 9] {
        side_rings_g(a_out, a_in, q, sk, 0, 0, paths)
    }

    /// `side_rings` with the masking `gm` (0 Smith G1(v) G1(l), 1 G1(l) only,
    /// 2 none) and the falloff read at `rd` (0 the incoming macro cosine, 1
    /// the outgoing |n·l|, 2 the microfacet's |v·h|; 10 + k: k on outside
    /// hits only, the inside's at 0).
    pub(super) fn side_rings_g(a_out: f64, a_in: f64, q: f64, sk: f64, gm: u8, rd: u8, paths: usize) -> [f64; 9] {
        let ior = 1.49;
        let mut rng = Rng(0x5eed_0212);
        let (mut sum, mut entered) = ([0.0; 9], [0usize; 9]);
        for _ in 0..paths {
            let rr = rng.next().sqrt();
            let j = ((rr * 10.0) as usize).min(9);
            if j < 9 {
                entered[j] += 1;
            }
            let phi = 2.0 * std::f64::consts::PI * rng.next();
            let mut p: V = [rr * phi.cos(), rr * phi.sin(), (1.0 - rr * rr).max(0.0).sqrt()];
            let mut d: V = [0.0, 0.0, -1.0];
            let (mut thr, mut inside) = (1.0, false);
            for _ in 0..DEPTH {
                let n = if inside { scale(p, -1.0) } else { p };
                let v = scale(d, -1.0);
                let eta = if inside { 1.0 / ior } else { ior };
                let a = if inside { a_in } else { a_out };
                let nk1 = dot(n, v).clamp(0.0, 1.0);
                let h = sample_h(Model::Smith, a, n, &mut rng);
                let Some((l, w, refr)) = lobe(Model::Smith, a, n, v, h, eta, Scatter::ReflectTransmit, false, &mut rng) else { break };
                let (gv, gl) = (smith_g1(nk1, a), smith_g1(dot(n, l), a));
                thr *= match gm {
                    1 => w / gv,
                    2 => w / (gv * gl),
                    _ => w,
                };
                if refr {
                    let k = if rd >= 10 && inside { 0 } else { rd % 10 };
                    let c = match k {
                        1 => dot(n, norm(l)).abs(),
                        2 => dot(v, h).abs(),
                        _ => nk1,
                    };
                    thr *= 1.0 + (q - 1.0) * falloff_curve(c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2, sk);
                    inside = !inside;
                }
                if !inside {
                    if j < 9 {
                        sum[j] += thr;
                    }
                    break;
                }
                let l = norm(l);
                p = norm(add(p, scale(l, -2.0 * dot(p, l))));
                d = l;
            }
        }
        std::array::from_fn(|j| sum[j] / entered[j].max(1) as f64)
    }

    #[test]
    #[ignore]
    fn print_side_probe() {
        // Probe `side` (#212): Calib-Furnace-Resin2 re-pinned, roughness a
        // Polygon Side Mix (black a, white b), specular 1, 1024 raw.
        // The pin 0.840896 is √α (`legacy_phong_pin(1)`): α = 0.7071.
        let a = 0.7071;
        let oct: [(&str, f64, f64, f64, f64, [f64; 9]); 8] = [
            ("U1 a,a flat", a, a, 1.0, 1.0, [0.800, 0.795, 0.784, 0.769, 0.747, 0.725, 0.705, 0.683, 0.648]),
            ("U2 0,a flat", 0.0, a, 1.0, 1.0, [0.999, 0.999, 0.999, 0.999, 0.999, 0.998, 0.947, 0.737, 0.446]),
            ("U3 a,0 flat", a, 0.0, 1.0, 1.0, [0.852, 0.847, 0.836, 0.817, 0.792, 0.764, 0.734, 0.705, 0.688]),
            ("U4 0,a q0", 0.0, a, 0.0, 1.0, [0.841, 0.781, 0.708, 0.630, 0.550, 0.471, 0.380, 0.263, 0.165]),
            ("U5 a,0 q0", a, 0.0, 0.0, 1.0, [0.785, 0.708, 0.623, 0.538, 0.455, 0.378, 0.307, 0.243, 0.204]),
            ("U6 0,a q.5", 0.0, a, 0.5, 1.0, [0.917, 0.885, 0.846, 0.802, 0.757, 0.709, 0.630, 0.467, 0.284]),
            ("U7 a,0 q.5", a, 0.0, 0.5, 1.0, [0.817, 0.774, 0.724, 0.669, 0.611, 0.553, 0.496, 0.444, 0.409]),
            ("U8 a flat", a, a, 1.0, 1.0, [0.808, 0.802, 0.792, 0.775, 0.756, 0.734, 0.713, 0.694, 0.668]),
        ];
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        for (lab, b, w, q, sk, o) in oct {
            for (which, out, inn) in [("black out", b, w), ("white out", w, b)] {
                let m = side_rings(out, inn, q, sk, 1 << 20);
                println!("{lab} {which}: model {}  Octane / model {}", fmt(&m), fmt(&std::array::from_fn::<f64, 9, _>(|j| o[j] / m[j])));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_side_readings() {
        // Probe `side`, white = the outside: U2 / U4 / U6 rough outside,
        // smooth inside (flat, q 0, q 0.5); U3 / U5 / U7 the other way.
        // The pin 0.840896 is √α (`legacy_phong_pin`): α = 0.7071.
        let a = 0.7071;
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        let o2 = [0.999, 0.999, 0.999, 0.999, 0.999, 0.998, 0.947, 0.737, 0.446];
        let o4 = [0.841, 0.781, 0.708, 0.630, 0.550, 0.471, 0.380, 0.263, 0.165];
        let o6 = [0.917, 0.885, 0.846, 0.802, 0.757, 0.709, 0.630, 0.467, 0.284];
        let o3 = [0.852, 0.847, 0.836, 0.817, 0.792, 0.764, 0.734, 0.705, 0.688];
        let o5 = [0.785, 0.708, 0.623, 0.538, 0.455, 0.378, 0.307, 0.243, 0.204];
        let o7 = [0.817, 0.774, 0.724, 0.669, 0.611, 0.553, 0.496, 0.444, 0.409];
        let r = |x: &[f64; 9], y: &[f64; 9]| std::array::from_fn::<f64, 9, _>(|j| x[j] / y[j]);
        for gm in [0u8, 1, 2] {
            for (out, inn, of, oq0, oq5, lab) in [(a, 0.0, o2, o4, o6, "rough out"), (0.0, a, o3, o5, o7, "rough in")] {
                let flat = side_rings_g(out, inn, 1.0, 1.0, gm, 0, 1 << 20);
                println!("g {gm} {lab}: flat model {}  Octane / model {}", fmt(&flat), fmt(&r(&of, &flat)));
                for rd in [0u8, 1, 2] {
                    let m0 = side_rings_g(out, inn, 0.0, 1.0, gm, rd, 1 << 20);
                    let m5 = side_rings_g(out, inn, 0.5, 1.0, gm, rd, 1 << 20);
                    println!("   read {rd}: keeps, Octane / model: q0 {}  q.5 {}", fmt(&r(&r(&oq0, &of), &r(&m0, &flat))), fmt(&r(&r(&oq5, &of), &r(&m5, &flat))));
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn print_side_entry_alpha() {
        // The rough outside of probe `side` (U2 / U4 / U6) against the model
        // at other entry α (inside smooth), masking `gm`.
        let fmt = |a: &[f64]| a.iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        let o2 = [0.999, 0.999, 0.999, 0.999, 0.999, 0.998, 0.947, 0.737, 0.446];
        let o4 = [0.841, 0.781, 0.708, 0.630, 0.550, 0.471, 0.380, 0.263, 0.165];
        let o6 = [0.917, 0.885, 0.846, 0.802, 0.757, 0.709, 0.630, 0.467, 0.284];
        let r = |x: &[f64; 9], y: &[f64; 9]| std::array::from_fn::<f64, 9, _>(|j| x[j] / y[j]);
        for gm in [0u8, 2] {
            for a in [0.4, 0.5, 0.6, 0.7071] {
                let flat = side_rings_g(a, 0.0, 1.0, 1.0, gm, 0, 1 << 20);
                let m0 = side_rings_g(a, 0.0, 0.0, 1.0, gm, 0, 1 << 20);
                let m5 = side_rings_g(a, 0.0, 0.5, 1.0, gm, 0, 1 << 20);
                println!("g {gm} a {a}: flat {}  q0 {}  q.5 {}", fmt(&r(&o2, &flat)), fmt(&r(&o4, &m0)), fmt(&r(&o6, &m5)));
            }
        }
    }
}
