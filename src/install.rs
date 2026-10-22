//! `orbis install` — set up the DAZ Studio plugin: this binary goes to
//! `~/Library/Application Support/ORBIS/bin/orbis`, and the script with its
//! Content Library thumbnails into a content library's `Scripts/ORBIS/`,
//! with the binary's path and the default output folder written into it.
//! Script and icons are compiled in, so one binary is all an install needs.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

/// The DAZ Studio script, as the Content Library lists it.
pub const SCRIPT_NAME: &str = "Export to Octane.dsa";
const SCRIPT: &str = include_str!("../daz/Scripts/ORBIS/Export to Octane.dsa");
/// Content Library thumbnail: `<script>.png` beside the script.
const SCRIPT_ICON: &[u8] = include_bytes!("../daz/Scripts/ORBIS/Export to Octane.png");
/// The folder's thumbnail: `Scripts/ORBIS.png`.
const FOLDER_ICON: &[u8] = include_bytes!("../daz/Scripts/ORBIS.png");

/// Where an install puts things.
#[derive(Debug, Clone)]
pub struct InstallPaths {
    /// The DAZ content library getting `Scripts/ORBIS/`.
    pub library: PathBuf,
    /// The installed `orbis` binary.
    pub binary: PathBuf,
    /// The script's default output folder.
    pub output_dir: PathBuf,
}

impl InstallPaths {
    /// The library named (or "My Library" from DAZ Studio's content
    /// directories, else the first one), the binary under Application
    /// Support, output to `~/Documents/ORBIS`.
    pub fn default_for(library: Option<PathBuf>) -> Result<InstallPaths> {
        let library = library.or_else(default_library).ok_or_else(|| anyhow!("no DAZ content library found; name one with --library"))?;
        let data = dirs::data_dir().ok_or_else(|| anyhow!("no application data folder"))?;
        Ok(InstallPaths { library, binary: data.join("ORBIS/bin/orbis"), output_dir: crate::usd::default_output_dir() })
    }
}

/// The content directory named "My Library" (DAZ Studio's default user
/// library), else the first one.
pub fn default_library() -> Option<PathBuf> {
    let dirs = crate::daz::content_dirs();
    dirs.iter().find(|d| d.file_name().is_some_and(|n| n == "My Library")).or(dirs.first()).cloned()
}

/// Install `binary` (normally this executable) and the script; returns the
/// script's folder.
pub fn install(paths: &InstallPaths, binary: &Path) -> Result<PathBuf> {
    let same = |a: &Path, b: &Path| matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b);
    if !same(binary, &paths.binary) {
        std::fs::create_dir_all(paths.binary.parent().unwrap())?;
        // Through a temporary file and a rename: overwriting a binary in
        // place breaks a running copy's code signature.
        let data = std::fs::read(binary).with_context(|| format!("reading {}", binary.display()))?;
        write_via_tmp(&paths.binary, &data)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&paths.binary, std::fs::Permissions::from_mode(0o755))?;
    }
    // A downloaded binary is quarantined until notarized (#3); a failure
    // here only means there was no flag.
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("xattr").args(["-d", "com.apple.quarantine"]).arg(&paths.binary).output();

    let scripts = paths.library.join("Scripts");
    let plugin = scripts.join("ORBIS");
    std::fs::create_dir_all(&plugin).with_context(|| format!("creating {}", plugin.display()))?;
    write_via_tmp(&plugin.join(SCRIPT_NAME), script(paths).as_bytes())?;
    write_via_tmp(&plugin.join("Export to Octane.png"), SCRIPT_ICON)?;
    write_via_tmp(&scripts.join("ORBIS.png"), FOLDER_ICON)?;
    // Earlier installs kept the binary here, in sight of the Content Library.
    let old_bin = plugin.join("bin");
    if old_bin.join("orbis").is_file() {
        std::fs::remove_dir_all(&old_bin)?;
    }
    Ok(plugin)
}

/// The script with the install's paths filled in.
fn script(paths: &InstallPaths) -> String {
    SCRIPT
        .replace("\"@ORBIS_BIN@\"", &js_string(&paths.binary))
        .replace("\"@DEFAULT_OUTPUT_DIR@\"", &js_string(&paths.output_dir))
}

/// A script string literal, ASCII only (DAZ Studio's script encoding is not
/// something to rely on).
fn js_string(path: &Path) -> String {
    let mut out = String::from("\"");
    for c in path.to_string_lossy().encode_utf16() {
        match c {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            0x20..=0x7e => out.push(c as u8 as char),
            _ => out.push_str(&format!("\\u{c:04x}")),
        }
    }
    out.push('"');
    out
}

fn write_via_tmp(path: &Path, data: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, data).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_binary_and_script() {
        let root = tempfile::tempdir().unwrap();
        let paths = InstallPaths {
            library: root.path().join("My Library"),
            binary: root.path().join("Support/ORBIS/bin/orbis"),
            output_dir: PathBuf::from("/Users/Jiří \"J\"/Documents/ORBIS"),
        };
        let src = root.path().join("fake-orbis");
        std::fs::write(&src, b"#!/bin/sh\n").unwrap();
        // A binary left by an older install inside the library goes away.
        std::fs::create_dir_all(paths.library.join("Scripts/ORBIS/bin")).unwrap();
        std::fs::write(paths.library.join("Scripts/ORBIS/bin/orbis"), b"old").unwrap();

        let plugin = install(&paths, &src).unwrap();
        assert_eq!(plugin, paths.library.join("Scripts/ORBIS"));
        let script = std::fs::read_to_string(plugin.join(SCRIPT_NAME)).unwrap();
        assert!(!script.contains("@ORBIS_BIN@") && !script.contains("@DEFAULT_OUTPUT_DIR@"));
        assert!(script.contains(&format!("var sBin = \"{}\";", paths.binary.display())));
        assert!(script.contains(r#"var sDefaultOutputDir = "/Users/Ji\u0159\u00ed \"J\"/Documents/ORBIS";"#));
        assert!(plugin.join("Export to Octane.png").is_file());
        assert!(paths.library.join("Scripts/ORBIS.png").is_file());
        assert!(!plugin.join("bin").exists());
        assert_eq!(std::fs::read(&paths.binary).unwrap(), b"#!/bin/sh\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&paths.binary).unwrap().permissions().mode() & 0o777, 0o755);
        }
        // Reinstalling from the installed binary keeps it intact.
        install(&paths, &paths.binary).unwrap();
        assert_eq!(std::fs::read(&paths.binary).unwrap(), b"#!/bin/sh\n");
    }
}
