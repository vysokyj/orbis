//! ORBX container — Octane's native scene package.
//!
//! OTOY publishes no specification; the layout was reverse-engineered from the
//! samples bundled with Octane X and is documented in `docs/octane/orbx.md`.
//! [`write_orbx`] reproduces all of them byte for byte.

use std::io::Write;

use anyhow::{anyhow, bail, Context, Result};

/// Files are stored in chunks of at most this size (the scene, `project.ocs`,
/// always as one chunk).
const CHUNK: usize = 1 << 20;

/// Footer magic after each index copy.
const MAGIC: [u8; 4] = [0x04, 0x11, 0x5a, 0x9f];

/// Unsigned length (blob / string length): the full payload width of each form.
fn enc_u(v: u64) -> Result<Vec<u8>> {
    Ok(match v {
        _ if v < 1 << 7 => vec![v as u8],
        _ if v < 1 << 14 => vec![0x80 | (v >> 8) as u8, v as u8],
        _ if v < 1 << 21 => vec![0xc0 | (v >> 16) as u8, (v >> 8) as u8, v as u8],
        _ if v < 1 << 28 => vec![0xe0 | (v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8],
        // Not seen in the samples: assumed to continue the UTF-8-like pattern.
        _ if v < 1 << 35 => vec![0xf0 | (v >> 32) as u8, (v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8],
        _ => bail!("orbx: length {v} too large"),
    })
}

/// Signed integer (every `x3`/`xb` tag in the index): each form keeps a sign bit.
fn enc_s(v: u64) -> Result<Vec<u8>> {
    Ok(match v {
        _ if v < 1 << 6 => vec![v as u8],
        _ if v < 1 << 13 => vec![0x80 | (v >> 8) as u8, v as u8],
        _ if v < 1 << 20 => vec![0xc0 | (v >> 16) as u8, (v >> 8) as u8, v as u8],
        _ if v < 1 << 27 => vec![0xe0 | (v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8],
        // Not seen in the samples (a package over ~128 MiB): assumed form.
        _ if v < 1 << 34 => vec![0xf0 | (v >> 32) as u8, (v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8],
        _ => bail!("orbx: integer {v} too large"),
    })
}

/// Write an ORBX package. `files` are `(path, contents)` in write order —
/// assets (`assets/…`) first, the scene `project.ocs` last, as Octane does.
/// `version` is the Octane version as decimal text (e.g. `15050000`).
pub fn write_orbx(out: &mut impl Write, version: &str, files: &[(&str, &[u8])]) -> Result<()> {
    let mut w = OrbxWriter::new(out, version)?;
    for (path, data) in files {
        w.add(path, data)?;
    }
    w.finish()
}

/// An ORBX package written file by file, each file streamed in: chunks go
/// out as they fill, the index (chunk sizes, files by path) at the end — a
/// big scene's assets never have to be in memory at once. Its `Write`
/// implementation feeds the file begun last.
pub struct OrbxWriter<'a, W: Write> {
    out: &'a mut W,
    /// Every chunk's size, in write order.
    chunks: Vec<u64>,
    /// Finished files: path, first chunk, length.
    files: Vec<(String, usize, u64)>,
    /// The file being written: path, first chunk, length so far, the
    /// unwritten tail of its data, whether it goes out as one chunk.
    current: Option<(String, usize, u64, Vec<u8>, bool)>,
}

impl<'a, W: Write> OrbxWriter<'a, W> {
    pub fn new(out: &'a mut W, version: &str) -> Result<Self> {
        let mut header = vec![0x0e, 0x02, b'O', b'X', 0x12, 0x01, 0x1a, 0x1d, 0x61, 0x5e, 0x0e];
        header.extend_from_slice(b"octane_version");
        header.push(0x66);
        header.extend(enc_u(version.len() as u64)?);
        header.extend_from_slice(version.as_bytes());
        header.push(0x00);
        out.write_all(&header)?;
        Ok(OrbxWriter { out, chunks: Vec::new(), files: Vec::new(), current: None })
    }

    fn chunk(&mut self, part: &[u8]) -> Result<()> {
        self.out.write_all(&[0x3f])?;
        self.out.write_all(&enc_u(part.len() as u64)?)?;
        self.out.write_all(part)?;
        self.chunks.push(part.len() as u64);
        Ok(())
    }

    /// Start a file; its data follows through `Write`, until `end`. The
    /// scene (`*.ocs`) is kept whole, as one chunk.
    pub fn begin(&mut self, path: &str) -> Result<()> {
        self.end()?;
        self.current = Some((path.to_string(), self.chunks.len(), 0, Vec::new(), path.ends_with(".ocs")));
        Ok(())
    }

    /// Finish the file begun last (its tail as the last chunk; an empty
    /// file is one empty chunk).
    pub fn end(&mut self) -> Result<()> {
        if let Some((path, first, len, tail, _)) = self.current.take() {
            if !tail.is_empty() || self.chunks.len() == first {
                self.chunk(&tail)?;
            }
            self.files.push((path, first, len));
        }
        Ok(())
    }

    /// A whole file.
    pub fn add(&mut self, path: &str, data: &[u8]) -> Result<()> {
        self.begin(path)?;
        self.write_all(data)?;
        self.end()
    }

    /// End the last file and write the index: the chunk list (offset deltas
    /// between consecutive chunk tags), then the files sorted by path.
    pub fn finish(mut self) -> Result<()> {
        self.end()?;
        let chunks = &self.chunks;
        let mut idx = vec![0x79, 0x79, 0x1e, 0x04];
        idx.extend_from_slice(b"file");
        idx.push(0x3b);
        idx.extend(enc_s(chunks.len() as u64)?);
        idx.push(0x43);
        idx.extend(enc_s(chunks.iter().sum())?);
        idx.extend_from_slice(&[0x00, 0x00, 0x71, 0x71]);
        let mut prev: Option<u64> = None;
        for &size in chunks {
            let delta = match prev {
                None => 0,
                Some(p) => p + 1 + enc_u(p)?.len() as u64,
            };
            idx.push(0x33);
            idx.extend(enc_s(delta)?);
            idx.push(0x53);
            idx.extend(enc_s(size)?);
            prev = Some(size);
        }
        idx.extend_from_slice(&[0x00, 0x00, 0x69, 0x69]);
        let mut files: Vec<&(String, usize, u64)> = self.files.iter().collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, first, len) in files {
            idx.push(0x23);
            idx.extend(enc_s(*first as u64)?);
            idx.push(0x33);
            idx.extend(enc_s(*len)?);
            idx.push(0x3e);
            idx.extend(enc_u(path.len() as u64)?);
            idx.extend_from_slice(path.as_bytes());
        }
        idx.extend_from_slice(&[0x00, 0x00]);

        // Index + footer, written twice.
        let len = u32::try_from(idx.len()).context("orbx index too large")?;
        for _ in 0..2 {
            self.out.write_all(&idx)?;
            self.out.write_all(&len.to_le_bytes())?;
            self.out.write_all(&MAGIC)?;
        }
        Ok(())
    }
}

impl<W: Write> Write for OrbxWriter<'_, W> {
    fn write(&mut self, mut data: &[u8]) -> std::io::Result<usize> {
        let n = data.len();
        let Some((_, _, len, tail, whole)) = self.current.as_mut() else {
            return Err(std::io::Error::other("orbx: no file begun"));
        };
        *len += n as u64;
        if *whole {
            tail.extend_from_slice(data);
            return Ok(n);
        }
        // Top up the pending chunk, send full chunks straight from `data`,
        // keep the rest.
        let mut tail = std::mem::take(tail);
        if !tail.is_empty() {
            let take = (CHUNK - tail.len()).min(data.len());
            tail.extend_from_slice(&data[..take]);
            data = &data[take..];
            if tail.len() == CHUNK {
                self.chunk(&tail).map_err(std::io::Error::other)?;
                tail.clear();
            }
        }
        while data.len() >= CHUNK {
            self.chunk(&data[..CHUNK]).map_err(std::io::Error::other)?;
            data = &data[CHUNK..];
        }
        tail.extend_from_slice(data);
        if let Some(cur) = self.current.as_mut() {
            cur.3 = tail;
        }
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}

/// Read an integer of either flavor (the forms share one layout).
fn dec(b: &[u8], i: &mut usize) -> Result<u64> {
    let c = *b.get(*i).ok_or_else(|| anyhow!("orbx: truncated"))? as u64;
    let n = match c {
        _ if c < 0x80 => 0,
        _ if c < 0xc0 => 1,
        _ if c < 0xe0 => 2,
        _ if c < 0xf0 => 3,
        _ => 4,
    };
    let mask = [0x7f, 0x3f, 0x1f, 0x0f, 0x07][n];
    let mut v = c & mask;
    for k in 1..=n {
        v = (v << 8) | *b.get(*i + k).ok_or_else(|| anyhow!("orbx: truncated"))? as u64;
    }
    *i += n + 1;
    Ok(v)
}

fn expect(b: &[u8], i: &mut usize, bytes: &[u8]) -> Result<()> {
    if b.get(*i..*i + bytes.len()) != Some(bytes) {
        bail!("orbx: unexpected bytes at {}", *i);
    }
    *i += bytes.len();
    Ok(())
}

/// Read an ORBX package: `(octane version, files in write order)`.
pub fn read_orbx(b: &[u8]) -> Result<(String, Vec<(String, Vec<u8>)>)> {
    let mut i = 0;
    expect(b, &mut i, &[0x0e, 0x02, b'O', b'X', 0x12, 0x01, 0x1a, 0x1d, 0x61, 0x5e, 0x0e])?;
    expect(b, &mut i, b"octane_version")?;
    expect(b, &mut i, &[0x66])?;
    let vl = dec(b, &mut i)? as usize;
    let version = String::from_utf8_lossy(&b[i..i + vl]).into_owned();
    i += vl;
    expect(b, &mut i, &[0x00])?;
    let base = i;

    if b.len() < 8 || b[b.len() - 4..] != MAGIC {
        bail!("orbx: missing footer");
    }
    let n = u32::from_le_bytes(b[b.len() - 8..b.len() - 4].try_into()?) as usize;
    let mut j = b.len() - 8 - n;
    expect(b, &mut j, &[0x79, 0x79, 0x1e, 0x04])?;
    expect(b, &mut j, b"file")?;
    expect(b, &mut j, &[0x3b])?;
    let count = dec(b, &mut j)? as usize;
    expect(b, &mut j, &[0x43])?;
    dec(b, &mut j)?;
    expect(b, &mut j, &[0x00, 0x00, 0x71, 0x71])?;
    let mut chunks = Vec::with_capacity(count);
    let mut pos = 0usize;
    for _ in 0..count {
        expect(b, &mut j, &[0x33])?;
        pos += dec(b, &mut j)? as usize;
        expect(b, &mut j, &[0x53])?;
        let size = dec(b, &mut j)? as usize;
        let mut k = base + pos;
        expect(b, &mut k, &[0x3f])?;
        dec(b, &mut k)?;
        chunks.push(&b[k..k + size]);
    }
    expect(b, &mut j, &[0x00, 0x00, 0x69, 0x69])?;
    let mut files = Vec::new();
    while b.get(j) == Some(&0x23) {
        j += 1;
        let first = dec(b, &mut j)? as usize;
        expect(b, &mut j, &[0x33])?;
        let size = dec(b, &mut j)? as usize;
        expect(b, &mut j, &[0x3e])?;
        let pl = dec(b, &mut j)? as usize;
        let path = String::from_utf8_lossy(&b[j..j + pl]).into_owned();
        j += pl;
        let mut data = Vec::with_capacity(size);
        let mut k = first;
        while data.len() < size {
            data.extend_from_slice(chunks.get(k).ok_or_else(|| anyhow!("orbx: chunk {k} missing"))?);
            k += 1;
        }
        files.push((first, path, data));
    }
    files.sort_by_key(|f| f.0);
    Ok((version, files.into_iter().map(|(_, p, d)| (p, d)).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(version: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        write_orbx(&mut buf, version, files).unwrap();
        let (v, back) = read_orbx(&buf).unwrap();
        assert_eq!(v, version);
        let back: Vec<(&str, &[u8])> = back.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
        assert_eq!(back, files);
        buf
    }

    #[test]
    fn integers_follow_the_signed_and_unsigned_forms() {
        assert_eq!(enc_u(2239).unwrap(), [0x88, 0xbf]);
        assert_eq!(enc_u(1 << 20).unwrap(), [0xd0, 0x00, 0x00]);
        assert_eq!(enc_s(1 << 20).unwrap(), [0xe0, 0x10, 0x00, 0x00]);
        assert_eq!(enc_s(64).unwrap(), [0x80, 0x40]);
        assert_eq!(enc_s(63).unwrap(), [0x3f]);
        for v in [0u64, 63, 64, 8191, 8192, 1 << 20, (1 << 27) - 1, 1 << 27, 300_000_000] {
            let mut i = 0;
            assert_eq!(dec(&enc_s(v).unwrap(), &mut i).unwrap(), v);
            let mut i = 0;
            assert_eq!(dec(&enc_u(v).unwrap(), &mut i).unwrap(), v);
        }
    }

    #[test]
    fn chunks_large_assets_but_not_the_scene() {
        let big: Vec<u8> = (0..(CHUNK * 2 + 10)).map(|i| i as u8).collect();
        let ocs = vec![b'x'; CHUNK + 5];
        let buf = roundtrip("15050000", &[("assets/b.obj", &big), ("assets/a.png", b"png"), ("project.ocs", &ocs)]);
        // 3 chunks for the asset, 1 for the png, 1 for the 1 MiB+ scene.
        let n = u32::from_le_bytes(buf[buf.len() - 8..buf.len() - 4].try_into().unwrap()) as usize;
        let idx = &buf[buf.len() - 8 - n..buf.len() - 8];
        assert_eq!(&idx[..10], &[0x79, 0x79, 0x1e, 0x04, b'f', b'i', b'l', b'e', 0x3b, 5]);
    }

    /// Streamed in uneven pieces, a package comes out as written whole.
    #[test]
    fn streaming_matches_whole_files() {
        let big: Vec<u8> = (0..(CHUNK * 3 + 77)).map(|i| (i * 7) as u8).collect();
        let ocs = vec![b'y'; CHUNK + 9];
        let files: [(&str, &[u8]); 4] = [("assets/b.bin", &big), ("assets/e", b""), ("assets/a.png", b"png"), ("project.ocs", &ocs)];
        let mut whole = Vec::new();
        write_orbx(&mut whole, "15050000", &files).unwrap();
        let mut streamed = Vec::new();
        let mut w = OrbxWriter::new(&mut streamed, "15050000").unwrap();
        for (path, data) in files {
            w.begin(path).unwrap();
            for piece in data.chunks(333_333) {
                w.write_all(piece).unwrap();
            }
        }
        w.finish().unwrap();
        assert!(streamed == whole);
    }

    /// The bundled Octane X sample, rebuilt from its own files, is identical.
    #[test]
    fn reproduces_octane_sample_byte_for_byte() {
        let sample = std::path::Path::new("/Applications/Octane X.app/Contents/Resources/cornell.orbx");
        let Ok(orig) = std::fs::read(sample) else { return };
        let (version, files) = read_orbx(&orig).unwrap();
        let files: Vec<(&str, &[u8])> = files.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
        let mut again = Vec::new();
        write_orbx(&mut again, &version, &files).unwrap();
        assert!(again == orig, "rebuilt cornell.orbx differs");
    }
}
