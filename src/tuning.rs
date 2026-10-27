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

use crate::material::SkinShader;
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
    /// Imager exposure at DAZ's default EV 13 (Render modes): 1 calibrated
    /// on Octane1's lights under direct lighting, 1.4 against Octane2's
    /// Iray render (HDRI only) under path tracing.
    pub base_exposure: f64,
    /// Octane's AI denoiser on the render (Render modes); `--no-denoise`
    /// turns it off in any variant.
    pub denoiser: bool,
    /// Skin with subsurface scattering, and on which material (Look,
    /// Universal): random walk needs path tracing.
    pub skin: SkinShader,
    /// Thin-walled glass at IOR 1 — Iray's showed none of its own
    /// reflection — instead of its Fresnel reflection (Look).
    pub thin_glass_ior_one: bool,
    /// Daylight (Sun-Sky) model: 3 Hosek-Wilkie, which warms a low sun; 1
    /// Octane's, with a white sunset colour like Iray's (Environment).
    pub daylight_model: u8,
    /// Daylight `power` at DAZ's default `SS Multiplier` (Environment):
    /// 2.7 Hosek-Wilkie, 2.27 Octane's model (floor 1.03 ×, shadow 0.96 ×
    /// Iray on Calib-L-Mode-SunSky-off; the white sunset still to fit).
    pub daylight_power: f64,
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
                base_exposure: 1.4,
                denoiser: true,
                skin: SkinShader::OpenPbr,
                thin_glass_ior_one: false,
                daylight_model: 3,
                daylight_power: SUN_SKY_POWER,
            },
            (Faithful, Final) => Tuning {
                kernel: Kernel::PathTracing,
                base_exposure: 1.4,
                denoiser: true,
                skin: SkinShader::StandardSurface,
                thin_glass_ior_one: true,
                daylight_model: 1,
                daylight_power: 2.27,
            },
            (Enhanced, Preview) => Tuning {
                kernel: Kernel::DirectLighting,
                base_exposure: 1.0,
                denoiser: false,
                skin: SkinShader::None,
                thin_glass_ior_one: false,
                daylight_model: 3,
                daylight_power: SUN_SKY_POWER,
            },
            (Faithful, Preview) => Tuning {
                kernel: Kernel::DirectLighting,
                base_exposure: 1.0,
                denoiser: false,
                skin: SkinShader::None,
                thin_glass_ior_one: true,
                daylight_model: 1,
                daylight_power: 2.27,
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
    fn random_walk_skin_only_under_path_tracing() {
        for look in [Look::Enhanced, Look::Faithful] {
            for mode in [OctaneMode::Final, OctaneMode::Preview] {
                let t = Variant::new(look, mode).tuning();
                assert_eq!(t.skin != SkinShader::None, t.kernel == Kernel::PathTracing, "{look:?} {mode:?}");
            }
        }
    }
}
