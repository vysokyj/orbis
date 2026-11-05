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

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use crate::daz_material::{Material, Mono};

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
    /// An IES light profile (`light_profile`): the file as the channel
    /// names it (content-relative or absolute).
    LightProfile(String),
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
    /// a colour channel, as is for a float one; a bump's strength. With
    /// the per-map scale and invert (`ImageMod`) folded in.
    pub scale: [f32; 3],
    /// Added to the texel × `scale` (`color_offset`): the channel's value ×
    /// the per-map offset (and scale, when inverted).
    pub offset: [f32; 3],
    /// Which texel channel a grey map reads (`mono_source`).
    pub mono: Mono,
    /// The channel's value as the factor, before the per-map settings
    /// (the Octane side applies those itself: `Material::image_mod`).
    pub value: [f32; 3],
    /// A glossiness map turned into roughness (`base::gloss_to_rough`).
    pub gloss_to_rough: bool,
    /// Then squared (`square_float`, Roughness / Glossiness Squared).
    pub squared: bool,
    /// The uvw transform's scale (u, v) and translation (u, v): the
    /// material's Horizontal / Vertical Tiles and Offset (the vertical
    /// offset negated), then the map's own tiles and tiling offsets
    /// (`ImageMod`).
    pub tiling: [f32; 4],
    /// The image's own gamma (`map_gamma`); None leaves it to Iray
    /// ("auto").
    pub gamma: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapKind {
    Colour,
    Grey,
    /// A height map (`base::file_bump_texture`), the scale its factor.
    Bump,
    /// A tangent-space normal map (`base::tangent_space_normal_texture`).
    Normal,
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

    // Base colour: linear (a pure 2.2 power) × Diffuse Strength; a
    // Diffuse Strength map attenuates the colour (`attenuate_color`,
    // Calib-Mi-AoA3).
    let strength = ch.float("Diffuse Strength", 1.0);
    let diffuse = match ch.map("Diffuse Strength", MapKind::Grey, ch.color("diffuse", [0.5; 3]).map(|c| c * strength)) {
        Some(m) if ch.image("diffuse").is_none() => Arg::Map(m),
        _ => ch.color_or_map("diffuse", [0.5; 3], strength),
    };
    out.insert("diffuse_color", diffuse);
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
    // A map drives the layered weight, scaled by the value (Calib-Mi-AoA3).
    out.insert("glossy_layered_weight", if mixing == 2 { Arg::Float(1.0) } else { ch.float_or_map("Glossy Layered Weight", 0.0) });
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
    // A weight map: `square_float` over the map × the value, so the
    // value is squared too (Calib-Overlay V3: 0.25 m²).
    let overlay_squared = ch.bool("Diffuse Overlay Weight Squared", false);
    let overlay_map = ch.map("Diffuse Overlay Weight", MapKind::Grey, [overlay; 3]).map(|m| Map { squared: overlay_squared, ..m });
    out.insert("diffuse_overlay_weight", overlay_map.map_or(Arg::Float(square_if(overlay, overlay_squared)), Arg::Map));
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
    // Emission Profile (an IES file): `measured_edf` instead of the diffuse
    // EDF, the luminance divided by the profile's power (Calib-L-IES-Emit).
    let profile = ch.string("Emission Profile").filter(|p| emits && !p.is_empty());
    out.insert("emission_profile_enable", Arg::Bool(profile.is_some()));
    if let Some(p) = profile {
        out.insert("emission_profile", Arg::LightProfile(p));
    }
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
    out.insert("top_coat_anisotropy_rotation", Arg::Float(if coat_aniso > 0.0 { ch.float("Top Coat Rotations", 0.0) } else { 0.0 }));
    out.insert("top_coat_color_transmit_enable", Arg::Int(if coat > 0.0 { ch.int("Top Coat Color Effect", 0) } else { 0 }));
    const MODES: [&str; 4] = ["tcw_principled_specular", "tcw_weighted", "tcw_fresnel", "tcw_custom_curve"];
    let mode = ch.int("Top Coat Layering Mode", 0).clamp(0, 3) as usize;
    out.insert("top_coat_layer_mode", Arg::Enum(if coat > 0.0 { MODES[mode] } else { MODES[0] }));
    out.insert("top_coat_curve_strength", Arg::Float(if coat > 0.0 && mode == 0 { REFLECTIVITY_SCALE * ch.float("Reflectivity", 0.5) } else { 0.04 }));

    // Bumps (#61): the base's height map (Bump Strength) or normal map, and
    // the Top Coat Bump — a height map, or a normal map in Top Coat Bump
    // Mode 1 — each at its channel's value as the factor (Calib-CoatBump,
    // Calib-Mi-Maps). The MDL gives the coat the two blended
    // (`calc_mix_layer_normal`); without its own, the base's.
    let base_bump = match (ch.image("Bump Strength").is_some(), ch.image("Normal Map").is_some()) {
        (true, false) => ch.map("Bump Strength", MapKind::Bump, [ch.float("Bump Strength", 1.0); 3]),
        (false, true) => ch.map("Normal Map", MapKind::Normal, [ch.float("Normal Map", 1.0); 3]),
        // Both (Calib-Mi-AoA3): the normal map is the base bump, the height
        // map the secondary one, which the MDL blends onto it
        // (`add_detail_normal`, `base::blend_normals` at weights 1).
        (true, true) => ch.map("Normal Map", MapKind::Normal, [ch.float("Normal Map", 1.0); 3]),
        _ => None,
    };
    out.insert("base_bump_enable", Arg::Bool(ch.image("Bump Strength").is_some() || ch.image("Normal Map").is_some()));
    if let Some(m) = base_bump {
        out.insert("base_bump", Arg::Map(m));
    }
    let both = ch.image("Bump Strength").is_some() && ch.image("Normal Map").is_some();
    out.insert("base_bump2_enable", Arg::Bool(both));
    if let Some(m) = ch.map("Bump Strength", MapKind::Bump, [ch.float("Bump Strength", 1.0); 3]).filter(|_| both) {
        out.insert("base_bump2", Arg::Map(m));
    }
    let coat_kind = if ch.int("Top Coat Bump Mode", 0) == 1 { MapKind::Normal } else { MapKind::Bump };
    match ch.map("Top Coat Bump", coat_kind, [ch.float("Top Coat Bump", 1.0); 3]).filter(|_| coat > 0.0) {
        Some(m) => {
            out.insert("top_coat_bump_enable", Arg::Bool(true));
            out.insert("top_coat_bump", Arg::Map(m));
        }
        None => {
            out.insert("top_coat_bump_enable", Arg::Bool(false));
        }
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

    /// A file channel's path (Emission Profile).
    fn string(&self, id: &str) -> Option<String> {
        self.value(id).and_then(Value::as_str).map(str::to_string)
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

    /// The channel's map, `scale` its value as the factor, with the
    /// channel's per-map settings (`ImageMod`, Calib-Mi-ImageMod, -ImageMod2):
    /// the map's tiles multiply the material's, its tiling offset adds to
    /// the material's offset × its tiles; scale / offset / invert become
    /// the colour scale and offset; a one-channel grey mode zeroes the
    /// other channels and reads their maximum. A bump or normal map takes
    /// the placement only.
    fn map(&self, id: &str, kind: MapKind, scale: [f32; 3]) -> Option<Map> {
        let c = self.0.channels.get(id)?;
        let image = c.image.clone()?;
        let im = c.image_mod.unwrap_or_default();
        let bump = matches!(kind, MapKind::Bump | MapKind::Normal);
        let im = if bump { im.placement() } else { im };
        let (mt, mo) = ([self.float("Horizontal Tiles", 1.0), self.float("Vertical Tiles", 1.0)], [self.float("Horizontal Offset", 0.0), self.float("Vertical Offset", 0.0)]);
        let (pt, po) = (im.tiles, im.tiling_offset);
        let tiling = [mt[0] * pt[0], mt[1] * pt[1], mo[0] * pt[0] + po[0], -(mo[1] * pt[1] + po[1])];
        let mask = match im.mono {
            Mono::Red => [1.0, 0.0, 0.0],
            Mono::Green => [0.0, 1.0, 0.0],
            Mono::Blue => [0.0, 0.0, 1.0],
            _ => [1.0; 3],
        };
        let (s, o) = if im.invert { (-im.scale, im.offset + im.scale) } else { (im.scale, im.offset) };
        let value = scale;
        let offset = [0, 1, 2].map(|i| scale[i] * o * mask[i]);
        let scale = [0, 1, 2].map(|i| scale[i] * s * mask[i]);
        Some(Map { image, kind, scale, offset, mono: im.mono, value, gloss_to_rough: false, squared: false, tiling, gamma: c.gamma })
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
///
/// Nothing else of a DAZ light reaches Iray: Cast Shadows, Shadow Type /
/// Intensity / Softness / Colour / Bias, Decay, Illumination 1 and 2
/// (diffuse / specular only) and Light Portal leave the .mi as it was
/// (Calib-L-Opt-*), and Iray renders them alike (#35).
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    pub mdl: &'static str,
    pub args: BTreeMap<&'static str, Arg>,
    pub shape: LightShape,
    /// The light's `visible` flag — seen by camera rays (Iray manual,
    /// ghost lights), still lighting and reflected: Render Emitter
    /// (Calib-L-Opt-EmitterOff, -Rect-Facing-EmitterOff, -Spot-Facing-EmitterOff); off for the headlamp.
    pub visible: bool,
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
    // A point light's Light Profile (an IES file; DAZ's spot and distant
    // lights have none) makes it an IES light, whatever its shape
    // (Calib-L-IES-*): the profile's shape at the light's flux.
    let profile = channels.get("Light Profile").and_then(Value::as_str).filter(|p| kind == LightKind::Point && !p.is_empty());
    let mdl = if kind == LightKind::Spot {
        args.insert("spot_exponent", Arg::Float(num("Beam Exponent", 1.0)));
        // The full Spread Angle, in radians.
        args.insert("spot_spread_angle", Arg::Float(num("Spread Angle", 60.0).to_radians()));
        "daz_3d::basic::dzpm_spot_light"
    } else if let Some(p) = profile {
        args.insert("profile", Arg::LightProfile(p.to_string()));
        "daz_3d::basic::dzpm_ies_light"
    } else {
        "daz_3d::basic::dzpm_diffuse_area_light"
    };
    Light { mdl, args, shape, visible: flag("Render Emitter", true) }
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
        "top_coat_thin_film_enable" | "top_coat_bump_enable" | "base_bump_enable" | "base_bump2_enable" | "top_coat_anisotropy_enable" | "glossy_anisotropy_enable" | "base_thin_film_enable" | "diffuse_overlay_enable" | "metallic_flakes_enable" => Arg::Bool(false),
        "top_coat_anisotropy" | "top_coat_anisotropy_rotation" | "glossy_anisotropy" | "glossy_anisotropy_rotation" | "diffuse_overlay_weight" | "diffuse_overlay_roughness" | "diffuse_roughness" | "metallic_flakes_weight" | "metallic_flakes_roughness" => Arg::Float(0.0),
        "top_coat_color_transmit_enable" | "glossy_color_transmit_enable" | "diffuse_overlay_color_transmit_enable" | "diffuse_color_transmit_enable" | "metallic_flakes_color_transmit_enable" => Arg::Int(0),
        "diffuse_overlay_color" => Arg::Color([0.5; 3]),
        "metallic_flakes_size" | "metallic_flakes_strength" | "metallic_flakes_density" => Arg::Float(1.0),
        "metallic_flakes_color" => Arg::Color([1.0; 3]),
        "refraction_color" | "backscattering_color" => Arg::Color([0.5; 3]),
        "refraction_roughness" => Arg::Float(0.0),
        "glossy_directional_color_enable" | "emission_enable" | "two_sided_area" | "emission_profile_enable" => Arg::Bool(false),
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

    /// A texture-driven argument; None for a value.
    pub fn map(&self, param: &str) -> Option<Map> {
        match self.arg(param)? {
            Arg::Map(m) => Some(m),
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
        let (image, image_mod) = out.channels.get(id).map_or((None, None), |c| (c.image.clone(), c.image_mod));
        out.channels.insert(id.to_string(), crate::daz_material::Channel { value: v, image, gamma: None, image_mod });
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

/// Any legacy shader DAZ converts to Iray Uber, as the Uber it renders:
/// DAZ Default ([`daz_default_as_uber`]), the scripted 3Delight shaders
/// ([`legacy_shader_as_uber`], here with their maps as DAZ hands them —
/// none), AoA Subsurface ([`aoa_subsurface_as_uber`]). None for others.
pub fn as_uber(mat: &Material) -> Option<Material> {
    daz_default_as_uber(mat).or_else(|| legacy_shader_as_uber(mat, false)).or_else(|| aoa_subsurface_as_uber(mat))
}

/// Whether the material is one of DAZ Studio's scripted 3Delight shaders
/// (UberSurface, HumanSurface, dzGlossy, dzPlastic, … —
/// `studio/material/daz_shader`).
pub fn is_legacy_shader(mat: &Material) -> bool {
    mat.shader.is_some() && !mat.brick
}

/// A scripted 3Delight shader as the Iray Uber DAZ Studio 6 renders it
/// (Calib-Mi-Legacy, -Legacy2; #64). DAZ Studio 6 has no 3Delight: it
/// can't load the shader's definition script, so the material keeps only
/// Diffuse Color and Opacity Strength, whatever the shader or the channels
/// an older save carries. Iray gets the diffuse colour (linear) and the
/// opacity as Cutout Opacity — **without their maps** — and Iray Uber's
/// MDL defaults otherwise: a glossy layer at weight 1, white, roughness 1.
/// `keep_maps` keeps the two maps (the export's choice: what the scene
/// means; DAZ's Iray drops them). None for other shaders.
pub fn legacy_shader_as_uber(mat: &Material, keep_maps: bool) -> Option<Material> {
    if !is_legacy_shader(mat) {
        return None;
    }
    let mut out = Material { channels: HashMap::new(), shader: None, url: None, ..mat.clone() };
    for (from, to) in [("diffuse", "diffuse"), ("transparency", "Cutout Opacity")] {
        if let Some(c) = mat.channels.get(from) {
            let image = if keep_maps { c.image.clone() } else { None };
            out.channels.insert(to.to_string(), crate::daz_material::Channel { image, ..c.clone() });
        }
    }
    for (id, v) in [
        ("Base Mixing", Value::from(0)),
        ("Glossy Layered Weight", Value::from(1.0)),
        ("Glossy Color", Value::from(vec![1.0, 1.0, 1.0])),
        ("Glossy Roughness", Value::from(1.0)),
        ("Roughness Squared", Value::from(false)),
        ("Thin Walled", Value::from(true)),
    ] {
        out.channels.insert(id.to_string(), crate::daz_material::Channel { value: v, image: None, gamma: None, image_mod: None });
    }
    Some(out)
}

/// Whether the material is Age of Armour's Subsurface shader (a 3Delight
/// brick graph in DAZ Studio's default resources), by its definition.
pub fn is_aoa_subsurface(mat: &Material) -> bool {
    mat.brick && mat.url.as_deref().is_some_and(|u| u.split('#').next().unwrap_or("").ends_with("/AoA_Subsurface/AoA_Subsurface.dsf"))
}

/// AoA Subsurface as the Iray Uber DAZ renders it (Calib-Mi-AoA, -AoA2,
/// -AoA3, -AoA4; #64). Its diffuse, Diffuse Strength, bump, normal map,
/// displacement, tiling and opacity carry Uber's own channel ids and pass as
/// they are (Normal Map Strength doesn't reach Iray). Specular is the glossy
/// layer as in DAZ Default — weight = the colour's strongest channel
/// (linear) × Specular Strength, its hue as the glossy colour — but with
/// roughness 1 − Glossiness. Specular 2 (`color2`, Strength `value222`,
/// Glossiness `strength22`) becomes the top coat by the same rule, roughness
/// min(1.5 (1 − g), 1) — or the glossy layer itself when Specular Strength
/// is 0. Maps: the Specular Strength map drives the glossy weight, a colour
/// map the glossy / top coat colour (its Color Effect 1); maps on the
/// glossinesses or the Specular 2 Strength don't reach Iray. Reflection,
/// velvet and ambient don't either; subsurface (only with SSS Material Type
/// Custom) isn't reproduced. None for other shaders.
pub fn aoa_subsurface_as_uber(mat: &Material) -> Option<Material> {
    if !is_aoa_subsurface(mat) {
        return None;
    }
    use crate::daz_material::Channel;
    let ch = Channels(mat);
    let mut out = Material { channels: HashMap::new(), brick: false, mdl: Vec::new(), url: None, ..mat.clone() };
    for id in ["diffuse", "Diffuse Strength", "Bump Strength", "Normal Map", "Displacement Strength", "Minimum Displacement", "Maximum Displacement", "Horizontal Tiles", "Vertical Tiles", "Horizontal Offset", "Vertical Offset"] {
        if let Some(c) = mat.channels.get(id) {
            out.channels.insert(id.to_string(), c.clone());
        }
    }
    if let Some(c) = mat.channels.get("transparency") {
        out.channels.insert("Cutout Opacity".into(), c.clone());
    }
    // Normal Map Strength (`alpha`) doesn't reach Iray: the factor stays 1.
    if let Some(c) = out.channels.get_mut("Normal Map") {
        c.value = Value::Null;
    }
    let encode = |c: [f32; 3]| Value::from(c.map(|x| f64::from(x.max(0.0).powf(1.0 / COLOUR_GAMMA))).to_vec());
    let image = |id: &str| mat.channels.get(id).and_then(|c| c.image.clone());
    let mut set = |id: &str, v: Value, image: Option<String>| {
        out.channels.insert(id.to_string(), Channel { value: v, image, gamma: None, image_mod: None });
    };
    // A lobe from a colour × strength: (weight, hue).
    let lobe = |colour: &str, strength: &str, default: f32| {
        let c = ch.color(colour, [1.0; 3]);
        let peak = c.into_iter().fold(0.0, f32::max);
        let w = peak * ch.float(strength, default);
        if w > 0.0 { (w, c.map(|x| x / peak)) } else { (0.0, c) }
    };
    let spec2_rough = (AOA_SPECULAR2_ROUGHNESS * (1.0 - ch.float("strength22", 0.85))).clamp(0.0, 1.0);
    let primary = ch.float("Specular Strength", 0.5) > 0.0;
    let (weight, hue, rough, weight_map, colour_id) = if primary {
        let (w, h) = lobe("Specular Color", "Specular Strength", 0.5);
        (w, h, (1.0 - ch.float("Glossiness", 0.65)).clamp(0.0, 1.0), image("Specular Strength"), "Specular Color")
    } else {
        let (w, h) = lobe("color2", "value222", 0.0);
        (w, h, spec2_rough, None, "color2")
    };
    set("Base Mixing", Value::from(0), None);
    set("Glossy Layered Weight", Value::from(f64::from(weight)), weight_map);
    set("Glossy Color", encode(hue), image(colour_id));
    set("Glossy Color Effect", Value::from(i32::from(image(colour_id).is_some())), None);
    set("Glossy Roughness", Value::from(f64::from(rough)), None);
    set("Roughness Squared", Value::from(false), None);
    // Subsurface (on, SSS Material Type Custom) closes the surface.
    let sss = ch.float("value23", 0.0) > 0.0 && ch.int("sssmaterial", 9) == 0;
    set("Thin Walled", Value::from(!sss), None);
    if primary {
        let (w, h) = lobe("color2", "value222", 0.0);
        if w > 0.0 {
            set("Top Coat Weight", Value::from(f64::from(w)), None);
            set("Top Coat Color", encode(h), image("color2"));
            set("Top Coat Color Effect", Value::from(i32::from(image("color2").is_some())), None);
            set("Top Coat Roughness", Value::from(f64::from(spec2_rough)), None);
            set("Top Coat Thin Film", Value::from(0.0), None);
        }
    }
    Some(out)
}

/// AoA Subsurface's Specular 2 roughness per unit of (1 − glossiness),
/// clamped to 1 (Calib-Mi-AoA*: Glossiness 0.95 / 0.9 / 0.85 / 0.8 / 0.5 /
/// 0.2 gave 0.075 / 0.15 / 0.225 / 0.3 / 0.75 / 1).
pub const AOA_SPECULAR2_ROUGHNESS: f32 = 1.5;

/// See [`daz_default_as_uber`]: Calib-Mi-DzDefault2's glossiness 0.3 / 0.5 /
/// 0.7 / 0.9 / 0.95 gave 0.5453 / 0.3078 / 0.1292 / 0.0200 / 0.0061.
pub const DAZ_DEFAULT_ROUGHNESS_POWER: f32 = 1.7;

/// What DAZ hands Iray for its Blended Dual Lobe Hair shader (a brick graph
/// on `dual_lobe_hair.mdl`, Calib-Mi-Hair / -Hair2), by the graph's argument
/// path: two `dual_lobe_hair` instances, the Root colours in `base` and the
/// Tip ones in the one `df::clamped_mix` weighs by
/// `get_line_root_to_tip_blend(bias, gain)` along the strand (`TIP`; the
/// root takes the rest); colours linear; roughnesses raw (the MDL squares
/// them); the rest as is, shared by both. Root To Tip Bias / Gain reach the
/// blend only when not 0.5 (the MDL's default, left out of the .mi);
/// `strength` reaches nothing.
pub fn blended_hair(mat: &Material) -> BTreeMap<&'static str, Arg> {
    macro_rules! tip {
        ($p:literal) => {
            concat!("material_surface_scattering.surface.scattering.components.0.component.s.s.", $p)
        };
    }
    macro_rules! blend {
        ($p:literal) => {
            concat!("material_surface_scattering.surface.scattering.components.0.weight.", $p)
        };
    }
    let ch = Channels(mat);
    let mut out = BTreeMap::new();
    out.insert("base.hair_color", Arg::Color(ch.color("Hair Root Color", [0.5; 3])));
    out.insert("base.base.surface.scattering.tint", Arg::Color(ch.color("Root Transmission Color", [0.5; 3])));
    out.insert("base.hightlight_color", Arg::Color(ch.color("Highlight Root Color", [1.0; 3])));
    out.insert(tip!("hair_color"), Arg::Color(ch.color("Hair Tip Color", [0.5; 3])));
    out.insert(tip!("base.surface.scattering.tint"), Arg::Color(ch.color("Tip Transmission Color", [0.5; 3])));
    out.insert(tip!("hightlight_color"), Arg::Color(ch.color("Tip Highlight Color", [1.0; 3])));
    for (param, channel, default) in [
        ("base.hair_weight", "Glossy Layer Weight", 0.5),
        ("base.base_roughness.roughness", "base_roughness", 0.2),
        ("base.highlight_roughness.roughness", "highlight_roughness", 0.1),
        ("base.highlight_weight", "Highlight Weight", 0.11),
        ("base.separation", "separation", 0.1),
        ("base.anisotropy", "Anisotropy", 1.0),
        ("base.anisotropy_rotation", "Anisotropy Rotations", 0.0),
        ("base.cutout_opacity", "Cutout Opacity", 1.0),
        (tip!("hair_weight"), "Glossy Layer Weight", 0.5),
        (tip!("base_roughness.roughness"), "base_roughness", 0.2),
        (tip!("highlight_roughness.roughness"), "highlight_roughness", 0.1),
        (tip!("highlight_weight"), "Highlight Weight", 0.11),
        (tip!("separation"), "separation", 0.1),
        (blend!("bias"), "Root To Tip Bias", 0.5),
        (blend!("gain"), "Root to Tip Gain", 0.5),
    ] {
        out.insert(param, Arg::Float(ch.float(channel, default)));
    }
    out
}

/// DAZ's `get_line_root_to_tip_blend` (`daz_3d/basic.mdl`): the tip's share
/// at `t` along the strand (0 root, 1 tip) — Schlick's fast bias, then his
/// gain built on it; bias and gain clamped to (0, 1), 0.5 / 0.5 = linear.
pub fn root_to_tip_blend(t: f32, bias: f32, gain: f32) -> f32 {
    let bias_fn = |t: f32, b: f32| t / ((1.0 / b - 2.0) * (1.0 - t) + 1.0);
    let (b, g) = (bias.clamp(0.00001, 0.99999), gain.clamp(0.00001, 0.99999));
    let t = bias_fn(t.clamp(0.0, 1.0), b);
    if t < 0.5 { bias_fn(t * 2.0, g) / 2.0 } else { bias_fn(t * 2.0 - 1.0, 1.0 - g) / 2.0 + 0.5 }
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
    out.insert("pbr_material.metallic", ch.float_or_map("Metallic Weight", 0.0));
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
    Light { mdl: "daz_3d::basic::dzpm_diffuse_area_light", args, shape: LightShape::Rectangle { width: 7.0, height: 3.5 }, visible: false }
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
    /// The lens options (#39), as DAZ passes them on (`mip_lens_*`,
    /// `mip_aperture_*`).
    pub lens: Lens,
}

/// Iray's lens options (Iray manual 13.3) as DAZ hands them over: the mm
/// channels in cm, the blade rotation in radians, the rest as they are
/// (Calib-Lens-*).
#[derive(Debug, Clone, PartialEq)]
pub struct Lens {
    /// Lens Shift X / Y (`mip_lens_shift_x/y`).
    pub shift: [f64; 2],
    /// Lens Stereo Offset (`mip_lens_stereo_offset`): the right eye's
    /// distance from the centre.
    pub stereo_offset: f64,
    /// Lens Thickness (`mip_lens_thickness`).
    pub thickness: f64,
    /// Lens Radial Bias (`mip_lens_radial_bias`): > 0.5 samples the lens
    /// towards its centre, < 0.5 towards its edge.
    pub radial_bias: f64,
    /// Lens Distortion Type (`mip_lens_distortion_type`).
    pub distortion: Distortion,
    /// K1–K5 (`mip_lens_distortion_params` K1–K3, `…params2` K4, K5).
    pub k: [f64; 5],
    /// Lens Distortion Scale (`mip_lens_distortion_scale`).
    pub distortion_scale: f64,
    /// Lens Maximum FOV (`mip_lens_max_fov`), radians.
    pub max_fov: f64,
    /// Aperture Blades (`mip_aperture_num_blades`): > 2 shapes the
    /// aperture as a polygon.
    pub blades: i64,
    /// Aperture Blade Rotation (`mip_aperture_blade_rotation_angle`), radians.
    pub blade_rotation: f64,
}

impl Default for Lens {
    /// DAZ's (and Iray's) defaults: a plain round lens.
    fn default() -> Self {
        Lens {
            shift: [0.0; 2],
            stereo_offset: 0.0,
            thickness: 0.0,
            radial_bias: 0.5,
            distortion: Distortion::None,
            k: [0.0; 5],
            distortion_scale: 1.0,
            max_fov: std::f64::consts::TAU,
            blades: 0,
            blade_rotation: 0.0,
        }
    }
}

/// Iray's `mip_lens_distortion_type`, in DAZ's Lens Distortion Type order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distortion {
    None,
    Spherical,
    Cylindrical,
    Poly3,
    InvPoly3,
    Poly5,
    InvPoly5,
    PtLens,
    InvPtLens,
    Brown,
    InvBrown,
    Equidistant,
}

impl Distortion {
    const ALL: [Distortion; 12] = [
        Self::None,
        Self::Spherical,
        Self::Cylindrical,
        Self::Poly3,
        Self::InvPoly3,
        Self::Poly5,
        Self::InvPoly5,
        Self::PtLens,
        Self::InvPtLens,
        Self::Brown,
        Self::InvBrown,
        Self::Equidistant,
    ];

    /// DAZ's enum index; out of range is none.
    pub fn from_daz(i: i64) -> Self {
        usize::try_from(i).ok().and_then(|i| Self::ALL.get(i)).copied().unwrap_or(Self::None)
    }

    /// The name Iray takes.
    pub fn mi_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Spherical => "spherical",
            Self::Cylindrical => "cylindrical",
            Self::Poly3 => "poly3",
            Self::InvPoly3 => "inv_poly3",
            Self::Poly5 => "poly5",
            Self::InvPoly5 => "inv_poly5",
            Self::PtLens => "ptlens",
            Self::InvPtLens => "inv_ptlens",
            Self::Brown => "brown",
            Self::InvBrown => "inv_brown",
            Self::Equidistant => "equidistant",
        }
    }
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
        lens: Lens {
            shift: [ch_num(ch, "Lens Shift X", 0.0) / 10.0, ch_num(ch, "Lens Shift Y", 0.0) / 10.0],
            stereo_offset: ch_num(ch, "Lens Stereo Offset", 0.0) / 10.0,
            thickness: ch_num(ch, "Lens Thickness", 0.0) / 10.0,
            radial_bias: ch_num(ch, "Lens Radial Bias", 0.5),
            distortion: Distortion::from_daz(ch_num(ch, "Lens Distortion Type", 0.0) as i64),
            k: [1, 2, 3, 4, 5].map(|i| ch_num(ch, &format!("Lens Distortion K{i}"), 0.0)),
            distortion_scale: ch_num(ch, "Lens Distortion Scale", 1.0),
            max_fov: ch_num(ch, "Lens Maximum FOV", std::f64::consts::TAU),
            blades: ch_num(ch, "Aperture Blades", 0.0) as i64,
            blade_rotation: ch_num(ch, "Aperture Blade Rotation", 0.0).to_radians(),
        },
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

/// Whether Iray burns the highlights per channel
/// (`mip_burn_highlights_per_component`, Burn Highlights Per Component);
/// off, it compresses the luminance and keeps the chromaticity.
pub fn burn_per_component(ch: &NodeChannels) -> bool {
    ch_bool(ch, "Burn Highlights Per Component", true)
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

/// The dome's placement and colour DAZ hands Iray in every mode but Scene
/// Only (Calib-L-Mi-Dome-turned): `environment_dome_rotation_angle` = Dome
/// Rotation in turns about Y, `environment_function_orientation` = Rz · Rx ·
/// Ry of Dome Orientation Z / X / Y (degrees, row-major), the tint linear,
/// and the shadow-catching ground.
#[derive(Debug, Clone, PartialEq)]
pub struct DomeSetup {
    pub rotation_turns: f64,
    pub orientation: [f64; 9],
    pub tint: [f64; 3],
    pub ground: bool,
    /// The ground's height (cm) in Manual position mode, else 0.
    pub ground_y: f64,
    pub ground_shadow: f64,
    pub ground_glossiness: f64,
    pub ground_reflectivity: [f64; 3],
    /// Ground Visible from Below — DAZ hands it only with Draw Ground on
    /// (Calib-Env-Mi-GroundBelow; Calib-Env-Mi-Box, ground off: true).
    pub ground_visible_from_below: bool,
}

pub fn dome_setup(ch: &NodeChannels) -> DomeSetup {
    let deg = |k: &str| ch_num(ch, k, 0.0).to_radians();
    let (x, y, z) = (deg("Dome Orientation X"), deg("Dome Orientation Y"), deg("Dome Orientation Z"));
    let rx = [[1.0, 0.0, 0.0], [0.0, x.cos(), -x.sin()], [0.0, x.sin(), x.cos()]];
    let ry = [[y.cos(), 0.0, y.sin()], [0.0, 1.0, 0.0], [-y.sin(), 0.0, y.cos()]];
    let rz = [[z.cos(), -z.sin(), 0.0], [z.sin(), z.cos(), 0.0], [0.0, 0.0, 1.0]];
    let mul = |a: [[f64; 3]; 3], b: [[f64; 3]; 3]| -> [[f64; 3]; 3] { std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum())) };
    let m = mul(mul(rz, rx), ry);
    DomeSetup {
        rotation_turns: ch_num(ch, "Dome Rotation", 0.0) / 360.0,
        orientation: std::array::from_fn(|i| m[i / 3][i % 3]),
        tint: ch_colour(ch, "Environment Tint", [1.0; 3]),
        ground: ch_bool(ch, "Draw Ground", true),
        ground_y: if ch_num(ch, "Ground Position Mode", 0.0) == 1.0 { ch_num(ch, "Ground Origin Y", 0.0) } else { 0.0 },
        ground_shadow: ch_num(ch, "Ground Shadow Intensity", 1.0),
        ground_glossiness: ch_num(ch, "Ground Glossiness", 0.1),
        ground_reflectivity: ch_colour(ch, "Ground Reflectivity", [0.0; 3]),
        ground_visible_from_below: !ch_bool(ch, "Draw Ground", true) || ch_bool(ch, "Ground Visible from Below", true),
    }
}

/// The dome's shape (Calib-Env-DomeMode*, Calib-Env-Mi-Box / -SphereGround):
/// `environment_dome_mode` for Dome Mode 0 – 5, the radius and box size in
/// cm = Dome Radius / Width / Height / Depth × Dome Scale Multiplier, the
/// position = Dome Origin (cm) as it is, `environment_dome_ground_texturescale`
/// = Ground Texture Scale (cm). The sizes and position reach the .mi only for
/// a finite dome; the texture scale always.
#[derive(Debug, Clone, PartialEq)]
pub struct DomeShape {
    pub mode: &'static str,
    pub radius: f64,
    pub size: [f64; 3],
    pub position: [f64; 3],
    pub ground_texture_scale: f64,
    /// Visualize Finite Dome: Iray's false-colour overlay of the dome.
    pub visualize: bool,
}

impl DomeShape {
    /// The infinite modes ("infinite", "ground").
    pub fn finite(&self) -> bool {
        !matches!(self.mode, "infinite" | "ground")
    }

    /// The lower half of the HDRI projected onto a ground plane.
    pub fn projected_ground(&self) -> bool {
        self.mode == "ground" || self.mode.ends_with("_with_ground")
    }
}

pub fn dome_shape(ch: &NodeChannels) -> DomeShape {
    const MODES: [&str; 6] = ["infinite", "ground", "sphere", "sphere_with_ground", "box", "box_with_ground"];
    let scale = ch_num(ch, "Dome Scale Multiplier", 100.0);
    DomeShape {
        mode: MODES[(ch_num(ch, "Dome Mode", 0.0).max(0.0) as usize).min(5)],
        radius: ch_num(ch, "Dome Radius", 100.0) * scale,
        size: [("Dome Width", 200.0), ("Dome Height", 200.0), ("Dome Depth", 200.0)].map(|(k, d)| ch_num(ch, k, d) * scale),
        position: ["X", "Y", "Z"].map(|a| ch_num(ch, &format!("Dome Origin {a}"), 0.0)),
        ground_texture_scale: ch_num(ch, "Ground Texture Scale", 100.0),
        visualize: ch_bool(ch, "Visualize Finite Dome", false),
    }
}

/// Iray's ground fog (`iray_ground_fog_*`, Calib-Env-GroundFog, -Mi-Fog,
/// -Mi-FogHG), when Ground Fog is on: the densities (extinction per meter)
/// are the linear Density Start / End colours × their scales, the heights
/// (m) Decay Start and Decay Start + Decay Height — absolute, not from the
/// ground (Calib-Env-Mi-Fog: the ground at 30 cm, the fog from 50 cm). The
/// albedo linear; the anisotropy only for Henyey-Greenstein (0 under Mie),
/// the particle size (µm) as it is.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundFog {
    pub albedo: [f64; 3],
    pub density0: [f64; 3],
    pub density1: [f64; 3],
    pub height0: f64,
    pub height1: f64,
    /// "approx_mie" (Scattering Type 1), else "hg".
    pub mie: bool,
    pub anisotropy: f64,
    pub particle_size: f64,
}

impl GroundFog {
    /// The density's exponential decay length (m) above `height0`, from
    /// NVIDIA's `decay = (h0 − h1) / ln(d1 / d0)` (Iray manual 10.6), on
    /// the densities' mean; infinite for a uniform fog.
    pub fn decay(&self) -> f64 {
        let mean = |d: [f64; 3]| d.iter().sum::<f64>() / 3.0;
        let (d0, d1) = (mean(self.density0), mean(self.density1).min(mean(self.density0)));
        if d1 <= 0.0 {
            return 0.0;
        }
        let r = (d1 / d0).ln();
        if r == 0.0 { f64::INFINITY } else { (self.height0 - self.height1) / r }
    }
}

pub fn ground_fog(ch: &NodeChannels) -> Option<GroundFog> {
    if !ch_bool(ch, "Ground Fog", false) {
        return None;
    }
    let start = ch_num(ch, "Ground Fog Decay Start", 0.0);
    let mie = ch_num(ch, "Ground Fog Scattering Type", 0.0) == 1.0;
    let density = |k: &str, scale: &str, d: f64| ch_colour(ch, k, [d.powf(COLOUR_GAMMA as f64); 3]).map(|c| c * ch_num(ch, scale, 1.0));
    Some(GroundFog {
        albedo: ch_colour(ch, "Ground Fog Albedo", [0.6705883, 0.682353, 0.6901961].map(|c: f64| c.powf(COLOUR_GAMMA as f64))),
        density0: density("Ground Fog Density Start", "Ground Fog Density Start Scale", 0.990859),
        density1: density("Ground Fog Density End", "Ground Fog Density End Scale", 0.01519911),
        height0: start / 100.0,
        height1: (start + ch_num(ch, "Ground Fog Decay Height", 91.44)) / 100.0,
        mie,
        anisotropy: if mie { 0.0 } else { ch_num(ch, "Ground Fog Anisotropy", 0.0) },
        particle_size: ch_num(ch, "Ground Fog Particle Size", 0.0),
    })
}

/// Iray's matte fog (`matte_fog_*`, Calib-Env-MatteFog, -Mi-MatteFog), when
/// Matte Fog is on: the visibility in meters as it is (Koschmieder's
/// contrast distance), the tints linear, the brightness and its "relative
/// to environment" switch as they are.
#[derive(Debug, Clone, PartialEq)]
pub struct MatteFog {
    pub visibility: f64,
    pub visibility_tint: [f64; 3],
    pub brightness: f64,
    pub relative: bool,
    pub brightness_tint: [f64; 3],
}

pub fn matte_fog(ch: &NodeChannels) -> Option<MatteFog> {
    ch_bool(ch, "Matte Fog", false).then(|| MatteFog {
        visibility: ch_num(ch, "Matte Fog Visibility", 10_000.0),
        visibility_tint: ch_colour(ch, "Matte Fog Visibility Tint", [1.0; 3]),
        brightness: ch_num(ch, "Matte Fog Brightness", 1.0),
        relative: ch_bool(ch, "Matte Fog Brightness Relative to Environment", true),
        brightness_tint: ch_colour(ch, "Matte Fog Brightness Tint", [1.0; 3]),
    })
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

/// Iray Photoreal's antialiasing filter (`filter` in the .mi options:
/// its kind and width, twice DAZ's Pixel Filter Radius).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFilter {
    Box,
    Triangle,
    Gauss,
}

impl PixelFilter {
    /// The .mi's name for it.
    pub fn mi_name(self) -> &'static str {
        match self {
            PixelFilter::Box => "box",
            PixelFilter::Triangle => "triangle",
            PixelFilter::Gauss => "gauss",
        }
    }
}

/// Iray's bloom filter (`iray_bloom_filtering_*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bloom {
    /// Blur radius as a fraction of the output resolution.
    pub radius: f64,
    /// The value above which a pixel blooms.
    pub threshold: f64,
    /// Scale of the blurred bright spots when composited.
    pub brightness_scale: f64,
}

/// Iray Photoreal's render options as DAZ passes them (the .mi's
/// `options`), from the render settings' channels
/// (`crate::usd::render_channels`).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// `iray_max_path_length`: segments of a light path, 2 = direct light
    /// only. None (DAZ's −1) = unbounded; DAZ then leaves it out.
    pub max_path_length: Option<u32>,
    /// `iray_max_sss_path_length`: volume vertices on a path.
    pub max_sss_path_length: Option<u32>,
    /// `iray_firefly_filter`.
    pub firefly_filter: bool,
    /// `iray_caustic_sampler`.
    pub caustic_sampler: bool,
    /// `iray_spectral_rendering`.
    pub spectral: bool,
    pub filter: PixelFilter,
    /// The filter's width in pixels (the .mi's `filter "gauss 3"`).
    pub filter_width: f64,
    /// `iray_bloom_filtering` and its settings when on.
    pub bloom: Option<Bloom>,
}

impl Default for RenderOptions {
    /// DAZ's defaults.
    fn default() -> Self {
        render_options(&NodeChannels::new())
    }
}

/// DAZ's path-length channels: −1 (or less than 1) is unbounded.
fn path_length(ch: &NodeChannels, k: &str) -> Option<u32> {
    let n = ch_num(ch, k, -1.0);
    (n >= 1.0).then_some(n as u32)
}

pub fn render_options(ch: &NodeChannels) -> RenderOptions {
    let filter = match ch_num(ch, "Pixel Filter", 2.0) as i32 {
        0 => PixelFilter::Box,
        1 => PixelFilter::Triangle,
        _ => PixelFilter::Gauss,
    };
    RenderOptions {
        max_path_length: path_length(ch, "Max Path Length"),
        max_sss_path_length: path_length(ch, "Max SSS Path Length"),
        firefly_filter: ch_bool(ch, "Firefly Filter Enable", true),
        caustic_sampler: ch_bool(ch, "Caustic Sampler", false),
        spectral: ch_bool(ch, "Spectral Rendering Enable", false),
        filter,
        filter_width: 2.0 * ch_num(ch, "Pixel Filter Radius", 1.5),
        bloom: ch_bool(ch, "Bloom Filter Enable", false).then(|| Bloom {
            radius: ch_num(ch, "Bloom Filter Radius", 0.05),
            threshold: ch_num(ch, "Bloom Filter Threshold", 2000.0),
            brightness_scale: ch_num(ch, "Bloom Filter Brightness Scale", 1.0),
        }),
    }
}

#[cfg(test)]
mod tests {
    //! Every rule against DAZ's own `.mi` export of the calibration scenes.
    use super::*;
    use crate::daz_material::{image_library, Material};
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn legacy_shaders_are_recognised_and_keep_maps_only_for_the_export() {
        let images = HashMap::new();
        let mut mat = Material::from_dson(&serde_json::json!({"id": "Default", "extra": [{"type": "studio/material/daz_shader", "definition": "support/DAZ/shaderDefinitions/surface/dzglossyDef.dsa"}]}), &images);
        mat.merge(
            &serde_json::json!({"url": "#Default", "extra": [{"type": "studio/material/daz_shader"}],
                "diffuse": {"channel": {"id": "diffuse", "current_value": [0.8, 0.4, 0.2], "image_file": "/Runtime/Textures/hair.jpg"}},
                "transparency": {"channel": {"id": "transparency", "current_value": 1, "image_file": "/Runtime/Textures/alpha.jpg"}}}),
            &images,
        );
        assert_eq!(mat.shader.as_deref(), Some("support/DAZ/shaderDefinitions/surface/dzglossyDef.dsa"));
        let iray = as_uber(&mat).unwrap();
        assert!(iray.image("diffuse").is_none() && iray.image("Cutout Opacity").is_none());
        let export = legacy_shader_as_uber(&mat, true).unwrap();
        assert_eq!(export.image("Cutout Opacity"), Some("/Runtime/Textures/alpha.jpg"));
        assert_eq!(export.image("diffuse"), Some("/Runtime/Textures/hair.jpg"));
        assert_eq!(Channels(&export).float("Glossy Layered Weight", 0.0), 1.0);

        let aoa = Material::from_dson(&serde_json::json!({"url": "/data/Age%20of%20Armour/Subsurface%20Shaders/AoA_Subsurface/AoA_Subsurface.dsf#AoA_Subsurface", "extra": [{"type": "studio/material/daz_brick"}]}), &images);
        assert!(is_aoa_subsurface(&aoa) && !is_legacy_shader(&aoa));
        let other = Material::from_dson(&serde_json::json!({"url": "/data/x/Other.dsf#Other", "extra": [{"type": "studio/material/daz_brick"}]}), &images);
        assert!(as_uber(&other).is_none());
    }

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
            Arg::LightProfile(_) => Arg::LightProfile(v["light_profile"]["file"].as_str()?.to_string()),
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
            // `file_texture`'s defaults: no offset, `mono_alpha`.
            let offset = f32s(&f["args"]["color_offset"]).map_or([0.0; 3], |c| [c[0], c[1], c[2]]);
            let mono = match f["args"]["mono_source"].as_str() {
                Some("mono_average") => Mono::Average,
                Some("mono_maximum") => Mono::Maximum,
                Some("mono_luminance") => return None,
                _ => Mono::Alpha,
            };
            let m = f32s(&f["args"]["uvw"]["args"]["transform"]).unwrap_or_else(|| vec![1.0, 0., 0., 0., 0., 1.0, 0., 0., 0., 0., 1.0, 0., 0., 0., 0., 1.0]);
            Some(Map {
                image: t["path"].as_str()?.to_string(),
                kind,
                scale,
                offset,
                mono,
                value: scale,
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
        if mdl.starts_with("daz_3d::irayubermaterial::attenuate_color") {
            let c = f32s(&a["color_value"]).filter(|c| c.len() == 3)?;
            return fixture_map(&a["attenuation"]).map(|m| Map { scale: [0, 1, 2].map(|i| m.scale[i] * c[i]), offset: [0, 1, 2].map(|i| m.offset[i] * c[i]), ..m });
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
        if mdl.starts_with("base::tangent_space_normal_texture") {
            return file(v, MapKind::Normal, [a["factor"].as_f64().unwrap_or(1.0) as f32; 3]);
        }
        None
    }

    fn mi_mono(m: Mono) -> &'static str {
        match m {
            Mono::Average => "mono_average",
            Mono::Alpha => "mono_alpha",
            Mono::Red | Mono::Green | Mono::Blue | Mono::Maximum => "mono_maximum",
        }
    }

    fn close(a: &Arg, b: &Arg) -> bool {
        // The .mi prints 6 significant digits.
        let near = |x: f32, y: f32| (x - y).abs() <= 1e-5 + 1e-5 * x.abs().max(y.abs());
        match (a, b) {
            (Arg::Float(x), Arg::Float(y)) => near(*x, *y),
            (Arg::Color(x), Arg::Color(y)) => (0..3).all(|i| near(x[i], y[i])),
            // The .mi names the file it found: compare the file names.
            (Arg::LightProfile(x), Arg::LightProfile(y)) => x.rsplit('/').next() == y.rsplit('/').next(),
            (Arg::Map(x), Arg::Map(y)) => {
                x.image.trim_start_matches('/') == y.image.trim_start_matches('/')
                    && x.kind == y.kind
                    && (0..3).all(|i| near(x.scale[i], y.scale[i]) && near(x.offset[i], y.offset[i]))
                    && (0..4).all(|i| near(x.tiling[i], y.tiling[i]))
                    // The .mi names the source a one-channel mode reads by
                    // (`mono_maximum`); a bump has none.
                    && (matches!(x.kind, MapKind::Bump | MapKind::Normal) || mi_mono(x.mono) == mi_mono(y.mono))
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
                let converted = as_uber(mat);
                let mat = converted.as_ref().unwrap_or(mat);
                // AoA Subsurface's subsurface (SSS Material Type Custom)
                // isn't reproduced (`aoa_subsurface_as_uber`).
                let aoa_sss = mi["args"].get("sub_surface_enable").is_some() && converted.is_some() && mats.get(key).is_some_and(is_aoa_subsurface);
                checked += 1;
                if mi["mdl"] != uber_entry(mat) {
                    wrong.push(format!("MDL entry: {scene} {key}: DAZ {}, ours {}", mi["mdl"], uber_entry(mat)));
                }
                for (param, ours) in uber(mat) {
                    if aoa_sss && (["sub_surface_", "scattering_", "transmitted_", "absorption_"].iter().any(|p| param.starts_with(p)) || param == "glossy_ior") {
                        continue;
                    }
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
                checked += 1;
                if mi["fields"]["visible"].as_bool() != Some(ours.visible) {
                    wrong.push(format!("visible: {scene} {label}: DAZ {}, ours {}", mi["fields"]["visible"], ours.visible));
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

            // Render options (#43).
            let r = render_options(&crate::usd::render_channels(&doc));
            let o = &mi["options"];
            let flag = |v: &Value| f64::from(u8::from(v.as_bool().unwrap_or(false)));
            let ours = |b: bool| f64::from(u8::from(b));
            let (kind, width) = o["filter"].as_str().and_then(|s| s.split_once(' ')).unwrap_or(("?", "NaN"));
            check(format!("{scene} pixel filter {}", r.filter.mi_name()), 1.0, ours(kind == r.filter.mi_name()));
            check(format!("{scene} pixel filter width"), width.parse().unwrap_or(f64::NAN), r.filter_width);
            check(format!("{scene} firefly filter"), flag(&o["iray_firefly_filter"]), ours(r.firefly_filter));
            check(format!("{scene} caustic sampler"), flag(&o["iray_caustic_sampler"]), ours(r.caustic_sampler));
            check(format!("{scene} spectral"), flag(&o["iray_spectral_rendering"]), ours(r.spectral));
            // DAZ leaves the path lengths out when unbounded.
            check(format!("{scene} max path length"), o["iray_max_path_length"].as_f64().unwrap_or(-1.0), r.max_path_length.map_or(-1.0, f64::from));
            check(format!("{scene} max sss path length"), o["iray_max_sss_path_length"].as_f64().unwrap_or(-1.0), r.max_sss_path_length.map_or(-1.0, f64::from));
            check(format!("{scene} bloom"), flag(&o["iray_bloom_filtering"]), ours(r.bloom.is_some()));
            if let Some(b) = r.bloom {
                check(format!("{scene} bloom radius"), num(&o["iray_bloom_filtering_radius"]), b.radius);
                check(format!("{scene} bloom threshold"), num(&o["iray_bloom_filtering_threshold"]), b.threshold);
                check(format!("{scene} bloom brightness"), num(&o["iray_bloom_filtering_brightness_scale"]), b.brightness_scale);
            }

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
                // Lens options (#39).
                let l = &c.lens;
                check(format!("{scene} lens shift x"), num(&m["mip_lens_shift_x"]), l.shift[0]);
                check(format!("{scene} lens shift y"), num(&m["mip_lens_shift_y"]), l.shift[1]);
                check(format!("{scene} stereo offset"), num(&m["mip_lens_stereo_offset"]), l.stereo_offset);
                check(format!("{scene} lens thickness"), num(&m["mip_lens_thickness"]), l.thickness);
                check(format!("{scene} radial bias"), num(&m["mip_lens_radial_bias"]), l.radial_bias);
                check(format!("{scene} distortion {}", l.distortion.mi_name()), 1.0, f64::from(u8::from(m["mip_lens_distortion_type"].as_str() == Some(l.distortion.mi_name()))));
                for i in 0..5 {
                    let (key, j) = if i < 3 { ("mip_lens_distortion_params", i) } else { ("mip_lens_distortion_params2", i - 3) };
                    check(format!("{scene} K{}", i + 1), num(&m[key][j]), l.k[i]);
                }
                check(format!("{scene} distortion scale"), num(&m["mip_lens_distortion_scale"]), l.distortion_scale);
                check(format!("{scene} max fov"), num(&m["mip_lens_max_fov"]), l.max_fov);
                check(format!("{scene} blades"), num(&m["mip_aperture_num_blades"]), l.blades as f64);
                check(format!("{scene} blade rotation"), num(&m["mip_aperture_blade_rotation_angle"]), l.blade_rotation);
            }
            if let Some(ch) = by_id(&nodes, "Tonemapper Options") {
                for (param, v) in tone(ch) {
                    check(format!("{scene} {param}"), num(&mi["camera"][param]), v);
                }
                let flag = |b: Option<bool>| b.map_or(f64::NAN, |b| b as u8 as f64);
                check(format!("{scene} burn per component"), flag(mi["camera"]["mip_burn_highlights_per_component"].as_bool()), flag(Some(burn_per_component(ch))));
                let wp = white_point(ch);
                for i in 0..3 {
                    check(format!("{scene} whitepoint {i}"), num(&mi["camera"]["mip_whitepoint"][i]), wp[i]);
                }
            }
            if let Some(ch) = by_id(&nodes, "Environment Options") {
                let o = &mi["options"];
                let d = dome_setup(ch);
                check(format!("{scene} dome rotation"), num(&o["environment_dome_rotation_angle"]), d.rotation_turns);
                for i in 0..9 {
                    check(format!("{scene} dome orientation {i}"), num(&o["environment_function_orientation"][i]), d.orientation[i]);
                }
                for i in 0..3 {
                    check(format!("{scene} dome tint {i}"), num(&o["environment_function_tint"][i]), d.tint[i]);
                    check(format!("{scene} ground reflectivity {i}"), num(&o["environment_dome_ground_reflectivity"][i]), d.ground_reflectivity[i]);
                }
                check(format!("{scene} ground"), f64::from(u8::from(o["environment_dome_ground"].as_bool().unwrap_or(false))), f64::from(u8::from(d.ground)));
                // The ground's height: its position's y, with DAZ's 1 mm drop at 0.
                let gy = num(&o["environment_dome_ground_position"][1]);
                check(format!("{scene} ground height"), if d.ground_y == 0.0 { gy + 0.001 } else { gy }, d.ground_y);
                check(format!("{scene} ground shadow"), num(&o["environment_dome_ground_shadow_intensity"]), d.ground_shadow);
                check(format!("{scene} ground glossiness"), num(&o["environment_dome_ground_glossiness"]), d.ground_glossiness);
                let flag = |v: &Value| f64::from(u8::from(v.as_bool().unwrap_or(false)));
                check(format!("{scene} ground visible from below"), flag(&o["environment_dome_ground_visible_from_below"]), f64::from(u8::from(d.ground_visible_from_below)));
                // The dome's shape; sizes and position only for a finite one.
                let s = dome_shape(ch);
                check(format!("{scene} dome mode {}", s.mode), 1.0, f64::from(u8::from(o["environment_dome_mode"].as_str() == Some(s.mode))));
                check(format!("{scene} dome visualize"), flag(&o["environment_dome_visualize"]), f64::from(u8::from(s.visualize)));
                check(format!("{scene} ground texture scale"), num(&o["environment_dome_ground_texturescale"]), s.ground_texture_scale);
                if s.mode.starts_with("sphere") {
                    check(format!("{scene} dome radius"), num(&o["environment_dome_radius"]), s.radius);
                }
                if s.mode.starts_with("box") {
                    for (i, k) in ["width", "height", "depth"].iter().enumerate() {
                        check(format!("{scene} dome {k}"), num(&o[format!("environment_dome_{k}")]), s.size[i]);
                    }
                }
                if s.finite() {
                    for i in 0..3 {
                        check(format!("{scene} dome position {i}"), num(&o["environment_dome_position"][i]), s.position[i]);
                    }
                }
                check(format!("{scene} ground fog"), flag(&o["iray_ground_fog"]), f64::from(u8::from(ground_fog(ch).is_some())));
                if let Some(g) = ground_fog(ch) {
                    for i in 0..3 {
                        check(format!("{scene} fog albedo {i}"), num(&o["iray_ground_fog_albedo"][i]), g.albedo[i]);
                        check(format!("{scene} fog density0 {i}"), num(&o["iray_ground_fog_density0"][i]), g.density0[i]);
                        check(format!("{scene} fog density1 {i}"), num(&o["iray_ground_fog_density1"][i]), g.density1[i]);
                    }
                    check(format!("{scene} fog height0"), num(&o["iray_ground_fog_height0"]), g.height0);
                    check(format!("{scene} fog height1"), num(&o["iray_ground_fog_height1"]), g.height1);
                    check(format!("{scene} fog mie"), f64::from(u8::from(o["iray_ground_fog_scattering_type"] == "approx_mie")), f64::from(u8::from(g.mie)));
                    check(format!("{scene} fog anisotropy"), num(&o["iray_ground_fog_anisotropy"]), g.anisotropy);
                    check(format!("{scene} fog particle size"), num(&o["iray_ground_fog_particle_size"]), g.particle_size);
                }
                check(format!("{scene} matte fog"), flag(&o["matte_fog"]), f64::from(u8::from(matte_fog(ch).is_some())));
                if let Some(m) = matte_fog(ch) {
                    check(format!("{scene} matte fog visibility"), num(&o["matte_fog_visibility"]), m.visibility);
                    check(format!("{scene} matte fog brightness"), num(&o["matte_fog_brightness"]), m.brightness);
                    check(format!("{scene} matte fog relative"), flag(&o["matte_fog_brightness_relative_to_environment"]), f64::from(u8::from(m.relative)));
                    for i in 0..3 {
                        check(format!("{scene} matte fog visibility tint {i}"), num(&o["matte_fog_visibility_tint"][i]), m.visibility_tint[i]);
                        check(format!("{scene} matte fog brightness tint {i}"), num(&o["matte_fog_brightness_tint"][i]), m.brightness_tint[i]);
                    }
                }
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
                // A map: the whole texture call (`fixture_map` reads it).
                Some(n) if n.starts_with("base::texture_return") => {
                    out.insert(format!("{func}.{k}"), a.clone());
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
            None if v.is_array() && v.as_array().is_some_and(|a| a.iter().all(Value::is_object)) => {
                for (k, a) in v.as_array().into_iter().flatten().enumerate() {
                    graph_leaves(a, &join(&k.to_string()), out);
                }
            }
            None if !v.is_object() => {
                out.insert(path.to_string(), v.clone());
            }
            None => {}
        }
    }

    #[test]
    fn root_to_tip_blend_is_daz_bias_and_gain() {
        // 0.5 / 0.5: linear; the ends stay put whatever the shape.
        for t in [0.0, 0.25, 0.5, 0.9, 1.0] {
            assert!((root_to_tip_blend(t, 0.5, 0.5) - t).abs() < 1e-6);
        }
        for (b, g) in [(0.2, 0.7), (0.8, 0.3)] {
            assert!(root_to_tip_blend(0.0, b, g).abs() < 1e-6 && (root_to_tip_blend(1.0, b, g) - 1.0).abs() < 1e-6);
        }
        // Bias 0.2 holds the root colour longer: Schlick's bias(0.5, 0.2) = 0.2.
        assert!((root_to_tip_blend(0.5, 0.2, 0.5) - 0.2).abs() < 1e-6);
        // Gain 0.7 flattens the middle: gain(0.25, 0.7) = bias(0.5, 0.7) / 2.
        assert!((root_to_tip_blend(0.25, 0.5, 0.7) - 0.35).abs() < 1e-6);
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
        for scene in ["Calib-Mi-Hair", "Calib-Mi-Hair2"] {
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
                // The blend's bias / gain at the MDL's 0.5 stay out of the .mi.
                let mdl_default = (param.ends_with(".bias") || param.ends_with(".gain")) && close(&arg, &Arg::Float(0.5));
                if !t.as_ref().map_or(mdl_default, |t| close(t, &arg)) {
                    wrong.push(format!("{param}: {scene} {key}: DAZ {t:?}, ours {arg:?}"));
                }
            }
        }
        }
        wrong.sort();
        assert!(checked > 0 && wrong.is_empty(), "{} of {checked} hair arguments differ from DAZ's .mi:\n{}", wrong.len(), wrong.join("\n"));
    }
}
