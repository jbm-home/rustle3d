//! XPBD with small steps (Macklin et al. 2016, 2019): many substeps per
//! frame, one constraint iteration each.
//!
//! Constraints:
//! - membrane, per triangle, in the fabric's material frame (warp u, weft v):
//!   stretch `|F·u| − 1`, `|F·v| − 1` and shear `F·u · F·v`, where `F` is the
//!   deformation gradient (Strain Based Dynamics, Müller et al. 2014);
//! - bending: dihedral angle on each interior edge (rest: flat), gradients
//!   from Bridson et al. 2003, stiffness from the bending rigidity as in
//!   discrete shells (Grinspun et al. 2003);
//! - seams: zero-length distances whose compliance ramps down while the
//!   garment closes;
//! - contacts with the body and the ground, with friction.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use crate::collide::StaticMesh;
use crate::scene::SimScene;

/// Substeps per frame.
pub const SUBSTEPS: u32 = 15;
/// Time over which seams pull together (s).
pub const SEWING_TIME: f32 = 0.6;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
/// Extra search distance around the cloth thickness for contacts (m).
const CONTACT_MARGIN: f32 = 0.005;

struct Membrane {
    v: [u32; 3],
    /// Coefficients giving `F·u` and `F·v` from the two edge vectors.
    cu: Vec2,
    cv: Vec2,
    alpha_u: f32,
    alpha_v: f32,
    alpha_s: f32,
}

struct Hinge {
    /// Opposite vertex of triangle A, opposite vertex of triangle B, then
    /// the shared edge (in triangle A's order).
    v: [u32; 4],
    alpha: f32,
}

/// Statistics of the last step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StepStats {
    /// Largest distance between two sewn vertices (m).
    pub max_seam_gap: f32,
    /// Largest particle speed (m/s).
    pub max_speed: f32,
}

/// The cloth solver.
pub struct Solver {
    x: Vec<Vec3>,
    prev: Vec<Vec3>,
    v: Vec<Vec3>,
    w: Vec<f32>,
    thickness: Vec<f32>,
    friction: Vec<f32>,
    damping: Vec<f32>,
    membranes: Vec<Membrane>,
    hinges: Vec<Hinge>,
    seams: Vec<[u32; 2]>,
    collider: Option<StaticMesh>,
    ground: bool,
    time: f32,
    stats: StepStats,
}

impl std::fmt::Debug for Solver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solver")
            .field("particles", &self.x.len())
            .field("membranes", &self.membranes.len())
            .field("hinges", &self.hinges.len())
            .field("seams", &self.seams.len())
            .field("time", &self.time)
            .finish()
    }
}

impl Solver {
    pub fn new(scene: SimScene) -> Self {
        let n = scene.particles.len();
        let mut mass = vec![0.0f32; n];
        let mut thickness = vec![0.001f32; n];
        let mut friction = vec![0.3f32; n];
        let mut damping = vec![0.1f32; n];
        let mut membranes = Vec::with_capacity(scene.triangles.len());
        let mut rest_area = Vec::with_capacity(scene.triangles.len());
        let rest = |i: u32| {
            scene
                .particles
                .get(i as usize)
                .map_or(Vec2::ZERO, |p| p.rest)
        };

        for t in &scene.triangles {
            let Some(fabric) = scene.fabrics.get(t.fabric as usize) else {
                rest_area.push(0.0);
                continue;
            };
            let [i0, i1, i2] = t.vertices;
            let warp = t.warp.try_normalize().unwrap_or(Vec2::X);
            let weft = warp.perp();
            let (d1, d2) = (rest(i1) - rest(i0), rest(i2) - rest(i0));
            // Dm = [[d1·u, d2·u], [d1·v, d2·v]]
            let (a, b, c, d) = (d1.dot(warp), d2.dot(warp), d1.dot(weft), d2.dot(weft));
            let det = a * d - b * c;
            let area = det.abs() / 2.0;
            rest_area.push(area);
            if area < 1e-10 {
                continue;
            }
            // Dm⁻¹ = [[d, −b], [−c, a]] / det; F·u uses its first column.
            let cu = Vec2::new(d, -c) / det;
            let cv = Vec2::new(-b, a) / det;
            membranes.push(Membrane {
                v: t.vertices,
                cu,
                cv,
                alpha_u: 1.0 / (fabric.stretch_warp * area),
                alpha_v: 1.0 / (fabric.stretch_weft * area),
                alpha_s: 1.0 / (fabric.shear * area),
            });
            for i in t.vertices {
                if let Some(m) = mass.get_mut(i as usize) {
                    *m += fabric.density * area / 3.0;
                }
                let i = i as usize;
                thickness[i] = thickness[i].max(fabric.thickness);
                friction[i] = fabric.friction;
                damping[i] = fabric.damping;
            }
        }

        // Interior edges → hinges.
        let mut edges: HashMap<(u32, u32), (usize, u32, u32, u32)> = HashMap::new();
        let mut hinges = Vec::new();
        for (ti, t) in scene.triangles.iter().enumerate() {
            let v = t.vertices;
            for k in 0..3 {
                let (a, b, opp) = (v[k], v[(k + 1) % 3], v[(k + 2) % 3]);
                let key = if a < b { (a, b) } else { (b, a) };
                if let Some((tj, a0, b0, opp0)) = edges.remove(&key) {
                    let (Some(fabric_a), Some(fabric_b)) = (
                        scene.fabrics.get(scene.triangles[tj].fabric as usize),
                        scene.fabrics.get(t.fabric as usize),
                    ) else {
                        continue;
                    };
                    let bending = 0.5 * (fabric_a.bending + fabric_b.bending);
                    let e2 = (rest(a0) - rest(b0)).length_squared();
                    let areas = rest_area[tj] + rest_area[ti];
                    if areas > 1e-10 && bending > 0.0 {
                        let k = bending * 6.0 * e2 / areas;
                        hinges.push(Hinge {
                            v: [opp0, opp, a0, b0],
                            alpha: 1.0 / k,
                        });
                    }
                } else {
                    edges.insert(key, (ti, a, b, opp));
                }
            }
        }

        let w = scene
            .particles
            .iter()
            .zip(&mass)
            .map(|(p, m)| if p.pinned || *m <= 0.0 { 0.0 } else { 1.0 / m })
            .collect();
        let x: Vec<Vec3> = scene.particles.iter().map(|p| p.position).collect();
        Self {
            prev: x.clone(),
            v: vec![Vec3::ZERO; n],
            x,
            w,
            thickness,
            friction,
            damping,
            membranes,
            hinges,
            seams: scene.seams,
            collider: scene.collider,
            ground: scene.ground,
            time: 0.0,
            stats: StepStats::default(),
        }
    }

    pub fn positions(&self) -> &[Vec3] {
        &self.x
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn stats(&self) -> StepStats {
        self.stats
    }

    /// Advance by `frame_dt` seconds.
    pub fn step(&mut self, frame_dt: f32) {
        let h = frame_dt / SUBSTEPS as f32;
        for _ in 0..SUBSTEPS {
            self.substep(h);
            self.time += h;
        }
        self.stats = StepStats {
            max_seam_gap: self
                .seams
                .iter()
                .map(|[a, b]| self.x[*a as usize].distance(self.x[*b as usize]))
                .fold(0.0, f32::max),
            max_speed: self.v.iter().map(|v| v.length()).fold(0.0, f32::max),
        };
    }

    fn substep(&mut self, h: f32) {
        for i in 0..self.x.len() {
            self.prev[i] = self.x[i];
            if self.w[i] > 0.0 {
                self.v[i] += GRAVITY * h;
                self.x[i] += self.v[i] * h;
            }
        }
        let h2 = h * h;
        for m in &self.membranes {
            membrane(&mut self.x, &self.w, m, h2);
        }
        for hinge in &self.hinges {
            bend(&mut self.x, &self.w, hinge, h2);
        }
        // Seams start soft and tighten while the garment closes.
        let ramp = (self.time / SEWING_TIME).clamp(0.0, 1.0);
        let seam_alpha = 10f32.powf(-3.0 - 6.0 * ramp);
        for [a, b] in &self.seams {
            let (a, b) = (*a as usize, *b as usize);
            let d = self.x[b] - self.x[a];
            let len = d.length();
            if len > 1e-9 {
                let n = d / len;
                project(
                    &mut self.x,
                    &self.w,
                    &[a, b],
                    &[-n, n],
                    len,
                    seam_alpha / h2,
                );
            }
        }
        self.collide();
        for i in 0..self.x.len() {
            self.v[i] = (self.x[i] - self.prev[i]) / h;
            self.v[i] *= (1.0 - self.damping[i] * h).max(0.0);
        }
    }

    fn collide(&mut self) {
        for i in 0..self.x.len() {
            if self.w[i] == 0.0 {
                continue;
            }
            let thickness = self.thickness[i];
            let mut contact_normal = None;
            // A particle outside at the start of the substep can only be as
            // deep as it moved: search that far, so fast motion (seams
            // closing a tight waist) cannot tunnel through the body.
            let reach = thickness + CONTACT_MARGIN + self.x[i].distance(self.prev[i]);
            if let Some(body) = &self.collider
                && let Some(c) = body.closest(self.x[i], reach)
                && (self.x[i] - c.point).dot(c.normal) < thickness
            {
                self.x[i] = c.point + c.normal * thickness;
                contact_normal = Some(c.normal);
            }
            if self.ground && self.x[i].y < thickness {
                self.x[i].y = thickness;
                contact_normal = Some(Vec3::Y);
            }
            if let Some(n) = contact_normal {
                // Coulomb-like friction: remove part of the sliding motion.
                let moved = self.x[i] - self.prev[i];
                let tangential = moved - n * moved.dot(n);
                self.x[i] -= tangential * self.friction[i];
            }
        }
    }
}

/// One XPBD update of a scalar constraint `c` with gradients `grads` on
/// particles `idx`; `alpha_tilde` = compliance / h².
fn project(x: &mut [Vec3], w: &[f32], idx: &[usize], grads: &[Vec3], c: f32, alpha_tilde: f32) {
    let denom: f32 = idx
        .iter()
        .zip(grads)
        .map(|(i, g)| w[*i] * g.length_squared())
        .sum::<f32>()
        + alpha_tilde;
    if denom <= 1e-12 || !c.is_finite() {
        return;
    }
    let dl = -c / denom;
    for (i, g) in idx.iter().zip(grads) {
        x[*i] += *g * (dl * w[*i]);
    }
}

fn membrane(x: &mut [Vec3], w: &[f32], m: &Membrane, h2: f32) {
    let [i0, i1, i2] = m.v.map(|i| i as usize);
    let idx = [i0, i1, i2];
    let deform = |x: &[Vec3], c: Vec2| (x[i1] - x[i0]) * c.x + (x[i2] - x[i0]) * c.y;
    for (coef, alpha) in [(m.cu, m.alpha_u), (m.cv, m.alpha_v)] {
        let f = deform(x, coef);
        let len = f.length();
        if len > 1e-9 {
            let n = f / len;
            let (g1, g2) = (n * coef.x, n * coef.y);
            project(x, w, &idx, &[-(g1 + g2), g1, g2], len - 1.0, alpha / h2);
        }
    }
    let (fu, fv) = (deform(x, m.cu), deform(x, m.cv));
    let g1 = fv * m.cu.x + fu * m.cv.x;
    let g2 = fv * m.cu.y + fu * m.cv.y;
    project(
        x,
        w,
        &idx,
        &[-(g1 + g2), g1, g2],
        fu.dot(fv),
        m.alpha_s / h2,
    );
}

/// Dihedral angle and its gradient for `[p0, p1, p2, p3]` (see [`Hinge`]).
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

fn bend(x: &mut [Vec3], w: &[f32], hinge: &Hinge, h2: f32) {
    let idx = hinge.v.map(|i| i as usize);
    let Some((theta, grads)) = dihedral(idx.map(|i| x[i])) else {
        return;
    };
    project(x, w, &idx, &grads, theta, hinge.alpha / h2);
}

#[cfg(test)]
mod tests {
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
