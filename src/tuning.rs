//! The four export variants — [`Look`] × [`OctaneMode`] — and everything
//! that differs between them, in one table ([`Variant::tuning`], #44).
//!
//! Calibration goes variant by variant: Enhanced + Final first (path
//! tracing, the photoreal one), then the same calibration scenes for each
//! of the other three (docs/calibration.md). A value that differs between
//! variants is a field of [`Tuning`], set in every row of the table, so
//! what a variant does differently is read off its row and every exception
//! is found by its field. Everything else (light powers, material rules,
//! hair) is shared and fitted on Enhanced + Final — see the spec's
//! "Variants" table (docs/spec/octane-export.md).

use crate::material::{RoughLobe, SkinShader, StrandHair};
use crate::usd::SUN_SKY_POWER;

/// Where Iray and Octane differ in a way the export can choose: stay with
/// Iray's look (`Faithful`) or keep Octane's more plausible one
/// (`Enhanced`, the default — docs/spec/octane-export.md, Look).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Look {
    Faithful,
    #[default]
    Enhanced,
}

/// How the Octane project renders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OctaneMode {
    /// Direct lighting (Octane's fast, biased preview kernel), Universal
    /// skin — what the lights were first calibrated on.
    #[default]
    Preview,
    /// Path tracing, skin with random-walk subsurface scattering (direct
    /// lighting renders that grey) and the denoiser on. Both modes take the
    /// imager from DAZ's tone mapping (`ToneMapping`).
    Final,
}

/// Octane's daylight model as Faithful lights with it: at 2.27 the 18:00
/// sun (Calib-L-Mode-SunSky-off, 15.5°) came out 1.86 × Iray, noon (40°)
/// 1.08 ×. The floor doesn't follow the power linearly (18:00 at power
/// 1.22 / 1.32 / 1.34 / 1.39: 0.85 / 0.92 / 0.94 / 1.12 ×), so P · exp(−τ /
/// sin e) went through noon at 2.10 and 18:00 at 1.36: 18:00 0.95, noon
/// 1.00, Calib-L-Mode-SunSky-dir (43°) 0.98 × Iray.
const FAITHFUL_DAYLIGHT_POWER: f64 = 2.864;
const FAITHFUL_DAYLIGHT_EXTINCTION: f64 = 0.2001;

/// Octane's maximum samples per pixel for a Final render (Render modes,
/// #180). A judgment call, not a calibration: 500 is what every export had
/// before the option; it trades detail for time. #47 measured Victoria's
/// random-walk skin pores (denoised) 15 - 25 % crisper at 2000, but 91 s ->
/// 428 s at 960 x 720; the user picks more per export (`--samples`, the
/// dialog). Revisit on real scenes: raise it if most Final renders look
/// soft at 500.
const FINAL_SAMPLES: u32 = 500;
/// Preview's (direct lighting): kept at the former 500 too, so an unchanged
/// export renders as before (#180). Direct lighting converges much sooner;
/// a lower default would make Preview faster - a judgment call left for
/// when Preview is judged on real scenes.
const PREVIEW_SAMPLES: u32 = 500;

/// Rough dielectric Phong lobes (α past Octane's Beckmann limit) per Look
/// (`RoughLobe`, #162, the user's call 2026-11-16). A contested choice:
/// neither model has Iray's lobe in both shape and level. Enhanced keeps
/// GGX energy preserving - Iray's level under an environment (rough gloss
/// 1.02 - 1.08 x Iray), a highlight ~0.8 x as wide under a small light.
/// Faithful takes STD - Iray's highlight shape head-on (widths 1.00 - 1.02
/// x), paying rough gloss 1.2 - 1.5 x Iray under an HDRI. Revisit if a
/// model in Octane gets both. The Uber skin's dual lobe follows the same
/// split (#175, the user's call 2026-11-16; `RoughLobe::skin_dual_lobe_layers`):
/// Enhanced keeps OpenPBR's GGX lobes (Iray's energy, a narrower core, the
/// faces' dual lobe p99.5 ~0.8 x Iray), Faithful puts them on Specular
/// Layers at this model - Iray's Phong shape. The sharp dielectric lobes
/// (α up to 0.25) follow too (#196): Faithful's STD keeps the diffuse under
/// them at Iray's flat 1 - R W where Beckmann dims it at the rim, Enhanced
/// keeps Beckmann (`RoughLobe::sharp_brdf`). PBRSkin's lobes are GGX,
/// so none of these splits applies to them (`material::ggx_brdf`, #186).
const ENHANCED_ROUGH_LOBE: RoughLobe = RoughLobe::GgxEnergyPreserving;
const FAITHFUL_ROUGH_LOBE: RoughLobe = RoughLobe::Std;

/// Iray's legacy rim on NVIDIA's textured `mdl 1.0` metals per Look
/// (`legacy_metal_rim`, #112). Iray's `simple_glossy_bsdf` in a module below
/// MDL 1.3 with a non-constant roughness (any texture lookup, any varying
/// expression) loses 0.65 - 0.69 x its 1.3 twin at the rim (θ_v 64°) and
/// gains or loses head-on (1.04 / 0.88 / 0.83 at r 0.1 / 0.4 / 0.7) - the
/// same lobe with a constant roughness, the 1.3 lobe and the Uber with a
/// Glossy Roughness map stay flat (Calib-Furnace-MdlRoughTex,
/// -UberRoughMap). A texture lookup changing the BSDF isn't physical, so
/// Enhanced keeps Octane's flat metal; Faithful draws Iray's rim with a
/// Falloff Mix on the metal's albedo (`legacy_metal_falloff`).
const ENHANCED_LEGACY_METAL_RIM: bool = false;
const FAITHFUL_LEGACY_METAL_RIM: bool = true;

/// Enhanced's skin detail contrast s (`skin_detail_contrast`, #47, the
/// user's call 2026-11-16): the baked skin colour × (D / D_low)^s, D / D_low
/// the diffuse map's own detail, so pores and fine wrinkles get (1 + s) x
/// their contrast. A look, not a match: denoised Enhanced already keeps
/// Iray's texture within 0.5 - 0.9 x (pores softened by the denoiser on the
/// random walk's grain, docs/notes/2026-11-12-skin-texture-contrast.md); it
/// trades Iray's texture for crisper skin. 0.15 for light and dark skins
/// alike is the starting point; the final s, and whether dark skins want
/// their own, is open in #179 - judge it on Calib-Skin fronts and Demo-*
/// figures, not spheres. Faithful keeps 0 (Iray's texture).
const ENHANCED_SKIN_DETAIL_CONTRAST: f32 = 0.15;
const FAITHFUL_SKIN_DETAIL_CONTRAST: f32 = 0.0;

/// One of the four exports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Variant {
    pub look: Look,
    pub mode: OctaneMode,
}

/// Octane's render kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kernel {
    /// Direct lighting (24), GI diffuse, depth 2.
    DirectLighting,
    /// Path tracing (25): diffuse 8, glossy 24, scatter 128, GI clamp 20.
    PathTracing,
}

/// Everything that differs between the variants. Each field names the spec
/// section its numbers come from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Render modes.
    pub kernel: Kernel,
    /// The kernel's maximum samples per pixel (Render modes); an export's
    /// `--samples` replaces it (`ExportOptions::tuning`).
    pub samples: u32,
    /// Imager exposure at DAZ's default EV 13 (Render modes): 1.4 under
    /// path tracing (Octane2's Iray render, HDRI only) and under direct
    /// lighting too — at the old 1 every Preview floor came out 0.71 –
    /// 0.74 × Iray (lights, Sun-Sky, HDRI); at 1.4 0.99 – 1.04.
    pub base_exposure: f64,
    /// Octane's AI denoiser on the render (Render modes); `--no-denoise`
    /// turns it off in any variant.
    pub denoiser: bool,
    /// Skin with subsurface scattering, and on which material (Look,
    /// Universal): random walk needs path tracing.
    pub skin: SkinShader,
    /// Thin-walled glass at IOR 1 — Iray's showed none of its own
    /// reflection — instead of its Fresnel reflection (Look). Only where
    /// Iray's refraction doesn't reflect (no glossy layer, #101).
    pub thin_glass_ior_one: bool,
    /// Daylight (Sun-Sky) model: 3 Hosek-Wilkie, which warms a low sun; 1
    /// Octane's, with a white sunset colour like Iray's (Environment).
    pub daylight_model: u8,
    /// Daylight `power` at DAZ's default `SS Multiplier` (Environment),
    /// × exp(−`daylight_extinction` / sin(sun elevation)).
    pub daylight_power: f64,
    /// Dims a low sun by its air mass: Hosek-Wilkie does that itself (0);
    /// Octane's model hardly does — at one power Calib-L-Mode-SunSky-off
    /// (18:00, 15.5°) came out 1.86 × Iray and -noon (40°) 1.08 ×.
    pub daylight_extinction: f64,
    /// The most of a surface's diffuse that Iray translucency may pass on as
    /// Universal's diffuse transmission (Universal): direct lighting renders
    /// a fully transmitting surface black (Calib-SSS's weight-1 sphere).
    pub max_transmission: f32,
    /// DAZ's Blended Dual Lobe Hair on strands (dForce hair): Octane's Hair
    /// material, or the shader's own layers as DAZ's MDL stacks them.
    pub strand_hair: StrandHair,
    /// Octane's model for a rough dielectric Phong lobe (Look): GGX energy
    /// preserving (Iray's level under an environment) or STD (Iray's
    /// highlight shape under a small light). Metals keep their own rule.
    /// STD also moves an Uber skin's dual lobe off OpenPBR's GGX onto
    /// Specular Layers (Look, Skin; #175), and a metal-free node's sharp
    /// dielectric lobes off Beckmann (Look, Gloss; #196).
    pub rough_lobe: RoughLobe,
    /// NVIDIA's textured `mdl 1.0` metals (Metal Brushed, Cast Iron) fall
    /// towards the rim as Iray's legacy lobe makes them (Look): Octane's
    /// flat metal, or Iray's rim as a Falloff Mix on the albedo.
    pub legacy_metal_rim: bool,
    /// The skin's detail contrast s (Look, Skin; #47, #179): the baked
    /// random-walk skin colour × (D / D_low)^s. Only the OpenPBR skin bake
    /// reads it.
    pub skin_detail_contrast: f32,
}

impl Variant {
    pub fn new(look: Look, mode: OctaneMode) -> Self {
        Variant { look, mode }
    }

    /// The table: one full row per variant. Faithful and Preview values not
    /// yet calibrated on their own are Enhanced + Final's (or, for the
    /// kernel-bound ones, the other mode's of the same look) — the spec's
    /// Variants table says which.
    pub fn tuning(self) -> Tuning {
        use Look::*;
        use OctaneMode::*;
        match (self.look, self.mode) {
            (Enhanced, Final) => Tuning {
                kernel: Kernel::PathTracing,
                samples: FINAL_SAMPLES,
                base_exposure: 1.4,
                denoiser: true,
                skin: SkinShader::OpenPbr,
                thin_glass_ior_one: false,
                daylight_model: 3,
                daylight_power: SUN_SKY_POWER,
                daylight_extinction: 0.0,
                max_transmission: 1.0,
                strand_hair: StrandHair::HairPile,
                rough_lobe: ENHANCED_ROUGH_LOBE,
                legacy_metal_rim: ENHANCED_LEGACY_METAL_RIM,
                skin_detail_contrast: ENHANCED_SKIN_DETAIL_CONTRAST,
            },
            (Faithful, Final) => Tuning {
                kernel: Kernel::PathTracing,
                samples: FINAL_SAMPLES,
                base_exposure: 1.4,
                denoiser: true,
                skin: SkinShader::IrayAlbedo,
                thin_glass_ior_one: true,
                daylight_model: 1,
                daylight_power: FAITHFUL_DAYLIGHT_POWER,
                daylight_extinction: FAITHFUL_DAYLIGHT_EXTINCTION,
                max_transmission: 1.0,
                strand_hair: StrandHair::MdlLayers,
                rough_lobe: FAITHFUL_ROUGH_LOBE,
                legacy_metal_rim: FAITHFUL_LEGACY_METAL_RIM,
                skin_detail_contrast: FAITHFUL_SKIN_DETAIL_CONTRAST,
            },
            (Enhanced, Preview) => Tuning {
                kernel: Kernel::DirectLighting,
                samples: PREVIEW_SAMPLES,
                base_exposure: 1.4,
                denoiser: false,
                skin: SkinShader::None,
                thin_glass_ior_one: false,
                daylight_model: 3,
                daylight_power: SUN_SKY_POWER,
                daylight_extinction: 0.0,
                max_transmission: 0.75,
                strand_hair: StrandHair::HairPile,
                rough_lobe: ENHANCED_ROUGH_LOBE,
                legacy_metal_rim: ENHANCED_LEGACY_METAL_RIM,
                skin_detail_contrast: ENHANCED_SKIN_DETAIL_CONTRAST,
            },
            (Faithful, Preview) => Tuning {
                kernel: Kernel::DirectLighting,
                samples: PREVIEW_SAMPLES,
                base_exposure: 1.4,
                denoiser: false,
                skin: SkinShader::None,
                thin_glass_ior_one: true,
                daylight_model: 1,
                daylight_power: FAITHFUL_DAYLIGHT_POWER,
                daylight_extinction: FAITHFUL_DAYLIGHT_EXTINCTION,
                max_transmission: 0.75,
                strand_hair: StrandHair::HairPile,
                rough_lobe: FAITHFUL_ROUGH_LOBE,
                legacy_metal_rim: FAITHFUL_LEGACY_METAL_RIM,
                skin_detail_contrast: FAITHFUL_SKIN_DETAIL_CONTRAST,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhanced_final_is_the_default_export() {
        // ExportOptions defaults to Preview (the library's cheap mode); the
        // CLI and the DAZ dialog default to Final and Enhanced.
        assert_eq!(Look::default(), Look::Enhanced);
        let t = Variant::new(Look::Enhanced, OctaneMode::Final).tuning();
        assert_eq!((t.kernel, t.skin, t.daylight_model), (Kernel::PathTracing, SkinShader::OpenPbr, 3));
    }

    #[test]
    fn skin_detail_contrast_is_enhanced_only() {
        for mode in [OctaneMode::Final, OctaneMode::Preview] {
            assert_eq!(Variant::new(Look::Enhanced, mode).tuning().skin_detail_contrast, ENHANCED_SKIN_DETAIL_CONTRAST);
            assert_eq!(Variant::new(Look::Faithful, mode).tuning().skin_detail_contrast, 0.0);
        }
    }

    #[test]
    fn faithful_final_takes_the_hair_shader_apart() {
        for look in [Look::Enhanced, Look::Faithful] {
            for mode in [OctaneMode::Final, OctaneMode::Preview] {
                let t = Variant::new(look, mode).tuning();
                let layers = (look, mode) == (Look::Faithful, OctaneMode::Final);
                assert_eq!(t.strand_hair == StrandHair::MdlLayers, layers, "{look:?} {mode:?}");
                // Everywhere else Octane's Hair, its albedo the pile's colour.
                assert_eq!(t.strand_hair == StrandHair::HairPile, !layers, "{look:?} {mode:?}");
            }
        }
    }

    #[test]
    fn rough_lobes_follow_the_look() {
        // Preview rows follow their Look (#162).
        for mode in [OctaneMode::Final, OctaneMode::Preview] {
            assert_eq!(Variant::new(Look::Enhanced, mode).tuning().rough_lobe, RoughLobe::GgxEnergyPreserving);
            assert_eq!(Variant::new(Look::Faithful, mode).tuning().rough_lobe, RoughLobe::Std);
        }
    }

    #[test]
    fn only_faithful_draws_the_legacy_metal_rim() {
        // #112: Preview rows follow their Look.
        for mode in [OctaneMode::Final, OctaneMode::Preview] {
            assert!(!Variant::new(Look::Enhanced, mode).tuning().legacy_metal_rim);
            assert!(Variant::new(Look::Faithful, mode).tuning().legacy_metal_rim);
        }
    }

    #[test]
    fn faithful_daylight_dims_a_low_sun() {
        let at = |elevation: f64, look| {
            let e = elevation.to_radians();
            let mut s = crate::usd::SunSky { sun_dir: [e.cos(), e.sin(), 0.0], turbidity: 2.0, power: SUN_SKY_POWER, sun_intensity: 0.55, sun_size: 1.0, ground: [0.4; 3], model: 3, ground_blend_deg: 1.0 };
            s.tune(&Variant::new(look, OctaneMode::Final).tuning());
            (s.model, s.power)
        };
        // The two points it was fitted through: noon (40°) and 18:00 (15.5°).
        let ((model, noon), (_, low)) = (at(40.0, Look::Faithful), at(15.5, Look::Faithful));
        assert_eq!(model, 1);
        assert!((noon - 2.10).abs() < 0.01 && (low - 1.36).abs() < 0.01, "{noon} {low}");
        assert_eq!(at(15.5, Look::Enhanced), (3, SUN_SKY_POWER));
    }

    #[test]
    fn random_walk_skin_only_under_path_tracing() {
        for look in [Look::Enhanced, Look::Faithful] {
            for mode in [OctaneMode::Final, OctaneMode::Preview] {
                let t = Variant::new(look, mode).tuning();
                assert_eq!(t.skin != SkinShader::None, t.kernel == Kernel::PathTracing, "{look:?} {mode:?}");
            }
        }
    }
}
