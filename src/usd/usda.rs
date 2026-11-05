//! USD (`.usda`) writing.

use super::*;

/// A valid, unique USD prim name from a DAZ label.
pub(super) fn unique_name(taken: &mut HashSet<String>, label: &str) -> String {
    let mut base: String = label.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    if base.is_empty() || base.starts_with(|c: char| c.is_ascii_digit()) {
        base.insert(0, '_');
    }
    let mut name = base.clone();
    let mut i = 2;
    while !taken.insert(name.clone()) {
        name = format!("{base}_{i}");
        i += 1;
    }
    name
}

pub(super) fn usd_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(super) fn write_ints(out: &mut String, xs: impl IntoIterator<Item = u32>) {
    out.push('[');
    for (i, x) in xs.into_iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{x}");
    }
    out.push(']');
}

/// `x` with at most `decimals` decimals, trailing zeros dropped — keeps the
/// text layer of a large scene small (positions are meters: 5 decimals =
/// 0.01 mm).
pub(super) fn push_num(out: &mut String, x: f32, decimals: usize) {
    let start = out.len();
    let _ = write!(out, "{x:.decimals$}");
    if out[start..].contains('.') {
        let trimmed = out[start..].trim_end_matches('0').trim_end_matches('.').len();
        out.truncate(start + trimmed);
    }
    if &out[start..] == "-0" {
        out.truncate(start);
        out.push('0');
    }
}

/// A number as the text layer spells it (the binary one stores the value).
#[derive(Clone, Copy, Debug)]
pub(super) enum Num {
    /// At most this many decimals, trailing zeros dropped (`push_num`).
    Trim(f64, usize),
    /// Rust's shortest round-trip form.
    Plain(f64),
    /// Exactly this many decimals.
    Fixed(f64, usize),
}

impl Num {
    pub(super) fn value(&self) -> f64 {
        match *self {
            Num::Trim(x, _) | Num::Plain(x) | Num::Fixed(x, _) => x,
        }
    }

    fn write(&self, o: &mut String) {
        match *self {
            Num::Trim(x, d) => push_num(o, x as f32, d),
            Num::Plain(x) => {
                let _ = write!(o, "{x}");
            }
            Num::Fixed(x, d) => {
                let _ = write!(o, "{x:.d$}");
            }
        }
    }
}

/// An attribute value. Arrays carry the decimals the text layer keeps.
#[derive(Debug)]
pub(super) enum Val<'a> {
    /// Declared without a value (`token outputs:surface`).
    None,
    Token(&'a str),
    Str(&'a str),
    Bool(bool),
    Float(Num),
    Color([Num; 3]),
    Matrix([[f64; 4]; 4]),
    Ints(&'a [u32]),
    Floats(&'a [f32], usize),
    Vec3s(&'a [[f32; 3]], usize),
    Vec2s(&'a [[f32; 2]], usize),
    /// Quaternions as (real, i, j, k).
    Quats(&'a [[f32; 4]], usize),
    Tokens(&'a [&'a str]),
}

/// An attribute: name, USD type name (`point3f[]`), value and metadata.
pub(super) struct Attr<'a> {
    pub(super) name: &'a str,
    pub(super) ty: &'a str,
    pub(super) value: Val<'a>,
    pub(super) uniform: bool,
    pub(super) custom: bool,
    pub(super) interpolation: Option<&'a str>,
    /// Connected to this attribute path (`name.connect = <…>`).
    pub(super) connect: Option<&'a str>,
}

impl<'a> Attr<'a> {
    pub(super) fn new(name: &'a str, ty: &'a str, value: Val<'a>) -> Self {
        Attr { name, ty, value, uniform: false, custom: false, interpolation: None, connect: None }
    }
    pub(super) fn uniform(self) -> Self {
        Attr { uniform: true, ..self }
    }
    pub(super) fn custom(self) -> Self {
        Attr { custom: true, ..self }
    }
    pub(super) fn interpolation(self, i: &'a str) -> Self {
        Attr { interpolation: Some(i), ..self }
    }
    pub(super) fn connect(self, target: &'a str) -> Self {
        Attr { connect: Some(target), ..self }
    }
}

/// A prim's metadata.
#[derive(Default)]
pub(super) struct PrimMeta<'a> {
    pub(super) display_name: Option<&'a str>,
    pub(super) kind: Option<&'a str>,
    /// Prepended `apiSchemas`.
    pub(super) api_schemas: &'a [&'a str],
}

/// Where a USD layer goes: the text form (`UsdaWriter`, kept for tests and
/// reading) or the binary one an export writes (`usdc::UsdcWriter`). Prims
/// nest between `begin_prim` and `end_prim`; the stage metadata (Y up,
/// meters, default prim `Scene`) comes with the writer.
pub(super) trait LayerSink {
    fn begin_prim(&mut self, ty: &str, name: &str, meta: &PrimMeta) -> Result<()>;
    fn end_prim(&mut self) -> Result<()>;
    fn attr(&mut self, a: Attr) -> Result<()>;
    /// A relationship to `targets` (absolute paths); `list` spells a single
    /// target as a list in text.
    fn rel(&mut self, name: &str, targets: &[&str], list: bool) -> Result<()>;
}

/// The text (`.usda`) form of a layer.
pub(super) struct UsdaWriter {
    pub(super) text: String,
    depth: usize,
}

impl UsdaWriter {
    pub(super) fn new() -> Self {
        let mut text = String::new();
        text.push_str("#usda 1.0\n(\n    defaultPrim = \"Scene\"\n    metersPerUnit = 1\n    upAxis = \"Y\"\n    doc = \"Exported by ORBIS\"\n)\n");
        UsdaWriter { text, depth: 0 }
    }

    fn indent(&mut self) {
        for _ in 0..self.depth {
            self.text.push_str("    ");
        }
    }
}

impl LayerSink for UsdaWriter {
    fn begin_prim(&mut self, ty: &str, name: &str, meta: &PrimMeta) -> Result<()> {
        self.text.push('\n');
        self.indent();
        let _ = write!(self.text, "def {ty} \"{name}\"");
        let mut lines = Vec::new();
        if let Some(d) = meta.display_name {
            lines.push(format!("displayName = {}", usd_string(d)));
        }
        if let Some(k) = meta.kind {
            lines.push(format!("kind = {}", usd_string(k)));
        }
        if !meta.api_schemas.is_empty() {
            let list: Vec<String> = meta.api_schemas.iter().map(|s| usd_string(s)).collect();
            lines.push(format!("prepend apiSchemas = [{}]", list.join(", ")));
        }
        if !lines.is_empty() {
            self.text.push_str(" (\n");
            for l in lines {
                self.depth += 1;
                self.indent();
                self.depth -= 1;
                self.text.push_str(&l);
                self.text.push('\n');
            }
            self.indent();
            self.text.push(')');
        }
        self.text.push('\n');
        self.indent();
        self.text.push_str("{\n");
        self.depth += 1;
        Ok(())
    }

    fn end_prim(&mut self) -> Result<()> {
        self.depth -= 1;
        self.indent();
        self.text.push_str("}\n");
        Ok(())
    }

    fn attr(&mut self, a: Attr) -> Result<()> {
        self.indent();
        let o = &mut self.text;
        if a.custom {
            o.push_str("custom ");
        }
        if a.uniform {
            o.push_str("uniform ");
        }
        let _ = write!(o, "{} {}", a.ty, a.name);
        if let Some(t) = a.connect {
            let _ = write!(o, ".connect = <{t}>");
        }
        if !matches!(a.value, Val::None) {
            o.push_str(" = ");
            write_value(o, &a.value);
        }
        if let Some(i) = a.interpolation {
            o.push_str(" (\n");
            let pad = "    ".repeat(self.depth);
            let _ = write!(o, "{pad}    interpolation = \"{i}\"\n{pad})");
        }
        o.push('\n');
        Ok(())
    }

    fn rel(&mut self, name: &str, targets: &[&str], list: bool) -> Result<()> {
        self.indent();
        let t: Vec<String> = targets.iter().map(|t| format!("<{t}>")).collect();
        if list || t.len() != 1 {
            let _ = writeln!(self.text, "rel {name} = [{}]", t.join(", "));
        } else {
            let _ = writeln!(self.text, "rel {name} = {}", t[0]);
        }
        Ok(())
    }
}

fn write_value(o: &mut String, v: &Val) {
    let list = |o: &mut String, n: usize, each: &mut dyn FnMut(&mut String, usize)| {
        o.push('[');
        for i in 0..n {
            if i > 0 {
                o.push_str(", ");
            }
            each(o, i);
        }
        o.push(']');
    };
    let tuple = |o: &mut String, xs: &[f32], d: usize| {
        o.push('(');
        for (k, x) in xs.iter().enumerate() {
            if k > 0 {
                o.push_str(", ");
            }
            push_num(o, *x, d);
        }
        o.push(')');
    };
    match v {
        Val::None => {}
        Val::Token(s) | Val::Str(s) => o.push_str(&usd_string(s)),
        Val::Bool(b) => o.push(if *b { '1' } else { '0' }),
        Val::Float(n) => n.write(o),
        Val::Color(c) => {
            o.push('(');
            for (k, n) in c.iter().enumerate() {
                if k > 0 {
                    o.push_str(", ");
                }
                n.write(o);
            }
            o.push(')');
        }
        Val::Matrix(m) => {
            let rows: Vec<String> = m.iter().map(|r| format!("({}, {}, {}, {})", r[0], r[1], r[2], r[3])).collect();
            let _ = write!(o, "({})", rows.join(", "));
        }
        Val::Ints(xs) => write_ints(o, xs.iter().copied()),
        Val::Floats(xs, d) => list(o, xs.len(), &mut |o, i| push_num(o, xs[i], *d)),
        Val::Vec3s(xs, d) => list(o, xs.len(), &mut |o, i| tuple(o, &xs[i], *d)),
        Val::Vec2s(xs, d) => list(o, xs.len(), &mut |o, i| tuple(o, &xs[i], *d)),
        Val::Quats(xs, d) => list(o, xs.len(), &mut |o, i| tuple(o, &xs[i], *d)),
        Val::Tokens(ts) => list(o, ts.len(), &mut |o, i| o.push_str(&usd_string(ts[i]))),
    }
}

/// A bound material: prim name, DAZ material, and the area (m²) of the faces
/// it covers — emission in lumens or watts is spread over it.
pub type BoundMaterial = (String, Rc<crate::daz_material::Material>, f64);

/// The scene's text layer and the bound materials (`.usda` output and
/// tests; an export writes the binary layer with `write_layer`).
pub(super) fn write_usda(meshes: &[Mesh], lights: &[SceneLight]) -> (String, Vec<BoundMaterial>) {
    let mut w = UsdaWriter::new();
    let materials = write_layer(&mut w, meshes, lights).expect("text layer");
    (w.text, materials)
}

/// Write the scene into `sink`: `Scene` with each mesh (meshes are dropped
/// as soon as they are written when passed by value), the lights and the
/// materials. Returns the bound materials by prim name (`/Scene/Materials/…`)
/// — Octane names its material inputs after them.
pub(super) fn write_layer<M: std::borrow::Borrow<Mesh>>(sink: &mut dyn LayerSink, meshes: impl IntoIterator<Item = M>, lights: &[SceneLight]) -> Result<Vec<BoundMaterial>> {
    sink.begin_prim("Xform", "Scene", &PrimMeta { kind: Some("assembly"), ..Default::default() })?;
    let mut taken = HashSet::new();
    let mut material_names = HashSet::new();
    let mut materials: Vec<BoundMaterial> = Vec::new();
    const BINDING: &[&str] = &["MaterialBindingAPI"];
    for m in meshes {
        let m: &Mesh = m.borrow();
        if let Some(copies) = &m.instances {
            begin_instancer(sink, &m.name, copies)?;
        }
        if let Some(widths) = &m.curve_widths {
            // Strand hair: linear curves, the material bound on the prim.
            let material = m.materials.first().cloned().flatten().map(|mat| {
                let name = unique_name(&mut material_names, &m.name);
                materials.push((name.clone(), mat, 0.0));
                name
            });
            let meta = PrimMeta { display_name: Some(&m.label), api_schemas: if material.is_some() { BINDING } else { &[] }, ..Default::default() };
            sink.begin_prim("BasisCurves", &m.name, &meta)?;
            sink.attr(Attr::new("type", "token", Val::Token("linear")).uniform())?;
            sink.attr(Attr::new("curveVertexCounts", "int[]", Val::Ints(&m.face_counts)))?;
            sink.attr(Attr::new("points", "point3f[]", Val::Vec3s(&m.points, 5)))?;
            sink.attr(Attr::new("widths", "float[]", Val::Floats(widths, 6)).interpolation("vertex"))?;
            if let Some(name) = material {
                sink.rel("material:binding", &[&format!("/Scene/Materials/{name}")], false)?;
            }
            sink.end_prim()?;
        } else {
            write_mesh(sink, m, &mut taken, &mut material_names, &mut materials)?;
        }
        if m.instances.is_some() {
            sink.end_prim()?; // Prototypes
            sink.end_prim()?; // the instancer
        }
    }
    write_lights(sink, lights)?;
    if !materials.is_empty() {
        sink.begin_prim("Scope", "Materials", &PrimMeta::default())?;
        for (name, mat, _) in &materials {
            write_preview_material(sink, name, mat)?;
        }
        sink.end_prim()?;
    }
    sink.end_prim()?;
    Ok(materials)
}

/// A mesh's points numbered in the order its faces first use them (points
/// no face uses after those), its face indices and per-vertex normals to
/// match. The surface stays the same; the indices then mostly step by a
/// few, which the binary layer's integer coding keeps in a byte or two: a
/// refined mesh lists its face, edge and corner points in three runs, and a
/// quad's corners jumped between them (4.0 bytes per index on intake).
pub(super) fn points_in_use_order(m: &Mesh) -> Option<(Vec<[f32; 3]>, Vec<u32>, Option<Vec<[f32; 3]>>)> {
    let n = m.points.len();
    if m.face_indices.iter().any(|&i| i as usize >= n) {
        return None;
    }
    let mut new_of = vec![u32::MAX; n];
    let mut order: Vec<u32> = Vec::with_capacity(n);
    let indices = m
        .face_indices
        .iter()
        .map(|&i| {
            let slot = &mut new_of[i as usize];
            if *slot == u32::MAX {
                *slot = order.len() as u32;
                order.push(i);
            }
            *slot
        })
        .collect();
    order.extend((0..n as u32).filter(|&i| new_of[i as usize] == u32::MAX));
    let points = order.iter().map(|&o| m.points[o as usize]).collect();
    let normals = match (&m.normals, &m.normal_indices) {
        (Some(nv), None) if nv.len() == n => Some(order.iter().map(|&o| nv[o as usize]).collect()),
        _ => None,
    };
    Some((points, indices, normals))
}

fn write_mesh(sink: &mut dyn LayerSink, m: &Mesh, taken: &mut HashSet<String>, material_names: &mut HashSet<String>, materials: &mut Vec<BoundMaterial>) -> Result<()> {
    sink.begin_prim("Mesh", &m.name, &PrimMeta { display_name: Some(&m.label), ..Default::default() })?;
    sink.attr(Attr::new("doubleSided", "bool", Val::Bool(false)).uniform())?;
    sink.attr(Attr::new("faceVertexCounts", "int[]", Val::Ints(&m.face_counts)))?;
    let ordered = points_in_use_order(m);
    let (points, indices, normals) = match &ordered {
        Some((p, i, n)) => (p, i, n.as_ref().or(m.normals.as_ref())),
        None => (&m.points, &m.face_indices, m.normals.as_ref()),
    };
    sink.attr(Attr::new("faceVertexIndices", "int[]", Val::Ints(indices)))?;
    sink.attr(Attr::new("points", "point3f[]", Val::Vec3s(points, 5)))?;
    if let Some(n) = normals {
        // `primvars:normals`: Octane's USD import reads that one (the
        // mesh node's `normals` + `polyNormalIndices`) and computes its
        // own at the import's maxSmoothAngle without it — plain
        // `normals` it ignored (Calib-Smooth probe, #40).
        match &m.normal_indices {
            // Split along hard edges (`smoothed_normals`): indexed per corner.
            Some(idx) => {
                sink.attr(Attr::new("primvars:normals", "normal3f[]", Val::Vec3s(n, 4)).interpolation("faceVarying"))?;
                sink.attr(Attr::new("primvars:normals:indices", "int[]", Val::Ints(idx)))?;
            }
            None => sink.attr(Attr::new("primvars:normals", "normal3f[]", Val::Vec3s(n, 4)).interpolation("vertex"))?,
        }
    }
    if let Some((vals, idx)) = &m.uvs {
        sink.attr(Attr::new("primvars:st", "texCoord2f[]", Val::Vec2s(vals, 6)).interpolation("faceVarying"))?;
        sink.attr(Attr::new("primvars:st:indices", "int[]", Val::Ints(idx)))?;
    }
    // Subdivision surfaces arrive refined (`subdivide_once`).
    sink.attr(Attr::new("subdivisionScheme", "token", Val::Token("none")).uniform())?;
    if !m.subsets.is_empty() {
        sink.attr(Attr::new("subsetFamily:materialBind:familyType", "token", Val::Token("partition")).uniform())?;
    }
    taken.clear();
    for (i, (group, faces)) in m.subsets.iter().enumerate() {
        let subset = unique_name(taken, group);
        let material = m.materials.get(i).cloned().flatten().map(|mat| {
            let name = unique_name(material_names, &format!("{}_{}", m.name, subset));
            materials.push((name.clone(), mat, faces_area(m, faces)));
            name
        });
        let meta = PrimMeta { display_name: Some(group), api_schemas: if material.is_some() { &["MaterialBindingAPI"] } else { &[] }, ..Default::default() };
        sink.begin_prim("GeomSubset", &subset, &meta)?;
        sink.attr(Attr::new("elementType", "token", Val::Token("face")).uniform())?;
        sink.attr(Attr::new("familyName", "token", Val::Token("materialBind")).uniform())?;
        sink.attr(Attr::new("indices", "int[]", Val::Ints(faces)))?;
        if let Some(name) = material {
            sink.rel("material:binding", &[&format!("/Scene/Materials/{name}")], false)?;
        }
        sink.end_prim()?;
    }
    sink.end_prim()
}

/// Open `<name>_copies`, a `PointInstancer` drawing the prim that follows
/// (named `name`, closed by the caller: two `end_prim`) at `copies`.
fn begin_instancer(sink: &mut dyn LayerSink, name: &str, copies: &[Placement]) -> Result<()> {
    let parts: Vec<_> = copies.iter().map(decompose).collect();
    let instancer = format!("{name}_copies");
    sink.begin_prim("PointInstancer", &instancer, &PrimMeta::default())?;
    sink.rel("prototypes", &[&format!("/Scene/{instancer}/Prototypes/{name}")], true)?;
    sink.attr(Attr::new("protoIndices", "int[]", Val::Ints(&vec![0; copies.len()])))?;
    sink.attr(Attr::new("positions", "point3f[]", Val::Vec3s(&parts.iter().map(|p| p.0).collect::<Vec<_>>(), 5)))?;
    sink.attr(Attr::new("orientations", "quath[]", Val::Quats(&parts.iter().map(|p| p.1).collect::<Vec<_>>(), 5)))?;
    sink.attr(Attr::new("scales", "float3[]", Val::Vec3s(&parts.iter().map(|p| p.2).collect::<Vec<_>>(), 5)))?;
    sink.begin_prim("Scope", "Prototypes", &PrimMeta::default())
}

/// Each material's UV scale: meters of surface per UV unit, the square root
/// of its faces' area over their UV area (all meshes it covers), keyed by
/// the material. Octane's bump slope goes by it (#75); none without UVs.
pub(super) fn uv_scales(meshes: &[Mesh]) -> crate::material::UvScales {
    let mut sums: HashMap<*const crate::daz_material::Material, (f64, f64)> = HashMap::new();
    for m in meshes.iter().filter(|m| m.curve_widths.is_none()) {
        for (i, (_, faces)) in m.subsets.iter().enumerate() {
            let (Some(Some(mat)), Some(uv)) = (m.materials.get(i), faces_uv_area(m, faces)) else { continue };
            let s = sums.entry(Rc::as_ptr(mat)).or_default();
            s.0 += faces_area(m, faces);
            s.1 += uv;
        }
    }
    sums.into_iter().filter(|(_, (a, uv))| *a > 0.0 && *uv > 0.0).map(|(k, (a, uv))| (k, (a / uv).sqrt())).collect()
}

/// Total UV area of some faces of a mesh (fan triangulation); `None`
/// without UVs.
pub(super) fn faces_uv_area(m: &Mesh, faces: &[u32]) -> Option<f64> {
    let (vals, idx) = m.uvs.as_ref()?;
    let mut starts = Vec::with_capacity(m.face_counts.len());
    let mut at = 0usize;
    for &c in &m.face_counts {
        starts.push(at);
        at += c as usize;
    }
    let uv = |corner: usize| vals[idx[corner] as usize].map(|v| v as f64);
    let mut area = 0.0;
    for &f in faces {
        let (s, c) = (starts[f as usize], m.face_counts[f as usize] as usize);
        for k in 1..c.saturating_sub(1) {
            let (a, b, d) = (uv(s), uv(s + k), uv(s + k + 1));
            area += 0.5 * ((b[0] - a[0]) * (d[1] - a[1]) - (b[1] - a[1]) * (d[0] - a[0])).abs();
        }
    }
    Some(area)
}

/// Total area (m²) of some faces of a mesh (fan triangulation).
pub(super) fn faces_area(m: &Mesh, faces: &[u32]) -> f64 {
    let mut starts = Vec::with_capacity(m.face_counts.len());
    let mut at = 0usize;
    for &c in &m.face_counts {
        starts.push(at);
        at += c as usize;
    }
    let p = |i: u32| m.points[i as usize].map(|v| v as f64);
    let mut area = 0.0;
    for &f in faces {
        let (s, c) = (starts[f as usize], m.face_counts[f as usize] as usize);
        let v = &m.face_indices[s..s + c];
        for k in 1..c.saturating_sub(1) {
            let (a, b, d) = (p(v[0]), p(v[k]), p(v[k + 1]));
            let (u, w) = ([0, 1, 2].map(|i| b[i] - a[i]), [0, 1, 2].map(|i| d[i] - a[i]));
            let cr = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
            area += 0.5 * (cr[0] * cr[0] + cr[1] * cr[1] + cr[2] * cr[2]).sqrt();
        }
    }
    area
}

/// A `UsdPreviewSurface` approximation of a DAZ material (constants only —
/// Octane gets the full material with textures from `project.ocs`).
pub(super) fn write_preview_material(sink: &mut dyn LayerSink, name: &str, mat: &crate::daz_material::Material) -> Result<()> {
    let c = mat.base_color().map(|v| Num::Trim(v as f64, 4));
    let surface = format!("/Scene/Materials/{name}/Surface.outputs:surface");
    sink.begin_prim("Material", name, &PrimMeta::default())?;
    sink.attr(Attr::new("outputs:surface", "token", Val::None).connect(&surface))?;
    sink.begin_prim("Shader", "Surface", &PrimMeta::default())?;
    sink.attr(Attr::new("info:id", "token", Val::Token("UsdPreviewSurface")).uniform())?;
    sink.attr(Attr::new("inputs:diffuseColor", "color3f", Val::Color(c)))?;
    let float = |x: f32| Val::Float(Num::Trim(x.clamp(0.0, 1.0) as f64, 4));
    sink.attr(Attr::new("inputs:roughness", "float", float(mat.roughness())))?;
    sink.attr(Attr::new("inputs:metallic", "float", float(crate::iray::Uber::of(mat).and_then(|u| u.float("metallic_weight")).unwrap_or_else(|| mat.metallic()))))?;
    sink.attr(Attr::new("inputs:opacity", "float", float(mat.opacity())))?;
    sink.attr(Attr::new("outputs:surface", "token", Val::None))?;
    sink.end_prim()?;
    sink.end_prim()
}
