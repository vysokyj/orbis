//! `orbis install` — put the DAZ Studio plugin into a content library:
//! `Scripts/ORBIS/` with the script, its Content Library thumbnails and this
//! binary as `bin/orbis`. The script and icons are compiled in, so one binary
//! is all an install needs (the release zip is staged the same way).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The DAZ Studio script, as the Content Library lists it.
pub const SCRIPT_NAME: &str = "Export to Octane.dsa";
const SCRIPT: &str = include_str!("../daz/Scripts/ORBIS/Export to Octane.dsa");
/// Content Library thumbnail: `<script>.png` beside the script.
const SCRIPT_ICON: &[u8] = include_bytes!("../daz/Scripts/ORBIS/Export to Octane.png");
/// The folder's thumbnail: `Scripts/ORBIS.png`.
const FOLDER_ICON: &[u8] = include_bytes!("../daz/Scripts/ORBIS.png");

/// The library to install into: the content directory named "My Library"
/// (DAZ Studio's default user library), else the first one.
pub fn default_library() -> Option<PathBuf> {
    let dirs = crate::daz::content_dirs();
    dirs.iter().find(|d| d.file_name().is_some_and(|n| n == "My Library")).or(dirs.first()).cloned()
}

/// Install the plugin into `library` with `binary` as its `bin/orbis`;
/// returns the plugin folder.
pub fn install(library: &Path, binary: &Path) -> Result<PathBuf> {
    let scripts = library.join("Scripts");
    let plugin = scripts.join("ORBIS");
    let bin_dir = plugin.join("bin");
    std::fs::create_dir_all(&bin_dir).with_context(|| format!("creating {}", bin_dir.display()))?;
    write_via_tmp(&plugin.join(SCRIPT_NAME), SCRIPT.as_bytes())?;
    write_via_tmp(&plugin.join("Export to Octane.png"), SCRIPT_ICON)?;
    write_via_tmp(&scripts.join("ORBIS.png"), FOLDER_ICON)?;

    let target = bin_dir.join("orbis");
    let same = |a: &Path, b: &Path| matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b);
    if !same(binary, &target) {
        // Through a temporary file and a rename: overwriting a binary in
        // place breaks a running copy's code signature.
        let data = std::fs::read(binary).with_context(|| format!("reading {}", binary.display()))?;
        write_via_tmp(&target, &data)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
    }
    // A downloaded binary is quarantined until notarized (#3); a failure
    // here only means there was no flag.
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("xattr").args(["-dr", "com.apple.quarantine"]).arg(&plugin).output();
    Ok(plugin)
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
    fn installs_the_plugin_folder() {
        let lib = tempfile::tempdir().unwrap();
        let bin = lib.path().join("fake-orbis");
        std::fs::write(&bin, b"#!/bin/sh\n").unwrap();
        let plugin = install(lib.path(), &bin).unwrap();
        assert_eq!(plugin, lib.path().join("Scripts/ORBIS"));
        assert!(std::fs::read_to_string(plugin.join(SCRIPT_NAME)).unwrap().contains("orbis-scene-dump"));
        assert!(plugin.join("Export to Octane.png").is_file());
        assert!(lib.path().join("Scripts/ORBIS.png").is_file());
        assert_eq!(std::fs::read(plugin.join("bin/orbis")).unwrap(), b"#!/bin/sh\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(plugin.join("bin/orbis")).unwrap().permissions().mode() & 0o777, 0o755);
        }
        // Reinstalling over itself (the installed binary) keeps it intact.
        install(lib.path(), &plugin.join("bin/orbis")).unwrap();
        assert_eq!(std::fs::read(plugin.join("bin/orbis")).unwrap(), b"#!/bin/sh\n");
    }
}
