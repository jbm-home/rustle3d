//! Phong tessellation (Boubekeur and Alexa 2008, "Phong Tessellation"):
//! each triangle is cut into smaller ones lifted onto the curved surface
//! its vertex normals describe, so the coarse simulation mesh shows round
//! silhouettes and soft folds instead of facets.

use glam::Vec3;

use crate::mesh::Mesh;

/// How far points move toward the curved surface (the paper's 3/4).
const SHAPE: f32 = 0.75;

/// A triangle's corner positions, normals and texture coordinates.
pub(crate) type Corners = ([Vec3; 3], [Vec3; 3], [[f32; 2]; 3]);

/// The triangles of `mesh`, each cut into `level²` curved ones. Edges on
/// the mesh's border stay straight, so two pieces sewn along a border
/// still meet.
pub(crate) fn tessellate(mesh: &Mesh, level: u32) -> Vec<Corners> {
    let level = level.max(1);
    let (triangles, _) = mesh.indices.as_chunks::<3>();
    let border = border_edges(triangles);
    let mut out = Vec::with_capacity(triangles.len() * (level * level) as usize);
    for t in triangles {
        let (Some(p), Some(n)) = (
            corners(&mesh.positions, t),
            corners(&mesh.normals, t).map(|n| n.map(|v| v.normalize_or(Vec3::Y))),
        ) else {
            continue;
        };
        // Texture coordinates are interpolated flat (none: zeros).
        let uv = t.map(|i| mesh.uvs.get(i as usize).copied().unwrap_or_default());
        // Which of the three edges (0: corners 0-1, 1: 1-2, 2: 2-0) is a border.
        let on_border = [
            border.contains(&key(t[0], t[1])),
            border.contains(&key(t[1], t[2])),
            border.contains(&key(t[2], t[0])),
        ];
        let at = |i: u32, j: u32| -> (Vec3, Vec3, [f32; 2]) {
            // Barycentric weights of corners 0, 1, 2.
            let w1 = i as f32 / level as f32;
            let w2 = j as f32 / level as f32;
            let w0 = 1.0 - w1 - w2;
            let w = [w0, w1, w2];
            let flat = p[0] * w0 + p[1] * w1 + p[2] * w2;
            let normal = (n[0] * w0 + n[1] * w1 + n[2] * w2).normalize_or(n[0]);
            let tex = [0, 1].map(|c| uv[0][c] * w0 + uv[1][c] * w1 + uv[2][c] * w2);
            let straight = (on_border[0] && j == 0)
                || (on_border[1] && i + j == level)
                || (on_border[2] && i == 0);
            if straight {
                return (flat, normal, tex);
            }
            let mut curved = Vec3::ZERO;
            for k in 0..3 {
                let projected = flat - (flat - p[k]).dot(n[k]) * n[k];
                curved += w[k] * projected;
            }
            (flat * (1.0 - SHAPE) + curved * SHAPE, normal, tex)
        };
        for i in 0..level {
            for j in 0..level - i {
                let (a, b, c) = (at(i, j), at(i + 1, j), at(i, j + 1));
                out.push(([a.0, b.0, c.0], [a.1, b.1, c.1], [a.2, b.2, c.2]));
                if i + j + 1 < level {
                    let d = at(i + 1, j + 1);
                    out.push(([b.0, d.0, c.0], [b.1, d.1, c.1], [b.2, d.2, c.2]));
                }
            }
        }
    }
    out
}

/// `mesh` with its holes closed (each loop of border edges becomes a fan
/// around its centre), so a camera cannot look inside a closed body
/// through, say, its eye sockets.
pub(crate) fn closed(mesh: &Mesh) -> Mesh {
    let (triangles, _) = mesh.indices.as_chunks::<3>();
    let mut directed = std::collections::HashSet::new();
    for t in triangles {
        directed.extend([(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]);
    }
    // Border edges, walked the other way round: a hole's loop.
    let mut next: std::collections::BTreeMap<u32, u32> = directed
        .iter()
        .filter(|(a, b)| !directed.contains(&(*b, *a)))
        .map(|&(a, b)| (b, a))
        .collect();
    let mut out = mesh.clone();
    while let Some((&start, _)) = next.iter().next() {
        let mut ring = vec![start];
        let mut at = start;
        while let Some(n) = next.remove(&at) {
            if n == start {
                break;
            }
            ring.push(n);
            at = n;
        }
        let points: Vec<Vec3> = ring
            .iter()
            .filter_map(|&i| mesh.positions.get(i as usize).copied())
            .collect();
        if points.len() < 3 || points.len() != ring.len() {
            continue;
        }
        let centre = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        let c = out.positions.len() as u32;
        out.positions.push(centre);
        out.normals.push(Vec3::ZERO);
        for k in 0..ring.len() {
            out.indices.extend([ring[k], ring[(k + 1) % ring.len()], c]);
        }
    }
    out.recompute_normals();
    out
}

fn corners(values: &[Vec3], t: &[u32; 3]) -> Option<[Vec3; 3]> {
    Some([
        *values.get(t[0] as usize)?,
        *values.get(t[1] as usize)?,
        *values.get(t[2] as usize)?,
    ])
}

fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// Edges used by a single triangle.
fn border_edges(triangles: &[[u32; 3]]) -> std::collections::HashSet<(u32, u32)> {
    let mut count = std::collections::HashMap::new();
    for t in triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *count.entry(key(a, b)).or_insert(0u32) += 1;
        }
    }
    count
        .into_iter()
        .filter_map(|(e, n)| (n == 1).then_some(e))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An octahedron with normals pointing out of its centre.
    fn octahedron() -> Mesh {
        let positions = vec![Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
        Mesh {
            normals: positions.clone(),
            positions,
            indices: vec![
                0, 2, 4, 4, 2, 1, 1, 2, 5, 5, 2, 0, 4, 3, 0, 1, 3, 4, 5, 3, 1, 0, 3, 5,
            ],
            colors: Vec::new(),
            uvs: Vec::new(),
        }
    }

    #[test]
    fn curved_triangles_bulge_toward_the_sphere() {
        let mesh = octahedron();
        let flat = tessellate(&mesh, 1);
        assert_eq!(flat.len(), 8);
        let fine = tessellate(&mesh, 4);
        assert_eq!(fine.len(), 8 * 16);
        // Flat faces come as close as 0.577 to the centre; the curved
        // ones stay near the unit sphere their normals describe.
        let lengths: Vec<f32> = fine
            .iter()
            .flat_map(|(p, _, _)| p.iter().map(|v| v.length()))
            .collect();
        let min = lengths.iter().copied().fold(f32::MAX, f32::min);
        let max = lengths.iter().copied().fold(0.0, f32::max);
        assert!(min > 0.8 && max < 1.05, "{min} {max}");
    }

    #[test]
    fn holes_are_closed() {
        let mut open = octahedron();
        // Remove two faces: one hole of four edges.
        open.indices.truncate(18);
        let closed = closed(&open);
        let (tris, _) = closed.indices.as_chunks::<3>();
        assert!(border_edges(tris).is_empty());
        assert_eq!(closed.positions.len(), 7);
        // The fan follows the winding of its neighbours: every edge is used
        // once in each direction.
        let mut directed = std::collections::HashSet::new();
        for t in tris {
            for e in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                assert!(directed.insert(e));
            }
        }
        // A closed mesh stays as it is.
        assert_eq!(super::closed(&octahedron()).indices.len(), 24);
    }

    #[test]
    fn border_edges_stay_straight() {
        // One triangle alone: every edge is a border, so it stays flat.
        let mesh = Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            normals: vec![Vec3::new(-1.0, -1.0, 1.0).normalize(), Vec3::X, Vec3::Y],
            indices: vec![0, 1, 2],
            colors: Vec::new(),
            uvs: vec![[0.0, 0.0], [3.0, 0.0], [0.0, 3.0]],
        };
        for (p, _, uv) in tessellate(&mesh, 3) {
            for (v, t) in p.iter().zip(uv) {
                // Edge points lie on the plane z = 0, their texture
                // coordinates where the flat triangle has them.
                if v.x.abs() < 1e-6 || v.y.abs() < 1e-6 || (v.x + v.y - 1.0).abs() < 1e-6 {
                    assert!(v.z.abs() < 1e-6, "{v}");
                    assert!((t[0] - 3.0 * v.x).abs() < 1e-5 && (t[1] - 3.0 * v.y).abs() < 1e-5);
                }
            }
        }
    }
}
