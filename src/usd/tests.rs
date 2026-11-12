use super::*;
use serde_json::json;

#[test]
fn usdz_data_is_64_byte_aligned_and_readable() {
    let mut buf = Vec::new();
    let files: Vec<(&str, &[u8])> =
        vec![("scene.usda", b"#usda 1.0\n"), ("textures/a.png", &[1, 2, 3]), ("b.jpg", &[9; 100])];
    write_usdz(&mut buf, &files).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(buf)).unwrap();
    assert_eq!(zip.len(), 3);
    for (i, (name, data)) in files.iter().enumerate() {
        let mut f = zip.by_index(i).unwrap();
        assert_eq!(f.name().unwrap(), *name);
        assert_eq!(f.compression(), zip::CompressionMethod::Stored);
        assert_eq!(f.data_start().unwrap() % 64, 0, "{name} misaligned");
        let mut got = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut got).unwrap();
        assert_eq!(&got, data);
    }
}

fn quad_geometry(id: &str, groups: &[&str], polys: Value, vcount: usize) -> Value {
    json!({
        "id": id, "type": "polygon_mesh",
        "vertices": {"count": vcount, "values": vec![[0.0, 0.0, 0.0]; vcount]},
        "polygon_material_groups": {"count": groups.len(), "values": groups},
        "polylist": {"count": 1, "values": polys},
    })
}

/// A host (2 quads) with a graft hiding its 2nd quad, and a shell over
/// both — all as local scene geometry.
fn scene() -> Value {
    let mut graft = quad_geometry("g", &["Patch"], json!([[0, 0, 0, 1, 2, 3]]), 4);
    graft["graft"] = json!({"vertex_count": 6, "poly_count": 2, "vertex_pairs": {"count": 1, "values": [[0, 1]]}, "hidden_polys": {"count": 1, "values": [1]}});
    let shell = json!({"id": "s", "vertices": {"count": 0, "values": []}, "polylist": {"count": 0, "values": []},
                       "extra": [{"type": "studio/geometry/shell"}]});
    json!({
        "geometry_library": [
            quad_geometry("h", &["Torso", "Legs"], json!([[0, 0, 0, 1, 2, 3], [0, 1, 2, 3, 4, 5]]), 6),
            graft, shell,
        ],
        "uv_set_library": [
            {"id": "huv", "vertex_count": 6, "uvs": {"count": 7, "values": [[0,0],[1,0],[1,1],[0,1],[2,0],[2,1],[0.5,0.5]]},
             "polygon_vertex_indices": [[0, 3, 6]]},
        ],
        "scene": {
            "nodes": [
                {"id": "Host", "name": "Host", "label": "Body", "geometries": [{"id": "hi", "url": "#h", "type": "subdivision_surface"}]},
                {"id": "Graft", "name": "Graft", "label": "Bits", "parent": "#Host", "conform_target": "#Host",
                 "geometries": [{"id": "gi", "url": "#g"}]},
                {"id": "Shell", "name": "Shell", "label": "Body Shell", "parent": "#Host",
                 "geometries": [{"id": "si", "url": "#s", "extra": [{"type": "studio/geometry/shell"}]}]},
            ],
            "materials": [
                {"geometry": "#hi", "groups": ["Torso", "Legs"], "uv_set": "#huv",
                 "diffuse": {"channel": {"id": "diffuse", "current_value": [1, 0.5, 0.25]}}},
            ],
        },
    })
}

fn dump() -> SceneDump {
    let verts = |n: usize, y: f32| (0..n).map(|i| [i as f32, y, 0.0]).collect::<Vec<_>>();
    SceneDump {
        nodes: vec![
            // lod 0: not refined, so the assembly stays checkable.
            DumpNode { name: "Host".into(), label: "Body".into(), lod: 0, vertices: verts(6, 0.0), ..Default::default() },
            DumpNode { name: "Graft".into(), label: "Bits".into(), lod: 0, vertices: verts(4, 1.0), ..Default::default() },
            DumpNode { name: "Shell".into(), label: "Body Shell".into(), lod: 0, vertices: verts(10, 2.0), ..Default::default() },
        ],
        ..Default::default()
    }
}

#[test]
fn builds_host_graft_and_shell_meshes() {
    let mut lib = Library::new(vec![], scene());
    let (meshes, report) = build_meshes(&mut lib, &dump()).unwrap();
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    // The graft is merged into its host's mesh: Body + Body Shell.
    assert_eq!(meshes.len(), 2);

    // Host: the grafted-over quad is hidden; UVs from the material's set,
    // with the polygon override applied to vertex 3 of quad 0. The graft
    // quad follows, shifted by the host's 6 vertices, its vertex 0 welded
    // onto host vertex 1.
    let host = &meshes[0];
    assert_eq!(host.name, "Body");
    assert_eq!(host.face_counts, vec![4, 4]);
    assert_eq!(host.face_indices, vec![0, 1, 2, 3, 1, 7, 8, 9]);
    assert_eq!(host.points.len(), 10);
    assert_eq!(host.points[6], [0.0, 0.01, 0.0], "graft points follow the host's");
    assert_eq!(
        host.subsets,
        vec![("Torso".to_string(), vec![0]), ("Graft_Patch".to_string(), vec![1])]
    );
    assert!(host.materials[0].is_some() && host.materials[1].is_none());
    let (vals, idx) = host.uvs.as_ref().unwrap();
    assert_eq!(&idx[..4], &[0, 1, 2, 6]);
    assert_eq!(vals[6], [0.5, 0.5]);
    assert!(host.normals.is_some(), "every mesh carries smooth normals");

    // Shell: host (minus hidden) + graft shifted by the host's 6 vertices
    // and welded like the host, graft groups prefixed by its name.
    let shell = &meshes[1];
    assert_eq!(shell.face_indices, vec![0, 1, 2, 3, 1, 7, 8, 9]);
    let groups: Vec<_> = shell.subsets.iter().map(|(g, _)| g.as_str()).collect();
    assert_eq!(groups, ["Torso", "Graft_Patch"]);
    assert_eq!(shell.points[0], [0.0, 0.02, 0.0], "centimeters → meters");
}

#[test]
fn shell_surface_visibility_and_hidden_nodes() {
    let mut duf = scene();
    let vis = |id: &str, on: bool| json!({"channel": {"id": id, "current_value": on}});
    // The shell shows only the graft's surface; the graft node is hidden.
    duf["scene"]["nodes"][2]["extra"] = json!([{"type": "studio_node_channels", "channels": [
        vis("Visible", true), vis("material_group_Torso_vis", false), vis("material_group_Graft_Patch_vis", true),
    ]}]);
    duf["scene"]["nodes"][1]["extra"] = json!([{"type": "studio_node_channels", "channels": [vis("Visible", false)]}]);
    let mut lib = Library::new(vec![], duf);
    let (meshes, report) = build_meshes(&mut lib, &dump()).unwrap();
    assert_eq!(report.skipped, vec!["Bits: hidden in the scene".to_string()]);
    let names: Vec<&str> = meshes.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["Body", "Body_Shell"]);
    // Hidden graft still shapes the host and the shell (DAZ keeps it attached).
    let shell = &meshes[1];
    // The patch: the host's vertex 1 (welded) and the graft's 7–9, the
    // points it uses renumbered.
    assert_eq!(shell.face_indices, vec![0, 1, 2, 3]);
    let xs: Vec<f32> = shell.points.iter().map(|p| (p[0] / CM).round()).collect();
    assert_eq!(xs, [1.0, 7.0, 8.0, 9.0]);
    assert_eq!(shell.subsets, vec![("Graft_Patch".to_string(), vec![0])]);
}

/// The host alone (no graft, no shell), its quads in facet groups Hip and
/// Thigh, with these node channels.
fn host_with_channels(channels: Value) -> Vec<Mesh> {
    let mut duf = scene();
    duf["geometry_library"][0]["polygon_groups"] = json!({"count": 2, "values": ["Hip", "Thigh"]});
    duf["geometry_library"][0]["polylist"]["values"] = json!([[0, 0, 0, 1, 2, 3], [1, 1, 1, 2, 3, 4]]);
    duf["scene"]["nodes"] = json!([duf["scene"]["nodes"][0].clone()]);
    duf["scene"]["nodes"][0]["extra"] = json!([{"type": "studio_node_channels", "channels": channels}]);
    let mut lib = Library::new(vec![], duf);
    build_meshes(&mut lib, &dump()).unwrap().0
}

#[test]
fn hidden_surfaces_facets_and_unrenderable_nodes() {
    let vis = |id: &str, on: bool| json!({"channel": {"id": id, "current_value": on}});
    let subsets = |m: &[Mesh]| m.iter().flat_map(|m| m.subsets.iter().map(|(g, f)| (g.clone(), f.len()))).collect::<Vec<_>>();
    assert_eq!(subsets(&host_with_channels(json!([]))), [("Torso".to_string(), 1), ("Legs".to_string(), 1)]);
    // A surface switched off on a figure (not only on a shell).
    let m = host_with_channels(json!([vis("material_group_Legs_vis", false), vis("material_group_Torso_vis", true)]));
    assert_eq!(subsets(&m), [("Torso".to_string(), 1)]);
    assert_eq!(m[0].points.len(), 4, "the points of the cut faces go");
    // A facet group switched off (Geometry Editor / Mesh Visibility).
    assert_eq!(subsets(&host_with_channels(json!([vis("facet_group_Hip_vis", false)]))), [("Legs".to_string(), 1)]);
    // Renderable off: not rendered, like Visible off.
    assert!(host_with_channels(json!([vis("Renderable", false)])).is_empty());
}

#[test]
fn shell_hides_a_grafts_own_facet_group() {
    // NGV8's Effect Layer: `WG_0_Rectum` off hides the graft's Rectum
    // facets; its Hip facets are `WG_0_Hip`, apart from the host's Hip.
    let graft = Topology { facet_groups: vec!["Hip".into(), "Rectum".into()], ..Default::default() };
    assert_eq!(mesh::graft_facets(&graft, 0), ["WG_0_Hip", "WG_0_Rectum"]);

    let mut duf = scene();
    duf["geometry_library"][1]["polygon_groups"] = json!({"count": 1, "values": ["Inner"]});
    let vis = |id: &str, on: bool| json!({"channel": {"id": id, "current_value": on}});
    duf["scene"]["nodes"][2]["extra"] = json!([{"type": "studio_node_channels", "channels": [
        vis("material_group_Torso_vis", true), vis("material_group_Graft_Patch_vis", true), vis("facet_group_WG_0_Inner_vis", false),
    ]}]);
    let mut lib = Library::new(vec![], duf);
    let (meshes, _) = build_meshes(&mut lib, &dump()).unwrap();
    assert_eq!(meshes[1].subsets, vec![("Torso".to_string(), vec![0])]);
}

#[test]
fn shell_refines_with_its_host() {
    // DAZ reports the shell at level 0; it renders at the host's SubD.
    let mut d = dump();
    d.nodes[0].lod = 1;
    let mut lib = Library::new(vec![], scene());
    let (meshes, _) = build_meshes(&mut lib, &d).unwrap();
    assert_eq!(meshes[0].face_counts.len(), 8, "host: 2 quads refined");
    assert_eq!(meshes[1].face_counts.len(), 8, "shell refined like its host");
}

#[test]
fn host_dumped_with_its_grafts_appended() {
    // DAZ hands out the host's cache with the graft's 4 vertices after its 6.
    let mut d = dump();
    let graft = d.nodes[1].vertices.clone();
    d.nodes[0].vertices.extend(graft);
    let mut lib = Library::new(vec![], scene());
    let (meshes, report) = build_meshes(&mut lib, &d).unwrap();
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    let (expected, _) = build_meshes(&mut Library::new(vec![], scene()), &dump()).unwrap();
    assert_eq!(meshes[0].points, expected[0].points);
}

#[test]
fn vertex_count_mismatch_is_skipped() {
    let mut d = dump();
    d.nodes[1].vertices.pop();
    let mut lib = Library::new(vec![], scene());
    let (meshes, report) = build_meshes(&mut lib, &d).unwrap();
    assert_eq!(meshes.len(), 2);
    assert_eq!(report.skipped, vec!["Bits: the dump has 3 vertices, geometry 4".to_string()]);
}

#[test]
fn usda_has_stage_metadata_subsets_and_indexed_uvs() {
    let mut lib = Library::new(vec![], scene());
    let (meshes, _) = build_meshes(&mut lib, &dump()).unwrap();
    let (usda, _) = write_usda(&meshes, &[]);
    assert!(usda.starts_with("#usda 1.0\n"));
    assert!(usda.contains("metersPerUnit = 1\n") && usda.contains("upAxis = \"Y\""));
    assert!(usda.contains("def Mesh \"Body_Shell\""));
    // Host corners use its UV set; the merged graft (no UV set) a (0, 0).
    assert!(usda.contains("int[] primvars:st:indices = [0, 1, 2, 6, 7, 7, 7, 7]"));
    assert!(usda.contains("def GeomSubset \"Graft_Patch\""));
    // The host's Torso subset is bound to a preview material from DAZ.
    assert!(usda.contains("rel material:binding = </Scene/Materials/Body_Torso>"));
    assert!(usda.contains("def Material \"Body_Torso\""));
    assert!(usda.contains("color3f inputs:diffuseColor = (1, 0.2176, 0.0474)"), "linear base colour");
    assert!(usda.contains("uniform token subdivisionScheme = \"none\""));
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-5)
}

#[test]
fn subdivides_a_cube_by_catmull_clark() {
    let c = [-1.0, 1.0];
    let points = (0..8).map(|i| [c[i & 1 ^ (i >> 1 & 1)], c[i >> 1 & 1], c[i >> 2]]).collect::<Vec<_>>();
    assert_eq!(points[6], [1.0, 1.0, 1.0]);
    let faces = [[0, 3, 2, 1], [4, 5, 6, 7], [0, 1, 5, 4], [2, 3, 7, 6], [1, 2, 6, 5], [0, 4, 7, 3]];
    let m = Mesh {
        points,
        face_counts: vec![4; 6],
        face_indices: faces.concat(),
        subsets: vec![("A".into(), vec![0]), ("B".into(), vec![1, 2, 3, 4, 5])],
        materials: vec![None, None],
        ..Default::default()
    };
    let s = subdivide_once(&m);
    assert_eq!((s.points.len(), s.face_counts.len()), (8 + 6 + 12, 24));
    assert!(s.face_counts.iter().all(|&c| c == 4));
    // A corner of valence 3: (Q + 2R) / 3 = 5/9 per axis.
    assert!(close(s.points[6], [5.0 / 9.0; 3]), "{:?}", s.points[6]);
    // Face points, then the edge 2–6: (ends + both face points) / 4.
    assert!(close(s.points[8], [0.0, 0.0, -1.0]));
    assert!(s.points.iter().any(|&p| close(p, [0.75, 0.75, 0.0])));
    // Each face's quads keep its subset.
    assert_eq!(s.subsets, vec![("A".into(), vec![0, 1, 2, 3]), ("B".into(), (4..24).collect())]);
    assert_eq!(&s.face_indices[..4], &[0, s.face_indices[1], 8, s.face_indices[3]]);
}

#[test]
fn subdivision_keeps_corners_and_follows_boundaries() {
    // Two quads in a strip; the shared bottom vertex 1 sits raised.
    let m = Mesh {
        points: vec![[0.0, 0.0, 0.0], [1.0, 0.4, 0.0], [2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [2.0, 1.0, 0.0]],
        face_counts: vec![4, 4],
        face_indices: vec![0, 1, 4, 3, 1, 2, 5, 4],
        uvs: Some((vec![[0.0, 0.0], [0.5, 0.0], [1.0, 0.0], [0.0, 1.0], [0.5, 1.0], [1.0, 1.0]], vec![0, 1, 4, 3, 1, 2, 5, 4])),
        subsets: vec![("A".into(), vec![0, 1])],
        materials: vec![None],
        ..Default::default()
    };
    let s = subdivide_once(&m);
    assert_eq!(s.points[0], [0.0, 0.0, 0.0], "a corner stays");
    // Along the boundary: (neighbours + 6 P) / 8.
    assert!(close(s.points[1], [1.0, 0.3, 0.0]), "{:?}", s.points[1]);
    // The boundary edge 0–1 is its midpoint.
    assert!(s.points.iter().any(|&p| close(p, [0.5, 0.2, 0.0])));
    // UVs split linearly: the first quad's corner, edge midpoint, centre,
    // edge midpoint; the edge 1–4 shared with the second quad.
    let (vals, idx) = s.uvs.as_ref().unwrap();
    assert_eq!(idx.len(), 8 * 4);
    let uv = |k: usize| vals[idx[k] as usize];
    assert_eq!([uv(0), uv(1), uv(2), uv(3)], [[0.0, 0.0], [0.25, 0.0], [0.25, 0.5], [0.0, 0.5]]);
    assert_eq!(idx.iter().filter(|&&i| vals[i as usize] == [0.5, 0.5]).count(), 4, "shared 1–4 midpoint");
}

#[test]
fn lifts_a_skin_tight_item_off_its_figure() {
    // Host: a 1 cm grid in the y = 0 plane, facing +y.
    let mut host = Mesh::default();
    for z in 0..3 {
        for x in 0..3 {
            host.points.push([x as f32 * 0.01, 0.0, z as f32 * 0.01]);
        }
    }
    for z in 0..2u32 {
        for x in 0..2u32 {
            let a = z * 3 + x;
            host.face_indices.extend([a, a + 3, a + 4, a + 1]);
            host.face_counts.push(4);
        }
    }
    assert!(vertex_normals(&host.points, &host.face_counts, &host.face_indices)[4][1] > 0.99, "faces +y");
    let mut guest = Mesh { points: vec![[0.01, -0.0003, 0.01], [0.01, 0.0002, 0.01], [0.01, 0.002, 0.01], [0.01, -0.02, 0.01]], ..Default::default() };
    assert_eq!(lift_off(&mut guest, &host), 2);
    assert!((guest.points[0][1] - LIFT_CLEARANCE).abs() < 1e-6, "under the skin → just above");
    assert!((guest.points[1][1] - LIFT_CLEARANCE).abs() < 1e-6, "too close → just above");
    assert_eq!(guest.points[2][1], 0.002, "clear of it: stays");
    assert_eq!(guest.points[3][1], -0.02, "out of reach: stays");
}

#[test]
fn strand_hair_is_written_as_basis_curves() {
    let mut m = Mesh { name: "Curls_Bangs".into(), label: "Curls Bangs".into(), ..Default::default() };
    m.points = vec![[0.0, 1.0, 0.0], [0.0, 0.9, 0.0], [0.0, 0.8, 0.0], [0.1, 1.0, 0.0], [0.1, 0.9, 0.0]];
    m.face_counts = vec![3, 2];
    m.subsets = vec![("Bangs".into(), vec![0, 1])];
    m.materials = vec![None];
    m.curve_widths = Some(vec![0.0004, 0.0003, 0.0002, 0.0004, 0.0002]);
    let (usda, mats) = write_usda(&[m], &[]);
    assert!(usda.contains("def BasisCurves \"Curls_Bangs\""), "{usda}");
    assert!(usda.contains("uniform token type = \"linear\""));
    assert!(usda.contains("int[] curveVertexCounts = [3, 2]"));
    assert!(usda.contains("float[] widths = [0.0004, 0.0003, 0.0002, 0.0004, 0.0002] (\n            interpolation = \"vertex\""));
    assert!(!usda.contains("faceVertexCounts") && mats.is_empty());
}

#[test]
fn points_follow_the_faces_that_use_them() {
    let mut m = Mesh { points: vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0], [4.0, 0.0, 0.0]], ..Default::default() };
    m.face_counts = vec![3, 3];
    m.face_indices = vec![3, 1, 4, 4, 1, 0];
    m.normals = Some((0..5).map(|k| [k as f32, 1.0, 0.0]).collect());
    let (points, indices, normals) = points_in_use_order(&m).unwrap();
    assert_eq!(indices, [0, 1, 2, 2, 1, 3]);
    let corners = |p: &[[f32; 3]], i: &[u32]| i.iter().map(|&i| p[i as usize]).collect::<Vec<_>>();
    assert_eq!(corners(&points, &indices), corners(&m.points, &m.face_indices), "the same faces");
    assert_eq!(points[4], [2.0, 0.0, 0.0], "an unused point goes last");
    assert!(points.iter().zip(&normals.unwrap()).all(|(p, n)| p[0] == n[0]), "normals move with their points");
    m.normal_indices = Some(vec![0; 6]);
    assert!(points_in_use_order(&m).unwrap().2.is_none(), "indexed normals stay");
}

#[test]
fn numbers_are_compact() {
    let mut s = String::new();
    for (x, d) in [(1.5f32, 5), (0.0123456, 5), (2.0, 5), (-0.000001, 5), (0.1, 4)] {
        push_num(&mut s, x, d);
        s.push(' ');
    }
    assert_eq!(s, "1.5 0.01235 2 0 0.1 ");
}

/// The camera of a real scene (test.duf): it stands at -Z and looks back
/// at the figures, slightly upward, with a 65 mm lens at f/12.
fn camera_scene() -> Value {
    json!({
        "node_library": [{"id": "Camera", "type": "camera", "rotation_order": "XYZ",
            "perspective": {"focal_length": 65, "depth_of_field": true, "focal_distance": 340.6856, "fstop": 12},
            "translation": [{"id": "x", "current_value": -0.7272959}, {"id": "y", "current_value": 60.27829}, {"id": "z", "current_value": -339.1028}],
            "rotation": [{"id": "x", "current_value": -175.4371}, {"id": "y", "current_value": -8.683981}, {"id": "z", "current_value": 179.9984}]}],
        "scene": {"current_camera": "#Camera-1", "nodes": [{"id": "Camera-1", "url": "#Camera"}],
                  "extra": [{"type": "studio_render_settings", "render_options": {"imageSize": [1280, 720]}}]},
    })
}

#[test]
fn camera_comes_from_the_daz_scene() {
    let mut lib = Library::new(vec![], camera_scene());
    let c = scene_camera(&mut lib).unwrap();
    let near = |a: [f64; 3], b: [f64; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 0.01);
    assert!(near(c.pos, [-0.0073, 0.6028, -3.391]), "{:?}", c.pos);
    assert!(near(c.target, [0.505, 0.874, -0.034]), "{:?}", c.target);
    assert!(c.up[1] > 0.99);
    // 65 mm on DAZ's 36 mm frame = 30.96° vertically; the 16:9 render
    // (1280×720) sees 52.4° across.
    assert!((c.fov_deg - 52.42).abs() < 0.05, "{}", c.fov_deg);
    // Iray: 65 / (2 · 12) taken as cm.
    assert!((c.aperture_cm - 2.708).abs() < 0.001);
    assert_eq!(render_size(&lib.duf), (1280, 720));
}

#[test]
fn without_a_current_camera_the_first_camera_node_is_used() {
    let mut duf = camera_scene();
    duf["scene"].as_object_mut().unwrap().remove("current_camera");
    duf["scene"]["nodes"] = json!([{"id": "Prop", "url": "#Prop"}, {"id": "Camera-1", "url": "#Camera"}]);
    duf["node_library"].as_array_mut().unwrap().push(json!({"id": "Prop", "type": "node"}));
    let mut lib = Library::new(vec![], duf);
    assert!((scene_camera(&mut lib).unwrap().pos[2] + 3.391).abs() < 0.01);
}

/// Every generated calibration scene saves its camera as current: without
/// `current_camera` DAZ renders the Iray reference through its own
/// Perspective View while `orbis` exports the camera node, so the two can't
/// be compared (#97; `scripts/dev/duf_camera.py --fix`).
#[test]
fn calibration_scenes_render_through_their_camera() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("calib/scenes");
    let mut missing = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|x| x.to_str()) != Some("duf") {
            continue;
        }
        let duf: Value = serde_json::from_str(&crate::dson::decompress_scene(&std::fs::read(&p).unwrap()).unwrap()).unwrap();
        let cameras = duf["node_library"].as_array().into_iter().flatten().filter(|n| n["type"] == "camera").count();
        if cameras > 0 && duf.pointer("/scene/current_camera").and_then(Value::as_str).is_none() {
            missing.push(p.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(missing.is_empty(), "no current_camera: {missing:?}");
}

#[test]
fn daz_spotlight_becomes_a_shaped_usdlux_light() {
    let ch = |id: &str, v: Value| json!({"channel": {"id": id, "current_value": v}});
    let duf = json!({
        "node_library": [{"id": "SpotLight", "type": "light", "rotation_order": "XYZ", "spot": {"falloff_angle": 30},
            "translation": [{"id": "x", "current_value": -184.609}, {"id": "y", "current_value": 99.206}, {"id": "z", "current_value": -209.38}],
            "rotation": [{"id": "x", "current_value": 176.186}, {"id": "y", "current_value": -45.924}, {"id": "z", "current_value": 179.984}],
            "extra": [{"type": "studio_node_channels", "channels": [
                ch("Photometric Mode", json!(true)), ch("Flux", json!(150000)), ch("Temperature", json!(6500)),
                ch("Spread Angle", json!(60)), ch("Light Geometry", json!(0)), ch("Color", json!([1, 1, 1])),
                ch("Beam Exponent", json!(4)),
            ]}]},
            {"id": "Off", "type": "light", "point": {}, "extra": [{"type": "studio_node_channels", "channels": [ch("Illumination", json!(0))]}]}],
        "scene": {"nodes": [{"id": "SpotLight-2", "url": "#SpotLight", "label": "SpotLight"}, {"id": "Off-1", "url": "#Off", "label": "Off"}]},
    });
    let mut lib = Library::new(vec![], duf);
    let lights = scene_lights(&mut lib);
    assert_eq!(lights.len(), 1, "a light with illumination off is left out");
    let l = &lights[0];
    assert_eq!((l.shape.clone(), l.cone_deg, l.kelvin, l.lumens), (LightShape::Point, Some(30.0), Some(6500.0), 150000.0));
    assert!((l.transform[3][0] + 1.846).abs() < 0.001 && (l.transform[3][2] + 2.094).abs() < 0.001);
    // The spot (local -Z) points into the scene, towards +Z.
    assert!(l.transform[2][2] < -0.5, "local Z row {:?}", l.transform[2]);

    // A spot becomes an Octane analytic disk light (UsdLux shaping is lost
    // on import), wired with the USD geometry into a group.
    let mut o = UsdaWriter::new();
    write_lights(&mut o, &lights).unwrap();
    assert!(!o.text.contains("SpotLight"), "spots are not UsdLux lights");
    let spots = octane_spots(&lights);
    // Iray's cos(θ · 90°/30°)⁴ beam: a soft Octane disk 0.78 × as wide.
    assert!((spots[0].spread_deg - 46.8).abs() < 1.0 && spots[0].watts == 10.0, "{:?}", spots[0]);
    assert!((octane_cone_half(30.0, 1.0) - 37.8).abs() < 1.0, "a cos¹ beam: a wider soft disk");
    let (nodes, mesh) = crate::material::scene_lights(&spots, None, None, &[], 500, 1439);
    assert!(nodes.contains("type='402' name='SpotLight'") && nodes.contains("type='3' name='DAZ 3D lights'"));
    assert!(nodes.contains("<pin name='Input 1' dynamicType='12' connect='1439'/>"));
    // DAZ "XYZ" (X applied first) is Octane "ZYX" (enum 5).
    assert!(nodes.contains("<attr name='rotationOrder' type='2'>5</attr>"));
    assert!(nodes.contains("<attr name='scale' type='8'>1 1 -1</attr>"), "disk faces DAZ's -Z");
    // 10 W × 0.039: DAZ's flux is the beam's.
    assert!(nodes.contains("<attr name='value' type='9'>0.39 0 0 0</attr>"), "{nodes}");
    assert!(nodes.contains("<pin name='analyticLightSpreadCutoffHardness'>"), "a soft cone");
    assert!(nodes.contains("<attr name='rotation' type='8'>176.186 -45.924 179.984</attr>"), "{nodes}");
    assert_eq!(nodes.matches("<node ").count(), nodes.matches("</node>").count());
    // Iray's ground: a shadow-catching infinite plane in the group, placed
    // just under y = 0 so a room's floor hides it.
    // Auto: the scene's lowest point; Manual: Ground Origin Y (cm).
    let low = Mesh { points: vec![[0.0, 0.25, 0.0], [1.0, -0.03, 0.0]], ..Default::default() };
    let mut e = SceneEnvironment { map: None, intensity: 1.0, tint: [1.0; 3], rotation_deg: 0.0, orientation_deg: [0.0; 3], scene_only: false, ground_shadow: Some(1.0), ground_origin_y: None, draw_dome: true, scene_lights: true, sun_sky: None, ground_fog: None, dome_ground: None };
    assert!((ground_height(&e, &[low]) + 0.03).abs() < 1e-6);
    assert_eq!(ground_height(&e, &[]), 0.0);
    // Instances (#125): a copy 0.6 m down doesn't lower DAZ's ground
    // (Calib-Env-Mi-GroundAuto-Instance).
    let floor = || Mesh { points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 1.0]], ..Default::default() };
    let copied = Mesh { points: vec![[0.75, 0.15, 0.0]], instances: Some(vec![IDENTITY, [1.0, 0.0, 0.0, 0.45, 0.0, 1.0, 0.0, -0.75, 0.0, 0.0, 1.0, 0.0]]), ..Default::default() };
    assert_eq!(ground_height(&e, &[floor(), copied]), 0.0);
    e.ground_origin_y = Some(12.0);
    assert!((ground_height(&e, &[]) - 0.12).abs() < 1e-6);
    let (lifted, _) = crate::material::scene_lights(&[], Some((0.8, 0.12)), None, &[], 500, 1439);
    assert!(lifted.contains("<attr name='translation' type='8'>0 0.119 0</attr>"), "{lifted}");
    let (g, gmesh) = crate::material::scene_lights(&[], Some((0.8, 0.0)), None, &[], 500, 1439);
    assert!(g.contains("type='4' name='DAZ ground'") && g.contains("<attr name='translation' type='8'>0 -0.001 0</attr>"), "{g}");
    assert!(g.contains("type='110' name='DAZ ground plane'") && g.contains("type='145'") && g.contains("<attr name='value' type='6'>0.712</attr>"), "{g}");
    assert!(g.contains("<pin name='Input 2' dynamicType='12' connect='501'/>") && gmesh != 1439, "{g}");
    assert_eq!(g.matches("<node ").count(), g.matches("</node>").count());
    assert_eq!(crate::material::scene_lights(&[], None, None, &[], 500, 1439), (String::new(), 1439));
    assert_ne!(mesh, 1439);
    assert!(light_emitters(&lights, &mut |_| None).is_empty(), "no UsdLux emitter for a spot");
    // A point light stays UsdLux: 1500 lm × 0.58 / 15 000 = 0.058 W, glowing.
    let mut point = lights[0].clone();
    point.cone_deg = None;
    point.lumens = 1500.0;
    point.label = "Bulb".into();
    let mut w = UsdaWriter::new();
    write_lights(&mut w, &[point.clone()]).unwrap();
    let o = w.text;
    assert!(o.contains("def SphereLight \"Bulb\"") && o.contains("bool treatAsPoint = 1") && o.contains("float inputs:colorTemperature = 6500"));
    assert!(o.contains("float inputs:intensity = 0.058000") && o.contains("bool inputs:normalize = 1"), "{o}");
    let em = light_emitters(&[point.clone()], &mut |_| None);
    let xml = crate::material::light_inputs(&em, 100);
    assert!(xml.contains("name='/Scene/Lights/Bulb_light'") && xml.contains("type='53'") && xml.contains("6500 0 0"));
    assert!(!xml.contains("distribution"));
    // An IES light (Light Profile): the profile on the emission's
    // distribution, in the light's object space, the power over its share.
    let profile = crate::ies::Profile::parse(&std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("calib/ies/Orbis-Asym.ies")).unwrap()).unwrap();
    let share = profile.mean_share();
    let ies_point = SceneLight { profile: Some("/Runtime/ORBIS/Orbis-Asym.ies".into()), ..point.clone() };
    let mut asked = Vec::new();
    let em = light_emitters(&[ies_point.clone()], &mut |p| {
        asked.push(p.to_string());
        Some(("assets/ies/00_Orbis-Asym.ies".into(), profile.clone()))
    });
    assert_eq!(asked, ["/Runtime/ORBIS/Orbis-Asym.ies"]);
    let d = em[0].ies.as_ref().expect("an IES emitter");
    // Iray's frame is the light's turned half round X (Calib-L-IES-*).
    assert_eq!(d.frame, [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]]);
    // A point light: ¼ of the flux-kept profile (Iray's renders).
    assert!((d.power_scale * share * IES_POINT_SHARE - 1.0).abs() < 1e-9 && !d.world);
    let xml = crate::material::light_inputs(&em, 100);
    assert!(xml.contains("<pin name='distribution'>") && xml.contains("assets/ies/00_Orbis-Asym.ies") && xml.contains("<attr name='value' type='2'>3</attr>"), "object space: {xml}");
    // Iray's axis (the light's −Z) onto Octane's (−Y), φ 0 (+X) from Octane's +Z.
    assert!(xml.contains("<attr name='rotation' type='8'>90 90 0</attr>"), "{xml}");
    // A rect prim is turned half round X already: Iray's frame is its own.
    let rect = SceneLight { shape: LightShape::Rect { width: 0.5, height: 0.5 }, ..ies_point };
    let em = light_emitters(&[rect.clone()], &mut |_| Some(("a.ies".into(), profile.clone())));
    let d = em[0].ies.as_ref().unwrap();
    assert_eq!(d.frame, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    // An area light keeps the flux through its front: Lambertian × profile.
    let front = profile.lambert_share(&[([0.0, 0.0, 1.0], 1.0)]);
    assert!((d.power_scale * front - 1.0).abs() < 1e-9 && front > share, "{front} {share}");
    let two = light_emitters(&[SceneLight { two_sided: true, ..rect }], &mut |_| Some(("a.ies".into(), profile.clone())));
    assert!(two[0].ies.as_ref().unwrap().power_scale > d.power_scale, "the back keeps less of a downlight");
}

#[test]
fn vdb_volume_goes_to_octane_not_as_its_box() {
    let dir = std::env::temp_dir().join(format!("orbis-volume-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("Runtime")).unwrap();
    std::fs::write(dir.join("Runtime/haze.vdb"), crate::vdb::tests::file()).unwrap();
    let mut duf = scene();
    duf["scene"]["nodes"].as_array_mut().unwrap().push(json!({"id": "Box", "name": "Box", "label": "Haze", "geometries": [{"id": "bi", "url": "#h"}]}));
    duf["scene"]["materials"].as_array_mut().unwrap().push(json!({"geometry": "#bi", "groups": ["Torso", "Legs"], "extra": [{"type": "studio_material_channels", "channels": [
        {"channel": {"id": "Volume File", "type": "file", "current_value": "/Runtime/haze.vdb"}},
        {"channel": {"id": "Scattering Multiplier", "type": "float_color", "current_value": [1, 0.5, 0]}},
        {"channel": {"id": "Absorption Multiplier", "type": "float_color", "current_value": [0.5, 0.5, 0.5]}},
        {"channel": {"id": "Density Multiplier", "type": "float", "current_value": 1000}},
        {"channel": {"id": "Density Relative To Size", "type": "bool", "current_value": true}},
    ]}]}));
    let mut d = dump();
    let t = [0.1, 0.0, 0.0, 5.0, 0.0, 0.1, 0.0, -10000.0, 0.0, 0.0, 0.1, 0.0];
    d.nodes.push(DumpNode { name: "Box".into(), label: "Haze".into(), lod: 0, vertices: (0..6).map(|i| [i as f32, 3.0, 0.0]).collect(), transform: Some(t), ..Default::default() });
    let mut lib = Library::new(vec![dir.clone()], duf);
    let (meshes, mut report) = build_meshes(&mut lib, &d).unwrap();
    assert!(meshes.iter().all(|m| m.label != "Haze"), "the box itself is not drawn");
    let v = volume::scene_volumes(&mut lib, &d, &mut report);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(v.len(), 1, "{:?}", report.skipped);
    // Relative to size: 1000 ÷ the node's scale 0.1; the file's first grid;
    // the multipliers decoded as colours (0.5 → 0.218).
    assert_eq!((v[0].grid.as_str(), v[0].transform), ("density", t));
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|k| (a[k] - b[k]).abs() < 0.1);
    assert!(near(v[0].scattering, [10000.0, 2176.38, 0.0]) && near(v[0].absorption, [2176.38; 3]), "{v:?}");
    let ov = crate::material::OctaneVolume { name: "Haze".into(), asset: Some("assets/vdb/00_haze.vdb".into()), grid: "density".into(), transform: t.map(|x| x * 0.01), scattering: v[0].scattering, absorption: v[0].absorption, anisotropy: 0.85 };
    let (x, mesh) = crate::material::scene_lights(&[], None, None, &[ov], 500, 1439);
    assert!(x.contains("<attr name='transform' type='13'>0.001 0 0 0.05 0 0.001 0 -100 0 0 0.001 0</attr>"), "{x}");
    assert!(x.contains("type='91' name='Haze'") && x.contains("<attr name='filename' type='11'>assets/vdb/00_haze.vdb</attr>"), "{x}");
    // 10⁴ per meter: the density pin's 10⁴ × the grids' 1; a 10-voxel step
    // (1 % of the VDB's own voxel at the placement's 0.001).
    assert!(x.contains("<attr name='volumeScatteringScale' type='6'>1</attr>") && x.contains("<attr name='value' type='9'>10000 0 0 0</attr>"), "{x}");
    assert!(x.contains("<pin name='rayMarchStepPercent'>") && x.contains("<attr name='value' type='9'>0.01 0 0 0</attr>"), "{x}");
    assert!(x.contains("<attr name='value' type='8'>1 0.217638 0</attr>") && x.contains("<attr name='value' type='8'>0.217638 0.217638 0.217638</attr>"), "{x}");
    assert!(x.contains("type='60'") && x.contains("<attr name='value' type='8'>0.85 0 0</attr>"), "{x}");
    assert!(x.contains(&format!("connect='{}'", 501)) && mesh != 1439);
    assert_eq!(x.matches("<node ").count(), x.matches("</node>").count());
}

#[test]
fn iray_tone_mapping_drives_the_imager() {
    let ch = |id: &str, v: Value| json!({"channel": {"id": id, "current_value": v}});
    let duf = json!({
        "node_library": [{"id": "Tonemapper Options", "extra": [{"type": "studio_node_channels", "channels": [
            ch("Tone Mapping Enable", json!(true)), ch("Exposure Value", json!(12)), ch("cm2 Factor", json!(1)),
            ch("White Point", json!([0.8, 0.9, 1])), ch("White Point Scale", json!(1)),
            ch("Burn Highlights", json!(0.25)), ch("Saturation", json!(1.2)),
        ]}]}],
        "scene": {"nodes": [{"id": "Tonemapper Options-1", "url": "#Tonemapper%20Options"}]},
    });
    let mut lib = Library::new(vec![], duf);
    let tone = scene_tone_mapping(&mut lib);
    assert_eq!(tone.white_point, [0.8f64.powf(2.2), 0.9f64.powf(2.2), 1.0], "linearised, as DAZ hands Iray");
    assert_eq!(tone.octane_exposure(1.4), 2.8, "EV 12 is one stop brighter than DAZ's 13");
    let cams = vec![("Camera".to_string(), framing_camera(&[]))];
    // Burn 0.25 is beyond Octane's imager: Iray's curve as a LUT, the
    // exposure lowered by its headroom, no highlight compression.
    let ocs = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &tone, false, None, true, &crate::iray::RenderOptions::default());
    assert!(ocs.contains("0.175000 0 0 0") && ocs.contains("1.200000 0 0 0"), "exposure 2.8 / 16, saturation");
    assert!(ocs.contains("type='103'") && ocs.contains(&format!("<attr name='filename' type='11'>{TONE_LUT_ASSET}</attr>")));
    // The white point is in the LUT; Octane's white balance stays white.
    assert!(ocs.contains("<attr name='value' type='8'>1.000000 1.000000 1.000000</attr>"));
    let plain = ToneMapping { burn_highlights: 1.0, white_point: [2.0; 3], ..tone.clone() };
    let ocs = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &plain, false, None, true, &crate::iray::RenderOptions::default());
    assert!(ocs.contains("2.800000 0 0 0") && !ocs.contains("type='103'") && !ocs.contains("@TONE_LUT@"));
    assert!(tone.burn_per_component && tone.vignetting == 0.0 && !ocs.contains("@VIGNETTING@"));
    // Vignetting: Iray's cos(θ)^v as Octane's frame-space cone, fitted per
    // camera (Calib-Tone: 68.7°, 16:9).
    let vig = |v: f64, fov: f64| ToneMapping { vignetting: v, ..ToneMapping::default() }.octane_vignetting(fov, 16.0 / 9.0);
    assert!((vig(0.5, 68.725571) - 0.0855).abs() < 0.001 && (vig(1.0, 68.725571) - 0.1653).abs() < 0.001, "{} {}", vig(0.5, 68.725571), vig(1.0, 68.725571));
    assert!(vig(1.0, 30.0) < vig(1.0, 68.7) && vig(0.0, 68.7) == 0.0 && vig(50.0, 170.0) == 1.0);
    // Lens options (#39): shift and blades on the thin lens, distortion as
    // an OSL camera in its place.
    assert!(!ocs.contains("@CAM_") && ocs.contains("<attr name='value' type='5'>6 0 0 0</attr>"));
    let mut lensed = cams.clone();
    lensed[0].1.lens = crate::iray::Lens { shift: [0.36, 0.0], blades: 5, ..Default::default() };
    let ocs = project_ocs("Shot", "scene.usdz", &lensed, (1280, 720), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &plain, false, None, true, &crate::iray::RenderOptions::default());
    // 0.36 × F / 2 = 0.9 cm of a 3.6 cm film, the view the other way.
    assert!(ocs.contains("<attr name='value' type='9'>-0.250000 0.000000 0 0</attr>") && ocs.contains("<attr name='value' type='5'>5 0 0 0</attr>"));
    lensed[0].1.lens.distortion = crate::iray::Distortion::Equidistant;
    let ocs = project_ocs("Shot", "scene.usdz", &lensed, (1280, 720), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &plain, false, None, true, &crate::iray::RenderOptions::default());
    assert!(ocs.contains("<node id='100005' type='126'>") && !ocs.contains("<node id='100005' type='13'>") && !ocs.contains("@CAM_") && ocs.contains("<pin name='environment'>"));
    let mut cams = cams.clone();
    cams[0].1.fov_deg = 68.725571;
    let ocs = project_ocs("Shot", "scene.usdz", &cams, (1920, 1080), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &ToneMapping { vignetting: 1.0, ..plain }, false, None, true, &crate::iray::RenderOptions::default());
    assert!(ocs.contains("<pin name='vignetting'>\n        <node id='100120' type='6'>\n         <attr name='value' type='9'>0.165"));
    assert_eq!(scene_tone_mapping(&mut Library::new(vec![], json!({"scene": {"nodes": []}}))), ToneMapping::default());
    // Iray exposes by shutter, f-number and ISO; DAZ's EV is only shown.
    let physical = |s: f64, n: f64, iso: f64| {
        let duf = json!({
            "node_library": [{"id": "Tonemapper Options", "extra": [{"type": "studio_node_channels", "channels": [
                ch("Exposure Value", json!(13)), ch("Shutter Speed", json!(s)), ch("Aperture", json!(n)), ch("Film ISO", json!(iso)),
            ]}]}],
            "scene": {"nodes": [{"id": "Tonemapper Options-1", "url": "#Tonemapper%20Options"}]},
        });
        scene_tone_mapping(&mut Library::new(vec![], duf)).exposure_value
    };
    assert_eq!(physical(128.0, 8.0, 100.0), 13.0);
    assert_eq!(physical(64.0, 8.0, 100.0), 12.0, "a stale EV 13 beside a 1/64 s shutter");
    assert_eq!(physical(128.0, 8.0, 200.0), 12.0, "ISO 200: one stop brighter");
}

#[test]
fn dome_rotation_is_a_u_shift() {
    let env = |rot: f64, oy: f64| SceneEnvironment { map: None, intensity: 1.0, tint: [1.0; 3], rotation_deg: rot, orientation_deg: [0.0, oy, 0.0], scene_only: false, ground_shadow: None, ground_origin_y: None, draw_dome: true, scene_lights: true, sun_sky: None, ground_fog: None, dome_ground: None };
    assert_eq!(env(0.0, 0.0).dome_u_shift(), 0.25);
    assert!((env(110.0, 0.0).dome_u_shift() - (0.25 + 110.0 / 360.0)).abs() < 1e-12);
    assert_eq!(env(270.0, 0.0).dome_u_shift(), 0.0);
    assert_eq!(env(180.0, 90.0).dome_u_shift(), 0.5);
    assert_eq!(env(180.0, 90.0).dome_tilt(), None);
}

#[test]
fn dome_tilt_is_a_spherical_projection_with_iray_orientation() {
    let e = SceneEnvironment { map: None, intensity: 1.0, tint: [1.0; 3], rotation_deg: 30.0, orientation_deg: [10.0, 20.0, 5.0], scene_only: false, ground_shadow: None, ground_origin_y: None, draw_dome: true, scene_lights: true, sun_sky: None, ground_fog: None, dome_ground: None };
    // Orientation Y moves from the U shift into the projection.
    assert_eq!(e.dome_tilt(), Some([10.0, 20.0, 5.0]));
    assert!((e.dome_u_shift() - (0.25 + 30.0 / 360.0)).abs() < 1e-12);
}

#[test]
fn iray_environment_becomes_an_hdri_texture_environment() {
    let ch = |id: &str, v: Value| json!({"channel": {"id": id, "current_value": v}});
    let duf = json!({
        "node_library": [{"id": "Environment Options", "extra": [{"type": "studio_node_channels", "channels": [
            ch("Environment Mode", json!(0)), ch("Environment Intensity", json!(2)), ch("Dome Rotation", json!(90)),
        ]}]}],
        "scene": {"nodes": [{"id": "Environment Options-1", "url": "#Environment%20Options",
            "extra": [{"type": "studio_node_channels", "channels": [
                {"channel": {"id": "Environment Map", "current_value": 2, "image_file": "/Environments/HDRI/sky%201.hdr"}}]}]}]},
    });
    let mut lib = Library::new(vec![], duf);
    let env = scene_environment(&mut lib).unwrap();
    assert_eq!(env, SceneEnvironment { map: Some("/Environments/HDRI/sky 1.hdr".into()), intensity: 4.0, tint: [1.0; 3], rotation_deg: 90.0, orientation_deg: [0.0; 3], scene_only: false, ground_shadow: Some(1.0), ground_origin_y: None, draw_dome: true, scene_lights: true, sun_sky: None, ground_fog: None, dome_ground: None });

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("Environments/HDRI")).unwrap();
    std::fs::write(dir.path().join("Environments/HDRI/sky 1.hdr"), b"#?RADIANCE").unwrap();
    let mut tex = crate::material::Textures::default();
    let xml = crate::material::environment_texture(Some(&env), &[dir.path().to_path_buf()], &mut tex, 10);
    assert!(xml.contains("<attr name='filename' type='11'>assets/tex/0000_sky_1.hdr</attr>"), "{xml}");
    assert!(xml.contains("<attr name='translation' type='8'>0.5 0 0</attr>"), "{xml}");
    assert_eq!(tex.files.len(), 1);
    // No HDRI: Octane's neutral white texture.
    let none = crate::material::environment_texture(None, &[], &mut tex, 20);
    assert!(none.contains("type='33'") && none.contains("1 1 1"));
}

#[test]
fn project_ocs_fills_every_placeholder() {
    let cam = vec![("Camera".to_string(), framing_camera(&[]))];
    let ocs = project_ocs("Bob's <scene>", "scene.usdz", &cam, (1280, 720), "", &Variant::new(Look::Enhanced, OctaneMode::Preview).tuning(), &ToneMapping::default(), false, None, true, &crate::iray::RenderOptions::default())
        .replace("@MATERIAL_INPUTS@", "")
        .replace("@SCENE_NODES@", "")
        .replace("@MESH_CONNECT@", "1439")
        .replace("@ENV_POWER@", "1");
    assert!(!ocs.contains('@'), "placeholder left");
    assert!(ocs.contains("<attr name='filename' type='11'>assets/scene.usdz</attr>"));

    assert!(ocs.contains("<attr name='value' type='4'>1280 720 0</attr>"));
    // The render target's denoiser runs during the render, not only once
    // it finishes — a render saved before then kept its noise (sparkles).
    let at = ocs.rfind("<pin name='denoiserOnce'>").unwrap();
    assert!(ocs[at..at + 120].contains("<attr name='value' type='1'>0</attr>"), "{}", &ocs[at..at + 120]);
    // The render target sits in the Scene graph, wired to the USD import.
    assert!(ocs.contains("type='56' name='Bob&apos;s &lt;scene&gt;'"), "render target named after the scene");
    assert!(ocs.contains("type='2' name='Bob&apos;s &lt;scene&gt; (DAZ 3D)'"));
    assert!(ocs.contains("<pin name='mesh' connect='1439'/>"));
}

#[test]
fn every_daz_camera_gets_its_own_render_target() {
    let mut duf = camera_scene();
    duf["scene"]["nodes"] = json!([
        {"id": "Camera-1", "url": "#Camera", "label": "Wide"},
        {"id": "Camera-2", "url": "#Camera", "label": "Close",
         "translation": [{"id": "z", "current_value": -100.0}]},
    ]);
    duf["scene"]["current_camera"] = json!("#Camera-2");
    let mut lib = Library::new(vec![], duf);
    let cams = scene_cameras(&mut lib);
    let labels: Vec<_> = cams.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(labels, ["Close", "Wide"], "current camera first");
    assert!((cams[0].1.pos[2] + 1.0).abs() < 0.01);

    let env = "        <node id='190001' type='33'>\n        </node>\n";
    let ocs = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), env, &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &ToneMapping::default(), false, None, true, &crate::iray::RenderOptions::default());
    assert!(ocs.contains("type='56' name='Shot – Close'") && ocs.contains("type='56' name='Shot – Wide'"));
    assert_eq!(ocs.matches("connect='@MESH_CONNECT@'").count(), 2, "both wired to the geometry");
    // Node ids stay unique across the copies.
    let mut ids: Vec<_> = ocs.split(" id='").skip(1).map(|s| s.split('\'').next().unwrap()).collect();
    let n = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), n, "duplicate node id");
    assert!(ocs.contains("id='1190001'"), "env texture moves with the second copy");
}

#[test]
fn octane_modes_set_kernel_and_imager() {
    let cams = vec![("A".to_string(), framing_camera(&[])), ("B".to_string(), framing_camera(&[]))];
    let fin = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &ToneMapping::default(), false, None, true, &crate::iray::RenderOptions::default());
    assert!(fin.contains("<node id='100060' type='25'>") && fin.contains("<node id='1100060' type='25'>"), "path tracing");
    assert_eq!(fin.matches("<pin name='maxScatterDepth'>").count(), 2);
    assert!(fin.contains("<attr name='value' type='9'>1.400000 0 0 0</attr>"), "Final exposure at EV 13");
    let pre = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), "", &Variant::new(Look::Enhanced, OctaneMode::Preview).tuning(), &ToneMapping::default(), false, None, true, &crate::iray::RenderOptions::default());
    assert!(pre.contains("<node id='100060' type='24'>") && pre.contains("<node id='1100060' type='24'>"), "direct lighting");
    for ocs in [&fin, &pre] {
        for ph in ["@KERNEL@", "@EXPOSURE@", "@HIGHLIGHT_COMPRESSION@", "@WHITE_BALANCE@", "@SATURATION@", "@DENOISER@"] {
            assert!(!ocs.contains(ph), "{ph} left");
        }
        for ph in ["@POST_ON@", "@BLOOM_", "@GLARE_POWER@", "_DEPTH@", "@GI_CLAMP@", "@FILTER_SIZE@"] {
            assert!(!ocs.contains(ph), "{ph} left");
        }
        assert!(!ocs.contains(" 0 0 0 0 0 0<"), "a value written twice over its placeholder's zeros");
        // Every render target's kernel: 1 µm, below the eye's 0.05 mm tear film gap.
        assert_eq!(ocs.matches("<attr name='value' type='9'>1e-06 0 0 0</attr>").count(), 2, "ray epsilon");
        let mut ids: Vec<_> = ocs.split(" id='").skip(1).map(|s| s.split('\'').next().unwrap()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate node id");
    }
}

#[test]
fn exports_an_orbx_with_the_scene_and_project() {
    let dir = tempfile::tempdir().unwrap();
    let mut duf = scene();
    duf["node_library"] = camera_scene()["node_library"].clone();
    duf["scene"]["current_camera"] = json!("#Camera-1");
    duf["scene"]["nodes"].as_array_mut().unwrap().push(json!({"id": "Camera-1", "url": "#Camera"}));
    let scene_path = dir.path().join("My Scene.duf");
    std::fs::write(&scene_path, duf.to_string()).unwrap();
    let d = dump();
    let nodes: Vec<Value> = d
        .nodes
        .iter()
        .map(|f| json!({"name": f.name, "label": f.label, "lod": f.lod, "vertices": f.vertices.concat()}))
        .collect();
    let dump_path = dir.path().join("dump.json");
    std::fs::write(&dump_path, json!({"format": "orbis-scene-dump", "format_version": 2, "nodes": nodes}).to_string()).unwrap();

    let out = dir.path().join("My Scene.orbx");
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = seen.clone();
    let opts = ExportOptions { progress: Progress::new(move |p, _| sink.lock().unwrap().push(p)), ..Default::default() };
    let report = export_scene_orbx(&scene_path, &dump_path, &out, &opts).unwrap();
    let seen = seen.lock().unwrap();
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "progress only grows: {seen:?}");
    assert_eq!((seen[0], *seen.last().unwrap()), (0, 100));
    assert_eq!(report.meshes, 2, "host with its merged graft, and the shell");
    let (version, files) = crate::orbx::read_orbx(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(version, OCTANE_VERSION);
    let names: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(names, ["assets/scene.usdc", "project.ocs"]);
    let ocs = String::from_utf8(files[1].1.clone()).unwrap();
    assert!(ocs.contains("type='56' name='My Scene'") && ocs.contains("-3.391028 0"));
    assert!(ocs.contains("<attr name='filename' type='11'>assets/scene.usdc</attr>"));
    // The binary layer, its header patched into the package's first chunk.
    let layer = read_usdc(&files[0].1);
    assert!(layer.prims().iter().any(|(path, ty)| path == "/Scene/Body" && ty == "Mesh"), "{:?}", layer.prims());

    // Off the calibrated tone mapping: the LUT the imager names is packed.
    let ch = |id: &str, v: Value| json!({"channel": {"id": id, "current_value": v}});
    duf["node_library"].as_array_mut().unwrap().push(json!({"id": "Tonemapper Options", "extra": [{"type": "studio_node_channels",
        "channels": [ch("Burn Highlights", json!(0.0))]}]}));
    duf["scene"]["nodes"].as_array_mut().unwrap().push(json!({"id": "Tonemapper Options-1", "url": "#Tonemapper%20Options"}));
    std::fs::write(&scene_path, duf.to_string()).unwrap();
    export_scene_orbx(&scene_path, &dump_path, &out, &ExportOptions::default()).unwrap();
    let (_, files) = crate::orbx::read_orbx(&std::fs::read(&out).unwrap()).unwrap();
    let names: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(names, ["assets/scene.usdc", TONE_LUT_ASSET, "project.ocs"]);
    assert!(String::from_utf8_lossy(&files[2].1).contains(TONE_LUT_ASSET));
}

#[test]
fn dump_matches_by_label_then_unique_name() {
    let d = SceneDump::parse(
        r#"{"format": "orbis-scene-dump", "format_version": 2,
            "nodes": [{"name": "Cuff", "label": "Cuff (2)", "vertices": [0,0,0, 2,2,2]},
                      {"name": "Prop", "label": "Prop (3)", "vertices": [1,1,1]},
                      {"name": "Empty", "label": "Empty", "vertices": []}]}"#,
    )
    .unwrap();
    assert_eq!(d.node("Cuff (2)", "Cuff").unwrap().vertices, [[0.0, 0.0, 0.0], [2.0, 2.0, 2.0]]);
    assert_eq!(d.node("Prop", "Prop").unwrap().vertices[0], [1.0, 1.0, 1.0]);
    assert!(d.node("Other", "Cuff").is_some(), "falls back to a unique name");
    assert!(d.node("Other", "Nope").is_none());
    assert!(d.node("Empty", "Empty").is_none(), "no vertices, no entry");
}

#[test]
fn dump_format_is_checked() {
    let err = |json: &str| SceneDump::parse(json).unwrap_err().to_string();
    assert!(err(r#"{"figures": []}"#).contains("not an ORBIS scene dump"));
    assert!(err(r#"{"format": "orbis-scene-dump", "format_version": 99}"#).contains("format 99"));
    // Format 3: strands as runs of consecutive vertices.
    let d = SceneDump::parse(r#"{"format": "orbis-scene-dump", "format_version": 3, "nodes": [{"label": "Hair",
        "vertices": [0,0,0, 1,0,0, 2,0,0, 0,1,0, 1,1,0], "polyline_counts": [3, 2], "polyline_materials": [0, 0]}]}"#).unwrap();
    assert_eq!(d.nodes[0].polylines, vec![vec![0, 1, 2], vec![3, 4]]);
    assert!(SceneDump::parse(r#"{"format": "orbis-scene-dump", "format_version": 2, "nodes": [{"vertices": [1, 2]}]}"#).is_err());
}

#[test]
fn dump_cache_path_is_per_scene() {
    let a = dump_cache_path(Path::new("/a/My Scene.duf"));
    let b = dump_cache_path(Path::new("/b/My Scene.duf"));
    assert_ne!(a, b);
    assert!(a.file_name().unwrap().to_str().unwrap().starts_with("My Scene-"));
    assert_eq!(a, dump_cache_path(Path::new("/a/My Scene.duf")));
}

#[test]
fn shell_surfaces_are_cut_after_refining() {
    // A creased strip of 3 quads (A B C); the shell shows only B. Cut after
    // refining, B's border lies on the refined whole — cut first, the
    // boundary rules keep it on the crease (under a smooth host there).
    let pts: Vec<[f32; 3]> = (0..8).map(|i| [(i % 4) as f32, (i / 4) as f32, if (i % 4) == 1 || (i % 4) == 2 { 0.0 } else { 1.0 }]).collect();
    let quad = |a: u32| [a, a + 1, a + 5, a + 4];
    let whole = Mesh {
        points: pts.clone(),
        face_counts: vec![4; 3],
        face_indices: [quad(0), quad(1), quad(2)].concat(),
        subsets: vec![("A".into(), vec![0]), ("B".into(), vec![1]), ("C".into(), vec![2])],
        materials: vec![None; 3],
        ..Default::default()
    };
    let keep: Vec<bool> = [false, true, false].iter().flat_map(|&k| [k; 4]).collect();
    let cut_after = subdivide::keep_faces(&subdivide::subdivide_once(&whole), &keep);
    assert_eq!(cut_after.face_counts.len(), 4);
    assert_eq!(cut_after.subsets, vec![("B".to_string(), vec![0, 1, 2, 3])]);
    let refined = subdivide::subdivide_once(&whole).points;
    assert!(cut_after.points.iter().all(|p| refined.contains(p)), "B's points are the whole's");
    let patch = Mesh { face_counts: vec![4], face_indices: quad(1).to_vec(), subsets: vec![("B".into(), vec![0])], materials: vec![None], points: pts.clone(), ..Default::default() };
    let cut_first = subdivide::subdivide_once(&patch);
    // B's border with A (x = 1): smoothed towards A like the host's, or left
    // in the crease.
    let border = |m: &Mesh| m.points.iter().filter(|p| p[0] == 1.0).map(|p| p[2]).fold(0.0f32, f32::min);
    let lifted = |m: &Mesh| m.points.iter().filter(|p| p[0] == 1.0).all(|p| p[2] > 0.0);
    assert!(lifted(&cut_after), "{:?}", cut_after.points);
    assert_eq!(border(&cut_first), 0.0, "the cut-first border stays in the crease");
}

#[test]
fn fully_transparent_surfaces_are_left_out() {
    // Legs at the DAZ Default shader's Opacity Strength 0 (NGV8's "Hiding").
    let mut duf = scene();
    duf["scene"]["materials"] = json!([
        {"geometry": "#hi", "groups": ["Torso"], "uv_set": "#huv"},
        {"geometry": "#hi", "groups": ["Legs"], "uv_set": "#huv",
         "transparency": {"channel": {"id": "transparency", "current_value": 0}}},
    ]);
    let mut lib = Library::new(vec![], duf);
    let (meshes, _) = build_meshes(&mut lib, &dump()).unwrap();
    let body = &meshes[0];
    let groups: Vec<&str> = body.subsets.iter().map(|(g, _)| g.as_str()).collect();
    assert!(groups.contains(&"Torso") && !groups.contains(&"Legs"), "{groups:?}");
}

#[test]
fn instance_math() {
    // Rotated 90° about Y, scaled 2, moved: inverse undoes it.
    let m: Affine = [0.0, 0.0, 2.0, 10.0, 0.0, 2.0, 0.0, 20.0, -2.0, 0.0, 0.0, 30.0];
    let id = compose(&m, &invert(&m).unwrap());
    for (k, v) in id.iter().enumerate() {
        let want = if [0, 5, 10].contains(&k) { 1.0 } else { 0.0 };
        assert!((v - want).abs() < 1e-12, "{id:?}");
    }
    let p: Placement = std::array::from_fn(|k| m[k] as f32);
    let (pos, q, scale) = decompose(&p);
    assert_eq!(pos, [10.0, 20.0, 30.0]);
    assert_eq!(scale, [2.0, 2.0, 2.0]);
    let s = std::f32::consts::FRAC_1_SQRT_2;
    assert!((q[0] - s).abs() < 1e-6 && (q[2] - s).abs() < 1e-6 && q[1].abs() < 1e-6 && q[3].abs() < 1e-6, "{q:?}");
    // A mirror keeps its rotation proper: the flip goes into the scale.
    let (_, _, scale) = decompose(&[-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    assert_eq!(scale, [-1.0, 1.0, 1.0]);
}

#[test]
fn instances_of_a_source() {
    // The source sits at x = 100 cm, a copy at x = 300 cm turned 180° about Y.
    let mut d = dump();
    d.nodes[0].transform = Some([1.0, 0.0, 0.0, 100.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    d.instances = vec![DumpInstance {
        target_label: "Body".into(),
        transforms: vec![[-1.0, 0.0, 0.0, 300.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0]],
        ..Default::default()
    }];
    let mut lib = Library::new(vec![], scene());
    let (meshes, report) = build_meshes(&mut lib, &d).unwrap();
    assert_eq!(report.instances, 1);
    let body = meshes.iter().find(|m| m.name == "Body").unwrap();
    let copies = body.instances.as_ref().unwrap();
    assert_eq!(copies.len(), 2, "the source itself, then its copy");
    assert_eq!(copies[0], IDENTITY);
    // copy · source⁻¹: a world point p of the source lands at R·(p − 1 m) + 3 m.
    assert_eq!(copies[1], [-1.0, 0.0, 0.0, 4.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0]);
    let (usda, _) = write_usda(&meshes, &[]);
    assert!(usda.contains("def PointInstancer \"Body_copies\""), "{}", &usda[..400]);
    assert!(usda.contains("rel prototypes = [</Scene/Body_copies/Prototypes/Body>]"));
    assert!(usda.contains("point3f[] positions = [(0, 0, 0), (4, 0, 0)]"), "{usda}");
    assert!(usda.contains("(0, 0, 1, 0)]"));
    // Balanced braces: the prototype is closed inside its instancer.
    assert_eq!(usda.matches('{').count(), usda.matches('}').count());

    // A hidden source takes its copies with it: DAZ's .mi has neither the
    // target nor its instances when the target is Visible or Visible in
    // Render off, for an instance node and an instance group alike (#130).
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let objects = |scene: &str| -> Vec<String> {
        let mi: Value = serde_json::from_str(&std::fs::read_to_string(root.join(format!("calib/references/mi/{scene}.json"))).unwrap()).unwrap();
        mi["objects"].as_object().unwrap().keys().cloned().collect()
    };
    let shown = objects("Calib-Mi-InstGroup");
    assert!(shown.contains(&"M1 tinted".to_string()) && shown.contains(&"SphereGroup".to_string()), "{shown:?}");
    for scene in ["Calib-Mi-InstGroup-Hidden", "Calib-Mi-InstGroup-NoRender", "Calib-Mi-Instance-NoRender", "Calib-Env-Mi-GroundAuto-InstanceHidden"] {
        let o = objects(scene);
        assert!(!o.iter().any(|k| k == "M1 tinted" || k.starts_with("Sphere")), "{scene}: {o:?}");
    }
    for flag in ["Visible", "Renderable"] {
        let mut duf = scene();
        duf["scene"]["nodes"][0]["extra"] = json!([{"type": "studio_node_channels", "channels": [{"channel": {"id": flag, "current_value": false}}]}]);
        let mut lib = Library::new(vec![], duf);
        let (meshes, report) = build_meshes(&mut lib, &d).unwrap();
        assert!(!meshes.iter().any(|m| m.name == "Body"), "{flag}");
        assert_eq!(report.instances, 0, "{flag}");
        assert!(report.skipped.contains(&"Body: hidden in the scene".to_string()), "{flag}: {:?}", report.skipped);
    }
}

#[test]
fn iray_tone_curve() {
    // Gladiator: Burn 0, Crush 0.07, Gamma 1.857 — Reinhard, then the crush,
    // then the gamma.
    let t = ToneMapping { burn_highlights: 0.0, crush_blacks: 0.06985294, gamma: 1.856618, ..ToneMapping::default() };
    assert!(!t.is_plain() && ToneMapping::default().is_plain());
    let want = |c: f64| {
        let r = c / (1.0 + c);
        (r * r / (r + 0.3 * 0.06985294)).powf(1.0 / 1.856618)
    };
    for c in [0.01, 0.2, 1.0, 5.0] {
        assert!((t.iray_curve(c) - want(c)).abs() < 1e-12);
    }
    // The defaults are Octane's Gamma 2.2 response.
    assert!((ToneMapping::default().iray_curve(0.25) - 0.25f64.powf(1.0 / 2.2)).abs() < 1e-12);
    // The LUT's entry k holds the curve at (k / 4095)^2.2 · 16.
    let lut = t.cube_lut();
    let rows: Vec<&str> = lut.lines().filter(|l| l.starts_with(|c: char| c.is_ascii_digit())).collect();
    assert_eq!(rows.len(), 4096);
    let mid: f64 = rows[2048].split(' ').next().unwrap().parse().unwrap();
    assert!((mid - want((2048.0f64 / 4095.0).powf(2.2) * 16.0)).abs() < 1e-6, "{mid}");
    assert!(lut.contains("LUT_1D_SIZE 4096"));
}

#[test]
fn white_point_gains() {
    // Iray divides by the (linear) white point and keeps its CIE RGB
    // luminance (#70): k / wp per channel, k as measured over
    // Calib-Tone-Default (per-pixel medians).
    for (wp, k) in [([1.0, 0.8, 0.6], 0.651), ([0.6, 0.8, 1.0], 0.531), ([0.9, 1.0, 0.7], 0.945)] {
        let lin = wp.map(|c: f64| c.powf(2.2));
        let g = ToneMapping { white_point: lin, ..ToneMapping::default() }.white_gains();
        for i in 0..3 {
            assert!((g[i] * lin[i] - k).abs() < 0.002, "{wp:?} {g:?}");
        }
    }
    let lin = [1.0f64, 0.8f64.powf(2.2), 0.6f64.powf(2.2)];
    let t = ToneMapping { white_point: lin, ..ToneMapping::default() };
    let g = t.white_gains();
    // The white point itself maps to grey at its own mean.
    let out = t.iray_rgb(lin.map(|v| v * 0.1));
    assert!((out[0] - out[1]).abs() < 1e-12 && (out[1] - out[2]).abs() < 1e-12);
    // Not Octane's imager: a LUT with the gains per channel.
    assert!(!t.is_plain());
    let lut = t.cube_lut();
    let row: Vec<f64> = lut.lines().filter(|l| l.starts_with(|c: char| c.is_ascii_digit())).nth(1000).unwrap().split(' ').map(|x| x.parse().unwrap()).collect();
    let x = (1000.0f64 / 4095.0).powf(2.2) * LUT_HEADROOM;
    for i in 0..3 {
        assert!((row[i] - (x * g[i]).powf(1.0 / 2.2).min(1.0)).abs() < 1e-6, "{row:?}");
    }
    // A grey white point at any scale changes nothing (Calib-Tone-WhiteScale2).
    let grey = ToneMapping { white_point: [2.0; 3], ..ToneMapping::default() };
    assert!(grey.is_plain() && grey.white_gains().iter().all(|g| (g - 1.0).abs() < 1e-12));
}

#[test]
fn luminance_burn_lut() {
    // Per component off: the luminance is burnt, the chromaticity kept.
    let t = ToneMapping { burn_highlights: 0.5, burn_per_component: false, ..ToneMapping::default() };
    let per = ToneMapping { burn_per_component: true, ..t.clone() };
    let c = [2.0, 0.5, 0.1];
    let y = 0.2126 * 2.0 + 0.7152 * 0.5 + 0.0722 * 0.1;
    let k = (1.0 + 0.5 * y) / (1.0 + y);
    let out = t.iray_rgb(c);
    for i in 0..3 {
        assert!((out[i] - (c[i] * k).powf(1.0 / 2.2)).abs() < 1e-12);
    }
    assert_ne!(out, per.iray_rgb(c));
    // Burn 1 compresses nothing: the 1D LUT, Octane's own response.
    assert!(ToneMapping { burn_highlights: 1.0, ..t.clone() }.cube_lut().contains("LUT_1D_SIZE"));
    // The 3D LUT, trilinearly read at its input (the Gamma 2.2 response of
    // c / 16) and clipped at white, stays within 0.2 / 255 of the curve.
    let lut = t.cube_lut();
    let n: usize = lut.lines().find_map(|l| l.strip_prefix("LUT_3D_SIZE ")).unwrap().parse().unwrap();
    let rows: Vec<[f64; 3]> = lut.lines().filter(|l| l.starts_with(|c: char| c.is_ascii_digit())).map(|l| {
        let v: Vec<f64> = l.split(' ').map(|x| x.parse().unwrap()).collect();
        [v[0], v[1], v[2]]
    }).collect();
    assert_eq!(rows.len(), n * n * n);
    let at = |r: usize, g: usize, b: usize| rows[r + n * (g + n * b)];
    let read = |c: [f64; 3]| -> [f64; 3] {
        let x = c.map(|v| (v / LUT_HEADROOM).powf(1.0 / 2.2) * (n - 1) as f64);
        let i = x.map(|v| (v.floor() as usize).min(n - 2));
        let f: Vec<f64> = (0..3).map(|k| x[k] - i[k] as f64).collect();
        let mut out = [0.0; 3];
        for corner in 0..8 {
            let d = [corner & 1, (corner >> 1) & 1, corner >> 2];
            let w: f64 = (0..3).map(|k| if d[k] == 1 { f[k] } else { 1.0 - f[k] }).product();
            let v = at(i[0] + d[0], i[1] + d[1], i[2] + d[2]);
            (0..3).for_each(|k| out[k] += w * v[k]);
        }
        out
    };
    let mut worst: f64 = 0.0;
    for r in 0..20 {
        for g in 0..20 {
            for b in 0..20 {
                let c = [r, g, b].map(|v| (v as f64 / 19.0).powi(2) * 1.5 + 0.003);
                let (want, got) = (t.iray_rgb(c).map(|v| v.min(1.0)), read(c).map(|v| v.min(1.0)));
                worst = (0..3).map(|k| (want[k] - got[k]).abs()).fold(worst, f64::max);
            }
        }
    }
    assert!(worst * 255.0 < 0.2, "{}", worst * 255.0);
}

#[test]
fn iray_light_flux_rules() {
    let light = |shape: LightShape, lumens: f64| SceneLight {
        label: "L".into(),
        transform: [[0.0; 4]; 4],
        shape,
        cone_deg: None,
        beam_exponent: 1.0,
        color: [1.0; 3],
        kelvin: None,
        lumens,
        euler_deg: [0.0; 3],
        rotation_order: "XYZ".into(),
        translation: [0.0; 3],
        two_sided: false,
        visible: true,
        profile: None,
    };
    // A sphere light gives a quarter of a point light of the same flux.
    let point = light_watts(&light(LightShape::Point, 5000.0));
    let sphere = light_watts(&light(LightShape::Sphere { radius: 0.1 }, 5000.0));
    assert!((sphere / point - 0.25).abs() < 1e-12);
    // Non-photometric: 1000 π lm per unit of Intensity (renders matched
    // it before DAZ's .mi export showed it exactly).
    assert!((crate::iray::LEGACY_LUMENS - 3141.59).abs() < 0.01);
}

#[test]
fn hidden_dome_gets_a_black_backplate() {
    let cams = vec![("Camera".to_string(), framing_camera(&[])), ("Other".to_string(), framing_camera(&[]))];
    let shown = project_ocs("Shot", "scene.usdz", &cams, (640, 480), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &ToneMapping::default(), false, None, true, &crate::iray::RenderOptions::default());
    assert!(!shown.contains("cameraEnvironment'") && !shown.contains("@VISIBLE_ENVIRONMENT@"));
    let hidden = project_ocs("Shot", "scene.usdz", &cams, (640, 480), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &ToneMapping::default(), true, None, true, &crate::iray::RenderOptions::default());
    // One per render target, each with its own ids.
    assert_eq!(hidden.matches("<pin name='cameraEnvironment'>").count(), 2);
    assert!(hidden.contains("id='100195'") && hidden.contains("id='1100195'"));
}

#[test]
fn dome_only_and_sun_sky_drop_the_scene_lights() {
    let ch = |id: &str, v: Value| json!({"channel": {"id": id, "current_value": v}});
    let duf = |mode: i64| {
        json!({
            "node_library": [
                {"id": "Point", "type": "light", "point": {}, "extra": [{"type": "studio_node_channels", "channels": [ch("Flux", json!(5000)), ch("Photometric Mode", json!(true))]}]},
                {"id": "Environment Options", "extra": [{"type": "studio_node_channels", "channels": [ch("Environment Mode", json!(mode))]}]},
            ],
            "scene": {"nodes": [{"id": "Point-1", "url": "#Point", "label": "Point"},
                                {"id": "Environment Options-1", "url": "#Environment%20Options"}]},
        })
    };
    for (mode, lights) in [(0, 1), (1, 0), (2, 0), (3, 1)] {
        assert_eq!(lights_iray_uses(&mut Library::new(vec![], duf(mode))).len(), lights, "Environment Mode {mode}");
    }
}

#[test]
fn hidden_in_the_scenes_own_node_definition() {
    // A primitive made in DAZ: its node is defined in the scene file, and
    // hiding it sets `Visible` there (Calib-Leaf's cubes), not on the node.
    let duf = json!({
        "node_library": [{"id": "pCube(1)", "extra": [{"type": "studio_node_channels",
            "channels": [{"channel": {"id": "Visible", "current_value": false}}]}]}],
        "scene": {"nodes": [{"id": "pCube(1)-1", "url": "#pCube%281%29", "name": "pCube(1)", "label": "Cube1",
            "geometries": [{"id": "geometry-1", "url": "#geometry"}]}]},
    });
    let nodes = super::mesh::geo_nodes(&duf);
    assert_eq!(nodes.len(), 1);
    assert!(!nodes[0].visible);
}

#[test]
fn hidden_under_a_hidden_group() {
    // Calib-Hair-Teens (#53): a hidden group hides what's under it, a hidden
    // figure (the dolly, a scalp cap under dForce hair) doesn't.
    let hidden = json!([{"type": "studio_node_channels", "channels": [{"channel": {"id": "Visible", "current_value": false}}]}]);
    let group = json!([{"type": "studio/node/group_node"}]);
    let duf = json!({
        "node_library": [{"id": "Group 2", "type": "node", "extra": group}],
        "scene": {"nodes": [
            {"id": "Group 2-1", "url": "#Group%202", "label": "Group", "extra": hidden},
            {"id": "Dolly", "label": "Dolly", "parent": "#Group%202-1", "geometries": [{"id": "d", "url": "#dolly"}]},
            {"id": "base-1", "label": "base", "parent": "#Dolly"},
            {"id": "Eva", "label": "Eva", "parent": "#base-1", "geometries": [{"id": "e", "url": "#eva"}]},
            {"id": "Scalp", "label": "Scalp", "extra": hidden, "geometries": [{"id": "s", "url": "#scalp"}]},
            {"id": "Hair", "label": "Hair", "parent": "#Scalp", "geometries": [{"id": "h", "url": "#hair"}]},
        ]},
    });
    let visible: HashMap<String, bool> = super::mesh::geo_nodes(&duf).into_iter().map(|n| (n.label, n.visible)).collect();
    assert_eq!(visible, HashMap::from([("Dolly".into(), false), ("Eva".into(), false), ("Scalp".into(), false), ("Hair".into(), true)]));
}

#[test]
fn open_sheets_and_closed_volumes() {
    // A tetrahedron split into two material groups (closed: its seam is no
    // edge), beside a lone quad card (open).
    let tet = Mesh {
        face_counts: vec![3; 4],
        face_indices: vec![0, 1, 2, 0, 3, 1, 1, 3, 2, 2, 3, 0],
        subsets: vec![("a".into(), vec![0, 1]), ("b".into(), vec![2, 3])],
        ..Default::default()
    };
    assert_eq!(super::mesh::open_subsets(&tet), [false, false]);
    let card = Mesh { face_counts: vec![4], face_indices: vec![0, 1, 2, 3], subsets: vec![("leaf".into(), vec![0])], ..Default::default() };
    assert_eq!(super::mesh::open_subsets(&card), [true]);
}

#[test]
fn iray_sun_sky_becomes_octane_daylight() {
    // DAZ's default Sun-Sky: Salt Lake City, 2015-03-10 18:00 (UTC −6) — the
    // sun low in the west-south-west (elevation 15.6°, azimuth 250.5°).
    let d = sun_direction(40.76426, -111.8972, 2_457_092.0, 64_800.0, -6.0);
    let el = d[1].asin().to_degrees();
    let az = d[0].atan2(-d[2]).to_degrees().rem_euclid(360.0);
    assert!((el - 15.55).abs() < 0.1 && (az - 250.5).abs() < 0.2, "{el} {az}");
    // Noon in midsummer on the equator's tropic: overhead.
    let noon = sun_direction(23.44, 0.0, 2_460_483.0, 43_200.0, 0.0);
    assert!(noon[1] > 0.99, "{noon:?}");

    let sky = SunSky { sun_dir: d, turbidity: 2.0, power: 1.0, sun_intensity: 1.0, sun_size: 1.0, ground: [0.13; 3], model: 3, ground_blend_deg: 1.0 };
    let cam = [("Camera".to_string(), framing_camera(&[]))];
    let ocs = project_ocs("Sun", "scene.usdz", &cam, (640, 480), "", &Variant::new(Look::Enhanced, OctaneMode::Final).tuning(), &ToneMapping::default(), false, Some(&sky), true, &crate::iray::RenderOptions::default());
    assert!(ocs.contains("<node id='100210' type='14'>") && ocs.contains("<pin name='sundir'>"), "daylight environment");
    assert!(ocs.contains("<pin name='groundBlendAngle'>"), "ground blend");
    // Iray's horizon blur 2: all ground 11.5° below the horizon; DAZ's 0.1
    // below Octane's 1° minimum.
    assert!((crate::usd::environment::ground_blend_angle(2.0) - 0.76 * 11.537).abs() < 0.01);
    assert_eq!(crate::usd::environment::ground_blend_angle(0.1), 1.0);
    assert!(!ocs.contains("<node id='100035' type='37'>"), "no texture environment on the render target");
}

#[test]
fn sun_sky_steered_by_a_node() {
    // "SS Sun Direction" names a distant light straight above the origin
    // (turned, which Iray ignores): the sun stands overhead, whatever the
    // date says.
    let duf = json!({
        "node_library": [
            {"id": "Environment Options", "extra": [{"type": "studio_node_channels", "channels": [
                {"channel": {"id": "Environment Mode", "current_value": 2}}]}]},
            {"id": "Sun", "type": "light", "directional": {}}
        ],
        "scene": {"nodes": [
            {"id": "Environment Options-1", "url": "#Environment%20Options", "extra": [{"type": "studio_node_channels", "channels": [
                {"channel": {"id": "SS Sun Direction", "type": "node", "node": "#Sun-1"}}]}]},
            {"id": "Sun-1", "url": "#Sun", "rotation": [{"id": "x", "current_value": 45}], "translation": [{"id": "y", "current_value": 500}]}
        ]},
    });
    let mut lib = Library::new(vec![], duf);
    let sky = scene_environment(&mut lib).unwrap().sun_sky.unwrap();
    assert!((sky.sun_dir[1] - 1.0).abs() < 1e-9, "{:?}", sky.sun_dir);
}

#[test]
fn camera_headlamp_when_iray_lights_with_it() {
    // A camera at the origin looking down −Z with its headlamp on, 30 cm to
    // the left and 20 cm up; then Auto with the render setting at 1 (no
    // lights in the scene) and 0.
    let scene = |mode: i64, auto: i64| {
        json!({
            "node_library": [{"id": "Camera", "type": "camera", "extra": [{"type": "studio_node_channels", "channels": [
                {"channel": {"id": "HeadlampMode", "current_value": mode}},
                {"channel": {"id": "XHeadlampOffset", "current_value": -30}},
                {"channel": {"id": "YHeadlamp", "current_value": 20}},
                {"channel": {"id": "Depth of Field", "current_value": 420.543}}]}]}],
            "scene": {"nodes": [{"id": "Camera-1", "url": "#Camera", "label": "Camera"}],
                      "extra": [{"type": "studio_render_settings", "render_options": {"render_elements": [
                          {"id": "General Render", "channels": [{"channel": {"id": "Auto Headlamp", "current_value": auto}}]}]}}]},
        })
    };
    let cam = SceneCamera { pos: [0.0; 3], target: [0.0, 0.0, -1.0], up: [0.0, 1.0, 0.0], fov_deg: 40.0, focal_depth: 1.0, aperture_cm: 0.0, film_cm: [3.6, 2.4], focal_cm: 5.0, lens: crate::iray::Lens::default() };
    let h = headlamp(&mut Library::new(vec![], scene(1, 0)), "Camera", &cam, true).expect("on");
    assert!((h.translation[0] + 0.3).abs() < 1e-6 && (h.translation[1] - 0.2).abs() < 1e-6, "{:?}", h.translation);
    // DAZ's 10 × 420.543² lm (Calib-Glossy's focus) as the calibrated 1.7 Mlm point.
    assert!((h.lumens / 1.7e6 - 1.0).abs() < 1e-5, "{}", h.lumens);
    assert!(headlamp(&mut Library::new(vec![], scene(0, 1)), "Camera", &cam, false).is_some(), "auto, no lights");
    assert!(headlamp(&mut Library::new(vec![], scene(0, 1)), "Camera", &cam, true).is_none(), "auto 1 with lights");
    assert!(headlamp(&mut Library::new(vec![], scene(0, 2)), "Camera", &cam, true).is_none(), "auto 2 with lights");
    assert!(headlamp(&mut Library::new(vec![], scene(1, 0)), "Camera", &cam, true).is_some(), "on, with lights");
    assert!(headlamp(&mut Library::new(vec![], scene(0, 0)), "Camera", &cam, false).is_none(), "auto 0");
    assert!(headlamp(&mut Library::new(vec![], scene(2, 2)), "Camera", &cam, false).is_none(), "off");
}

#[test]
fn cast_shadows_off_still_casts() {
    // DAZ doesn't hand Iray a node's Cast Shadows (#36): the mesh is
    // exported as any other, and the template loads no object layers.
    let vis = |id: &str, on: bool| json!({"channel": {"id": id, "current_value": on}});
    let meshes = host_with_channels(json!([vis("Cast Shadows", false)]));
    let faces = |m: &[Mesh]| m.iter().map(|m| (m.name.clone(), m.subsets.len(), m.points.len())).collect::<Vec<_>>();
    assert_eq!(faces(&meshes), faces(&host_with_channels(json!([]))));
    assert!(include_str!("../../assets/octane/project.ocs").contains(OBJECT_LAYERS_HIDDEN), "the template's import setting");
}

#[test]
fn headlamp_sits_where_daz_puts_it() {
    // DAZ's .mi export of the headlamp scenes (calib/references/mi): the
    // rect's world position — its instance transform is world → object, so
    // the origin maps back as −t · Rᵀ (cm).
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for scene in ["Calib-Headlamp-on-a1", "Calib-Mi-Headlamp-offset0", "Calib-Mi-Headlamp-dof200", "Calib-Mi-Headlamp-i2"] {
        let mi: Value = serde_json::from_str(&std::fs::read_to_string(root.join(format!("calib/references/mi/{scene}.json"))).unwrap()).unwrap();
        let t: Vec<Vec<f64>> = mi["lights"]["Camera"]["transform"].as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()).collect();
        let daz = [0, 1, 2].map(|i| -(0..3).map(|j| t[3][j] * t[i][j]).sum::<f64>() * CM as f64);
        let duf: Value = serde_json::from_str(&decompress_scene(&std::fs::read(root.join(format!("calib/scenes/{scene}.duf"))).unwrap()).unwrap()).unwrap();
        let mut lib = Library::new(vec![], duf);
        let (label, cam) = scene_cameras(&mut lib).into_iter().next().unwrap();
        let ours = headlamp(&mut lib, &label, &cam, false).expect("lit").translation;
        assert!((0..3).all(|i| (ours[i] - daz[i]).abs() < 1e-4), "{scene}: DAZ {daz:?}, ours {ours:?}");
        checked += 1;
    }
    assert_eq!(checked, 4);
}

#[test]
fn subd_levels_add_up_as_daz_renders_them() {
    // DAZ's .mi export of Genesis 8 Female (definition 1 + 1): the SubD
    // Level and Render SubD Level add up; the scene's instance overrides.
    assert_eq!(mesh::subd_levels([None, None], [Some(1), Some(1)]), 2);
    assert_eq!(mesh::subd_levels([None, Some(0)], [Some(1), Some(1)]), 1, "the lashes: render level 0");
    assert_eq!(mesh::subd_levels([Some(0), Some(0)], [Some(1), Some(1)]), 0);
    assert_eq!(mesh::subd_levels([None, None], [None, None]), 1, "DAZ's defaults 1 + 0");
    assert_eq!(mesh::subd_levels([Some(2), Some(2)], [None, None]), 3, "capped");
}

#[test]
fn render_emitter_sets_whether_the_camera_sees_a_light() {
    let ch = |id: &str, v: Value| json!({"channel": {"id": id, "current_value": v}});
    let light = |id: &str, kind: &str, extra: Vec<Value>| {
        let mut channels = vec![ch("Photometric Mode", json!(true)), ch("Flux", json!(5000)), ch("Light Geometry", json!(1))];
        channels.extend(extra);
        let mut def = json!({"id": id, "type": "light", "extra": [{"type": "studio_node_channels", "channels": channels}]});
        def[kind] = json!({});
        def
    };
    // Options DAZ doesn't hand Iray (Calib-L-Opt-*): nothing changes.
    let ignored = vec![ch("Cast Shadows", json!(false)), ch("Shadow Intensity", json!(0.0)), ch("Shadow Softness", json!(0.5)), ch("Shadow Color", json!([0.3, 0, 0])), ch("Decay", json!(0)), ch("Illumination", json!(1)), ch("Light Portal", json!(true))];
    let duf = json!({
        "node_library": [light("Shown", "point", vec![]), light("Hidden", "point", vec![ch("Render Emitter", json!(false))]),
            light("Ignored", "point", ignored), light("Spot", "spot", vec![ch("Render Emitter", json!(false))])],
        "scene": {"nodes": [{"id": "a", "url": "#Shown", "label": "Shown"}, {"id": "b", "url": "#Hidden", "label": "Hidden"},
            {"id": "c", "url": "#Ignored", "label": "Ignored"}, {"id": "d", "url": "#Spot", "label": "Spot"}]},
    });
    let lights = scene_lights(&mut Library::new(vec![], duf));
    assert_eq!(lights.iter().map(|l| l.visible).collect::<Vec<_>>(), [true, false, true, false]);
    let (shown, ignored) = (&lights[0], &lights[2]);
    assert_eq!(SceneLight { label: shown.label.clone(), ..ignored.clone() }, *shown, "Iray ignores them, so does the export");
    // UsdLux lights: Octane hides them from the camera unless an object
    // layer input shows them — the rect light that Iray shows does.
    assert_eq!(camera_shown_lights(&lights), ["Lights/Shown", "Lights/Ignored"]);
    let point = SceneLight { shape: LightShape::Point, ..shown.clone() };
    assert!(camera_shown_lights(&[point]).is_empty(), "a point has no face to see");
    let xml = crate::material::object_layer_inputs(&[("Lights/Shown".into(), "camera_visibility", true)], 160_000);
    assert!(xml.contains("type='20017' name='/Scene/Lights/Shown_object'") && xml.contains("<pin name='camera_visibility'>"), "{xml}");
    assert!(xml.contains("<attr name='value' type='1'>1</attr>"), "{xml}");
    // The spot (an analytic light): its own object layer.
    let spots = octane_spots(&lights);
    let (nodes, _) = crate::material::scene_lights(&spots, None, None, &[], 500, 1439);
    assert!(nodes.contains("<pin name='objectLayer'>") && nodes.contains("<pin name='camera_visibility'>"), "{nodes}");
    assert_eq!(nodes.matches("<node ").count(), nodes.matches("</node>").count());
    let (shown_spot, _) = crate::material::scene_lights(&[crate::material::OctaneSpot { visible: true, ..spots[0].clone() }], None, None, &[], 500, 1439);
    assert!(!shown_spot.contains("objectLayer"));
}

#[test]
fn ground_fog_slabs_follow_iray_exponential_decay() {
    // DAZ's default fog: 0.98 / m at the ground, 1e-4 at 0.9144 m.
    let fog = crate::iray::GroundFog { albedo: [0.4; 3], density0: [0.98; 3], density1: [1e-4; 3], height0: 0.0, height1: 0.9144, mie: false, anisotropy: 0.0, particle_size: 0.0 };
    let decay = fog.decay();
    assert!((decay - 0.9144 / (0.98f64 / 1e-4).ln()).abs() < 1e-9);
    let s = ground_fog_slabs(&fog);
    // The uniform part below, then the decay up to 1/1000 of the ground's density.
    assert_eq!((s[0].y0, s[0].y1, s[0].extinction), (-FOG_RADIUS, 0.0, [0.98; 3]));
    assert!(s.len() - 1 <= FOG_SLABS && (s.last().unwrap().y1 - decay * 1000f64.ln()).abs() < 1e-9);
    // Contiguous, thinning, and each slab's optical depth that of the decay over it.
    for w in s[1..].windows(2) {
        assert!((w[0].y1 - w[1].y0).abs() < 1e-12 && w[1].extinction[0] < w[0].extinction[0]);
    }
    for x in &s[1..] {
        let exact = 0.98 * decay * ((-x.y0 / decay).exp() - (-x.y1 / decay).exp());
        assert!((x.extinction[0] * (x.y1 - x.y0) - exact).abs() < 1e-9);
    }
    // Equal densities: uniform, unbounded upward (one tall slab).
    let uniform = crate::iray::GroundFog { density1: [0.98; 3], ..fog };
    assert_eq!(ground_fog_slabs(&uniform).len(), 2);
    assert_eq!(crate::material::unit_box([-1.0, 0.0, -1.0], [1.0, 0.5, 1.0]), [2.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.25, 0.0, 0.0, 2.0, 0.0]);
}

#[test]
fn dome_ground_is_an_emitting_slab_with_the_hdri_from_the_virtual_camera() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("Environments/HDRI")).unwrap();
    std::fs::write(dir.path().join("Environments/HDRI/sky.hdr"), b"#?RADIANCE").unwrap();
    let env = SceneEnvironment { map: Some("/Environments/HDRI/sky.hdr".into()), intensity: 2.0, tint: [1.0; 3], rotation_deg: 0.0, orientation_deg: [0.0; 3], scene_only: false, ground_shadow: None, ground_origin_y: None, draw_dome: true, scene_lights: true, sun_sky: None, ground_fog: None, dome_ground: Some(1.0) };
    let mut tex = crate::material::Textures::default();
    let t = crate::material::projected_environment_texture(&env, &[dir.path().to_path_buf()], &mut tex, 10, [0.0, 1.25, 0.0]).unwrap();
    // A world-space Spherical projection at the virtual camera, mirrored in X.
    assert!(t.contains("type='77'") && t.contains("<attr name='translation' type='8'>0 1.25 0</attr>") && t.contains("<attr name='scale' type='8'>-1 1 1</attr>"), "{t}");
    // The environment's own lookup stays unmirrored.
    assert!(!crate::material::environment_texture(Some(&env), &[dir.path().to_path_buf()], &mut tex, 10).contains("-1 1 1"));
    let g = crate::material::DomeGround { texture: t, height: 0.25, power: 2.0 };
    let (x, mesh) = crate::material::scene_lights(&[], None, Some(&g), &[], 500, 1439);
    assert!(mesh != 1439 && x.contains("type='296' name='DAZ dome ground slab'") && x.contains("type='54'") && x.contains("<attr name='value' type='8'>2 0 0</attr>"), "{x}");
    // Its top 2 mm under the ground: 0.25 − 0.002 − 0.0005.
    assert!(x.contains("<attr name='translation' type='8'>0 0.2475 0</attr>"), "{x}");
    assert!(crate::material::projected_environment_texture(&SceneEnvironment { map: None, ..env }, &[], &mut tex, 10, [0.0; 3]).is_none());
}

/// A cube (side 2, centred) as quads, outward winding.
fn unit_cube() -> (Vec<[f32; 3]>, Vec<u32>, Vec<u32>) {
    let c = [-1.0, 1.0];
    let points = (0..8).map(|i| [c[i & 1 ^ (i >> 1 & 1)], c[i >> 1 & 1], c[i >> 2]]).collect::<Vec<_>>();
    let faces = [[0, 3, 2, 1], [4, 5, 6, 7], [0, 1, 5, 4], [2, 3, 7, 6], [1, 2, 6, 5], [0, 4, 7, 3]];
    (points, vec![4; 6], faces.concat())
}

#[test]
fn smoothing_splits_edges_sharper_than_the_angle() {
    let (p, counts, idx) = unit_cube();
    let deg = |a: f32| a.to_radians().cos();
    // DAZ's default 89.9°: the 90° edges are hard, every corner its face's normal.
    let (n, fv) = smoothed_normals(&p, &counts, &idx, &[deg(89.9); 6]);
    let fv = fv.expect("split");
    assert_eq!((n.len(), fv.len()), (24, 24));
    assert!(close(n[fv[4] as usize].map(f32::abs), [0.0, 0.0, 1.0]), "face 1 lies at z = 1");
    // 91°: one normal per vertex, along the diagonal (outward or not).
    let (n, fv) = smoothed_normals(&p, &counts, &idx, &[deg(91.0); 6]);
    assert!(fv.is_none() && n.len() == 8);
    let k = 1.0 / 3f32.sqrt();
    assert!(close(n[6].map(f32::abs), [k, k, k]), "{:?}", n[6]);
    // Smooth off on one face: its corners alone, the rest stays smooth.
    let mut cos = [deg(91.0); 6];
    cos[1] = smoothing_cos(Some(&crate::daz_material::Material::from_dson(&json!({"extra": [{"channels": [{"channel": {"id": "Smooth On", "current_value": false}}]}]}), &HashMap::new())));
    let (n, fv) = smoothed_normals(&p, &counts, &idx, &cos);
    let fv = fv.unwrap();
    assert!(close(n[fv[4] as usize].map(f32::abs), [0.0, 0.0, 1.0]), "the faceted face");
    assert!(n[fv[0] as usize][0].abs() > 0.5, "the opposite face smoothed with the sides: {:?}", n[fv[0] as usize]);
}

#[test]
fn smoothing_joins_faces_through_soft_edges() {
    // An icosahedron: edges 41.8°, faces across a vertex further apart.
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let p: Vec<[f32; 3]> = [[-1.0, t, 0.0], [1.0, t, 0.0], [-1.0, -t, 0.0], [1.0, -t, 0.0], [0.0, -1.0, t], [0.0, 1.0, t], [0.0, -1.0, -t], [0.0, 1.0, -t], [t, 0.0, -1.0], [t, 0.0, 1.0], [-t, 0.0, -1.0], [-t, 0.0, 1.0]].to_vec();
    let f = [[0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11], [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8], [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9], [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1]];
    let idx: Vec<u32> = f.concat();
    let counts = vec![3; 20];
    let deg = |a: f32| a.to_radians().cos();
    // 45° (Calib-Smooth S3): smooth, the vertex normal radial.
    let (n, fv) = smoothed_normals(&p, &counts, &idx, &[deg(45.0); 20]);
    assert!(fv.is_none());
    let r = (1.0 + t * t).sqrt();
    assert!(close(n[0], [-1.0 / r, t / r, 0.0]), "{:?}", n[0]);
    // 30° (S2): faceted, 60 corners.
    let (n, fv) = smoothed_normals(&p, &counts, &idx, &[deg(30.0); 20]);
    assert_eq!((n.len(), fv.map(|v| v.len())), (60, Some(60)));
}

#[test]
fn split_normals_are_written_as_indexed_primvars() {
    let (points, face_counts, face_indices) = unit_cube();
    let (normals, normal_indices) = smoothed_normals(&points, &face_counts, &face_indices, &[0.5; 6]);
    let m = Mesh { name: "Box".into(), label: "Box".into(), points, face_counts, face_indices, normals: Some(normals), normal_indices, ..Default::default() };
    let (o, _) = write_usda(&[m], &[]);
    assert!(o.contains("normal3f[] primvars:normals = ") && o.contains("interpolation = \"faceVarying\"") && o.contains("int[] primvars:normals:indices = "), "{o}");
    assert!(!o.contains("normal3f[] normals"), "plain `normals` Octane ignores");
}

#[test]
fn round_corners_not_across_materials_split_the_mesh() {
    // Two quads sharing an edge (points 1, 4), A and B; B rounds but not
    // across materials (Calib-RoundCorners R8) → a mesh of its own.
    let rc = |across: bool| {
        Some(Rc::new(crate::daz_material::Material::from_dson(
            &json!({"extra": [{"channels": [
                {"channel": {"id": "Round Corners Radius", "current_value": 0.01}},
                {"channel": {"id": "Round Corners Across Materials", "current_value": across}},
            ]}]}),
            &HashMap::new(),
        )))
    };
    let pts: Vec<[f32; 3]> = (0..6).map(|i| [(i % 3) as f32, (i / 3) as f32, 0.0]).collect();
    let whole = |b| Mesh {
        name: "Box".into(),
        points: pts.clone(),
        face_counts: vec![4, 4],
        face_indices: vec![0, 1, 4, 3, 1, 2, 5, 4],
        subsets: vec![("A".into(), vec![0]), ("B".into(), vec![1])],
        materials: vec![rc(true), b],
        uvs: Some(((0..8).map(|i| [i as f32, 0.0]).collect(), (0..8).collect())),
        normals: Some(vec![[0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]),
        normal_indices: Some(vec![0, 0, 0, 0, 1, 1, 1, 1]),
        ..Default::default()
    };
    // Across on (or no Round Corners): untouched.
    let kept = split_round_corner_borders(vec![whole(rc(true)), whole(None)]);
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].face_indices, vec![0, 1, 4, 3, 1, 2, 5, 4]);
    let parts = split_round_corner_borders(vec![whole(rc(false))]);
    assert_eq!(parts.len(), 2);
    let (a, b) = (&parts[0], &parts[1]);
    assert_eq!((a.name.as_str(), b.name.as_str()), ("Box", "Box_B"));
    assert_eq!(a.points, vec![pts[0], pts[1], pts[4], pts[3]]);
    assert_eq!(b.points, vec![pts[1], pts[2], pts[5], pts[4]]);
    assert_eq!((a.face_indices.clone(), b.face_indices.clone()), (vec![0, 1, 2, 3], vec![0, 1, 2, 3]));
    assert_eq!(b.subsets, vec![("B".to_string(), vec![0])]);
    assert!(b.materials[0].is_some() && a.materials[0].is_some());
    assert_eq!(b.normals, Some(vec![[1.0, 0.0, 0.0]]));
    assert_eq!(b.normal_indices, Some(vec![0; 4]));
    assert_eq!(b.uvs, Some(((4..8).map(|i| [i as f32, 0.0]).collect(), (0..4).collect())));
    let (usda, _) = write_usda(&parts, &[]);
    assert!(usda.contains("def Mesh \"Box_B\""), "{usda}");
}

#[test]
fn emission_profile_surface_normals_by_area() {
    // A 2 × 1 quad facing −Y and a 1 × 1 quad facing +X.
    let points = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
    let n = mesh::face_normal_shares(&points, &[4, 4], &[0, 1, 2, 3, 7, 6, 5, 4], &[0, 4], &[0, 1]);
    assert_eq!(n.len(), 2);
    assert!((n[0].0[1] + 1.0).abs() < 1e-9 && (n[0].1 - 2.0 / 3.0).abs() < 1e-9, "{n:?}");
    assert!((n[1].0[0].abs() - 1.0).abs() < 1e-9 && (n[1].1 - 1.0 / 3.0).abs() < 1e-9, "{n:?}");
}

/// A `.usdc` read back as far as the tests look: tokens, paths and each
/// spec's fields (`usdc.rs` writes LZ4 blocks of literals only, but this
/// reads any LZ4).
struct Crate {
    data: Vec<u8>,
    tokens: Vec<String>,
    strings: Vec<u32>,
    paths: Vec<String>,
    /// (path, spec type, fields as (name, value rep)).
    specs: Vec<(String, u32, Vec<(String, u64)>)>,
}

impl Crate {
    fn prims(&self) -> Vec<(String, String)> {
        self.specs.iter().filter(|s| s.1 == 6).map(|s| (s.0.clone(), self.token_field(&s.0, "typeName").unwrap_or_default())).collect()
    }

    fn field(&self, path: &str, name: &str) -> Option<u64> {
        let spec = self.specs.iter().find(|s| s.0 == path)?;
        spec.2.iter().find(|f| f.0 == name).map(|f| f.1)
    }

    fn token_field(&self, path: &str, name: &str) -> Option<String> {
        self.field(path, name).map(|r| self.tokens[(r & 0xffff_ffff) as usize].clone())
    }

    fn u64_at(&self, at: usize) -> u64 {
        u64::from_le_bytes(self.data[at..at + 8].try_into().unwrap())
    }

    /// An array value: its element count and where its data starts.
    fn array(&self, rep: u64) -> (usize, usize) {
        let at = (rep & ((1 << 48) - 1)) as usize;
        (self.u64_at(at) as usize, at + 8)
    }

    fn ints(&self, rep: u64) -> Vec<u32> {
        let (n, at) = self.array(rep);
        if rep & (1 << 61) != 0 {
            let size = self.u64_at(at) as usize;
            decode_ints(&self.data[at + 8..at + 8 + size], n)
        } else {
            (0..n).map(|k| u32::from_le_bytes(self.data[at + 4 * k..at + 4 * k + 4].try_into().unwrap())).collect()
        }
    }

    fn floats(&self, rep: u64) -> Vec<f32> {
        let (n, at) = self.array(rep);
        (0..n).map(|k| f32::from_le_bytes(self.data[at + 4 * k..at + 4 * k + 4].try_into().unwrap())).collect()
    }
}

fn lz4_decode(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    let len = |i: &mut usize, mut l: usize| {
        if l == 15 {
            loop {
                let b = src[*i];
                *i += 1;
                l += b as usize;
                if b != 255 {
                    break;
                }
            }
        }
        l
    };
    while i < src.len() {
        let t = src[i];
        i += 1;
        let l = len(&mut i, (t >> 4) as usize);
        out.extend_from_slice(&src[i..i + l]);
        i += l;
        if i >= src.len() {
            break;
        }
        let off = u16::from_le_bytes([src[i], src[i + 1]]) as usize;
        i += 2;
        let m = len(&mut i, (t & 15) as usize) + 4;
        for _ in 0..m {
            out.push(out[out.len() - off]);
        }
    }
    out
}

/// TfFastCompression framing: one block, or a count of int32-sized chunks.
fn fast_decode(buf: &[u8]) -> Vec<u8> {
    if buf[0] == 0 {
        return lz4_decode(&buf[1..]);
    }
    let (mut out, mut p) = (Vec::new(), 1);
    for _ in 0..buf[0] {
        let n = i32::from_le_bytes(buf[p..p + 4].try_into().unwrap()) as usize;
        out.extend(lz4_decode(&buf[p + 4..p + 4 + n]));
        p += 4 + n;
    }
    out
}

fn decode_ints(buf: &[u8], n: usize) -> Vec<u32> {
    let d = fast_decode(buf);
    let common = i32::from_le_bytes(d[..4].try_into().unwrap());
    let codes = &d[4..4 + (2 * n).div_ceil(8)];
    let mut p = 4 + codes.len();
    let mut prev = 0i32;
    (0..n)
        .map(|k| {
            let v = match (codes[k / 4] >> (2 * (k % 4))) & 3 {
                0 => common,
                1 => {
                    p += 1;
                    d[p - 1] as i8 as i32
                }
                2 => {
                    p += 2;
                    i16::from_le_bytes([d[p - 2], d[p - 1]]) as i32
                }
                _ => {
                    p += 4;
                    i32::from_le_bytes(d[p - 4..p].try_into().unwrap())
                }
            };
            prev = prev.wrapping_add(v);
            prev as u32
        })
        .collect()
}

fn read_usdc(data: &[u8]) -> Crate {
    assert_eq!(&data[..8], b"PXR-USDC");
    assert_eq!(&data[8..11], &[0, 8, 0]);
    let mut c = Crate { data: data.to_vec(), tokens: vec![], strings: vec![], paths: vec![], specs: vec![] };
    let toc = c.u64_at(16) as usize;
    let mut sections = HashMap::new();
    for k in 0..c.u64_at(toc) as usize {
        let at = toc + 8 + 32 * k;
        let name = String::from_utf8(data[at..at + 16].iter().take_while(|&&b| b != 0).copied().collect()).unwrap();
        sections.insert(name, c.u64_at(at + 16) as usize);
    }
    let compressed = |at: usize, n: usize| -> (Vec<u32>, usize) {
        let size = u64::from_le_bytes(data[at..at + 8].try_into().unwrap()) as usize;
        (decode_ints(&data[at + 8..at + 8 + size], n), at + 8 + size)
    };
    let at = sections["TOKENS"];
    let n = c.u64_at(at) as usize;
    let size = c.u64_at(at + 16) as usize;
    let text = fast_decode(&data[at + 24..at + 24 + size]);
    assert_eq!(text.len(), c.u64_at(at + 8) as usize);
    c.tokens = text.split(|&b| b == 0).take(n).map(|t| String::from_utf8(t.to_vec()).unwrap()).collect();
    let at = sections["STRINGS"];
    c.strings = (0..c.u64_at(at) as usize).map(|k| u32::from_le_bytes(data[at + 8 + 4 * k..at + 12 + 4 * k].try_into().unwrap())).collect();
    let at = sections["FIELDS"];
    let n = c.u64_at(at) as usize;
    let (names, at) = compressed(at + 8, n);
    let size = c.u64_at(at) as usize;
    let reps: Vec<u64> = fast_decode(&data[at + 8..at + 8 + size]).chunks(8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).collect();
    assert_eq!(reps.len(), n);
    let at = sections["FIELDSETS"];
    let (sets, _) = compressed(at + 8, c.u64_at(at) as usize);
    let at = sections["PATHS"];
    let n = c.u64_at(at) as usize;
    let m = c.u64_at(at + 8) as usize;
    let (idx, at) = compressed(at + 16, m);
    let (elem, at) = compressed(at, m);
    let (jumps, _) = compressed(at, m);
    c.paths = vec![String::new(); n];
    fn build(i: usize, parent: Option<&str>, c: &mut Crate, idx: &[u32], elem: &[u32], jumps: &[u32]) -> usize {
        let mut i = i;
        loop {
            let me = i;
            i += 1;
            let path = match parent {
                None => "/".to_string(),
                Some(p) => {
                    let e = elem[me] as i32;
                    let sep = if e < 0 { "." } else if p == "/" { "" } else { "/" };
                    format!("{p}{sep}{}", c.tokens[e.unsigned_abs() as usize])
                }
            };
            c.paths[idx[me] as usize] = path.clone();
            let j = jumps[me] as i32;
            let (child, sibling) = (j > 0 || j == -1, j >= 0);
            if child {
                if sibling {
                    build(me + j as usize, parent, c, idx, elem, jumps);
                }
                i = build(i, Some(&path), c, idx, elem, jumps);
            }
            if !sibling || child {
                return i;
            }
        }
    }
    build(0, None, &mut c, &idx, &elem, &jumps);
    let at = sections["SPECS"];
    let n = c.u64_at(at) as usize;
    let (spec_paths, at) = compressed(at + 8, n);
    let (spec_sets, at) = compressed(at, n);
    let (spec_types, _) = compressed(at, n);
    for k in 0..n {
        let mut fields = Vec::new();
        let mut i = spec_sets[k] as usize;
        while sets[i] != u32::MAX {
            let f = sets[i] as usize;
            fields.push((c.tokens[names[f] as usize].clone(), reps[f]));
            i += 1;
        }
        c.specs.push((c.paths[spec_paths[k] as usize].clone(), spec_types[k], fields));
    }
    c
}

/// A layer with a bit of everything the export writes, its numbers exact
/// in 5 decimals so text and binary say the same.
fn sample_layer_meshes() -> (Vec<Mesh>, Vec<SceneLight>) {
    let mat = Rc::new(crate::daz_material::Material::default());
    let mut quad = Mesh { name: "Quad".into(), label: "A \"quad\"".into(), ..Default::default() };
    // A strip of 40 quads: index arrays long enough to be compressed, with
    // small, 16-bit and 32-bit differences.
    for k in 0..41u32 {
        quad.points.push([k as f32 * 0.125, 0.0, 0.0]);
        quad.points.push([k as f32 * 0.125, 0.5, 0.0]);
    }
    for k in 0..40u32 {
        quad.face_counts.push(4);
        quad.face_indices.extend([2 * k, 2 * k + 2, 2 * k + 3, 2 * k + 1]);
    }
    let mut uv_idx: Vec<u32> = (0..160).map(|k| k % 3).collect();
    (uv_idx[7], uv_idx[8], uv_idx[9]) = (70_000, 2_000_000_000, 1);
    quad.uvs = Some((vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.75]], uv_idx));
    quad.normals = Some(vec![[0.0, 0.0, 1.0]; 82]);
    quad.subsets = vec![("Front".into(), (0..20).collect()), ("Back".into(), (20..40).collect())];
    quad.materials = vec![Some(mat.clone()), None];
    let mut hair = Mesh { name: "Hair".into(), label: "Hair".into(), ..Default::default() };
    hair.points = vec![[0.0, 1.0, 0.0], [0.0, 0.875, 0.0], [0.25, 1.0, 0.0], [0.25, 0.75, 0.0]];
    hair.face_counts = vec![2, 2];
    hair.subsets = vec![("Hair".into(), vec![0, 1])];
    hair.materials = vec![Some(mat)];
    hair.curve_widths = Some(vec![0.0005, 0.00025, 0.0005, 0.00025]);
    hair.instances = Some(vec![IDENTITY, [-1.0, 0.0, 0.0, 4.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0]]);
    let light = SceneLight {
        label: "Panel".into(),
        shape: LightShape::Rect { width: 0.5, height: 0.25 },
        transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [1.0, 2.0, 3.0, 1.0]],
        color: [1.0, 0.5, 0.25],
        kelvin: Some(5000.0),
        lumens: 0.0, // watts in 6 decimals in text
        cone_deg: None,
        beam_exponent: 1.0,
        euler_deg: [0.0; 3],
        rotation_order: "XYZ".into(),
        translation: [1.0, 2.0, 3.0],
        two_sided: false,
        visible: false,
        profile: None,
    };
    (vec![quad, hair], vec![light])
}

#[test]
fn binary_layer_holds_what_the_text_one_says() {
    let (meshes, lights) = sample_layer_meshes();
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut w = UsdcWriter::new(&mut buf).unwrap();
    let mats = write_layer(&mut w, &meshes, &lights).unwrap();
    let size = w.finish().unwrap();
    assert_eq!(size as usize, buf.get_ref().len());
    assert_eq!(mats.len(), 2);
    let c = read_usdc(buf.get_ref());
    let mut prims = c.prims();
    prims.sort();
    let want = [
        ("/Scene", "Xform"),
        ("/Scene/Hair_copies", "PointInstancer"),
        ("/Scene/Hair_copies/Prototypes", "Scope"),
        ("/Scene/Hair_copies/Prototypes/Hair", "BasisCurves"),
        ("/Scene/Lights", "Scope"),
        ("/Scene/Lights/Panel", "RectLight"),
        ("/Scene/Materials", "Scope"),
        ("/Scene/Materials/Hair", "Material"),
        ("/Scene/Materials/Hair/Surface", "Shader"),
        ("/Scene/Materials/Quad_Front", "Material"),
        ("/Scene/Materials/Quad_Front/Surface", "Shader"),
        ("/Scene/Quad", "Mesh"),
        ("/Scene/Quad/Back", "GeomSubset"),
        ("/Scene/Quad/Front", "GeomSubset"),
    ];
    assert_eq!(prims, want.map(|(p, t)| (p.to_string(), t.to_string())));
    assert_eq!(c.token_field("/", "defaultPrim").as_deref(), Some("Scene"));
    let ints = |path: &str| c.ints(c.field(path, "default").unwrap());
    assert_eq!(ints("/Scene/Quad.faceVertexIndices"), points_in_use_order(&meshes[0]).unwrap().1, "points in use order");
    assert_eq!(ints("/Scene/Quad.primvars:st:indices"), meshes[0].uvs.as_ref().unwrap().1, "compressed ints round-trip");
    assert_eq!(ints("/Scene/Quad/Back.indices"), (20..40).collect::<Vec<u32>>());
    assert_eq!(ints("/Scene/Hair_copies/Prototypes/Hair.curveVertexCounts"), [2, 2], "short: plain");
    assert_eq!(c.floats(c.field("/Scene/Hair_copies/Prototypes/Hair.widths", "default").unwrap()), [0.0005, 0.00025, 0.0005, 0.00025]);
    assert_eq!(c.token_field("/Scene/Hair_copies/Prototypes/Hair.widths", "interpolation").as_deref(), Some("vertex"));
    let display = c.field("/Scene/Quad", "displayName").unwrap();
    assert_eq!(c.tokens[c.strings[(display & 0xffff_ffff) as usize] as usize], "A \"quad\"");
    assert!(c.field("/Scene/Quad/Front.material:binding", "targetPaths").is_some());
    assert!(c.field("/Scene/Materials/Hair.outputs:surface", "connectionPaths").is_some());
    assert_eq!(crate::usd::usdc::f16_bits_for_test(1.0), 0x3c00);
    assert_eq!(crate::usd::usdc::f16_bits_for_test(-2.0), 0xc000);
    assert_eq!(crate::usd::usdc::f16_bits_for_test(std::f32::consts::FRAC_1_SQRT_2), 0x39a8);
    assert_eq!(crate::usd::usdc::f16_bits_for_test(1e-6), 0x0011, "subnormal");

    // Where Pixar's own tools are installed (macOS ships them), both forms
    // print the same.
    let usdcat = Path::new("/usr/bin/usdcat");
    if !usdcat.exists() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (text, _) = write_usda(&meshes, &lights);
    std::fs::write(dir.path().join("a.usda"), text).unwrap();
    std::fs::write(dir.path().join("b.usdc"), buf.get_ref()).unwrap();
    let cat = |f: &str| {
        let o = std::process::Command::new(usdcat).arg(dir.path().join(f)).output().unwrap();
        assert!(o.status.success() && o.stderr.is_empty(), "{f}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8(o.stdout).unwrap()
    };
    let (a, b) = (cat("a.usda"), cat("b.usdc"));
    assert!(b.contains("def PointInstancer \"Hair_copies\""), "{b}");
    assert_eq!(a, b);
}

#[test]
fn line_tessellation_sides_come_from_the_scene_modifier() {
    let scene = |sides: Option<i64>| {
        let modifiers = match sides {
            Some(n) => json!([{"id": "Line Tessellation-1", "url": "#Line%20Tessellation", "parent": "#Hair%20A",
                "extra": [{"type": "studio/modifier/line_tessellation"}, {"type": "studio_modifier_channels", "channels": [
                    {"channel": {"id": "Viewport Line Tessellation Sides", "type": "int", "current_value": 0}},
                    {"channel": {"id": "Render Line Tessellation Sides", "type": "int", "current_value": n}}]}]}]),
            None => json!([{"id": "Smooth", "parent": "#Hair%20A", "extra": [{"type": "studio/modifier/smoothing"}]}]),
        };
        Library::new(vec![], json!({"scene": {"modifiers": modifiers}}))
    };
    // Prisms and ribbons are surfaces; one side or no modifier: Iray's fibres.
    assert_eq!(mesh::line_tessellation_sides(&mut scene(Some(3)), "Hair A"), Some(3));
    assert_eq!(mesh::line_tessellation_sides(&mut scene(Some(1)), "Hair A"), Some(1));
    assert_eq!(mesh::line_tessellation_sides(&mut scene(None), "Hair A"), None);
    assert_eq!(mesh::line_tessellation_sides(&mut scene(Some(3)), "Other"), None);
}
