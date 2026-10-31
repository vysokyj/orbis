//! DAZ Studio → Iray: the parameters DAZ Studio hands Iray's MDL for a
//! material's channels — the conversion its compiled renderer plug-in
//! (`libdsp_dzirayrenderer`) does before Iray sees anything.
//!
//! **Verified, not fitted.** Every rule here reproduces DAZ's own `.mi`
//! export (`scripts/dev/daz-export-mi.dsa`) of the calibration scenes,
//! kept as `calib/references/mi/*.json` (`scripts/dev/mi-oracle.py`); the
//! test at the bottom compares each rule with every fixture. Calibration
//! (renders against Iray) never changes this module — only what the
//! Octane side does with its parameters (`crate::material`). A rule
//! changes only with a fixture that proves it (#62).
//!
//! Parameters this module doesn't produce yet are simply absent from
//! [`uber`]'s result: unverified, not defaulted.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::daz_material::Material;

/// One MDL argument as DAZ passes it.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Bool(bool),
    Int(i32),
    Float(f32),
    /// Linear RGB.
    Color([f32; 3]),
    /// An MDL enum value (`blm_layered`).
    Enum(&'static str),
    /// A texture-driven argument: the image (content path as the channel
    /// names it).
    Map(String),
}

/// The MDL material DAZ instantiates for an Iray Uber material:
/// `iray_uber_unified`, or `iray_uber` when Share Glossy Inputs is off
/// (refraction and backscattering then take their own colour and
/// roughness).
pub fn uber_entry(mat: &Material) -> &'static str {
    if Channels(mat).bool("Share Glossy Inputs", true) { "daz_3d::irayubermaterial::iray_uber_unified" } else { "daz_3d::irayubermaterial::iray_uber" }
}

/// The `iray_uber` MDL arguments for an Iray Uber material, by MDL
/// parameter name. Only the verified ones (module docs).
pub fn uber(mat: &Material) -> BTreeMap<&'static str, Arg> {
    let ch = Channels(mat);
    let mut out = BTreeMap::new();
    let mixing = ch.int("Base Mixing", 0);
    out.insert("base_mixing", Arg::Enum(if mixing == 2 { "blm_normalized" } else { "blm_layered" }));

    // Base colour: linear (a pure 2.2 power) × Diffuse Strength.
    let strength = ch.float("Diffuse Strength", 1.0);
    out.insert("diffuse_color", ch.color_or_map("diffuse", [0.5; 3], strength));
    let metallic = if mixing == 0 { ch.float("Metallic Weight", 0.0) } else { 0.0 };
    let refraction = ch.float("Refraction Weight", 0.0);
    let backscattering = ch.float("Backscattering Weight", 0.0);
    let translucency = ch.float("Translucency Weight", 0.0);
    // A layer at full weight covers the diffuse: DAZ switches it off.
    let diffuse_on = metallic < 1.0 && refraction < 1.0 && backscattering < 1.0;
    out.insert("diffuse_enable", Arg::Bool(diffuse_on));
    out.insert("diffuse_weight", Arg::Float(if mixing == 2 { ch.float("Diffuse Weight", 1.0) } else { 1.0 }));

    // Glossy.
    let weight = if mixing == 2 { ch.float("Glossy Weight", 0.0) } else { ch.float("Glossy Layered Weight", 0.0) };
    out.insert("glossy_weight", Arg::Float(if mixing == 2 { weight } else { 0.0 }));
    out.insert("glossy_layered_weight", Arg::Float(if mixing == 2 { 1.0 } else { weight }));
    out.insert("glossy_enable", Arg::Bool(diffuse_on && weight > 0.0));
    // Weighted mixing without gloss leaves the colour at the MDL default.
    let colour = if mixing == 2 && weight <= 0.0 { Arg::Color([0.5; 3]) } else { ch.color_or_map("Glossy Color", [1.0; 3], 1.0) };
    out.insert("glossy_color", colour);
    let rough = match mixing {
        1 => square_if(1.0 - ch.float("Glossiness", 1.0), ch.bool("Glossiness Squared", true)),
        _ => square_if(ch.float("Glossy Roughness", 0.0), ch.bool("Roughness Squared", true)),
    };
    out.insert("glossy_roughness", Arg::Float(rough));
    out.insert("glossy_roughness_enable", Arg::Bool(rough > 0.0));
    let reflectivity = match mixing {
        0 => REFLECTIVITY_SCALE * ch.float("Glossy Reflectivity", 0.5),
        1 => ch.color("Glossy Specular", [0.04; 3]).into_iter().fold(0.0, f32::max),
        _ => 0.04,
    };
    out.insert("glossy_reflectivity", Arg::Float(reflectivity));

    let aniso = ch.float("Glossy Anisotropy", 0.0);
    out.insert("glossy_anisotropy", Arg::Float(aniso));
    out.insert("glossy_anisotropy_enable", Arg::Bool(aniso > 0.0));
    out.insert("glossy_anisotropy_rotation", Arg::Float(if aniso > 0.0 { ch.float("Glossy Anisotropy Rotations", 0.0) } else { 0.0 }));
    out.insert("glossy_color_transmit_enable", Arg::Int(ch.int("Glossy Color Effect", 0)));

    // Base thin film.
    let base_film = ch.float("Base Thin Film", 0.0);
    out.insert("base_thin_film_thickness", Arg::Float(base_film));
    out.insert("base_thin_film_enable", Arg::Bool(base_film > 0.0));
    out.insert("base_thin_film_ior", Arg::Float(if base_film > 0.0 { ch.float("Base Thin Film IOR", 1.5) } else { 3.0 }));

    // Diffuse overlay.
    let overlay = ch.float("Diffuse Overlay Weight", 0.0);
    let overlay_on = overlay > 0.0;
    out.insert("diffuse_overlay_enable", Arg::Bool(overlay_on));
    out.insert("diffuse_overlay_weight", Arg::Float(square_if(overlay, ch.bool("Diffuse Overlay Weight Squared", false))));
    out.insert("diffuse_overlay_color", if overlay_on { ch.color_or_map("Diffuse Overlay Color", [0.5; 3], 1.0) } else { Arg::Color([0.5; 3]) });
    out.insert("diffuse_overlay_roughness", Arg::Float(if overlay_on { ch.float("Diffuse Overlay Roughness", 0.0) } else { 0.0 }));
    out.insert("diffuse_overlay_color_transmit_enable", Arg::Int(if overlay_on { ch.int("Diffuse Overlay Color Effect", 0) } else { 0 }));
    // Base Color Effect only matters where translucency mixes with the diffuse.
    out.insert("diffuse_color_transmit_enable", Arg::Int(if translucency > 0.0 { ch.int("Base Color Effect", 0) } else { 0 }));
    out.insert("diffuse_roughness", Arg::Float(ch.float("Diffuse Roughness", 0.0)));

    // Metallic flakes: roughness like the glossy's, size cm → m.
    let flakes = ch.float("Metallic Flakes Weight", 0.0);
    let on = flakes > 0.0;
    out.insert("metallic_flakes_enable", Arg::Bool(on));
    out.insert("metallic_flakes_weight", Arg::Float(flakes));
    let flake_rough = match mixing {
        1 => square_if(1.0 - ch.float("Metallic Flakes Glossiness", 1.0), ch.bool("Glossiness Squared", true)),
        _ => square_if(ch.float("Metallic Flakes Roughness", 0.0), ch.bool("Roughness Squared", true)),
    };
    out.insert("metallic_flakes_roughness", Arg::Float(if on { flake_rough } else { 0.0 }));
    out.insert("metallic_flakes_size", Arg::Float(if on { CM * ch.float("Metallic Flakes Size", 0.001) } else { 1.0 }));
    out.insert("metallic_flakes_strength", Arg::Float(if on { ch.float("Metallic Flakes Strength", 1.0) } else { 1.0 }));
    out.insert("metallic_flakes_density", Arg::Float(if on { ch.float("Metallic Flakes Density", 1.0) } else { 1.0 }));
    out.insert("metallic_flakes_color", if on { ch.color_or_map("Metallic Flakes Color", [1.0; 3], 1.0) } else { Arg::Color([1.0; 3]) });
    out.insert("metallic_flakes_color_transmit_enable", Arg::Int(if on { ch.int("Metallic Flakes Color Effect", 0) } else { 0 }));
    out.insert("metallic_flakes_thin_film_thickness", Arg::Float(if on { ch.float("Metallic Flakes Thin Film", 0.0) } else { 300.0 }));
    out.insert("metallic_flakes_thin_film_ior", Arg::Float(if on { ch.float("Metallic Flakes Thin Film IOR", 1.5) } else { 3.0 }));

    // Specular / glossiness: Glossy Specular's hue as a directional tint.
    let specular = ch.color("Glossy Specular", [0.04; 3]);
    let spec_peak = specular.into_iter().fold(0.0, f32::max);
    let hue = mixing == 1 && spec_peak > 0.0 && specular.iter().any(|c| *c < spec_peak);
    out.insert("glossy_directional_color_enable", Arg::Bool(hue));
    out.insert("glossy_directional_normal_color", Arg::Color(if hue { specular.map(|c| c / spec_peak) } else { [1.0; 3] }));

    // Emission.
    let emission = ch.color("Emission Color", [0.0; 3]);
    let emits = emission.iter().any(|c| *c > 0.0);
    out.insert("emission_tint", Arg::Color(emission));
    out.insert("emission_enable", Arg::Bool(emits));
    out.insert("emission_temperature", Arg::Float(if emits { ch.float("Emission Temperature", 6500.0) } else { 6000.0 }));
    out.insert("emission_scene_unit_size", Arg::Float(if emits { 1.0 } else { 0.01 }));
    out.insert("two_sided_area", Arg::Bool(emits && ch.bool("Two Sided Light", false)));
    // Luminance by its unit: cd/m² as is, lm as a power, W through the
    // luminous efficacy (lm/W). Other units: not verified, left out.
    let luminance = ch.float("Luminance", 1500.0);
    let units = ch.int("Luminance Units", 0);
    let flux = match units {
        0 => Some((luminance, "intensity_radiant_exitance")),
        4 => Some((luminance, "intensity_power")),
        5 => Some((luminance * ch.float("Luminous Efficacy", 15.0), "intensity_power")),
        _ => None,
    };
    if !emits {
        out.insert("emission_flux", Arg::Color([500.0; 3]));
        out.insert("emission_mode", Arg::Enum("intensity_radiant_exitance"));
    } else if let Some((flux, mode)) = flux {
        out.insert("emission_flux", Arg::Color([flux; 3]));
        out.insert("emission_mode", Arg::Enum(mode));
    }

    // Metal (metallicity / roughness only).
    out.insert("metallic_weight", Arg::Float(metallic));
    out.insert("metallic_enable", Arg::Bool(metallic > 0.0));
    // The metal's head-on reflectance is the base colour's strongest
    // channel, its hue a directional tint.
    let base = ch.color("diffuse", [0.5; 3]).map(|c| c * strength);
    let peak = base.into_iter().fold(0.0, f32::max);
    let metal = metallic > 0.0 && peak > 0.0;
    out.insert("metallic_custom_curve_normal", Arg::Float(if metal { peak } else { 0.0 }));
    // A grey metal has no hue: no directional colour.
    out.insert("metallic_directional_color_enable", Arg::Bool(metal && base.iter().any(|c| *c < peak)));
    out.insert("metallic_directional_normal_color", Arg::Color(if metal { base.map(|c| c / peak) } else { [1.0; 3] }));

    // Translucency.
    out.insert("translucency_weight", Arg::Float(translucency));
    out.insert("translucency_enable", Arg::Bool(translucency > 0.0));
    out.insert("invert_transmission_normal", Arg::Bool(translucency > 0.0));
    out.insert("translucency_color", if translucency > 0.0 { ch.color_or_map("Translucency Color", [0.5; 3], 1.0) } else { Arg::Color([0.5; 3]) });

    // Dual lobe specular.
    let lobes = ch.float("Dual Lobe Specular Weight", 0.0);
    out.insert("dual_lobe_specular_weight", Arg::Float(lobes));
    out.insert("dual_lobe_uniform_specular_weight", Arg::Float(lobes));
    let lobe_rough = |r: &str, g: &str| match mixing {
        1 => square_if(1.0 - ch.float(g, 1.0), ch.bool("Glossiness Squared", true)),
        _ => square_if(ch.float(r, 0.0), ch.bool("Roughness Squared", true)),
    };
    out.insert("dual_lobe_specular_roughness_1", Arg::Float(if lobes > 0.0 { lobe_rough("Specular Lobe 1 Roughness", "Specular Lobe 1 Glossiness") } else { 0.3 }));
    out.insert("dual_lobe_specular_roughness_2", Arg::Float(if lobes > 0.0 { lobe_rough("Specular Lobe 2 Roughness", "Specular Lobe 2 Glossiness") } else { 0.03 }));
    out.insert("dual_lobe_specular_roughness_ratio", Arg::Float(if lobes > 0.0 { ch.float("Dual Lobe Specular Ratio", 0.85) } else { 0.85 }));
    out.insert("dual_lobe_specular_reflectivity", Arg::Float(if lobes > 0.0 { REFLECTIVITY_SCALE * ch.float("Dual Lobe Specular Reflectivity", 0.5) } else { 0.04 }));

    // Backscattering.
    // Weighted mixing without gloss: the gloss part isn't passed at all.
    let gloss_part = !(mixing == 2 && weight <= 0.0);
    out.insert("backscattering_weight", Arg::Float(if gloss_part { backscattering } else { 0.5 }));
    out.insert("backscattering_enable", Arg::Bool(backscattering > 0.0));
    let back_rough = match mixing {
        1 => square_if(1.0 - ch.float("Backscattering Glossiness", 1.0), ch.bool("Glossiness Squared", true)),
        _ => square_if(ch.float("Backscattering Roughness", 0.0), ch.bool("Roughness Squared", true)),
    };
    out.insert("backscattering_roughness", Arg::Float(if backscattering > 0.0 { back_rough } else { 0.5 }));

    // Refraction.
    out.insert("refraction_weight", Arg::Float(refraction));
    out.insert("refraction_enable", Arg::Bool(refraction > 0.0));
    out.insert("glossy_ior", Arg::Float(if refraction > 0.0 { ch.float("Refraction Index", 1.5) } else { 1.5 }));
    let abbe = ch.float("Abbe", 0.0);
    out.insert("dispersion_enable", Arg::Bool(refraction > 0.0 && abbe > 0.0));
    out.insert("abbe_number", Arg::Float(if refraction > 0.0 { abbe } else { 55.0 }));
    let thin = ch.bool("Thin Walled", false);
    out.insert("thin_walled", Arg::Bool(thin));
    // The refraction reflects through the glossy layer: without gloss it
    // doesn't reflect at all; otherwise (and without refraction) DAZ asks
    // for the full Fresnel reflection, unless it splits refraction from
    // gloss (Share Glossy Inputs off).
    let share = ch.bool("Share Glossy Inputs", true);
    let reflects = !(refraction > 0.0 && weight <= 0.0);
    out.insert("refraction_reflection_enable", Arg::Bool(reflects));
    out.insert("full_refraction_reflection_enable", Arg::Bool(mixing != 2 && reflects && share));

    // Share Glossy Inputs off: DAZ switches to `iray_uber` (`uber_entry`),
    // which takes the refraction's and backscattering's own colour and
    // roughness.
    let own = !share && refraction > 0.0;
    out.insert("refraction_color", if own { ch.color_or_map("Refraction Color", [1.0; 3], 1.0) } else { Arg::Color([0.5; 3]) });
    let refr_rough = match mixing {
        1 => square_if(1.0 - ch.float("Refraction Glossiness", 1.0), ch.bool("Glossiness Squared", true)),
        _ => square_if(ch.float("Refraction Roughness", 0.0), ch.bool("Roughness Squared", true)),
    };
    out.insert("refraction_roughness", Arg::Float(if own { refr_rough } else { 0.0 }));
    out.insert("backscattering_color", if !share && backscattering > 0.0 { ch.color_or_map("Backscattering Color", [1.0; 3], 1.0) } else { Arg::Color([0.5; 3]) });

    // Volume and subsurface scattering (Calib-SSS, -SSSColor, -Glass,
    // -Leaf). The measurement distances, cm → m, and the direction only
    // inside a closed surface that lets light in; absorption and scattering
    // switch on by their colour / amount even on a thin wall.
    let sss_mode = ch.int("SSS Mode", 0);
    let sss = ch.float("SSS Amount", 0.0);
    let enters = refraction > 0.0 || translucency > 0.0 || sss > 0.0;
    let volume = enters && !thin;
    let transmitted = ch.color("Transmitted Color", [0.0; 3]);
    out.insert("transmitted_color", Arg::Color(if volume { transmitted } else { [1.0; 3] }));
    out.insert("transmitted_distance_of_measurement", Arg::Float(if volume { CM * ch.float("Transmitted Measurement Distance", 0.1) } else { 1.0 }));
    out.insert("absorption_enable", Arg::Bool(enters && transmitted.iter().any(|c| *c > 0.0)));
    out.insert("scattering_distance_of_measurement", Arg::Float(if volume { CM * ch.float("Scattering Measurement Distance", 0.1) } else { 1.0 }));
    out.insert("sub_surface_scattering_direction", Arg::Float(if volume { ch.float("SSS Direction", 0.0) } else { 0.3 }));
    out.insert("sub_surface_enable", Arg::Bool(sss > 0.0));
    out.insert("logarithmic_scatter_enable", Arg::Bool(enters && sss_mode == 1));
    // Chromatic mode: the SSS Color, linear (the amount and the reflectance
    // tint don't reach the MDL); mono: the amount, clamped to 1, as grey.
    let amount = if sss_mode == 1 { ch.color("SSS Color", [0.0; 3]) } else { [sss.clamp(0.0, 1.0); 3] };
    out.insert("sub_surface_scattering_amount", Arg::Color(if sss > 0.0 { amount } else { [0.0; 3] }));

    // Top coat.
    let coat = ch.float("Top Coat Weight", 0.0);
    out.insert("top_coat_weight", Arg::Float(coat));
    out.insert("top_coat_enable", Arg::Bool(coat > 0.0));
    let coat_rough = match mixing {
        1 => square_if(1.0 - ch.float("Top Coat Glossiness", 1.0), ch.bool("Glossiness Squared", true)),
        _ => square_if(ch.float("Top Coat Roughness", 0.0), ch.bool("Roughness Squared", true)),
    };
    out.insert("top_coat_roughness", Arg::Float(if coat > 0.0 { coat_rough } else { 0.0 }));
    out.insert("top_coat_color", if coat > 0.0 { ch.color_or_map("Top Coat Color", [1.0; 3], 1.0) } else { Arg::Color([1.0; 3]) });
    let film = if coat > 0.0 { ch.float("Top Coat Thin Film", 0.0) } else { 300.0 };
    out.insert("top_coat_thin_film_thickness", Arg::Float(film));
    out.insert("top_coat_thin_film_ior", Arg::Float(if coat > 0.0 { ch.float("Top Coat Thin Film IOR", 1.5) } else { 3.0 }));
    out.insert("top_coat_thin_film_enable", Arg::Bool(coat > 0.0 && film > 0.0));
    let coat_aniso = if coat > 0.0 { ch.float("Top Coat Anisotropy", 0.0) } else { 0.0 };
    out.insert("top_coat_anisotropy", Arg::Float(coat_aniso));
    out.insert("top_coat_anisotropy_enable", Arg::Bool(coat_aniso > 0.0));
    out.insert("top_coat_color_transmit_enable", Arg::Int(if coat > 0.0 { ch.int("Top Coat Color Effect", 0) } else { 0 }));
    const MODES: [&str; 4] = ["tcw_principled_specular", "tcw_weighted", "tcw_fresnel", "tcw_custom_curve"];
    let mode = ch.int("Top Coat Layering Mode", 0).clamp(0, 3) as usize;
    out.insert("top_coat_layer_mode", Arg::Enum(if coat > 0.0 { MODES[mode] } else { MODES[0] }));
    out.insert("top_coat_curve_strength", Arg::Float(if coat > 0.0 && mode == 0 { REFLECTIVITY_SCALE * ch.float("Reflectivity", 0.5) } else { 0.04 }));


    // Branches no fixture covers yet are left out — unverified, not
    // guessed: the other lobes' roughness outside Base Mixing 0, the top
    // coat's and the metal under Weighted mixing.
    let mut unverified: Vec<&str> = Vec::new();
    if mixing != 0 {
        if lobes > 0.0 {
            unverified.extend(["dual_lobe_specular_roughness_1", "dual_lobe_specular_roughness_2"]);
        }
        if backscattering > 0.0 {
            unverified.push("backscattering_roughness");
        }
        if on {
            unverified.push("metallic_flakes_roughness");
        }
        if own {
            unverified.push("refraction_roughness");
        }
    }
    if mixing == 2 {
        if coat > 0.0 {
            unverified.push("top_coat_roughness");
        }
        if ch.float("Metallic Weight", 0.0) > 0.0 {
            unverified.extend(["metallic_weight", "metallic_enable", "metallic_custom_curve_normal", "metallic_directional_color_enable", "metallic_directional_normal_color"]);
        }
    }
    for k in unverified {
        out.remove(k);
    }
    out
}

/// Gamma DAZ decodes a colour channel's value with before passing it on:
/// a pure power, not the sRGB curve.
pub const COLOUR_GAMMA: f32 = 2.2;

/// Glossy / Top Coat / Dual Lobe Reflectivity r → the MDL's head-on
/// reflectance 0.08 r (r 0.5, the default, is 4 %).
pub const REFLECTIVITY_SCALE: f32 = 0.08;

/// DAZ's distances are in cm, the MDL's in m.
pub const CM: f32 = 0.01;

fn square_if(x: f32, squared: bool) -> f32 {
    if squared { x * x } else { x }
}

/// A material's raw channel values, as DAZ stores them (no aliases, no
/// enable gating: those are the Octane side's business).
struct Channels<'a>(&'a Material);

impl Channels<'_> {
    fn value(&self, id: &str) -> Option<&Value> {
        self.0.channels.get(id).map(|c| &c.value)
    }

    fn image(&self, id: &str) -> Option<&str> {
        self.0.channels.get(id).and_then(|c| c.image.as_deref())
    }

    fn float(&self, id: &str, default: f32) -> f32 {
        match self.value(id) {
            Some(Value::Number(n)) => n.as_f64().map_or(default, |f| f as f32),
            Some(Value::Bool(b)) => f32::from(u8::from(*b)),
            _ => default,
        }
    }

    fn int(&self, id: &str, default: i32) -> i32 {
        self.float(id, default as f32).round() as i32
    }

    fn bool(&self, id: &str, default: bool) -> bool {
        match self.value(id) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
            _ => default,
        }
    }

    /// A colour channel, linear.
    fn color(&self, id: &str, default: [f32; 3]) -> [f32; 3] {
        match self.value(id).and_then(Value::as_array) {
            Some(a) => [0, 1, 2].map(|i| (a.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32).max(0.0).powf(COLOUR_GAMMA)),
            None => default,
        }
    }

    fn color_or_map(&self, id: &str, default: [f32; 3], scale: f32) -> Arg {
        match self.image(id) {
            Some(m) => Arg::Map(m.to_string()),
            None => Arg::Color(self.color(id, default).map(|c| c * scale)),
        }
    }
}

/// A DAZ light's kind, from its node definition (`spot`, `directional`,
/// else a point light whose Light Geometry may make it an area light).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightKind {
    Point,
    Spot,
    Distant,
}

/// The emitter shape DAZ gives Iray, in the light's own space (cm).
#[derive(Debug, Clone, PartialEq)]
pub enum LightShape {
    /// At the origin.
    Point,
    /// Along −Z.
    Distant,
    Rectangle { width: f32, height: f32 },
    Disc { radius: f32 },
    Sphere { radius: f32 },
}

/// What DAZ hands Iray for a scene light: the MDL light material, its
/// arguments, and the emitter shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    pub mdl: &'static str,
    pub args: BTreeMap<&'static str, Arg>,
    pub shape: LightShape,
}

/// Flux of a non-photometric DAZ light per unit of Intensity, lumens:
/// 1000 π, whatever its Intensity Scale (Calib-L-Point-legacy: 3141.59 at
/// Intensity 1, 6283.19 at 2; renders had fitted 3146 / 6281 lm).
pub const LEGACY_LUMENS: f32 = 1000.0 * std::f32::consts::PI;

/// A scene light (not the camera headlamp) from its channels (definition
/// overlaid by the scene node's).
pub fn light(kind: LightKind, channels: &std::collections::HashMap<String, Value>) -> Light {
    let num = |k: &str, d: f32| channels.get(k).and_then(Value::as_f64).map_or(d, |v| v as f32);
    let flag = |k: &str, d: bool| channels.get(k).and_then(Value::as_bool).unwrap_or(d);
    let photometric = flag("Photometric Mode", false);
    let mut args = BTreeMap::new();
    args.insert("flux", Arg::Float(if photometric { num("Flux", 1500.0) } else { LEGACY_LUMENS * num("Intensity", 1.0) }));
    // A non-photometric light has no colour temperature.
    args.insert("temperature", Arg::Float(if photometric { num("Temperature", 6500.0) } else { 0.0 }));
    args.insert("scene_unit_size", Arg::Float(1.0));
    let colour = channels.get("Color").and_then(Value::as_array).map_or([1.0; 3], |a| {
        [0, 1, 2].map(|i| (a.get(i).and_then(Value::as_f64).unwrap_or(1.0) as f32).max(0.0).powf(COLOUR_GAMMA))
    });
    args.insert("emission_color", Arg::Color(colour));
    let cm = |k: &str| num(k, 10.0);
    let shape = match kind {
        LightKind::Distant => LightShape::Distant,
        LightKind::Spot => LightShape::Point,
        LightKind::Point => match num("Light Geometry", 0.0) as i32 {
            1 => LightShape::Rectangle { width: cm("Width"), height: cm("Height") },
            // Disc and sphere: Height as the diameter (Width equal in every
            // fixture so far).
            2 => LightShape::Disc { radius: cm("Height") / 2.0 },
            3 => LightShape::Sphere { radius: cm("Height") / 2.0 },
            _ => LightShape::Point,
        },
    };
    let area = matches!(shape, LightShape::Rectangle { .. } | LightShape::Disc { .. });
    args.insert("two_sided_area", Arg::Bool(area && flag("Two Sided", false)));
    let mdl = if kind == LightKind::Spot {
        args.insert("spot_exponent", Arg::Float(num("Beam Exponent", 1.0)));
        // The full Spread Angle, in radians.
        args.insert("spot_spread_angle", Arg::Float(num("Spread Angle", 60.0).to_radians()));
        "daz_3d::basic::dzpm_spot_light"
    } else {
        "daz_3d::basic::dzpm_diffuse_area_light"
    };
    Light { mdl, args, shape }
}

/// The MDL default of a parameter [`uber`] produces (irayubermaterial.mdl,
/// `iray_uber_unified`): what Iray takes when DAZ doesn't pass it.
pub fn uber_default(param: &str) -> Option<Arg> {
    Some(match param {
        "base_mixing" => Arg::Enum("blm_normalized"),
        "diffuse_color" => Arg::Color([0.5; 3]),
        "diffuse_enable" => Arg::Bool(true),
        "diffuse_weight" => Arg::Float(1.0),
        "glossy_weight" => Arg::Float(0.0),
        "glossy_layered_weight" => Arg::Float(1.0),
        "glossy_enable" | "glossy_roughness_enable" | "metallic_enable" => Arg::Bool(false),
        "glossy_color" => Arg::Color([0.5; 3]),
        "glossy_roughness" | "metallic_weight" => Arg::Float(0.0),
        "glossy_reflectivity" | "dual_lobe_specular_reflectivity" | "top_coat_curve_strength" => Arg::Float(0.04),
        "metallic_custom_curve_normal" => Arg::Float(0.0),
        "metallic_directional_color_enable" | "translucency_enable" | "invert_transmission_normal" | "backscattering_enable" | "refraction_enable" | "dispersion_enable" | "thin_walled" | "top_coat_enable" => {
            Arg::Bool(false)
        }
        "metallic_directional_normal_color" | "top_coat_color" => Arg::Color([1.0; 3]),
        "translucency_weight" | "backscattering_weight" | "backscattering_roughness" => Arg::Float(0.5),
        "translucency_color" => Arg::Color([0.5; 3]),
        "dual_lobe_specular_weight" => Arg::Float(1.0),
        "dual_lobe_uniform_specular_weight" | "refraction_weight" | "top_coat_roughness" => Arg::Float(0.0),
        "dual_lobe_specular_roughness_1" => Arg::Float(0.3),
        "dual_lobe_specular_roughness_2" => Arg::Float(0.03),
        "dual_lobe_specular_roughness_ratio" => Arg::Float(0.85),
        "glossy_ior" => Arg::Float(1.5),
        "abbe_number" => Arg::Float(55.0),
        "refraction_reflection_enable" => Arg::Bool(true),
        "full_refraction_reflection_enable" | "absorption_enable" => Arg::Bool(false),
        "transmitted_color" => Arg::Color([1.0; 3]),
        "transmitted_distance_of_measurement" | "scattering_distance_of_measurement" => Arg::Float(1.0),
        "sub_surface_scattering_direction" => Arg::Float(0.3),
        "sub_surface_enable" | "logarithmic_scatter_enable" => Arg::Bool(false),
        "sub_surface_scattering_amount" => Arg::Color([0.0; 3]),
        "top_coat_thin_film_thickness" | "base_thin_film_thickness" | "metallic_flakes_thin_film_thickness" => Arg::Float(300.0),
        "top_coat_thin_film_ior" | "base_thin_film_ior" | "metallic_flakes_thin_film_ior" => Arg::Float(3.0),
        "top_coat_thin_film_enable" | "top_coat_anisotropy_enable" | "glossy_anisotropy_enable" | "base_thin_film_enable" | "diffuse_overlay_enable" | "metallic_flakes_enable" => Arg::Bool(false),
        "top_coat_anisotropy" | "glossy_anisotropy" | "glossy_anisotropy_rotation" | "diffuse_overlay_weight" | "diffuse_overlay_roughness" | "diffuse_roughness" | "metallic_flakes_weight" | "metallic_flakes_roughness" => Arg::Float(0.0),
        "top_coat_color_transmit_enable" | "glossy_color_transmit_enable" | "diffuse_overlay_color_transmit_enable" | "diffuse_color_transmit_enable" | "metallic_flakes_color_transmit_enable" => Arg::Int(0),
        "diffuse_overlay_color" => Arg::Color([0.5; 3]),
        "metallic_flakes_size" | "metallic_flakes_strength" | "metallic_flakes_density" => Arg::Float(1.0),
        "metallic_flakes_color" => Arg::Color([1.0; 3]),
        "refraction_color" | "backscattering_color" => Arg::Color([0.5; 3]),
        "refraction_roughness" => Arg::Float(0.0),
        "glossy_directional_color_enable" | "emission_enable" | "two_sided_area" => Arg::Bool(false),
        "glossy_directional_normal_color" | "emission_tint" => Arg::Color([1.0; 3]),
        "emission_temperature" => Arg::Float(6000.0),
        "emission_scene_unit_size" => Arg::Float(0.01),
        "emission_flux" => Arg::Color([500.0; 3]),
        "emission_mode" => Arg::Enum("intensity_radiant_exitance"),
        "top_coat_weight" => Arg::Float(1.0),
        "top_coat_layer_mode" => Arg::Enum("tcw_principled_specular"),
        _ => return None,
    })
}


/// An Iray Uber material's MDL arguments as DAZ passes them, with the MDL
/// defaults for the rest: what the Octane side reads instead of the
/// channels wherever this module covers them.
pub struct Uber(BTreeMap<&'static str, Arg>);

impl Uber {
    /// None unless the material is an Iray Uber one.
    pub fn of(mat: &Material) -> Option<Self> {
        is_uber(mat).then(|| Uber(uber(mat)))
    }

    /// The argument, or its MDL default when DAZ doesn't pass it; None
    /// where `uber` left it out as unverified.
    fn arg(&self, param: &str) -> Option<Arg> {
        match self.0.get(param) {
            Some(a) => Some(a.clone()),
            None if MAY_BE_UNVERIFIED.contains(&param) => None,
            None => uber_default(param),
        }
    }

    /// A float argument; None where it is unverified or a map.
    pub fn float(&self, param: &str) -> Option<f32> {
        match self.arg(param)? {
            Arg::Float(f) => Some(f),
            _ => None,
        }
    }

    pub fn color(&self, param: &str) -> Option<[f32; 3]> {
        match self.arg(param)? {
            Arg::Color(c) => Some(c),
            _ => None,
        }
    }

    /// An enum argument (`intensity_power`).
    pub fn mode(&self, param: &str) -> Option<&'static str> {
        match self.arg(param)? {
            Arg::Enum(e) => Some(e),
            _ => None,
        }
    }

    pub fn bool(&self, param: &str) -> Option<bool> {
        match self.arg(param)? {
            Arg::Bool(b) => Some(b),
            _ => None,
        }
    }
}

/// Parameters [`uber`] leaves out in branches no fixture covers (see its
/// end): absent means unverified for these, not the MDL default.
const MAY_BE_UNVERIFIED: &[&str] = &[
    "dual_lobe_specular_roughness_1",
    "dual_lobe_specular_roughness_2",
    "backscattering_roughness",
    "metallic_flakes_roughness",
    "refraction_roughness",
    "top_coat_roughness",
    "metallic_weight",
    "metallic_enable",
    "metallic_custom_curve_normal",
    "metallic_directional_color_enable",
    "metallic_directional_normal_color",
    "emission_flux",
    "emission_mode",
];

/// Whether DAZ renders the material with Iray Uber: its channels (Base
/// Mixing among them), not PBRSkin's (its "… Enable" switches) nor a brick
/// graph's.
pub fn is_uber(mat: &Material) -> bool {
    mat.mdl.is_empty() && mat.channels.contains_key("Base Mixing") && !mat.channels.contains_key("Dual Lobe Specular Enable")
}

#[cfg(test)]
mod tests {
    //! Every rule against DAZ's own `.mi` export of the calibration scenes.
    use super::*;
    use crate::daz_material::{image_library, Material};
    use std::collections::HashMap;
    use std::path::Path;

    /// `node label/surface` → the scene's merged material, as the export
    /// reads it (definition + the scene's own channels).
    fn scene_materials(duf: &Path) -> HashMap<String, Material> {
        let doc: Value = serde_json::from_str(&crate::dson::decompress_scene(&std::fs::read(duf).unwrap()).unwrap()).unwrap();
        let images = image_library(&doc);
        let lib: HashMap<&str, &Value> = doc["material_library"].as_array().into_iter().flatten().filter_map(|m| Some((m["id"].as_str()?, m))).collect();
        let mut label = HashMap::new();
        for n in doc.pointer("/scene/nodes").and_then(Value::as_array).into_iter().flatten() {
            for g in n["geometries"].as_array().into_iter().flatten() {
                if let (Some(id), Some(l)) = (g["id"].as_str(), n["label"].as_str().or(n["id"].as_str())) {
                    label.insert(id.to_string(), l.to_string());
                }
            }
        }
        let mut out = HashMap::new();
        for m in doc.pointer("/scene/materials").and_then(Value::as_array).into_iter().flatten() {
            let url = m["url"].as_str().unwrap_or("");
            let mut mat = lib.get(url.trim_start_matches('#')).map_or_else(Material::default, |d| Material::from_dson(d, &images));
            mat.merge(m, &images);
            let geo = m["geometry"].as_str().unwrap_or("").rsplit('#').next().unwrap_or("");
            let node = label.get(geo).cloned().unwrap_or_else(|| geo.to_string());
            for g in m["groups"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                out.insert(format!("{node}/{g}"), mat.clone());
            }
        }
        out
    }

    /// A fixture value as an [`Arg`] of the kind `like` is.
    fn fixture_arg(v: &Value, like: &Arg) -> Option<Arg> {
        Some(match like {
            Arg::Bool(_) => Arg::Bool(v.as_bool()?),
            Arg::Int(_) => Arg::Int(v.as_i64()? as i32),
            Arg::Float(_) => Arg::Float(v.as_f64()? as f32),
            Arg::Enum(_) | Arg::Color(_) | Arg::Map(_) => {
                if let Some(s) = v.as_str() {
                    return Some(Arg::Enum(Box::leak(s.to_string().into_boxed_str())));
                }
                if let Some(a) = v.as_array() {
                    let c: Vec<f32> = a.iter().filter_map(Value::as_f64).map(|f| f as f32).collect();
                    return (c.len() == 3).then(|| Arg::Color([c[0], c[1], c[2]]));
                }
                if v.is_object() {
                    return Some(Arg::Map(texture_path(v).unwrap_or_default()));
                }
                return None;
            }
        })
    }

    /// The image a texture-call tree reads.
    fn texture_path(v: &Value) -> Option<String> {
        if let Some(p) = v.pointer("/texture/path").and_then(Value::as_str) {
            return Some(p.to_string());
        }
        v.as_object()?.values().find_map(texture_path)
    }

    fn close(a: &Arg, b: &Arg) -> bool {
        // The .mi prints 6 significant digits.
        let near = |x: f32, y: f32| (x - y).abs() <= 1e-5 + 1e-5 * x.abs().max(y.abs());
        match (a, b) {
            (Arg::Float(x), Arg::Float(y)) => near(*x, *y),
            (Arg::Color(x), Arg::Color(y)) => (0..3).all(|i| near(x[i], y[i])),
            (Arg::Map(x), Arg::Map(y)) => x.trim_start_matches('/') == y.trim_start_matches('/'),
            _ => a == b,
        }
    }

    #[test]
    fn uber_matches_daz_mi_export() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut checked = 0;
        let mut wrong = Vec::new();
        for f in std::fs::read_dir(root.join("calib/references/mi")).unwrap() {
            let f = f.unwrap().path();
            let scene = f.file_stem().unwrap().to_string_lossy().to_string();
            let fixture: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
            let mats = scene_materials(&root.join(format!("calib/scenes/{scene}.duf")));
            for (key, mi) in fixture["materials"].as_object().unwrap() {
                if !mi["mdl"].as_str().unwrap_or("").starts_with("daz_3d::irayubermaterial::iray_uber") {
                    continue;
                }
                let mat = mats.get(key).unwrap_or_else(|| panic!("{scene}: no material {key}"));
                checked += 1;
                if mi["mdl"] != uber_entry(mat) {
                    wrong.push(format!("MDL entry: {scene} {key}: DAZ {}, ours {}", mi["mdl"], uber_entry(mat)));
                }
                for (param, ours) in uber(mat) {
                    let theirs = match mi["args"].get(param) {
                        Some(v) => fixture_arg(v, &ours),
                        None => uber_default(param),
                    };
                    checked += 1;
                    if !theirs.as_ref().is_some_and(|t| close(t, &ours)) {
                        wrong.push(format!("{param}: {scene} {key}: DAZ {theirs:?}, ours {ours:?}"));
                    }
                }
            }
        }
        wrong.sort();
        assert!(wrong.is_empty(), "{} of {checked} arguments differ from DAZ's .mi:\n{}", wrong.len(), wrong.join("\n"));
    }

    /// `dzpm_diffuse_area_light` / `dzpm_spot_light` defaults (daz_3d/basic.mdl).
    fn light_default(param: &str) -> Arg {
        match param {
            "flux" => Arg::Float(1500.0),
            "temperature" => Arg::Float(6500.0),
            "scene_unit_size" => Arg::Float(0.01),
            "emission_color" => Arg::Color([1.0; 3]),
            "two_sided_area" => Arg::Bool(false),
            "spot_exponent" => Arg::Float(1.0),
            "spot_spread_angle" => Arg::Float(6.283),
            _ => panic!("no MDL default for {param}"),
        }
    }

    /// `node label` → the light's kind and channels (definition + node).
    fn scene_lights(duf: &Path) -> HashMap<String, (LightKind, HashMap<String, Value>)> {
        let doc: Value = serde_json::from_str(&crate::dson::decompress_scene(&std::fs::read(duf).unwrap()).unwrap()).unwrap();
        let lib: HashMap<&str, &Value> = doc["node_library"].as_array().into_iter().flatten().filter_map(|m| Some((m["id"].as_str()?, m))).collect();
        let channels = |n: &Value, out: &mut HashMap<String, Value>| {
            for e in n["extra"].as_array().into_iter().flatten() {
                for c in e["channels"].as_array().into_iter().flatten() {
                    if let (Some(id), Some(v)) = (c["channel"]["id"].as_str(), c["channel"].get("current_value").or(c["channel"].get("value"))) {
                        out.insert(id.to_string(), v.clone());
                    }
                }
            }
        };
        let mut out = HashMap::new();
        for n in doc.pointer("/scene/nodes").and_then(Value::as_array).into_iter().flatten() {
            let Some(def) = n["url"].as_str().and_then(|u| lib.get(u.rsplit('#').next().unwrap_or(""))) else { continue };
            if def["type"] != "light" {
                continue;
            }
            let kind = if def.get("spot").is_some() { LightKind::Spot } else if def.get("directional").is_some() { LightKind::Distant } else { LightKind::Point };
            let mut ch = HashMap::new();
            channels(def, &mut ch);
            channels(n, &mut ch);
            let label = n["label"].as_str().or(def["label"].as_str()).unwrap_or("").to_string();
            out.insert(label, (kind, ch));
        }
        out
    }

    #[test]
    fn lights_match_daz_mi_export() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let (mut checked, mut wrong) = (0, Vec::new());
        for f in std::fs::read_dir(root.join("calib/references/mi")).unwrap() {
            let f = f.unwrap().path();
            let scene = f.file_stem().unwrap().to_string_lossy().to_string();
            let fixture: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
            let lights = scene_lights(&root.join(format!("calib/scenes/{scene}.duf")));
            for (label, mi) in fixture["lights"].as_object().unwrap() {
                // The camera headlamp isn't a scene light (not verified yet).
                let Some((kind, ch)) = lights.get(label) else { continue };
                let ours = light(*kind, ch);
                checked += 1;
                if mi["material"]["mdl"] != ours.mdl {
                    wrong.push(format!("mdl: {scene} {label}: DAZ {}, ours {}", mi["material"]["mdl"], ours.mdl));
                }
                for (param, arg) in &ours.args {
                    let theirs = match mi["material"]["args"].get(*param) {
                        Some(v) => fixture_arg(v, arg),
                        None => Some(light_default(param)),
                    };
                    checked += 1;
                    if !theirs.as_ref().is_some_and(|t| close(t, arg)) {
                        wrong.push(format!("{param}: {scene} {label}: DAZ {theirs:?}, ours {arg:?}"));
                    }
                }
                let field = |k: &str| mi["fields"][k].as_array().map(|a| a.iter().filter_map(Value::as_f64).map(|v| v as f32).collect::<Vec<_>>());
                let theirs = if let Some(r) = field("rectangle") {
                    LightShape::Rectangle { width: r[0], height: r[4] }
                } else if let Some(d) = field("disc") {
                    LightShape::Disc { radius: d[3] }
                } else if let Some(s) = field("sphere") {
                    LightShape::Sphere { radius: s[0] }
                } else if mi["fields"].get("direction").is_some() {
                    LightShape::Distant
                } else {
                    LightShape::Point
                };
                checked += 1;
                if theirs != ours.shape {
                    wrong.push(format!("shape: {scene} {label}: DAZ {theirs:?}, ours {:?}", ours.shape));
                }
            }
        }
        wrong.sort();
        assert!(checked > 0 && wrong.is_empty(), "{} of {checked} light values differ from DAZ's .mi:\n{}", wrong.len(), wrong.join("\n"));
    }
}
