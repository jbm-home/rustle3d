//! Triangle meshes drawn by the 3D view.

use glam::Vec3;

use crate::theme::Rgba;

/// A triangle mesh in metres: positions, per-vertex normals and triangles
/// (cloth from the simulation, the avatar body, …).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    /// Per-vertex colours (sRGB) drawn instead of the material's base
    /// colour, e.g. a fit map; empty for the material's.
    pub colors: Vec<Rgba>,
}

impl Mesh {
    /// Recompute area-weighted vertex normals from the triangles.
    pub fn recompute_normals(&mut self) {
        self.normals.clear();
        self.normals.resize(self.positions.len(), Vec3::ZERO);
        let (triangles, _) = self.indices.as_chunks::<3>();
        for t in triangles {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            let (Some(pa), Some(pb), Some(pc)) = (
                self.positions.get(a),
                self.positions.get(b),
                self.positions.get(c),
            ) else {
                continue;
            };
            let n = (*pb - *pa).cross(*pc - *pa);
            for i in [a, b, c] {
                self.normals[i] += n;
            }
        }
        for n in &mut self.normals {
            *n = n.normalize_or(Vec3::Y);
        }
    }

    /// A thin strip along a line drawn on a surface: `points` are the
    /// line's positions with the surface normal there. The strip is
    /// `width` wide and lifted by `lift` off the surface, so it shows on
    /// top of it (style lines and selected edges on the garment).
    pub fn ribbon(points: &[(Vec3, Vec3)], width: f32, lift: f32) -> Self {
        let mut mesh = Mesh::default();
        let n = points.len();
        if n < 2 {
            return mesh;
        }
        for (i, (p, normal)) in points.iter().enumerate() {
            let prev = points[i.saturating_sub(1)].0;
            let next = points[(i + 1).min(n - 1)].0;
            let side = (next - prev).cross(*normal).normalize_or_zero() * (width / 2.0);
            let base = *p + *normal * lift;
            mesh.positions.extend([base - side, base + side]);
            mesh.normals.extend([*normal, *normal]);
        }
        for i in 0..n as u32 - 1 {
            let k = 2 * i;
            mesh.indices.extend([k, k + 1, k + 3, k, k + 3, k + 2]);
        }
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normals_are_unit_and_face_the_winding() {
        let mut quad = Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 0.0, -1.0), -Vec3::Z],
            normals: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
            colors: Vec::new(),
        };
        quad.recompute_normals();
        assert!(quad.normals.iter().all(|n| (*n - Vec3::Y).length() < 1e-6));
    }

    #[test]
    fn a_ribbon_lies_on_the_surface_across_the_line() {
        let line = [
            (Vec3::ZERO, Vec3::Y),
            (Vec3::X, Vec3::Y),
            (Vec3::new(2.0, 0.0, 0.0), Vec3::Y),
        ];
        let r = Mesh::ribbon(&line, 0.2, 0.01);
        assert_eq!(r.positions.len(), 6);
        assert_eq!(r.indices.len(), 12);
        assert!(
            r.positions
                .iter()
                .all(|p| (p.y - 0.01).abs() < 1e-6 && (p.z.abs() - 0.1).abs() < 1e-6)
        );
        assert!(Mesh::ribbon(&line[..1], 0.2, 0.0).indices.is_empty());
    }
}
