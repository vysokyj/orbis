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
    assert_eq!(shell.face_indices, vec![1, 7, 8, 9]);
    assert_eq!(shell.subsets, vec![("Graft_Patch".to_string(), vec![0])]);
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
    assert!(usda.contains("color3f inputs:diffuseColor = (1, 0.5, 0.25)"));
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
    assert!((c.aperture_cm - 0.2708).abs() < 0.001);
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
    let mut o = String::new();
    write_lights(&mut o, &lights);
    assert!(!o.contains("SpotLight"), "spots are not UsdLux lights");
    let spots = octane_spots(&lights);
    assert_eq!((spots[0].spread_deg, spots[0].watts), (60.0, 10.0));
    let (nodes, mesh) = crate::material::scene_lights(&spots, None, 500, 1439);
    assert!(nodes.contains("type='402' name='SpotLight'") && nodes.contains("type='3' name='DAZ 3D lights'"));
    assert!(nodes.contains("<pin name='Input 1' dynamicType='12' connect='1439'/>"));
    // DAZ "XYZ" (X applied first) is Octane "ZYX" (enum 5).
    assert!(nodes.contains("<attr name='rotationOrder' type='2'>5</attr>"));
    assert!(nodes.contains("<attr name='scale' type='8'>1 1 -1</attr>"), "disk faces DAZ's -Z");
    // 10 W × (1 − cos 30°) / 2 × 0.75 (trim against Blender) ≈ 0.5 W
    // into the 60° cone.
    assert!(nodes.contains("<attr name='value' type='9'>0.502405 0 0 0</attr>"), "{nodes}");
    assert!(nodes.contains("<attr name='rotation' type='8'>176.186 -45.924 179.984</attr>"), "{nodes}");
    assert_eq!(nodes.matches("<node ").count(), nodes.matches("</node>").count());
    // Iray's ground: a shadow-catching infinite plane in the group, placed
    // just under y = 0 so a room's floor hides it.
    // Auto: the scene's lowest point; Manual: Ground Origin Y (cm).
    let low = Mesh { points: vec![[0.0, 0.25, 0.0], [1.0, -0.03, 0.0]], ..Default::default() };
    let mut e = SceneEnvironment { map: None, intensity: 1.0, rotation_deg: 0.0, orientation_deg: [0.0; 3], scene_only: false, ground_shadow: Some(1.0), ground_origin_y: None };
    assert!((ground_height(&e, &[low]) + 0.03).abs() < 1e-6);
    assert_eq!(ground_height(&e, &[]), 0.0);
    e.ground_origin_y = Some(12.0);
    assert!((ground_height(&e, &[]) - 0.12).abs() < 1e-6);
    let (lifted, _) = crate::material::scene_lights(&[], Some((0.8, 0.12)), 500, 1439);
    assert!(lifted.contains("<attr name='translation' type='8'>0 0.119 0</attr>"), "{lifted}");
    let (g, gmesh) = crate::material::scene_lights(&[], Some((0.8, 0.0)), 500, 1439);
    assert!(g.contains("type='4' name='DAZ ground'") && g.contains("<attr name='translation' type='8'>0 -0.001 0</attr>"), "{g}");
    assert!(g.contains("type='110' name='DAZ ground plane'") && g.contains("type='145'") && g.contains("<attr name='value' type='6'>0.712</attr>"), "{g}");
    assert!(g.contains("<pin name='Input 2' dynamicType='12' connect='501'/>") && gmesh != 1439, "{g}");
    assert_eq!(g.matches("<node ").count(), g.matches("</node>").count());
    assert_eq!(crate::material::scene_lights(&[], None, 500, 1439), (String::new(), 1439));
    assert_ne!(mesh, 1439);
    assert!(light_emitters(&lights).is_empty(), "no UsdLux emitter for a spot");
    // A point light stays UsdLux: 1500 lm ×3 / 15 000 = 0.3 W, glowing.
    let mut point = lights[0].clone();
    point.cone_deg = None;
    point.lumens = 1500.0;
    point.label = "Bulb".into();
    let mut o = String::new();
    write_lights(&mut o, &[point.clone()]);
    assert!(o.contains("def SphereLight \"Bulb\"") && o.contains("bool treatAsPoint = 1") && o.contains("float inputs:colorTemperature = 6500"));
    assert!(o.contains("float inputs:intensity = 0.300000") && o.contains("bool inputs:normalize = 1"), "{o}");
    let em = light_emitters(&[point]);
    let xml = crate::material::light_inputs(&em, 100);
    assert!(xml.contains("name='/Scene/Lights/Bulb_light'") && xml.contains("type='53'") && xml.contains("6500 0 0"));
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
    assert_eq!(tone.white_point, [0.8, 0.9, 1.0]);
    assert_eq!(tone.octane_exposure(1.4), 2.8, "EV 12 is one stop brighter than DAZ's 13");
    let cams = vec![("Camera".to_string(), framing_camera(&[]))];
    let ocs = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), "", OctaneMode::Final, &tone);
    assert!(ocs.contains("2.800000 0 0 0") && ocs.contains("0.750000 0 0 0") && ocs.contains("1.200000 0 0 0"));
    assert!(ocs.contains("0.800000 0.900000 1.000000"));
    assert_eq!(scene_tone_mapping(&mut Library::new(vec![], json!({"scene": {"nodes": []}}))), ToneMapping::default());
}

#[test]
fn dome_rotation_is_a_u_shift() {
    let env = |rot: f64, oy: f64| SceneEnvironment { map: None, intensity: 1.0, rotation_deg: rot, orientation_deg: [0.0, oy, 0.0], scene_only: false, ground_shadow: None, ground_origin_y: None };
    assert_eq!(env(0.0, 0.0).dome_u_shift(), 0.25);
    assert!((env(110.0, 0.0).dome_u_shift() - (0.25 + 110.0 / 360.0)).abs() < 1e-12);
    assert_eq!(env(270.0, 0.0).dome_u_shift(), 0.0);
    assert_eq!(env(180.0, 90.0).dome_u_shift(), 0.5);
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
    assert_eq!(env, SceneEnvironment { map: Some("/Environments/HDRI/sky 1.hdr".into()), intensity: 4.0, rotation_deg: 90.0, orientation_deg: [0.0; 3], scene_only: false, ground_shadow: Some(1.0), ground_origin_y: None });

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
    let ocs = project_ocs("Bob's <scene>", "scene.usdz", &cam, (1280, 720), "", OctaneMode::Preview, &ToneMapping::default())
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
    let ocs = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), env, OctaneMode::Final, &ToneMapping::default());
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
    let fin = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), "", OctaneMode::Final, &ToneMapping::default());
    assert!(fin.contains("<node id='100060' type='25'>") && fin.contains("<node id='1100060' type='25'>"), "path tracing");
    assert_eq!(fin.matches("<pin name='maxScatterDepth'>").count(), 2);
    assert!(fin.contains("<attr name='value' type='9'>1.400000 0 0 0</attr>"), "Final exposure at EV 13");
    let pre = project_ocs("Shot", "scene.usdz", &cams, (1280, 720), "", OctaneMode::Preview, &ToneMapping::default());
    assert!(pre.contains("<node id='100060' type='24'>") && pre.contains("<node id='1100060' type='24'>"), "direct lighting");
    for ocs in [&fin, &pre] {
        for ph in ["@KERNEL@", "@EXPOSURE@", "@HIGHLIGHT_COMPRESSION@", "@WHITE_BALANCE@", "@SATURATION@", "@DENOISER@"] {
            assert!(!ocs.contains(ph), "{ph} left");
        }
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
    std::fs::write(&dump_path, json!({"format": "orbis-scene-dump", "format_version": 1, "nodes": nodes}).to_string()).unwrap();

    let out = dir.path().join("My Scene.orbx");
    let report = export_scene_orbx(&scene_path, &dump_path, &out, &ExportOptions::default()).unwrap();
    assert_eq!(report.meshes, 2, "host with its merged graft, and the shell");
    let (version, files) = crate::orbx::read_orbx(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(version, OCTANE_VERSION);
    let names: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(names, ["assets/scene.usdz", "project.ocs"]);
    let ocs = String::from_utf8(files[1].1.clone()).unwrap();
    assert!(ocs.contains("type='56' name='My Scene'") && ocs.contains("-3.391028 0"));
    let mut usdz = zip::ZipArchive::new(std::io::Cursor::new(files[0].1.clone())).unwrap();
    assert_eq!(usdz.by_index(0).unwrap().name().unwrap(), "scene.usda");
}

#[test]
fn dump_matches_by_label_then_unique_name() {
    let d = SceneDump::parse(
        r#"{"format": "orbis-scene-dump", "format_version": 1,
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
    assert!(SceneDump::parse(r#"{"format": "orbis-scene-dump", "format_version": 1, "nodes": [{"vertices": [1, 2]}]}"#).is_err());
}

#[test]
fn dump_cache_path_is_per_scene() {
    let a = dump_cache_path(Path::new("/a/My Scene.duf"));
    let b = dump_cache_path(Path::new("/b/My Scene.duf"));
    assert_ne!(a, b);
    assert!(a.file_name().unwrap().to_str().unwrap().starts_with("My Scene-"));
    assert_eq!(a, dump_cache_path(Path::new("/a/My Scene.duf")));
}
