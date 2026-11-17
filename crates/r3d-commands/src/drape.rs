//! The sim bridge: build a cloth simulation scene from the document.
//!
//! Every placed panel is meshed (`r3d_geom::mesh`) with its boundary
//! sampled edge by edge; the two edges of a seam get the same number of
//! samples, so their vertices pair up one to one. Panels start wrapped
//! around the body (see `start_radius`), and the cloth collides with the
//! fitted avatar and the ground.
//!
//! [`simulate`] runs a drape headless (CLI, agents); its result is cached in
//! the project folder, keyed by a fingerprint of the document, so renders
//! can show it without simulating again.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::Path;
use std::time::Instant;

use glam::{Vec2, Vec3};
use r3d_avatar::fit::{fit, fitted_positions};
use r3d_core::{Around, EdgeId, Fabric, PanelId, PinId, Project};
use r3d_geom::DVec2;
use r3d_geom::mesh::mesh_polygon;
use r3d_sim::collide::StaticMesh;
use r3d_sim::scene::{Particle, Triangle};
use r3d_sim::{CpuSolver, FabricPreset, SUBSTEPS, SimScene, Solver, StepStats};

use crate::error::CommandError;
use crate::queries::body_model;

/// Default distance between cloth vertices (mm): interactive on a laptop.
pub const DEFAULT_SPACING_MM: f64 = 25.0;
/// Grid step of the avatar's distance field (m).
const BODY_VOXEL: f32 = 0.005;
/// Stature assumed when there is no avatar (m).
const DEFAULT_HEIGHT_M: f32 = 1.68;
/// Body radius assumed when there is no avatar (m).
const DEFAULT_BODY_RADIUS_M: f32 = 0.16;

/// A built simulation scene.
#[derive(Debug, Clone)]
pub struct Drape {
    pub scene: SimScene,
    /// One entry per simulated panel, in particle order.
    pub pieces: Vec<Piece>,
    /// The particles each pin holds.
    pub pins: Vec<(PinId, Vec<usize>)>,
    /// The particles along each simulated contour edge, start to end.
    pub edges: BTreeMap<EdgeId, Vec<usize>>,
}

/// The particles and triangles of one panel, to draw it on its own.
#[derive(Debug, Clone)]
pub struct Piece {
    pub panel: PanelId,
    pub particles: Range<usize>,
    /// Triangle corners, indices local to `particles`.
    pub indices: Vec<u32>,
}

/// The dress form garments are draped on: the fitted body (m), its
/// profile, its arms and its distance field. Building one takes a
/// fraction of a second, so callers keep it for as long as the
/// measurements do not change.
#[derive(Debug, Clone)]
pub struct Form {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    /// The body's distance from its vertical axis (m), every
    /// `PROFILE_STEP` from the floor and every 360° / `ANGLE_BINS` around
    /// (angle 0 = front, as placements count), arms left out.
    radial: Vec<[f32; ANGLE_BINS]>,
    /// The left (+x) and right arms.
    arms: [Arm; 2],
    collider: StaticMesh,
}

/// An arm: an axis down the upper arm to the elbow, then down the
/// forearm, and how far the arm reaches from it.
#[derive(Debug, Clone)]
struct Arm {
    /// A point on the upper arm's axis (m).
    point: Vec3,
    /// Unit direction down the upper arm.
    down: Vec3,
    /// Distance from `point` down the upper arm to the elbow (m), where the
    /// axis turns to `fore`.
    elbow: f32,
    /// Unit direction down the forearm.
    fore: Vec3,
    /// Largest distance of the arm's surface from the axis (m), every
    /// `PROFILE_STEP` along it from `point - REACH_FROM * down`.
    reach: Vec<f32>,
}

/// Height step of the body profile (m).
const PROFILE_STEP: f32 = 0.01;
/// Angular bins of the body profile.
const ANGLE_BINS: usize = 36;
/// Body points farther from the axis than this are forearms and hands (in
/// the A pose the hands hang at hip height, about 0.4 m out).
const ARM_REACH_M: f32 = 0.3;
/// The upper arm, as fractions of stature in the A pose of the base body:
/// between these heights, points farther out than `ARM_SIDE` are the arm.
const UPPER_ARM_HEIGHTS: (f32, f32) = (0.69, 0.79);
const ARM_SIDE: f32 = 0.13;
/// The forearm, likewise: between the wrist and the elbow.
const FOREARM_HEIGHTS: (f32, f32) = (0.60, 0.66);
/// Points closer than this to an arm's axis, below the shoulder
/// (`SHOULDER` of stature), belong to the arm (m; the arm itself measures
/// 5–6 cm from its axis, the torso 13 cm or more).
const ARM_RADIUS_M: f32 = 0.08;
const SHOULDER: f32 = 0.78;
/// The arm profile starts this far up the axis from its point (m).
const REACH_FROM: f32 = 0.5;
/// Arm reach assumed between measured rows and without an avatar (m).
const DEFAULT_ARM_RADIUS_M: f32 = 0.05;

fn angle_bin(theta: f32) -> usize {
    let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
    ((t * ANGLE_BINS as f32) as usize).min(ANGLE_BINS - 1)
}

/// Unit principal direction of `points` around their centroid (power
/// iteration from `start`).
fn principal_axis(points: &[Vec3], centre: Vec3, start: Vec3) -> Vec3 {
    let mut cov = glam::Mat3::ZERO;
    for p in points {
        let d = *p - centre;
        cov += glam::Mat3::from_cols(d * d.x, d * d.y, d * d.z);
    }
    let mut axis = start;
    for _ in 0..64 {
        axis = (cov * axis).normalize_or(start);
    }
    axis
}

impl Arm {
    /// The arm on `side` (+1 left, −1 right) of a body of `height` (m),
    /// without an avatar: a typical A pose.
    fn default(side: f32, height: f32) -> Self {
        let down = Vec3::new(side * 0.6, -0.8, 0.0);
        Self {
            point: Vec3::new(side * 0.17 * height, 0.72 * height, 0.0),
            down,
            elbow: 0.0,
            fore: down,
            reach: Vec::new(),
        }
    }

    /// The point `t` along the axis, and the axis's direction there.
    fn axis(&self, t: f32) -> (Vec3, Vec3) {
        if t <= self.elbow {
            (self.point + self.down * t, self.down)
        } else {
            let elbow = self.point + self.down * self.elbow;
            (elbow + self.fore * (t - self.elbow), self.fore)
        }
    }

    /// Find the arm on `side` of a body (A pose).
    fn find(positions: &[Vec3], height: f32, side: f32) -> Self {
        let segment = |(lo, hi): (f32, f32), start: Vec3| {
            let points: Vec<Vec3> = positions
                .iter()
                .copied()
                .filter(|p| {
                    side * p.x > ARM_SIDE * height && p.y > lo * height && p.y < hi * height
                })
                .collect();
            (points.len() >= 3).then(|| {
                let centre = points.iter().copied().sum::<Vec3>() / points.len() as f32;
                let axis = principal_axis(&points, centre, start);
                (centre, if axis.y > 0.0 { -axis } else { axis })
            })
        };
        let Some((point, down)) =
            segment(UPPER_ARM_HEIGHTS, Vec3::new(side, -1.0, 0.0).normalize())
        else {
            return Self::default(side, height);
        };
        // The elbow: where the upper arm's axis comes closest to the
        // forearm's. Without a forearm (or parallel to it) the axis goes on
        // straight.
        let (elbow, fore) = segment(FOREARM_HEIGHTS, down)
            .and_then(|(centre, fore)| {
                let b = down.dot(fore);
                let w = point - centre;
                let parallel = 1.0 - b * b;
                (parallel > 1e-4).then(|| {
                    let t = (b * fore.dot(w) - down.dot(w)) / parallel;
                    (t.max(0.0), fore)
                })
            })
            .unwrap_or((0.0, down));
        let mut arm = Self {
            point,
            down,
            elbow,
            fore,
            reach: Vec::new(),
        };
        for p in positions {
            if let Some((t, r)) = arm.holds(*p, height) {
                let k = ((t + REACH_FROM) / PROFILE_STEP).round().max(0.0) as usize;
                if arm.reach.len() <= k {
                    arm.reach.resize(k + 1, 0.0);
                }
                if let Some(slot) = arm.reach.get_mut(k) {
                    *slot = slot.max(r);
                }
            }
        }
        arm
    }

    /// Where `p` is along the axis and how far from it, if it is part of
    /// this arm.
    fn holds(&self, p: Vec3, height: f32) -> Option<(f32, f32)> {
        let q = p - self.point;
        let t = q.dot(self.down);
        let (t, r) = if t <= self.elbow {
            (t, (q - self.down * t).length())
        } else {
            let q = q - self.down * self.elbow;
            let s = q.dot(self.fore).max(0.0);
            (self.elbow + s, (q - self.fore * s).length())
        };
        (r < ARM_RADIUS_M && p.y < SHOULDER * height).then_some((t, r))
    }

    /// How far the arm reaches from its axis between `t0` and `t1` along
    /// it (m).
    fn reach_between(&self, t0: f32, t1: f32) -> f32 {
        let index = |t: f32| ((t + REACH_FROM) / PROFILE_STEP).round().max(0.0) as usize;
        let measured = self
            .reach
            .get(index(t0.min(t1))..=index(t0.max(t1)).min(self.reach.len().saturating_sub(1)))
            .map_or(0.0, |rows| rows.iter().copied().fold(0.0, f32::max));
        if measured > 0.0 {
            measured
        } else {
            DEFAULT_ARM_RADIUS_M
        }
    }
}

impl Form {
    pub fn new(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let arms = [1.0, -1.0].map(|side| Arm::find(&positions, height, side));
        let steps = (height / PROFILE_STEP).ceil() as usize;
        let mut radial = vec![[0.0f32; ANGLE_BINS]; steps + 1];
        for p in &positions {
            let r = Vec2::new(p.x, p.z).length();
            let k = (p.y / PROFILE_STEP).round().max(0.0) as usize;
            let on_arm = arms.iter().any(|a| a.holds(*p, height).is_some());
            if r < ARM_REACH_M
                && !on_arm
                && let Some(row) = radial.get_mut(k)
            {
                let bin = &mut row[angle_bin(p.x.atan2(p.z))];
                *bin = bin.max(r);
            }
        }
        // Few vertices fall in each 1 cm × 10° cell: fill the gaps (max over
        // ±2 cm and ±30°), then smooth the steps (twice the mean of
        // neighbours), so the start pose is a smooth surface.
        let rows = radial.len();
        let at = |r: &[[f32; ANGLE_BINS]], k: usize, b: usize, dk: usize, db: usize| {
            let k = (k + dk).saturating_sub(2).min(rows.saturating_sub(1));
            r.get(k)
                .map_or(0.0, |row| row[(b + ANGLE_BINS + db - 3) % ANGLE_BINS])
        };
        let mut smooth: Vec<[f32; ANGLE_BINS]> = (0..rows)
            .map(|k| {
                std::array::from_fn(|b| {
                    (0..5)
                        .flat_map(|dk| (0..7).map(move |db| (dk, db)))
                        .map(|(dk, db)| at(&radial, k, b, dk, db))
                        .fold(0.0, f32::max)
                })
            })
            .collect();
        for _ in 0..2 {
            for row in &mut smooth {
                let r = *row;
                *row = std::array::from_fn(|b| {
                    (r[(b + ANGLE_BINS - 1) % ANGLE_BINS] + r[b] + r[(b + 1) % ANGLE_BINS]) / 3.0
                });
            }
        }
        let radial = smooth;
        let collider = StaticMesh::new(positions.clone(), triangles.clone(), BODY_VOXEL);
        Self {
            positions,
            triangles,
            radial,
            arms,
            collider,
        }
    }

    /// The project's avatar fitted to its measurements, if it has one.
    pub fn of_project(project: &Project) -> Result<Option<Self>, CommandError> {
        let Some(avatar) = project.avatar() else {
            return Ok(None);
        };
        let model = body_model()?;
        let positions = fitted_positions(model, &fit(model, &avatar.body));
        Ok(Some(Self::new(positions, model.triangles().to_vec())))
    }
}

/// How far out the body reaches at angle `theta`, anywhere between heights
/// `top` and `top - below` (m): a garment hangs outside everything above
/// it.
fn body_radius(form: Option<&Form>, top: f32, below: f32, theta: f32) -> f32 {
    let Some(form) = form else {
        return DEFAULT_BODY_RADIUS_M;
    };
    let last = form.radial.len().saturating_sub(1);
    let index = |y: f32| ((y / PROFILE_STEP).round().max(0.0) as usize).min(last);
    let bin = angle_bin(theta);
    form.radial
        .get(index(top - below)..=index(top))
        .map_or(0.0, |rows| {
            rows.iter().map(|row| row[bin]).fold(0.0, f32::max)
        })
}

/// Horizontal span of a closed polygon at height `y` (same units).
fn span_at(polygon: &[DVec2], y: f64) -> f64 {
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        if (a.y - y) * (b.y - y) <= 0.0 && a.y != b.y {
            let x = a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y);
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    if hi >= lo { hi - lo } else { 0.0 }
}

/// A placed panel, meshed, before it is positioned in 3D.
struct Laid {
    panel: PanelId,
    place: r3d_core::Placement,
    boundary: Vec<DVec2>,
    /// Index in `boundary` of each contour edge's first sample.
    starts: Vec<usize>,
    mesh: r3d_geom::mesh::PanelMesh,
    /// Pattern height of the panel's top (mm).
    top: f64,
}

impl Laid {
    /// Width of the panel `below` its top (m).
    fn width_below(&self, below: f32) -> f32 {
        (span_at(&self.boundary, self.top - f64::from(below) * 1000.0) / 1000.0) as f32
    }

    fn on_body(&self) -> bool {
        self.place.around == Around::Body
    }
}

/// The pattern abscissa of a laid panel that goes to its placement
/// angle (mm): the placement's, or the panel's horizontal centre.
fn centre_x(l: &Laid) -> f64 {
    if let Some(x) = l.place.centre_x_mm {
        return x;
    }
    let lo = l.mesh.vertices.iter().fold(f64::MAX, |m, v| m.min(v.x));
    let hi = l.mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.x));
    (lo + hi) / 2.0
}

/// Start positions of a panel wrapped around the body: its horizontal
/// centre goes to the placement angle, its top to the placement height;
/// distances along the pattern are kept as arc lengths. At each height the
/// radius is the larger of the body's (plus the gap) and the garment's own
/// (`around`: circumference at a height / 2π), so the panels start just
/// touching.
fn wrap_body(l: &Laid, form: Option<&Form>, around: &dyn Fn(f32) -> f32) -> Vec<Vec3> {
    let centre = centre_x(l);
    let gap = (l.place.offset_mm / 1000.0) as f32;
    let top_y = l.place.top_cm as f32 / 100.0;
    let angle0 = (l.place.angle_deg as f32).to_radians();
    l.mesh
        .vertices
        .iter()
        .map(|v| {
            let below = ((l.top - v.y) / 1000.0) as f32;
            let y = top_y - below;
            // Arc lengths are kept (no stretch in the start pose), so the
            // angle depends on the radius and the radius on the body's
            // reach at that angle: a couple of fixed-point steps settle it.
            let own = (around(y) / std::f32::consts::TAU).max(1e-3);
            let arc = ((v.x - centre) / 1000.0) as f32;
            let mut r = own;
            for _ in 0..2 {
                r = (body_radius(form, top_y, below, angle0 + arc / r) + gap).max(own);
            }
            let theta = angle0 + arc / r;
            Vec3::new(r * theta.sin(), y, r * theta.cos())
        })
        .collect()
}

/// Start positions of a panel wrapped around the arm on `side` (+1
/// left, −1 right), like a sleeve: its top on the arm's axis at the
/// placement height, running down the upper arm, then the forearm past
/// the elbow, its horizontal centre at the
/// placement angle (0 = the outside of the arm). The radius is the larger
/// of the arm's (plus the gap) and the sleeve's own (its width / 2π).
///
/// The pattern is seen from outside, as on the body: on the left arm its
/// right (+x) goes towards the back, on the right arm towards the front,
/// so a sleeve and its mirror copy dress both arms alike.
fn wrap_arm(l: &Laid, form: Option<&Form>, side: f32) -> Vec<Vec3> {
    let arm = match form.and_then(|f| f.arms.get(usize::from(side < 0.0))) {
        Some(a) => a.clone(),
        None => Arm::default(side, DEFAULT_HEIGHT_M),
    };
    // The directions out from the axis (away from the body) and across it
    // (towards the back on the left arm), for an axis going down `d`.
    let frame = |d: Vec3| {
        let out = (Vec3::X * side - d * (d.x * side)).normalize_or(Vec3::X * side);
        let front = (Vec3::Z - d * d.z - out * out.z).normalize_or(Vec3::Z);
        (out, -side * front)
    };
    let top_y = l.place.top_cm as f32 / 100.0;
    let t_top = if arm.down.y.abs() > 1e-3 {
        (top_y - arm.point.y) / arm.down.y
    } else {
        0.0
    };
    let gap = (l.place.offset_mm / 1000.0) as f32;
    let angle0 = (l.place.angle_deg as f32).to_radians();
    let centre = centre_x(l);
    l.mesh
        .vertices
        .iter()
        .map(|v| {
            let below = ((l.top - v.y) / 1000.0) as f32;
            let t = t_top + below;
            let own = (l.width_below(below) / std::f32::consts::TAU).max(1e-3);
            let r = (arm.reach_between(t - 0.02, t + 0.02) + gap).max(own);
            let theta = angle0 + ((v.x - centre) / 1000.0) as f32 / r;
            let (on_axis, d) = arm.axis(t);
            let (out, across) = frame(d);
            on_axis + (out * theta.cos() + across * theta.sin()) * r
        })
        .collect()
}

/// Start positions of a flat panel: a vertical sheet facing outward at the
/// placement angle, `offset_mm` from the body's axis, its top at the
/// placement height; seen from outside like the body's panels.
fn lay_flat(l: &Laid) -> Vec<Vec3> {
    let theta = (l.place.angle_deg as f32).to_radians();
    let out = Vec3::new(theta.sin(), 0.0, theta.cos());
    let across = Vec3::new(theta.cos(), 0.0, -theta.sin());
    let r = (l.place.offset_mm / 1000.0) as f32;
    let top_y = l.place.top_cm as f32 / 100.0;
    let centre = centre_x(l);
    l.mesh
        .vertices
        .iter()
        .map(|v| {
            let below = ((l.top - v.y) / 1000.0) as f32;
            out * r + across * ((v.x - centre) / 1000.0) as f32 + Vec3::Y * (top_y - below)
        })
        .collect()
}

pub fn fabric_preset(f: Fabric) -> FabricPreset {
    match f {
        Fabric::Cotton => FabricPreset::Cotton,
        Fabric::Denim => FabricPreset::Denim,
        Fabric::Jersey => FabricPreset::Jersey,
        Fabric::Silk => FabricPreset::Silk,
        Fabric::Wool => FabricPreset::Wool,
    }
}

fn internal(detail: impl Into<String>) -> CommandError {
    CommandError::Internal {
        detail: detail.into(),
    }
}

/// Number of boundary segments for each edge of the placed panels.
fn segment_counts(project: &Project, spacing: f64) -> BTreeMap<EdgeId, usize> {
    let count = |len: f64| ((len / spacing).ceil() as usize).max(1);
    let mut counts = BTreeMap::new();
    for panel in project.panels().values().filter(|p| p.placement.is_some()) {
        for e in &panel.contour {
            counts.insert(*e, count(project.edge_length(*e).unwrap_or_default()));
        }
    }
    // Both sides of a seam share the larger count.
    for seam in project.seams().values() {
        if let (Some(a), Some(b)) = (counts.get(&seam.a.edge), counts.get(&seam.b.edge)) {
            let n = (*a).max(*b);
            counts.insert(seam.a.edge, n);
            counts.insert(seam.b.edge, n);
        }
    }
    counts
}

/// Build the scene. Panels without a placement are left out.
pub fn build(
    project: &Project,
    form: Option<&Form>,
    spacing_mm: f64,
) -> Result<Drape, CommandError> {
    let spacing = spacing_mm.max(2.0);
    let counts = segment_counts(project, spacing);
    let garment = project
        .default_garment()
        .and_then(|g| project.garments().get(&g));
    let fabric = fabric_preset(garment.map(|g| g.fabric).unwrap_or_default()).params();

    let mut scene = SimScene {
        fabrics: vec![fabric],
        ground: true,
        ..SimScene::default()
    };
    let mut pieces = Vec::new();
    // Particle indices along each edge, start to end inclusive.
    let mut edge_vertices: BTreeMap<EdgeId, Vec<u32>> = BTreeMap::new();

    // Mesh every placed panel first: the starting radius depends on the
    // width of all of them.
    let mut laid = Vec::new();
    for (panel_id, panel) in project.panels() {
        let Some(place) = panel.placement else {
            continue;
        };
        let mut boundary: Vec<DVec2> = Vec::new();
        let mut starts = Vec::with_capacity(panel.contour.len());
        for e in &panel.contour {
            let geom = project
                .edge_geometry(*e)
                .ok_or_else(|| internal(format!("{e} has no geometry")))?;
            let len = geom.length();
            let n = counts.get(e).copied().unwrap_or(1);
            starts.push(boundary.len());
            for k in 0..n {
                boundary.push(geom.point_at_length(len * k as f64 / n as f64).0);
            }
        }
        let mesh =
            mesh_polygon(&boundary, spacing).map_err(|e| internal(format!("{panel_id}: {e}")))?;
        let top = mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.y));
        laid.push(Laid {
            panel: *panel_id,
            place,
            boundary,
            starts,
            mesh,
            top,
        });
    }
    // The garment's circumference at a height: all its body panels side by
    // side.
    let around = |y: f32| {
        laid.iter()
            .filter(|l| l.on_body())
            .map(|l| l.width_below(l.place.top_cm as f32 / 100.0 - y))
            .sum::<f32>()
    };

    for l in &laid {
        let Some(panel) = project.panel(l.panel) else {
            continue;
        };
        let mesh = &l.mesh;
        let offset = scene.particles.len() as u32;
        let total = l.boundary.len();
        for (e, start) in panel.contour.iter().zip(&l.starts) {
            let n = counts.get(e).copied().unwrap_or(1);
            let verts = (0..=n)
                .filter_map(|k| mesh.boundary.get((start + k) % total))
                .map(|b| offset + b)
                .collect();
            edge_vertices.insert(*e, verts);
        }

        let start = match l.place.around {
            Around::Body => wrap_body(l, form, &around),
            Around::LeftArm => wrap_arm(l, form, 1.0),
            Around::RightArm => wrap_arm(l, form, -1.0),
            Around::Flat => lay_flat(l),
        };
        scene.particles.extend(
            mesh.vertices
                .iter()
                .zip(start)
                .map(|(v, position)| Particle {
                    position,
                    rest: Vec2::new((v.x / 1000.0) as f32, (v.y / 1000.0) as f32),
                    pinned: false,
                }),
        );
        pieces.push(Piece {
            panel: l.panel,
            particles: offset as usize..scene.particles.len(),
            indices: mesh.triangles.iter().flatten().copied().collect(),
        });
        let grain = (panel.grain_angle_deg as f32).to_radians();
        let warp = Vec2::new(grain.cos(), grain.sin());
        scene
            .triangles
            .extend(mesh.triangles.iter().map(|t| Triangle {
                vertices: t.map(|i| offset + i),
                fabric: 0,
                warp,
            }));
    }

    for seam in project.seams().values() {
        let side = |s: &r3d_core::SeamSide| {
            edge_vertices.get(&s.edge).map(|v| {
                let mut v = v.clone();
                if s.reversed {
                    v.reverse();
                }
                v
            })
        };
        if let (Some(a), Some(b)) = (side(&seam.a), side(&seam.b)) {
            scene.seams.extend(
                a.iter()
                    .zip(&b)
                    .filter(|(x, y)| x != y)
                    .map(|(x, y)| [*x, *y]),
            );
        }
    }

    scene.collider = form.map(|f| f.collider.clone());
    // Pins: the cloth around each pin starts at the pin's place, keeping
    // its local shape, and does not move.
    let mut pins = Vec::new();
    let rest: Vec<Vec2> = scene.particles.iter().map(|p| p.rest).collect();
    for (id, pin) in project.pins() {
        let Some(piece) = pieces.iter().find(|p| p.panel == pin.panel) else {
            continue;
        };
        let held = pinned_by(piece, &rest, (pin.at / 1000.0).as_vec2());
        let Some(&center) = held.first() else {
            continue;
        };
        let origin = scene.particles[center].position;
        for i in &held {
            let p = &mut scene.particles[*i];
            p.position = pin.position.as_vec3() + (p.position - origin);
            p.pinned = true;
        }
        pins.push((*id, held));
    }
    let edges = edge_vertices
        .into_iter()
        .map(|(e, v)| (e, v.into_iter().map(|i| i as usize).collect()))
        .collect();
    Ok(Drape {
        scene,
        pieces,
        pins,
        edges,
    })
}

/// Cloth within this distance of a pin (on the pattern) is held (m).
pub const PIN_RADIUS_M: f32 = 0.03;

/// The particles of `piece` a pin at `at` (pattern, m) holds: those within
/// [`PIN_RADIUS_M`] on the pattern, the closest first. `rest` is every
/// particle's pattern position.
pub fn pinned_by(piece: &Piece, rest: &[Vec2], at: Vec2) -> Vec<usize> {
    let mut near: Vec<(f32, usize)> = piece
        .particles
        .clone()
        .filter_map(|i| Some((rest.get(i)?.distance(at), i)))
        .filter(|(d, _)| *d < PIN_RADIUS_M)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.into_iter().map(|(_, i)| i).collect()
}

/// Build the scene for the project as it is, at the default spacing.
pub fn prepare(project: &Project, form: Option<&Form>) -> Result<Drape, CommandError> {
    build(project, form, DEFAULT_SPACING_MM)
}

/// Where pattern points lie in a drape: for a point of a panel (pattern
/// coordinates, m), the triangle of its piece that contains it, or the
/// nearest one outside, and its barycentric weights there. A drape's 3D
/// position of the point is then the weighted sum of the corners.
#[derive(Debug)]
pub struct Locator<'a> {
    rest: Vec<Vec2>,
    pieces: Vec<(&'a Piece, PieceGrid)>,
}

/// A piece's triangles (global particle indices), binned by their pattern
/// bounding box so a point inside the shape only tests a few.
#[derive(Debug)]
struct PieceGrid {
    corners: Vec<[usize; 3]>,
    grid: HashMap<[i32; 2], Vec<usize>>,
}

fn cell(p: Vec2) -> [i32; 2] {
    (p / CARRY_CELL_M).floor().as_ivec2().to_array()
}

impl<'a> Locator<'a> {
    pub fn new(drape: &'a Drape) -> Self {
        let rest: Vec<Vec2> = drape.scene.particles.iter().map(|p| p.rest).collect();
        let pieces = drape
            .pieces
            .iter()
            .map(|piece| {
                let corners: Vec<[usize; 3]> = piece
                    .indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| t.map(|i| piece.particles.start + i as usize))
                    .collect();
                let mut grid: HashMap<[i32; 2], Vec<usize>> = HashMap::new();
                for (k, t) in corners.iter().enumerate() {
                    let pts: Vec<Vec2> = t.iter().filter_map(|i| rest.get(*i).copied()).collect();
                    let lo = pts.iter().fold(Vec2::MAX, |a, b| a.min(*b));
                    let hi = pts.iter().fold(Vec2::MIN, |a, b| a.max(*b));
                    let (a, b) = (cell(lo), cell(hi));
                    for x in a[0]..=b[0] {
                        for y in a[1]..=b[1] {
                            grid.entry([x, y]).or_default().push(k);
                        }
                    }
                }
                (piece, PieceGrid { corners, grid })
            })
            .collect();
        Self { rest, pieces }
    }

    /// The triangle of `panel`'s piece holding pattern point `p` (m), and
    /// the point's weights in it; `None` if the panel is not simulated.
    pub fn locate(&self, panel: PanelId, p: Vec2) -> Option<([usize; 3], Vec3)> {
        let (_, g) = self.pieces.iter().find(|(piece, _)| piece.panel == panel)?;
        let weights = |t: &[usize; 3]| {
            let [a, b, c] = t.map(|k| self.rest.get(k).copied());
            barycentric(p, a?, b?, c?)
        };
        // The triangle whose smallest weight is largest: the one containing
        // the point, or the closest one outside.
        let best_of = |candidates: &mut dyn Iterator<Item = &[usize; 3]>| {
            candidates
                .filter_map(|t| Some((*t, weights(t)?)))
                .max_by(|x, y| x.1.min_element().total_cmp(&y.1.min_element()))
        };
        let near = g.grid.get(&cell(p)).map(Vec::as_slice).unwrap_or_default();
        best_of(&mut near.iter().filter_map(|k| g.corners.get(*k)))
            .filter(|(_, w)| w.min_element() >= -1e-4)
            .or_else(|| best_of(&mut g.corners.iter()))
    }

    /// Where pattern point `p` of `panel` is in 3D, for the drape's
    /// particle `positions`.
    pub fn position(&self, panel: PanelId, p: Vec2, positions: &[Vec3]) -> Option<Vec3> {
        let (t, w) = self.locate(panel, p)?;
        let [a, b, c] = t.map(|k| positions.get(k).copied());
        Some(a? * w.x + b? * w.y + c? * w.z)
    }
}

/// Internal lines are sampled this often along the pattern (mm).
const LINE_STEP_MM: f64 = 10.0;

/// A line on the draped cloth: points with the cloth's normal there.
pub type ClothLine = Vec<(Vec3, Vec3)>;

/// Pattern lines where they are on a drape (style lines, the selected
/// edge), for drawing them on the garment.
#[derive(Debug)]
pub struct LinesOnCloth<'a> {
    drape: &'a Drape,
    locator: Locator<'a>,
    positions: &'a [Vec3],
    /// Cloth normal per particle.
    normals: Vec<Vec3>,
}

impl<'a> LinesOnCloth<'a> {
    /// For `drape` with its particles at `positions`.
    pub fn new(drape: &'a Drape, positions: &'a [Vec3]) -> Self {
        let mut normals = vec![Vec3::ZERO; positions.len()];
        for piece in &drape.pieces {
            for t in piece.indices.as_chunks::<3>().0 {
                let [a, b, c] = t.map(|i| piece.particles.start + i as usize);
                if let (Some(pa), Some(pb), Some(pc)) =
                    (positions.get(a), positions.get(b), positions.get(c))
                {
                    let n = (*pb - *pa).cross(*pc - *pa);
                    for i in [a, b, c] {
                        if let Some(slot) = normals.get_mut(i) {
                            *slot += n;
                        }
                    }
                }
            }
        }
        for n in &mut normals {
            *n = n.normalize_or(Vec3::Y);
        }
        Self {
            drape,
            locator: Locator::new(drape),
            positions,
            normals,
        }
    }

    fn normal(&self, i: usize) -> Vec3 {
        self.normals.get(i).copied().unwrap_or(Vec3::Y)
    }

    /// Every internal line of the simulated panels.
    pub fn internal_lines(&self, project: &Project) -> Vec<ClothLine> {
        project
            .internal_lines()
            .iter()
            .filter_map(|(id, line)| {
                let polyline = project.internal_line_polyline(*id)?;
                let mut points = Vec::new();
                for (a, b) in polyline.iter().zip(polyline.iter().skip(1)) {
                    let steps = ((a.distance(*b) / LINE_STEP_MM).ceil() as usize).max(1);
                    points.extend((0..steps).map(|k| a.lerp(*b, k as f64 / steps as f64)));
                }
                points.extend(polyline.last());
                let on: ClothLine = points
                    .iter()
                    .filter_map(|p| {
                        let (t, w) = self.locator.locate(line.panel, (*p / 1000.0).as_vec2())?;
                        let [a, b, c] = t.map(|k| self.positions.get(k).copied());
                        let at = a? * w.x + b? * w.y + c? * w.z;
                        let [na, nb, nc] = t.map(|k| self.normal(k));
                        Some((at, (na * w.x + nb * w.y + nc * w.z).normalize_or(Vec3::Y)))
                    })
                    .collect();
                (on.len() > 1).then_some(on)
            })
            .collect()
    }

    /// A contour edge of a simulated panel.
    pub fn edge(&self, edge: EdgeId) -> Option<ClothLine> {
        let particles = self.drape.edges.get(&edge)?;
        Some(
            particles
                .iter()
                .filter_map(|i| Some((*self.positions.get(*i)?, self.normal(*i))))
                .collect(),
        )
    }
}

/// Start `new` where `old` is now (`positions`): each particle takes the
/// position of the same pattern point in the old drape, interpolated in
/// the old triangle that contains it, or extrapolated from the nearest one
/// when the panel grew. Particles of new panels, and pinned ones, keep
/// their own start. This is how pattern edits reach a running simulation
/// without a jump (architecture §7.8).
pub fn carry_over(old: &Drape, positions: &[Vec3], new: &mut Drape) {
    let locator = Locator::new(old);
    for piece in &new.pieces {
        for i in piece.particles.clone() {
            let Some(p) = new.scene.particles.get_mut(i) else {
                continue;
            };
            if p.pinned {
                continue;
            }
            if let Some(at) = locator.position(piece.panel, p.rest, positions) {
                p.position = at;
            }
        }
    }
}

/// Bin size of the carry-over grid on the pattern (m).
const CARRY_CELL_M: f32 = 0.05;

/// Barycentric weights of `p` in triangle `abc` (2D); `None` if it is flat.
fn barycentric(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> Option<Vec3> {
    let (v0, v1, v2) = (b - a, c - a, p - a);
    let det = v0.perp_dot(v1);
    if det.abs() < 1e-12 {
        return None;
    }
    let v = v2.perp_dot(v1) / det;
    let w = v0.perp_dot(v2) / det;
    Some(Vec3::new(1.0 - v - w, v, w))
}

/// What a headless drape produced.
#[derive(Debug, Clone)]
pub struct Simulated {
    pub drape: Drape,
    pub form: Option<Form>,
    pub positions: Vec<Vec3>,
    pub frames: u32,
    pub ms_per_frame: f64,
    pub stats: StepStats,
}

/// Simulate `frames` frames of 1/60 s from the starting positions, on the
/// CPU backend: deterministic, so a cached drape can be reproduced.
pub fn simulate(project: &Project, frames: u32) -> Result<Simulated, CommandError> {
    let form = Form::of_project(project)?;
    let drape = prepare(project, form.as_ref())?;
    if drape.pieces.is_empty() {
        return Err(CommandError::NothingToSimulate);
    }
    let mut solver = CpuSolver::new(drape.scene.clone());
    let start = Instant::now();
    for _ in 0..frames {
        solver
            .step(1.0 / 60.0)
            .map_err(|e| internal(e.to_string()))?;
    }
    let ms_per_frame = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames.max(1));
    Ok(Simulated {
        positions: solver.positions().to_vec(),
        stats: solver.stats(),
        drape,
        form,
        frames,
        ms_per_frame,
    })
}

/// Where the last headless drape is cached, relative to the project folder.
pub const CACHE_FILE: &str = ".r3d-cache/drape.bin";
const CACHE_MAGIC: &[u8; 8] = b"R3DDRP01";

/// Fingerprint of everything a drape depends on: the document (schema
/// JSON), the mesh spacing and the solver's substeps. FNV-1a, stable
/// across runs and platforms.
pub fn fingerprint(project: &Project) -> Result<u64, CommandError> {
    let json = r3d_core::file::to_json(project).map_err(|e| internal(e.to_string()))?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let tail = [
        DEFAULT_SPACING_MM.to_le_bytes(),
        u64::from(SUBSTEPS).to_le_bytes(),
    ];
    for b in json.as_bytes().iter().chain(tail.iter().flatten()) {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    Ok(hash)
}

/// Encode a drape for [`CACHE_FILE`].
pub fn encode_cache(fingerprint: u64, positions: &[Vec3]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16 + positions.len() * 12);
    bytes.extend_from_slice(CACHE_MAGIC);
    bytes.extend_from_slice(&fingerprint.to_le_bytes());
    for p in positions {
        for c in p.to_array() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
    }
    bytes
}

/// The cached drape positions in the project folder `dir`, if they were
/// made from this exact document; `None` otherwise (stale, missing or
/// damaged cache: it is only a cache).
pub fn load_cache(dir: &Path, project: &Project, particles: usize) -> Option<Vec<Vec3>> {
    let bytes = std::fs::read(dir.join(CACHE_FILE)).ok()?;
    let (head, body) = bytes.split_at_checked(16)?;
    let (magic, print) = head.split_at(8);
    if magic != CACHE_MAGIC
        || print != fingerprint(project).ok()?.to_le_bytes()
        || body.len() != particles * 12
    {
        return None;
    }
    Some(
        body.as_chunks::<12>()
            .0
            .iter()
            .map(|c| {
                let (xyz, _) = c.as_chunks::<4>();
                let [x, y, z] =
                    [0, 1, 2].map(|i| xyz.get(i).map_or(0.0, |b| f32::from_le_bytes(*b)));
                Vec3::new(x, y, z)
            })
            .collect(),
    )
}

/// The garment draped at `positions` (one per particle of `drape`), as
/// meshes to export: one per piece, named after it, with its pattern
/// position as texture coordinates; the dress form too when `form` is
/// given.
pub fn scene3d(
    project: &Project,
    drape: &Drape,
    positions: &[Vec3],
    form: Option<&Form>,
) -> r3d_io::scene3d::Scene3d {
    use r3d_io::scene3d::{Material3d, Mesh3d, Scene3d};
    let garment = project
        .default_garment()
        .and_then(|id| project.garments().get(&id));
    let fabric = garment.map(|g| g.fabric).unwrap_or_default();
    let look = fabric.look();
    let rgb = garment.map_or(look.rgb, |g| g.rgb());
    let mut materials = vec![Material3d {
        name: format!("{fabric:?}"),
        rgb,
        sheen: look.sheen,
        sheen_roughness: look.sheen_roughness,
        roughness: look.roughness,
        double_sided: true,
    }];
    let pieces = drape
        .pieces
        .iter()
        .filter_map(|piece| {
            let at = positions.get(piece.particles.clone())?;
            let rest = drape.scene.particles.get(piece.particles.clone())?;
            Some(Mesh3d {
                name: project
                    .panel(piece.panel)
                    .map_or_else(|| piece.panel.to_string(), |p| p.name.clone()),
                positions: at.iter().map(|p| p.to_array()).collect(),
                uvs: rest.iter().map(|p| [p.rest.x, -p.rest.y]).collect(),
                indices: piece.indices.clone(),
                material: 0,
            })
        })
        .collect();
    let avatar = form.map(|f| {
        materials.push(Material3d {
            name: "Avatar".into(),
            rgb: 0xC9C6C1,
            sheen: 0.0,
            sheen_roughness: 0.0,
            roughness: 0.7,
            double_sided: false,
        });
        Mesh3d {
            name: "Avatar".into(),
            positions: f.positions.iter().map(|p| p.to_array()).collect(),
            uvs: Vec::new(),
            indices: f.triangles.iter().flatten().copied().collect(),
            material: 1,
        }
    });
    Scene3d {
        generator: format!("Rustle3D {}", env!("CARGO_PKG_VERSION")),
        name: project.name().to_owned(),
        pieces,
        avatar,
        materials,
    }
}
