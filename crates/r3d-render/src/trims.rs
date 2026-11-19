//! Trims on the draped garment in the 3D view (spec 26): buttons as small
//! discs on the cloth, zippers as a band along their closed stretch.

use glam::Vec3;

use crate::mesh::Mesh;
use crate::theme::{Rgba, hex};
use crate::view3d::Material;

/// A button's colour: dark horn, readable on light and dark fabrics.
const BUTTON: Rgba = hex(0x2E2925);
/// A zipper's teeth: brass.
const ZIPPER: Rgba = hex(0xB8862B);
/// Discs and bands sit this far off the cloth (m), as style lines do.
const LIFT_M: f32 = 0.002;
/// A zipper's band is this wide (m): both rows of teeth.
const ZIPPER_WIDTH_M: f32 = 0.008;

fn flat(base: Rgba) -> Material {
    Material {
        base,
        sheen: base,
        sheen_amount: 0.0,
    }
}

/// Meshes and materials for `buttons` (centre on the cloth, the cloth's
/// normal there, radius; m) and the closed stretches of `zippers` (points
/// on the cloth with its normal).
pub fn meshes(
    buttons: &[(Vec3, Vec3, f32)],
    zippers: &[Vec<(Vec3, Vec3)>],
) -> Vec<(Mesh, Material)> {
    let mut out: Vec<(Mesh, Material)> = buttons
        .iter()
        .map(|(c, n, r)| (disc(*c, *n, *r), flat(BUTTON)))
        .collect();
    out.extend(
        zippers
            .iter()
            .filter(|z| z.len() > 1)
            .map(|z| (Mesh::ribbon(z, ZIPPER_WIDTH_M, LIFT_M), flat(ZIPPER))),
    );
    out
}

/// A disc of `radius` lying on the surface at `centre` facing `normal`,
/// lifted off it.
fn disc(centre: Vec3, normal: Vec3, radius: f32) -> Mesh {
    let n = normal.normalize_or(Vec3::Y);
    let (u, v) = n.any_orthonormal_pair();
    let middle = centre + n * LIFT_M;
    let segments = 16u32;
    let mut mesh = Mesh::default();
    mesh.positions.push(middle);
    mesh.normals.push(n);
    for k in 0..segments {
        let a = k as f32 / segments as f32 * std::f32::consts::TAU;
        mesh.positions
            .push(middle + (u * a.cos() + v * a.sin()) * radius);
        mesh.normals.push(n);
    }
    for k in 0..segments {
        mesh.indices.extend([0, 1 + k, 1 + (k + 1) % segments]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disc_lies_on_the_cloth_facing_out() {
        let m = disc(Vec3::ZERO, Vec3::Z, 0.01);
        assert!(m.positions.iter().all(|p| (p.z - LIFT_M).abs() < 1e-6));
        // Counter-clockwise seen from its normal.
        let [a, b, c] = [0, 1, 2].map(|i| m.positions[m.indices[i] as usize]);
        assert!((b - a).cross(c - a).dot(Vec3::Z) > 0.0);
    }
}
