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

/// Octane's daylight model as Faithful lights with it: at 2.27 the 18:00
/// sun (Calib-L-Mode-SunSky-off, 15.5°) came out 1.86 × Iray, noon (40°)
/// 1.08 ×. The floor doesn't follow the power linearly (18:00 at power
/// 1.22 / 1.32 / 1.34 / 1.39: 0.85 / 0.92 / 0.94 / 1.12 ×), so P · exp(−τ /
/// sin e) went through noon at 2.10 and 18:00 at 1.36: 18:00 0.95, noon
/// 1.00, Calib-L-Mode-SunSky-dir (43°) 0.98 × Iray.
const FAITHFUL_DAYLIGHT_POWER: f64 = 2.864;
const FAITHFUL_DAYLIGHT_EXTINCTION: f64 = 0.2001;

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
    /// reflection — instead of its Fresnel reflection (Look).
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
                daylight_extinction: 0.0,
                max_transmission: 1.0,
            },
            (Faithful, Final) => Tuning {
                kernel: Kernel::PathTracing,
                base_exposure: 1.4,
                denoiser: true,
                skin: SkinShader::StandardSurface,
                thin_glass_ior_one: true,
                daylight_model: 1,
                daylight_power: FAITHFUL_DAYLIGHT_POWER,
                daylight_extinction: FAITHFUL_DAYLIGHT_EXTINCTION,
                max_transmission: 1.0,
            },
            (Enhanced, Preview) => Tuning {
                kernel: Kernel::DirectLighting,
                base_exposure: 1.4,
                denoiser: false,
                skin: SkinShader::None,
                thin_glass_ior_one: false,
                daylight_model: 3,
                daylight_power: SUN_SKY_POWER,
                daylight_extinction: 0.0,
                max_transmission: 0.75,
            },
            (Faithful, Preview) => Tuning {
                kernel: Kernel::DirectLighting,
                base_exposure: 1.4,
                denoiser: false,
                skin: SkinShader::None,
                thin_glass_ior_one: true,
                daylight_model: 1,
                daylight_power: FAITHFUL_DAYLIGHT_POWER,
                daylight_extinction: FAITHFUL_DAYLIGHT_EXTINCTION,
                max_transmission: 0.75,
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
    fn faithful_daylight_dims_a_low_sun() {
        let at = |elevation: f64, look| {
            let e = elevation.to_radians();
            let mut s = crate::usd::SunSky { sun_dir: [e.cos(), e.sin(), 0.0], turbidity: 2.0, power: SUN_SKY_POWER, sun_intensity: 0.55, sun_size: 1.0, ground: [0.4; 3], model: 3 };
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
