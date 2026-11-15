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
const MARGIN: f32 = 0.01;

pub(crate) struct SelfCollision {
    radius: f32,
    /// For each particle, the particles near it on the cloth (sorted).
    excluded: Vec<Vec<u32>>,
    pairs: Vec<[u32; 2]>,
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

impl SelfCollision {
    /// `rest`: flat pattern positions; `triangles` and `seams` give the
    /// cloth's connectivity. Indices are assumed valid.
    pub fn new(rest: &[Vec2], triangles: &[[u32; 3]], seams: &[[u32; 2]]) -> Self {
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
        Self {
            radius,
            excluded,
            pairs: Vec::new(),
        }
    }

    /// Gather the pairs that may touch during the next frame.
    pub fn refresh(&mut self, x: &[Vec3], w: &[f32]) {
        self.pairs.clear();
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
                            self.pairs.push([iu, ju]);
                        }
                    }
                }
            }
        }
        self.pairs.sort_unstable();
    }

    /// Push apart the candidate pairs that overlap.
    pub fn solve(&self, x: &mut [Vec3], w: &[f32]) {
        let min = 2.0 * self.radius;
        for [a, b] in &self.pairs {
            let (a, b) = (*a as usize, *b as usize);
            let d = x[b] - x[a];
            let len = d.length();
            let wsum = w[a] + w[b];
            if len >= min || len < 1e-9 || wsum == 0.0 {
                continue;
            }
            let push = d / len * ((min - len) / wsum);
            x[a] -= push * w[a];
            x[b] += push * w[b];
        }
    }

    #[cfg(test)]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    #[cfg(test)]
    pub fn is_excluded(&self, a: u32, b: u32) -> bool {
        self.excluded[a as usize].binary_search(&b).is_ok()
    }
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
        let s = SelfCollision::new(&rest, &tris, &[]);
        assert!((s.radius() - 0.004).abs() < 1e-6);
        // Reach 4 radii = 16 mm: 1 cm and the 1.4 cm diagonal are in, 3 cm
        // along the strip is out.
        assert!(s.is_excluded(0, 2) && s.is_excluded(0, 3));
        assert!(!s.is_excluded(0, 6) && !s.is_excluded(0, 40));
        // A seam joining both ends makes them neighbours.
        let s = SelfCollision::new(&rest, &tris, &[[0, 40]]);
        assert!(s.is_excluded(0, 40) && s.is_excluded(2, 40));
    }
}
