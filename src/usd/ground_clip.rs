//! Iray's ground hides what lies below it (#69): the implicit ground plane
//! is a matte object, opaque to camera and secondary rays alike (Iray
//! manual 12.3, 4.4). Octane's shadow catcher is see-through, so where the
//! export follows Iray (`Tuning::clip_below_ground`) the geometry is cut at
//! the ground's height instead: through the catcher the camera then sees
//! the environment in the ray's direction, Iray's matte look.

use super::*;

/// `meshes` with everything below `ground` (world y, meters) cut away:
/// faces crossing it clipped (points, UVs and normals interpolated along
/// the cut edges, shared between the faces on either side), strands split
/// where they cross it, meshes wholly below dropped. An instanced mesh is
/// judged per copy: copies above stay copies, copies below go, a crossing
/// copy becomes a mesh of its own, baked in place and clipped. Points at
/// the ground's height count as above.
pub(super) fn clip_below_ground(meshes: Vec<Mesh>, ground: f32) -> Vec<Mesh> {
    let mut names: HashSet<String> = meshes.iter().map(|m| m.name.clone()).collect();
    let mut out = Vec::with_capacity(meshes.len());
    let (mut cut, mut dropped) = (0, 0);
    for mut m in meshes {
        let Some(copies) = m.instances.take() else {
            match side(m.points.iter().map(|p| p[1]), ground) {
                Side::Above => out.push(m),
                Side::Below => dropped += 1,
                Side::Across => {
                    cut += 1;
                    out.extend(clip(&m, ground));
                }
            }
            continue;
        };
        let mut kept = Vec::with_capacity(copies.len());
        for (k, p) in copies.iter().enumerate() {
            match side(m.points.iter().map(|q| p[4] * q[0] + p[5] * q[1] + p[6] * q[2] + p[7]), ground) {
                Side::Above => kept.push(*p),
                Side::Below => dropped += 1,
                Side::Across => {
                    cut += 1;
                    let name = unique_name(&mut names, &format!("{}_copy{k}", m.name));
                    out.extend(clip(&Mesh { name, ..placed(&m, p) }, ground));
                }
            }
        }
        if !kept.is_empty() {
            m.instances = Some(kept);
            out.push(m);
        }
    }
    if cut + dropped > 0 {
        tracing::info!(cut, dropped, ground, "USD export: geometry below Iray's ground cut away");
    }
    out
}

#[derive(Debug, PartialEq)]
enum Side {
    Above,
    Below,
    Across,
}

fn side(ys: impl Iterator<Item = f32>, ground: f32) -> Side {
    let (lo, hi) = ys.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), y| (lo.min(y), hi.max(y)));
    if lo >= ground || lo > hi {
        Side::Above
    } else if hi < ground {
        Side::Below
    } else {
        Side::Across
    }
}

/// The mesh as copy `p` draws it, in world space: points moved, normals by
/// the inverse transpose, faces turned round under a mirroring placement
/// (so the winding still agrees with the normals), strand widths scaled.
fn placed(m: &Mesh, p: &Placement) -> Mesh {
    let at = |q: &[f32; 3]| std::array::from_fn(|r| p[r * 4] * q[0] + p[r * 4 + 1] * q[1] + p[r * 4 + 2] * q[2] + p[r * 4 + 3]);
    let affine: Affine = std::array::from_fn(|k| f64::from(p[k]));
    let det = {
        let a = |r: usize, c: usize| affine[r * 4 + c];
        a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1)) - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0)) + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0))
    };
    let inv = invert(&affine).unwrap_or(affine);
    let normal = |n: &[f32; 3]| {
        // (L⁻¹)ᵀ n: column r of L⁻¹ dotted with n.
        let v: [f64; 3] = std::array::from_fn(|r| (0..3).map(|k| inv[k * 4 + r] * f64::from(n[k])).sum());
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l > 0.0 { v.map(|x| (x / l) as f32) } else { *n }
    };
    let mut out = Mesh {
        name: m.name.clone(),
        label: m.label.clone(),
        points: m.points.iter().map(at).collect(),
        face_counts: m.face_counts.clone(),
        face_indices: m.face_indices.clone(),
        subsets: m.subsets.clone(),
        materials: m.materials.clone(),
        uvs: m.uvs.clone(),
        normals: m.normals.as_ref().map(|ns| ns.iter().map(normal).collect()),
        normal_indices: m.normal_indices.clone(),
        curve_widths: m.curve_widths.as_ref().map(|w| {
            let s = det.abs().cbrt() as f32;
            w.iter().map(|x| x * s).collect()
        }),
        instances: None,
        // After the ground is placed: no longer read.
        base_low: None,
    };
    if det < 0.0 && out.curve_widths.is_none() {
        let mut at = 0;
        for &c in &m.face_counts {
            let r = at..at + c as usize;
            out.face_indices[r.clone()].reverse();
            if let Some(idx) = &mut out.normal_indices {
                idx[r.clone()].reverse();
            }
            if let Some((_, idx)) = &mut out.uvs {
                idx[r].reverse();
            }
            at += c as usize;
        }
    }
    out
}

/// One mesh cut at `ground`; None when nothing is left above it.
fn clip(m: &Mesh, ground: f32) -> Option<Mesh> {
    if m.curve_widths.is_some() { clip_strands(m, ground) } else { clip_faces(m, ground) }
}

/// A face corner: its point and, when the mesh has them, its faceVarying
/// UV and normal indices.
#[derive(Clone, Copy)]
struct Corner {
    point: u32,
    uv: Option<u32>,
    normal: Option<u32>,
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 { v.map(|x| x / l) } else { v }
}

fn clip_faces(m: &Mesh, ground: f32) -> Option<Mesh> {
    let mut points = m.points.clone();
    let per_vertex_normals = m.normal_indices.is_none();
    let mut normals = m.normals.clone();
    let mut uvs = m.uvs.as_ref().map(|(v, _)| v.clone());
    // Cut points by edge (lower point index first), cut UVs / faceVarying
    // normals by edge and the two corners' indices: faces sharing an edge
    // share its cut, a UV seam keeps its two sides.
    let mut cut_point: HashMap<(u32, u32), u32> = HashMap::new();
    let mut cut_uv: HashMap<(u32, u32, u32, u32), u32> = HashMap::new();
    let mut cut_normal: HashMap<(u32, u32, u32, u32), u32> = HashMap::new();
    let (mut face_counts, mut face_indices) = (Vec::new(), Vec::new());
    let (mut uv_idx, mut normal_idx) = (Vec::new(), Vec::new());
    let mut new_face = vec![None; m.face_counts.len()];
    let mut start = 0usize;
    for (f, &c) in m.face_counts.iter().enumerate() {
        let corners: Vec<Corner> = (start..start + c as usize)
            .map(|k| Corner { point: m.face_indices[k], uv: m.uvs.as_ref().map(|(_, i)| i[k]), normal: m.normal_indices.as_ref().map(|i| i[k]) })
            .collect();
        start += c as usize;
        let above = |q: &Corner| m.points[q.point as usize][1] >= ground;
        let mut poly: Vec<Corner> = Vec::with_capacity(corners.len() + 2);
        for (k, a) in corners.iter().enumerate() {
            let b = &corners[(k + 1) % corners.len()];
            if above(a) {
                poly.push(*a);
            }
            if above(a) != above(b) {
                // The cut on edge a–b, always from its lower point index.
                let (lo, hi) = if a.point < b.point { (a, b) } else { (b, a) };
                let (pl, ph) = (m.points[lo.point as usize], m.points[hi.point as usize]);
                let t = (ground - pl[1]) / (ph[1] - pl[1]);
                let key = (lo.point, hi.point);
                let point = *cut_point.entry(key).or_insert_with(|| {
                    let mut p = lerp3(pl, ph, t);
                    p[1] = ground;
                    points.push(p);
                    if per_vertex_normals {
                        if let Some(ns) = &mut normals {
                            let n = unit(lerp3(ns[lo.point as usize], ns[hi.point as usize], t));
                            ns.push(n);
                        }
                    }
                    (points.len() - 1) as u32
                });
                let uv = match (lo.uv, hi.uv, &mut uvs) {
                    (Some(ul), Some(uh), Some(vals)) => Some(*cut_uv.entry((key.0, key.1, ul, uh)).or_insert_with(|| {
                        let (a, b) = (vals[ul as usize], vals[uh as usize]);
                        vals.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
                        (vals.len() - 1) as u32
                    })),
                    _ => None,
                };
                let normal = match (lo.normal, hi.normal, &mut normals) {
                    (Some(nl), Some(nh), Some(ns)) if !per_vertex_normals => Some(*cut_normal.entry((key.0, key.1, nl, nh)).or_insert_with(|| {
                        let n = unit(lerp3(ns[nl as usize], ns[nh as usize], t));
                        ns.push(n);
                        (ns.len() - 1) as u32
                    })),
                    _ => None,
                };
                poly.push(Corner { point, uv, normal });
            }
        }
        if poly.len() < 3 {
            continue;
        }
        new_face[f] = Some(face_counts.len() as u32);
        face_counts.push(poly.len() as u32);
        for q in &poly {
            face_indices.push(q.point);
            if let Some(u) = q.uv {
                uv_idx.push(u);
            }
            if let Some(n) = q.normal {
                normal_idx.push(n);
            }
        }
    }
    if face_counts.is_empty() {
        return None;
    }
    // Only the points (and per-vertex normals) a face still uses.
    let mut used = vec![u32::MAX; points.len()];
    let (mut kept_points, mut kept_normals) = (Vec::new(), Vec::new());
    for i in &mut face_indices {
        let slot = &mut used[*i as usize];
        if *slot == u32::MAX {
            *slot = kept_points.len() as u32;
            kept_points.push(points[*i as usize]);
            if per_vertex_normals {
                if let Some(ns) = &normals {
                    kept_normals.push(ns[*i as usize]);
                }
            }
        }
        *i = *slot;
    }
    let subsets = m.subsets.iter().map(|(g, faces)| (g.clone(), faces.iter().filter_map(|&f| new_face.get(f as usize).copied().flatten()).collect())).collect();
    Some(Mesh {
        name: m.name.clone(),
        label: m.label.clone(),
        points: kept_points,
        face_counts,
        face_indices,
        subsets,
        materials: m.materials.clone(),
        uvs: uvs.map(|v| (v, uv_idx)),
        normals: if per_vertex_normals { normals.map(|_| kept_normals) } else { normals },
        normal_indices: m.normal_indices.as_ref().map(|_| normal_idx),
        curve_widths: None,
        instances: None,
        base_low: None,
    })
}

/// Strands cut where they cross the ground: each run above it a strand of
/// its own, ending at the crossing (point and width interpolated).
fn clip_strands(m: &Mesh, ground: f32) -> Option<Mesh> {
    let widths = m.curve_widths.as_ref()?;
    let (mut points, mut out_widths, mut counts) = (Vec::new(), Vec::new(), Vec::new());
    let mut start = 0usize;
    for &c in &m.face_counts {
        let range = start..start + c as usize;
        start += c as usize;
        let mut run = 0u32;
        let mut close = |run: &mut u32, points: &mut Vec<[f32; 3]>, widths: &mut Vec<f32>| {
            match *run {
                0 => {}
                1 => {
                    points.pop();
                    widths.pop();
                }
                n => counts.push(n),
            }
            *run = 0;
        };
        for k in range.clone() {
            let (p, w) = (m.points[k], widths[k]);
            if k > range.start {
                let (q, v) = (m.points[k - 1], widths[k - 1]);
                if (q[1] >= ground) != (p[1] >= ground) {
                    let t = (ground - q[1]) / (p[1] - q[1]);
                    let mut x = lerp3(q, p, t);
                    x[1] = ground;
                    points.push(x);
                    out_widths.push(v + (w - v) * t);
                    run += 1;
                    if p[1] < ground {
                        close(&mut run, &mut points, &mut out_widths);
                    }
                }
            }
            if p[1] >= ground {
                points.push(p);
                out_widths.push(w);
                run += 1;
            }
        }
        close(&mut run, &mut points, &mut out_widths);
    }
    if counts.is_empty() {
        return None;
    }
    let group = m.subsets.first().map_or_else(|| "Hair".to_string(), |(g, _)| g.clone());
    Some(Mesh {
        name: m.name.clone(),
        label: m.label.clone(),
        subsets: vec![(group, (0..counts.len() as u32).collect())],
        face_counts: counts,
        points,
        materials: m.materials.clone(),
        curve_widths: Some(out_widths),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles of an upright square, y from -1 to 1, UVs = (x, y), one
    /// faceVarying normal (+z) per corner.
    fn square() -> Mesh {
        Mesh {
            name: "Square".into(),
            label: "Square".into(),
            points: vec![[0.0, -1.0, 0.0], [1.0, -1.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            face_counts: vec![3, 3],
            face_indices: vec![0, 1, 2, 0, 2, 3],
            subsets: vec![("A".into(), vec![0]), ("B".into(), vec![1])],
            materials: vec![None, None],
            uvs: Some((vec![[0.0, -1.0], [1.0, -1.0], [1.0, 1.0], [0.0, 1.0]], vec![0, 1, 2, 0, 2, 3])),
            normals: Some(vec![[0.0, 0.0, 1.0]]),
            normal_indices: Some(vec![0; 6]),
            ..Default::default()
        }
    }

    #[test]
    fn faces_across_the_ground_are_cut_with_their_uvs() {
        let out = clip_below_ground(vec![square()], 0.0);
        assert_eq!(out.len(), 1);
        let m = &out[0];
        assert!(m.points.iter().all(|p| p[1] >= 0.0), "{:?}", m.points);
        // The diagonal 0-2 is cut once and shared: (0.5, 0); the sides at (0, 0) / (1, 0).
        assert_eq!(m.points.len(), 5, "{:?}", m.points);
        assert_eq!(m.points.iter().filter(|p| p[1] == 0.0).count(), 3);
        // Every corner's UV is its point's (x, y), the cut ones included.
        let (vals, idx) = m.uvs.as_ref().unwrap();
        for (k, &i) in m.face_indices.iter().enumerate() {
            let (p, uv) = (m.points[i as usize], vals[idx[k] as usize]);
            assert!((p[0] - uv[0]).abs() < 1e-6 && (p[1] - uv[1]).abs() < 1e-6, "{p:?} {uv:?}");
        }
        let normals = m.normals.as_ref().unwrap();
        assert!(m.normal_indices.as_ref().unwrap().iter().all(|&n| normals[n as usize] == [0.0, 0.0, 1.0]));
        assert_eq!(m.normal_indices.as_ref().unwrap().len(), m.face_indices.len());
        // Both faces keep their subsets: a triangle and a quad above y 0.
        assert_eq!(m.face_counts, vec![3, 4]);
        assert_eq!(m.subsets, vec![("A".to_string(), vec![0]), ("B".to_string(), vec![1])]);
    }

    #[test]
    fn meshes_above_stay_and_below_go() {
        let out = clip_below_ground(vec![square()], -1.0);
        assert_eq!(out[0].points, square().points);
        assert!(clip_below_ground(vec![square()], 1.5).is_empty());
    }

    #[test]
    fn per_vertex_normals_follow_the_cut() {
        let mut m = square();
        m.normal_indices = None;
        m.normals = Some(vec![[0.0, 0.0, 1.0]; 4]);
        let out = clip_below_ground(vec![m], 0.5);
        let m = &out[0];
        assert_eq!(m.normals.as_ref().unwrap().len(), m.points.len());
        assert!(m.points.iter().all(|p| p[1] >= 0.5));
    }

    #[test]
    fn strands_are_split_where_they_cross() {
        // A strand down through the ground and up again, and one wholly below.
        let m = Mesh {
            name: "Hair".into(),
            label: "Hair".into(),
            points: vec![[0.0, 1.0, 0.0], [1.0, -1.0, 0.0], [2.0, 1.0, 0.0], [0.0, -2.0, 0.0], [0.0, -3.0, 0.0]],
            face_counts: vec![3, 2],
            subsets: vec![("Hair".into(), vec![0, 1])],
            materials: vec![None],
            curve_widths: Some(vec![1.0, 2.0, 3.0, 1.0, 1.0]),
            ..Default::default()
        };
        let out = clip_below_ground(vec![m], 0.0);
        let h = &out[0];
        assert_eq!(h.face_counts, vec![2, 2]);
        assert_eq!(h.points, vec![[0.0, 1.0, 0.0], [0.5, 0.0, 0.0], [1.5, 0.0, 0.0], [2.0, 1.0, 0.0]]);
        assert_eq!(h.curve_widths, Some(vec![1.0, 1.5, 2.5, 3.0]));
        assert_eq!(h.subsets, vec![("Hair".to_string(), vec![0, 1])]);
    }

    #[test]
    fn instance_copies_are_judged_one_by_one() {
        // The square lifted 2 m (above), at its place (across) and 5 m down
        // (below): one copy stays a copy, one is baked and cut, one goes.
        let lift = |y: f32| [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, 0.0];
        let mut m = square();
        m.instances = Some(vec![IDENTITY, lift(2.0), lift(-5.0)]);
        let out = clip_below_ground(vec![m], 0.0);
        assert_eq!(out.len(), 2);
        let baked = out.iter().find(|m| m.instances.is_none()).unwrap();
        assert_eq!(baked.name, "Square_copy0");
        assert!(baked.points.iter().all(|p| p[1] >= 0.0));
        let kept = out.iter().find(|m| m.instances.is_some()).unwrap();
        assert_eq!(kept.instances, Some(vec![lift(2.0)]));
        assert_eq!(kept.points, square().points);
    }

    #[test]
    fn a_mirroring_copy_turns_its_faces_round() {
        let mirror = [-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut m = square();
        m.normals = Some(vec![[1.0, 0.0, 0.0]]);
        let p = placed(&m, &mirror);
        assert_eq!(&p.face_indices[..3], &[2, 1, 0]);
        assert_eq!(p.normals, Some(vec![[-1.0, 0.0, 0.0]]));
        assert_eq!(p.points[1], [-1.0, -1.0, 0.0]);
    }
}
