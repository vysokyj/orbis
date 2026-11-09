//! Iray Uber's surface detail as one tangent-space normal map (#85).
//!
//! DAZ hands Iray a height map through MDL's `base::file_bump_texture`
//! (MDL SDK `base.mdl`): the slope is the height difference of the texels
//! one either side, `h[i+1] − h[i−1]` (smootherstep-interpolated between
//! texels), applied on the unit texture tangents by `apply_bump`:
//! `n = normalize(−factor · du · t_u − factor · dv · t_v + n)`. That is a
//! per-texel tangent-space normal, so a normal map holds it exactly.
//!
//! Octane's own bump can't: it reads the height map per texel without
//! interpolation and takes its slope as a finite difference about a pixel
//! wide (`octane-probe-bumpfreq.lua`: the normal is flat inside a texel and
//! tilts in a one-pixel line at each texel edge). Its normals scatter where
//! Iray's stay smooth, which widened the skin's highlights (#85). Octane
//! interpolates a normal map bilinearly and reads it as Iray does at
//! strength 1, so the height maps go in baked.
//!
//! A normal map and a height map together are DAZ's base and detail bump,
//! combined by `base::blend_normals` (reoriented normal mapping; DAZ's
//! `add_detail_normal`).
//!
//! Each map is decoded at the gamma Iray reads it with (#91): the image's
//! own `map_gamma`, else gamma "auto" — Iray's loader gives the canvas its
//! pixel type's gamma, 2.2 for 8 bits, 1.0 for 16 bits or float
//! ([`auto_gamma`]). Calib-BumpGamma: an 8-bit sawtooth at `map_gamma` 0
//! tilted the normal as its 2.2-decoded heights' slope, at `map_gamma` 1
//! and as a 16-bit PNG as the linear slope.

/// One input: a height map read as MDL's `file_bump_texture` at `factor`,
/// or a tangent-space normal map (`tangent_space_normal_texture`, factor 1);
/// its texels decoded at `gamma`.
pub enum Source<'a> {
    Height { image: &'a image::DynamicImage, factor: f32, gamma: f32 },
    Normal { image: &'a image::DynamicImage, gamma: f32 },
}

impl Source<'_> {
    fn dims(&self) -> (u32, u32) {
        match self {
            Source::Height { image, .. } | Source::Normal { image, .. } => (image.width(), image.height()),
        }
    }
}

/// The gamma Iray's image loader gives a canvas under gamma "auto" (MDL
/// `gamma_default`) by its pixel type, ignoring the file's colour
/// metadata (#90): 8 bits a channel 2.2, deeper 1.0. Calib-BumpGamma's
/// 16-bit greyscale height map rendered linear (#91); a 16-bit RGB map is
/// taken the same way (the loader's type rule), unverified.
pub fn auto_gamma(color: image::ColorType) -> f32 {
    if color.bytes_per_pixel() as u16 / color.channel_count() as u16 <= 1 { 2.2 } else { 1.0 }
}

/// A tangent-space normal field at `w` × `h` texels, row by row from the
/// image's top.
struct Field {
    w: usize,
    n: Vec<[f32; 3]>,
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 { v.map(|c| c / l) } else { [0.0, 0.0, 1.0] }
}

/// Texels as Iray reads them at `gamma` (a pure power, as the colour maps).
fn decode(mut rgb: image::Rgb32FImage, gamma: f32) -> image::Rgb32FImage {
    if gamma != 1.0 {
        rgb.pixels_mut().for_each(|p| p.0 = p.0.map(|c| c.max(0.0).powf(gamma)));
    }
    rgb
}

fn field(src: &Source, w: u32, h: u32) -> Field {
    let resize = |img: &image::DynamicImage| {
        let rgb = img.to_rgb32f();
        if rgb.dimensions() == (w, h) { rgb } else { image::imageops::resize(&rgb, w, h, image::imageops::FilterType::Triangle) }
    };
    let (wu, hu) = (w as usize, h as usize);
    match src {
        Source::Normal { image, gamma } => {
            // MDL: (texel − 0.5) · 2, then apply_bump with the normal's
            // weight |z|.
            let rgb = decode(resize(image), *gamma);
            let n = rgb.pixels().map(|p| normalize([2.0 * p[0] - 1.0, 2.0 * p[1] - 1.0, (2.0 * p[2] - 1.0).abs()])).collect();
            Field { w: wu, n }
        }
        Source::Height { image, factor, gamma } => {
            // `mono_average` of the decoded texels.
            let (sw, sh) = (image.width(), image.height());
            let rgb = decode(resize(image), *gamma);
            let ht: Vec<f32> = rgb.pixels().map(|p| (p[0] + p[1] + p[2]) / 3.0).collect();
            // The slope per texel of the map itself: a map resampled to a
            // finer grid keeps its slope per its own texel.
            let (rx, ry) = (w as f32 / sw.max(1) as f32, h as f32 / sh.max(1) as f32);
            let at = |x: usize, y: usize| ht[y * wu + x];
            let mut n = Vec::with_capacity(wu * hu);
            for y in 0..hu {
                // v runs up the image: the texel above is v + 1.
                let (up, down) = ((y + hu - 1) % hu, (y + 1) % hu);
                for x in 0..wu {
                    let (l, r) = ((x + wu - 1) % wu, (x + 1) % wu);
                    let du = rx * (at(r, y) - at(l, y));
                    let dv = ry * (at(x, up) - at(x, down));
                    n.push(normalize([-factor * du, -factor * dv, 1.0]));
                }
            }
            Field { w: wu, n }
        }
    }
}

/// MDL's `base::blend_normals` in tangent space, both weights 1.
pub fn blend_normals(base: [f32; 3], detail: [f32; 3]) -> [f32; 3] {
    let n1 = [base[0], base[1], base[2] + 1.0];
    let n2 = [-detail[0], -detail[1], detail[2]];
    let d = n1[0] * n2[0] + n1[1] * n2[1] + n1[2] * n2[2];
    normalize([0, 1, 2].map(|k| n1[k] * d / n1[2] - n2[k]))
}

/// The tangent-space normal map of `base` with `detail` blended onto it,
/// at the larger input's size, 8 bits per channel ((n + 1) / 2, linear).
pub fn bake(base: &Source, detail: Option<&Source>) -> image::RgbImage {
    let dims = |s: &Source| s.dims();
    let (w, h) = detail.map_or(dims(base), |d| {
        let (a, b) = (dims(base), dims(d));
        if (b.0 as u64 * b.1 as u64) > (a.0 as u64 * a.1 as u64) { b } else { a }
    });
    let fb = field(base, w, h);
    let fd = detail.map(|d| field(d, w, h));
    let enc = |c: f32| ((c * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    crate::bake::par_rgb(w, h, |y, px| {
        let row = y as usize * fb.w;
        for (x, o) in px.chunks_exact_mut(3).enumerate() {
            let nb = fb.n[row + x];
            let n = fd.as_ref().map_or(nb, |d| blend_normals(nb, d.n[row + x]));
            o.copy_from_slice(&n.map(enc));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(w: u32, h: u32, f: impl Fn(u32, u32) -> f32) -> image::DynamicImage {
        image::DynamicImage::ImageLuma16(image::ImageBuffer::from_fn(w, h, |x, y| image::Luma([(f(x, y) * 65535.0).round() as u16])))
    }

    fn decode(p: &image::Rgb<u8>) -> [f32; 3] {
        p.0.map(|c| c as f32 / 255.0 * 2.0 - 1.0)
    }

    #[test]
    fn slope_is_mdls_central_difference() {
        // A ramp of 0.01 per texel along u: MDL's du = 0.02, at factor 2
        // the normal leans to −0.04 along u.
        let img = grey(64, 8, |x, _| 0.2 + 0.01 * (x as f32));
        let out = bake(&Source::Height { image: &img, factor: 2.0, gamma: 1.0 }, None);
        let n = decode(out.get_pixel(20, 4));
        let want = normalize([-0.04, 0.0, 1.0]);
        assert!((n[0] - want[0]).abs() < 0.01 && n[1].abs() < 0.01, "{n:?}");
    }

    #[test]
    fn v_runs_up_the_image() {
        // Height rising towards the image's top = rising with v: the normal
        // leans to −v (green below 0.5).
        let img = grey(8, 64, |_, y| 0.8 - 0.01 * (y as f32));
        let out = bake(&Source::Height { image: &img, factor: 4.0, gamma: 1.0 }, None);
        let n = decode(out.get_pixel(4, 30));
        assert!(n[1] < -0.05 && n[0].abs() < 0.01, "{n:?}");
    }

    #[test]
    fn nyquist_pattern_is_flat() {
        // Texels alternating 0 / 1: the central difference sees none of it
        // (Octane's own bump tilts hardest there).
        let img = grey(32, 4, |x, _| (x % 2) as f32);
        let out = bake(&Source::Height { image: &img, factor: 4.0, gamma: 1.0 }, None);
        assert!(out.pixels().all(|p| p.0 == [128, 128, 255]), "{:?}", out.get_pixel(3, 1));
    }

    #[test]
    fn coarser_map_keeps_its_slope() {
        // The same ramp at half the resolution blended onto a flat normal
        // map twice its size: the slope per the height map's own texel.
        let img = grey(32, 4, |x, _| 0.2 + 0.02 * (x as f32));
        let flat = image::DynamicImage::ImageRgb8(image::ImageBuffer::from_pixel(64, 8, image::Rgb([128, 128, 255])));
        let out = bake(&Source::Normal { image: &flat, gamma: 1.0 }, Some(&Source::Height { image: &img, factor: 1.0, gamma: 1.0 }));
        assert_eq!(out.dimensions(), (64, 8));
        let n = decode(out.get_pixel(30, 4));
        assert!((n[0] + 0.04).abs() < 0.012, "{n:?}");
    }

    #[test]
    fn auto_gamma_decodes_8_bit_heights_at_2_2() {
        // Calib-BumpGamma's sawtooth, 8 levels a texel: at gamma 2.2 the
        // slope at texel j is (8 (j + 1) / 255)^2.2 − (8 (j − 1) / 255)^2.2.
        let img = image::DynamicImage::ImageLuma8(image::ImageBuffer::from_fn(4, 32, |_, y| image::Luma([(8 * (31 - y)) as u8])));
        assert_eq!(auto_gamma(img.color()), 2.2);
        assert_eq!(auto_gamma(image::ColorType::L16), 1.0);
        assert_eq!(auto_gamma(image::ColorType::Rgb32F), 1.0);
        let out = bake(&Source::Height { image: &img, factor: 4.0, gamma: 2.2 }, None);
        for j in [5u32, 25] {
            let d = |k: u32| (8.0 * k as f32 / 255.0).powf(2.2);
            let want = normalize([0.0, -4.0 * (d(j + 1) - d(j - 1)), 1.0]);
            let n = decode(out.get_pixel(1, 31 - j));
            assert!((n[1] - want[1]).abs() < 0.01, "j {j}: {n:?} vs {want:?}");
        }
    }

    #[test]
    fn blend_onto_flat_base_is_the_detail() {
        let d = normalize([0.3, -0.2, 1.0]);
        let n = blend_normals([0.0, 0.0, 1.0], d);
        assert!((0..3).all(|k| (n[k] - d[k]).abs() < 1e-5), "{n:?}");
        let b = normalize([0.2, 0.1, 1.0]);
        assert!((0..3).all(|k| (blend_normals(b, [0.0, 0.0, 1.0])[k] - b[k]).abs() < 1e-5));
    }
}
