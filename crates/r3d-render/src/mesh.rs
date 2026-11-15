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

    /// An animated flared skirt of `around × down` vertices, standing in for
    /// simulation output until M4 (10 000 vertices with 100 × 100).
    pub fn demo_skirt(around: u32, down: u32, time: f32) -> Self {
        let around = around.max(3);
        let down = down.max(2);
        let mut positions = Vec::with_capacity((around * down) as usize);
        for j in 0..down {
            let v = j as f32 / (down - 1) as f32; // 0 at the waist, 1 at the hem
            let y = 1.0 - 0.6 * v;
            for i in 0..around {
                let theta = i as f32 / around as f32 * std::f32::consts::TAU;
                let folds = (theta * 9.0 + time * 1.3).sin() * 0.035 * v * v;
                let sway = (time * 0.9 + theta).sin() * 0.01 * v;
                let r = 0.16 + 0.2 * v.powf(1.3) + folds;
                positions.push(Vec3::new(r * theta.cos() + sway, y, r * theta.sin()));
            }
        }
        let mut indices = Vec::with_capacity((around * (down - 1) * 6) as usize);
        for j in 0..down - 1 {
            for i in 0..around {
                let a = j * around + i;
                let b = j * around + (i + 1) % around;
                let c = a + around;
                let d = b + around;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        let mut mesh = Self {
            positions,
            normals: Vec::new(),
            indices,
        };
        mesh.recompute_normals();
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_skirt_has_ten_thousand_vertices_and_unit_normals() {
        let m = Mesh::demo_skirt(100, 100, 0.5);
        assert_eq!(m.positions.len(), 10_000);
        assert_eq!(m.indices.len(), 100 * 99 * 6);
        assert!(m.normals.iter().all(|n| (n.length() - 1.0).abs() < 1e-4));
    }
}
