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
