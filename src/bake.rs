//! Per-texel baking helpers: exact lookup tables for the 8-bit decode and
//! encode, and a parallel row loop (#78).
//!
//! The textures the export bakes (skin colour, roughness, blends) were
//! computed with `powf` per channel per texel, single-threaded: three
//! minutes of a five-minute export on a multi-figure scene. The tables here
//! give the same bytes as those expressions, not approximations: the decode
//! table holds `colour_to_linear(c / 255)` for each byte, and the encoder
//! finds, for each output byte, the smallest `f32` the original
//! `(v.clamp(0, 1).powf(1 / gamma) · 255 + 0.5) as u8` maps to it, so a
//! lookup is a binary search over 255 thresholds.

use std::sync::{Mutex, OnceLock};

/// `colour_to_linear(c / 255)` for each byte `c` (DAZ's 2.2 decode).
pub fn decode_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| std::array::from_fn(|c| decode_ref(c as u8)))
}

/// The decode the tables reproduce.
pub fn decode_ref(c: u8) -> f32 {
    crate::daz_material::colour_to_linear(f32::from(c) / 255.0)
}

/// The encode the encoder reproduces: linear → 8 bits at `gamma`.
pub fn encode_ref(v: f32, gamma: f32) -> u8 {
    (v.clamp(0.0, 1.0).powf(1.0 / gamma) * 255.0 + 0.5) as u8
}

/// [`encode_ref`] at one gamma as a table lookup.
#[derive(Clone)]
pub struct Encoder {
    /// `thresholds[n - 1]`: the smallest value that encodes to `n` or more.
    thresholds: [f32; 255],
}

impl Encoder {
    pub fn new(gamma: f32) -> Self {
        // Non-negative floats order as their bit patterns: bisect on those.
        let thresholds = std::array::from_fn(|i| {
            let n = i as u8 + 1;
            let (mut lo, mut hi) = (0u32, 1.0f32.to_bits());
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if encode_ref(f32::from_bits(mid), gamma) >= n {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            f32::from_bits(lo)
        });
        Self { thresholds }
    }

    /// The gamma-2.2 encoder, built once.
    pub fn srgb() -> &'static Self {
        static ENC: OnceLock<Encoder> = OnceLock::new();
        ENC.get_or_init(|| Self::new(2.2))
    }

    /// The linear (gamma 1) encoder, built once.
    pub fn linear() -> &'static Self {
        static ENC: OnceLock<Encoder> = OnceLock::new();
        ENC.get_or_init(|| Self::new(1.0))
    }

    /// The encoder for `gamma`: the shared ones for 2.2 and 1.
    pub fn for_gamma(gamma: f32) -> std::borrow::Cow<'static, Self> {
        if gamma == 2.2 {
            std::borrow::Cow::Borrowed(Self::srgb())
        } else if gamma == 1.0 {
            std::borrow::Cow::Borrowed(Self::linear())
        } else {
            std::borrow::Cow::Owned(Self::new(gamma))
        }
    }

    /// `encode_ref(v, gamma)`; NaN encodes to 0, as `as u8` casts it.
    #[inline]
    pub fn encode(&self, v: f32) -> u8 {
        self.thresholds.partition_point(|&t| t <= v) as u8
    }
}

/// An RGB image `w` × `h` whose rows `row(y, pixels)` fills, rows spread
/// over the machine's cores in blocks (taken as threads free up, so slower
/// efficiency cores don't hold the rest back).
pub fn par_rgb(w: u32, h: u32, row: impl Fn(u32, &mut [u8]) + Sync) -> image::RgbImage {
    const BLOCK: usize = 16;
    let stride = w as usize * 3;
    let mut buf = vec![0u8; stride * h as usize];
    if stride > 0 {
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(h as usize / BLOCK + 1);
        let blocks = Mutex::new(buf.chunks_mut(stride * BLOCK).enumerate());
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| loop {
                    let Some((b, chunk)) = blocks.lock().unwrap_or_else(|e| e.into_inner()).next() else { break };
                    for (r, px) in chunk.chunks_mut(stride).enumerate() {
                        row((b * BLOCK + r) as u32, px);
                    }
                });
            }
        });
    }
    image::RgbImage::from_raw(w, h, buf).expect("buffer sized to the image")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_table_matches() {
        for c in 0..=255u8 {
            assert_eq!(decode_lut()[c as usize].to_bits(), decode_ref(c).to_bits());
        }
    }

    /// The encoder against `encode_ref` around every threshold, on a dense
    /// sweep of [0, 1] and outside it.
    #[test]
    fn encoder_matches() {
        for gamma in [2.2f32, 1.0, 1.8] {
            let enc = Encoder::new(gamma);
            let check = |v: f32| assert_eq!(enc.encode(v), encode_ref(v, gamma), "gamma {gamma}, v {v:e}");
            for t in enc.thresholds {
                let b = t.to_bits();
                for d in 0..8u32 {
                    check(f32::from_bits(b + d));
                    check(f32::from_bits(b.saturating_sub(d)));
                }
            }
            (0..=1_000_000).for_each(|i| check(i as f32 / 1_000_000.0));
            [-1.0, -0.0, 0.0, 1.0, 1.5, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, f32::MIN_POSITIVE, 1e-30].into_iter().for_each(check);
        }
    }

    /// Every `f32` in [0, 1] (about 10⁹): run by hand in a release build
    /// (`cargo test --release -- --ignored encoder_exhaustive`) when the
    /// platform's `powf` changes — the thresholds assume it is monotonic.
    #[test]
    #[ignore]
    fn encoder_exhaustive() {
        for gamma in [2.2f32, 1.0] {
            let enc = Encoder::new(gamma);
            for b in 0..=1.0f32.to_bits() {
                let v = f32::from_bits(b);
                assert_eq!(enc.encode(v), encode_ref(v, gamma), "gamma {gamma}, v {v:e}");
            }
        }
    }

    #[test]
    fn parallel_rows_fill_every_row() {
        for (w, h) in [(0, 0), (1, 1), (5, 17), (64, 100), (3, 33)] {
            let img = par_rgb(w, h, |y, px| px.chunks_mut(3).enumerate().for_each(|(x, p)| p.copy_from_slice(&[x as u8, y as u8, 7])));
            assert_eq!(img.dimensions(), (w, h));
            assert!(img.enumerate_pixels().all(|(x, y, p)| p.0 == [x as u8, y as u8, 7]));
        }
    }
}
