//! Collisions with a static triangle mesh (the avatar), using a uniform
//! grid of triangles for fast nearest-surface queries.

use std::collections::HashMap;

use glam::Vec3;

/// A static triangle mesh prepared for proximity queries.
#[derive(Debug, Clone)]
pub struct StaticMesh {
    positions: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    normals: Vec<Vec3>,
    cell: f32,
    grid: HashMap<[i32; 3], Vec<u32>>,
}

/// The closest point of the mesh to a query point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub point: Vec3,
    /// Outward face normal at the closest triangle.
    pub normal: Vec3,
}

impl StaticMesh {
    /// `cell` should be a few times the query radius.
    pub fn new(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>, cell: f32) -> Self {
        let cell = cell.max(1e-3);
        let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        let mut normals = Vec::with_capacity(triangles.len());
        for (i, t) in triangles.iter().enumerate() {
            let [a, b, c] = t.map(|v| positions.get(v as usize).copied().unwrap_or_default());
            normals.push((b - a).cross(c - a).normalize_or_zero());
            let lo = a.min(b).min(c);
            let hi = a.max(b).max(c);
            let (l, h) = (key(lo, cell), key(hi, cell));
            for x in l[0]..=h[0] {
                for y in l[1]..=h[1] {
                    for z in l[2]..=h[2] {
                        grid.entry([x, y, z]).or_default().push(i as u32);
                    }
                }
            }
        }
        Self {
            positions,
            triangles,
            normals,
            cell,
            grid,
        }
    }

    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }

    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    /// The closest surface point within `radius` of `p`, if any.
    pub fn closest(&self, p: Vec3, radius: f32) -> Option<Contact> {
        let (l, h) = (
            key(p - Vec3::splat(radius), self.cell),
            key(p + Vec3::splat(radius), self.cell),
        );
        let mut best: Option<(f32, Contact)> = None;
        for x in l[0]..=h[0] {
            for y in l[1]..=h[1] {
                for z in l[2]..=h[2] {
                    let Some(list) = self.grid.get(&[x, y, z]) else {
                        continue;
                    };
                    for &t in list {
                        let [a, b, c] = self.triangles[t as usize]
                            .map(|v| self.positions.get(v as usize).copied().unwrap_or_default());
                        let q = closest_on_triangle(p, a, b, c);
                        let d = q.distance_squared(p);
                        if d <= radius * radius && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                            best = Some((
                                d,
                                Contact {
                                    point: q,
                                    normal: self.normals[t as usize],
                                },
                            ));
                        }
                    }
                }
            }
        }
        best.map(|(_, c)| c)
    }
}

fn key(p: Vec3, cell: f32) -> [i32; 3] {
    [
        (p.x / cell).floor() as i32,
        (p.y / cell).floor() as i32,
        (p.z / cell).floor() as i32,
    ]
}

/// Closest point on triangle `abc` to `p` (Ericson, Real-Time Collision
/// Detection, §5.1.5).
pub fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}
