//! Mesh refinement — Catmull-Clark `subdivide_once`, `lift_off`, vertex normals.

use super::*;

/// How far (meters) a guest's vertex stays above its host figure's refined
/// surface; and the reach within which it counts as lying on it.
pub(super) const LIFT_CLEARANCE: f32 = 0.0005;
pub(super) const LIFT_REACH: f32 = 0.005;

/// Lift `guest`'s vertices lying on or under `host` (within `LIFT_REACH` of
/// a host vertex) to `LIFT_CLEARANCE` above it, along the nearest host
/// vertex's normal. DAZ fits a cap or brows onto the figure's subdivided
/// surface with its unrefined faces; as straight chords over the curved
/// head they dip under it and z-fight (dots along Octane2's hairline, up to
/// 0.8 mm under). Returns the number of vertices moved.
pub(super) fn lift_off(guest: &mut Mesh, host: &Mesh) -> usize {
    let normals = vertex_normals(&host.points, &host.face_counts, &host.face_indices);
    let cell = |p: [f32; 3]| p.map(|c| (c / LIFT_REACH).floor() as i32);
    let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
    for (i, &p) in host.points.iter().enumerate() {
        grid.entry(cell(p)).or_default().push(i as u32);
    }
    let mut lifted = 0;
    for p in &mut guest.points {
        let c = cell(*p);
        let mut best: Option<(f32, u32)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for &h in grid.get(&[c[0] + dx, c[1] + dy, c[2] + dz]).into_iter().flatten() {
                        let q = host.points[h as usize];
                        let d2 = (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f32>();
                        if d2 < LIFT_REACH * LIFT_REACH && best.is_none_or(|(b, _)| d2 < b) {
                            best = Some((d2, h));
                        }
                    }
                }
            }
        }
        let Some((_, h)) = best else { continue };
        let (q, n) = (host.points[h as usize], normals[h as usize]);
        let height: f32 = (0..3).map(|k| (p[k] - q[k]) * n[k]).sum();
        if height < LIFT_CLEARANCE {
            (0..3).for_each(|k| p[k] += n[k] * (LIFT_CLEARANCE - height));
            lifted += 1;
        }
    }
    lifted
}

/// One level of Catmull-Clark subdivision, as DAZ renders SubD level 1 (a
/// refined mesh, not the limit surface): every n-gon becomes n quads.
/// Boundaries stay put at corners (one face) and follow the boundary curve
/// elsewhere ("edges and corners"); vertices on non-manifold edges keep
/// their place. UVs are split linearly per face, so seams stay sharp.
/// Octane can't take the level per mesh from USD — only "subdivide all
/// meshes", which would round props DAZ keeps hard — so the export refines.
pub(super) fn subdivide_once(m: &Mesh) -> Mesh {
    let nv = m.points.len();
    let p = |i: u32| m.points[i as usize].map(f64::from);
    let mut starts = Vec::with_capacity(m.face_counts.len());
    let mut at = 0usize;
    for &c in &m.face_counts {
        starts.push(at);
        at += c as usize;
    }
    let face = |f: usize| &m.face_indices[starts[f]..starts[f] + m.face_counts[f] as usize];
    let key = |a: u32, b: u32| if a < b { (a, b) } else { (b, a) };

    // Face points and each edge's faces.
    let mut face_pts: Vec<[f64; 3]> = Vec::with_capacity(m.face_counts.len());
    let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for f in 0..m.face_counts.len() {
        let vs = face(f);
        let mut c = [0.0; 3];
        for &v in vs {
            let q = p(v);
            (0..3).for_each(|k| c[k] += q[k] / vs.len() as f64);
        }
        face_pts.push(c);
        for i in 0..vs.len() {
            edge_faces.entry(key(vs[i], vs[(i + 1) % vs.len()])).or_default().push(f);
        }
    }

    // Edge points: smooth inside, midpoints on boundaries / non-manifold edges.
    let mut edge_ids: HashMap<(u32, u32), u32> = HashMap::with_capacity(edge_faces.len());
    let mut edge_pts: Vec<[f64; 3]> = Vec::with_capacity(edge_faces.len());
    // Per vertex: face-point sum, incident faces, edge-midpoint sum, edges,
    // boundary neighbours, on a non-manifold edge.
    let mut fsum = vec![[0.0; 3]; nv];
    let mut nfaces = vec![0u32; nv];
    let mut msum = vec![[0.0; 3]; nv];
    let mut nedges = vec![0u32; nv];
    let mut boundary: Vec<Vec<u32>> = vec![Vec::new(); nv];
    let mut pinned = vec![false; nv];
    let mut edges: Vec<_> = edge_faces.iter().collect();
    edges.sort_unstable_by_key(|(k, _)| **k);
    for (&(a, b), fs) in edges {
        let (pa, pb) = (p(a), p(b));
        let mid = [0, 1, 2].map(|k| (pa[k] + pb[k]) / 2.0);
        let ep = if fs.len() == 2 {
            let (f0, f1) = (face_pts[fs[0]], face_pts[fs[1]]);
            [0, 1, 2].map(|k| (pa[k] + pb[k] + f0[k] + f1[k]) / 4.0)
        } else {
            mid
        };
        edge_ids.insert((a, b), (nv + face_pts.len() + edge_pts.len()) as u32);
        edge_pts.push(ep);
        for (v, other) in [(a, b), (b, a)] {
            let v = v as usize;
            (0..3).for_each(|k| msum[v][k] += mid[k]);
            nedges[v] += 1;
            match fs.len() {
                1 => boundary[v].push(other),
                2 => {}
                _ => pinned[v] = true,
            }
        }
    }
    for f in 0..m.face_counts.len() {
        for &v in face(f) {
            (0..3).for_each(|k| fsum[v as usize][k] += face_pts[f][k]);
            nfaces[v as usize] += 1;
        }
    }

    let mut points: Vec<[f32; 3]> = Vec::with_capacity(nv + face_pts.len() + edge_pts.len());
    for v in 0..nv {
        let pv = m.points[v].map(f64::from);
        let n = nedges[v] as f64;
        let new = if pinned[v] || nfaces[v] == 0 || (boundary[v].is_empty() && nedges[v] < 3) {
            pv
        } else if !boundary[v].is_empty() {
            if nfaces[v] == 1 || boundary[v].len() != 2 {
                pv // a corner (or a boundary pinch): sharp
            } else {
                let (a, b) = (p(boundary[v][0]), p(boundary[v][1]));
                [0, 1, 2].map(|k| (a[k] + b[k] + 6.0 * pv[k]) / 8.0)
            }
        } else {
            let nf = nfaces[v] as f64;
            [0, 1, 2].map(|k| (fsum[v][k] / nf + 2.0 * msum[v][k] / n + (n - 3.0) * pv[k]) / n)
        };
        points.push(new.map(|x| x as f32));
    }
    points.extend(face_pts.iter().map(|c| c.map(|x| x as f32)));
    points.extend(edge_pts.iter().map(|c| c.map(|x| x as f32)));

    // Quads, the UVs alongside: corner UV, then the face's edge midpoints and
    // centre, interpolated linearly (shared across the face's UV edge).
    let mut out = Mesh { name: m.name.clone(), label: m.label.clone(), materials: m.materials.clone(), points, ..Default::default() };
    let mut uv_vals: Vec<[f32; 2]> = m.uvs.as_ref().map(|(v, _)| v.clone()).unwrap_or_default();
    let mut uv_edge: HashMap<(u32, u32), u32> = HashMap::new();
    let mut uv_idx: Vec<u32> = Vec::new();
    let mut new_faces: Vec<std::ops::Range<u32>> = Vec::with_capacity(m.face_counts.len());
    for f in 0..m.face_counts.len() {
        let vs = face(f);
        let n = vs.len();
        let first = out.face_counts.len() as u32;
        let fp = (nv + f) as u32;
        let e = |i: usize| edge_ids[&key(vs[i], vs[(i + 1) % n])];
        let uvf: Option<(Vec<u32>, Vec<u32>, u32)> = m.uvs.as_ref().map(|(vals, idx)| {
            let corners = idx[starts[f]..starts[f] + n].to_vec();
            let mut c = [0.0f32; 2];
            for &i in &corners {
                (0..2).for_each(|k| c[k] += vals[i as usize][k] / n as f32);
            }
            let centre = uv_vals.len() as u32;
            uv_vals.push(c);
            let mids = (0..n)
                .map(|i| {
                    let (a, b) = (corners[i], corners[(i + 1) % n]);
                    *uv_edge.entry(key(a, b)).or_insert_with(|| {
                        let (pa, pb) = (vals[a as usize], vals[b as usize]);
                        uv_vals.push([(pa[0] + pb[0]) / 2.0, (pa[1] + pb[1]) / 2.0]);
                        uv_vals.len() as u32 - 1
                    })
                })
                .collect();
            (corners, mids, centre)
        });
        for i in 0..n {
            let prev = (i + n - 1) % n;
            out.face_counts.push(4);
            out.face_indices.extend([vs[i], e(i), fp, e(prev)]);
            if let Some((corners, mids, centre)) = &uvf {
                uv_idx.extend([corners[i], mids[i], *centre, mids[prev]]);
            }
        }
        new_faces.push(first..out.face_counts.len() as u32);
    }
    out.subsets = m.subsets.iter().map(|(g, faces)| (g.clone(), faces.iter().flat_map(|&f| new_faces[f as usize].clone()).collect())).collect();
    if m.uvs.is_some() {
        out.uvs = Some((uv_vals, uv_idx));
    }
    out
}

/// The faces `keep` marks (with their materials and UVs), the points they
/// use renumbered in order; subsets left empty go.
pub(super) fn keep_faces(m: &Mesh, keep: &[bool]) -> Mesh {
    let mut starts = Vec::with_capacity(m.face_counts.len());
    let mut at = 0usize;
    for &c in &m.face_counts {
        starts.push(at);
        at += c as usize;
    }
    let mut out = Mesh { name: m.name.clone(), label: m.label.clone(), ..Default::default() };
    let mut point_of: HashMap<u32, u32> = HashMap::new();
    let mut uv_idx: Vec<u32> = Vec::new();
    for (i, (group, faces)) in m.subsets.iter().enumerate() {
        let mut new_faces = Vec::with_capacity(faces.len());
        for &f in faces.iter().filter(|&&f| keep.get(f as usize).copied().unwrap_or(true)) {
            let (s, n) = (starts[f as usize], m.face_counts[f as usize] as usize);
            new_faces.push(out.face_counts.len() as u32);
            out.face_counts.push(n as u32);
            for &v in &m.face_indices[s..s + n] {
                let next = point_of.len() as u32;
                let p = *point_of.entry(v).or_insert_with(|| {
                    out.points.push(m.points[v as usize]);
                    next
                });
                out.face_indices.push(p);
            }
            if let Some((_, idx)) = &m.uvs {
                uv_idx.extend_from_slice(&idx[s..s + n]);
            }
        }
        if new_faces.is_empty() {
            continue;
        }
        out.subsets.push((group.clone(), new_faces));
        out.materials.push(m.materials.get(i).cloned().flatten());
    }
    if let Some((vals, _)) = &m.uvs {
        out.uvs = Some((vals.clone(), uv_idx));
    }
    out
}

/// DAZ's auto-smoothing (the material's /Geometry/Smoothing channels) as
/// the cosine a neighbouring face's normal must reach to be averaged in:
/// `Smooth On` off → none (> 1: the face alone), else cos(`Smooth Angle`),
/// default 89.9° (DAZ's) — a cube's 90° edges stay hard.
pub(super) fn smoothing_cos(mat: Option<&crate::daz_material::Material>) -> f32 {
    let on = mat.and_then(|m| m.float("Smooth On")).map_or(true, |v| v > 0.0);
    let angle = mat.and_then(|m| m.float("Smooth Angle")).unwrap_or(89.9).clamp(0.0, 180.0);
    if on { angle.to_radians().cos() } else { 2.0 }
}

/// Normals as DAZ hands them to Iray (Calib-Smooth .mi): an edge is hard
/// when its faces' normals lie further apart than the smoothing angle
/// (`face_cos`, from `smoothing_cos`; the stricter of the two faces, Smooth
/// off makes all of a face's edges hard). Around each vertex the faces
/// joined through soft edges share the area-weighted sum of their normals —
/// an icosahedron (41.8° edges) at 45° is smooth although faces across the
/// vertex lie 70° apart (S3), a cylinder at 89.9° smooth round its side and
/// split at the caps (S8). Material borders don't split (S5: one normal per
/// vertex across two materials). Returns the normals and, when some vertex
/// carries more than one, faceVarying indices into them; else they are per
/// vertex (as `vertex_normals`).
pub(super) fn smoothed_normals(points: &[[f32; 3]], counts: &[u32], indices: &[u32], face_cos: &[f32]) -> (Vec<[f32; 3]>, Option<Vec<u32>>) {
    if face_cos.iter().all(|&c| c <= -1.0) {
        return (vertex_normals(points, counts, indices), None);
    }
    let mut starts = Vec::with_capacity(counts.len());
    let mut face_of = Vec::with_capacity(indices.len());
    let mut at = 0usize;
    let mut area = Vec::with_capacity(counts.len());
    for (fi, &c) in counts.iter().enumerate() {
        starts.push(at);
        let f = &indices[at..at + c as usize];
        at += c as usize;
        face_of.extend(std::iter::repeat_n(fi as u32, f.len()));
        let mut n = [0f32; 3];
        for i in 0..f.len() {
            let a = points[f[i] as usize];
            let b = points[f[(i + 1) % f.len()] as usize];
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        area.push(n);
    }
    let unit = |n: [f32; 3]| {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 0.0 { n.map(|x| x / len) } else { [0.0; 3] }
    };
    let dir: Vec<[f32; 3]> = area.iter().map(|&n| unit(n)).collect();
    let cos = |f: usize| face_cos.get(f).copied().unwrap_or(-1.0);
    // Corners joined across soft edges (union-find over face corners).
    let mut parent: Vec<u32> = (0..indices.len() as u32).collect();
    fn root(parent: &mut [u32], mut k: u32) -> u32 {
        while parent[k as usize] != k {
            parent[k as usize] = parent[parent[k as usize] as usize];
            k = parent[k as usize];
        }
        k
    }
    // Edge (low, high vertex) → (face, corner at low, corner at high).
    let mut edges: HashMap<(u32, u32), Vec<(u32, u32, u32)>> = HashMap::new();
    for (f, &c) in counts.iter().enumerate() {
        let (s0, n) = (starts[f], c as usize);
        for i in 0..n {
            let (ka, kb) = (s0 + i, s0 + (i + 1) % n);
            let (a, b) = (indices[ka], indices[kb]);
            if a == b {
                continue;
            }
            let e = if a < b { ((a, b), (ka as u32, kb as u32)) } else { ((b, a), (kb as u32, ka as u32)) };
            edges.entry(e.0).or_default().push((f as u32, e.1 .0, e.1 .1));
        }
    }
    for list in edges.values() {
        for x in 0..list.len() {
            for y in x + 1..list.len() {
                let ((f, fa, fb), (g, ga, gb)) = (list[x], list[y]);
                let (df, dg) = (dir[f as usize], dir[g as usize]);
                let c = cos(f as usize).max(cos(g as usize));
                let degenerate = df == [0.0; 3] || dg == [0.0; 3];
                if c <= 1.0 && (degenerate || df[0] * dg[0] + df[1] * dg[1] + df[2] * dg[2] >= c) {
                    for (p, q) in [(fa, ga), (fb, gb)] {
                        let (rp, rq) = (root(&mut parent, p), root(&mut parent, q));
                        if rp != rq {
                            parent[rp.max(rq) as usize] = rp.min(rq);
                        }
                    }
                }
            }
        }
    }
    // Each group: the sum of its faces' area normals.
    let mut sum: HashMap<u32, [f32; 3]> = HashMap::new();
    let roots: Vec<u32> = (0..indices.len() as u32).map(|k| root(&mut parent, k)).collect();
    for (k, &r) in roots.iter().enumerate() {
        let a = area[face_of[k] as usize];
        let n = sum.entry(r).or_insert([0.0; 3]);
        *n = [n[0] + a[0], n[1] + a[1], n[2] + a[2]];
    }
    let corner: Vec<[f32; 3]> = roots.iter().map(|r| unit(sum[r])).map(|n| if n == [0.0; 3] { [0.0, 1.0, 0.0] } else { n }).collect();
    // One normal per vertex where all its corners agree.
    let mut first: Vec<Option<[f32; 3]>> = vec![None; points.len()];
    let mut split = false;
    for (k, &v) in indices.iter().enumerate() {
        match first[v as usize] {
            None => first[v as usize] = Some(corner[k]),
            Some(n) if n != corner[k] => split = true,
            _ => {}
        }
    }
    if !split {
        return (first.into_iter().map(|n| n.unwrap_or([0.0, 1.0, 0.0])).collect(), None);
    }
    let mut values = Vec::new();
    let mut seen: HashMap<u32, u32> = HashMap::new();
    let idx = roots
        .iter()
        .zip(&corner)
        .map(|(r, n)| {
            *seen.entry(*r).or_insert_with(|| {
                values.push(*n);
                values.len() as u32 - 1
            })
        })
        .collect();
    (values, Some(idx))
}

/// Area-weighted smooth vertex normals (Newell's method per face).
pub(super) fn vertex_normals(points: &[[f32; 3]], counts: &[u32], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut acc = vec![[0f32; 3]; points.len()];
    let mut at = 0usize;
    for &c in counts {
        let f = &indices[at..at + c as usize];
        at += c as usize;
        let mut n = [0f32; 3];
        for i in 0..f.len() {
            let a = points[f[i] as usize];
            let b = points[f[(i + 1) % f.len()] as usize];
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        for &v in f {
            for k in 0..3 {
                acc[v as usize][k] += n[k];
            }
        }
    }
    for n in &mut acc {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 0.0 {
            n.iter_mut().for_each(|x| *x /= len);
        } else {
            *n = [0.0, 1.0, 0.0];
        }
    }
    acc
}
