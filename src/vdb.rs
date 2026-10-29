//! OpenVDB files: just enough of the header to name their grids.
//!
//! The layout (OpenVDB's `io/File.cc`, `io/Archive.cc`): a magic number,
//! the file and library versions, whether grid offsets follow, the UUID,
//! the file's metadata, then one descriptor per grid — its name, type and
//! where it lies in the file. Octane reads the voxels itself.

use std::io::{Read, Seek, SeekFrom};

use anyhow::{bail, Context, Result};

const MAGIC: u64 = 0x5644_4220;

/// The oldest file version read (OpenVDB 2.x writes 222 and up; 216 added
/// grid instancing, which the descriptors carry).
const OLDEST: u32 = 216;

/// A grid of a `.vdb` file.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub name: String,
    /// `Tree_float_5_4_3`, `Tree_vec3s_5_4_3`, … (`_HalfFloat` when saved as halves).
    pub kind: String,
}

/// The grids of the `.vdb` at `path`, in file order.
pub fn grids(path: &std::path::Path) -> Result<Vec<Grid>> {
    let f = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    read_grids(&mut std::io::BufReader::new(f)).with_context(|| path.display().to_string())
}

fn read_grids(r: &mut (impl Read + Seek)) -> Result<Vec<Grid>> {
    if u64_(r)? != MAGIC {
        bail!("not an OpenVDB file");
    }
    let version = u32_(r)?;
    if version < OLDEST {
        bail!("OpenVDB file version {version} (older than {OLDEST})");
    }
    skip(r, 8)?; // library major, minor
    if bytes(r, 1)?[0] == 0 {
        bail!("no grid offsets (written as a stream)");
    }
    if version < 220 {
        skip(r, 1)?; // compression, per file before 220
    }
    skip(r, 36)?; // UUID
    for _ in 0..u32_(r)? {
        string(r)?; // name
        string(r)?; // type
        string(r)?; // value, length-prefixed like a string
    }
    let count = u32_(r)?;
    let mut grids = Vec::new();
    for k in 0..count {
        // A name taken twice is made unique with "\x1e<n>".
        let name = string(r)?.split('\x1e').next().unwrap_or_default().to_string();
        let kind = string(r)?;
        string(r)?; // instance parent
        skip(r, 16)?; // grid and block positions
        let end = u64_(r)?;
        grids.push(Grid { name, kind });
        // The grid's voxels lie between its descriptor and the next.
        if k + 1 < count {
            r.seek(SeekFrom::Start(end))?;
        }
    }
    Ok(grids)
}

fn bytes(r: &mut impl Read, n: usize) -> Result<Vec<u8>> {
    let mut b = vec![0; n];
    r.read_exact(&mut b).context("truncated header")?;
    Ok(b)
}

fn skip(r: &mut impl Read, n: usize) -> Result<()> {
    bytes(r, n).map(drop)
}

fn u32_(r: &mut impl Read) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes(r, 4)?.try_into().unwrap()))
}

fn u64_(r: &mut impl Read) -> Result<u64> {
    Ok(u64::from_le_bytes(bytes(r, 8)?.try_into().unwrap()))
}

fn string(r: &mut impl Read) -> Result<String> {
    let n = u32_(r)? as usize;
    if n > 1 << 20 {
        bail!("string of {n} bytes in the header");
    }
    Ok(String::from_utf8_lossy(&bytes(r, n)?).into_owned())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn string(s: &str) -> Vec<u8> {
        let mut v = (s.len() as u32).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v
    }

    /// A header as OpenVDB writes it: two grids, the first's voxels (here
    /// 1000 bytes of nothing) between their descriptors.
    pub(crate) fn file() -> Vec<u8> {
        let mut b = MAGIC.to_le_bytes().to_vec();
        b.extend(224u32.to_le_bytes());
        b.extend([11, 0, 0, 0, 0, 0, 0, 0, 1]);
        b.extend([b'0'; 36]);
        b.extend(1u32.to_le_bytes());
        b.extend(string("creator"));
        b.extend(string("string"));
        b.extend(string("test"));
        b.extend(2u32.to_le_bytes());
        b.extend(string("density"));
        b.extend(string("Tree_float_5_4_3"));
        b.extend(string(""));
        let end = b.len() as u64 + 24 + 1000;
        b.extend([0u8; 16]);
        b.extend(end.to_le_bytes());
        b.extend(vec![0u8; 1000]);
        b.extend(string("temperature\x1e1"));
        b.extend(string("Tree_float_5_4_3_HalfFloat"));
        b.extend(string(""));
        b.extend([0u8; 24]);
        b
    }

    #[test]
    fn names_the_grids_past_their_voxels() {
        let dir = std::env::temp_dir().join(format!("orbis-vdb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("two.vdb");
        std::fs::write(&p, file()).unwrap();
        let g = grids(&p).unwrap();
        assert_eq!(g, vec![Grid { name: "density".into(), kind: "Tree_float_5_4_3".into() }, Grid { name: "temperature".into(), kind: "Tree_float_5_4_3_HalfFloat".into() }]);
        std::fs::write(&p, b"not a vdb at all").unwrap();
        assert!(grids(&p).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
