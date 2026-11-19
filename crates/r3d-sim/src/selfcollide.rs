//! Cloth self-collision: particle–particle repulsion (docs/physics.md §5,
//! as in Müller et al., "Unified Particle Physics", 2014), with candidate
//! pairs from a spatial hash (Teschner et al. 2003).
//!
//! Every particle is a sphere of `radius` (≈ 40 % of the mesh spacing, so
//! one layer cannot slip between another's particles). Pairs that are close
//! *on the cloth itself* (geodesic rest distance below `EXCLUDE_RADII`
//! radii, through seams too) are never candidates: they touch because the
//! cloth is folding, and repelling them would resist ordinary folds.
//!
//! Particles sewn together (seam partners, transitively) that have closed
//! to within a radius of each other move as one when pushed: pushing one
//! side alone would reopen the seam, since contacts are solved after it.
//!
//! Candidate pairs are gathered once per frame with a margin and resolved
//! at every substep. They are sorted, so results are deterministic.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use glam::{Vec2, Vec3};

/// Particle radius as a fraction of the median rest edge length.
const RADIUS_PER_SPACING: f32 = 0.4;
/// Pairs closer than this many radii along the cloth do not collide.
const EXCLUDE_RADII: f32 = 4.0;
/// Extra distance when gathering candidates, for motion within a frame (m).
pub const MARGIN: f32 = 0.01;

/// What self-collision needs to know of the cloth, fixed for a scene.
#[derive(Debug, Clone, Default)]
pub struct SelfContact {
    /// Particle radius (m).
    pub radius: f32,
    /// For each particle, the particles near it on the cloth (sorted).
    pub excluded: Vec<Vec<u32>>,
    /// For each particle, the others sewn to it (its seam group, empty
    /// for most particles).
    pub sewn: Vec<Vec<u32>>,
}

/// The particles within `reach` of `start` along the cloth `graph`
/// (bounded Dijkstra); `best` is all infinite and left so.
fn patch(graph: &[Vec<(u32, f32)>], start: u32, reach: f32, best: &mut [f32]) -> Vec<u32> {
    let mut seen = vec![start];
    let Some(slot) = best.get_mut(start as usize) else {
        return Vec::new();
    };
    *slot = 0.0;
    let mut heap = BinaryHeap::from([Visit(0.0, start)]);
    while let Some(Visit(d, v)) = heap.pop() {
        if d > best[v as usize] {
            continue;
        }
        for &(u, l) in &graph[v as usize] {
            let nd = d + l;
            if nd < reach && nd < best[u as usize] {
                if best[u as usize].is_infinite() {
                    seen.push(u);
                }
                best[u as usize] = nd;
                heap.push(Visit(nd, u));
            }
        }
    }
    for v in &seen {
        best[*v as usize] = f32::INFINITY;
    }
    seen
}

/// Entry of the bounded Dijkstra search (min-heap on distance).
struct Visit(f32, u32);

impl PartialEq for Visit {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Visit {}
impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}

impl SelfContact {
    /// `rest`: flat pattern positions; `triangles` and `seams` give the
    /// cloth's connectivity. `overlaps`: pairs of points whose
    /// surroundings (within the given distance on the cloth, m) lie on
    /// each other on purpose (a buttoned band): those never collide.
    /// Indices are assumed valid.
    pub fn new(
        rest: &[Vec2],
        triangles: &[[u32; 3]],
        seams: &[[u32; 2]],
        overlaps: &[([u32; 2], f32)],
    ) -> Self {
        let n = rest.len();
        // Cloth graph: mesh edges (rest length) and seams (length 0).
        let mut graph: Vec<Vec<(u32, f32)>> = vec![Vec::new(); n];
        let mut lengths = Vec::with_capacity(triangles.len() * 3);
        for t in triangles {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                let l = rest[a as usize].distance(rest[b as usize]);
                lengths.push(l);
                graph[a as usize].push((b, l));
                graph[b as usize].push((a, l));
            }
        }
        for [a, b] in seams {
            graph[*a as usize].push((*b, 0.0));
            graph[*b as usize].push((*a, 0.0));
        }
        lengths.sort_by(f32::total_cmp);
        let spacing = lengths.get(lengths.len() / 2).copied().unwrap_or(0.0);
        let radius = RADIUS_PER_SPACING * spacing;
        let reach = EXCLUDE_RADII * radius;

        let mut best = vec![f32::INFINITY; n];
        let mut excluded = Vec::with_capacity(n);
        for start in 0..n {
            let mut seen = Vec::new();
            let mut heap = BinaryHeap::from([Visit(0.0, start as u32)]);
            best[start] = 0.0;
            seen.push(start as u32);
            while let Some(Visit(d, v)) = heap.pop() {
                if d > best[v as usize] {
                    continue;
                }
                for &(u, l) in &graph[v as usize] {
                    let nd = d + l;
                    if nd < reach && nd < best[u as usize] {
                        if best[u as usize].is_infinite() {
                            seen.push(u);
                        }
                        best[u as usize] = nd;
                        heap.push(Visit(nd, u));
                    }
                }
            }
            for v in &seen {
                best[*v as usize] = f32::INFINITY;
            }
            seen.sort_unstable();
            excluded.push(seen);
        }
        for ([a, b], reach) in overlaps {
            let (pa, pb) = (
                patch(&graph, *a, *reach, &mut best),
                patch(&graph, *b, *reach, &mut best),
            );
            for (from, to) in [(&pa, &pb), (&pb, &pa)] {
                for i in from {
                    if let Some(list) = excluded.get_mut(*i as usize) {
                        list.extend_from_slice(to);
                        list.sort_unstable();
                        list.dedup();
                    }
                }
            }
        }
        Self {
            radius,
            excluded,
            sewn: seam_groups(n, seams),
        }
    }

    /// The particles that move with `i` when it is pushed: itself and the
    /// particles sewn to it that are within a radius (the seam has closed).
    fn welded<'a>(&'a self, x: &'a [Vec3], i: usize) -> impl Iterator<Item = usize> + 'a {
        let r2 = self.radius * self.radius;
        let partners = self.sewn.get(i).map(Vec::as_slice).unwrap_or_default();
        std::iter::once(i).chain(
            partners
                .iter()
                .map(|j| *j as usize)
                .filter(move |j| x[*j].distance_squared(x[i]) <= r2),
        )
    }

    /// Inverse mass of `i` with what is welded to it (0 if any is pinned).
    fn welded_w(&self, x: &[Vec3], w: &[f32], i: usize) -> f32 {
        let mut mass = 0.0;
        for j in self.welded(x, i) {
            if w[j] == 0.0 {
                return 0.0;
            }
            mass += 1.0 / w[j];
        }
        1.0 / mass
    }

    /// Move `i` and what is welded to it by `delta`.
    fn shift(&self, x: &mut [Vec3], i: usize, delta: Vec3) {
        let partners = self.sewn.get(i).map(Vec::as_slice).unwrap_or_default();
        let r2 = self.radius * self.radius;
        let at = x[i];
        for j in partners.iter().map(|j| *j as usize) {
            if x[j].distance_squared(at) <= r2 {
                x[j] += delta;
            }
        }
        x[i] += delta;
    }

    /// Gather into `pairs` those that may touch during the next frame.
    pub fn candidates(&self, x: &[Vec3], w: &[f32], pairs: &mut Vec<[u32; 2]>) {
        pairs.clear();
        if self.radius <= 0.0 {
            return;
        }
        let reach = 2.0 * self.radius + MARGIN;
        let cell = |p: Vec3| (p / reach).floor().as_ivec3().to_array();
        let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (i, p) in x.iter().enumerate() {
            grid.entry(cell(*p)).or_default().push(i as u32);
        }
        for (i, p) in x.iter().enumerate() {
            let c = cell(*p);
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let Some(list) = grid.get(&[c[0] + dx, c[1] + dy, c[2] + dz]) else {
                            continue;
                        };
                        for &j in list {
                            let (iu, ju) = (i as u32, j);
                            if ju <= iu
                                || w[i] + w[j as usize] == 0.0
                                || x[j as usize].distance_squared(*p) > reach * reach
                                || self.excluded[i].binary_search(&ju).is_ok()
                            {
                                continue;
                            }
                            pairs.push([iu, ju]);
                        }
                    }
                }
            }
        }
        pairs.sort_unstable();
    }

    /// Push apart the candidate `pairs` that overlap.
    pub fn solve(&self, pairs: &[[u32; 2]], x: &mut [Vec3], w: &[f32]) {
        let min = 2.0 * self.radius;
        for [a, b] in pairs {
            let (a, b) = (*a as usize, *b as usize);
            let d = x[b] - x[a];
            let len = d.length();
            if len >= min || len < 1e-9 {
                continue;
            }
            let (wa, wb) = (self.welded_w(x, w, a), self.welded_w(x, w, b));
            if wa + wb == 0.0 {
                continue;
            }
            let push = d / len * ((min - len) / (wa + wb));
            self.shift(x, a, -push * wa);
            self.shift(x, b, push * wb);
        }
    }

    #[cfg(test)]
    fn is_excluded(&self, a: u32, b: u32) -> bool {
        self.excluded[a as usize].binary_search(&b).is_ok()
    }
}

/// For each of `n` particles, the others joined to it by seams, directly
/// or through other seams (union–find).
fn seam_groups(n: usize, seams: &[[u32; 2]]) -> Vec<Vec<u32>> {
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn root(parent: &mut [u32], mut i: u32) -> u32 {
        while parent[i as usize] != i {
            let up = parent[parent[i as usize] as usize];
            parent[i as usize] = up;
            i = up;
        }
        i
    }
    for [a, b] in seams {
        let (ra, rb) = (root(&mut parent, *a), root(&mut parent, *b));
        if ra != rb {
            parent[ra.max(rb) as usize] = ra.min(rb);
        }
    }
    let mut members: HashMap<u32, Vec<u32>> = HashMap::new();
    for i in 0..n as u32 {
        let r = root(&mut parent, i);
        members.entry(r).or_default().push(i);
    }
    let mut sewn = vec![Vec::new(); n];
    for group in members.values().filter(|g| g.len() > 1) {
        for &i in group {
            sewn[i as usize] = group.iter().copied().filter(|j| *j != i).collect();
        }
    }
    sewn
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strip of `n` squares of side 1 cm along x.
    fn strip(n: u32) -> (Vec<Vec2>, Vec<[u32; 3]>) {
        let rest = (0..=n)
            .flat_map(|i| {
                [
                    Vec2::new(i as f32 * 0.01, 0.0),
                    Vec2::new(i as f32 * 0.01, 0.01),
                ]
            })
            .collect();
        let triangles = (0..n)
            .flat_map(|i| {
                let a = 2 * i;
                [[a, a + 2, a + 3], [a, a + 3, a + 1]]
            })
            .collect();
        (rest, triangles)
    }

    #[test]
    fn near_on_the_cloth_is_excluded_far_is_not() {
        let (rest, tris) = strip(20);
        let s = SelfContact::new(&rest, &tris, &[], &[]);
        assert!((s.radius - 0.004).abs() < 1e-6);
        // Reach 4 radii = 16 mm: 1 cm and the 1.4 cm diagonal are in, 3 cm
        // along the strip is out.
        assert!(s.is_excluded(0, 2) && s.is_excluded(0, 3));
        assert!(!s.is_excluded(0, 6) && !s.is_excluded(0, 40));
        // A seam joining both ends makes them neighbours.
        let s = SelfContact::new(&rest, &tris, &[[0, 40]], &[]);
        assert!(s.is_excluded(0, 40) && s.is_excluded(2, 40));
    }

    #[test]
    fn an_overlap_excludes_both_patches() {
        // Two strips, their first ends buttoned together: everything
        // within 2 cm of one end ignores everything within 2 cm of the
        // other, and no more.
        let (mut rest, mut tris) = strip(20);
        let (rest2, tris2) = strip(20);
        rest.extend(rest2);
        tris.extend(tris2.iter().map(|t| t.map(|v| v + 42)));
        let s = SelfContact::new(&rest, &tris, &[[0, 42]], &[([0, 42], 0.02)]);
        assert!(s.is_excluded(2, 45) && s.is_excluded(45, 2));
        assert!(!s.is_excluded(8, 50));
    }

    #[test]
    fn a_closed_seam_is_pushed_as_one() {
        // Two strips sewn end to end (0 ↔ 42), closed; particle 20 of the
        // first strip, far along the cloth, lies on the seam.
        let (mut rest, mut tris) = strip(20);
        let (rest2, tris2) = strip(20);
        rest.extend(rest2);
        tris.extend(tris2.iter().map(|t| t.map(|v| v + 42)));
        let s = SelfContact::new(&rest, &tris, &[[0, 42]], &[]);
        let mut x: Vec<Vec3> = (0..84).map(|i| Vec3::new(i as f32, 1.0, 0.0)).collect();
        x[0] = Vec3::ZERO;
        x[42] = Vec3::new(0.0, 0.0, 0.0005);
        x[20] = Vec3::new(0.001, 0.0, 0.0);
        let w = vec![1.0; 84];
        let mut pairs = Vec::new();
        s.candidates(&x, &w, &mut pairs);
        s.solve(&pairs, &mut x, &w);
        // Pushed apart, and the seam stayed as it was: both sides moved
        // together, as one particle of twice the mass.
        assert!(x[20].distance(x[0]) > 1.9 * s.radius - 1e-6);
        assert!((x[42] - x[0]).distance(Vec3::new(0.0, 0.0, 0.0005)) < 1e-6);
        assert!(x[20].x - 0.001 > 1.9 * (0.0 - x[0].x));
    }

    #[test]
    fn seam_groups_follow_chains() {
        let g = seam_groups(5, &[[0, 3], [3, 4]]);
        assert_eq!(g[0], vec![3, 4]);
        assert_eq!(g[4], vec![0, 3]);
        assert!(g[1].is_empty() && g[2].is_empty());
    }
}
