//! USDZ container writer.

use super::*;

/// Write a `.usdz`: a stored (uncompressed) zip whose file data all start at
/// 64-byte offsets. The first file is the default layer.
pub fn write_usdz(out: &mut impl Write, files: &[(&str, &[u8])]) -> Result<()> {
    if files.is_empty() {
        bail!("empty usdz");
    }
    let mut w = UsdzWriter::new(out);
    for (name, data) in files {
        w.add(name, data)?;
    }
    w.finish()
}

/// A `.usdz` written a file at a time (`write_usdz`), so the textures of
/// a big scene needn't all be held at once.
pub struct UsdzWriter<'a, W: Write> {
    out: &'a mut W,
    offset: u64,
    central: Vec<u8>,
    files: usize,
}

const ALIGN: u64 = 64;
const PAD_ID: u16 = 0x1986; // the extra-field id Pixar's packager uses

fn le16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn le32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

impl<'a, W: Write> UsdzWriter<'a, W> {
    pub fn new(out: &'a mut W) -> Self {
        UsdzWriter { out, offset: 0, central: Vec::new(), files: 0 }
    }

    /// The bytes written so far.
    pub fn len(&self) -> u64 {
        self.offset
    }

    pub fn add(&mut self, name: &str, data: &[u8]) -> Result<()> {
        let size = u32::try_from(data.len()).map_err(|_| anyhow!("{name}: over 4 GiB"))?;
        let header_offset = u32::try_from(self.offset).map_err(|_| anyhow!("usdz over 4 GiB"))?;
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let crc = crc.sum();

        // Pad through the extra field so the data lands on a 64-byte boundary;
        // a non-empty extra field needs its own 4-byte header.
        let base = self.offset + 30 + name.len() as u64;
        let mut pad = (ALIGN - base % ALIGN) % ALIGN;
        if pad > 0 && pad < 4 {
            pad += ALIGN;
        }
        let mut h = Vec::with_capacity(30 + name.len() + pad as usize);
        le32(&mut h, 0x0403_4b50);
        le16(&mut h, 20); // version needed
        le16(&mut h, 0); // flags
        le16(&mut h, 0); // stored
        le16(&mut h, 0); // time
        le16(&mut h, 0x21); // date: 1980-01-01
        le32(&mut h, crc);
        le32(&mut h, size);
        le32(&mut h, size);
        le16(&mut h, name.len() as u16);
        le16(&mut h, pad as u16);
        h.extend_from_slice(name.as_bytes());
        if pad > 0 {
            le16(&mut h, PAD_ID);
            le16(&mut h, (pad - 4) as u16);
            h.resize(h.len() + (pad - 4) as usize, 0);
        }
        self.out.write_all(&h)?;
        self.out.write_all(data)?;

        let c = &mut self.central;
        le32(c, 0x0201_4b50);
        le16(c, 20); // version made by
        le16(c, 20); // version needed
        le16(c, 0);
        le16(c, 0);
        le16(c, 0);
        le16(c, 0x21);
        le32(c, crc);
        le32(c, size);
        le32(c, size);
        le16(c, name.len() as u16);
        le16(c, 0); // extra
        le16(c, 0); // comment
        le16(c, 0); // disk
        le16(c, 0); // internal attrs
        le32(c, 0); // external attrs
        le32(c, header_offset);
        c.extend_from_slice(name.as_bytes());

        self.offset += h.len() as u64 + data.len() as u64;
        self.files += 1;
        Ok(())
    }

    pub fn finish(self) -> Result<()> {
        if self.files == 0 {
            bail!("empty usdz");
        }
        let cd_offset = u32::try_from(self.offset).map_err(|_| anyhow!("usdz over 4 GiB"))?;
        let n = u16::try_from(self.files).map_err(|_| anyhow!("too many files"))?;
        let mut end = Vec::new();
        le32(&mut end, 0x0605_4b50);
        le16(&mut end, 0);
        le16(&mut end, 0);
        le16(&mut end, n);
        le16(&mut end, n);
        le32(&mut end, self.central.len() as u32);
        le32(&mut end, cd_offset);
        le16(&mut end, 0);
        self.out.write_all(&self.central)?;
        self.out.write_all(&end)?;
        Ok(())
    }
}
