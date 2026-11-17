//! The solver interface shared by the backends, and the constants of the
//! method (docs/physics.md §1, §4, §5).
//!
//! XPBD with small steps (Macklin et al. 2016, 2019): many substeps per
//! frame, one constraint iteration each. Per substep: predict, then solve
//! membranes, bending, seams, self-collision, the grab, and the contacts
//! with the body and the ground (with friction), then update velocities.
//! The CPU backend ([`crate::CpuSolver`]) is the reference; others must
//! pass the same reference scenes.

use glam::Vec3;

/// Substeps per frame.
pub const SUBSTEPS: u32 = 30;
/// Time over which seams pull together (s).
pub const SEWING_TIME: f32 = 0.6;
pub const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
/// Distance kept between the cloth and the body on top of the fabric
/// thickness (m). Cloth triangles (~25 mm) are flat chords of a curved
/// surface, and so are the avatar's: without this margin the body pokes
/// through between cloth vertices.
pub const SKIN: f32 = 0.0025;
/// Particles this much farther from the body than their contact distance
/// still count as near it, for the edges between them (m). Wider than an
/// edge's sag across the body (~1 mm for 25 mm edges over the bust), so an
/// edge that pushed its ends out keeps holding them there.
pub const NEAR: f32 = 0.005;
/// Particles within this distance of the grabbed one move with it (m).
pub const GRAB_RADIUS: f32 = 0.03;
/// Compliance of the grab (m/N): firm, but the cloth can still resist.
pub const GRAB_COMPLIANCE: f32 = 1e-4;

/// How far the sewing phase has gone at `time` (0 → 1).
pub fn sewing_ramp(time: f32) -> f32 {
    (time / SEWING_TIME).clamp(0.0, 1.0)
}

/// Seams start soft and tighten while the garment closes; meanwhile
/// gravity grows from zero, so pieces do not fall before they are sewn
/// (a skirt would close below the waist and slide off). Gravity and seam
/// compliance (m/N) at `time`.
pub fn sewing(time: f32, has_seams: bool) -> (Vec3, f32) {
    let ramp = sewing_ramp(time);
    let gravity = if has_seams { GRAVITY * ramp } else { GRAVITY };
    (gravity, 10f32.powf(-3.0 - 6.0 * ramp))
}

/// Statistics of the last step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StepStats {
    /// Largest distance between two sewn vertices (m).
    pub max_seam_gap: f32,
    /// Largest particle speed (m/s).
    pub max_speed: f32,
}

impl StepStats {
    /// Measured on positions `x` and velocities `v`; `seams` index both.
    pub fn measure(x: &[Vec3], v: &[Vec3], seams: &[[u32; 2]]) -> Self {
        let at = |i: u32| x.get(i as usize).copied().unwrap_or_default();
        Self {
            max_seam_gap: seams
                .iter()
                .map(|[a, b]| at(*a).distance(at(*b)))
                .fold(0.0, f32::max),
            max_speed: v.iter().map(|v| v.length()).fold(0.0, f32::max),
        }
    }
}

/// Why a step failed (a GPU backend can lose its device).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StepError {
    #[error("the simulation device failed: {0}")]
    Device(String),
}

/// A cloth solver backend.
pub trait Solver: Send + std::fmt::Debug {
    /// Where it runs: `"cpu"` or `"gpu"`.
    fn backend(&self) -> &'static str;

    /// Advance by `frame_dt` seconds. On error the positions stay those
    /// of the last good step.
    fn step(&mut self, frame_dt: f32) -> Result<(), StepError>;

    /// Particle positions after the last step (m).
    fn positions(&self) -> &[Vec3];

    /// Simulated time (s).
    fn time(&self) -> f32;

    fn stats(&self) -> StepStats;

    /// Carry on a simulation that was rebuilt (after a pattern edit) at
    /// time `t`: past the sewing ramp, seams are already stiff.
    fn continue_at(&mut self, t: f32);

    /// Hold the cloth around particle `index` (and its neighbours within
    /// [`GRAB_RADIUS`]) where it is now; [`Solver::move_grab`] then pulls
    /// it. Pinned particles are not held.
    fn grab(&mut self, index: usize);

    /// Move the held patch's target (its grabbed particle) to `target`.
    fn move_grab(&mut self, target: Vec3);

    fn release(&mut self);
}

/// The particles held by a grab of particle `index`: those free to move
/// within [`GRAB_RADIUS`] of it, with their offsets from it.
pub fn grabbed(x: &[Vec3], inv_mass: &[f32], index: usize) -> Option<(Vec3, Vec<(usize, Vec3)>)> {
    let center = *x.get(index)?;
    let particles = x
        .iter()
        .zip(inv_mass)
        .enumerate()
        .filter(|(_, (p, w))| **w > 0.0 && p.distance(center) < GRAB_RADIUS)
        .map(|(i, (p, _))| (i, *p - center))
        .collect();
    Some((center, particles))
}

/// Dihedral angle and its gradient for `[p0, p1, p2, p3]` (see
/// [`crate::model::Hinge`]): gradients from Bridson et al. 2003.
pub fn dihedral(p: [Vec3; 4]) -> Option<(f32, [Vec3; 4])> {
    let [p0, p1, p2, p3] = p;
    let n1 = (p2 - p0).cross(p3 - p0);
    let n2 = (p3 - p1).cross(p2 - p1);
    let e = p3 - p2;
    let (l1, l2, le) = (n1.length_squared(), n2.length_squared(), e.length());
    if l1 < 1e-20 || l2 < 1e-20 || le < 1e-10 {
        return None;
    }
    let (m1, m2) = (n1 / l1.sqrt(), n2 / l2.sqrt());
    // Sign convention matching the gradients below (Bridson et al. 2003).
    let theta = m2.cross(m1).dot(e / le).atan2(m1.dot(m2));
    let (q1, q2) = (n1 / l1, n2 / l2);
    let u0 = q1 * le;
    let u1 = q2 * le;
    let u2 = q1 * ((p0 - p3).dot(e) / le) + q2 * ((p1 - p3).dot(e) / le);
    let u3 = -q1 * ((p0 - p2).dot(e) / le) - q2 * ((p1 - p2).dot(e) / le);
    Some((theta, [u0, u1, u2, u3]))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn dihedral_gradient_matches_finite_differences() {
        let p = [
            Vec3::new(0.1, 0.9, 0.2),
            Vec3::new(0.2, -0.8, 0.5),
            Vec3::new(-0.6, 0.05, 0.0),
            Vec3::new(0.7, -0.1, 0.1),
        ];
        let (theta, grads) = dihedral(p).unwrap();
        let h = 1e-3;
        for i in 0..4 {
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let mut q = p;
                q[i] += axis * h;
                let (t2, _) = dihedral(q).unwrap();
                let numeric = (t2 - theta) / h;
                let analytic = grads[i].dot(axis);
                assert!(
                    (numeric - analytic).abs() < 2e-2 * (1.0 + analytic.abs()),
                    "vertex {i} axis {axis}: numeric {numeric} analytic {analytic}"
                );
            }
        }
    }

    #[test]
    fn flat_hinge_has_zero_angle() {
        let p = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ];
        assert!(dihedral(p).unwrap().0.abs() < 1e-6);
    }
}
