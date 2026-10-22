//! DSON helpers shared by the scene readers: DAZ files (`.duf`, `.dsf`) are
//! JSON, optionally gzip-compressed, with percent-encoded asset URLs.

use anyhow::Result;

/// The JSON text of a DAZ file, gunzipped when it starts with the gzip magic.
pub fn decompress_scene(bytes: &[u8]) -> Result<String> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        use std::io::Read;
        let mut s = String::new();
        flate2::read::GzDecoder::new(bytes).read_to_string(&mut s)?;
        Ok(s)
    } else {
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Percent-decode a DAZ asset URL (`/data/DAZ%203D/…` → `/data/DAZ 3D/…`).
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex_nibble(b[i + 1]), hex_nibble(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decodes_asset_urls() {
        assert_eq!(percent_decode("/data/DAZ%203D/Genesis%209"), "/data/DAZ 3D/Genesis 9");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn reads_plain_and_gzipped_scenes() {
        use std::io::Write;
        let json = r#"{"file_version":"0.6.0.0"}"#;
        assert_eq!(decompress_scene(json.as_bytes()).unwrap(), json);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(json.as_bytes()).unwrap();
        assert_eq!(decompress_scene(&gz.finish().unwrap()).unwrap(), json);
    }
}
