//! The scene dump — what DAZ Studio evaluated: final posed vertices per node
//! and the rendered strands of strand hair. Written by the DAZ Studio script
//! (`daz/Scripts/ORBIS/Export to Octane.dsa`); format in
//! `docs/spec/octane-export.md`.

use super::*;

/// The dump format this build reads (`format_version`).
pub const DUMP_FORMAT_VERSION: u32 = 3;

/// The oldest dump format still read: 2 differs from 3 only in writing every
/// strand's indices (3 may give their lengths, `polyline_counts`).
const DUMP_FORMAT_OLDEST: u32 = 2;

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
    /// Instance nodes and instance groups (UltraScatter): copies of a node
    /// placed elsewhere.
    pub(super) instances: Vec<DumpInstance>,
}

/// An affine transform, the rows of DAZ's 3×4 `DzMatrix3` (`m11 m12 m13 m14
/// m21 …`): p' = M · p + t, the 4th column t in cm.
pub(super) type Affine = [f64; 12];

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct DumpInstance {
    pub(super) name: String,
    pub(super) label: String,
    pub(super) target_name: String,
    pub(super) target_label: String,
    /// World space, one per copy: the instance node's, or each item's of an
    /// instance group.
    pub(super) transforms: Vec<Affine>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct DumpNode {
    pub(super) name: String,
    pub(super) label: String,
    /// Resolution level DAZ Studio renders at (0 = base, 1 = SubD); the
    /// vertices are always the base mesh's.
    pub(super) lod: i64,
    /// DAZ class: `DzFigure` for figures and what is fitted to them.
    pub(super) class: String,
    /// The node's world-space transform — instances of it replace it.
    pub(super) transform: Option<Affine>,
    /// World space, cm, Y up, morphs and pose applied — `x y z` flat in the file.
    #[serde(deserialize_with = "flat_points")]
    pub(super) vertices: Vec<[f32; 3]>,
    /// Strand hair (dForce): each strand's vertex indices, posed as rendered.
    pub(super) polylines: Vec<Vec<u32>>,
    /// Strand hair as runs of consecutive vertices: each strand's length
    /// (the dump writes these when it can; `parse` turns them into
    /// `polylines`).
    pub(super) polyline_counts: Vec<u32>,
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
        if !(DUMP_FORMAT_OLDEST..=DUMP_FORMAT_VERSION).contains(&dump.format_version) {
            bail!(
                "scene dump format {} — this orbis reads {DUMP_FORMAT_VERSION}; install the matching DAZ Studio script (orbis install)",
                dump.format_version
            );
        }
        let mut dump = dump;
        for n in &mut dump.nodes {
            if n.polylines.is_empty() && !n.polyline_counts.is_empty() {
                let mut at = 0u32;
                n.polylines = std::mem::take(&mut n.polyline_counts)
                    .into_iter()
                    .map(|c| {
                        let s = (at..at + c).collect();
                        at += c;
                        s
                    })
                    .collect();
            }
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
    dumps_dir().join(dump_file_name(scene))
}

fn dumps_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("orbis/dumps")
}

/// `<scene stem>-<FNV-1a of its path>.json`.
pub(super) fn dump_file_name(scene: &Path) -> String {
    let stem = scene.file_stem().and_then(|s| s.to_str()).unwrap_or("scene");
    // FNV-1a: stable across Rust versions, unlike `DefaultHasher`.
    let hash = scene.to_string_lossy().bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    format!("{stem}-{:08x}.json", hash as u32)
}

/// The cached dump for a scene given by any path (#159): its own key as
/// given, as an absolute and as a canonical path, else a dump of the same
/// stem whose `scene` (the path DAZ Studio saw) is a byte-identical file —
/// a copy such as `calib/scenes/X.duf` of `My Library/Scenes/X.duf`. The
/// newest such dump wins.
pub fn find_dump(scene: &Path) -> Option<PathBuf> {
    find_dump_in(&dumps_dir(), scene)
}

pub(super) fn find_dump_in(dir: &Path, scene: &Path) -> Option<PathBuf> {
    let mut keys = vec![scene.to_path_buf()];
    keys.extend(std::path::absolute(scene).ok());
    keys.extend(std::fs::canonicalize(scene).ok());
    if let Some(d) = keys.iter().map(|k| dir.join(dump_file_name(k))).find(|d| d.is_file()) {
        return Some(d);
    }
    let stem = scene.file_stem()?.to_str()?;
    let wanted = std::fs::read(scene).ok()?;
    let same_key = |name: &str| {
        name.strip_prefix(stem)
            .and_then(|r| r.strip_prefix('-'))
            .and_then(|r| r.strip_suffix(".json"))
            .is_some_and(|h| h.len() == 8 && h.bytes().all(|b| b.is_ascii_hexdigit()))
    };
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_str().is_some_and(same_key))
        .filter(|e| dump_scene(&e.path()).is_some_and(|s| std::fs::read(s).is_ok_and(|b| b == wanted)))
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
}

/// The `scene` field of a dump (the `.duf` path DAZ Studio saw), read from
/// the head of the file: the script writes it before the nodes.
fn dump_scene(dump: &Path) -> Option<PathBuf> {
    use std::io::Read;
    let mut head = Vec::new();
    std::fs::File::open(dump).ok()?.take(64 * 1024).read_to_end(&mut head).ok()?;
    let head = String::from_utf8_lossy(&head);
    let at = head.find("\"scene\"")? + "\"scene\"".len();
    let rest = head[at..].trim_start().strip_prefix(':')?.trim_start();
    let mut de = serde_json::Deserializer::from_str(rest);
    String::deserialize(&mut de).ok().map(PathBuf::from)
}

/// Keep a fresh dump (written by the DAZ Studio script) as the scene's cached
/// one and return its new place. Keyed by the absolute path, so a relative
/// one finds it again from anywhere.
pub fn store_dump(scene: &Path, dump: &Path) -> Result<PathBuf> {
    let cached = dump_cache_path(&std::path::absolute(scene).unwrap_or_else(|_| scene.to_path_buf()));
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
