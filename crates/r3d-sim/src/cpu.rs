//! The CPU backend: XPBD constraints solved serially (Gauss–Seidel in a
//! fixed order), contacts in parallel per particle. Deterministic, for any
//! number of threads: the reference the other backends are checked against.
//!
//! Constraints (docs/physics.md §3):
//! - membrane, per triangle, in the fabric's material frame (warp u, weft v):
//!   stretch `|F·u| − 1`, `|F·v| − 1` and shear `F·u · F·v`, where `F` is the
//!   deformation gradient (Strain Based Dynamics, Müller et al. 2014);
//! - bending: dihedral angle on each interior edge (rest: flat), gradients
//!   from Bridson et al. 2003, stiffness from the bending rigidity as in
//!   discrete shells (Grinspun et al. 2003);
//! - seams: zero-length distances whose compliance ramps down while the
//!   garment closes;
//! - contacts with the body and the ground, with friction; mesh edges
//!   between two particles near the body are kept out of it at their
//!   middle too (a straight edge cuts into a convex body such as the bust);
//! - self-collision between distant parts of the cloth (`selfcollide`);
//! - grab: the user pulls a patch of cloth towards a moving target with a
//!   soft zero-length constraint per particle (offsets kept).

use glam::{Vec2, Vec3};
use rayon::prelude::*;

use crate::model::{Hinge, Membrane, Model};
use crate::scene::SimScene;
use crate::solver::{
    GRAB_COMPLIANCE, NEAR, SKIN, SUBSTEPS, Solver, StepError, StepStats, dihedral, grabbed, sewing,
};

/// A patch of cloth held by the user.
#[derive(Debug, Clone)]
struct Grab {
    /// Particles and their offsets from the target.
    particles: Vec<(usize, Vec3)>,
    target: Vec3,
}

/// The CPU cloth solver.
pub struct CpuSolver {
    model: Model,
    x: Vec<Vec3>,
    prev: Vec<Vec3>,
    v: Vec<Vec3>,
    /// Self-collision candidates, gathered once per frame.
    pairs: Vec<[u32; 2]>,
    /// Particles near the body (within their contact distance + [`NEAR`])
    /// in the last contact pass.
    near: Vec<bool>,
    /// Per particle: the sum and count of its edges' pushes out of the
    /// body in a substep (reused buffer).
    edge_push: Vec<(Vec3, u32)>,
    grab: Option<Grab>,
    time: f32,
    stats: StepStats,
}

impl std::fmt::Debug for CpuSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpuSolver")
            .field("particles", &self.x.len())
            .field("membranes", &self.model.membranes.len())
            .field("hinges", &self.model.hinges.len())
            .field("seams", &self.model.seams.len())
            .field("time", &self.time)
            .finish()
    }
}

impl CpuSolver {
    pub fn new(scene: SimScene) -> Self {
        Self::from_model(Model::new(scene))
    }

    pub fn from_model(model: Model) -> Self {
        let n = model.positions.len();
        Self {
            x: model.positions.clone(),
            prev: model.positions.clone(),
            v: vec![Vec3::ZERO; n],
            pairs: Vec::new(),
            near: vec![false; n],
            edge_push: vec![(Vec3::ZERO, 0); n],
            grab: None,
            time: 0.0,
            stats: StepStats::default(),
            model,
        }
    }

    fn substep(&mut self, h: f32) {
        let (gravity, seam_alpha) = sewing(self.time, !self.model.seams.is_empty());
        let w = &self.model.inv_mass;
        for i in 0..self.x.len() {
            self.prev[i] = self.x[i];
            if w[i] > 0.0 {
                self.v[i] += gravity * h;
                self.x[i] += self.v[i] * h;
            }
        }
        let h2 = h * h;
        for m in &self.model.membranes {
            membrane(&mut self.x, w, m, h2);
        }
        for hinge in &self.model.hinges {
            bend(&mut self.x, w, hinge, h2);
        }
        for [a, b] in &self.model.seams {
            let (a, b) = (*a as usize, *b as usize);
            let d = self.x[b] - self.x[a];
            let len = d.length();
            if len > 1e-9 {
                let n = d / len;
                project(&mut self.x, w, &[a, b], &[-n, n], len, seam_alpha / h2);
            }
        }
        self.model.self_contact.solve(&self.pairs, &mut self.x, w);
        if let Some(g) = &self.grab {
            let alpha = GRAB_COMPLIANCE / h2;
            for (i, offset) in &g.particles {
                let wi = w[*i];
                // XPBD on |x − goal| = 0 with a single moving particle.
                let pull = (g.target + *offset - self.x[*i]) * (wi / (wi + alpha));
                self.x[*i] += pull;
            }
        }
        self.collide();
        for i in 0..self.x.len() {
            self.v[i] = (self.x[i] - self.prev[i]) / h;
            self.v[i] *= (1.0 - self.model.damping[i] * h).max(0.0);
        }
    }

    /// Contacts with the body and the ground. The most expensive part of a
    /// substep (a closest-point search per particle), and independent per
    /// particle: solved in parallel, with the same result for any number of
    /// threads.
    fn collide(&mut self) {
        let m = &self.model;
        let (w, thickness, friction) = (&m.inv_mass, &m.thickness, &m.friction);
        let (collider, ground) = (m.collider.as_ref(), m.ground);
        self.x
            .par_iter_mut()
            .zip(&self.prev)
            .zip(self.near.par_iter_mut())
            .enumerate()
            .with_min_len(64)
            .for_each(|(i, ((x, prev), near))| {
                *near = false;
                let (Some(w), Some(thickness), Some(friction)) =
                    (w.get(i), thickness.get(i), friction.get(i))
                else {
                    return;
                };
                if *w == 0.0 {
                    return;
                }
                // The contact's normal and how far the particle was pushed
                // out along it (the normal force, in position terms).
                let mut contact: Option<(Vec3, f32)> = None;
                // The body's distance field sees penetrations up to its band
                // (2 cm) deep, far more than a particle moves in a substep.
                let distance = thickness + SKIN;
                if let Some(body) = collider
                    && let Some(c) = body.sample(*x)
                    && c.distance < distance + NEAR
                {
                    *near = true;
                    if c.distance < distance {
                        let depth = distance - c.distance;
                        *x += c.normal * depth;
                        contact = Some((c.normal, depth));
                    }
                }
                if ground && x.y < *thickness {
                    let depth = *thickness - x.y;
                    x.y = *thickness;
                    contact = Some((Vec3::Y, depth));
                }
                if let Some((n, depth)) = contact {
                    // Coulomb friction in positions (Macklin et al. 2014,
                    // §6.1): the sliding this substep is cancelled up to
                    // `friction` × the push out of the body, so the cloth
                    // holds where it presses and slides where it only rests.
                    let moved = *x - *prev;
                    let tangential = moved - n * moved.dot(n);
                    let sliding = tangential.length();
                    if sliding > 0.0 {
                        *x -= tangential * (friction * depth / sliding).min(1.0);
                    }
                }
            });
        self.collide_edges();
    }

    /// Edges whose two particles are near the body: keep their middle out
    /// of it too, pushing both ends outward (by their inverse masses).
    /// Cloth edges (~25 mm) are chords: across a convex part of the body
    /// their middle sinks in while their ends rest on the skin. Only edges
    /// near the body are sampled, so the cost follows the contact area.
    ///
    /// The edges stay active while their ends are near ([`NEAR`]), not only
    /// touching: gated on touching, an edge pushed its ends off the skin,
    /// let go of them, they fell back and were pushed again, every
    /// substep, and the garment never came to rest. A particle ends about
    /// six edges: their pushes are averaged (constraint averaging, Macklin
    /// et al. 2014 §4.2), not summed.
    fn collide_edges(&mut self) {
        let m = &self.model;
        let Some(body) = m.collider.as_ref() else {
            return;
        };
        self.edge_push.fill((Vec3::ZERO, 0));
        for [a, b] in &m.edges {
            let (a, b) = (*a as usize, *b as usize);
            if !(self.near[a] && self.near[b]) {
                continue;
            }
            let (wa, wb) = (m.inv_mass[a], m.inv_mass[b]);
            if wa + wb == 0.0 {
                continue;
            }
            let middle = (self.x[a] + self.x[b]) / 2.0;
            let distance = (m.thickness[a] + m.thickness[b]) / 2.0 + SKIN;
            if let Some(c) = body.sample(middle)
                && c.distance < distance
            {
                // Moving a by 2δ·wa/(wa+wb) and b by 2δ·wb/(wa+wb) moves
                // the middle by δ.
                let push = c.normal * (2.0 * (distance - c.distance) / (wa + wb));
                for (i, wi) in [(a, wa), (b, wb)] {
                    if let Some((sum, count)) = self.edge_push.get_mut(i) {
                        *sum += push * wi;
                        *count += 1;
                    }
                }
            }
        }
        for (x, (sum, count)) in self.x.iter_mut().zip(&self.edge_push) {
            if *count > 0 {
                *x += *sum / *count as f32;
            }
        }
    }
}

impl Solver for CpuSolver {
    fn step(&mut self, frame_dt: f32) -> Result<(), StepError> {
        let h = frame_dt / SUBSTEPS as f32;
        self.model
            .self_contact
            .candidates(&self.x, &self.model.inv_mass, &mut self.pairs);
        for _ in 0..SUBSTEPS {
            self.substep(h);
            self.time += h;
        }
        self.stats = StepStats::measure(&self.x, &self.v, &self.model.seams);
        Ok(())
    }

    fn positions(&self) -> &[Vec3] {
        &self.x
    }

    fn time(&self) -> f32 {
        self.time
    }

    fn stats(&self) -> StepStats {
        self.stats
    }

    fn continue_at(&mut self, t: f32) {
        self.time = t.max(0.0);
    }

    fn grab(&mut self, index: usize) {
        if let Some((target, particles)) = grabbed(&self.x, &self.model.inv_mass, index) {
            self.grab = Some(Grab { particles, target });
        }
    }

    fn move_grab(&mut self, target: Vec3) {
        if let Some(g) = &mut self.grab {
            g.target = target;
        }
    }

    fn release(&mut self) {
        self.grab = None;
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

fn bend(x: &mut [Vec3], w: &[f32], hinge: &Hinge, h2: f32) {
    let idx = hinge.v.map(|i| i as usize);
    let Some((theta, grads)) = dihedral(idx.map(|i| x[i])) else {
        return;
    };
    project(x, w, &idx, &grads, theta, hinge.alpha / h2);
}
