//! Texture memory budget for the Octane export.
//!
//! Renderers hold textures decoded: a 4096² RGBA map is 64 MB however small
//! its JPEG. A scene with ~400 such maps (yoga2: 17.8 GB decoded) does not
//! fit Octane X on a 24 GB Apple-silicon Mac and fails to render. When the
//! decoded total exceeds the budget, every texture is scaled down to the
//! largest power-of-two edge that fits, re-encoded in its own format.

use std::io::Cursor;

use anyhow::{anyhow, Result};
use image::{imageops::FilterType, DynamicImage, ImageFormat, ImageReader};

/// Decoded textures above this (RGBA8) get scaled down.
pub const BUDGET_BYTES: u64 = 8 << 30;

/// Longest edges tried, largest first.
const LIMITS: [u32; 3] = [2048, 1024, 512];

/// Width and height from the image header (no decoding); `None` for formats
/// the build can't read (HDR, EXR — left as they are).
pub fn dimensions(data: &[u8]) -> Option<(u32, u32)> {
    ImageReader::new(Cursor::new(data)).with_guessed_format().ok()?.into_dimensions().ok()
}

/// A texture file's size in pixels, from its header alone.
pub fn file_dimensions(path: &std::path::Path) -> Option<(u32, u32)> {
    ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_dimensions().ok()
}

fn decoded_bytes((w, h): (u32, u32), limit: Option<u32>) -> u64 {
    let scale = limit.map(|l| (l as f64 / w.max(h) as f64).min(1.0)).unwrap_or(1.0);
    let (w, h) = ((w as f64 * scale).round() as u64, (h as f64 * scale).round() as u64);
    w * h * 4
}

/// The longest-edge limit that keeps the decoded total within `budget`:
/// `None` when everything fits at full size, else the largest of 2048 / 1024
/// / 512 that fits (512 if none does).
pub fn choose_limit(dims: &[(u32, u32)], budget: u64) -> Option<u32> {
    let total = |limit| dims.iter().map(|d| decoded_bytes(*d, limit)).sum::<u64>();
    if total(None) <= budget {
        return None;
    }
    Some(LIMITS.into_iter().find(|l| total(Some(*l)) <= budget).unwrap_or(LIMITS[LIMITS.len() - 1]))
}

/// `data` scaled so its longest edge is at most `limit`, re-encoded in its own
/// format (JPEG at quality 90, PNG, TIFF); `None` when it already fits or
/// can't be read.
pub fn downscale(data: &[u8], limit: u32) -> Result<Option<Vec<u8>>> {
    let Some((w, h)) = dimensions(data) else { return Ok(None) };
    if w.max(h) <= limit {
        return Ok(None);
    }
    let format = image::guess_format(data)?;
    let img = image::load_from_memory_with_format(data, format)?;
    let scale = limit as f64 / w.max(h) as f64;
    let (nw, nh) = (((w as f64 * scale).round() as u32).max(1), ((h as f64 * scale).round() as u32).max(1));
    let img = img.resize_exact(nw, nh, FilterType::Triangle);
    let mut out = Vec::new();
    match format {
        ImageFormat::Jpeg => {
            let img = match img {
                DynamicImage::ImageLuma8(_) | DynamicImage::ImageLumaA8(_) => DynamicImage::ImageLuma8(img.to_luma8()),
                _ => DynamicImage::ImageRgb8(img.to_rgb8()),
            };
            img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90))?;
        }
        ImageFormat::Png => img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)?,
        // DAZ normal maps often come as 8- or 16-bit TIFF.
        ImageFormat::Tiff => img.write_to(&mut Cursor::new(&mut out), ImageFormat::Tiff)?,
        other => return Err(anyhow!("can't re-encode {other:?}")),
    }
    Ok(Some(out))
}

/// The longest-edge limit (as `choose_limit`'s) that keeps textures of
/// `items` (bytes as packed, pixel size) within `cap` bytes, assuming a
/// file shrinks with its pixel count: `None` when they fit as they are.
/// A `.usdz` is a plain zip, under 4 GiB (#16, #199).
pub fn choose_limit_for_bytes(items: &[(u64, (u32, u32))], cap: u64) -> Option<u32> {
    let total = |limit: Option<u32>| -> u64 {
        items.iter().map(|(bytes, (w, h))| {
            let s = limit.map_or(1.0, |l| (l as f64 / (*w).max(*h).max(1) as f64).min(1.0));
            (*bytes as f64 * s * s) as u64
        }).sum()
    };
    if total(None) <= cap {
        return None;
    }
    Some(LIMITS.into_iter().find(|l| total(Some(*l)) <= cap).unwrap_or(LIMITS[LIMITS.len() - 1]))
}

/// A texture as a `.usdz` packs it: PNG and JPEG as they are (scaled to
/// `limit` in their own format), anything else (TIFF, BMP, TGA: not a
/// USDZ image type) decoded and written as a PNG of the same depth,
/// scaled on the way. Returns the bytes and whether it was scaled.
pub fn usdz_texture(data: &[u8], limit: Option<u32>, to_png: bool) -> Result<(Vec<u8>, bool)> {
    if !to_png {
        return Ok(match limit.map(|l| downscale(data, l)) {
            Some(Ok(Some(small))) => (small, true),
            _ => (data.to_vec(), false),
        });
    }
    let mut img = image::load_from_memory(data)?;
    let (w, h) = (img.width(), img.height());
    let scaled = limit.is_some_and(|l| w.max(h) > l);
    if let Some(l) = limit.filter(|_| scaled) {
        let s = l as f64 / w.max(h) as f64;
        img = img.resize_exact(((w as f64 * s).round() as u32).max(1), ((h as f64 * s).round() as u32).max(1), FilterType::Triangle);
    }
    // PNG takes 8 and 16 bits per channel, not floats.
    let img = match img {
        DynamicImage::ImageRgb32F(_) => DynamicImage::ImageRgb16(img.to_rgb16()),
        DynamicImage::ImageRgba32F(_) => DynamicImage::ImageRgba16(img.to_rgba16()),
        other => other,
    };
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)?;
    Ok((out, scaled))
}

/// `f` over `items` on a thread each, results in `items`' order.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    std::thread::scope(|s| {
        let handles: Vec<_> = items.iter().map(|i| s.spawn(|| f(i))).collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
    })
}

/// Scale every texture in `files` (in place) to `limit`, in parallel. Returns
/// how many were scaled; unreadable ones are left as they are (logged).
pub fn downscale_all(files: &mut [(String, Vec<u8>)], limit: u32) -> usize {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = files.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = files
            .chunks_mut(chunk)
            .map(|part| {
                s.spawn(move || {
                    let mut n = 0;
                    for (name, data) in part {
                        match downscale(data, limit) {
                            Ok(Some(small)) => {
                                *data = small;
                                n += 1;
                            }
                            Ok(None) => {}
                            Err(e) => tracing::warn!(texture = %name, "Octane export: texture not scaled: {e:#}"),
                        }
                    }
                    n
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or(0)).sum()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, RgbImage};

    #[test]
    fn limit_only_when_over_budget() {
        let maps = vec![(4096, 4096); 4]; // 256 MB decoded
        assert_eq!(choose_limit(&maps, 1 << 30), None);
        assert_eq!(choose_limit(&maps, 100 << 20), Some(2048), "4 × 16 MB fits");
        assert_eq!(choose_limit(&maps, 20 << 20), Some(1024), "4 × 4 MB fits");
        assert_eq!(choose_limit(&maps, 1), Some(512), "smallest when nothing fits");
    }

    #[test]
    fn usdz_limit_by_packed_bytes() {
        let maps = vec![(64 << 20, (4096, 4096)); 4];
        assert_eq!(choose_limit_for_bytes(&maps, 1 << 30), None);
        assert_eq!(choose_limit_for_bytes(&maps, 64 << 20), Some(2048), "a quarter each");
    }

    #[test]
    fn usdz_texture_turns_a_tiff_into_png() {
        let mut tif = Vec::new();
        DynamicImage::ImageRgb16(image::ImageBuffer::from_pixel(40, 20, image::Rgb([1000u16, 2000, 65535]))).write_to(&mut Cursor::new(&mut tif), ImageFormat::Tiff).unwrap();
        let (png, scaled) = usdz_texture(&tif, Some(10), true).unwrap();
        assert!(scaled);
        assert_eq!(image::guess_format(&png).unwrap(), ImageFormat::Png);
        let back = image::load_from_memory(&png).unwrap();
        assert_eq!((back.width(), back.height()), (10, 5));
        assert!(matches!(back, DynamicImage::ImageRgb16(_)), "16 bits kept");
        let (same, scaled) = usdz_texture(&png, None, false).unwrap();
        assert!(!scaled && same == png);
    }

    #[test]
    fn downscale_keeps_format_and_aspect() {
        let mut jpg = Vec::new();
        DynamicImage::ImageRgb8(RgbImage::from_pixel(400, 200, image::Rgb([200, 100, 50])))
            .write_to(&mut Cursor::new(&mut jpg), ImageFormat::Jpeg)
            .unwrap();
        let small = downscale(&jpg, 100).unwrap().unwrap();
        assert_eq!(image::guess_format(&small).unwrap(), ImageFormat::Jpeg);
        assert_eq!(dimensions(&small), Some((100, 50)));
        assert!(downscale(&small, 100).unwrap().is_none(), "already fits");

        let mut png = Vec::new();
        DynamicImage::ImageLuma8(GrayImage::from_pixel(64, 64, image::Luma([7])))
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        let mut files = vec![("a.png".to_string(), png), ("b.tif".to_string(), b"not an image".to_vec())];
        assert_eq!(downscale_all(&mut files, 32), 1);
        assert_eq!(dimensions(&files[0].1), Some((32, 32)));
        assert_eq!(files[1].1, b"not an image");
    }
}
