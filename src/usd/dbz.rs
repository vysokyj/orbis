//! The `.dbz` posed-scene export — final posed vertices per node.

use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct Dbz {
    pub(super) rootpaths: RootPaths,
    pub(super) figures: Vec<DbzFigure>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct RootPaths {
    pub(super) content: Vec<String>,
    /// DAZ Studio's Iray shader folders — `/resources/…` (e.g. the default
    /// HDRI) resolves against these.
    pub(super) builtin_mdl: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct DbzFigure {
    pub(super) name: String,
    pub(super) label: Option<String>,
    pub(super) lod: i64,
    pub(super) vertices: Vec<[f32; 3]>,
    /// Strand hair (dForce): each strand's vertex indices, posed as rendered.
    pub(super) polylines: Vec<Vec<u32>>,
    /// Each strand's index into `material_groups`.
    pub(super) polyline_materials: Vec<u32>,
    #[serde(rename = "material groups")]
    pub(super) material_groups: Vec<String>,
}

impl Dbz {
    pub(super) fn load(path: &Path) -> Result<Dbz> {
        Self::parse(&decompress_scene(&std::fs::read(path)?)?)
    }

    pub(super) fn parse(json: &str) -> Result<Dbz> {
        let dbz: Dbz = serde_json::from_str(json)?;
        Ok(dbz)
    }

    /// The entry for a scene node: by label (unique per scene), else by a
    /// name only one entry carries.
    pub(super) fn figure(&self, label: &str, name: &str) -> Option<&DbzFigure> {
        let with_verts = || self.figures.iter().filter(|f| !f.vertices.is_empty());
        if let Some(f) = with_verts().find(|f| f.label.as_deref() == Some(label)) {
            return Some(f);
        }
        let mut by_name = with_verts().filter(|f| f.name == name);
        match (by_name.next(), by_name.next()) {
            (Some(f), None) => Some(f),
            _ => None,
        }
    }
}
