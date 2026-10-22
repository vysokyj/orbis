//! USDZ container writer.

use super::*;

/// Write a `.usdz`: a stored (uncompressed) zip whose file data all start at
/// 64-byte offsets. The first file is the default layer.
pub fn write_usdz(out: &mut impl Write, files: &[(&str, &[u8])]) -> Result<()> {
    if files.is_empty() {
        bail!("empty usdz");
    }
    const ALIGN: u64 = 64;
    const PAD_ID: u16 = 0x1986; // the extra-field id Pixar's packager uses
    let mut offset: u64 = 0;
    let mut central = Vec::new();
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());

    for (name, data) in files {
        let size = u32::try_from(data.len()).map_err(|_| anyhow!("{name}: over 4 GiB"))?;
        let header_offset = u32::try_from(offset).map_err(|_| anyhow!("usdz over 4 GiB"))?;
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let crc = crc.sum();

        // Pad through the extra field so the data lands on a 64-byte boundary;
        // a non-empty extra field needs its own 4-byte header.
        let base = offset + 30 + name.len() as u64;
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
        out.write_all(&h)?;
        out.write_all(data)?;

        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20); // version made by
        le16(&mut central, 20); // version needed
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0x21);
        le32(&mut central, crc);
        le32(&mut central, size);
        le32(&mut central, size);
        le16(&mut central, name.len() as u16);
        le16(&mut central, 0); // extra
        le16(&mut central, 0); // comment
        le16(&mut central, 0); // disk
        le16(&mut central, 0); // internal attrs
        le32(&mut central, 0); // external attrs
        le32(&mut central, header_offset);
        central.extend_from_slice(name.as_bytes());

        offset += h.len() as u64 + data.len() as u64;
    }
    let cd_offset = u32::try_from(offset).map_err(|_| anyhow!("usdz over 4 GiB"))?;
    let n = u16::try_from(files.len()).map_err(|_| anyhow!("too many files"))?;
    let mut end = Vec::new();
    le32(&mut end, 0x0605_4b50);
    le16(&mut end, 0);
    le16(&mut end, 0);
    le16(&mut end, n);
    le16(&mut end, n);
    le32(&mut end, central.len() as u32);
    le32(&mut end, cd_offset);
    le16(&mut end, 0);
    out.write_all(&central)?;
    out.write_all(&end)?;
    Ok(())
}
