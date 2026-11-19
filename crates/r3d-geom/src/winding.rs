//! Generalized winding numbers of a triangle mesh: how many times the
//! surface wraps around a point, ≈ 1 inside and ≈ 0 outside, and still
//! meaningful when the mesh has holes or non-manifold edges (Jacobson,
//! Kavan & Sorkine-Hornung, "Robust inside-outside segmentation using
//! generalized winding numbers", SIGGRAPH 2013).
//!
//! The winding number is the sum of the triangles' signed solid angles
//! over 4π (the solid angle after Van Oosterom & Strackee, "The solid
//! angle of a plane triangle", IEEE TBME 1983). [`Winding`] evaluates it
//! through a bounding-volume hierarchy as Barill, Dickson, Schmidt, Levin
//! & Jacobson do ("Fast winding numbers for soups and clouds", SIGGRAPH
//! 2018): a cluster far from the point counts as one dipole, its
//! area-weighted normal at its centre; near clusters are summed exactly.

use glam::{DVec3, Vec3};

/// Triangles per leaf of the hierarchy.
const LEAF: usize = 8;
/// A cluster counts as a dipole when the point is farther from its centre
/// than this many times its radius (Barill et al. use 2 with higher-order terms; the
/// dipole alone is then within a few per cent).
const FAR: f64 = 3.0;

/// A node of the hierarchy.
#[derive(Debug, Clone)]
struct Node {
    /// Area-weighted centre of its triangles.
    centre: DVec3,
    /// Largest distance from `centre` to a corner of its triangles.
    radius: f64,
    /// Sum of its triangles' area-weighted normals (½ of the cross
    /// products).
    normal: DVec3,
    /// Its triangles (`order[first..first + count]`), for a leaf.
    first: usize,
    count: usize,
    /// Its children, for an inner node.
    children: Option<(usize, usize)>,
}

/// A mesh prepared for winding-number queries.
#[derive(Debug, Clone)]
pub struct Winding {
    triangles: Vec<[DVec3; 3]>,
    order: Vec<usize>,
    nodes: Vec<Node>,
}

/// The signed solid angle of triangle `t` seen from `q`.
fn solid_angle(q: DVec3, t: &[DVec3; 3]) -> f64 {
    let [a, b, c] = t.map(|p| p - q);
    let (la, lb, lc) = (a.length(), b.length(), c.length());
    let det = a.dot(b.cross(c));
    let div = la * lb * lc + a.dot(b) * lc + a.dot(c) * lb + b.dot(c) * la;
    2.0 * det.atan2(div)
}

impl Winding {
    /// Prepare `triangles` (corners indexing `positions`; invalid ones are
    /// skipped).
    pub fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> Self {
        let triangles: Vec<[DVec3; 3]> = triangles
            .iter()
            .filter_map(|t| {
                let [a, b, c] = t.map(|v| positions.get(v as usize).map(|p| p.as_dvec3()));
                Some([a?, b?, c?])
            })
            .collect();
        let mut out = Self {
            order: (0..triangles.len()).collect(),
            triangles,
            nodes: Vec::new(),
        };
        if !out.triangles.is_empty() {
            out.build(0, out.triangles.len());
        }
        out
    }

    /// Build the node over `order[first..first + count]`; returns its index.
    fn build(&mut self, first: usize, count: usize) -> usize {
        let range = first..first + count;
        let (mut area_sum, mut centre, mut normal) = (0.0, DVec3::ZERO, DVec3::ZERO);
        let (mut lo, mut hi) = (DVec3::MAX, DVec3::MIN);
        for &i in self.order.get(range.clone()).unwrap_or(&[]) {
            let Some(t) = self.triangles.get(i) else {
                continue;
            };
            let n = (t[1] - t[0]).cross(t[2] - t[0]) * 0.5;
            let area = n.length();
            let mid = (t[0] + t[1] + t[2]) / 3.0;
            area_sum += area;
            centre += mid * area;
            normal += n;
            lo = lo.min(mid);
            hi = hi.max(mid);
        }
        let centre = if area_sum > 0.0 {
            centre / area_sum
        } else {
            (lo + hi) / 2.0
        };
        let radius = self
            .order
            .get(range.clone())
            .unwrap_or(&[])
            .iter()
            .filter_map(|i| self.triangles.get(*i))
            .flatten()
            .map(|p| p.distance(centre))
            .fold(0.0, f64::max);
        let index = self.nodes.len();
        self.nodes.push(Node {
            centre,
            radius,
            normal,
            first,
            count,
            children: None,
        });
        if count > LEAF {
            // Split at the median along the longest side of the centres'
            // box.
            let extent = hi - lo;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            let tris = &self.triangles;
            let mid_of = |i: &usize| {
                tris.get(*i)
                    .map_or(0.0, |t| ((t[0] + t[1] + t[2]) / 3.0)[axis])
            };
            if let Some(slice) = self.order.get_mut(range) {
                slice.sort_by(|a, b| mid_of(a).total_cmp(&mid_of(b)));
            }
            let half = count / 2;
            let left = self.build(first, half);
            let right = self.build(first + half, count - half);
            if let Some(node) = self.nodes.get_mut(index) {
                node.children = Some((left, right));
            }
        }
        index
    }

    /// The winding number at `p`: ≈ 1 inside a closed outward-facing
    /// surface, ≈ 0 outside, in between near holes.
    pub fn at(&self, p: Vec3) -> f32 {
        let q = p.as_dvec3();
        let mut sum = 0.0;
        let mut stack = if self.nodes.is_empty() {
            vec![]
        } else {
            vec![0]
        };
        while let Some(i) = stack.pop() {
            let Some(node) = self.nodes.get(i) else {
                continue;
            };
            let d = node.centre - q;
            let dist = d.length();
            if dist > FAR * node.radius && dist > 0.0 {
                sum += d.dot(node.normal) / (dist * dist * dist);
                continue;
            }
            match node.children {
                Some((l, r)) => {
                    stack.push(l);
                    stack.push(r);
                }
                None => {
                    for t in self
                        .order
                        .get(node.first..node.first + node.count)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(|t| self.triangles.get(*t))
                    {
                        sum += solid_angle(q, t);
                    }
                }
            }
        }
        (sum / (4.0 * std::f64::consts::PI)) as f32
    }

    /// Whether `p` is inside: a winding number above ½.
    pub fn inside(&self, p: Vec3) -> bool {
        self.at(p) > 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UV sphere of radius 1, outward-facing, `n` bands.
    pub(super) fn sphere(n: u32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let mut p = Vec::new();
        for i in 0..=n {
            let theta = std::f32::consts::PI * i as f32 / n as f32;
            for j in 0..2 * n {
                let phi = std::f32::consts::PI * j as f32 / n as f32;
                p.push(Vec3::new(
                    theta.sin() * phi.cos(),
                    theta.cos(),
                    theta.sin() * phi.sin(),
                ));
            }
        }
        let w = 2 * n;
        let mut t = Vec::new();
        for i in 0..n {
            for j in 0..w {
                let (a, b) = (i * w + j, i * w + (j + 1) % w);
                let (c, d) = (a + w, b + w);
                t.push([a, b, c]);
                t.push([b, d, c]);
            }
        }
        (p, t)
    }

    #[test]
    fn inside_and_outside_a_sphere_even_with_a_hole() {
        let (p, t) = sphere(24);
        let w = Winding::new(&p, &t);
        // The mesh's handedness: inside reads +1 when it faces outward.
        let sign = w.at(Vec3::ZERO).signum();
        assert!((w.at(Vec3::ZERO).abs() - 1.0).abs() < 0.03);
        assert!(w.at(Vec3::new(0.0, 0.5, 0.3)) * sign > 0.95);
        assert!(w.at(Vec3::new(3.0, 0.0, 0.0)).abs() < 0.03);
        assert!(w.at(Vec3::new(1.05, 0.0, 0.0)).abs() < 0.1);
        // Cut a cap off the top: points just under it still read inside.
        let open: Vec<[u32; 3]> = t.iter().skip(2 * 48).copied().collect();
        let w = Winding::new(&p, &open);
        assert!(w.at(Vec3::new(0.0, 0.8, 0.0)) * sign > 0.5);
        assert!(w.at(Vec3::new(0.0, 0.0, 0.0)) * sign > 0.9);
        assert!(w.at(Vec3::new(0.0, 1.3, 0.0)) * sign < 0.5);
    }

    #[test]
    fn the_hierarchy_matches_the_exact_sum() {
        let (p, t) = sphere(16);
        let w = Winding::new(&p, &t);
        for q in [
            Vec3::new(0.2, 0.9, 0.1),
            Vec3::new(1.4, 0.3, -0.2),
            Vec3::splat(0.5),
        ] {
            let exact: f64 = w
                .triangles
                .iter()
                .map(|tri| solid_angle(q.as_dvec3(), tri))
                .sum::<f64>()
                / (4.0 * std::f64::consts::PI);
            assert!((f64::from(w.at(q)) - exact).abs() < 0.03, "{q}");
        }
    }
}
