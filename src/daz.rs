//! Where DAZ Studio keeps things on this machine: content directories and
//! the application bundle.

use std::path::PathBuf;

/// DAZ Studio's configured content directories, read from its macOS
/// preferences (`ContentDir0..N`), newest Studio version first; falls back to
/// the home default if none are found. Inside DAZ Studio the plugin script
/// passes the content manager's list instead (`--content-dir`).
pub fn content_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "macos")]
    for domain in ["com.daz.Studio6", "com.daz.Studio4"] {
        for i in 0..64 {
            let out = std::process::Command::new("defaults")
                .args(["read", domain, &format!("ContentDir{i}")])
                .output();
            match out {
                Ok(o) if o.status.success() => {
                    let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if !p.is_empty() {
                        let path = PathBuf::from(p);
                        if !dirs.contains(&path) {
                            dirs.push(path);
                        }
                    }
                }
                _ => break, // ContentDir indices are contiguous; stop at the first gap
            }
        }
    }

    if dirs.is_empty() {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            dirs.push(home.join("Documents/DAZ 3D/Studio/My Library"));
        }
    }
    dirs
}

/// Locate the DAZ Studio application (macOS), newest version first.
pub fn studio_app() -> Option<PathBuf> {
    ["DAZStudio6 64-bit", "DAZStudio4 64-bit"]
        .iter()
        .map(|d| PathBuf::from("/Applications/DAZ 3D").join(d).join("DAZStudio.app"))
        .find(|p| p.exists())
}

