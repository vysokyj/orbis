//! Scene → meshes: geometry nodes, grafts, materials and dForce hair curves.

use super::*;

/// A scene node with geometry.
pub(super) struct GeoNode {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) label: String,
    pub(super) parent: Option<String>,
    pub(super) conform_target: Option<String>,
    /// Geometry instance id — scene materials refer to it.
    pub(super) instance_id: String,
    pub(super) url: String,
    pub(super) shell_instance: bool,
    /// The instance's own `type` (the scene can turn subdivision on or off).
    pub(super) subdivision: Option<bool>,
    /// The node's "Visible" channel (hidden nodes are left out).
    pub(super) visible: bool,
    /// A geometry shell's visible material groups (`material_group_<g>_vis`),
    /// when it lists any.
    pub(super) shell_groups: Option<HashSet<String>>,
}

/// The node channels (`extra[].channels[]`) of a scene node: id → value.
pub(super) fn node_channels(n: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for e in n.get("extra").and_then(Value::as_array).into_iter().flatten() {
        for c in e.get("channels").and_then(Value::as_array).into_iter().flatten() {
            let Some(ch) = c.get("channel") else { continue };
            if let (Some(id), Some(v)) = (ch.get("id").and_then(Value::as_str), ch.get("current_value").or_else(|| ch.get("value"))) {
                out.insert(id.to_string(), v.clone());
            }
        }
    }
    out
}

pub(super) fn ref_id(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(|s| percent_decode(s.trim_start_matches('#')))
}

pub(super) fn geo_nodes(duf: &Value) -> Vec<GeoNode> {
    let nodes = duf.pointer("/scene/nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    // The scene's own node definitions (primitives, props made in DAZ): a
    // hidden one keeps its `Visible` there, not on the scene node — Calib-Leaf's
    // hidden cubes rendered in Octane, not in Iray.
    let local: HashMap<String, &Value> = duf.pointer("/node_library").and_then(Value::as_array).into_iter().flatten()
        .filter_map(|d| Some((percent_decode(d.get("id")?.as_str()?), d)))
        .collect();
    nodes
        .iter()
        .filter_map(|n| {
            let gi = n.get("geometries")?.as_array()?.first()?;
            let id = percent_decode(n.get("id")?.as_str()?);
            let mut channels = n.get("url").and_then(Value::as_str)
                .filter(|u| u.starts_with('#'))
                .and_then(|u| local.get(&percent_decode(&u[1..])))
                .map(|d| node_channels(d))
                .unwrap_or_default();
            channels.extend(node_channels(n));
            let shell_groups: HashSet<String> = channels
                .iter()
                .filter(|(_, v)| v.as_bool() == Some(true))
                .filter_map(|(k, _)| k.strip_prefix("material_group_")?.strip_suffix("_vis").map(String::from))
                .collect();
            let lists_groups = channels.keys().any(|k| k.starts_with("material_group_") && k.ends_with("_vis"));
            let name = n.get("name").and_then(Value::as_str).unwrap_or(&id).to_string();
            Some(GeoNode {
                label: n.get("label").and_then(Value::as_str).unwrap_or(&name).to_string(),
                name,
                id,
                parent: ref_id(n.get("parent")),
                conform_target: ref_id(n.get("conform_target")),
                instance_id: percent_decode(gi.get("id").and_then(Value::as_str).unwrap_or_default()),
                url: gi.get("url").and_then(Value::as_str)?.to_string(),
                subdivision: gi.get("type").and_then(Value::as_str).map(|t| t == "subdivision_surface"),
                shell_instance: gi.get("extra").and_then(Value::as_array).is_some_and(|e| {
                    e.iter().any(|x| x.get("type").and_then(Value::as_str) == Some("studio/geometry/shell"))
                }),
                visible: channels.get("Visible").and_then(Value::as_bool).unwrap_or(true),
                shell_groups: lists_groups.then_some(shell_groups),
            })
        })
        .collect()
}

/// Material group → UV set URL, from the scene materials of one geometry
/// instance.
pub(super) fn material_uv_sets(duf: &Value, instance_id: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for m in duf.pointer("/scene/materials").and_then(Value::as_array).into_iter().flatten() {
        if ref_id(m.get("geometry")).as_deref() != Some(instance_id) {
            continue;
        }
        let Some(uv) = m.get("uv_set").and_then(Value::as_str) else { continue };
        for g in m.get("groups").and_then(Value::as_array).into_iter().flatten() {
            if let Some(g) = g.as_str() {
                out.insert(g.to_string(), uv.to_string());
            }
        }
    }
    out
}

/// Material group → merged DAZ material, for one geometry instance: the
/// definition (`url` into a `material_library`) overlaid with the scene
/// material's own channels.
pub(super) fn scene_materials(lib: &mut Library, instance_id: &str) -> HashMap<String, Rc<crate::daz_material::Material>> {
    use crate::daz_material::{image_library, Material};
    let images = image_library(&lib.duf);
    let mats: Vec<Value> = lib
        .duf
        .pointer("/scene/materials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|m| ref_id(m.get("geometry")).as_deref() == Some(instance_id))
        .cloned()
        .collect();
    let mut out = HashMap::new();
    for m in mats {
        let mut mat = match m.get("url").and_then(Value::as_str).and_then(|u| lib.entry(u, "material_library")) {
            Some(def) => Material::from_dson(&def, &images),
            None => Material::default(),
        };
        mat.merge(&m, &images);
        let mat = Rc::new(mat);
        for g in m.get("groups").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            out.insert(g.to_string(), mat.clone());
        }
    }
    out
}

/// One source geometry contributing to an output mesh (a shell combines its
/// host with the host's grafts).
pub(super) struct Part {
    pub(super) topo: Rc<Topology>,
    pub(super) vert_offset: u32,
    pub(super) hidden: HashSet<u32>,
    pub(super) group_prefix: Option<String>,
    pub(super) uv_by_group: HashMap<String, String>,
    /// Vertices welded onto the host (a merged graft's seam): local → mesh index.
    pub(super) weld: HashMap<u32, u32>,
    /// Materials by the group name the mesh uses (prefixed for grafts).
    pub(super) materials: HashMap<String, Rc<crate::daz_material::Material>>,
}

/// An output mesh, ready to write.
#[derive(Debug, Default)]
pub(super) struct Mesh {
    pub(super) name: String,
    pub(super) label: String,
    pub(super) points: Vec<[f32; 3]>,
    pub(super) face_counts: Vec<u32>,
    pub(super) face_indices: Vec<u32>,
    /// Material group name → faces.
    pub(super) subsets: Vec<(String, Vec<u32>)>,
    /// The DAZ material of each subset (same order), if the scene has one.
    pub(super) materials: Vec<Option<Rc<crate::daz_material::Material>>>,
    /// Indexed faceVarying UVs: (values, one index per face corner).
    pub(super) uvs: Option<(Vec<[f32; 2]>, Vec<u32>)>,
    pub(super) normals: Option<Vec<[f32; 3]>>,
    /// Strand hair: `face_counts` are the strands' vertex counts, the points
    /// run strand by strand, one subset (its material group) — written as
    /// `BasisCurves` with these per-vertex widths (meters).
    pub(super) curve_widths: Option<Vec<f32>>,
    /// Instanced: where its copies go (the mesh itself, when drawn, is one at
    /// `IDENTITY`) — written as a `PointInstancer` with it as the prototype.
    pub(super) instances: Option<Vec<Placement>>,
}

/// Meters per unit of DAZ's hair line widths (`Line Start Width` /
/// `Line End Width`, which DAZ labels in mm) — 2× that: Calib-Eye's
/// close-up covers the background as Iray's at 2× (0.47 vs 0.43; 1× 0.28,
/// 4× 0.63), and Calib-Hair's fringe and sides match Iray's fullness. The
/// earlier 4× was fitted on guide-only strands (the missing PR hairs).
pub(super) const LINE_WIDTH_SCALE: f32 = 0.002;

/// One `Mesh` (curves) per material group of a strand-hair figure, its
/// strands' widths tapering from root to tip as DAZ's hair shader sets them.
pub(super) fn hair_curves(lib: &mut Library, n: &GeoNode, fig: &DumpNode, names: &mut HashSet<String>) -> Vec<Mesh> {
    let materials = scene_materials(lib, &n.instance_id);
    let mut by_group: Vec<Vec<&Vec<u32>>> = vec![Vec::new(); fig.material_groups.len().max(1)];
    for (k, strand) in fig.polylines.iter().enumerate() {
        let g = fig.polyline_materials.get(k).copied().unwrap_or(0) as usize;
        if let Some(list) = by_group.get_mut(g) {
            list.push(strand);
        }
    }
    let mut out = Vec::new();
    for (g, strands) in by_group.into_iter().enumerate() {
        if strands.is_empty() {
            continue;
        }
        let group = fig.material_groups.get(g).cloned().unwrap_or_else(|| "Hair".into());
        let mat = materials.get(&group).cloned();
        let width = |id: &str, default: f32| mat.as_ref().and_then(|m| m.float(id)).unwrap_or(default) * LINE_WIDTH_SCALE;
        let (root, tip) = (width("Line Start Width", 0.1), width("Line End Width", 0.05));
        let mut mesh = Mesh {
            label: format!("{} {group}", n.label),
            name: unique_name(names, &format!("{}_{group}", n.label)),
            ..Default::default()
        };
        let mut widths = Vec::new();
        for s in &strands {
            let len = s.len().max(2) as f32 - 1.0;
            for (k, &v) in s.iter().enumerate() {
                let p = fig.vertices.get(v as usize).copied().unwrap_or_default();
                mesh.points.push(p.map(|c| c * CM));
                widths.push(root + (tip - root) * k as f32 / len);
            }
            mesh.face_counts.push(s.len() as u32);
        }
        mesh.subsets = vec![(group, (0..strands.len() as u32).collect())];
        mesh.materials = vec![mat];
        mesh.curve_widths = Some(widths);
        out.push(mesh);
    }
    out
}

pub(super) fn build_meshes(lib: &mut Library, dump: &SceneDump) -> Result<(Vec<Mesh>, ExportReport)> {
    let nodes = geo_nodes(&lib.duf);
    let mut report = ExportReport::default();

    // Grafts per host node id, in scene order.
    let mut topo_of: HashMap<String, Rc<Topology>> = HashMap::new();
    for n in &nodes {
        if let Some(t) = lib.topology(&n.url) {
            topo_of.insert(n.id.clone(), t);
        }
    }
    let mut grafts: HashMap<String, Vec<&GeoNode>> = HashMap::new();
    for n in &nodes {
        if topo_of.get(&n.id).is_some_and(|t| t.graft_hidden.is_some()) {
            if let Some(host) = n.conform_target.as_ref().or(n.parent.as_ref()) {
                grafts.entry(host.clone()).or_default().push(n);
            }
        }
    }
    let hidden_of = |host: &str| -> HashSet<u32> {
        grafts
            .get(host)
            .into_iter()
            .flatten()
            .filter_map(|g| topo_of.get(&g.id)?.graft_hidden.clone())
            .flatten()
            .collect()
    };
    let by_id: HashMap<&str, &GeoNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Grafts DAZ welds onto a visible host get merged into the host's mesh, so
    // the seam subdivides as one surface (separately subdivided meshes pull
    // apart and leave a crack).
    let mergeable = |g: &GeoNode| {
        g.visible
            && dump.node(&g.label, &g.name).is_some_and(|f| topo_of.get(&g.id).is_some_and(|t| t.vcount == f.vertices.len()))
    };
    let mut merged: HashSet<&str> = HashSet::new();
    for (host, gs) in &grafts {
        let Some(h) = by_id.get(host.as_str()) else { continue };
        if !h.visible || dump.node(&h.label, &h.name).is_none() || !topo_of.contains_key(host) {
            continue;
        }
        merged.extend(gs.iter().filter(|g| mergeable(g)).map(|g| g.id.as_str()));
    }

    let mut names = HashSet::new();
    let mut meshes = Vec::new();
    // Polygon meshes: (index in `meshes`, node id, conform target, refined).
    let mut surfaces: Vec<(usize, String, Option<String>, bool)> = Vec::new();
    for n in &nodes {
        // A hidden node's instances still render (UltraScatter hides its
        // sources); the node is then only their prototype.
        let copies = dump.node(&n.label, &n.name).and_then(|f| placements(dump, &n.label, &n.name, f, n.visible));
        if !n.visible && copies.is_none() {
            report.skipped.push(format!("{}: hidden in the scene", n.label));
            continue;
        }
        if merged.contains(n.id.as_str()) {
            continue; // part of its host's mesh
        }
        let instances = copies.as_ref().map_or(0, |c| c.len() - usize::from(n.visible));
        let Some(fig) = dump.node(&n.label, &n.name) else {
            report.skipped.push(format!("{}: not in the scene dump", n.label));
            continue;
        };
        if !fig.polylines.is_empty() {
            // Strand hair: the dump holds the rendered strands; the library
            // mesh is only their guide.
            for mut mesh in hair_curves(lib, n, fig, &mut names) {
                mesh.instances = copies.clone();
                report.instances += instances;
                report.meshes += 1;
                report.points += mesh.points.len();
                meshes.push(mesh);
            }
            continue;
        }
        let Some(topo) = topo_of.get(&n.id).cloned() else {
            report.skipped.push(format!("{}: geometry {} not found", n.label, n.url));
            continue;
        };
        let is_shell = topo.shell || n.shell_instance;
        // `shape_node` owns the geometry instance whose type decides subdivision
        // (a shell subdivides like its host).
        let (parts, shape_node): (Vec<Part>, &GeoNode) = if is_shell {
            // A shell copies its host's cached geometry, grafts included
            // (unwelded); its materials belong to the shell's own instance and
            // already name the grafts' groups with their prefix.
            let Some(host) = n.parent.as_deref().and_then(|p| by_id.get(p)) else {
                report.skipped.push(format!("{}: shell without a host", n.label));
                continue;
            };
            let Some(host_topo) = topo_of.get(&host.id).cloned() else {
                report.skipped.push(format!("{}: shell host geometry not found", n.label));
                continue;
            };
            let mats = scene_materials(lib, &n.instance_id);
            let mut parts = vec![Part {
                vert_offset: 0,
                hidden: hidden_of(&host.id),
                group_prefix: None,
                uv_by_group: material_uv_sets(&lib.duf, &host.instance_id),
                topo: host_topo.clone(),
                weld: HashMap::new(),
                materials: mats.clone(),
            }];
            let mut offset = host_topo.vcount as u32;
            for g in grafts.get(&host.id).into_iter().flatten() {
                let t = topo_of[&g.id].clone();
                parts.push(Part {
                    vert_offset: offset,
                    hidden: HashSet::new(),
                    group_prefix: Some(g.name.clone()),
                    uv_by_group: material_uv_sets(&lib.duf, &g.instance_id),
                    // Weld like the host's own mesh, or the shell's graft
                    // border floats beside the body (a visible outline).
                    weld: t.graft_pairs.iter().copied().collect(),
                    topo: t.clone(),
                    materials: mats.clone(),
                });
                offset += t.vcount as u32;
            }
            (parts, host)
        } else {
            let mut parts = vec![Part {
                vert_offset: 0,
                hidden: hidden_of(&n.id),
                group_prefix: None,
                uv_by_group: material_uv_sets(&lib.duf, &n.instance_id),
                topo: topo.clone(),
                weld: HashMap::new(),
                materials: scene_materials(lib, &n.instance_id),
            }];
            let mut offset = topo.vcount as u32;
            for g in grafts.get(&n.id).into_iter().flatten().filter(|g| merged.contains(g.id.as_str())) {
                let t = topo_of[&g.id].clone();
                let materials = scene_materials(lib, &g.instance_id)
                    .into_iter()
                    .map(|(group, m)| (format!("{}_{group}", g.name), m))
                    .collect();
                parts.push(Part {
                    vert_offset: offset,
                    hidden: HashSet::new(),
                    group_prefix: Some(g.name.clone()),
                    uv_by_group: material_uv_sets(&lib.duf, &g.instance_id),
                    weld: t.graft_pairs.iter().map(|&(gv, hv)| (gv, hv)).collect(),
                    topo: t.clone(),
                    materials,
                });
                offset += t.vcount as u32;
            }
            (parts, n)
        };
        // Points: this node's, then (merged grafts) each graft's.
        let own: usize = if is_shell { parts.iter().map(|p| p.topo.vcount).sum() } else { parts[0].topo.vcount };
        // A host's cache can come with its grafts' vertices appended (as a
        // shell's does) — DAZ attaches them once the grafts' caches are
        // built; the host's own come first.
        let with_grafts = own + grafts.get(&n.id).into_iter().flatten().filter_map(|g| topo_of.get(&g.id)).map(|t| t.vcount).sum::<usize>();
        let host_with_grafts = !is_shell && with_grafts > own && fig.vertices.len() == with_grafts;
        if own != fig.vertices.len() && !host_with_grafts {
            let breakdown: Vec<String> = parts.iter().map(|p| p.topo.vcount.to_string()).collect();
            report.skipped.push(format!(
                "{}: the dump has {} vertices, geometry {}{}",
                n.label,
                fig.vertices.len(),
                own,
                if is_shell && parts.len() > 1 { format!(" ({})", breakdown.join(" + ")) } else { String::new() }
            ));
            continue;
        }
        let mut points: Vec<[f32; 3]> = fig.vertices[..own].to_vec();
        if !is_shell {
            for g in grafts.get(&n.id).into_iter().flatten().filter(|g| merged.contains(g.id.as_str())) {
                points.extend_from_slice(&dump.node(&g.label, &g.name).map(|f| f.vertices.clone()).unwrap_or_default());
            }
        }
        // A shell has no resolution level of its own (0 in the dump); it
        // renders at its host's.
        let lod = if is_shell { dump.node(&shape_node.label, &shape_node.name).map_or(fig.lod, |h| h.lod) } else { fig.lod };
        let refined = shape_node.subdivision.unwrap_or(parts[0].topo.subdivision) && lod > 0;
        // A shell renders only the surfaces it switches on — picked after
        // refining it whole, as its host is: a cut-out patch refines its
        // border by the boundary rules and pulled it up to 0.3 mm under the
        // skin in creases, where random-walk skin lit it white (Octane4).
        let visible = if is_shell { n.shell_groups.as_ref() } else { None };
        let mut mesh = assemble(lib, &parts, if refined { None } else { visible });
        let prop = !fig.class.contains("Figure");
        let open = open_subsets(&mesh);
        for (m, open) in mesh.materials.iter_mut().zip(open) {
            if let Some(m) = m.as_mut().filter(|_| prop || open) {
                *m = Rc::new(crate::daz_material::Material { prop: m.prop || prop, open, ..(**m).clone() });
            }
        }
        mesh.label = n.label.clone();
        mesh.name = unique_name(&mut names, &n.label);
        // DAZ works in centimeters; write meters. Octane's USD import ignores
        // `metersPerUnit` and takes points as meters.
        mesh.points = points.iter().map(|p| [p[0] * CM, p[1] * CM, p[2] * CM]).collect();
        if refined {
            mesh = subdivide_once(&mesh);
            if let Some(v) = visible {
                mesh = keep_groups(&mesh, v);
            }
        }
        mesh.instances = copies;
        report.instances += instances;
        report.meshes += 1;
        report.points += mesh.points.len();
        report.faces += mesh.face_counts.len();
        surfaces.push((meshes.len(), n.id.clone(), n.conform_target.clone(), refined));
        meshes.push(mesh);
    }
    // Unrefined items DAZ fitted onto a refined figure (a hair cap, brows)
    // sink into it here and there: lift them clear of its surface.
    for (guest, _, target, refined) in &surfaces {
        let host = target.as_ref().and_then(|t| surfaces.iter().find(|(_, id, _, r)| id == t && *r));
        if let (false, Some((host, ..))) = (refined, host) {
            let host_mesh = std::mem::take(&mut meshes[*host]);
            let lifted = lift_off(&mut meshes[*guest], &host_mesh);
            meshes[*host] = host_mesh;
            if lifted > 0 {
                tracing::info!(mesh = %meshes[*guest].label, lifted, "USD export: lifted off its figure's surface");
            }
        }
    }
    for (m, ..) in &surfaces {
        let mesh = &mut meshes[*m];
        mesh.normals = Some(vertex_normals(&mesh.points, &mesh.face_counts, &mesh.face_indices));
    }
    for s in &report.skipped {
        tracing::info!(reason = %s, "USD export: node skipped");
    }
    Ok((meshes, report))
}

/// Per subset: whether any of its faces has an edge no other face of the
/// mesh shares — an open sheet rather than (part of) a closed volume.
pub(super) fn open_subsets(mesh: &Mesh) -> Vec<bool> {
    let mut starts = Vec::with_capacity(mesh.face_counts.len());
    let mut at = 0usize;
    for &c in &mesh.face_counts {
        starts.push(at);
        at += c as usize;
    }
    let edges = |f: usize| {
        let (s, n) = (starts[f], mesh.face_counts[f] as usize);
        (0..n).map(move |k| {
            let (a, b) = (mesh.face_indices[s + k], mesh.face_indices[s + (k + 1) % n]);
            (a.min(b), a.max(b))
        })
    };
    let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
    for f in 0..mesh.face_counts.len() {
        for e in edges(f) {
            *uses.entry(e).or_default() += 1;
        }
    }
    mesh.subsets.iter().map(|(_, faces)| faces.iter().any(|&f| edges(f as usize).any(|e| uses[&e] == 1))).collect()
}

/// Merge `parts` into one mesh; `visible` (a shell's switched-on material
/// groups) drops every polygon of any other group.
pub(super) fn assemble(lib: &mut Library, parts: &[Part], visible: Option<&HashSet<String>>) -> Mesh {
    let mut mesh = Mesh::default();
    let mut group_index: HashMap<String, usize> = HashMap::new();
    let mut uv_values: Vec<[f32; 2]> = Vec::new();
    let mut uv_base: HashMap<String, Option<(u32, Rc<UvSet>)>> = HashMap::new();
    let mut uv_indices: Vec<u32> = Vec::new();
    let mut zero_uv: Option<u32> = None;
    let mut any_uv = false;

    for part in parts {
        for (pi, poly) in part.topo.polys.iter().enumerate() {
            let pi = pi as u32;
            if part.hidden.contains(&pi) {
                continue;
            }
            let group = part.topo.groups.get(poly.mat as usize).cloned().unwrap_or_else(|| format!("group{}", poly.mat));
            let group_name = match &part.group_prefix {
                Some(p) => format!("{p}_{group}"),
                None => group.clone(),
            };
            if visible.is_some_and(|v| !v.contains(&group_name)) {
                continue;
            }
            // A fully transparent surface (opacity 0, no map) is left out: it
            // renders nothing, yet under random-walk skin it is a boundary —
            // the NGV8 pubic hair's "Hiding" strands 0–2 mm under the skin
            // showed as white specks (Octane4).
            if part.materials.get(&group_name).is_some_and(|m| m.opacity() <= 0.0 && m.image(m.opacity_channel()).is_none()) {
                continue;
            }
            let face = mesh.face_counts.len() as u32;
            let gi = *group_index.entry(group_name.clone()).or_insert_with(|| {
                mesh.materials.push(part.materials.get(&group_name).cloned());
                mesh.subsets.push((group_name.clone(), Vec::new()));
                mesh.subsets.len() - 1
            });
            mesh.subsets[gi].1.push(face);
            mesh.face_counts.push(poly.n as u32);
            mesh.face_indices.extend(poly.verts().iter().map(|v| part.weld.get(v).copied().unwrap_or(v + part.vert_offset)));

            // UVs: the material's UV set, else the geometry default.
            let url = part.uv_by_group.get(&group).cloned().or_else(|| part.topo.default_uv.clone());
            let set = url.and_then(|u| {
                uv_base
                    .entry(u.clone())
                    .or_insert_with(|| {
                        let s = lib.uv_set(&u).filter(|s| s.vertex_count == part.topo.vcount)?;
                        let base = uv_values.len() as u32;
                        uv_values.extend_from_slice(&s.uvs);
                        Some((base, s))
                    })
                    .clone()
            });
            for &v in poly.verts() {
                match &set {
                    Some((base, s)) => {
                        any_uv = true;
                        uv_indices.push(base + s.uv_index(pi, v));
                    }
                    None => {
                        let z = *zero_uv.get_or_insert_with(|| {
                            uv_values.push([0.0, 0.0]);
                            uv_values.len() as u32 - 1
                        });
                        uv_indices.push(z);
                    }
                }
            }
        }
    }
    if any_uv {
        // Clamp stray indices (malformed overrides) to a valid value.
        let max = uv_values.len() as u32;
        for i in &mut uv_indices {
            if *i >= max {
                *i = 0;
            }
        }
        mesh.uvs = Some((uv_values, uv_indices));
    }
    mesh
}
