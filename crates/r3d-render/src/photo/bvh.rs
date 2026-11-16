//! Bounding volume hierarchy over triangles, built on the CPU with the
//! surface area heuristic over binned centroids (Wald 2007, "On fast
//! Construction of SAH-based Bounding Volume Hierarchies") and traversed by
//! the path tracer's compute shader.
//!
//! Nodes are laid out depth first: an inner node's left child follows it,
//! its right child's index is stored in the node.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Most triangles in a leaf.
const MAX_LEAF: usize = 4;
/// Deepest node; the shader's traversal stack holds 64 entries.
const MAX_DEPTH: u32 = 48;
/// Centroid bins per axis when looking for a split.
const BINS: usize = 16;

/// A node as the shader reads it (32 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct Node {
    pub lo: [f32; 3],
    /// Leaf: first triangle (in [`Bvh::order`]). Inner: right child.
    pub index: u32,
    pub hi: [f32; 3],
    /// Triangles in a leaf; 0 for an inner node.
    pub count: u32,
}

#[derive(Clone, Copy, Debug)]
struct Aabb {
    lo: Vec3,
    hi: Vec3,
}

impl Aabb {
    const EMPTY: Self = Self {
        lo: Vec3::splat(f32::MAX),
        hi: Vec3::splat(f32::MIN),
    };

    fn grow(&mut self, other: &Aabb) {
        self.lo = self.lo.min(other.lo);
        self.hi = self.hi.max(other.hi);
    }

    fn grow_point(&mut self, p: Vec3) {
        self.lo = self.lo.min(p);
        self.hi = self.hi.max(p);
    }

    fn area(&self) -> f32 {
        let d = (self.hi - self.lo).max(Vec3::ZERO);
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }
}

/// A built hierarchy: its nodes and the triangle order its leaves use.
#[derive(Debug, Default)]
pub(crate) struct Bvh {
    pub nodes: Vec<Node>,
    /// Triangle indices, leaf after leaf.
    pub order: Vec<u32>,
}

struct Builder<'a> {
    boxes: &'a [Aabb],
    centroids: &'a [Vec3],
    order: Vec<u32>,
    nodes: Vec<Node>,
}

impl Bvh {
    /// The hierarchy over `triangles` (corner positions).
    pub fn build(triangles: &[[Vec3; 3]]) -> Self {
        let boxes: Vec<Aabb> = triangles
            .iter()
            .map(|t| {
                let mut b = Aabb::EMPTY;
                for p in t {
                    b.grow_point(*p);
                }
                b
            })
            .collect();
        let centroids: Vec<Vec3> = boxes.iter().map(|b| (b.lo + b.hi) * 0.5).collect();
        let mut builder = Builder {
            boxes: &boxes,
            centroids: &centroids,
            order: (0..triangles.len() as u32).collect(),
            nodes: Vec::with_capacity(triangles.len().max(1) * 2 / MAX_LEAF + 1),
        };
        if !triangles.is_empty() {
            builder.node(0, triangles.len(), 0);
        }
        Bvh {
            nodes: builder.nodes,
            order: builder.order,
        }
    }
}

impl Builder<'_> {
    fn bounds(&self, start: usize, end: usize) -> (Aabb, Aabb) {
        let mut b = Aabb::EMPTY;
        let mut c = Aabb::EMPTY;
        for &t in self.order.get(start..end).unwrap_or_default() {
            if let (Some(tb), Some(tc)) =
                (self.boxes.get(t as usize), self.centroids.get(t as usize))
            {
                b.grow(tb);
                c.grow_point(*tc);
            }
        }
        (b, c)
    }

    /// Build the node over `order[start..end]`; returns its index.
    fn node(&mut self, start: usize, end: usize, depth: u32) -> u32 {
        let index = self.nodes.len();
        let (bounds, centroids) = self.bounds(start, end);
        self.nodes.push(Node {
            lo: bounds.lo.to_array(),
            hi: bounds.hi.to_array(),
            index: start as u32,
            count: (end - start) as u32,
        });
        let count = end - start;
        if count <= MAX_LEAF || depth >= MAX_DEPTH {
            return index as u32;
        }
        let mid = self.split(start, end, &bounds, &centroids);
        let Some(mid) = mid else {
            return index as u32;
        };
        self.node(start, mid, depth + 1);
        let right = self.node(mid, end, depth + 1);
        if let Some(n) = self.nodes.get_mut(index) {
            n.index = right;
            n.count = 0;
        }
        index as u32
    }

    /// Partition `order[start..end]` where the surface area heuristic is
    /// lowest; `None` keeps a leaf. Falls back to halving along the
    /// longest axis when every centroid falls in one bin.
    fn split(&mut self, start: usize, end: usize, bounds: &Aabb, c: &Aabb) -> Option<usize> {
        let count = end - start;
        let extent = c.hi - c.lo;
        let mut best: Option<(f32, usize, usize)> = None; // cost, axis, bin
        for axis in 0..3 {
            if extent[axis] <= 1e-9 {
                continue;
            }
            let scale = BINS as f32 / extent[axis];
            let mut bins = [(Aabb::EMPTY, 0usize); BINS];
            for &t in self.order.get(start..end).unwrap_or_default() {
                let (Some(tc), Some(tb)) =
                    (self.centroids.get(t as usize), self.boxes.get(t as usize))
                else {
                    continue;
                };
                let b = (((tc[axis] - c.lo[axis]) * scale) as usize).min(BINS - 1);
                bins[b].0.grow(tb);
                bins[b].1 += 1;
            }
            // Sweep from the right to know each split's right side.
            let mut right_area = [0.0f32; BINS];
            let mut right_count = [0usize; BINS];
            let mut acc = Aabb::EMPTY;
            let mut n = 0;
            for i in (1..BINS).rev() {
                acc.grow(&bins[i].0);
                n += bins[i].1;
                right_area[i] = acc.area();
                right_count[i] = n;
            }
            let mut left = Aabb::EMPTY;
            let mut nl = 0;
            for i in 1..BINS {
                left.grow(&bins[i - 1].0);
                nl += bins[i - 1].1;
                let nr = right_count[i];
                if nl == 0 || nr == 0 {
                    continue;
                }
                let cost = left.area() * nl as f32 + right_area[i] * nr as f32;
                if best.is_none_or(|(b, _, _)| cost < b) {
                    best = Some((cost, axis, i));
                }
            }
        }
        let area = bounds.area().max(1e-12);
        match best {
            Some((cost, axis, bin)) => {
                // Traversing costs about one triangle test.
                if 1.0 + cost / area >= count as f32 && count <= 2 * MAX_LEAF {
                    return None;
                }
                let scale = BINS as f32 / extent[axis];
                let (lo, centroids) = (c.lo[axis], self.centroids);
                let slice = self.order.get_mut(start..end)?;
                let mut i = 0;
                for j in 0..slice.len() {
                    let b = centroids
                        .get(slice[j] as usize)
                        .map_or(0, |tc| (((tc[axis] - lo) * scale) as usize).min(BINS - 1));
                    if b < bin {
                        slice.swap(i, j);
                        i += 1;
                    }
                }
                (i > 0 && i < slice.len()).then_some(start + i)
            }
            None => {
                // All centroids coincide: split the list in two.
                (count > MAX_LEAF).then_some(start + count / 2)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Closest hit by walking the tree (as the shader does).
    fn walk(bvh: &Bvh, tris: &[[Vec3; 3]], o: Vec3, d: Vec3) -> Option<(f32, u32)> {
        let mut best: Option<(f32, u32)> = None;
        let mut stack = vec![0u32];
        while let Some(i) = stack.pop() {
            let n = bvh.nodes[i as usize];
            if !hits_box(&n, o, d, best.map_or(f32::MAX, |b| b.0)) {
                continue;
            }
            if n.count > 0 {
                for k in n.index..n.index + n.count {
                    let t = bvh.order[k as usize];
                    if let Some(h) = hit(&tris[t as usize], o, d)
                        && best.is_none_or(|b| h < b.0)
                    {
                        best = Some((h, t));
                    }
                }
            } else {
                stack.push(i + 1);
                stack.push(n.index);
            }
        }
        best
    }

    fn hits_box(n: &Node, o: Vec3, d: Vec3, tmax: f32) -> bool {
        let inv = d.recip();
        let t0 = (Vec3::from(n.lo) - o) * inv;
        let t1 = (Vec3::from(n.hi) - o) * inv;
        let enter = t0.min(t1).max_element().max(0.0);
        let exit = t0.max(t1).min_element().min(tmax);
        enter <= exit
    }

    fn hit(t: &[Vec3; 3], o: Vec3, d: Vec3) -> Option<f32> {
        let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
        let p = d.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-12 {
            return None;
        }
        let s = o - t[0];
        let u = s.dot(p) / det;
        let q = s.cross(e1);
        let v = d.dot(q) / det;
        let h = e2.dot(q) / det;
        (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && h > 0.0).then_some(h)
    }

    fn soup(seed: u64, n: usize) -> Vec<[Vec3; 3]> {
        // A small deterministic generator (no dependency for tests).
        let mut s = seed;
        let mut r = move || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (1u64 << 31) as f32
        };
        (0..n)
            .map(|_| {
                let c = Vec3::new(r(), r(), r()) * 2.0 - 1.0;
                let a = Vec3::new(r(), r(), r()) * 0.2 - 0.1;
                let b = Vec3::new(r(), r(), r()) * 0.2 - 0.1;
                [c, c + a, c + b]
            })
            .collect()
    }

    #[test]
    fn every_triangle_is_in_one_leaf_and_inside_its_boxes() {
        let tris = soup(7, 3000);
        let bvh = Bvh::build(&tris);
        let mut seen = vec![0; tris.len()];
        for t in &bvh.order {
            seen[*t as usize] += 1;
        }
        assert!(seen.iter().all(|&n| n == 1));
        let mut leaves = 0;
        for n in &bvh.nodes {
            if n.count > 0 {
                leaves += n.count;
                assert!(n.count as usize <= 2 * MAX_LEAF);
                for k in n.index..n.index + n.count {
                    for p in tris[bvh.order[k as usize] as usize] {
                        assert!(p.cmpge(Vec3::from(n.lo) - 1e-6).all());
                        assert!(p.cmple(Vec3::from(n.hi) + 1e-6).all());
                    }
                }
            }
        }
        assert_eq!(leaves as usize, tris.len());
    }

    #[test]
    fn walking_the_tree_finds_the_closest_hit() {
        let tris = soup(11, 2000);
        let bvh = Bvh::build(&tris);
        let rays = soup(3, 300);
        for r in &rays {
            let o = r[0] * 3.0;
            let d = (r[1] * 0.3 - o).normalize();
            let brute = tris
                .iter()
                .enumerate()
                .filter_map(|(i, t)| hit(t, o, d).map(|h| (h, i as u32)))
                .min_by(|a, b| a.0.total_cmp(&b.0));
            assert_eq!(walk(&bvh, &tris, o, d), brute);
        }
    }

    #[test]
    fn degenerate_input_builds() {
        assert!(Bvh::build(&[]).nodes.is_empty());
        let p = Vec3::ONE;
        let same = vec![[p, p, p]; 50];
        let bvh = Bvh::build(&same);
        assert_eq!(bvh.order.len(), 50);
        assert!(bvh.nodes.iter().all(|n| n.count as usize <= 50));
    }
}
