//! A scene compiled into what every solver backend works on: per-particle
//! data and flat constraint arrays, with their compliances (docs/physics.md
//! §3). Plain data, built once per scene; the backends only differ in how
//! they run it.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use crate::collide::StaticMesh;
use crate::layers::LayerContact;
use crate::motion::BodyMotion;
use crate::scene::SimScene;
use crate::selfcollide::SelfContact;
use crate::solver::{WEIGHTLESS_DAMPING, WEIGHTLESS_INERTIA};

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
    /// Along an elastic: warp and weft resist stretching only. The cloth
    /// there gathers in folds finer than the mesh rather than compresses
    /// (Choi and Ko 2002).
    pub buckles: bool,
}

/// How far from an elastic the cloth gathers in folds finer than the
/// mesh, on the pattern (m; docs/physics.md, "Elastic"). Narrower, the
/// cloth just beyond had to take the whole gathering in a few coarse
/// folds and held a sleeve's hem 25 % open.
pub const BUCKLING_BAND: f32 = 0.04;

/// One segment of an elastic: a distance that only pulls, between two
/// consecutive particles of its line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElasticLink {
    pub v: [u32; 2],
    /// Their distance on the pattern (m): the rest length at the start of
    /// the sewing ramp.
    pub full: f32,
    /// The elastic's relaxed length there (m), reached at its end.
    pub rest: f32,
    /// Newtons per unit strain (> 0).
    pub stiffness: f32,
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

/// No seam partner (in [`Model::partner`]).
pub const UNSEWN: u32 = u32::MAX;

/// Per particle of `n`, the other end of its first seam in `seams`.
fn partners(n: usize, seams: &[[u32; 2]]) -> Vec<u32> {
    let mut partner = vec![UNSEWN; n];
    for [a, b] in seams {
        for (i, j) in [(*a, *b), (*b, *a)] {
            if let Some(p) = partner.get_mut(i as usize)
                && *p == UNSEWN
                && i != j
            {
                *p = j;
            }
        }
    }
    partner
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
    /// Per particle, the one it is sewn to (by its first seam), or
    /// [`UNSEWN`]: friction holds a sewn pair by its common motion
    /// (docs/physics.md §5, "Friction on a seam").
    pub partner: Vec<u32>,
    /// Vertex pairs held by closures (buttons, closed zippers).
    pub closures: Vec<[u32; 2]>,
    pub elastics: Vec<ElasticLink>,
    /// Mesh edges, each once, sorted.
    pub edges: Vec<[u32; 2]>,
    pub self_contact: SelfContact,
    /// Contacts between garments.
    pub layers: LayerContact,
    pub collider: Option<StaticMesh>,
    /// How the body moves, if it does (only with a collider).
    pub motion: Option<BodyMotion>,
    pub ground: bool,
    /// No gravity ([`SimScene::weightless`]).
    pub weightless: bool,
    /// The stuffing's pressure (Pa, [`SimScene::pressure`]).
    pub pressure: f32,
    /// Per cloth triangle (in `layers.triangles` order): whether the
    /// pressure pushes it, i.e. it lies on cloth closed all around.
    pub inflated: Vec<bool>,
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
        // Elastic segments, and the particles they gather.
        let mut elastics = Vec::new();
        for line in &scene.elastics {
            if !(line.stiffness > 0.0 && line.ratio > 0.0 && line.ratio <= 1.0) {
                continue;
            }
            for w in line.particles.windows(2) {
                let &[a, b] = w else { continue };
                let (Some(pa), Some(pb)) = (
                    scene.particles.get(a as usize),
                    scene.particles.get(b as usize),
                ) else {
                    continue;
                };
                let full = pa.rest.distance(pb.rest);
                if a == b || full <= 1e-9 {
                    continue;
                }
                elastics.push(ElasticLink {
                    v: [a, b],
                    full,
                    rest: full * line.ratio,
                    stiffness: line.stiffness,
                });
            }
        }
        // The cloth near an elastic on the pattern gathers in folds finer
        // than the mesh.
        let segments: Vec<(Vec2, Vec2)> = elastics
            .iter()
            .filter_map(|l| {
                let [a, b] = l.v.map(|i| scene.particles.get(i as usize).map(|p| p.rest));
                Some((a?, b?))
            })
            .collect();
        let gathers = |t: &crate::scene::Triangle| {
            let c = t
                .vertices
                .iter()
                .filter_map(|i| scene.particles.get(*i as usize))
                .map(|p| p.rest)
                .sum::<Vec2>()
                / 3.0;
            segments.iter().any(|(a, b)| {
                let ab = *b - *a;
                let s = ((c - *a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
                c.distance(*a + ab * s) < BUCKLING_BAND
            })
        };
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
                buckles: gathers(t),
            });
            for i in t.vertices {
                let i = i as usize;
                mass[i] += fabric.density * area / 3.0;
                thickness[i] = thickness[i].max(fabric.thickness);
                friction[i] = fabric.friction;
                damping[i] = fabric.damping;
            }
        }

        // Interior edges → hinges; creases bend as they say.
        let creases: HashMap<(u32, u32), f32> = scene
            .creases
            .iter()
            .map(|([a, b], bending)| ((*a.min(b), *a.max(b)), *bending))
            .collect();
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
                    let bending = creases
                        .get(&key)
                        .copied()
                        .unwrap_or(0.5 * (fabric_a.bending + fabric_b.bending));
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

        if scene.weightless {
            for m in &mut mass {
                *m *= WEIGHTLESS_INERTIA;
            }
            for d in &mut damping {
                *d = d.max(WEIGHTLESS_DAMPING);
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
        // Gathered cloth lies in folds on itself: within twice the band
        // around each point of an elastic (along the cloth), it does not
        // repel itself (a narrower patch held a sleeve's hem 5 % open).
        let overlaps: Vec<([u32; 2], f32)> = closures
            .iter()
            .filter(|c| c.overlap > 0.0)
            .map(|c| (c.pair, c.overlap))
            .chain(
                elastics
                    .iter()
                    .map(|l| ([l.v[0], l.v[0]], 2.0 * BUCKLING_BAND)),
            )
            .collect();
        let rest_positions: Vec<Vec2> = scene.particles.iter().map(|p| p.rest).collect();
        let corners: Vec<[u32; 3]> = triangles.iter().map(|t| t.vertices).collect();
        let garments: Vec<u16> = triangles.iter().map(|t| t.garment).collect();
        let longest = edges
            .iter()
            .map(|[a, b]| rest(*a).distance(rest(*b)))
            .fold(0.0f32, f32::max);
        let mut sewn = vec![false; n];
        for [a, b] in &ties {
            sewn[*a as usize] = true;
            sewn[*b as usize] = true;
        }
        let mut layers = LayerContact::new(
            n,
            &corners,
            &garments,
            &seams,
            &sewn,
            longest,
            thickness.iter().copied().fold(0.0, f32::max),
        );
        let inflated = if scene.pressure > 0.0 {
            closed(n, &corners, &ties, &layers.free)
        } else {
            vec![false; corners.len()]
        };
        let self_contact =
            SelfContact::new(&rest_positions, &corners, &ties, &overlaps, &layers.garment);
        layers.sewn = self_contact.sewn.clone();
        Self {
            positions: scene.particles.iter().map(|p| p.position).collect(),
            inv_mass,
            thickness,
            friction,
            damping,
            membranes,
            hinges,
            partner: partners(n, &seams),
            seams,
            closures: closures.iter().map(|c| c.pair).collect(),
            elastics,
            edges,
            self_contact,
            layers,
            motion: scene.motion.filter(|_| scene.collider.is_some()),
            collider: scene.collider,
            ground: scene.ground,
            weightless: scene.weightless,
            pressure: scene.pressure.max(0.0),
            inflated,
        }
    }
}

/// Per triangle of `corners` (over `n` particles, joined by `ties`, with
/// [`LayerContact::free`] edges `free`): whether it lies on a piece of
/// cloth closed all around, its pieces joined by seams and closures with
/// no free edge left. Only those are stuffed: an open part pushed on all
/// its faces but its mouth would drift away (docs/physics.md, "Objects").
fn closed(n: usize, corners: &[[u32; 3]], ties: &[[u32; 2]], free: &[u8]) -> Vec<bool> {
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn root(parent: &mut [u32], mut i: u32) -> u32 {
        while let Some(&p) = parent.get(i as usize) {
            if p == i {
                break;
            }
            // Path halving.
            let up = parent.get(p as usize).copied().unwrap_or(p);
            if let Some(slot) = parent.get_mut(i as usize) {
                *slot = up;
            }
            i = up;
        }
        i
    }
    let join = |parent: &mut Vec<u32>, a: u32, b: u32| {
        let (ra, rb) = (root(parent, a), root(parent, b));
        if let Some(slot) = parent.get_mut(ra as usize) {
            *slot = rb;
        }
    };
    for t in corners {
        join(&mut parent, t[0], t[1]);
        join(&mut parent, t[1], t[2]);
    }
    for [a, b] in ties {
        join(&mut parent, *a, *b);
    }
    let edges = (1u8 << 3) - 1;
    let mut open = vec![false; n];
    for (t, bits) in corners.iter().zip(free) {
        if bits & edges != 0 {
            let r = root(&mut parent, t[0]);
            if let Some(o) = open.get_mut(r as usize) {
                *o = true;
            }
        }
    }
    corners
        .iter()
        .map(|t| {
            let r = root(&mut parent, t[0]);
            !open.get(r as usize).copied().unwrap_or(true)
        })
        .collect()
}
