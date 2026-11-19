//! Triangle meshes in 3D (an imported body): how they hang together and
//! which way they face.
//!
//! [`topology`] counts what makes a surface unfit for a signed distance:
//! holes (boundary loops), edges shared by more than two triangles,
//! separate pieces and flat triangles. [`orient`] turns triangles so that
//! neighbours agree and every piece faces outward (positive volume).

use std::collections::HashMap;

use glam::Vec3;

/// An undirected edge, its smaller vertex first.
pub type EdgeKey = (u32, u32);

fn key(a: u32, b: u32) -> EdgeKey {
    if a < b { (a, b) } else { (b, a) }
}

/// For each edge, the triangles using it and whether they run it from
/// its smaller vertex to its larger one.
pub fn edges(triangles: &[[u32; 3]]) -> HashMap<EdgeKey, Vec<(usize, bool)>> {
    let mut map: HashMap<EdgeKey, Vec<(usize, bool)>> = HashMap::new();
    for (i, t) in triangles.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            map.entry(key(a, b)).or_default().push((i, a < b));
        }
    }
    map
}

/// What a surface is made of, for the check of an imported body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Topology {
    /// Edges used by one triangle only.
    pub boundary_edges: usize,
    /// Loops of boundary edges.
    pub holes: usize,
    /// Edges used by more than two triangles.
    pub non_manifold_edges: usize,
    /// Groups of triangles joined by edges.
    pub pieces: usize,
    /// Triangles without area (or with a repeated corner).
    pub degenerate: usize,
}

impl Topology {
    /// Closed, every edge between exactly two triangles.
    pub fn watertight(&self) -> bool {
        self.boundary_edges == 0 && self.non_manifold_edges == 0
    }
}

/// Union–find over `n` items.
struct Sets(Vec<usize>);

impl Sets {
    fn new(n: usize) -> Self {
        Self((0..n).collect())
    }

    fn root(&mut self, mut i: usize) -> usize {
        while let Some(&p) = self.0.get(i) {
            if p == i {
                break;
            }
            let grand = self.0.get(p).copied().unwrap_or(p);
            if let Some(slot) = self.0.get_mut(i) {
                *slot = grand;
            }
            i = p;
        }
        i
    }

    fn join(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.root(a), self.root(b));
        if ra != rb
            && let Some(slot) = self.0.get_mut(ra)
        {
            *slot = rb;
        }
    }
}

/// Whether a triangle has no area compared to the mesh's size `scale`.
fn flat(positions: &[Vec3], t: &[u32; 3], scale: f32) -> bool {
    let [Some(a), Some(b), Some(c)] = t.map(|v| positions.get(v as usize)) else {
        return true;
    };
    t[0] == t[1]
        || t[1] == t[2]
        || t[0] == t[2]
        || (*b - *a).cross(*c - *a).length() <= 1e-10 * scale * scale
}

/// The size of a mesh: its bounding box's diagonal.
fn size(positions: &[Vec3]) -> f32 {
    let (lo, hi) = positions
        .iter()
        .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
            (lo.min(*p), hi.max(*p))
        });
    if lo.x > hi.x { 0.0 } else { lo.distance(hi) }
}

/// The triangles of `triangles` that have an area and valid corners.
pub fn without_degenerate(positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<[u32; 3]> {
    let scale = size(positions);
    triangles
        .iter()
        .filter(|t| !flat(positions, t, scale))
        .copied()
        .collect()
}

/// Count holes, non-manifold edges, pieces and flat triangles.
pub fn topology(positions: &[Vec3], triangles: &[[u32; 3]]) -> Topology {
    let scale = size(positions);
    let degenerate = triangles
        .iter()
        .filter(|t| flat(positions, t, scale))
        .count();
    let map = edges(triangles);
    let mut boundary = Sets::new(positions.len());
    let (mut boundary_edges, mut non_manifold_edges) = (0, 0);
    let mut on_boundary = Vec::new();
    for (&(a, b), users) in &map {
        match users.len() {
            1 => {
                boundary_edges += 1;
                boundary.join(a as usize, b as usize);
                on_boundary.push(a as usize);
            }
            2 => {}
            _ => non_manifold_edges += 1,
        }
    }
    let mut loops: Vec<usize> = on_boundary.iter().map(|v| boundary.root(*v)).collect();
    loops.sort_unstable();
    loops.dedup();
    let mut pieces = Sets::new(triangles.len());
    for users in map.values() {
        if let Some((first, _)) = users.first() {
            for (other, _) in users.iter().skip(1) {
                pieces.join(*first, *other);
            }
        }
    }
    let mut roots: Vec<usize> = (0..triangles.len()).map(|t| pieces.root(t)).collect();
    roots.sort_unstable();
    roots.dedup();
    Topology {
        boundary_edges,
        holes: loops.len(),
        non_manifold_edges,
        pieces: roots.len(),
        degenerate,
    }
}

/// What [`orient`] changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Oriented {
    /// Triangles turned to agree with their neighbours.
    pub turned: usize,
    /// The largest piece faced inward as a whole and was turned.
    pub inside_out: bool,
}

/// Six times the signed volume enclosed by `triangles` (positive when
/// they face outward), in `f64`.
fn volume6(positions: &[Vec3], triangles: impl Iterator<Item = [u32; 3]>) -> f64 {
    triangles
        .filter_map(|t| {
            let [a, b, c] = t.map(|v| positions.get(v as usize).map(|p| p.as_dvec3()));
            Some(a?.dot(b?.cross(c?)))
        })
        .sum()
}

/// Turn triangles so that neighbours across every two-triangle edge run
/// it in opposite directions, then turn each piece whose volume is
/// negative: the surface faces outward.
pub fn orient(positions: &[Vec3], triangles: &mut [[u32; 3]]) -> Oriented {
    let original: Vec<[u32; 3]> = triangles.to_vec();
    let map = edges(triangles);
    let mut flip = vec![false; triangles.len()];
    let mut seen = vec![false; triangles.len()];
    let mut pieces: Vec<Vec<usize>> = Vec::new();
    for seed in 0..triangles.len() {
        if seen[seed] {
            continue;
        }
        seen[seed] = true;
        let mut piece = vec![seed];
        let mut stack = vec![seed];
        while let Some(t) = stack.pop() {
            let Some(tri) = original.get(t) else { continue };
            for k in 0..3 {
                let (a, b) = (tri[k], tri[(k + 1) % 3]);
                let Some(users) = map.get(&key(a, b)) else {
                    continue;
                };
                if users.len() != 2 {
                    continue;
                }
                let mine = (a < b) != flip[t];
                for &(other, forward) in users {
                    if other == t || seen[other] {
                        continue;
                    }
                    // Neighbours must run the edge in opposite directions.
                    flip[other] = forward == mine;
                    seen[other] = true;
                    piece.push(other);
                    stack.push(other);
                }
            }
        }
        pieces.push(piece);
    }
    let turned_tri = |t: [u32; 3]| [t[0], t[2], t[1]];
    for (t, f) in triangles.iter_mut().zip(&flip) {
        if *f {
            *t = turned_tri(*t);
        }
    }
    let largest = pieces
        .iter()
        .enumerate()
        .max_by_key(|(_, p)| p.len())
        .map(|(i, _)| i);
    let mut out = Oriented::default();
    for (i, piece) in pieces.iter().enumerate() {
        let inward = volume6(
            positions,
            piece.iter().filter_map(|t| triangles.get(*t).copied()),
        ) < 0.0;
        if inward {
            for t in piece {
                if let Some(slot) = triangles.get_mut(*t) {
                    *slot = turned_tri(*slot);
                }
            }
        }
        let changed = piece
            .iter()
            .filter(|t| triangles.get(**t) != original.get(**t))
            .count();
        // Most of the piece turned: it was inside out, and the few that
        // did not were the ones out of step.
        if 2 * changed > piece.len() {
            out.turned += piece.len() - changed;
            if Some(i) == largest {
                out.inside_out = true;
            }
        } else {
            out.turned += changed;
        }
    }
    out
}

/// The vertices on an edge that is open or shared by more than two
/// triangles: where a pseudo-normal does not tell inside from outside.
pub fn bad_vertices(n: usize, triangles: &[[u32; 3]]) -> Vec<bool> {
    let mut bad = vec![false; n];
    for (&(a, b), users) in &edges(triangles) {
        if users.len() != 2 {
            for v in [a, b] {
                if let Some(slot) = bad.get_mut(v as usize) {
                    *slot = true;
                }
            }
        }
    }
    bad
}

/// The signed volume enclosed by a mesh (m³ for metres; positive when it
/// faces outward).
pub fn volume(positions: &[Vec3], triangles: &[[u32; 3]]) -> f64 {
    volume6(positions, triangles.iter().copied()) / 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit cube, outward-facing.
    fn cube() -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let p = (0..8)
            .map(|i| Vec3::new((i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32))
            .collect();
        let quads = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let t = quads
            .iter()
            .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
            .collect();
        (p, t)
    }

    #[test]
    fn a_cube_is_closed_and_faces_out() {
        let (p, mut t) = cube();
        let topo = topology(&p, &t);
        assert!(topo.watertight());
        assert_eq!(topo.pieces, 1);
        assert!((volume(&p, &t) - 1.0).abs() < 1e-9);
        assert_eq!(orient(&p, &mut t), Oriented::default());
    }

    #[test]
    fn holes_flips_and_inside_out_are_found_and_fixed() {
        let (p, mut t) = cube();
        // One flipped triangle.
        t[3] = [t[3][0], t[3][2], t[3][1]];
        let fixed = orient(&p, &mut t);
        assert_eq!(fixed.turned, 1);
        assert!(!fixed.inside_out);
        assert!(volume(&p, &t) > 0.0);
        // All of them.
        for tri in &mut t {
            *tri = [tri[0], tri[2], tri[1]];
        }
        let fixed = orient(&p, &mut t);
        assert!(fixed.inside_out);
        assert!(volume(&p, &t) > 0.0);
        // A hole: one face gone.
        t.truncate(10);
        let topo = topology(&p, &t);
        assert_eq!((topo.holes, topo.boundary_edges), (1, 4));
        assert_eq!(bad_vertices(8, &t).iter().filter(|b| **b).count(), 4);
        // A degenerate triangle, and a separate piece.
        let mut p2 = p.clone();
        p2.extend([
            Vec3::splat(5.0),
            Vec3::new(6.0, 5.0, 5.0),
            Vec3::new(5.0, 6.0, 5.0),
        ]);
        let mut t2 = t.clone();
        t2.push([8, 9, 10]);
        t2.push([0, 0, 1]);
        let topo = topology(&p2, &t2);
        assert_eq!(topo.degenerate, 1);
        assert_eq!(topo.pieces, 2);
        assert_eq!(without_degenerate(&p2, &t2).len(), t2.len() - 1);
    }
}
