//! Triangle meshes drawn by the 3D view.

use glam::Vec3;

/// A triangle mesh in metres: positions, per-vertex normals and triangles
/// (cloth from the simulation, the avatar body, …).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
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
        };
        quad.recompute_normals();
        assert!(quad.normals.iter().all(|n| (*n - Vec3::Y).length() < 1e-6));
    }
}
