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

pub(super) fn write_vec3(out: &mut String, xs: &[[f32; 3]], decimals: usize) {
    out.push('[');
    for (i, p) in xs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('(');
        for (k, v) in p.iter().enumerate() {
            if k > 0 {
                out.push_str(", ");
            }
            push_num(out, *v, decimals);
        }
        out.push(')');
    }
    out.push(']');
}

pub(super) fn write_vec2(out: &mut String, xs: &[[f32; 2]]) {
    out.push('[');
    for (i, p) in xs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('(');
        push_num(out, p[0], 6);
        out.push_str(", ");
        push_num(out, p[1], 6);
        out.push(')');
    }
    out.push(']');
}

/// The USD layer, and the bound materials by prim name (`/Scene/Materials/…`)
/// — Octane names its material inputs after them.
/// A bound material: prim name, DAZ material, and the area (m²) of the faces
/// it covers — emission in lumens or watts is spread over it.
pub type BoundMaterial = (String, Rc<crate::daz_material::Material>, f64);

pub(super) fn write_usda(meshes: &[Mesh], lights: &[SceneLight]) -> (String, Vec<BoundMaterial>) {
    let mut o = String::new();
    o.push_str("#usda 1.0\n(\n    defaultPrim = \"Scene\"\n    metersPerUnit = 1\n    upAxis = \"Y\"\n    doc = \"Exported by ORBIS\"\n)\n\n");
    o.push_str("def Xform \"Scene\" (\n    kind = \"assembly\"\n)\n{\n");
    let mut taken = HashSet::new();
    let mut material_names = HashSet::new();
    let mut materials: Vec<BoundMaterial> = Vec::new();
    for m in meshes {
        if let Some(copies) = &m.instances {
            begin_instancer(&mut o, &m.name, copies);
        }
        if let Some(widths) = &m.curve_widths {
            // Strand hair: linear curves, the material bound on the prim.
            let material = m.materials.first().cloned().flatten().map(|mat| {
                let name = unique_name(&mut material_names, &m.name);
                materials.push((name.clone(), mat, 0.0));
                name
            });
            let _ = writeln!(
                o,
                "    def BasisCurves \"{}\" (\n        displayName = {}{}\n    )\n    {{",
                m.name,
                usd_string(&m.label),
                if material.is_some() { "\n        prepend apiSchemas = [\"MaterialBindingAPI\"]" } else { "" }
            );
            o.push_str("        uniform token type = \"linear\"\n        int[] curveVertexCounts = ");
            write_ints(&mut o, m.face_counts.iter().copied());
            o.push_str("\n        point3f[] points = ");
            write_vec3(&mut o, &m.points, 5);
            o.push_str("\n        float[] widths = [");
            for (k, w) in widths.iter().enumerate() {
                if k > 0 {
                    o.push_str(", ");
                }
                push_num(&mut o, *w, 6);
            }
            o.push_str("] (\n            interpolation = \"vertex\"\n        )\n");
            if let Some(name) = material {
                let _ = writeln!(o, "        rel material:binding = </Scene/Materials/{name}>");
            }
            o.push_str("    }\n\n");
            if m.instances.is_some() {
                o.push_str("    }\n    }\n\n");
            }
            continue;
        }
        let _ = writeln!(o, "    def Mesh \"{}\" (\n        displayName = {}\n    )\n    {{", m.name, usd_string(&m.label));
        o.push_str("        uniform bool doubleSided = 0\n");
        o.push_str("        int[] faceVertexCounts = ");
        write_ints(&mut o, m.face_counts.iter().copied());
        o.push_str("\n        int[] faceVertexIndices = ");
        write_ints(&mut o, m.face_indices.iter().copied());
        o.push_str("\n        point3f[] points = ");
        write_vec3(&mut o, &m.points, 5);
        o.push('\n');
        if let Some(n) = &m.normals {
            o.push_str("        normal3f[] normals = ");
            write_vec3(&mut o, n, 4);
            o.push_str(" (\n            interpolation = \"vertex\"\n        )\n");
        }
        if let Some((vals, idx)) = &m.uvs {
            o.push_str("        texCoord2f[] primvars:st = ");
            write_vec2(&mut o, vals);
            o.push_str(" (\n            interpolation = \"faceVarying\"\n        )\n        int[] primvars:st:indices = ");
            write_ints(&mut o, idx.iter().copied());
            o.push('\n');
        }
        // Subdivision surfaces arrive refined (`subdivide_once`).
        o.push_str("        uniform token subdivisionScheme = \"none\"\n");
        if !m.subsets.is_empty() {
            o.push_str("        uniform token subsetFamily:materialBind:familyType = \"partition\"\n");
        }
        taken.clear();
        for (i, (group, faces)) in m.subsets.iter().enumerate() {
            let subset = unique_name(&mut taken, group);
            let material = m.materials.get(i).cloned().flatten().map(|mat| {
                let name = unique_name(&mut material_names, &format!("{}_{}", m.name, subset));
                materials.push((name.clone(), mat, faces_area(m, faces)));
                name
            });
            let _ = writeln!(
                o,
                "\n        def GeomSubset \"{subset}\" (\n            displayName = {}{}\n        )\n        {{",
                usd_string(group),
                if material.is_some() { "\n            prepend apiSchemas = [\"MaterialBindingAPI\"]" } else { "" }
            );
            o.push_str("            uniform token elementType = \"face\"\n            uniform token familyName = \"materialBind\"\n            int[] indices = ");
            write_ints(&mut o, faces.iter().copied());
            if let Some(name) = material {
                let _ = write!(o, "\n            rel material:binding = </Scene/Materials/{name}>");
            }
            o.push_str("\n        }\n");
        }
        o.push_str("    }\n\n");
        if m.instances.is_some() {
            o.push_str("    }\n    }\n\n");
        }
    }
    write_lights(&mut o, lights);
    if !materials.is_empty() {
        o.push_str("    def Scope \"Materials\"\n    {\n");
        for (name, mat, _) in &materials {
            write_preview_material(&mut o, name, mat);
        }
        o.push_str("    }\n");
    }
    o.push_str("}\n");
    (o, materials)
}

/// Open `<name>_copies`, a `PointInstancer` drawing the prim that follows
/// (named `name`, closed by the caller with two braces) at `copies`.
fn begin_instancer(o: &mut String, name: &str, copies: &[Placement]) {
    let parts: Vec<_> = copies.iter().map(decompose).collect();
    let _ = writeln!(o, "    def PointInstancer \"{name}_copies\"\n    {{");
    let _ = writeln!(o, "        rel prototypes = [</Scene/{name}_copies/Prototypes/{name}>]");
    o.push_str("        int[] protoIndices = ");
    write_ints(o, copies.iter().map(|_| 0));
    o.push_str("\n        point3f[] positions = ");
    write_vec3(o, &parts.iter().map(|p| p.0).collect::<Vec<_>>(), 5);
    o.push_str("\n        quath[] orientations = [");
    for (i, (_, q, _)) in parts.iter().enumerate() {
        if i > 0 {
            o.push_str(", ");
        }
        o.push('(');
        for (k, v) in q.iter().enumerate() {
            if k > 0 {
                o.push_str(", ");
            }
            push_num(o, *v, 5);
        }
        o.push(')');
    }
    o.push_str("]\n        float3[] scales = ");
    write_vec3(o, &parts.iter().map(|p| p.2).collect::<Vec<_>>(), 5);
    o.push_str("\n\n        def Scope \"Prototypes\"\n        {\n");
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
pub(super) fn write_preview_material(o: &mut String, name: &str, mat: &crate::daz_material::Material) {
    let c = mat.base_color();
    let _ = writeln!(o, "        def Material \"{name}\"\n        {{");
    let _ = writeln!(o, "            token outputs:surface.connect = </Scene/Materials/{name}/Surface.outputs:surface>");
    o.push_str("            def Shader \"Surface\"\n            {\n");
    o.push_str("                uniform token info:id = \"UsdPreviewSurface\"\n");
    o.push_str("                color3f inputs:diffuseColor = (");
    for (k, v) in c.iter().enumerate() {
        if k > 0 {
            o.push_str(", ");
        }
        push_num(o, *v, 4);
    }
    o.push_str(")\n                float inputs:roughness = ");
    push_num(o, mat.roughness().clamp(0.0, 1.0), 4);
    o.push_str("\n                float inputs:metallic = ");
    push_num(o, mat.metallic().clamp(0.0, 1.0), 4);
    o.push_str("\n                float inputs:opacity = ");
    push_num(o, mat.opacity().clamp(0.0, 1.0), 4);
    o.push_str("\n                token outputs:surface\n            }\n        }\n");
}
