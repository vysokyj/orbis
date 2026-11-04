//! DAZ's Iray render settings (`crate::iray::RenderOptions`) → Octane's
//! kernel and post-processing (#43; spec, Render settings).

use crate::iray::{Bloom, PixelFilter, RenderOptions};
use crate::tuning::Kernel;

/// Path tracing's bounce limits and GI clamp (Render modes).
const PT_DIFFUSE_DEPTH: u32 = 8;
const PT_GLOSSY_DEPTH: u32 = 24;
const PT_SCATTER_DEPTH: u32 = 128;
const PT_GI_CLAMP: f64 = 20.0;
/// Octane's GI clamp default: no clamp, for Iray's firefly filter off.
const GI_UNCLAMPED: f64 = 1_000_000.0;
/// Direct lighting's limits (Render modes).
const DL_DIFFUSE_DEPTH: u32 = 2;
const DL_GLOSSY_DEPTH: u32 = 2;
const DL_SPECULAR_DEPTH: u32 = 5;

/// Octane's filter size for Iray's default pixel filter (Gauss, radius
/// 1.5): Octane's own default.
const OCTANE_FILTER_SIZE: f64 = 1.2;
/// Octane's filter size range.
const OCTANE_FILTER_SIZE_RANGE: (f64, f64) = (1.0, 8.0);

/// The standard deviation of Iray's pixel filter in pixels, by its width
/// (twice DAZ's radius): a Gauss reaches 3 σ at the radius, a box of
/// width w has σ = w / √12, a triangle w / √24.
pub(super) fn filter_sigma(filter: PixelFilter, width: f64) -> f64 {
    match filter {
        PixelFilter::Gauss => width / 6.0,
        PixelFilter::Box => width / 12f64.sqrt(),
        PixelFilter::Triangle => width / 24f64.sqrt(),
    }
}

/// Octane's `filtersize` for Iray's pixel filter: its σ relative to the
/// default's, on Octane's default.
pub(super) fn octane_filter_size(o: &RenderOptions) -> f64 {
    let d = RenderOptions::default();
    let k = filter_sigma(o.filter, o.filter_width) / filter_sigma(d.filter, d.filter_width);
    (OCTANE_FILTER_SIZE * k).clamp(OCTANE_FILTER_SIZE_RANGE.0, OCTANE_FILTER_SIZE_RANGE.1)
}

/// Octane's depths for Iray's Max Path Length: the same number. Octane's
/// depth counts path segments as Iray does (Calib-RS-PathLen2 at depth 2:
/// glass black, floor 1.04 × Iray; at depth 1 the floor lost half its
/// light; Calib-RS-PathLen4 at depth 4 within 5 % per sphere, at 3 the
/// spheres at half).
fn bounces(o: &RenderOptions) -> u32 {
    o.max_path_length.map_or(u32::MAX, |n| n.max(1))
}

/// The kernel's XML (`assets/octane/kernel_*.ocs`) with the render
/// settings filled in.
pub(super) fn kernel_xml(kernel: Kernel, o: &RenderOptions) -> String {
    let b = bounces(o);
    let int = |n: u32| n.to_string();
    let filter = format!("{:.6}", octane_filter_size(o));
    match kernel {
        Kernel::PathTracing => include_str!("../../assets/octane/kernel_path_tracing.ocs")
            .replace("@DIFFUSE_DEPTH@", &int(PT_DIFFUSE_DEPTH.min(b)))
            .replace("@GLOSSY_DEPTH@", &int(PT_GLOSSY_DEPTH.min(b)))
            .replace("@SCATTER_DEPTH@", &int(o.max_sss_path_length.map_or(PT_SCATTER_DEPTH, |n| PT_SCATTER_DEPTH.min(n.max(1)))))
            .replace("@GI_CLAMP@", &format!("{}", if o.firefly_filter { PT_GI_CLAMP } else { GI_UNCLAMPED }))
            .replace("@FILTER_SIZE@", &filter),
        Kernel::DirectLighting => include_str!("../../assets/octane/kernel_direct_lighting.ocs")
            .replace("@DIFFUSE_DEPTH@", &int(DL_DIFFUSE_DEPTH.min(b)))
            .replace("@GLOSSY_DEPTH@", &int(DL_GLOSSY_DEPTH.min(b)))
            .replace("@SPECULAR_DEPTH@", &int(DL_SPECULAR_DEPTH.min(b)))
            .replace("@FILTER_SIZE@", &filter),
    }
}

/// Octane's post processing (the render target's `postproc`) for Iray's
/// bloom, at the imager's `exposure`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PostProcessing {
    pub on: bool,
    /// Octane blooms pixels brighter than this after the exposure.
    pub cutoff: f64,
    pub bloom_power: f64,
    /// Smallest and largest blur radius, as a share of the larger side.
    pub spread: (f64, f64),
}

/// Octane's smallest bloom spread: about half a pixel at 1920 (Octane's
/// advice; its pin allows ≤ 0.01).
const SPREAD_START: f64 = 0.0001;

/// Iray's bloom → Octane's. Iray's threshold is in its raw pixel values,
/// before the tone mapper (DAZ passes the channel as is; at 1.0 every pixel
/// of Calib-RS-Bloom bloomed, DAZ's default is 2000): ÷ the dome scale
/// (Iray's 10⁴ per unit of Octane's power) × the exposure gives Octane's
/// cutoff. Iray's radius (σ = radius × width / 6 in Calib-RS-Bloom) is the
/// largest spread. Iray replaces what is over the threshold by its blur;
/// Octane adds a glow — the power is Iray's brightness scale, not
/// calibrated (#43).
pub(super) fn post_processing(bloom: Option<&Bloom>, exposure: f64) -> PostProcessing {
    match bloom {
        None => PostProcessing { on: false, cutoff: 0.0, bloom_power: 1.0, spread: (SPREAD_START, 1.0) },
        Some(b) => PostProcessing {
            on: true,
            cutoff: (b.threshold / crate::iray::DOME_INTENSITY_SCALE * exposure).clamp(0.0, 4096.0),
            bloom_power: b.brightness_scale.max(0.0),
            spread: (SPREAD_START, b.radius.clamp(0.001, 1.0)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// Render options from render-settings channels as a scene carries
    /// them (Photoreal's in a child element).
    fn options(channels: &[(&str, Value)]) -> RenderOptions {
        let ch: Vec<Value> = channels.iter().map(|(id, v)| json!({"channel": {"id": id, "current_value": v}})).collect();
        let duf = json!({"scene": {"extra": [{"type": "studio_render_settings", "render_options": {"render_elements": [
            {"id": "NVIDIA Iray Render Options", "channels": [], "children": [{"id": "NVIDIA Iray Photoreal Options", "channels": ch}]}]}}]}});
        crate::iray::render_options(&super::super::render_channels(&duf))
    }

    fn pin(xml: &str, name: &str) -> String {
        let at = xml.find(&format!("<pin name='{name}'>")).unwrap_or_else(|| panic!("{name}"));
        let v = &xml[at..];
        let s = v.find("'>").unwrap() + 2;
        let v = &v[v[s..].find("<attr").unwrap() + s..];
        let v = &v[v.find('>').unwrap() + 1..];
        v[..v.find(' ').unwrap()].to_string()
    }

    #[test]
    fn defaults_keep_the_calibrated_kernels() {
        let d = RenderOptions::default();
        let pt = kernel_xml(Kernel::PathTracing, &d);
        assert_eq!([pin(&pt, "maxDiffuseDepth"), pin(&pt, "maxGlossyDepth"), pin(&pt, "maxScatterDepth"), pin(&pt, "giClamp")], ["8", "24", "128", "20"]);
        assert_eq!(pin(&pt, "filtersize"), "1.200000");
        let dl = kernel_xml(Kernel::DirectLighting, &d);
        assert_eq!([pin(&dl, "diffusedepth"), pin(&dl, "glossydepth"), pin(&dl, "speculardepth"), pin(&dl, "filtersize")], ["2", "2", "5", "1.200000"]);
        assert!(!pt.contains('@') && !dl.contains('@'));
        assert!(!post_processing(d.bloom.as_ref(), 1.4).on);
    }

    #[test]
    fn max_path_length_caps_every_bounce_kind() {
        // 2 segments = direct light: Octane's depth 2.
        let o = options(&[("Max Path Length", json!(2))]);
        assert_eq!(o.max_path_length, Some(2));
        let pt = kernel_xml(Kernel::PathTracing, &o);
        assert_eq!([pin(&pt, "maxDiffuseDepth"), pin(&pt, "maxGlossyDepth")], ["2", "2"]);
        let pt = kernel_xml(Kernel::PathTracing, &options(&[("Max Path Length", json!(4))]));
        assert_eq!([pin(&pt, "maxDiffuseDepth"), pin(&pt, "maxGlossyDepth")], ["4", "4"]);
        let dl = kernel_xml(Kernel::DirectLighting, &options(&[("Max Path Length", json!(4))]));
        assert_eq!([pin(&dl, "diffusedepth"), pin(&dl, "glossydepth"), pin(&dl, "speculardepth")], ["2", "2", "4"]);
        // −1: unbounded.
        assert_eq!(options(&[("Max Path Length", json!(-1))]).max_path_length, None);
        let pt = kernel_xml(Kernel::PathTracing, &options(&[("Max SSS Path Length", json!(16))]));
        assert_eq!(pin(&pt, "maxScatterDepth"), "16");
    }

    #[test]
    fn firefly_filter_off_unclamps_gi() {
        let pt = kernel_xml(Kernel::PathTracing, &options(&[("Firefly Filter Enable", json!(false))]));
        assert_eq!(pin(&pt, "giClamp"), "1000000");
    }

    #[test]
    fn pixel_filter_scales_octanes_filter_size_by_sigma() {
        let d = RenderOptions::default();
        assert_eq!((d.filter, d.filter_width), (PixelFilter::Gauss, 3.0));
        let size = |ch: &[(&str, Value)]| octane_filter_size(&options(ch));
        assert!((size(&[("Pixel Filter Radius", json!(3.0))]) - 2.0 * OCTANE_FILTER_SIZE).abs() < 1e-9);
        // A box of radius 1.5 (width 3): σ 0.87 against the Gauss's 0.5.
        assert!((size(&[("Pixel Filter", json!(0))]) - OCTANE_FILTER_SIZE * 3f64.sqrt()).abs() < 1e-9);
        assert!(size(&[("Pixel Filter Radius", json!(0.1))]) >= OCTANE_FILTER_SIZE_RANGE.0);
    }

    #[test]
    fn bloom_turns_on_octanes_post_processing() {
        let o = options(&[("Bloom Filter Enable", json!(true))]);
        let p = post_processing(o.bloom.as_ref(), 1.4);
        // DAZ's default threshold 2000 at EV 13: 0.2 of the dome's unit, exposed.
        assert!(p.on && (p.cutoff - 0.28).abs() < 1e-9, "{p:?}");
        assert_eq!((p.bloom_power, p.spread), (1.0, (SPREAD_START, 0.05)));
    }
}
