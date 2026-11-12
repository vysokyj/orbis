//! Library assets — DSON URL resolution, topology and UV sets from `.dsf`.

use super::*;

/// One polygon: facet (polygon) group and material group indices, 3 or 4
/// vertex indices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Poly {
    pub(super) facet: u32,
    pub(super) mat: u32,
    pub(super) n: u8,
    pub(super) v: [u32; 4],
}

impl Poly {
    pub(super) fn verts(&self) -> &[u32] {
        &self.v[..self.n as usize]
    }
}

/// A `.dsf`/`.duf` geometry definition.
#[derive(Debug, Default)]
pub(super) struct Topology {
    pub(super) vcount: usize,
    pub(super) polys: Vec<Poly>,
    pub(super) groups: Vec<String>,
    /// Facet groups (`polygon_groups`: a figure's bone regions).
    pub(super) facet_groups: Vec<String>,
    pub(super) default_uv: Option<String>,
    pub(super) subdivision: bool,
    /// SubD Level (`SubDIALevel`) and Render SubD Level (`SubDRenderLevel`)
    /// as the definition sets them (`subd_levels`).
    pub(super) subd: [Option<i64>; 2],
    /// Host polygons this geometry hides when grafted.
    pub(super) graft_hidden: Option<Vec<u32>>,
    /// Graft vertex → host vertex it is welded to.
    pub(super) graft_pairs: Vec<(u32, u32)>,
    pub(super) shell: bool,
}

#[derive(Debug, Default)]
pub(super) struct UvSet {
    pub(super) vertex_count: usize,
    pub(super) uvs: Vec<[f32; 2]>,
    /// (polygon, vertex) → uv index.
    pub(super) overrides: HashMap<(u32, u32), u32>,
}

impl UvSet {
    pub(super) fn uv_index(&self, poly: u32, vertex: u32) -> u32 {
        self.overrides.get(&(poly, vertex)).copied().unwrap_or(vertex)
    }
}

/// Resolves and caches DSON assets: the scene's own libraries (`#id`) and
/// library files (`/data/…/x.dsf#id`).
pub(super) struct Library {
    pub(super) dirs: Vec<PathBuf>,
    pub(super) duf: Value,
    pub(super) files: HashMap<String, Option<Rc<Value>>>,
    pub(super) topologies: HashMap<String, Option<Rc<Topology>>>,
    pub(super) uv_sets: HashMap<String, Option<Rc<UvSet>>>,
    /// The scene's image gammas (`daz_material::scene_image_gammas`), once.
    pub(super) image_gammas: Option<Rc<HashMap<String, f32>>>,
    /// Meters per unit of DAZ's hair line widths (`Tuning::line_width_scale`).
    pub(super) line_width_scale: f32,
}

impl Library {
    pub(super) fn new(dirs: Vec<PathBuf>, duf: Value) -> Self {
        Library { dirs, duf, files: HashMap::new(), topologies: HashMap::new(), uv_sets: HashMap::new(), image_gammas: None, line_width_scale: super::mesh::LINE_WIDTH_SCALE }
    }

    /// Split a DSON URL into (decoded file path, decoded fragment id); an
    /// empty path means the scene file itself.
    pub(super) fn split(url: &str) -> (String, String) {
        let (file, id) = url.split_once('#').unwrap_or((url, ""));
        (percent_decode(file), percent_decode(id))
    }

    pub(super) fn file(&mut self, path: &str) -> Option<Rc<Value>> {
        if let Some(v) = self.files.get(path) {
            return v.clone();
        }
        let rel = path.trim_start_matches('/');
        let loaded = self.dirs.iter().map(|d| d.join(rel)).find(|p| p.is_file()).and_then(|p| {
            let text = decompress_scene(&std::fs::read(&p).ok()?).ok()?;
            match serde_json::from_str::<DsonFile>(&text) {
                Ok(v) => Some(Rc::new(v.0)),
                Err(e) => {
                    tracing::warn!(path = %p.display(), error = %e, "unparseable DSON asset");
                    None
                }
            }
        });
        if loaded.is_none() {
            tracing::warn!(path, "DSON asset not found in content directories");
        }
        self.files.insert(path.to_string(), loaded.clone());
        loaded
    }

    /// The `library` entry (e.g. `geometry_library`) a URL points at.
    pub(super) fn entry(&mut self, url: &str, library: &str) -> Option<Value> {
        let (file, id) = Self::split(url);
        let find = |doc: &Value| {
            doc.get(library)?
                .as_array()?
                .iter()
                .find(|e| e.get("id").and_then(Value::as_str).map(percent_decode).as_deref() == Some(id.as_str()))
                .cloned()
        };
        if file.is_empty() {
            find(&self.duf)
        } else {
            find(&*self.file(&file)?)
        }
    }

    /// Like `entry`, but taking the entry out of its document: geometry and
    /// UV sets are parsed once and cached as such, and as JSON values they
    /// are the bulk of a scene's library (intake: 6.4 GB of 300 MB files).
    fn take_entry(&mut self, url: &str, library: &str) -> Option<Value> {
        let (file, id) = Self::split(url);
        let take = |doc: &mut Value| {
            let list = doc.get_mut(library)?.as_array_mut()?;
            let at = list.iter().position(|e| e.get("id").and_then(Value::as_str).map(percent_decode).as_deref() == Some(id.as_str()))?;
            Some(list.swap_remove(at))
        };
        if file.is_empty() {
            return take(&mut self.duf);
        }
        self.file(&file)?;
        match self.files.get_mut(&file) {
            Some(Some(doc)) => match Rc::get_mut(doc) {
                Some(doc) => take(doc),
                None => self.entry(url, library),
            },
            _ => None,
        }
    }

    /// A cache key: the URL's file and id, decoded — one entry however a
    /// URL spells them (`take_entry` leaves nothing for a second look).
    fn key(url: &str) -> String {
        let (file, id) = Self::split(url);
        format!("{file}#{id}")
    }

    pub(super) fn topology(&mut self, url: &str) -> Option<Rc<Topology>> {
        let key = Self::key(url);
        if let Some(t) = self.topologies.get(&key) {
            return t.clone();
        }
        let t = self.take_entry(url, "geometry_library").map(|g| Rc::new(parse_topology(&g, url)));
        self.topologies.insert(key, t.clone());
        t
    }

    /// A UV set by (absolute or scene-local) URL.
    pub(super) fn uv_set(&mut self, url: &str) -> Option<Rc<UvSet>> {
        let key = Self::key(url);
        if let Some(u) = self.uv_sets.get(&key) {
            return u.clone();
        }
        let u = self.take_entry(url, "uv_set_library").map(|v| Rc::new(parse_uv_set(&v)));
        self.uv_sets.insert(key, u.clone());
        u
    }
}

/// A library file's JSON without its `modifier_library` (morphs and
/// projection data — never read, half of a hair's 94 MB file).
struct DsonFile(Value);

impl<'de> Deserialize<'de> for DsonFile {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Top;
        impl<'de> serde::de::Visitor<'de> for Top {
            type Value = DsonFile;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a DSON object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> std::result::Result<DsonFile, A::Error> {
                let mut out = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if key == "modifier_library" {
                        map.next_value::<serde::de::IgnoredAny>()?;
                    } else {
                        out.insert(key, map.next_value()?);
                    }
                }
                Ok(DsonFile(Value::Object(out)))
            }
        }
        d.deserialize_map(Top)
    }
}

/// `#id` URLs inside a library file refer to that file, not the scene.
pub(super) fn absolutize(url: &str, base: &str) -> String {
    if url.starts_with('#') {
        let file = base.split_once('#').map(|(f, _)| f).unwrap_or(base);
        format!("{file}{url}")
    } else {
        url.to_string()
    }
}

pub(super) fn values(v: &Value, key: &str) -> Vec<Value> {
    v.get(key)
        .and_then(|x| x.get("values").or(Some(x)))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub(super) fn parse_topology(g: &Value, url: &str) -> Topology {
    let polys = values(g, "polylist")
        .iter()
        .filter_map(|p| {
            let p = p.as_array()?;
            let n = p.len().checked_sub(2)?;
            if !(3..=4).contains(&n) {
                return None;
            }
            let mut v = [0u32; 4];
            for (i, x) in p[2..].iter().enumerate() {
                v[i] = x.as_u64()? as u32;
            }
            Some(Poly { facet: p[0].as_u64()? as u32, mat: p[1].as_u64()? as u32, n: n as u8, v })
        })
        .collect();
    let vcount = g
        .get("vertices")
        .and_then(|v| v.get("count").and_then(Value::as_u64).map(|c| c as usize))
        .unwrap_or_else(|| values(g, "vertices").len());
    let shell = g
        .get("extra")
        .and_then(Value::as_array)
        .is_some_and(|e| e.iter().any(|x| x.get("type").and_then(Value::as_str) == Some("studio/geometry/shell")));
    Topology {
        vcount,
        polys,
        groups: values(g, "polygon_material_groups").iter().filter_map(|s| s.as_str().map(String::from)).collect(),
        facet_groups: values(g, "polygon_groups").iter().filter_map(|s| s.as_str().map(String::from)).collect(),
        default_uv: g.get("default_uv_set").and_then(Value::as_str).map(|u| absolutize(u, url)),
        subdivision: g.get("type").and_then(Value::as_str) == Some("subdivision_surface"),
        subd: subd_channels(g),
        // Hair, brows and lashes often carry an empty `"graft": {}` — only a
        // graft that welds vertices onto its host is one.
        graft_hidden: g.get("graft").filter(|gr| !values(gr, "vertex_pairs").is_empty()).map(|gr| {
            values(gr, "hidden_polys").iter().filter_map(|x| x.as_u64().map(|i| i as u32)).collect()
        }),
        graft_pairs: g
            .get("graft")
            .map(|gr| {
                values(gr, "vertex_pairs")
                    .iter()
                    .filter_map(|p| {
                        let p = p.as_array()?;
                        Some((p.first()?.as_u64()? as u32, p.get(1)?.as_u64()? as u32))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        shell,
    }
}

pub(super) fn parse_uv_set(u: &Value) -> UvSet {
    let uvs = values(u, "uvs")
        .iter()
        .filter_map(|p| {
            let p = p.as_array()?;
            Some([p.first()?.as_f64()? as f32, p.get(1)?.as_f64()? as f32])
        })
        .collect();
    let overrides = u
        .get("polygon_vertex_indices")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|e| {
                    let e = e.as_array()?;
                    Some(((e.first()?.as_u64()? as u32, e.get(1)?.as_u64()? as u32), e.get(2)?.as_u64()? as u32))
                })
                .collect()
        })
        .unwrap_or_default();
    UvSet { vertex_count: u.get("vertex_count").and_then(Value::as_u64).unwrap_or(0) as usize, uvs, overrides }
}

/// A geometry's (or geometry instance's) `SubDIALevel` and
/// `SubDRenderLevel` channels, where it sets them.
pub(super) fn subd_channels(g: &Value) -> [Option<i64>; 2] {
    let mut out = [None, None];
    for c in g.get("extra").and_then(Value::as_array).into_iter().flatten().flat_map(|e| e.get("channels").and_then(Value::as_array).into_iter().flatten()) {
        let Some(c) = c.get("channel") else { continue };
        let v = c.get("current_value").or_else(|| c.get("value")).and_then(Value::as_i64);
        match c.get("id").and_then(Value::as_str) {
            Some("SubDIALevel") => out[0] = v,
            Some("SubDRenderLevel") => out[1] = v,
            _ => {}
        }
    }
    out
}
