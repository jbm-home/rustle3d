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
//! - contacts with the body and the ground, with friction;
//! - self-collision between distant parts of the cloth (`selfcollide`);
//! - grab: the user pulls a patch of cloth towards a moving target with a
//!   soft zero-length constraint per particle (offsets kept).

use std::collections::HashMap;

use glam::{Vec2, Vec3};
use rayon::prelude::*;

use crate::collide::StaticMesh;
use crate::scene::SimScene;
use crate::selfcollide::SelfCollision;

/// Substeps per frame.
pub const SUBSTEPS: u32 = 30;
/// Time over which seams pull together (s).
pub const SEWING_TIME: f32 = 0.6;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
/// Distance kept between the cloth and the body on top of the fabric
/// thickness (m). Cloth triangles (~25 mm) are flat chords of a curved
/// surface, and so are the avatar's: without this margin the body pokes
/// through between cloth vertices.
const SKIN: f32 = 0.0025;
/// Particles within this distance of the grabbed one move with it (m).
pub const GRAB_RADIUS: f32 = 0.03;
/// Compliance of the grab (m/N): firm, but the cloth can still resist.
const GRAB_COMPLIANCE: f32 = 1e-4;

/// A patch of cloth held by the user.
#[derive(Debug, Clone)]
struct Grab {
    /// Particles and their offsets from the target.
    particles: Vec<(usize, Vec3)>,
    target: Vec3,
}

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
    self_collision: SelfCollision,
    grab: Option<Grab>,
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
        // Triangles and seams on particles that do not exist are dropped,
        // so particles can be indexed freely afterwards.
        let valid = |v: &[u32]| v.iter().all(|i| (*i as usize) < n);
        let triangles: Vec<_> = scene
            .triangles
            .iter()
            .filter(|t| valid(&t.vertices))
            .copied()
            .collect();
        let seams: Vec<[u32; 2]> = scene.seams.iter().filter(|s| valid(*s)).copied().collect();
        let mut membranes = Vec::with_capacity(triangles.len());
        let mut rest_area = Vec::with_capacity(triangles.len());
        let rest = |i: u32| {
            scene
                .particles
                .get(i as usize)
                .map_or(Vec2::ZERO, |p| p.rest)
        };

        for t in &triangles {
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
        for (ti, t) in triangles.iter().enumerate() {
            let v = t.vertices;
            for k in 0..3 {
                let (a, b, opp) = (v[k], v[(k + 1) % 3], v[(k + 2) % 3]);
                let key = if a < b { (a, b) } else { (b, a) };
                if let Some((tj, a0, b0, opp0)) = edges.remove(&key) {
                    let (Some(fabric_a), Some(fabric_b)) = (
                        scene.fabrics.get(triangles[tj].fabric as usize),
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
            self_collision: SelfCollision::new(
                &scene.particles.iter().map(|p| p.rest).collect::<Vec<_>>(),
                &triangles.iter().map(|t| t.vertices).collect::<Vec<_>>(),
                &seams,
            ),
            seams,
            grab: None,
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

    /// Hold the cloth around particle `index` (and its neighbours within
    /// [`GRAB_RADIUS`]) where it is now; [`Solver::move_grab`] then pulls
    /// it. Pinned particles are not held.
    pub fn grab(&mut self, index: usize) {
        let Some(&center) = self.x.get(index) else {
            return;
        };
        let particles = self
            .x
            .iter()
            .enumerate()
            .filter(|(i, p)| self.w[*i] > 0.0 && p.distance(center) < GRAB_RADIUS)
            .map(|(i, p)| (i, *p - center))
            .collect();
        self.grab = Some(Grab {
            particles,
            target: center,
        });
    }

    /// Move the held patch's target (its grabbed particle) to `target`.
    pub fn move_grab(&mut self, target: Vec3) {
        if let Some(g) = &mut self.grab {
            g.target = target;
        }
    }

    /// Carry on a simulation that was rebuilt (after a pattern edit) at
    /// time `t`: past the sewing ramp, seams are already stiff.
    pub fn continue_at(&mut self, t: f32) {
        self.time = t.max(0.0);
    }

    pub fn release(&mut self) {
        self.grab = None;
    }

    /// Advance by `frame_dt` seconds.
    pub fn step(&mut self, frame_dt: f32) {
        let h = frame_dt / SUBSTEPS as f32;
        self.self_collision.refresh(&self.x, &self.w);
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
        // Seams start soft and tighten while the garment closes; meanwhile
        // gravity grows from zero, so pieces do not fall before they are
        // sewn (a skirt would close below the waist and slide off).
        let ramp = (self.time / SEWING_TIME).clamp(0.0, 1.0);
        let gravity = if self.seams.is_empty() {
            GRAVITY
        } else {
            GRAVITY * ramp
        };
        for i in 0..self.x.len() {
            self.prev[i] = self.x[i];
            if self.w[i] > 0.0 {
                self.v[i] += gravity * h;
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
        self.self_collision.solve(&mut self.x, &self.w);
        if let Some(g) = &self.grab {
            let alpha = GRAB_COMPLIANCE / h2;
            for (i, offset) in &g.particles {
                let w = self.w[*i];
                // XPBD on |x − goal| = 0 with a single moving particle.
                let pull = (g.target + *offset - self.x[*i]) * (w / (w + alpha));
                self.x[*i] += pull;
            }
        }
        self.collide();
        for i in 0..self.x.len() {
            self.v[i] = (self.x[i] - self.prev[i]) / h;
            self.v[i] *= (1.0 - self.damping[i] * h).max(0.0);
        }
    }

    /// Contacts with the body and the ground. The most expensive part of a
    /// substep (a closest-point search per particle), and independent per
    /// particle: solved in parallel, with the same result for any number of
    /// threads.
    fn collide(&mut self) {
        let (w, thickness, friction) = (&self.w, &self.thickness, &self.friction);
        let (collider, ground) = (self.collider.as_ref(), self.ground);
        self.x
            .par_iter_mut()
            .zip(&self.prev)
            .enumerate()
            .with_min_len(64)
            .for_each(|(i, (x, prev))| {
                let (Some(w), Some(thickness), Some(friction)) =
                    (w.get(i), thickness.get(i), friction.get(i))
                else {
                    return;
                };
                if *w == 0.0 {
                    return;
                }
                let mut contact_normal = None;
                // The body's distance field sees penetrations up to its band
                // (2 cm) deep, far more than a particle moves in a substep.
                let distance = thickness + SKIN;
                if let Some(body) = collider
                    && let Some(c) = body.sample(*x)
                    && c.distance < distance
                {
                    *x += c.normal * (distance - c.distance);
                    contact_normal = Some(c.normal);
                }
                if ground && x.y < *thickness {
                    x.y = *thickness;
                    contact_normal = Some(Vec3::Y);
                }
                if let Some(n) = contact_normal {
                    // Coulomb-like friction: remove part of the sliding motion.
                    let moved = *x - *prev;
                    let tangential = moved - n * moved.dot(n);
                    *x -= tangential * *friction;
                }
            });
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
