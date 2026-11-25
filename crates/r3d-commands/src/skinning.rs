//! Skin weights carried from one surface to another: a garment's from
//! the body it is draped on (spec 15), an imported body's from the
//! built-in one posed as it stands (spec 27). Each vertex takes the
//! weights at the closest point of the other surface, smoothed over its
//! own mesh (closest-point transfer, as in Abdrashitov et al., *Robust
//! Skin Weights Transfer via Weight Inpainting*, SIGGRAPH Asia 2023,
//! without the inpainting).

use std::collections::HashMap;

use glam::Vec3;
use r3d_avatar::pack::Influences;

/// Laplacian smoothing passes over the transferred weights.
const SMOOTHING_PASSES: usize = 3;

/// Body triangles binned in a grid, for closest-point queries.
pub(crate) struct Nearest<'a> {
    positions: &'a [Vec3],
    triangles: &'a [[u32; 3]],
    cell: f32,
    grid: HashMap<[i32; 3], Vec<u32>>,
}

impl<'a> Nearest<'a> {
    pub(crate) fn new(positions: &'a [Vec3], triangles: &'a [[u32; 3]]) -> Self {
        let cell = 0.04;
        let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        let key = |p: Vec3| (p / cell).floor().as_ivec3().to_array();
        for (i, t) in triangles.iter().enumerate() {
            let Some(c) = corners(positions, *t) else {
                continue;
            };
            let (a, b) = (key(c[0].min(c[1]).min(c[2])), key(c[0].max(c[1]).max(c[2])));
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        grid.entry([x, y, z]).or_default().push(i as u32);
                    }
                }
            }
        }
        Self {
            positions,
            triangles,
            cell,
            grid,
        }
    }

    /// The triangle closest to `p` and the closest point's barycentric
    /// weights in it.
    pub(crate) fn find(&self, p: Vec3) -> Option<([u32; 3], Vec3)> {
        let centre = (p / self.cell).floor().as_ivec3();
        let mut best: Option<(f32, [u32; 3], Vec3)> = None;
        for ring in 0..64i32 {
            // Every cell at Chebyshev distance `ring` from the centre.
            for x in -ring..=ring {
                for y in -ring..=ring {
                    for z in -ring..=ring {
                        if x.abs().max(y.abs()).max(z.abs()) != ring {
                            continue;
                        }
                        let key = (centre + glam::IVec3::new(x, y, z)).to_array();
                        for t in self.grid.get(&key).into_iter().flatten() {
                            let Some(tri) = self.triangles.get(*t as usize) else {
                                continue;
                            };
                            let Some(c) = corners(self.positions, *tri) else {
                                continue;
                            };
                            let (q, _) = r3d_sim::collide::closest_feature(p, c[0], c[1], c[2]);
                            let d = q.distance_squared(p);
                            if best.as_ref().is_none_or(|b| d < b.0) {
                                best = Some((d, *tri, barycentric(q, c)));
                            }
                        }
                    }
                }
            }
            // Cells beyond this ring are at least `ring` cells away.
            if let Some((d, ..)) = best
                && d.sqrt() <= ring as f32 * self.cell
            {
                break;
            }
        }
        best.map(|(_, t, w)| (t, w))
    }
}

pub(crate) fn corners<T: Copy>(values: &[T], t: [u32; 3]) -> Option<[T; 3]> {
    Some([
        *values.get(t[0] as usize)?,
        *values.get(t[1] as usize)?,
        *values.get(t[2] as usize)?,
    ])
}

/// Barycentric weights of `q` (in the triangle's plane) in `c`.
fn barycentric(q: Vec3, c: [Vec3; 3]) -> Vec3 {
    let (v0, v1, v2) = (c[1] - c[0], c[2] - c[0], q - c[0]);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-20 {
        return Vec3::new(1.0, 0.0, 0.0);
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    Vec3::new(1.0 - v - w, v, w)
}

/// A surface's skin weights from a body's (`weights`, one per vertex of
/// the body `finder` searches): at each vertex those of the closest body
/// point, smoothed over the surface's edges, the four heaviest kept and
/// normalized.
pub(crate) fn transfer(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    finder: &Nearest<'_>,
    weights: &[Influences],
) -> Vec<Influences> {
    let bones = weights
        .iter()
        .flat_map(|w| w.bones)
        .map(|b| usize::from(b) + 1)
        .max()
        .unwrap_or(1);
    let mut dense: Vec<Vec<f32>> = positions
        .iter()
        .map(|p| {
            let mut out = vec![0.0f32; bones];
            if let Some((t, w)) = finder.find(*p) {
                for (corner, share) in t.iter().zip([w.x, w.y, w.z]) {
                    if let Some(inf) = weights.get(*corner as usize) {
                        for (b, x) in inf.bones.iter().zip(inf.weights) {
                            if let Some(slot) = out.get_mut(usize::from(*b)) {
                                *slot += x * share.max(0.0);
                            }
                        }
                    }
                }
            }
            out
        })
        .collect();
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); positions.len()];
    for t in triangles {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            if let Some(n) = neighbours.get_mut(a as usize) {
                n.push(b);
            }
            if let Some(n) = neighbours.get_mut(b as usize) {
                n.push(a);
            }
        }
    }
    for n in &mut neighbours {
        n.sort_unstable();
        n.dedup();
    }
    for _ in 0..SMOOTHING_PASSES {
        dense = dense
            .iter()
            .zip(&neighbours)
            .map(|(own, around)| {
                if around.is_empty() {
                    return own.clone();
                }
                let mut avg = vec![0.0f32; bones];
                for v in around {
                    if let Some(w) = dense.get(*v as usize) {
                        for (a, x) in avg.iter_mut().zip(w) {
                            *a += x;
                        }
                    }
                }
                let k = around.len() as f32;
                own.iter()
                    .zip(&avg)
                    .map(|(o, a)| 0.5 * o + 0.5 * a / k)
                    .collect()
            })
            .collect();
    }
    dense.iter().map(|w| strongest(w)).collect()
}

/// The four heaviest bones of `dense`, their weights summing to 1
/// (unused slots: bone 0, weight 0).
pub(crate) fn strongest(dense: &[f32]) -> Influences {
    let mut order: Vec<usize> = (0..dense.len()).collect();
    order.sort_by(|a, b| {
        let w = |i: &usize| dense.get(*i).copied().unwrap_or(0.0);
        w(b).total_cmp(&w(a)).then(a.cmp(b))
    });
    let mut out = Influences::default();
    let top: Vec<(usize, f32)> = order
        .into_iter()
        .take(4)
        .map(|i| (i, dense.get(i).copied().unwrap_or(0.0)))
        .filter(|(_, w)| *w > 1e-4)
        .collect();
    let total: f32 = top.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        out.weights[0] = 1.0;
        return out;
    }
    for (k, (b, w)) in top.iter().enumerate() {
        if let (Some(slot), Some(weight)) = (out.bones.get_mut(k), out.weights.get_mut(k)) {
            *slot = u8::try_from(*b).unwrap_or(0);
            *weight = w / total;
        }
    }
    // Exactly 1: the rounding left on the heaviest.
    let rest: f32 = out.weights.iter().skip(1).sum();
    out.weights[0] = 1.0 - rest;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strongest_four_bones_sum_to_one() {
        let w = strongest(&[0.1, 0.0, 0.3, 0.2, 0.25, 0.15]);
        assert_eq!(w.bones, [2, 4, 3, 5]);
        assert!((w.weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(w.weights[0] > w.weights[3]);
        let none = strongest(&[0.0; 4]);
        assert_eq!(none.weights, [1.0, 0.0, 0.0, 0.0]);
    }
}
