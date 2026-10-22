//! The scene dump — what DAZ Studio evaluated: final posed vertices per node
//! and the rendered strands of strand hair. Written by the DAZ Studio script
//! (`daz/Scripts/ORBIS/Export to Octane.dsa`); format in
//! `docs/spec/octane-export.md`.

use super::*;

/// The dump format this build reads (`format_version`).
pub const DUMP_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct SceneDump {
    pub(super) format: String,
    pub(super) format_version: u32,
    /// DAZ Studio's content directories, in its order.
    pub(super) content_dirs: Vec<String>,
    /// DAZ Studio's Iray shader folders — `/resources/…` (e.g. the default
    /// HDRI) resolves against these.
    pub(super) builtin_mdl_dirs: Vec<String>,
    pub(super) nodes: Vec<DumpNode>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct DumpNode {
    pub(super) name: String,
    pub(super) label: String,
    /// Resolution level DAZ Studio renders at (0 = base, 1 = SubD); the
    /// vertices are always the base mesh's.
    pub(super) lod: i64,
    /// World space, cm, Y up, morphs and pose applied — `x y z` flat in the file.
    #[serde(deserialize_with = "flat_points")]
    pub(super) vertices: Vec<[f32; 3]>,
    /// Strand hair (dForce): each strand's vertex indices, posed as rendered.
    pub(super) polylines: Vec<Vec<u32>>,
    /// Each strand's index into `material_groups`.
    pub(super) polyline_materials: Vec<u32>,
    pub(super) material_groups: Vec<String>,
}

fn flat_points<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Vec<[f32; 3]>, D::Error> {
    let flat = Vec::<f32>::deserialize(d)?;
    if flat.len() % 3 != 0 {
        return Err(serde::de::Error::custom(format!("{} vertex coordinates, not a multiple of 3", flat.len())));
    }
    Ok(flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

impl SceneDump {
    pub(super) fn load(path: &Path) -> Result<SceneDump> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    pub(super) fn parse(json: &str) -> Result<SceneDump> {
        let dump: SceneDump = serde_json::from_str(json)?;
        if dump.format != "orbis-scene-dump" {
            bail!("not an ORBIS scene dump");
        }
        if dump.format_version != DUMP_FORMAT_VERSION {
            bail!(
                "scene dump format {} — this orbis reads {DUMP_FORMAT_VERSION}; install the matching DAZ Studio script (orbis install)",
                dump.format_version
            );
        }
        Ok(dump)
    }

    /// The entry for a scene node: by label (unique per scene), else by a
    /// name only one entry carries.
    pub(super) fn node(&self, label: &str, name: &str) -> Option<&DumpNode> {
        let with_verts = || self.nodes.iter().filter(|f| !f.vertices.is_empty());
        if let Some(f) = with_verts().find(|f| f.label == label) {
            return Some(f);
        }
        let mut by_name = with_verts().filter(|f| f.name == name);
        match (by_name.next(), by_name.next()) {
            (Some(f), None) => Some(f),
            _ => None,
        }
    }
}

/// Where `orbis` keeps the last dump of a scene, so an export re-runs without
/// DAZ Studio: `<cache>/orbis/dumps/<scene stem>-<hash of its path>.json`.
pub fn dump_cache_path(scene: &Path) -> PathBuf {
    let stem = scene.file_stem().and_then(|s| s.to_str()).unwrap_or("scene");
    // FNV-1a: stable across Rust versions, unlike `DefaultHasher`.
    let hash = scene.to_string_lossy().bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("orbis/dumps")
        .join(format!("{stem}-{:08x}.json", hash as u32))
}

/// Keep a fresh dump (written by the DAZ Studio script) as the scene's cached
/// one and return its new place.
pub fn store_dump(scene: &Path, dump: &Path) -> Result<PathBuf> {
    let cached = dump_cache_path(scene);
    if cached == dump {
        return Ok(cached);
    }
    std::fs::create_dir_all(cached.parent().unwrap())?;
    // Rename when on the same volume, else copy.
    if std::fs::rename(dump, &cached).is_err() {
        std::fs::copy(dump, &cached).with_context(|| format!("copying {} to {}", dump.display(), cached.display()))?;
        let _ = std::fs::remove_file(dump);
    }
    Ok(cached)
}
