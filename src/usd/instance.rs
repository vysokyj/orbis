//! Instances (DAZ instance nodes, UltraScatter's instance groups): a node's
//! mesh drawn again elsewhere, written as a USD `PointInstancer`.

use super::*;

/// A copy's placement relative to its target's own, in meters: rows of a
/// 3×4 affine transform (translation in the 4th column).
pub(super) type Placement = [f32; 12];

pub(super) const IDENTITY: Placement = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];

/// Where the copies of a node go, relative to its world-space mesh — `None`
/// when nothing instances it. An instance's transform replaces its target's
/// (UltraScatter's trees stand on the terrain only so), so a copy is
/// instance · target⁻¹. The node itself is drawn too when it is `visible`.
pub(super) fn placements(dump: &SceneDump, label: &str, name: &str, target: &DumpNode, visible: bool) -> Option<Vec<Placement>> {
    let copies: Vec<&Affine> = dump
        .instances
        .iter()
        .filter(|i| i.target_label == label || (i.target_label.is_empty() && i.target_name == name))
        .flat_map(|i| &i.transforms)
        .collect();
    if copies.is_empty() {
        return None;
    }
    let Some(inverse) = target.transform.as_ref().and_then(invert) else {
        tracing::warn!(node = label, "USD export: instanced node without an invertible transform in the dump");
        return None;
    };
    let mut out = Vec::with_capacity(copies.len() + 1);
    if visible {
        out.push(IDENTITY);
    }
    for c in copies {
        let m = compose(c, &inverse);
        out.push(std::array::from_fn(|k| if k % 4 == 3 { (m[k] * CM as f64) as f32 } else { m[k] as f32 }));
    }
    Some(out)
}

/// a · b (b applied first).
pub(super) fn compose(a: &Affine, b: &Affine) -> Affine {
    let mut m = [0.0; 12];
    for r in 0..3 {
        for c in 0..4 {
            m[r * 4 + c] = (0..3).map(|k| a[r * 4 + k] * b[k * 4 + c]).sum::<f64>() + if c == 3 { a[r * 4 + 3] } else { 0.0 };
        }
    }
    m
}

pub(super) fn invert(m: &Affine) -> Option<Affine> {
    let a = |r: usize, c: usize| m[r * 4 + c];
    let cof = |r: usize, c: usize| {
        let (r0, r1) = ((r + 1) % 3, (r + 2) % 3);
        let (c0, c1) = ((c + 1) % 3, (c + 2) % 3);
        a(r0, c0) * a(r1, c1) - a(r0, c1) * a(r1, c0)
    };
    let det: f64 = (0..3).map(|c| a(0, c) * cof(0, c)).sum();
    if det.abs() < 1e-12 {
        return None;
    }
    let mut out = [0.0; 12];
    for r in 0..3 {
        for c in 0..3 {
            out[r * 4 + c] = cof(c, r) / det;
        }
    }
    for r in 0..3 {
        out[r * 4 + 3] = -(0..3).map(|k| out[r * 4 + k] * a(k, 3)).sum::<f64>();
    }
    Some(out)
}

/// A placement as `PointInstancer` parts: position, orientation (w, x, y, z)
/// and per-axis scale — translate · rotate · scale. Shear (a non-uniform
/// scale under a rotation further up) is lost.
pub(super) fn decompose(p: &Placement) -> ([f32; 3], [f32; 4], [f32; 3]) {
    let col = |c: usize| [p[c], p[4 + c], p[8 + c]];
    let len = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let mut scale = [0, 1, 2].map(|c| len(col(c)));
    let mut axes = [0, 1, 2].map(|c| col(c).map(|v| if scale[c] > 0.0 { v / scale[c] } else { 0.0 }));
    let [x, y, z] = axes;
    let det = x[0] * (y[1] * z[2] - y[2] * z[1]) - y[0] * (x[1] * z[2] - x[2] * z[1]) + z[0] * (x[1] * y[2] - x[2] * y[1]);
    if det < 0.0 {
        scale[0] = -scale[0];
        axes[0] = axes[0].map(|v| -v);
    }
    // Rotation matrix (columns = axes) → quaternion.
    let r = |row: usize, c: usize| axes[c][row];
    let trace = r(0, 0) + r(1, 1) + r(2, 2);
    let q = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [0.25 * s, (r(2, 1) - r(1, 2)) / s, (r(0, 2) - r(2, 0)) / s, (r(1, 0) - r(0, 1)) / s]
    } else if r(0, 0) > r(1, 1) && r(0, 0) > r(2, 2) {
        let s = (1.0 + r(0, 0) - r(1, 1) - r(2, 2)).sqrt() * 2.0;
        [(r(2, 1) - r(1, 2)) / s, 0.25 * s, (r(0, 1) + r(1, 0)) / s, (r(0, 2) + r(2, 0)) / s]
    } else if r(1, 1) > r(2, 2) {
        let s = (1.0 + r(1, 1) - r(0, 0) - r(2, 2)).sqrt() * 2.0;
        [(r(0, 2) - r(2, 0)) / s, (r(0, 1) + r(1, 0)) / s, 0.25 * s, (r(1, 2) + r(2, 1)) / s]
    } else {
        let s = (1.0 + r(2, 2) - r(0, 0) - r(1, 1)).sqrt() * 2.0;
        [(r(1, 0) - r(0, 1)) / s, (r(0, 2) + r(2, 0)) / s, (r(1, 2) + r(2, 1)) / s, 0.25 * s]
    };
    ([p[3], p[7], p[11]], q, scale)
}
