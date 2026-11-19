//! A scene compiled into what every solver backend works on: per-particle
//! data and flat constraint arrays, with their compliances (docs/physics.md
//! §3). Plain data, built once per scene; the backends only differ in how
//! they run it.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use crate::collide::StaticMesh;
use crate::motion::BodyMotion;
use crate::scene::SimScene;
use crate::selfcollide::SelfContact;

/// Strain-based membrane on one triangle (Müller et al. 2014): stretch
/// `|F·u| − 1`, `|F·v| − 1` and shear `F·u · F·v` in the material frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Membrane {
    pub v: [u32; 3],
    /// Coefficients giving `F·u` and `F·v` from the two edge vectors.
    pub cu: Vec2,
    pub cv: Vec2,
    /// Compliances (m/N) of warp and weft stretch and of shear.
    pub alpha_u: f32,
    pub alpha_v: f32,
    pub alpha_s: f32,
}

/// Dihedral bending on an interior edge (rest: flat).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hinge {
    /// Opposite vertex of triangle A, opposite vertex of triangle B, then
    /// the shared edge (in triangle A's order).
    pub v: [u32; 4],
    /// Compliance (1/(N·m)).
    pub alpha: f32,
}

/// A scene ready to simulate. Indices in every constraint are valid.
#[derive(Debug, Clone)]
pub struct Model {
    /// Initial positions (m).
    pub positions: Vec<Vec3>,
    /// Inverse masses (1/kg); 0 for pinned particles.
    pub inv_mass: Vec<f32>,
    /// Collision thickness per particle (m).
    pub thickness: Vec<f32>,
    /// Coulomb friction against the body per particle.
    pub friction: Vec<f32>,
    /// Velocity damping per second, per particle.
    pub damping: Vec<f32>,
    pub membranes: Vec<Membrane>,
    pub hinges: Vec<Hinge>,
    /// Vertex pairs sewn together.
    pub seams: Vec<[u32; 2]>,
    /// Vertex pairs held by closures (buttons, closed zippers).
    pub closures: Vec<[u32; 2]>,
    /// Mesh edges, each once, sorted.
    pub edges: Vec<[u32; 2]>,
    pub self_contact: SelfContact,
    pub collider: Option<StaticMesh>,
    /// How the body moves, if it does (only with a collider).
    pub motion: Option<BodyMotion>,
    pub ground: bool,
}

impl Model {
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
        let closures: Vec<crate::scene::Closure> = scene
            .closures
            .iter()
            .filter(|c| valid(&c.pair) && c.pair[0] != c.pair[1])
            .copied()
            .collect();
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
                let i = i as usize;
                mass[i] += fabric.density * area / 3.0;
                thickness[i] = thickness[i].max(fabric.thickness);
                friction[i] = fabric.friction;
                damping[i] = fabric.damping;
            }
        }

        // Interior edges → hinges.
        let mut open: HashMap<(u32, u32), (usize, u32, u32, u32)> = HashMap::new();
        let mut hinges = Vec::new();
        for (ti, t) in triangles.iter().enumerate() {
            let v = t.vertices;
            for k in 0..3 {
                let (a, b, opp) = (v[k], v[(k + 1) % 3], v[(k + 2) % 3]);
                let key = if a < b { (a, b) } else { (b, a) };
                if let Some((tj, a0, b0, opp0)) = open.remove(&key) {
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
                    open.insert(key, (ti, a, b, opp));
                }
            }
        }

        let inv_mass = scene
            .particles
            .iter()
            .zip(&mass)
            .map(|(p, m)| if p.pinned || *m <= 0.0 { 0.0 } else { 1.0 / m })
            .collect();
        let mut edges: Vec<[u32; 2]> = triangles
            .iter()
            .flat_map(|t| {
                let v = t.vertices;
                [[v[0], v[1]], [v[1], v[2]], [v[2], v[0]]]
            })
            .map(|[a, b]| [a.min(b), a.max(b)])
            .collect();
        edges.sort_unstable();
        edges.dedup();
        // Closed closures join the cloth as seams do; buttoned bands lie
        // on each other.
        let ties: Vec<[u32; 2]> = seams
            .iter()
            .copied()
            .chain(closures.iter().map(|c| c.pair))
            .collect();
        let overlaps: Vec<([u32; 2], f32)> = closures
            .iter()
            .filter(|c| c.overlap > 0.0)
            .map(|c| (c.pair, c.overlap))
            .collect();
        let self_contact = SelfContact::new(
            &scene.particles.iter().map(|p| p.rest).collect::<Vec<_>>(),
            &triangles.iter().map(|t| t.vertices).collect::<Vec<_>>(),
            &ties,
            &overlaps,
        );
        Self {
            positions: scene.particles.iter().map(|p| p.position).collect(),
            inv_mass,
            thickness,
            friction,
            damping,
            membranes,
            hinges,
            seams,
            closures: closures.iter().map(|c| c.pair).collect(),
            edges,
            self_contact,
            motion: scene.motion.filter(|_| scene.collider.is_some()),
            collider: scene.collider,
            ground: scene.ground,
        }
    }
}
