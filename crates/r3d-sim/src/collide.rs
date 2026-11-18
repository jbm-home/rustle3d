//! Collisions with a static closed triangle mesh (the avatar) through a
//! narrow-band signed distance field (docs/physics.md §5).
//!
//! The field is sampled on grid nodes `VOXEL` apart, only within `BAND` of
//! the surface, stored in 8×8×8 bricks so that empty space costs nothing.
//! The sign comes from angle-weighted pseudo-normals (Bærentzen & Aanæs,
//! "Signed distance computation using the angle weighted pseudonormal",
//! IEEE TVCG 2005), which is exact for closed manifold meshes such as the
//! avatar. A query is a trilinear interpolation: constant time.
//!
//! A body that moves (an animated avatar) keeps the field of its rest
//! pose: each node also records the body vertex nearest to its closest
//! surface point, whose skin weights let [`crate::motion`] follow the
//! skin as the bones move it.

use std::collections::HashMap;

use glam::Vec3;
use rayon::prelude::*;

/// Nodes per brick side.
pub const BRICK: usize = 8;
const BRICK_NODES: usize = BRICK * BRICK * BRICK;
/// Distance from the surface (m) within which the field is computed. A
/// point deeper inside than this is not seen as inside.
pub const BAND: f32 = 0.02;

/// A static closed mesh prepared for distance queries.
#[derive(Debug, Clone)]
pub struct StaticMesh {
    origin: Vec3,
    voxel: f32,
    /// Bricks along x, y, z.
    dims: [usize; 3],
    /// For each brick, its slot in `values`, or `u32::MAX` when far from
    /// the surface.
    slots: Vec<u32>,
    values: Vec<f32>,
    /// Per node (as `values`), the vertex nearest to the closest surface
    /// point (`u32::MAX`: none); empty for a body without a skin.
    nodes: Vec<u32>,
    /// Per vertex, how it follows the bones.
    skin: Vec<Skin>,
    /// Per bone, the box (rest pose) holding the nodes whose vertex it
    /// leads ([`Skin::leading`]).
    part_boxes: Vec<(Vec3, Vec3)>,
}

/// How a vertex of a moving body follows its bones (parts): up to four,
/// weights summing to 1 (linear blend skinning; unused slots weigh 0).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Skin {
    pub bones: [u8; 4],
    pub weights: [f32; 4],
}

/// A bone leads a vertex when it weighs at least this share of the
/// vertex's heaviest bone.
const LEADING_SHARE: f32 = 0.5;

impl Skin {
    /// The bones that lead this vertex, one bit each.
    pub fn leading(&self) -> u32 {
        let heaviest = self.weights.iter().copied().fold(0.0, f32::max);
        self.bones
            .iter()
            .zip(self.weights)
            .filter(|(b, w)| *w > 0.0 && *w >= LEADING_SHARE * heaviest && usize::from(**b) < 32)
            .fold(0, |m, (b, _)| m | 1u32 << b)
    }
}

/// The field's storage, for backends that sample it themselves (a node
/// `(x, y, z)` is in brick `(x, y, z) / BRICK`, at `local` within it, as
/// in [`StaticMesh::sample`]).
#[derive(Debug, Clone, Copy)]
pub struct Grid<'a> {
    /// Position of node (0, 0, 0) (m).
    pub origin: Vec3,
    /// Node spacing (m).
    pub voxel: f32,
    /// Bricks along x, y, z.
    pub dims: [usize; 3],
    /// For each brick (x fastest), its slot in `values`, or `u32::MAX`
    /// when far from the surface.
    pub slots: &'a [u32],
    /// `BRICK³` values per slot (x fastest); [`BAND`] far from the surface.
    pub values: &'a [f32],
    /// Per value, the vertex nearest to the closest surface point
    /// (`u32::MAX`: none); empty for a body without a skin.
    pub nodes: &'a [u32],
    /// Per vertex, how it follows the bones.
    pub skin: &'a [Skin],
}

/// The surface seen from a query point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Signed distance to the surface (m): negative inside.
    pub distance: f32,
    /// Outward surface direction (unit).
    pub normal: Vec3,
}

/// Which part of a triangle a closest point lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Vertex(usize),
    /// Edge from corner `i` to corner `(i + 1) % 3`.
    Edge(usize),
    Face,
}

/// Outward normals used for the sign: per face, per edge, per vertex.
struct PseudoNormals {
    face: Vec<Vec3>,
    edge: HashMap<(u32, u32), Vec3>,
    vertex: Vec<Vec3>,
}

impl PseudoNormals {
    fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> Self {
        let mut face = Vec::with_capacity(triangles.len());
        let mut edge: HashMap<(u32, u32), Vec3> = HashMap::new();
        let mut vertex = vec![Vec3::ZERO; positions.len()];
        for t in triangles {
            let p = t.map(|v| positions[v as usize]);
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            face.push(n);
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *edge.entry((a.min(b), a.max(b))).or_default() += n;
                let (u, w) = (p[(k + 1) % 3] - p[k], p[(k + 2) % 3] - p[k]);
                vertex[t[k] as usize] += n * u.angle_between(w);
            }
        }
        Self { face, edge, vertex }
    }

    fn at(&self, triangle: usize, t: &[u32; 3], feature: Feature) -> Vec3 {
        match feature {
            Feature::Face => self.face[triangle],
            Feature::Edge(k) => {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                self.edge
                    .get(&(a.min(b), a.max(b)))
                    .copied()
                    .unwrap_or_default()
            }
            Feature::Vertex(k) => self.vertex[t[k] as usize],
        }
    }
}

impl StaticMesh {
    /// Build the field of a closed mesh with nodes `voxel` metres apart.
    /// Triangles with out-of-range corners are ignored.
    pub fn new(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>, voxel: f32) -> Self {
        Self::with_skin(positions, triangles, voxel, Vec::new())
    }

    /// The same, for a body whose bones move it: `skin` holds each
    /// vertex's (bones below 32); empty: a body that stands still.
    pub fn with_skin(
        positions: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
        voxel: f32,
        skin: Vec<Skin>,
    ) -> Self {
        let voxel = voxel.max(1e-3);
        let n = positions.len();
        let triangles: Vec<[u32; 3]> = triangles
            .into_iter()
            .filter(|t| t.iter().all(|v| (*v as usize) < n))
            .collect();
        let normals = PseudoNormals::new(&positions, &triangles);
        let (lo, hi) = positions
            .iter()
            .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            });
        let pad = Vec3::splat(BAND + voxel);
        let origin = if n == 0 { Vec3::ZERO } else { lo - pad };
        let brick = voxel * BRICK as f32;
        let size = if n == 0 {
            Vec3::ZERO
        } else {
            hi + pad - origin
        };
        let dims = [size.x, size.y, size.z].map(|s| ((s / brick).ceil() as usize).max(1));
        let brick_id = |b: [usize; 3]| (b[2] * dims[1] + b[1]) * dims[0] + b[0];
        let to_brick = |p: Vec3, round_up: bool| -> [usize; 3] {
            let q = (p - origin) / brick;
            [q.x, q.y, q.z]
                .map(|v| if round_up { v.ceil() } else { v.floor() })
                .map(|v| v.max(0.0) as usize)
        };

        // Triangles near each brick.
        let mut near: Vec<Vec<u32>> = vec![Vec::new(); dims[0] * dims[1] * dims[2]];
        for (i, t) in triangles.iter().enumerate() {
            let p = t.map(|v| positions[v as usize]);
            let (a, b) = (
                to_brick(p[0].min(p[1]).min(p[2]) - pad, false),
                to_brick(p[0].max(p[1]).max(p[2]) + pad, true),
            );
            for z in a[2]..b[2].min(dims[2]) {
                for y in a[1]..b[1].min(dims[1]) {
                    for x in a[0]..b[0].min(dims[0]) {
                        near[brick_id([x, y, z])].push(i as u32);
                    }
                }
            }
        }
        let mut slots = vec![u32::MAX; near.len()];
        let mut filled = Vec::new();
        for (id, list) in near.iter().enumerate() {
            if !list.is_empty() {
                slots[id] = filled.len() as u32;
                filled.push(id);
            }
        }

        // Each brick's nodes, from the triangles near it (in parallel; each
        // brick writes only its own values).
        let skinned = !skin.is_empty() && skin.len() == n;
        let mut nodes = vec![(BAND, u32::MAX); filled.len() * BRICK_NODES];
        nodes
            .par_chunks_mut(BRICK_NODES)
            .zip(&filled)
            .for_each(|(out, id)| {
                let b = [
                    id % dims[0],
                    (id / dims[0]) % dims[1],
                    id / (dims[0] * dims[1]),
                ];
                for (k, (value, vertex)) in out.iter_mut().enumerate() {
                    let local = [k % BRICK, (k / BRICK) % BRICK, k / (BRICK * BRICK)];
                    let node = Vec3::new(
                        (b[0] * BRICK + local[0]) as f32,
                        (b[1] * BRICK + local[1]) as f32,
                        (b[2] * BRICK + local[2]) as f32,
                    );
                    let p = origin + node * voxel;
                    let mut best = (BAND * BAND, None);
                    for &ti in &near[*id] {
                        let t = &triangles[ti as usize];
                        let [a, b, c] = t.map(|v| positions[v as usize]);
                        let (q, feature) = closest_feature(p, a, b, c);
                        let d2 = q.distance_squared(p);
                        if d2 < best.0 {
                            best = (d2, Some((q, ti as usize, feature)));
                        }
                    }
                    if let (d2, Some((q, ti, feature))) = best {
                        let pn = normals.at(ti, &triangles[ti], feature);
                        let sign = if (p - q).dot(pn) < 0.0 { -1.0 } else { 1.0 };
                        *value = sign * d2.sqrt();
                        if skinned {
                            // The corner of the closest triangle nearest
                            // to the node.
                            *vertex = triangles[ti]
                                .iter()
                                .copied()
                                .min_by(|a, b| {
                                    let da = positions[*a as usize].distance_squared(p);
                                    let db = positions[*b as usize].distance_squared(p);
                                    da.total_cmp(&db)
                                })
                                .unwrap_or(u32::MAX);
                        }
                    }
                }
            });
        let (values, nodes): (Vec<f32>, Vec<u32>) = nodes.into_iter().unzip();
        let part_boxes = if skinned {
            boxes(&values, &nodes, &skin, &filled, dims, origin, voxel)
        } else {
            Vec::new()
        };
        Self {
            origin,
            voxel,
            dims,
            slots,
            values,
            nodes: if skinned { nodes } else { Vec::new() },
            skin: if skinned { skin } else { Vec::new() },
            part_boxes,
        }
    }

    /// Per bone, the box (rest pose) around the field's nodes near the
    /// surface it leads; a point outside it is not near that bone's skin.
    /// Empty for a body without a skin.
    pub fn part_boxes(&self) -> &[(Vec3, Vec3)] {
        &self.part_boxes
    }

    pub fn grid(&self) -> Grid<'_> {
        Grid {
            origin: self.origin,
            voxel: self.voxel,
            dims: self.dims,
            slots: &self.slots,
            values: &self.values,
            nodes: &self.nodes,
            skin: &self.skin,
        }
    }

    /// Index in `values` of grid node `(x, y, z)`, if its brick is near
    /// the surface.
    fn index(&self, x: usize, y: usize, z: usize) -> Option<usize> {
        let b = [x / BRICK, y / BRICK, z / BRICK];
        if b[0] >= self.dims[0] || b[1] >= self.dims[1] || b[2] >= self.dims[2] {
            return None;
        }
        let id = (b[2] * self.dims[1] + b[1]) * self.dims[0] + b[0];
        let slot = *self.slots.get(id)?;
        if slot == u32::MAX {
            return None;
        }
        let local = ((z % BRICK) * BRICK + y % BRICK) * BRICK + x % BRICK;
        Some(slot as usize * BRICK_NODES + local)
    }

    /// Field value at grid node `(x, y, z)`; `BAND` far from the surface.
    fn node(&self, x: usize, y: usize, z: usize) -> f32 {
        self.index(x, y, z)
            .and_then(|i| self.values.get(i))
            .copied()
            .unwrap_or(BAND)
    }

    /// How the skin closest to `p` follows the bones: the weights of the
    /// vertex recorded at the nearest node. `None` far from the surface or
    /// for a body without a skin.
    pub fn skin_at(&self, p: Vec3) -> Option<&Skin> {
        let q = ((p - self.origin) / self.voxel).round();
        if q.min_element() < 0.0 {
            return None;
        }
        let i = self.index(q.x as usize, q.y as usize, q.z as usize)?;
        self.skin.get(*self.nodes.get(i)? as usize)
    }

    /// Signed distance and outward direction at `p`, when `p` is within
    /// the band around the surface.
    pub fn sample(&self, p: Vec3) -> Option<Contact> {
        let q = (p - self.origin) / self.voxel;
        if q.min_element() < 0.0 {
            return None;
        }
        let (x, y, z) = (q.x as usize, q.y as usize, q.z as usize);
        let f = q - Vec3::new(x as f32, y as f32, z as f32);
        let c = |dx: usize, dy: usize, dz: usize| self.node(x + dx, y + dy, z + dz);
        let corners = [
            c(0, 0, 0),
            c(1, 0, 0),
            c(0, 1, 0),
            c(1, 1, 0),
            c(0, 0, 1),
            c(1, 0, 1),
            c(0, 1, 1),
            c(1, 1, 1),
        ];
        if corners.iter().all(|v| *v >= BAND) {
            return None;
        }
        let [c000, c100, c010, c110, c001, c101, c011, c111] = corners;
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        // Along x, then y, then z; the gradient from the same factors.
        let (x00, x10) = (lerp(c000, c100, f.x), lerp(c010, c110, f.x));
        let (x01, x11) = (lerp(c001, c101, f.x), lerp(c011, c111, f.x));
        let (y0, y1) = (lerp(x00, x10, f.y), lerp(x01, x11, f.y));
        let distance = lerp(y0, y1, f.z);
        let dx = lerp(
            lerp(c100 - c000, c110 - c010, f.y),
            lerp(c101 - c001, c111 - c011, f.y),
            f.z,
        );
        let dy = lerp(x10 - x00, x11 - x01, f.z);
        let dz = y1 - y0;
        let normal = Vec3::new(dx, dy, dz).try_normalize()?;
        Some(Contact { distance, normal })
    }
}

/// Per bone, the box around the nodes within the band whose vertex it
/// leads, one node spacing wider.
fn boxes(
    values: &[f32],
    nodes: &[u32],
    skin: &[Skin],
    filled: &[usize],
    dims: [usize; 3],
    origin: Vec3,
    voxel: f32,
) -> Vec<(Vec3, Vec3)> {
    let lead = |node: &u32| skin.get(*node as usize).map_or(0, Skin::leading);
    let count = nodes
        .iter()
        .map(|v| (u32::BITS - lead(v).leading_zeros()) as usize)
        .max()
        .unwrap_or(0);
    let mut out = vec![(Vec3::MAX, Vec3::MIN); count];
    for (slot, id) in filled.iter().enumerate() {
        let b = [
            id % dims[0],
            (id / dims[0]) % dims[1],
            id / (dims[0] * dims[1]),
        ];
        for k in 0..BRICK_NODES {
            let i = slot * BRICK_NODES + k;
            let (Some(v), Some(m)) = (values.get(i), nodes.get(i).map(lead)) else {
                continue;
            };
            if *v >= BAND || m == 0 {
                continue;
            }
            let local = [k % BRICK, (k / BRICK) % BRICK, k / (BRICK * BRICK)];
            let p = origin
                + Vec3::new(
                    (b[0] * BRICK + local[0]) as f32,
                    (b[1] * BRICK + local[1]) as f32,
                    (b[2] * BRICK + local[2]) as f32,
                ) * voxel;
            for (part, (lo, hi)) in out.iter_mut().enumerate() {
                if m & (1 << part) != 0 {
                    *lo = lo.min(p - voxel);
                    *hi = hi.max(p + voxel);
                }
            }
        }
    }
    out
}

/// Closest point on triangle `abc` to `p`, and the feature it lies on
/// (Ericson, Real-Time Collision Detection, §5.1.5).
pub fn closest_feature(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, Feature) {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, Feature::Vertex(0));
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (b, Feature::Vertex(1));
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (a + ab * (d1 / (d1 - d3)), Feature::Edge(0));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (c, Feature::Vertex(2));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (a + ac * (d2 / (d2 - d6)), Feature::Edge(2));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * t, Feature::Edge(1));
    }
    let denom = 1.0 / (va + vb + vc);
    (a + ab * (vb * denom) + ac * (vc * denom), Feature::Face)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// A closed cube of side `s` centred at the origin (12 triangles,
    /// counter-clockwise seen from outside).
    fn cube(s: f32) -> StaticMesh {
        let h = s / 2.0;
        let positions = (0..8)
            .map(|i| {
                Vec3::new(
                    if i & 1 == 0 { -h } else { h },
                    if i & 2 == 0 { -h } else { h },
                    if i & 4 == 0 { -h } else { h },
                )
            })
            .collect();
        let quads = [
            [0, 2, 3, 1], // -z
            [4, 5, 7, 6], // +z
            [0, 1, 5, 4], // -y
            [2, 6, 7, 3], // +y
            [0, 4, 6, 2], // -x
            [1, 3, 7, 5], // +x
        ];
        let triangles = quads
            .iter()
            .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
            .collect();
        StaticMesh::new(positions, triangles, 0.005)
    }

    #[test]
    fn distances_and_normals_near_a_cube() {
        let m = cube(0.2);
        let cases = [
            (Vec3::new(0.0, 0.11, 0.0), 0.01, Vec3::Y),
            (Vec3::new(0.0, 0.09, 0.0), -0.01, Vec3::Y),
            (Vec3::new(-0.105, 0.02, 0.03), 0.005, -Vec3::X),
            (Vec3::new(0.03, -0.02, 0.115), 0.015, Vec3::Z),
        ];
        for (p, d, n) in cases {
            let c = m.sample(p).unwrap();
            assert!((c.distance - d).abs() < 1e-3, "{p}: {} vs {d}", c.distance);
            assert!(c.normal.dot(n) > 0.99, "{p}: {}", c.normal);
        }
        // Far outside, and deep inside beyond the band: nothing.
        assert!(m.sample(Vec3::new(0.0, 0.3, 0.0)).is_none());
        assert!(m.sample(Vec3::ZERO).is_none());
    }

    #[test]
    fn closest_feature_regions() {
        let (a, b, c) = (Vec3::ZERO, Vec3::X, Vec3::Y);
        assert_eq!(
            closest_feature(Vec3::new(-1.0, -1.0, 0.0), a, b, c).1,
            Feature::Vertex(0)
        );
        assert_eq!(
            closest_feature(Vec3::new(0.5, -1.0, 0.0), a, b, c).1,
            Feature::Edge(0)
        );
        assert_eq!(
            closest_feature(Vec3::new(1.0, 1.0, 0.0), a, b, c).1,
            Feature::Edge(1)
        );
        assert_eq!(
            closest_feature(Vec3::new(-1.0, 0.5, 0.0), a, b, c).1,
            Feature::Edge(2)
        );
        let (q, feature) = closest_feature(Vec3::new(0.2, 0.2, 1.0), a, b, c);
        assert_eq!(feature, Feature::Face);
        assert!(q.distance(Vec3::new(0.2, 0.2, 0.0)) < 1e-6);
    }
}
