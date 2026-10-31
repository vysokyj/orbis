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
//! Parameters this module doesn't produce are absent from [`uber`]'s
//! result: not covered, not defaulted.

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
    /// A texture-driven argument.
    Map(Map),
}

/// A channel's map as DAZ hands it Iray: `base::file_texture` read as a
/// colour (`texture_return.tint`) or grey (`.mono`), or a bump
/// (`file_bump_texture`).
#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    /// Content path, as the channel names it.
    pub image: String,
    pub kind: MapKind,
    /// The channel's value as the map's factor (`color_scale`): linear for
    /// a colour channel, as is for a float one; a bump's strength.
    pub scale: [f32; 3],
    /// A glossiness map turned into roughness (`base::gloss_to_rough`).
    pub gloss_to_rough: bool,
    /// Then squared (`square_float`, Roughness / Glossiness Squared).
    pub squared: bool,
    /// UV tiles (u, v) and offsets: Horizontal / Vertical Tiles and
    /// Offset, the vertical offset negated.
    pub tiling: [f32; 4],
    /// The image's own gamma (`map_gamma`); None leaves it to Iray
    /// ("auto").
    pub gamma: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapKind {
    Colour,
    Grey,
    Bump,
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
    // (Not when a map drives it: Calib-DataGamma's metal maps at weight 1.)
    let full = |w: f32, id: &str| w >= 1.0 && ch.image(id).is_none();
    let diffuse_on = !full(metallic, "Metallic Weight") && !full(refraction, "Refraction Weight") && !full(backscattering, "Backscattering Weight");
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
    let rough_map = match mixing {
        1 => ch.map("Glossiness", MapKind::Grey, [ch.float("Glossiness", 1.0); 3]).map(|m| Map { gloss_to_rough: true, squared: ch.bool("Glossiness Squared", true), ..m }),
        _ => ch.map("Glossy Roughness", MapKind::Grey, [ch.float("Glossy Roughness", 0.0); 3]).map(|m| Map { squared: ch.bool("Roughness Squared", true), ..m }),
    };
    out.insert("glossy_roughness", rough_map.map_or(Arg::Float(rough), Arg::Map));
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
    out.insert("two_sided_area", Arg::Bool(emits && ch.bool("Two Sided Light", false)));
    // Luminance by its unit (Calib-Mi-Emit): a luminance per area of
    // `emission_scene_unit_size` (m) — cd/m², kcd/m² (× 1000), cd/ft²
    // (0.3048 m) and kcd/ft², which DAZ passes as cd per cm² (0.01 m, no
    // × 1000) —, or a power: lm, W × the luminous efficacy (lm/W).
    let luminance = ch.float("Luminance", 1500.0);
    let (flux, mode, unit) = match ch.int("Luminance Units", 0) {
        1 => (luminance * 1000.0, "intensity_radiant_exitance", 1.0),
        2 => (luminance, "intensity_radiant_exitance", 0.3048),
        3 => (luminance, "intensity_radiant_exitance", 0.01),
        4 => (luminance, "intensity_power", 1.0),
        5 => (luminance * ch.float("Luminous Efficacy", 15.0), "intensity_power", 1.0),
        _ => (luminance, "intensity_radiant_exitance", 1.0),
    };
    out.insert("emission_flux", Arg::Color([if emits { flux } else { 500.0 }; 3]));
    out.insert("emission_mode", Arg::Enum(if emits { mode } else { "intensity_radiant_exitance" }));
    out.insert("emission_scene_unit_size", Arg::Float(if emits { unit } else { 0.01 }));

    // Metal (metallicity / roughness only).
    out.insert("metallic_weight", if mixing == 0 { ch.float_or_map("Metallic Weight", 0.0) } else { Arg::Float(0.0) });
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
    out.insert("top_coat_weight", ch.float_or_map("Top Coat Weight", 0.0));
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

/// A colour channel as DAZ hands a light's colour to Iray: rounded to 8
/// bits, then the sRGB curve.
pub fn srgb_8bit_to_linear(c: f32) -> f32 {
    let c = (c.clamp(0.0, 1.0) * 255.0).round() / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

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

    fn map(&self, id: &str, kind: MapKind, scale: [f32; 3]) -> Option<Map> {
        let image = self.image(id)?.to_string();
        let gamma = self.0.channels.get(id).and_then(|c| c.gamma);
        let tiling = [self.float("Horizontal Tiles", 1.0), self.float("Vertical Tiles", 1.0), self.float("Horizontal Offset", 0.0), -self.float("Vertical Offset", 0.0)];
        Some(Map { image, kind, scale, gloss_to_rough: false, squared: false, tiling, gamma })
    }

    fn color_or_map(&self, id: &str, default: [f32; 3], scale: f32) -> Arg {
        let colour = self.color(id, default).map(|c| c * scale);
        match self.map(id, MapKind::Colour, colour) {
            Some(m) => Arg::Map(m),
            None => Arg::Color(colour),
        }
    }

    /// A float channel, or its map scaled by the value.
    fn float_or_map(&self, id: &str, default: f32) -> Arg {
        let v = self.float(id, default);
        match self.map(id, MapKind::Grey, [v; 3]) {
            Some(m) => Arg::Map(m),
            None => Arg::Float(v),
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
    // A light's colour, unlike a material's, goes through 8 bits and the
    // sRGB curve (Calib-L-Mi-Point-colour: 0.3 → 77 / 255 → 0.07421).
    let colour = channels.get("Color").and_then(Value::as_array).map_or([1.0; 3], |a| [0, 1, 2].map(|i| srgb_8bit_to_linear(a.get(i).and_then(Value::as_f64).unwrap_or(1.0) as f32)));
    args.insert("emission_color", Arg::Color(colour));
    let cm = |k: &str| num(k, 10.0);
    // A spot takes Light Geometry too (Calib-L-Mi-Spot-rect).
    let shape = match kind {
        LightKind::Distant => LightShape::Distant,
        LightKind::Spot | LightKind::Point => match num("Light Geometry", 0.0) as i32 {
            1 => LightShape::Rectangle { width: cm("Width"), height: cm("Height") },
            // Disc and sphere: Height as the diameter (Calib-L-Mi-*-30x60).
            2 => LightShape::Disc { radius: cm("Height") / 2.0 },
            3 => LightShape::Sphere { radius: cm("Height") / 2.0 },
            _ => LightShape::Point,
        },
    };
    let area = matches!(shape, LightShape::Rectangle { .. } | LightShape::Disc { .. } | LightShape::Sphere { .. });
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

    /// The argument, or its MDL default when DAZ doesn't pass it.
    fn arg(&self, param: &str) -> Option<Arg> {
        self.0.get(param).cloned().or_else(|| uber_default(param))
    }

    /// A float argument; None for a map.
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

/// Whether DAZ renders the material with Iray Uber: its channels (Base
/// Mixing among them), not PBRSkin's (its "… Enable" switches) nor a brick
/// graph's — even one on Uber's own bricks (HFGG's terrain, OOT's hair):
/// what DAZ makes of a graph isn't this conversion.
pub fn is_uber(mat: &Material) -> bool {
    !mat.brick && mat.mdl.is_empty() && mat.channels.contains_key("Base Mixing") && !mat.channels.contains_key("Dual Lobe Specular Enable")
}

/// Whether the material is DAZ's default shader (its channels at the top
/// level: `specular_strength`, `glossiness`, …).
pub fn is_daz_default(mat: &Material) -> bool {
    !mat.brick && !mat.channels.contains_key("Base Mixing") && mat.channels.contains_key("specular_strength") && mat.channels.contains_key("glossiness")
}

/// DAZ's default shader as the Iray Uber material DAZ renders it with
/// (Calib-Mi-DzDefault2): diffuse × diffuse_strength; a glossy layer at
/// the specular colour's strongest channel (linear) × specular_strength,
/// its hue as the glossy colour, roughness (1 − glossiness)^1.7; refraction
/// at refraction_strength in its own colour and IOR (Share Glossy Inputs
/// off then: `iray_uber`), closed when it refracts; transparency as Cutout Opacity. Ambient,
/// reflection and the lighting model don't reach Iray. None for other
/// shaders.
pub fn daz_default_as_uber(mat: &Material) -> Option<Material> {
    if !is_daz_default(mat) {
        return None;
    }
    let ch = Channels(mat);
    let mut out = mat.clone();
    let mut set = |id: &str, v: Value| {
        let image = out.channels.get(id).and_then(|c| c.image.clone());
        out.channels.insert(id.to_string(), crate::daz_material::Channel { value: v, image, gamma: None });
    };
    let spec = ch.color("specular", [0.6f32.powf(COLOUR_GAMMA); 3]);
    let peak = spec.into_iter().fold(0.0, f32::max);
    let strength = ch.float("specular_strength", 1.0);
    let (weight, hue) = if peak * strength > 0.0 { (peak * strength, spec.map(|c| c / peak)) } else { (0.0, spec) };
    // Back to DAZ's gamma-encoded colour, which `uber` linearises.
    let encode = |c: [f32; 3]| Value::from(c.map(|x| f64::from(x.max(0.0).powf(1.0 / COLOUR_GAMMA))).to_vec());
    let refraction = ch.float("refraction_strength", 0.0);
    set("Base Mixing", Value::from(0));
    set("Diffuse Strength", Value::from(f64::from(ch.float("diffuse_strength", 1.0))));
    set("Glossy Layered Weight", Value::from(f64::from(weight)));
    set("Glossy Color", encode(hue));
    set("Glossy Roughness", Value::from(f64::from((1.0 - ch.float("glossiness", 1.0)).clamp(0.0, 1.0).powf(DAZ_DEFAULT_ROUGHNESS_POWER))));
    set("Roughness Squared", Value::from(false));
    set("Refraction Weight", Value::from(f64::from(refraction)));
    set("Refraction Color", encode(ch.color("refraction", [1.0; 3])));
    set("Refraction Index", Value::from(f64::from(ch.float("ior", 1.5))));
    set("Share Glossy Inputs", Value::from(refraction <= 0.0));
    set("Thin Walled", Value::from(refraction <= 0.0));
    set("Cutout Opacity", Value::from(f64::from(ch.float("transparency", 1.0))));
    Some(out)
}

/// See [`daz_default_as_uber`]: Calib-Mi-DzDefault2's glossiness 0.3 / 0.5 /
/// 0.7 / 0.9 / 0.95 gave 0.5453 / 0.3078 / 0.1292 / 0.0200 / 0.0061.
pub const DAZ_DEFAULT_ROUGHNESS_POWER: f32 = 1.7;

/// What DAZ hands Iray for its Blended Dual Lobe Hair shader (a brick graph
/// on `dual_lobe_hair.mdl`, Calib-Mi-Hair), by the graph's argument path:
/// the Root colours only, linear (the Tip ones don't reach Iray);
/// roughnesses raw (the MDL squares them); the rest as is.
pub fn blended_hair(mat: &Material) -> BTreeMap<&'static str, Arg> {
    let ch = Channels(mat);
    let mut out = BTreeMap::new();
    out.insert("base.hair_color", Arg::Color(ch.color("Hair Root Color", [0.5; 3])));
    out.insert("base.base.surface.scattering.tint", Arg::Color(ch.color("Root Transmission Color", [0.5; 3])));
    out.insert("base.hightlight_color", Arg::Color(ch.color("Highlight Root Color", [1.0; 3])));
    for (param, channel, default) in [
        ("base.hair_weight", "Glossy Layer Weight", 0.5),
        ("base.base_roughness.roughness", "base_roughness", 0.2),
        ("base.highlight_roughness.roughness", "highlight_roughness", 0.1),
        ("base.highlight_weight", "Highlight Weight", 0.11),
        ("base.separation", "separation", 0.1),
        ("base.anisotropy", "Anisotropy", 1.0),
        ("base.anisotropy_rotation", "Anisotropy Rotations", 0.0),
        ("base.cutout_opacity", "Cutout Opacity", 1.0),
    ] {
        out.insert(param, Arg::Float(ch.float(channel, default)));
    }
    out
}

/// Whether the material is DAZ's Blended Dual Lobe Hair shader.
pub fn is_blended_hair(mat: &Material) -> bool {
    mat.channels.contains_key("Glossy Layer Weight") && mat.channels.contains_key("Hair Root Color") && mat.channels.contains_key("separation")
}

/// What DAZ hands Iray for its OmniHair Root-Tip Blend shader (a brick
/// graph on `OmniSurface::OmniHairBase`, Calib-Mi-Hair), by argument path:
/// root and tip blended along the strand (`color_1` root, `color_2` tip),
/// colours linear; Diffuse Strength as the base colour's weight, Highlight
/// Weight as the diffuse reflection's.
pub fn omni_hair(mat: &Material) -> BTreeMap<&'static str, Arg> {
    let ch = Channels(mat);
    let mut out = BTreeMap::new();
    out.insert("base_color.s.color_1", Arg::Color(ch.color("Hair Root Color", [1.0; 3])));
    out.insert("base_color.s.color_2", Arg::Color(ch.color("Hair Tip Color", [1.0; 3])));
    out.insert("diffuse_reflection_color.s.color_1", Arg::Color(ch.color("Highlight Root Color", [1.0; 3])));
    out.insert("diffuse_reflection_color.s.color_2", Arg::Color(ch.color("Tip Highlight Color", [1.0; 3])));
    out.insert("diffuse_reflection_weight.a", Arg::Color([ch.float("Highlight Weight", 0.0); 3]));
    for (param, channel, default) in [
        ("base_color_weight", "Diffuse Strength", 1.0),
        ("melanin_concentration", "Melanin", 0.5),
        ("melanin_redness", "Melanin Redness", 0.5),
        ("specular_reflection_roughness.s.color_1.value", "Roughness", 0.2),
        ("specular_reflection_roughness.s.color_2.value", "Roughness Tip", 0.2),
        ("specular_reflection_azimuthal_roughness.s.color_1.value", "Azimuthal Roughness", 0.2),
        ("specular_reflection_azimuthal_roughness.s.color_2.value", "Azimuthal Roughness Tip", 0.2),
        ("specular_reflection_shift", "Specular Reflection Shift", 3.0),
        ("geometry_opacity", "Cutout Opacity", 1.0),
    ] {
        out.insert(param, Arg::Float(ch.float(channel, default)));
    }
    out
}

/// Whether the material is DAZ's OmniHair Root-Tip Blend shader.
pub fn is_omni_hair(mat: &Material) -> bool {
    mat.channels.contains_key("Melanin") && mat.channels.contains_key("Specular Reflection Shift")
}

/// What DAZ hands Iray for a PBRSkin material (`pbr_skin.mdl`): the
/// arguments of `pbr_material`, `pbr_add_clearcoat`,
/// `pbr_add_metallicflakes` and `pbr_add_volume`, as `function.param`. Raw
/// values where Uber converts: roughness unsquared (the MDL squares it),
/// distances in cm; colours linear. The "… Enable" switches reach the MDL
/// as expressions (DAZ's graph), not as values: they aren't here.
pub fn pbrskin(mat: &Material) -> BTreeMap<&'static str, Arg> {
    let ch = Channels(mat);
    let on = |k: &str| ch.bool(k, false);
    let mut out = BTreeMap::new();
    out.insert("pbr_material.basecolor", ch.color_or_map("diffuse", [0.5; 3], 1.0));
    out.insert("pbr_material.metallic", Arg::Float(ch.float("Metallic Weight", 0.0)));
    out.insert("pbr_material.diffuse_roughness", Arg::Float(ch.float("Diffuse Roughness", 0.0)));
    out.insert("pbr_material.dual_lobe_specular_weight", Arg::Float(ch.float("Dual Lobe Specular Weight", 1.0)));
    out.insert("pbr_material.specularlevel", Arg::Float(REFLECTIVITY_SCALE * ch.float("Dual Lobe Specular Reflectivity", 0.5)));
    out.insert("pbr_material.roughness", Arg::Float(ch.float("Specular Lobe 1 Roughness", 0.6)));
    // Lobe 2's roughness as a multiple of lobe 1's.
    out.insert("pbr_material.roughness2", Arg::Float(ch.float("Specular Lobe 2 Roughness Mult", 0.4)));
    out.insert("pbr_material.roughness2_weight", Arg::Float(ch.float("Dual Lobe Specular Ratio", 0.15)));
    // The diffuse's share: 1 − Translucency Weight (`translucency_weight_inverted`).
    out.insert("pbr_material.translucency_weight", Arg::Float(1.0 - ch.float("Translucency Weight", 0.8)));
    out.insert("pbr_material.translucency_color", ch.color_or_map("Translucency Color", [1.0; 3], 1.0));
    out.insert("pbr_material.makeup_weight", Arg::Float(ch.float("Makeup Weight", 1.0)));
    out.insert("pbr_material.detail_weight", Arg::Float(ch.float("Detail Weight", 1.0)));
    out.insert("pbr_material.occlusion_weight", Arg::Float(ch.float("Ambient Occlusion Weight", 1.0)));
    out.insert("pbr_material.specular_occlusion_facing", Arg::Float(ch.float("Specular Occusion Facing", 0.6)));
    out.insert("pbr_material.specular_occlusion_grazing", Arg::Float(ch.float("Specular Occlusion Grazing", 0.1)));
    out.insert("pbr_add_clearcoat.top_coat_weight", Arg::Float(ch.float("Top Coat Weight", 0.5)));
    out.insert("pbr_add_clearcoat.top_coat_roughness", Arg::Float(ch.float("Top Coat Roughness", 0.05)));
    out.insert("pbr_add_clearcoat.top_coat_specular_level", Arg::Float(REFLECTIVITY_SCALE * ch.float("Top Coat Reflectivity", 0.5)));
    out.insert("pbr_add_clearcoat.top_coat_color", ch.color_or_map("Top Coat Color", [1.0; 3], 1.0));
    out.insert("pbr_add_metallicflakes.metallic_flakes_enable", Arg::Bool(on("Metallic Flakes Enable")));
    out.insert("pbr_add_metallicflakes.metallic_flakes_weight", Arg::Float(ch.float("Metallic Flakes Weight", 1.0)));
    out.insert("pbr_add_metallicflakes.metallic_flakes_roughness", Arg::Float(ch.float("Metallic Flakes Roughness", 0.0)));
    out.insert("pbr_add_metallicflakes.metallic_flakes_density", Arg::Float(ch.float("Metallic Flakes Density", 0.2)));
    out.insert("pbr_add_metallicflakes.metallic_flakes_color", ch.color_or_map("Metallic Flakes Color", [1.0; 3], 1.0));
    out.insert("pbr_add_volume.sub_surface_enable", Arg::Bool(on("Sub Surface Enable")));
    out.insert("pbr_add_volume.sub_surface_scattering_color", Arg::Color(ch.color("SSS Color", [0.5; 3])));
    out.insert("pbr_add_volume.scattering_distance_of_measurement", Arg::Float(ch.float("Scattering Measurement Distance", 0.015)));
    out.insert("pbr_add_volume.transmitted_color", Arg::Color(ch.color("Transmitted Color", [0.0; 3])));
    out
}

/// Whether DAZ renders the material with PBRSkin (its "… Enable"
/// switches).
pub fn is_pbrskin(mat: &Material) -> bool {
    mat.channels.contains_key("Dual Lobe Specular Enable") && mat.channels.contains_key("Specular Lobe 2 Roughness Mult")
}

/// The camera headlamp DAZ adds when it lights (`usd::lights::headlamp`
/// decides when): a 7 × 3.5 cm rect, colour 0.8, no temperature, its flux
/// 10 × the camera's Depth of Field (cm)² × Headlamp Intensity
/// (Calib-Headlamp-*, Calib-Mi-Headlamp-*), at the camera moved by the
/// offsets in cm along its axes (`usd::lights::headlamp`, tested there).
pub fn headlamp(camera: &std::collections::HashMap<String, Value>) -> Light {
    let num = |k: &str, d: f32| camera.get(k).and_then(Value::as_f64).map_or(d, |v| v as f32);
    let focus = num("Depth of Field", 300.0);
    let mut args = BTreeMap::new();
    args.insert("flux", Arg::Float(HEADLAMP_FLUX_PER_CM2 * focus * focus * num("Headlamp Intensity", 1.0)));
    args.insert("temperature", Arg::Float(0.0));
    args.insert("scene_unit_size", Arg::Float(1.0));
    args.insert("emission_color", Arg::Color([srgb_8bit_to_linear(0.8); 3]));
    args.insert("two_sided_area", Arg::Bool(false));
    Light { mdl: "daz_3d::basic::dzpm_diffuse_area_light", args, shape: LightShape::Rectangle { width: 7.0, height: 3.5 } }
}

/// The headlamp's flux per cm² of the focus distance, squared.
pub const HEADLAMP_FLUX_PER_CM2: f32 = 10.0;

type NodeChannels = std::collections::HashMap<String, Value>;

fn ch_num(ch: &NodeChannels, k: &str, d: f64) -> f64 {
    ch.get(k).and_then(Value::as_f64).unwrap_or(d)
}

fn ch_bool(ch: &NodeChannels, k: &str, d: bool) -> bool {
    ch.get(k).and_then(Value::as_bool).unwrap_or(d)
}

/// A DAZ colour channel, linear.
fn ch_colour(ch: &NodeChannels, k: &str, d: [f64; 3]) -> [f64; 3] {
    ch.get(k).and_then(Value::as_array).map_or(d, |a| [0, 1, 2].map(|i| a.get(i).and_then(Value::as_f64).unwrap_or(0.0).max(0.0).powf(COLOUR_GAMMA as f64)))
}

/// What DAZ hands Iray's camera, in cm.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    /// Focal length.
    pub focal: f64,
    /// The film's width: DAZ fits Frame Width (mm) to the image's
    /// *height*, so the width is it × the aspect.
    pub aperture: f64,
    pub clip: [f64; 2],
    /// Lens radius: focal length (mm) / (2 · f-stop) — in cm, 10× a real
    /// lens's; 0 without DOF.
    pub lens_radius: f64,
    /// Focus distance; 0 without DOF.
    pub lens_focus: f64,
}

/// The camera from its channels (definition overlaid by the node's) and
/// the render size.
pub fn camera(ch: &NodeChannels, (w, h): (u32, u32)) -> Camera {
    let focal_mm = ch_num(ch, "Focal Length", 65.0);
    let frame_mm = ch.get("Frame Width").and_then(Value::as_f64).filter(|f| *f > 0.0).unwrap_or(36.0);
    let dof = ch_bool(ch, "DOF", false);
    Camera {
        focal: focal_mm / 10.0,
        aperture: frame_mm / 10.0 * w as f64 / h.max(1) as f64,
        clip: [ch_num(ch, "Near Clip", 5.0), ch_num(ch, "Far Clip", 4000.0)],
        // DAZ spells the f-stop channel "Aperature".
        lens_radius: if dof { focal_mm / (2.0 * ch_num(ch, "Aperature", 22.0).max(0.1)) } else { 0.0 },
        lens_focus: if dof { ch_num(ch, "Depth of Field", 300.0) } else { 0.0 },
    }
}

/// Iray's photographic tone mapping as DAZ passes it (`mip_*` on the
/// camera), from the Tonemapper Options channels: all as they are — the
/// exposure is Shutter Speed, Aperture (f-number) and Film ISO, not the
/// Exposure Value DAZ shows; the white point × its scale.
pub fn tone(ch: &NodeChannels) -> BTreeMap<&'static str, f64> {
    let mut out = BTreeMap::new();
    for (param, channel, default) in [
        ("mip_film_iso", "Film ISO", 100.0),
        ("mip_camera_shutter", "Shutter Speed", 128.0),
        ("mip_f_number", "Aperture", 8.0),
        ("mip_cm2_factor", "cm2 Factor", 1.0),
        ("mip_burn_highlights", "Burn Highlights", 1.0),
        ("mip_crush_blacks", "Crush Blacks", 0.0),
        ("mip_saturation", "Saturation", 1.0),
        ("mip_gamma", "Gamma", 2.2),
        ("mip_vignetting", "Vignetting", 0.0),
    ] {
        out.insert(param, ch_num(ch, channel, default));
    }
    out
}

/// The white point Iray tone-maps to (`mip_whitepoint`): White Point ×
/// White Point Scale, as entered (not linearised).
pub fn white_point(ch: &NodeChannels) -> [f64; 3] {
    let scale = ch_num(ch, "White Point Scale", 1.0);
    let wp = ch.get("White Point").and_then(Value::as_array);
    [0, 1, 2].map(|i| wp.and_then(|a| a.get(i)).and_then(Value::as_f64).unwrap_or(1.0) * scale)
}

/// Iray's environment as DAZ sets it up, from the Environment Options
/// channels.
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    /// Environment Mode 3 (Scene Only): none.
    None,
    /// An HDRI dome (modes 0 / 1): `environment_function_intensity` =
    /// Environment Intensity × the map channel's value × 10 000.
    Dome { intensity: f64 },
    /// `base::sun_and_sky` (mode 2), its arguments. The sun's direction
    /// isn't here: DAZ passes the position of the node `SS Sun Direction`
    /// names (unnormalised, Calib-L-Mode-SunSky-dir), else the sun's place
    /// for SS Latitude / Longitude / Day / Time — `usd::sun_direction`
    /// (tested against it).
    SunSky { args: BTreeMap<&'static str, f64>, ground_color: [f64; 3] },
}

/// Iray's dome brightness per unit of DAZ's Environment Intensity × map value.
pub const DOME_INTENSITY_SCALE: f64 = 10_000.0;

pub fn environment(ch: &NodeChannels) -> Environment {
    match ch_num(ch, "Environment Mode", 0.0) as i32 {
        3 => Environment::None,
        2 => {
            let mut args = BTreeMap::new();
            for (param, channel, default) in [
                ("multiplier", "SS Multiplier", 0.10132),
                ("haze", "SS Haze", 0.0),
                ("sun_disk_intensity", "SS Sun Disk Intensity", 1.0),
                ("sun_disk_scale", "SS Sun Disk Scale", 4.0),
                ("saturation", "SS Saturation", 1.0),
                ("horizon_height", "SS Horizon Height", 0.0),
            ] {
                args.insert(param, ch_num(ch, channel, default));
            }
            Environment::SunSky { args, ground_color: ch_colour(ch, "SS Ground Color", [0.4f64.powf(2.2); 3]) }
        }
        _ => Environment::Dome { intensity: ch_num(ch, "Environment Intensity", 1.0) * ch_num(ch, "Environment Map", 1.0) * DOME_INTENSITY_SCALE },
    }
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
        let gammas = crate::daz_material::image_gammas(&doc);
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
            for c in mat.channels.values_mut() {
                c.gamma = c.image.as_ref().and_then(|i| gammas.get(i)).copied();
            }
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
                    return fixture_map(v).map(Arg::Map);
                }
                return None;
            }
        })
    }

    /// A texture-call tree from the .mi as a [`Map`].
    fn fixture_map(v: &Value) -> Option<Map> {
        let mdl = v["mdl"].as_str()?;
        let a = &v["args"];
        let f32s = |x: &Value| x.as_array().map(|a| a.iter().filter_map(Value::as_f64).map(|f| f as f32).collect::<Vec<_>>());
        let file = |f: &Value, kind: MapKind, scale: [f32; 3]| -> Option<Map> {
            let t = &f["args"]["texture"]["texture"];
            let m = f32s(&f["args"]["uvw"]["args"]["transform"]).unwrap_or_else(|| vec![1.0, 0., 0., 0., 0., 1.0, 0., 0., 0., 0., 1.0, 0., 0., 0., 0., 1.0]);
            Some(Map {
                image: t["path"].as_str()?.to_string(),
                kind,
                scale,
                gloss_to_rough: false,
                squared: false,
                tiling: [m[0], m[5], m[12], m[13]],
                gamma: t["gamma"].as_f64().map(|g| g as f32),
            })
        };
        let scale = |f: &Value| f32s(&f["args"]["color_scale"]).map_or([1.0; 3], |c| [c[0], c[1], c[2]]);
        if mdl.starts_with("daz_3d::irayubermaterial::square_float") {
            return fixture_map(&a["val"]).map(|m| Map { squared: true, ..m });
        }
        if mdl.starts_with("base::gloss_to_rough") {
            return fixture_map(&a["glossiness"]).map(|m| Map { gloss_to_rough: true, ..m });
        }
        if mdl.starts_with("base::texture_return.tint") {
            return file(&a["s"], MapKind::Colour, scale(&a["s"]));
        }
        if mdl.starts_with("base::texture_return.mono") {
            return file(&a["s"], MapKind::Grey, scale(&a["s"]));
        }
        if mdl.starts_with("base::file_bump_texture") {
            return file(v, MapKind::Bump, [a["factor"].as_f64().unwrap_or(1.0) as f32; 3]);
        }
        None
    }

    fn close(a: &Arg, b: &Arg) -> bool {
        // The .mi prints 6 significant digits.
        let near = |x: f32, y: f32| (x - y).abs() <= 1e-5 + 1e-5 * x.abs().max(y.abs());
        match (a, b) {
            (Arg::Float(x), Arg::Float(y)) => near(*x, *y),
            (Arg::Color(x), Arg::Color(y)) => (0..3).all(|i| near(x[i], y[i])),
            (Arg::Map(x), Arg::Map(y)) => {
                x.image.trim_start_matches('/') == y.image.trim_start_matches('/')
                    && x.kind == y.kind
                    && (0..3).all(|i| near(x.scale[i], y.scale[i]))
                    && (0..4).all(|i| near(x.tiling[i], y.tiling[i]))
                    && (x.gloss_to_rough, x.squared, x.gamma) == (y.gloss_to_rough, y.squared, y.gamma)
            }
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
                let converted = daz_default_as_uber(mat);
                let mat = converted.as_ref().unwrap_or(mat);
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
            let duf = root.join(format!("calib/scenes/{scene}.duf"));
            let lights = scene_lights(&duf);
            let cameras: HashMap<String, HashMap<String, Value>> = scene_nodes(&duf)
                .1
                .into_iter()
                .filter(|(d, _, _)| d["type"] == "camera")
                .map(|(d, n, ch)| (n["label"].as_str().or(d["label"].as_str()).unwrap_or("").to_string(), ch))
                .collect();
            for (label, mi) in fixture["lights"].as_object().unwrap() {
                // The camera headlamp: from the camera's channels.
                let ours = match lights.get(label) {
                    Some((kind, ch)) => light(*kind, ch),
                    None => match cameras.get(label) {
                        Some(ch) => headlamp(ch),
                        None => continue,
                    },
                };
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

    /// A scene's nodes as (definition, node, channels: definition overlaid
    /// by the node's).
    fn scene_nodes(duf: &Path) -> (Value, Vec<(Value, Value, HashMap<String, Value>)>) {
        let doc: Value = serde_json::from_str(&crate::dson::decompress_scene(&std::fs::read(duf).unwrap()).unwrap()).unwrap();
        let lib: HashMap<String, Value> = doc["node_library"].as_array().into_iter().flatten().filter_map(|m| Some((m["id"].as_str()?.to_string(), m.clone()))).collect();
        let channels = |n: &Value, out: &mut HashMap<String, Value>| {
            for e in n["extra"].as_array().into_iter().flatten() {
                for c in e["channels"].as_array().into_iter().flatten() {
                    if let (Some(id), Some(v)) = (c["channel"]["id"].as_str(), c["channel"].get("current_value").or(c["channel"].get("value"))) {
                        out.insert(id.to_string(), v.clone());
                    }
                }
            }
        };
        let mut out = Vec::new();
        for n in doc.pointer("/scene/nodes").and_then(Value::as_array).into_iter().flatten() {
            let def = n["url"].as_str().and_then(|u| lib.get(&crate::dson::percent_decode(u.rsplit('#').next().unwrap_or("")))).cloned().unwrap_or(Value::Null);
            let mut ch = HashMap::new();
            channels(&def, &mut ch);
            channels(n, &mut ch);
            out.push((def, n.clone(), ch));
        }
        (doc, out)
    }

    fn by_id<'a>(nodes: &'a [(Value, Value, HashMap<String, Value>)], prefix: &str) -> Option<&'a HashMap<String, Value>> {
        nodes.iter().find(|(d, n, _)| [d, n].iter().any(|v| v["id"].as_str().is_some_and(|i| i.starts_with(prefix)))).map(|(_, _, c)| c)
    }

    #[test]
    fn camera_tone_and_environment_match_daz_mi_export() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let near = |a: f64, b: f64| (a - b).abs() <= 1e-4 * (1.0 + a.abs().max(b.abs()));
        let num = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
        let (mut checked, mut wrong) = (0, Vec::new());
        let mut check = |what: String, daz: f64, ours: f64| {
            checked += 1;
            if !near(daz, ours) {
                wrong.push(format!("{what}: DAZ {daz}, ours {ours}"));
            }
        };
        for f in std::fs::read_dir(root.join("calib/references/mi")).unwrap() {
            let f = f.unwrap().path();
            let scene = f.file_stem().unwrap().to_string_lossy().to_string();
            let mi: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
            let (doc, nodes) = scene_nodes(&root.join(format!("calib/scenes/{scene}.duf")));
            let size = crate::usd::render_size(&doc);

            // The one camera DAZ exports (every calibration scene has one).
            let cams: Vec<_> = nodes.iter().filter(|(d, _, _)| d["type"] == "camera").collect();
            if let [(_, _, ch)] = cams.as_slice() {
                let c = camera(ch, size);
                let m = &mi["camera"];
                check(format!("{scene} focal"), num(&m["focal"]), c.focal);
                check(format!("{scene} aperture"), num(&m["aperture"]), c.aperture);
                check(format!("{scene} clip near"), num(&m["clip"][0]), c.clip[0]);
                check(format!("{scene} clip far"), num(&m["clip"][1]), c.clip[1]);
                check(format!("{scene} lens radius"), num(&m["mip_lens_radius"]), c.lens_radius);
                check(format!("{scene} lens focus"), num(&m["mip_lens_focus"]), c.lens_focus);
                check(format!("{scene} resolution"), num(&m["resolution"][0]), size.0 as f64);
            }
            if let Some(ch) = by_id(&nodes, "Tonemapper Options") {
                for (param, v) in tone(ch) {
                    check(format!("{scene} {param}"), num(&mi["camera"][param]), v);
                }
                let wp = white_point(ch);
                for i in 0..3 {
                    check(format!("{scene} whitepoint {i}"), num(&mi["camera"]["mip_whitepoint"][i]), wp[i]);
                }
            }
            if let Some(ch) = by_id(&nodes, "Environment Options") {
                let env = &mi["environment"];
                match environment(ch) {
                    Environment::None => check(format!("{scene} no environment"), 1.0, f64::from(u8::from(env.is_null()))),
                    Environment::Dome { intensity } => {
                        check(format!("{scene} dome"), 1.0, f64::from(u8::from(env["mdl"].as_str().is_some_and(|m| m.starts_with("base::environment_spherical")))));
                        check(format!("{scene} dome intensity"), num(&mi["options"]["environment_function_intensity"]), intensity);
                    }
                    Environment::SunSky { args, ground_color } => {
                        for (param, v) in args {
                            check(format!("{scene} sun-sky {param}"), env["args"].get(param).map_or(f64::NAN, num), v);
                        }
                        for i in 0..3 {
                            check(format!("{scene} sun-sky ground {i}"), num(&env["args"]["ground_color"][i]), ground_color[i]);
                        }
                        // The export's own solar position; a steering node's
                        // position comes through unnormalised.
                        let theirs: Vec<f64> = (0..3).map(|i| num(&env["args"]["sun_direction"][i])).collect();
                        if (theirs.iter().map(|x| x * x).sum::<f64>().sqrt() - 1.0).abs() < 1e-3 {
                            let n = |k: &str, d: f64| ch_num(ch, k, d);
                            let ours = crate::usd::sun_direction(n("SS Latitude", 0.0), n("SS Longitude", 0.0), n("SS Day", 0.0), n("SS Time", 0.0), n("SS UTC Offset", 0.0));
                            let len = theirs.iter().map(|x| x * x).sum::<f64>().sqrt();
                            let angle = ((0..3).map(|i| ours[i] * theirs[i]).sum::<f64>() / len).clamp(-1.0, 1.0).acos().to_degrees();
                            // Within 0.1°.
                            check(format!("{scene} sun direction off by (°)"), 0.0, if angle < 0.1 { 0.0 } else { angle });
                        }
                        check(format!("{scene} sun-sky intensity"), num(&mi["options"]["environment_function_intensity"]), 1.0);
                    }
                }
            }
        }
        wrong.sort();
        assert!(checked > 0 && wrong.is_empty(), "{} of {checked} values differ from DAZ's .mi:\n{}", wrong.len(), wrong.join("\n"));
    }

    /// A PBRSkin call tree's arguments as `function.param`: `pass_*`
    /// wrappers unwrapped, nested layers flattened; expressions (enable
    /// switches DAZ computes in MDL) left out.
    fn flatten_pbr(v: &Value, out: &mut HashMap<String, Value>) {
        let Some(m) = v["mdl"].as_str() else { return };
        let func = m.split('(').next().unwrap_or(m).rsplit("::").next().unwrap_or(m).to_string();
        for (k, a) in v["args"].as_object().into_iter().flatten() {
            match a["mdl"].as_str() {
                Some(n) if (n.contains("pbr_") || n.contains("::add_")) && !n.contains("normal") => flatten_pbr(a, out),
                Some(n) if n.starts_with("daz_3d::basic::pass_") && !a["args"]["v"].is_object() => {
                    out.insert(format!("{func}.{k}"), a["args"]["v"].clone());
                }
                Some(_) => {}
                None => {
                    out.insert(format!("{func}.{k}"), a.clone());
                }
            }
        }
    }

    #[test]
    fn pbrskin_matches_daz_mi_export() {
        // PBRSkin's definition is DAZ's (`PBRSkin.dsf`): its channel
        // defaults only, extracted by scripts/dev/daz-defaults.py.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let defaults: serde_json::Map<String, Value> = serde_json::from_str(&std::fs::read_to_string(root.join("calib/references/daz-defaults/PBRSkin.json")).unwrap()).unwrap();
        let channels: Vec<Value> = defaults.into_iter().map(|(id, v)| serde_json::json!({"channel": {"id": id, "current_value": v}})).collect();
        let base = Material::from_dson(&serde_json::json!({"extra": [{"channels": channels}]}), &HashMap::new());
        let (mut checked, mut wrong) = (0, Vec::new());
        for f in std::fs::read_dir(root.join("calib/references/mi")).unwrap() {
            let f = f.unwrap().path();
            let scene = f.file_stem().unwrap().to_string_lossy().to_string();
            let fixture: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
            let doc: Value = serde_json::from_str(&crate::dson::decompress_scene(&std::fs::read(root.join(format!("calib/scenes/{scene}.duf"))).unwrap()).unwrap()).unwrap();
            let mut label = HashMap::new();
            for n in doc.pointer("/scene/nodes").and_then(Value::as_array).into_iter().flatten() {
                for g in n["geometries"].as_array().into_iter().flatten() {
                    label.insert(g["id"].as_str().unwrap_or("").to_string(), n["label"].as_str().unwrap_or("").to_string());
                }
            }
            for m in doc.pointer("/scene/materials").and_then(Value::as_array).into_iter().flatten() {
                if !m["url"].as_str().is_some_and(|u| u.contains("PBRSkin.dsf")) {
                    continue;
                }
                let mut mat = base.clone();
                mat.merge(m, &HashMap::new());
                let node = &label[m["geometry"].as_str().unwrap().rsplit('#').next().unwrap()];
                for g in m["groups"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    let key = format!("{node}/{g}");
                    let mi = &fixture["materials"][&key];
                    let mut theirs = HashMap::new();
                    flatten_pbr(&mi["args"]["v"], &mut theirs);
                    for (param, ours) in pbrskin(&mat) {
                        checked += 1;
                        let t = theirs.get(param).and_then(|v| fixture_arg(v, &ours));
                        if !t.as_ref().is_some_and(|t| close(t, &ours)) {
                            wrong.push(format!("{param}: {scene} {key}: DAZ {t:?}, ours {ours:?}"));
                        }
                    }
                }
            }
        }
        wrong.sort();
        assert!(checked > 0 && wrong.is_empty(), "{} of {checked} PBRSkin arguments differ from DAZ's .mi:\n{}", wrong.len(), wrong.join("\n"));
    }

    /// A brick graph's leaf values by argument path (`pass_*` unwrapped).
    fn graph_leaves(v: &Value, path: &str, out: &mut HashMap<String, Value>) {
        let join = |k: &str| if path.is_empty() { k.to_string() } else { format!("{path}.{k}") };
        match v["mdl"].as_str() {
            Some(m) if m.starts_with("daz_3d::basic::pass_") && !v["args"]["v"].is_object() => {
                out.insert(path.to_string(), v["args"]["v"].clone());
            }
            Some(_) => {
                for (k, a) in v["args"].as_object().into_iter().flatten() {
                    graph_leaves(a, &join(k), out);
                }
            }
            None if !v.is_object() => {
                out.insert(path.to_string(), v.clone());
            }
            None => {}
        }
    }

    #[test]
    fn hair_shaders_match_daz_mi_export() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let defaults = |name: &str| {
            let d: serde_json::Map<String, Value> = serde_json::from_str(&std::fs::read_to_string(root.join(format!("calib/references/daz-defaults/{name}.json"))).unwrap()).unwrap();
            let channels: Vec<Value> = d.into_iter().map(|(id, v)| serde_json::json!({"channel": {"id": id, "current_value": v}})).collect();
            Material::from_dson(&serde_json::json!({"extra": [{"channels": channels}]}), &HashMap::new())
        };
        let (blended, omni) = (defaults("BlendedDualLobeHair"), defaults("OmniHairRootTipBlend"));
        let (mut checked, mut wrong) = (0, Vec::new());
        let scene = "Calib-Mi-Hair";
        let fixture: Value = serde_json::from_str(&std::fs::read_to_string(root.join(format!("calib/references/mi/{scene}.json"))).unwrap()).unwrap();
        let doc: Value = serde_json::from_str(&crate::dson::decompress_scene(&std::fs::read(root.join(format!("calib/scenes/{scene}.duf"))).unwrap()).unwrap()).unwrap();
        let mut label = HashMap::new();
        for n in doc.pointer("/scene/nodes").and_then(Value::as_array).into_iter().flatten() {
            for g in n["geometries"].as_array().into_iter().flatten() {
                label.insert(g["id"].as_str().unwrap_or("").to_string(), n["label"].as_str().unwrap_or("").to_string());
            }
        }
        for m in doc.pointer("/scene/materials").and_then(Value::as_array).into_iter().flatten() {
            let url = m["url"].as_str().unwrap_or("");
            let mut mat = if url.contains("Blended%20Dual%20Lobe") { blended.clone() } else if url.contains("OmniHair") { omni.clone() } else { continue };
            mat.merge(m, &HashMap::new());
            let ours = if is_blended_hair(&mat) { blended_hair(&mat) } else if is_omni_hair(&mat) { omni_hair(&mat) } else { panic!("{url}: not recognised") };
            let node = &label[m["geometry"].as_str().unwrap().rsplit('#').next().unwrap()];
            let key = format!("{node}/Default");
            let mut theirs = HashMap::new();
            graph_leaves(&fixture["materials"][&key]["args"]["v"], "", &mut theirs);
            // The Blended hair graph sits under `replace_scattering(base: …)`.
            for (param, arg) in ours {
                checked += 1;
                let t = theirs.get(param).or_else(|| theirs.get(&format!("base.{param}"))).and_then(|v| fixture_arg(v, &arg));
                if !t.as_ref().is_some_and(|t| close(t, &arg)) {
                    wrong.push(format!("{param}: {key}: DAZ {t:?}, ours {arg:?}"));
                }
            }
        }
        wrong.sort();
        assert!(checked > 0 && wrong.is_empty(), "{} of {checked} hair arguments differ from DAZ's .mi:\n{}", wrong.len(), wrong.join("\n"));
    }
}
