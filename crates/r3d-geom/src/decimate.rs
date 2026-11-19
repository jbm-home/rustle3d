//! Triangle mesh simplification by quadric error metrics (Garland &
//! Heckbert, "Surface Simplification Using Quadric Error Metrics",
//! SIGGRAPH 1997), for the lighter levels of detail of a game export.
//!
//! Each vertex carries the sum of the squared distances to the planes of
//! its triangles (a quadric, weighted by area); an edge collapse costs
//! the quadric error of the vertex that is kept. Collapses are half-edge
//! collapses onto an existing vertex (the paper's "subset placement"), so
//! the simplified mesh uses a subset of the original vertices and keeps
//! their texture coordinates and skin weights as they are.
//!
//! Vertices on the mesh's border (edges used by one triangle) or on a
//! non-manifold edge never move: a pattern piece keeps its outline, so
//! seams stay closed. A collapse is refused when it would pinch the
//! surface (the link condition), turn a triangle over in 3D, or, when
//! texture coordinates are given, turn one over in texture space.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use glam::{DVec3, Vec2, Vec3};

/// A triangle may tilt at most this far in one collapse (cosine of the
/// angle between its normals before and after).
const MIN_TILT_COS: f64 = 0.2;

/// Sum of squared distances to planes, as the 10 coefficients of a
/// symmetric 4 × 4 matrix.
#[derive(Clone, Copy, Debug, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    /// The plane `n · p + d = 0` (unit `n`), weighted by `w`.
    fn plane(n: DVec3, d: f64, w: f64) -> Self {
        let (a, b, c) = (n.x, n.y, n.z);
        Self([
            a * a,
            a * b,
            a * c,
            a * d,
            b * b,
            b * c,
            b * d,
            c * c,
            c * d,
            d * d,
        ])
        .scaled(w)
    }

    fn scaled(self, w: f64) -> Self {
        Self(self.0.map(|x| x * w))
    }

    fn plus(&self, o: &Self) -> Self {
        let mut out = *self;
        for (a, b) in out.0.iter_mut().zip(o.0) {
            *a += b;
        }
        out
    }

    fn error(&self, p: DVec3) -> f64 {
        let [xx, xy, xz, xw, yy, yz, yw, zz, zw, ww] = self.0;
        let (x, y, z) = (p.x, p.y, p.z);
        xx * x * x
            + 2.0 * xy * x * y
            + 2.0 * xz * x * z
            + 2.0 * xw * x
            + yy * y * y
            + 2.0 * yz * y * z
            + 2.0 * yw * y
            + zz * z * z
            + 2.0 * zw * z
            + ww
    }
}

/// A candidate collapse of `from` onto its best neighbour, valid while
/// `from`'s stamp has not changed.
#[derive(Debug, PartialEq)]
struct Entry {
    cost: f64,
    from: u32,
    stamp: u32,
}

impl Eq for Entry {}

impl Ord for Entry {
    // The cheapest first; ties by vertex, so the result is deterministic.
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost
            .total_cmp(&self.cost)
            .then(o.from.cmp(&self.from))
            .then(o.stamp.cmp(&self.stamp))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

struct Simplifier<'a> {
    positions: Vec<DVec3>,
    uvs: &'a [Vec2],
    tris: Vec<[u32; 3]>,
    alive: Vec<bool>,
    /// Sign of each triangle's area in texture space, as given.
    uv_sign: Vec<f64>,
    incident: Vec<Vec<u32>>,
    locked: Vec<bool>,
    quadrics: Vec<Quadric>,
    stamps: Vec<u32>,
}

fn cross3(p: [DVec3; 3]) -> DVec3 {
    (p[1] - p[0]).cross(p[2] - p[0])
}

fn uv_area(p: [Vec2; 3]) -> f64 {
    f64::from((p[1] - p[0]).perp_dot(p[2] - p[0]))
}

impl Simplifier<'_> {
    fn pos(&self, v: u32) -> DVec3 {
        self.positions.get(v as usize).copied().unwrap_or_default()
    }

    fn uv(&self, v: u32) -> Vec2 {
        self.uvs.get(v as usize).copied().unwrap_or_default()
    }

    fn tri(&self, t: u32) -> [u32; 3] {
        self.tris.get(t as usize).copied().unwrap_or_default()
    }

    fn around(&self, v: u32) -> &[u32] {
        self.incident.get(v as usize).map_or(&[], Vec::as_slice)
    }

    /// The vertices sharing a triangle with `v`, sorted.
    fn neighbours(&self, v: u32) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .around(v)
            .iter()
            .flat_map(|t| self.tri(*t))
            .filter(|w| *w != v)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `u` may collapse onto `v`.
    fn valid(&self, u: u32, v: u32) -> bool {
        if self.locked.get(u as usize).copied().unwrap_or(true) {
            return false;
        }
        // Link condition: the only vertices next to both are the ones
        // opposite the edge (Dey et al.; keeps the surface a manifold).
        let shared: Vec<u32> = self
            .around(u)
            .iter()
            .map(|t| self.tri(*t))
            .filter(|t| t.contains(&v))
            .flat_map(|t| t.into_iter().filter(|w| *w != u && *w != v))
            .collect();
        let nv = self.neighbours(v);
        let common = self
            .neighbours(u)
            .iter()
            .filter(|w| nv.binary_search(w).is_ok())
            .count();
        if shared.is_empty() || common != shared.len() {
            return false;
        }
        // No triangle may turn over, in 3D or in texture space.
        for t in self.around(u) {
            let corners = self.tri(*t);
            if corners.contains(&v) {
                continue;
            }
            let moved = corners.map(|w| if w == u { v } else { w });
            let before = cross3(corners.map(|w| self.pos(w)));
            let after = cross3(moved.map(|w| self.pos(w)));
            let (lb, la) = (before.length(), after.length());
            if la <= 1e-12 * lb.max(1e-30) || before.dot(after) < MIN_TILT_COS * lb * la {
                return false;
            }
            if !self.uvs.is_empty() {
                let area = uv_area(moved.map(|w| self.uv(w)));
                let sign = self.uv_sign.get(*t as usize).copied().unwrap_or(0.0);
                if area * sign <= 1e-14 {
                    return false;
                }
            }
        }
        true
    }

    /// The cheapest valid collapse of `u`: its cost and the vertex kept.
    fn best(&self, u: u32) -> Option<(f64, u32)> {
        let qu = self.quadrics.get(u as usize)?;
        self.neighbours(u)
            .into_iter()
            .filter(|v| self.valid(u, *v))
            .filter_map(|v| {
                Some((
                    qu.plus(self.quadrics.get(v as usize)?).error(self.pos(v)),
                    v,
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
    }

    fn push(&mut self, heap: &mut BinaryHeap<Entry>, u: u32) {
        let Some(stamp) = self.stamps.get_mut(u as usize) else {
            return;
        };
        *stamp += 1;
        let stamp = *stamp;
        if let Some((cost, _)) = self.best(u) {
            heap.push(Entry {
                cost,
                from: u,
                stamp,
            });
        }
    }

    /// Collapse `u` onto `v`; the number of triangles removed.
    fn collapse(&mut self, u: u32, v: u32) -> usize {
        let mut removed = 0;
        let around: Vec<u32> = self.around(u).to_vec();
        for t in around {
            let corners = self.tri(t);
            if corners.contains(&v) {
                if let Some(a) = self.alive.get_mut(t as usize) {
                    *a = false;
                }
                removed += 1;
                for w in corners {
                    if w != u
                        && let Some(list) = self.incident.get_mut(w as usize)
                    {
                        list.retain(|x| *x != t);
                    }
                }
            } else {
                if let Some(c) = self.tris.get_mut(t as usize) {
                    *c = corners.map(|w| if w == u { v } else { w });
                }
                if let Some(list) = self.incident.get_mut(v as usize) {
                    list.push(t);
                }
            }
        }
        if let Some(list) = self.incident.get_mut(u as usize) {
            list.clear();
        }
        let qu = self.quadrics.get(u as usize).copied().unwrap_or_default();
        if let Some(qv) = self.quadrics.get_mut(v as usize) {
            *qv = qv.plus(&qu);
        }
        removed
    }
}

/// Simplify `triangles` (corners counter-clockwise, indices into
/// `positions`) to about `target` triangles; the triangles kept, with
/// corners among the original vertices. `uvs` (one per position, or
/// empty) must not turn over. Stops early when no collapse is allowed,
/// so the result can have more than `target` triangles. Deterministic.
pub fn decimate(
    positions: &[Vec3],
    uvs: &[Vec2],
    triangles: &[[u32; 3]],
    target: usize,
) -> Vec<[u32; 3]> {
    let n = positions.len();
    let uvs = if uvs.len() == n { uvs } else { &[] };
    let tris: Vec<[u32; 3]> = triangles
        .iter()
        .filter(|t| {
            t.iter().all(|v| (*v as usize) < n) && t[0] != t[1] && t[1] != t[2] && t[0] != t[2]
        })
        .copied()
        .collect();
    let mut s = Simplifier {
        positions: positions.iter().map(|p| p.as_dvec3()).collect(),
        uvs,
        alive: vec![true; tris.len()],
        uv_sign: Vec::new(),
        incident: vec![Vec::new(); n],
        locked: vec![false; n],
        quadrics: vec![Quadric::default(); n],
        stamps: vec![0; n],
        tris,
    };
    s.uv_sign = s
        .tris
        .iter()
        .map(|t| uv_area(t.map(|v| s.uv(v))).signum())
        .collect();
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    for (i, t) in s.tris.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            *edges.entry((a.min(b), a.max(b))).or_default() += 1;
        }
        let p = t.map(|v| s.positions.get(v as usize).copied().unwrap_or_default());
        let normal = cross3(p);
        let area = normal.length() / 2.0;
        if area > 0.0 {
            let unit = normal / (2.0 * area);
            let q = Quadric::plane(unit, -unit.dot(p[0]), area);
            for v in t {
                if let Some(qv) = s.quadrics.get_mut(*v as usize) {
                    *qv = qv.plus(&q);
                }
            }
        }
        for v in t {
            if let Some(list) = s.incident.get_mut(*v as usize) {
                list.push(i as u32);
            }
        }
    }
    for ((a, b), uses) in edges {
        if uses != 2 {
            for v in [a, b] {
                if let Some(l) = s.locked.get_mut(v as usize) {
                    *l = true;
                }
            }
        }
    }
    let mut count = s.tris.len();
    let mut heap = BinaryHeap::new();
    for u in 0..n as u32 {
        s.push(&mut heap, u);
    }
    while count > target {
        let Some(e) = heap.pop() else {
            break;
        };
        if s.stamps.get(e.from as usize) != Some(&e.stamp) {
            continue;
        }
        let Some((cost, v)) = s.best(e.from) else {
            continue;
        };
        if cost > e.cost + 1e-12 * (1.0 + e.cost.abs()) {
            heap.push(Entry { cost, ..e });
            continue;
        }
        count -= s.collapse(e.from, v);
        if let Some(stamp) = s.stamps.get_mut(e.from as usize) {
            *stamp += 1;
        }
        for w in std::iter::once(v).chain(s.neighbours(v)) {
            s.push(&mut heap, w);
        }
    }
    s.tris
        .iter()
        .zip(&s.alive)
        .filter(|(_, a)| **a)
        .map(|(t, _)| *t)
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use proptest::prelude::*;

    /// A `k` × `k` grid of quads on the unit square, its height `h(x, y)`.
    fn grid(k: u32, h: impl Fn(f32, f32) -> f32) -> (Vec<Vec3>, Vec<Vec2>, Vec<[u32; 3]>) {
        let mut p = Vec::new();
        let mut uv = Vec::new();
        for j in 0..=k {
            for i in 0..=k {
                let (x, y) = (i as f32 / k as f32, j as f32 / k as f32);
                p.push(Vec3::new(x, y, h(x, y)));
                uv.push(Vec2::new(x, y));
            }
        }
        let id = |i: u32, j: u32| j * (k + 1) + i;
        let mut t = Vec::new();
        for j in 0..k {
            for i in 0..k {
                t.push([id(i, j), id(i + 1, j), id(i + 1, j + 1)]);
                t.push([id(i, j), id(i + 1, j + 1), id(i, j + 1)]);
            }
        }
        (p, uv, t)
    }

    fn border(k: u32) -> Vec<u32> {
        (0..(k + 1) * (k + 1))
            .filter(|v| {
                let (i, j) = (v % (k + 1), v / (k + 1));
                i == 0 || j == 0 || i == k || j == k
            })
            .collect()
    }

    fn check(p: &[Vec3], uv: &[Vec2], before: &[[u32; 3]], after: &[[u32; 3]], k: u32) {
        let used: std::collections::HashSet<u32> = after.iter().flatten().copied().collect();
        for v in border(k) {
            assert!(used.contains(&v), "border vertex {v} lost");
        }
        let total = |tris: &[[u32; 3]]| -> f64 {
            tris.iter()
                .map(|t| uv_area(t.map(|v| uv[v as usize])))
                .sum()
        };
        // Every triangle keeps its orientation, and the pieces still cover
        // the same texture area: no fold, no hole.
        for t in after {
            assert!(uv_area(t.map(|v| uv[v as usize])) > 0.0);
            assert!(t.iter().all(|v| (*v as usize) < p.len()));
        }
        assert!((total(before) - total(after)).abs() < 1e-6);
    }

    #[test]
    fn a_flat_sheet_goes_down_to_the_target() {
        let (p, uv, t) = grid(20, |_, _| 0.0);
        let out = decimate(&p, &uv, &t, t.len() / 5);
        assert!(out.len() <= t.len() / 5, "{}", out.len());
        check(&p, &uv, &t, &out, 20);
        assert_eq!(out, decimate(&p, &uv, &t, t.len() / 5));
    }

    #[test]
    fn wrinkles_are_kept_longer_than_flat_parts() {
        // Folds on the left half, flat on the right: what is left of the
        // vertices lies mostly where the folds are.
        let (p, uv, t) = grid(24, |x, y| {
            if x < 0.5 {
                0.02 * (x * 40.0).sin() * (y * 40.0).sin()
            } else {
                0.0
            }
        });
        let out = decimate(&p, &uv, &t, t.len() / 2);
        check(&p, &uv, &t, &out, 24);
        let edge = border(24);
        let used: std::collections::HashSet<u32> = out
            .iter()
            .flatten()
            .copied()
            .filter(|v| !edge.contains(v))
            .collect();
        let left = used.iter().filter(|v| p[**v as usize].x < 0.45).count();
        let right = used.iter().filter(|v| p[**v as usize].x > 0.55).count();
        assert!(left > 3 * right, "{left} {right}");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(16))]
        #[test]
        fn decimation_keeps_borders_and_orientation(
            a in 0.0f32..0.1, f in 1.0f32..20.0, ratio in 0.05f64..1.0,
        ) {
            let (p, uv, t) = grid(10, |x, y| a * (f * x).sin() * (f * y).cos());
            let target = (t.len() as f64 * ratio) as usize;
            let out = decimate(&p, &uv, &t, target);
            check(&p, &uv, &t, &out, 10);
        }
    }
}
