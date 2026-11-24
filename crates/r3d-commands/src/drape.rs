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
//!
//! The avatar can move ([`Animation`]): its skeleton's motion is sampled
//! for the solver ([`Form::body_motion`]) and its skin posed for the views
//! ([`Form::posed`]).

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use glam::{Vec2, Vec3};
use r3d_avatar::captured::CapturedMotion;
use r3d_avatar::fit::{fitted_body, fitted_positions};
use r3d_avatar::motion::{EASE_IN_S, Motion, ease};
use r3d_avatar::pack::Influences;
use r3d_avatar::skeleton::{Skeleton, skin};
use r3d_core::seam::{Span, Track};
use r3d_core::{
    Around, ButtonId, DetailId, EdgeId, Fabric, GarmentId, PanelId, PinId, PointId, Project,
    SeamSide,
};
use r3d_geom::DVec2;
use r3d_geom::Rigid;
use r3d_geom::mesh::{FineZone, mesh_panel};
use r3d_sim::collide::{Skin, StaticMesh};
use r3d_sim::motion::BodyMotion;
use r3d_sim::scene::{Closure, Particle, Triangle};
use r3d_sim::{CpuSolver, ElasticLine, FabricPreset, SUBSTEPS, SimScene, Solver, StepStats};

mod clearance;

use crate::error::CommandError;
use crate::progress::{Progress, Status};
use crate::queries::{body_model, fitted};

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
    /// The elastic of each of `scene.elastics`, in its order.
    pub elastics: Vec<DetailId>,
    /// How far each particle stands out of the cloth where it is quilted
    /// (m): the padding's puff between stitched lines, drawn, not
    /// simulated (docs/physics.md, "Composite cloth").
    pub puff: Vec<f32>,
}

impl Drape {
    /// `positions` with the quilted cloth puffed out along its normals;
    /// the positions themselves when nothing is quilted.
    pub fn puffed(&self, positions: &[Vec3]) -> Vec<Vec3> {
        if self.puff.iter().all(|p| *p == 0.0) {
            return positions.to_vec();
        }
        let normals = vertex_normals(self, positions);
        positions
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let out = self.puff.get(i).copied().unwrap_or(0.0);
                *p + normals.get(i).copied().unwrap_or(Vec3::ZERO) * out
            })
            .collect()
    }

    /// Each elastic's length on the cloth at `positions` and its relaxed
    /// length (m).
    pub fn elastic_lengths(&self, positions: &[Vec3]) -> Vec<(DetailId, f32, f32)> {
        self.elastics
            .iter()
            .zip(&self.scene.elastics)
            .map(|(id, line)| {
                let mut on = 0.0;
                let mut rest = 0.0;
                for w in line.particles.windows(2) {
                    let &[a, b] = w else { continue };
                    let (pa, pb) = (positions.get(a as usize), positions.get(b as usize));
                    if let (Some(pa), Some(pb)) = (pa, pb) {
                        on += pa.distance(*pb);
                    }
                    let flat = |i: u32| self.scene.particles.get(i as usize).map(|p| p.rest);
                    if let (Some(ra), Some(rb)) = (flat(a), flat(b)) {
                        rest += ra.distance(rb) * line.ratio;
                    }
                }
                (*id, on, rest)
            })
            .collect()
    }
}

/// The particles and triangles of one panel, to draw it on its own.
#[derive(Debug, Clone)]
pub struct Piece {
    pub panel: PanelId,
    pub particles: Range<usize>,
    /// Triangle corners, indices local to `particles`.
    pub indices: Vec<u32>,
    /// Where the panel's outline corners are on the pattern (m, as the
    /// particles' rest positions): a piece moved on the board is told
    /// apart from a reshaped one when edits are carried over.
    pub corners: BTreeMap<PointId, Vec2>,
}

/// The dress form garments are draped on: the fitted body (m), its
/// profile, its arms and its distance field. Building one takes a
/// fraction of a second, so callers keep it for as long as the
/// measurements do not change.
#[derive(Debug, Clone)]
pub struct Form {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    /// The body's distance from its vertical axis, arms left out.
    radial: Profile,
    /// The left (+x) and right arms.
    arms: [Arm; 2],
    /// The left (+x) and right legs.
    legs: [Leg; 2],
    /// Height of the crotch (m): the lowest body point on the median
    /// plane, below which the legs are apart.
    crotch: f32,
    collider: StaticMesh,
    /// What moves the body, if it can move.
    rig: Option<Rig>,
    /// Heights (m) of the chest, waist and hips, for an imported body
    /// (its arrangement points); the built-in body's are found on it.
    pub levels: Option<[f32; 3]>,
}

/// Where a body's limbs are (m): each arm's shoulder, elbow and wrist,
/// each leg's thigh and shin middles, left (+x) first, and the height of
/// the crotch. Found on the body ([`Parts::find`]) or given by an
/// imported body's arrangement points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parts {
    pub arms: [[Vec3; 3]; 2],
    pub legs: [[Vec3; 2]; 2],
    pub crotch: f32,
}

/// A body's skeleton and skin weights.
#[derive(Debug, Clone)]
struct Rig {
    skeleton: Skeleton,
    weights: Vec<Influences>,
    /// The arms, kept clear of the body in every pose.
    arms: clearance::Arms,
}

/// The solver gets a motion sampled this many times a second.
const MOTION_FPS: f32 = 60.0;
/// A motion starts once the garment has closed and settled on the body:
/// not before this much draping (s).
pub const MOTION_START_S: f32 = 1.5;
/// A stopped motion brings the body back to rest over this time (s).
pub const STOP_S: f32 = 1.0;

/// The avatar's motion over simulated time.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub motion: Motion,
    /// When it starts (simulated time, s).
    pub start: f32,
    /// When it was stopped, if it was: the body then comes back to rest
    /// over [`STOP_S`]...
    pub stop: Option<f32>,
    /// ...and starts this motion, if one was chosen meanwhile.
    pub then: Option<Motion>,
}

impl Animation {
    /// `motion`, from [`MOTION_START_S`] of draping or `now`, whichever
    /// comes later.
    pub fn starting(motion: Motion, now: f32) -> Self {
        Self {
            motion,
            start: now.max(MOTION_START_S),
            stop: None,
            then: None,
        }
    }

    /// What the body does once `to` (`None`: stand still) is chosen at
    /// `now`, after `current`: a motion that has begun first comes back
    /// to rest, so the body never jumps.
    pub fn switch(current: Option<Self>, to: Option<Motion>, now: f32) -> Option<Self> {
        match current.and_then(|a| a.settled(now)) {
            None => to.map(|m| Self::starting(m, now)),
            Some(a) if a.stop.is_none() && to.as_ref() == Some(&a.motion) => Some(a),
            Some(a) if now < a.start => to.map(|m| Self::starting(m, now)),
            Some(a) => Some(Self {
                stop: Some(a.stop.unwrap_or(now)),
                then: to,
                ..a
            }),
        }
    }

    /// The same animation seen from `now` on: once a stopped motion is
    /// back at rest, the next one (or nothing).
    pub fn settled(self, now: f32) -> Option<Self> {
        match self.stop {
            Some(stop) if now >= stop + STOP_S => self.then.map(|m| Self {
                motion: m,
                start: stop + STOP_S,
                stop: None,
                then: None,
            }),
            _ => Some(self),
        }
    }

    /// The same animation for a drape started again from the beginning.
    pub fn restarted(self) -> Option<Self> {
        self.chosen().map(|m| Self::starting(m, 0.0))
    }

    /// The motion chosen last, if any: playing, or next after a stop.
    pub fn chosen(&self) -> Option<Motion> {
        match self.stop {
            None => Some(self.motion.clone()),
            Some(_) => self.then.clone(),
        }
    }
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
    /// `PROFILE_STEP` along it from `point - REACH_FROM * down`: the wrist
    /// is where it is smallest.
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
/// Where to look for the wrist: this far past the elbow along the
/// forearm (m; a forearm is 0.14–0.15 of stature).
const WRIST_SEARCH: (f32, f32) = (0.12, 0.32);
/// A forearm's length, as a fraction of stature.
const FOREARM: f32 = 0.145;
/// The shoulder joint's height, as a fraction of stature (just below
/// the acromion, 0.81).
const SHOULDER_JOINT: f32 = 0.79;
/// How far above the upper arm's middle the shoulder is put on a raised
/// arm (m).
const RAISED_SHOULDER_M: f32 = 0.1;
/// Cloth narrower than a leg starts this far out from it (m).
const LIMB_CLEARANCE_M: f32 = 0.005;
/// A leg's axis runs through the middle of the thigh and of the shin at
/// these fractions of stature: well below the crotch (about 0.46 of
/// stature; a male body's genitals reach down to 0.40) and above the
/// ankle.
const LEG_SLICES: (f32, f32) = (0.34, 0.10);
/// Half the height of a slice (m).
const LEG_SLICE_M: f32 = 0.015;
/// Leg reach assumed without an avatar (m).
const DEFAULT_LEG_RADIUS_M: f32 = 0.07;
/// Cloth around a leg starts at least this far from the body's median
/// plane (m), so the two trouser legs start apart, the left on the left.
const MEDIAN_GAP_M: f32 = 0.006;
/// Around a leg, the cloth clears the body this far above and below its
/// own height (m).
const LEG_REACH_SPAN_M: f32 = 0.03;
/// The crotch as a fraction of stature, without an avatar; on one, the
/// lowest body point within `MEDIAN_POINT_M` of the median plane above
/// `CROTCH_ABOVE` of stature (the feet touch the plane in some poses).
const CROTCH_FRACTION: f32 = 0.46;
const MEDIAN_POINT_M: f32 = 0.01;
const CROTCH_ABOVE: f32 = 0.3;
/// A trouser leg goes from around the leg to around the body over this
/// height above the crotch (m).
const CROTCH_BLEND_M: f32 = 0.08;

/// How far a body part reaches from a vertical axis (m): every
/// `PROFILE_STEP` from the floor and every 360° / `ANGLE_BINS` around
/// (angle 0 = front, as placements count).
#[derive(Debug, Clone, Default)]
struct Profile(Vec<[f32; ANGLE_BINS]>);

impl Profile {
    /// The profile of `points`, each a height and an offset (x, z) from
    /// the axis at that height.
    fn new(height: f32, points: impl Iterator<Item = (f32, Vec2)>) -> Self {
        let steps = (height / PROFILE_STEP).ceil() as usize;
        let mut radial = vec![[0.0f32; ANGLE_BINS]; steps + 1];
        for (y, d) in points {
            let k = (y / PROFILE_STEP).round().max(0.0) as usize;
            if let Some(row) = radial.get_mut(k) {
                let bin = &mut row[angle_bin(d.x.atan2(d.y))];
                *bin = bin.max(d.length());
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
        Self(smooth)
    }

    /// How far out the profile reaches at angle `theta`, anywhere between
    /// heights `top` and `top - below` (m): a garment hangs outside
    /// everything above it.
    fn reach(&self, top: f32, below: f32, theta: f32) -> f32 {
        let last = self.0.len().saturating_sub(1);
        let index = |y: f32| ((y / PROFILE_STEP).round().max(0.0) as usize).min(last);
        let bin = angle_bin(theta);
        self.0
            .get(index(top - below)..=index(top))
            .map_or(0.0, |rows| {
                rows.iter().map(|row| row[bin]).fold(0.0, f32::max)
            })
    }
}

/// A leg: a straight axis from the middle of the thigh to the middle of
/// the shin, carried on up to the waist, and how far that half of the
/// body reaches from it.
#[derive(Debug, Clone)]
struct Leg {
    /// A point on the axis (m).
    point: Vec3,
    /// Unit direction down the leg.
    down: Vec3,
    /// The body on this leg's side, around the axis.
    profile: Profile,
}

impl Leg {
    /// The leg on `side` (+1 left, −1 right) of a body of `height` (m),
    /// without an avatar.
    fn default(side: f32, height: f32) -> Self {
        Self {
            point: Vec3::new(side * 0.06 * height, 0.36 * height, 0.0),
            down: Vec3::new(side * 0.07, -1.0, 0.0).normalize(),
            profile: Profile::default(),
        }
    }

    /// The axis at height `y`.
    fn centre(&self, y: f32) -> Vec3 {
        if self.down.y.abs() < 1e-3 {
            return self.point;
        }
        self.point + self.down * ((y - self.point.y) / self.down.y)
    }

    /// The leg on `side` through the middles of its thigh and its shin
    /// (an imported body's arrangement points); `body` as for
    /// [`Leg::find`].
    fn through([thigh, shin]: [Vec3; 2], body: &[Vec3], height: f32, side: f32) -> Self {
        let mut leg = Self {
            point: thigh,
            down: (shin - thigh).normalize_or(Vec3::NEG_Y),
            profile: Profile::default(),
        };
        leg.profile = leg.profile_of(body, height, side);
        leg
    }

    /// The body on this leg's side around its axis.
    fn profile_of(&self, body: &[Vec3], height: f32, side: f32) -> Profile {
        let half = body.iter().filter(|p| side * p.x > 0.0).map(|p| {
            let c = self.centre(p.y);
            (p.y, Vec2::new(p.x - c.x, p.z - c.z))
        });
        Profile::new(height, half)
    }

    /// Find the leg on `side` of a body; `body` holds the points that are
    /// not arms or hands. The middle of a slice is the middle of its
    /// bounding box: the vertices are not spread evenly around a leg.
    fn find(body: &[Vec3], height: f32, side: f32) -> Self {
        let middle = |fraction: f32| {
            let y = fraction * height;
            let (lo, hi) = body
                .iter()
                .filter(|p| side * p.x > 0.0 && (p.y - y).abs() < LEG_SLICE_M)
                .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
                    (lo.min(*p), hi.max(*p))
                });
            (lo.x <= hi.x).then(|| Vec3::new((lo.x + hi.x) / 2.0, y, (lo.z + hi.z) / 2.0))
        };
        let (Some(thigh), Some(shin)) = (middle(LEG_SLICES.0), middle(LEG_SLICES.1)) else {
            return Self::default(side, height);
        };
        let mut leg = Self {
            point: thigh,
            down: (shin - thigh).normalize_or(Vec3::NEG_Y),
            profile: Profile::default(),
        };
        leg.profile = leg.profile_of(body, height, side);
        leg
    }

    /// How far this half of the body reaches from the axis at `theta`
    /// near height `y` (m).
    fn reach(&self, y: f32, theta: f32) -> f32 {
        let r = self
            .profile
            .reach(y + LEG_REACH_SPAN_M, 2.0 * LEG_REACH_SPAN_M, theta);
        if r > 0.0 { r } else { DEFAULT_LEG_RADIUS_M }
    }
}

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

    /// The arm through a shoulder, an elbow and a wrist (an imported
    /// body's arrangement points).
    fn through([shoulder, elbow, wrist]: [Vec3; 3], positions: &[Vec3], height: f32) -> Self {
        let mut arm = Self {
            point: shoulder,
            down: (elbow - shoulder).normalize_or(Vec3::NEG_Y),
            elbow: shoulder.distance(elbow),
            fore: Vec3::NEG_Y,
            reach: Vec::new(),
        };
        arm.fore = (wrist - elbow).normalize_or(arm.down);
        arm.measure(positions, height);
        arm
    }

    /// The shoulder, elbow and wrist of an arm found on a body: the axis
    /// where it reaches the shoulder's height (or a little above its
    /// upper arm when the arm is raised), the elbow, and the middle of
    /// the wrist (or a forearm's length past the elbow).
    fn joints(&self, positions: &[Vec3], height: f32) -> [Vec3; 3] {
        let up = if self.down.y < -0.3 {
            (SHOULDER_JOINT * height - self.point.y) / self.down.y
        } else {
            -RAISED_SHOULDER_M
        };
        let elbow = self.point + self.down * self.elbow;
        let wrist = self
            .wrist(positions, height)
            .unwrap_or(elbow + self.fore * FOREARM * height);
        [self.point + self.down * up, elbow, wrist]
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
        arm.measure(positions, height);
        // The forearm's band is short and thick: its principal axis can be
        // off by tens of degrees, and the reach measured from it then
        // makes the wrist look as thick as the elbow. Aim the forearm from
        // the elbow at the middle of the wrist (its thinnest part) instead.
        if let Some(wrist) = arm.wrist(positions, height) {
            let elbow_point = arm.point + arm.down * arm.elbow;
            arm.fore = (wrist - elbow_point).normalize_or(arm.fore);
            arm.measure(positions, height);
        }
        arm
    }

    /// Tabulate how far the arm reaches from its axis.
    fn measure(&mut self, positions: &[Vec3], height: f32) {
        self.reach.clear();
        for p in positions {
            if let Some((t, r)) = self.holds(*p, height) {
                let k = ((t + REACH_FROM) / PROFILE_STEP).round().max(0.0) as usize;
                if self.reach.len() <= k {
                    self.reach.resize(k + 1, 0.0);
                }
                if let Some(slot) = self.reach.get_mut(k) {
                    *slot = slot.max(r);
                }
            }
        }
    }

    /// The middle of the wrist: of the arm's points around the thinnest
    /// part of the forearm (`WRIST_SEARCH` past the elbow), the bounding
    /// box's centre.
    fn wrist(&self, positions: &[Vec3], height: f32) -> Option<Vec3> {
        let index = |t: f32| ((t + REACH_FROM) / PROFILE_STEP).round().max(0.0) as usize;
        let (from, to) = WRIST_SEARCH;
        let (k, _) = (index(self.elbow + from)..=index(self.elbow + to))
            .filter_map(|k| Some((k, *self.reach.get(k)?)))
            .filter(|(_, r)| *r > 0.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        let t = k as f32 * PROFILE_STEP - REACH_FROM;
        let (lo, hi) = positions
            .iter()
            .filter(|p| {
                self.holds(**p, height)
                    .is_some_and(|(s, _)| (s - t).abs() < PROFILE_STEP * 1.5)
            })
            .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            });
        (lo.x <= hi.x).then(|| (lo + hi) / 2.0)
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
}

/// The body's points that are not arms or hands (the torso and the legs).
fn torso_and_legs(positions: &[Vec3], arms: &[Arm; 2], height: f32) -> Vec<Vec3> {
    positions
        .iter()
        .copied()
        .filter(|p| {
            Vec2::new(p.x, p.z).length() < ARM_REACH_M
                && !arms.iter().any(|a| a.holds(*p, height).is_some())
        })
        .collect()
}

/// The height of the crotch: the lowest body point on the median plane,
/// above the feet.
fn crotch_of(body: &[Vec3], height: f32) -> f32 {
    body.iter()
        .filter(|p| p.x.abs() < MEDIAN_POINT_M && p.y > CROTCH_ABOVE * height)
        .map(|p| p.y)
        .fold(CROTCH_FRACTION * height, f32::min)
}

impl Parts {
    /// The limbs of a body standing with its arms down or in an A pose
    /// (positions in m, feet at y = 0, facing +Z): the guess for an
    /// imported body's arrangement points.
    pub fn find(positions: &[Vec3]) -> Self {
        let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let arms = [1.0, -1.0].map(|side| Arm::find(positions, height, side));
        let body = torso_and_legs(positions, &arms, height);
        let legs = [1.0, -1.0].map(|side| {
            let leg = Leg::find(&body, height, side);
            [leg.point, leg.centre(LEG_SLICES.1 * height)]
        });
        Self {
            arms: arms.map(|a| a.joints(positions, height)),
            legs,
            crotch: crotch_of(&body, height),
        }
    }
}

impl Form {
    /// A dress form that stands still.
    pub fn new(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        Self::with_rig(positions, triangles, None)
    }

    /// A dress form for an imported body: its limbs where `parts` says,
    /// its chest, waist and hips at `levels` (m); it does not move.
    pub fn imported(
        positions: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
        parts: &Parts,
        levels: [f32; 3],
    ) -> Self {
        let mut form = Self::build(positions, triangles, None, Some(parts));
        form.levels = Some(levels);
        form
    }

    fn with_rig(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>, rig: Option<Rig>) -> Self {
        Self::build(positions, triangles, rig, None)
    }

    fn build(
        positions: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
        rig: Option<Rig>,
        parts: Option<&Parts>,
    ) -> Self {
        let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let arms = match parts {
            Some(p) => p
                .arms
                .map(|joints| Arm::through(joints, &positions, height)),
            None => [1.0, -1.0].map(|side| Arm::find(&positions, height, side)),
        };
        let body = torso_and_legs(&positions, &arms, height);
        let radial = Profile::new(height, body.iter().map(|p| (p.y, Vec2::new(p.x, p.z))));
        let legs = match parts {
            Some(p) => {
                let [left, right] = p.legs;
                [
                    Leg::through(left, &body, height, 1.0),
                    Leg::through(right, &body, height, -1.0),
                ]
            }
            None => [1.0, -1.0].map(|side| Leg::find(&body, height, side)),
        };
        let crotch = parts.map_or_else(|| crotch_of(&body, height), |p| p.crotch);
        let skin: Vec<Skin> = rig
            .iter()
            .flat_map(|r| &r.weights)
            .map(|w| Skin {
                bones: w.bones,
                weights: w.weights,
            })
            .collect();
        let collider =
            StaticMesh::with_skin(positions.clone(), triangles.clone(), BODY_VOXEL, skin);
        Self {
            positions,
            triangles,
            radial,
            arms,
            legs,
            crotch,
            collider,
            rig,
            levels: None,
        }
    }

    /// The body's skeleton and each vertex's skin weights; `None` for a
    /// body without a skeleton (an imported one).
    pub fn rig(&self) -> Option<(&Skeleton, &[Influences])> {
        self.rig
            .as_ref()
            .map(|r| (&r.skeleton, r.weights.as_slice()))
    }

    /// Each bone's motion at `time` (simulated, s) of `animation`; `None`
    /// for a form without a skeleton.
    pub fn transforms(&self, animation: &Animation, time: f32) -> Option<Vec<Rigid>> {
        let rig = self.rig.as_ref()?;
        let since = |t: f32| (t - animation.start).max(0.0);
        let pose = match animation.stop {
            Some(stop) if time >= stop => match &animation.then {
                Some(next) if time >= stop + STOP_S => {
                    next.pose(&rig.skeleton, time - stop - STOP_S)
                }
                _ => {
                    let left = animation.motion.pose(&rig.skeleton, since(stop));
                    left.scaled(1.0 - ease((time - stop) / STOP_S))
                }
            },
            _ => animation.motion.pose(&rig.skeleton, since(time)),
        };
        let mut pose = pose;
        rig.arms.clear(&rig.skeleton, &self.collider, &mut pose);
        Some(rig.skeleton.transforms(&pose))
    }

    /// The body's surface at `time` of `animation` (at rest without one).
    pub fn posed(&self, animation: Option<&Animation>, time: f32) -> Vec<Vec3> {
        match (animation.and_then(|a| self.transforms(a, time)), &self.rig) {
            (Some(t), Some(rig)) => skin(&self.positions, &rig.weights, &t),
            _ => self.positions.clone(),
        }
    }

    /// `animation` as the solver runs it, sampled from where it starts (or
    /// was stopped) on; `None` for a form without a skeleton.
    pub fn body_motion(&self, animation: &Animation) -> Option<BodyMotion> {
        let frames = |from: f32, seconds: f32| -> Option<Vec<Vec<Rigid>>> {
            (0..(seconds * MOTION_FPS).round() as usize)
                .map(|k| self.transforms(animation, from + k as f32 / MOTION_FPS))
                .collect()
        };
        let (start, intro, cycle) = match (animation.stop, &animation.then) {
            // Back to rest, then the next motion.
            (Some(stop), Some(next)) => (
                stop,
                frames(stop, STOP_S + EASE_IN_S)?,
                frames(stop + STOP_S + EASE_IN_S, next.period())?,
            ),
            // Back to rest, then still (the last sample holds).
            (Some(stop), None) => (stop, frames(stop, STOP_S + 1.0 / MOTION_FPS)?, Vec::new()),
            (None, _) => (
                animation.start,
                frames(animation.start, EASE_IN_S)?,
                frames(animation.start + EASE_IN_S, animation.motion.period())?,
            ),
        };
        Some(BodyMotion {
            start,
            fps: MOTION_FPS,
            intro,
            cycle,
        })
    }

    /// The project's avatar fitted to its measurements, if it has one
    /// (in the shown size's).
    /// An imported body (spec 27) is draped as it is, in every size.
    pub fn of_project(project: &Project) -> Result<Option<Self>, CommandError> {
        if let Some(mesh) = project.avatar().and_then(|a| a.mesh.as_ref()) {
            return Ok(Some(crate::bodies::form(mesh)?.as_ref().clone()));
        }
        project
            .shown_avatar()
            .map(|a| Self::of_body(&a.body, &a.measures))
            .transpose()
    }

    /// The built-in body fitted to `body` and the measurements typed
    /// besides, with its skeleton (seconds the first time).
    pub fn of_body(
        body: &r3d_core::Body,
        typed: &r3d_core::Measures,
    ) -> Result<Self, CommandError> {
        let model = body_model()?;
        let shaped = fitted_body(model, &fitted(body, typed)?);
        let skeleton = Skeleton::new(model, &shaped);
        let arms = clearance::Arms::new(&skeleton, &shaped.positions, model.weights());
        let rig = Rig {
            skeleton,
            weights: model.weights().to_vec(),
            arms,
        };
        Ok(Self::with_rig(
            shaped.positions,
            model.triangles().to_vec(),
            Some(rig),
        ))
    }
}

/// How much lower a panel on an arm is placed per metre further down the
/// arm: `wrap_arm` measures along the upper arm's axis from the height
/// it is given, so a cuff below a sleeve `len` long starts `len × drop`
/// below the sleeve's top.
pub(crate) fn arm_drop(project: &Project) -> Result<f64, CommandError> {
    if let Some(mesh) = project.avatar().and_then(|a| a.mesh.as_ref()) {
        let form = crate::bodies::form(mesh)?;
        return Ok(form.arms.first().map_or(0.8, |a| f64::from(-a.down.y)));
    }
    let arm = match project.avatar() {
        Some(avatar) => {
            let model = body_model()?;
            let positions = fitted_positions(model, &fitted(&avatar.body, &avatar.measures)?);
            let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
            Arm::find(&positions, height, 1.0)
        }
        None => Arm::default(1.0, DEFAULT_HEIGHT_M),
    };
    Ok(f64::from(-arm.down.y))
}

/// What a garment's pieces start around: the body, how many garments
/// are worn under it, and whether others are draped with it.
struct Under<'a> {
    form: Option<&'a Form>,
    depth: usize,
    /// How many of the garments under it have sleeves: only those push a
    /// sleeve out (a sleeve started 25 mm wide of a bare arm slid up it).
    sleeves_under: usize,
    /// More than one garment is draped. Pieces around the body then start
    /// just outside it near their own height rather than at their
    /// placement offset outside everything above them: both open gaps at
    /// the seams (the panels keep their widths), and a seam closing across
    /// a gap cuts straight through the other garments.
    several: bool,
    /// How far the garments under it start from the body's axis (their
    /// pieces around the body and the legs). Loose trousers start wider
    /// than the body plus a layer gap at the front and back of the hips,
    /// and a T-shirt started inside them there.
    worn: Profile,
}

/// A garment starts this much farther out per garment worn under it (m),
/// so the layers start in order. Layers touch through thin contacts
/// (`r3d_sim::layers`) and settle as close as their cloth is thick; the
/// gap is room for the first frames, when the seams close at up to
/// 20 m/s: at 10 mm, more than twice as much of a tucked shirt started
/// outside the trousers (docs/physics.md, Several garments).
const LAYER_GAP_M: f32 = 0.025;

impl Under<'_> {
    /// How far out the body reaches at angle `theta`, anywhere between
    /// heights `top` and `top - below` (m).
    fn radius(&self, top: f32, below: f32, theta: f32) -> f32 {
        self.form
            .map_or(DEFAULT_BODY_RADIUS_M, |f| f.radial.reach(top, below, theta))
    }

    /// The leg on `side` (+1 left).
    fn leg(&self, side: f32) -> Option<&Leg> {
        self.form.and_then(|f| f.legs.get(usize::from(side < 0.0)))
    }

    /// How far out a piece starts at angle `theta` between heights `top`
    /// and `top - below` (m), `alone` being where it would start with
    /// nothing under it: a layer gap per garment under it farther out,
    /// and a layer gap outside where those garments start.
    fn outside(&self, alone: f32, top: f32, below: f32, theta: f32) -> f32 {
        let worn = self.worn.reach(top, below, theta);
        let worn = if worn > 0.0 { worn + LAYER_GAP_M } else { 0.0 };
        (alone + self.layered()).max(worn)
    }

    /// How much farther out than alone the garment starts (m).
    fn layered(&self) -> f32 {
        LAYER_GAP_M * self.depth as f32
    }
}

/// Leftmost and rightmost points of a closed polygon at height `y` (same
/// units), if it reaches that height.
fn span_ends(polygon: &[DVec2], y: f64) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        if (a.y - y) * (b.y - y) <= 0.0 && a.y != b.y {
            let x = a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y);
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    (hi >= lo).then_some((lo, hi))
}

/// Horizontal span of a closed polygon at height `y` (same units).
fn span_at(polygon: &[DVec2], y: f64) -> f64 {
    span_ends(polygon, y).map_or(0.0, |(lo, hi)| hi - lo)
}

/// A placed panel, meshed, before it is positioned in 3D.
struct Laid {
    panel: PanelId,
    /// Its garment's place among the garments simulated, next to the
    /// body first.
    layer: usize,
    place: r3d_core::Placement,
    boundary: Vec<DVec2>,
    /// The mesh vertices along each track of the panel (contour and hole
    /// edges, sewn internal lines), at its samples.
    tracks: BTreeMap<Track, Vec<u32>>,
    /// The mesh vertex of each button (end 0) and buttonhole (end 1) on
    /// the panel.
    trims: BTreeMap<(ButtonId, usize), u32>,
    mesh: r3d_geom::mesh::PanelMesh,
    /// The mesh vertices along each stitched line of its quiltings.
    quilted: Vec<Vec<u32>>,
    /// How far each mesh vertex puffs out (m).
    puff: Vec<f32>,
    /// Pattern height of the panel's top (mm).
    top: f64,
}

impl Laid {
    /// Width of the panel `below` its top (m).
    fn width_below(&self, below: f32) -> f32 {
        (span_at(&self.boundary, self.top - f64::from(below) * 1000.0) / 1000.0) as f32
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
fn wrap_body(l: &Laid, under: &Under, around: &dyn Fn(f32) -> f32) -> Vec<Vec3> {
    let centre = centre_x(l);
    let gap = if under.several {
        LIMB_CLEARANCE_M
    } else {
        (l.place.offset_mm / 1000.0) as f32
    };
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
            // Alone, a garment starts outside everything above it (a skirt
            // over the hips); among others, outside the body near its own
            // height, as below the shoulders that is wide of the body and
            // the side seams then start far apart.
            let (from, span) = if under.several {
                (y + LEG_REACH_SPAN_M, 2.0 * LEG_REACH_SPAN_M)
            } else {
                (top_y, below)
            };
            for _ in 0..2 {
                let theta = angle0 + arc / r;
                let alone = (under.radius(from, span, theta) + gap).max(own);
                r = under.outside(alone, from, span, theta);
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
/// placement angle (0 = the outside of the arm). The radius is the
/// sleeve's own (its width / 2π): it starts as a closed tube, and where
/// the arm is thicker the collisions push it out. Starting outside the
/// arm's reach (as on the body, or with only a small gap) opened a narrow
/// sleeve or cuff into a part of a tube, which its seam then closed across
/// the arm, catching a fold; the reach, the farthest point from an axis
/// that is not quite centred, also overstates the arm near the elbow and
/// the wrist.
///
/// The pattern is seen from outside, as on the body: on the left arm its
/// right (+x) goes towards the back, on the right arm towards the front,
/// so a sleeve and its mirror copy dress both arms alike.
fn wrap_arm(l: &Laid, under: &Under, side: f32) -> Vec<Vec3> {
    let arm = match under.form.and_then(|f| f.arms.get(usize::from(side < 0.0))) {
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
    let angle0 = (l.place.angle_deg as f32).to_radians();
    let centre = centre_x(l);
    l.mesh
        .vertices
        .iter()
        .map(|v| {
            let below = ((l.top - v.y) / 1000.0) as f32;
            let t = t_top + below;
            let r = (l.width_below(below) / std::f32::consts::TAU).max(1e-3)
                + LAYER_GAP_M * under.sleeves_under as f32;
            let theta = angle0 + ((v.x - centre) / 1000.0) as f32 / r;
            let (on_axis, d) = arm.axis(t);
            let (out, across) = frame(d);
            on_axis + (out * theta.cos() + across * theta.sin()) * r
        })
        .collect()
}

/// Start positions of a panel wrapped around the leg on `side` (+1 left,
/// −1 right), like a trouser leg. Below the crotch, as a sleeve: around
/// the leg's axis, the piece's crease (`centre_x_mm`) at the placement
/// angle, the leg's pieces a closed tube unless the leg reaches farther
/// (then just outside it). Above the crotch, around the body: the
/// piece's inner edge (centre front or back) at the placement angle, the
/// leg's pieces sharing the half turn out to the side in proportion to
/// their widths, so their side seams meet; just outside the body, or at
/// the radius of both legs' pieces together. The two blend over
/// `CROTCH_BLEND_M`. Around the leg, the cloth stays on its side of the
/// median plane, pressed flat against it where it would cross, and above
/// the crotch in front of the body (or behind it): the two legs start
/// apart. Pushed out there below the crotch too, where nothing lies
/// between the legs, the back crotch point started 18 cm behind the
/// front one and their seam swept through the gap.
///
/// A piece's share of the half turn is measured against the leg's
/// circumference no higher than `rim`, the lowest top of the leg's
/// pieces: above the front's top, a back (which rises higher) is the
/// whole circumference, narrowing to nothing at its top, so its share
/// exploded there and its waist started in front of the body, then swept
/// round it (through a shirt tucked in).
fn wrap_leg(
    l: &Laid,
    under: &Under,
    side: f32,
    rim: f32,
    around_leg: &dyn Fn(f32) -> f32,
    around_hips: &dyn Fn(f32) -> f32,
) -> Vec<Vec3> {
    let default;
    let leg = match under.leg(side) {
        Some(leg) => leg,
        None => {
            default = Leg::default(side, DEFAULT_HEIGHT_M);
            &default
        }
    };
    let crotch = under
        .form
        .map_or(CROTCH_FRACTION * DEFAULT_HEIGHT_M, |f| f.crotch);
    let centre = centre_x(l);
    let top_y = l.place.top_cm as f32 / 100.0;
    let angle0 = (l.place.angle_deg as f32).to_radians();
    // Pattern +x runs towards the body's left at the front and its right
    // at the back: the inner edge lies on +x where that is the median.
    let inner_on_plus = side * angle0.cos() < 0.0;
    let outside = |y: f32, angle: f32| {
        let (top, span) = (y + LEG_REACH_SPAN_M, 2.0 * LEG_REACH_SPAN_M);
        under.outside(
            under.radius(top, span, angle) + LIMB_CLEARANCE_M,
            top,
            span,
            angle,
        )
    };
    l.mesh
        .vertices
        .iter()
        .map(|v| {
            let below = ((l.top - v.y) / 1000.0) as f32;
            let y = top_y - below;
            let on_leg = {
                let axis = leg.centre(y);
                let own = (around_leg(y) / std::f32::consts::TAU).max(1e-3);
                let arc = ((v.x - centre) / 1000.0) as f32;
                let mut r = own;
                for _ in 0..2 {
                    r = (leg.reach(y, angle0 + arc / r) + LIMB_CLEARANCE_M).max(own)
                        + under.layered();
                }
                let theta = angle0 + arc / r;
                let (x, z) = (axis.x + r * theta.sin(), axis.z + r * theta.cos());
                if side * x >= MEDIAN_GAP_M {
                    Vec3::new(x, y, z)
                } else if y < crotch {
                    // Nothing lies between the legs below the crotch.
                    Vec3::new(side * MEDIAN_GAP_M, y, z)
                } else if theta.cos() >= 0.0 {
                    Vec3::new(side * MEDIAN_GAP_M, y, z.max(outside(y, 0.0)))
                } else {
                    let back = outside(y, std::f32::consts::PI);
                    Vec3::new(side * MEDIAN_GAP_M, y, z.min(-back))
                }
            };
            let blend = ((y - crotch) / CROTCH_BLEND_M).clamp(0.0, 1.0);
            if blend <= 0.0 {
                return on_leg;
            }
            let inner = span_ends(&l.boundary, v.y)
                .map_or(centre, |(lo, hi)| if inner_on_plus { hi } else { lo });
            let own = (around_hips(y) / std::f32::consts::TAU).max(1e-3);
            let share = ((v.x - inner) / 1000.0) as f32 / around_leg(y.min(rim)).max(1e-3);
            let theta = angle0 + std::f32::consts::PI * share;
            let r = outside(y, theta).max(own);
            let on_body = Vec3::new(r * theta.sin(), y, r * theta.cos());
            on_leg.lerp(on_body, blend)
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

/// `n` segments shared among a seam side's stretches by their lengths,
/// at least one each (largest remainders).
fn split(n: usize, spans: &[Span]) -> Vec<usize> {
    let total: f64 = spans.iter().map(Span::length).sum();
    let spare = n.saturating_sub(spans.len());
    let share: Vec<f64> = spans
        .iter()
        .map(|s| {
            if total > 0.0 {
                spare as f64 * s.length() / total
            } else {
                0.0
            }
        })
        .collect();
    let mut out: Vec<usize> = share.iter().map(|x| 1 + x.floor() as usize).collect();
    let mut left = n.saturating_sub(out.iter().sum::<usize>());
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|a, b| {
        let frac = |i: &usize| share.get(*i).map_or(0.0, |x| x - x.floor());
        frac(b).total_cmp(&frac(a))
    });
    for i in order {
        if left == 0 {
            break;
        }
        if let Some(slot) = out.get_mut(i) {
            *slot += 1;
            left -= 1;
        }
    }
    out
}

/// Along an elastic the cloth is meshed this fine at most (mm), so its
/// gathers are small (docs/physics.md, "Elastic").
const ELASTIC_SPACING_MM: f64 = 8.0;
/// Rows of finer triangles on each side of an elastic.
const ELASTIC_ROWS: usize = 3;

/// `[from, to]` without the parts `taken` covers (sorted pieces).
fn subtract(from: f64, to: f64, taken: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut sorted: Vec<(f64, f64)> = taken.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut cursor = from;
    for (a, b) in sorted {
        if a > cursor {
            out.push((cursor, a.min(to)));
        }
        cursor = cursor.max(b);
        if cursor >= to {
            break;
        }
    }
    if cursor < to {
        out.push((cursor, to));
    }
    out.retain(|(a, b)| b - a > 1e-9);
    out
}

/// The cloth under topstitching reaches this far past its last row (mm).
const TOPSTITCH_BAND_EXTRA_MM: f64 = 2.0;

/// What changes the cloth of a triangle (spec 31): the details covering
/// it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Covers {
    topstitched: bool,
    interfacing: Option<(r3d_sim::InterfacingMake, r3d_sim::InterfacingWeight)>,
    /// A quilting's loft (0.1 mm) and padding (g/m²).
    quilted: Option<(u32, u32)>,
}

impl Covers {
    /// `base` changed by what covers it (docs/physics.md, "Composite
    /// cloth").
    fn fabric(self, base: r3d_sim::FabricParams) -> r3d_sim::FabricParams {
        let mut out = base;
        if let Some((loft, gsm)) = self.quilted {
            out = out.quilted(loft as f32 / 10.0, gsm as f32);
        }
        if let Some((make, weight)) = self.interfacing {
            out = out.fused(r3d_sim::fabric::interfacing(make, weight));
        }
        if self.topstitched {
            out = out.topstitched();
        }
        out
    }
}

/// The solver's interfacing for the document's.
fn interfacing_of(
    i: &r3d_core::Interfacing,
) -> (r3d_sim::InterfacingMake, r3d_sim::InterfacingWeight) {
    use r3d_core::{InterfacingFabric as F, InterfacingWeight as W};
    let make = match i.fabric {
        F::Woven => r3d_sim::InterfacingMake::Woven,
        F::NonWoven => r3d_sim::InterfacingMake::NonWoven,
        F::Knit => r3d_sim::InterfacingMake::Knit,
    };
    let weight = match i.weight {
        W::Light => r3d_sim::InterfacingWeight::Light,
        W::Medium => r3d_sim::InterfacingWeight::Medium,
        W::Heavy => r3d_sim::InterfacingWeight::Heavy,
    };
    (make, weight)
}

/// Where the details of `panel` change its cloth, on the pattern (mm).
struct PieceCovers {
    /// Each topstitch's side and how far from it its band reaches.
    stitched: Vec<(Vec<DVec2>, f64)>,
    /// Each interfacing's outline and what it is.
    interfaced: Vec<(
        Vec<DVec2>,
        (r3d_sim::InterfacingMake, r3d_sim::InterfacingWeight),
    )>,
    /// Each quilting's outline and its padding.
    quilted: Vec<(Vec<DVec2>, (u32, u32))>,
}

impl PieceCovers {
    fn of(project: &Project, panel: PanelId) -> Self {
        let quilted = project
            .quiltings()
            .filter(|(_, q)| q.panel == panel)
            .filter_map(|(_, q)| {
                let padding = (
                    (q.loft_mm * 10.0).round() as u32,
                    q.padding_gsm.round() as u32,
                );
                Some((project.quilting_outline(q)?, padding))
            })
            .collect();
        let interfaced = project
            .interfacings()
            .filter(|(_, i)| i.panel == panel)
            .filter_map(|(_, i)| Some((project.interfacing_outline(i)?, interfacing_of(i))))
            .collect();
        let stitched = project
            .topstitches()
            .filter(|(_, t)| project.side_panel(&t.side) == Some(panel))
            .filter_map(|(_, t)| {
                let line = project.side_polyline(&t.side)?;
                Some((line, t.reach_mm().abs() + TOPSTITCH_BAND_EXTRA_MM))
            })
            .collect();
        Self {
            stitched,
            interfaced,
            quilted,
        }
    }

    /// What covers the triangle with corners `corners` (mm).
    fn covers(&self, corners: [DVec2; 3]) -> Covers {
        let near = |line: &[DVec2], reach: f64| {
            corners.iter().any(|p| {
                line.windows(2).any(|w| match w {
                    [a, b] => r3d_geom::fit::distance_to_segment(*p, *a, *b) <= reach,
                    _ => false,
                })
            })
        };
        let centroid = (corners[0] + corners[1] + corners[2]) / 3.0;
        Covers {
            topstitched: self.stitched.iter().any(|(line, reach)| near(line, *reach)),
            interfacing: self
                .interfaced
                .iter()
                .find(|(outline, _)| r3d_geom::polygon::contains(outline, centroid))
                .map(|(_, what)| *what),
            quilted: self
                .quilted
                .iter()
                .find(|(outline, _)| r3d_geom::polygon::contains(outline, centroid))
                .map(|(_, padding)| *padding),
        }
    }
}

/// A quilted piece is meshed this many points per spacing of its lines.
const QUILT_POINTS_PER_SPACING: f64 = 4.0;

/// `line` with points added so none are more than `step` apart, every
/// point of it kept.
fn subdivided(line: &[DVec2], step: f64) -> Vec<DVec2> {
    let mut out = Vec::new();
    for w in line.windows(2) {
        if let [a, b] = w {
            let n = ((a.distance(*b) / step).ceil() as usize).max(1);
            out.extend((0..n).map(|k| a.lerp(*b, k as f64 / n as f64)));
        }
    }
    out.extend(line.last());
    out
}

/// A seam side's stretches with some length.
fn spans(project: &Project, side: &SeamSide) -> Option<Vec<Span>> {
    let spans: Vec<Span> = project
        .side_spans(side)?
        .into_iter()
        .filter(|s| s.length() > 1e-9)
        .collect();
    (!spans.is_empty()).then_some(spans)
}

/// Where each track of the placed panels is sampled for the mesh: arc
/// lengths from its start, both ends included. The two sides of a seam get
/// the same number of segments (the larger of what each needs), shared
/// among their stretches by length; the rest is cut at about `spacing`.
/// Contour and hole edges are tracks; internal lines are when sewn.
fn sample_tracks(project: &Project, spacing: f64) -> BTreeMap<Track, Vec<f64>> {
    let count = |len: f64| ((len / spacing).ceil() as usize).max(1);
    let fine = spacing.min(ELASTIC_SPACING_MM);
    // The stretches elastics cover, sampled finer.
    let mut elastic: BTreeMap<Track, Vec<(f64, f64)>> = BTreeMap::new();
    for (_, e) in project.elastics() {
        if project
            .side_panel(&e.side)
            .and_then(|p| project.panel(p))
            .is_none_or(|p| p.placement.is_none())
        {
            continue;
        }
        for span in spans(project, &e.side).unwrap_or_default() {
            elastic
                .entry(span.track)
                .or_default()
                .push((span.from, span.to));
        }
    }
    // Segments a stretch needs: finer where an elastic covers it.
    let count_span = |span: &Span| {
        let covered: f64 = elastic.get(&span.track).map_or(0.0, |list| {
            list.iter()
                .map(|(a, b)| (b.min(span.to) - a.max(span.from)).max(0.0))
                .sum()
        });
        let rest = (span.length() - covered).max(0.0);
        ((covered / fine + rest / spacing).ceil() as usize).max(1)
    };
    let placed = |panel: Option<PanelId>| {
        panel
            .and_then(|p| project.panel(p))
            .is_some_and(|p| p.placement.is_some())
    };
    let mut fixed: BTreeMap<Track, Vec<(f64, f64, usize)>> = BTreeMap::new();
    // Seams and zippers: their two sides meet point for point.
    let joins = project
        .seams()
        .values()
        .map(|s| (&s.a, &s.b))
        .chain(project.zippers().values().map(|z| (&z.a, &z.b)));
    for (side_a, side_b) in joins {
        if !(placed(project.side_panel(side_a)) && placed(project.side_panel(side_b))) {
            continue;
        }
        let (Some(a), Some(b)) = (spans(project, side_a), spans(project, side_b)) else {
            continue;
        };
        let need = |side: &[Span]| side.iter().map(count_span).sum::<usize>();
        let n = need(&a).max(need(&b));
        for side in [&a, &b] {
            for (span, k) in side.iter().zip(split(n, side)) {
                fixed
                    .entry(span.track)
                    .or_default()
                    .push((span.from, span.to, k));
            }
        }
    }
    // Elastics where seams have not set the samples already.
    for (track, list) in &elastic {
        let taken: Vec<(f64, f64)> = fixed
            .get(track)
            .map(|l| l.iter().map(|(a, b, _)| (*a, *b)).collect())
            .unwrap_or_default();
        for (from, to) in list {
            for (a, b) in subtract(*from, *to, &taken) {
                let k = (((b - a) / fine).ceil() as usize).max(1);
                fixed.entry(*track).or_default().push((a, b, k));
            }
        }
    }
    let mut tracks: Vec<Track> = Vec::new();
    for panel in project.panels().values().filter(|p| p.placement.is_some()) {
        for e in panel.contour.iter().chain(panel.holes.iter().flatten()) {
            tracks.push(Track::Edge(*e));
        }
    }
    tracks.extend(fixed.keys().filter(|t| matches!(t, Track::Line(_))));
    let mut out = BTreeMap::new();
    for track in tracks {
        let len = project.track_length(track).unwrap_or_default();
        let mut intervals = fixed.get(&track).cloned().unwrap_or_default();
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut positions = vec![0.0];
        let mut cursor = 0.0;
        let walk = |to: f64, n: usize, positions: &mut Vec<f64>, cursor: &mut f64| {
            let from = *cursor;
            for i in 1..=n {
                positions.push(from + (to - from) * i as f64 / n as f64);
            }
            *cursor = to;
        };
        for (from, to, k) in intervals {
            let from = from.max(cursor);
            if to <= from {
                continue;
            }
            if from - cursor > 1e-9 {
                walk(from, count(from - cursor), &mut positions, &mut cursor);
            }
            walk(to, k, &mut positions, &mut cursor);
        }
        if len - cursor > 1e-9 {
            walk(len, count(len - cursor), &mut positions, &mut cursor);
        }
        out.insert(track, positions);
    }
    out
}

/// The index of `s` among sorted sample positions.
fn sample_index(positions: &[f64], s: f64) -> Option<usize> {
    positions.iter().position(|p| (p - s).abs() < 1e-6)
}

/// Build the scene, in the project's shown size. Panels without a
/// placement are left out.
pub fn build(
    project: &Project,
    form: Option<&Form>,
    spacing_mm: f64,
) -> Result<Drape, CommandError> {
    let project = &*project.as_shown();
    let spacing = spacing_mm.max(2.0);
    let samples = sample_tracks(project, spacing);
    // One fabric per garment, and the garments simulated (those with a
    // placed panel) in their layers.
    let garments = project.garments_by_layer();
    let fabrics = garments
        .iter()
        .map(|id| {
            let garment = project.garments().get(id);
            let mut params = fabric_preset(garment.map(|g| g.fabric).unwrap_or_default()).params();
            // The garment's own weight, when set (g/m² to kg/m²).
            if let Some(w) = garment.and_then(|g| g.fabric_details.weight_gsm) {
                params.density = (w / 1000.0) as f32;
            }
            params
        })
        .collect();
    let worn: Vec<GarmentId> = garments
        .iter()
        .copied()
        .filter(|g| {
            project
                .panels()
                .values()
                .any(|p| p.garment == *g && p.placement.is_some())
        })
        .collect();

    let mut scene = SimScene {
        fabrics,
        ground: true,
        ..SimScene::default()
    };
    let mut pieces = Vec::new();
    // Particle indices along each edge, start to end inclusive.
    let mut edge_vertices: BTreeMap<EdgeId, Vec<u32>> = BTreeMap::new();
    // Particle indices along each track, at its samples.
    let mut track_particles: BTreeMap<Track, Vec<u32>> = BTreeMap::new();
    // The particle of each button and buttonhole.
    let mut trim_particles: BTreeMap<(ButtonId, usize), u32> = BTreeMap::new();

    // Mesh every placed panel first: the starting radius depends on the
    // width of all of them.
    let mut laid = Vec::new();
    for (panel_id, panel) in project.panels() {
        let Some(place) = panel.placement else {
            continue;
        };
        // A loop of edges sampled: its points (each edge's but its last),
        // and where each edge starts among them.
        let sampled_loop = |edges: &[EdgeId]| -> Result<(Vec<DVec2>, Vec<usize>), CommandError> {
            let mut pts = Vec::new();
            let mut starts = Vec::with_capacity(edges.len());
            for e in edges {
                let at = samples
                    .get(&Track::Edge(*e))
                    .ok_or_else(|| internal(format!("{e} is not sampled")))?;
                starts.push(pts.len());
                for s in at.iter().take(at.len().saturating_sub(1)) {
                    let (p, _) = project
                        .track_point(Track::Edge(*e), *s)
                        .ok_or_else(|| internal(format!("{e} has no geometry")))?;
                    pts.push(p);
                }
            }
            Ok((pts, starts))
        };
        let (boundary, starts) = sampled_loop(&panel.contour)?;
        let holes: Vec<(Vec<DVec2>, Vec<usize>)> = panel
            .holes
            .iter()
            .map(|h| sampled_loop(h))
            .collect::<Result<_, _>>()?;
        let lines: Vec<(Track, Vec<DVec2>)> = samples
            .iter()
            .filter(|(t, _)| project.track_panel(**t) == Some(*panel_id))
            .filter(|(t, _)| matches!(t, Track::Line(_)))
            .map(|(t, at)| {
                let pts = at
                    .iter()
                    .filter_map(|s| project.track_point(*t, *s).map(|(p, _)| p))
                    .collect();
                (*t, pts)
            })
            .collect();
        let hole_pts: Vec<Vec<DVec2>> = holes.iter().map(|(p, _)| p.clone()).collect();
        let mut line_pts: Vec<Vec<DVec2>> = lines.iter().map(|(_, p)| p.clone()).collect();
        // Buttons and buttonholes on the piece become mesh vertices (each
        // a line of one point), so a closure holds exactly there; those
        // too near the outline or a hole are left to the nearest vertex.
        let ends: Vec<((ButtonId, usize), DVec2)> = button_ends(project, *panel_id)
            .into_iter()
            .filter(|(_, p)| clear_of(*p, &boundary, &hole_pts, 0.25 * spacing))
            .collect();
        line_pts.extend(ends.iter().map(|(_, p)| vec![*p]));
        // Interfacings end on mesh lines (but on lines meshed already).
        for (_, i) in project.interfacings().filter(|(_, i)| i.panel == *panel_id) {
            if let r3d_core::InterfacingArea::Inside { line } = &i.area
                && samples.contains_key(&Track::Line(*line))
            {
                continue;
            }
            for bound in project.interfacing_bounds(i) {
                let len = r3d_core::seam::polyline_length(&bound);
                let n = ((len / spacing).ceil() as usize).max(1);
                line_pts.push(
                    (0..=n)
                        .map(|k| r3d_core::seam::point_along(&bound, len * k as f64 / n as f64).0)
                        .collect(),
                );
            }
        }
        // Finer triangles along the piece's elastics.
        let fine: Vec<FineZone> = project
            .elastics()
            .filter(|(_, e)| project.side_panel(&e.side) == Some(*panel_id))
            .filter_map(|(_, e)| {
                let mut line: Vec<DVec2> = Vec::new();
                for span in spans(project, &e.side)? {
                    let at = samples.get(&span.track)?;
                    for s in at
                        .iter()
                        .filter(|s| **s >= span.from - 1e-6 && **s <= span.to + 1e-6)
                    {
                        let (p, _) = project.track_point(span.track, *s)?;
                        if line.last().is_none_or(|q| q.distance(p) > 1e-9) {
                            line.push(p);
                        }
                    }
                }
                Some(FineZone {
                    line,
                    spacing: spacing.min(ELASTIC_SPACING_MM),
                    rows: ELASTIC_ROWS,
                })
            })
            .collect();
        // Quilted: the stitched lines are mesh lines, the piece meshed at
        // a quarter of the lines' spacing at most so the puffs show.
        let quilts: Vec<&r3d_core::Quilting> = project
            .quiltings()
            .filter(|(_, q)| q.panel == *panel_id)
            .map(|(_, q)| q)
            .collect();
        let piece_spacing = quilts
            .iter()
            .map(|q| q.spacing_mm / QUILT_POINTS_PER_SPACING)
            .fold(spacing, f64::min)
            .max(2.0);
        let quilt_start = line_pts.len();
        for q in &quilts {
            for line in project.quilting_lines(q) {
                line_pts.push(subdivided(&line, piece_spacing));
            }
        }
        let quilt_end = line_pts.len();
        let mesh = mesh_panel(&boundary, &hole_pts, &line_pts, &fine, piece_spacing)
            .map_err(|e| internal(format!("{panel_id}: {e}")))?;
        let quilted: Vec<Vec<u32>> = mesh
            .lines
            .get(quilt_start..quilt_end)
            .map(<[Vec<u32>]>::to_vec)
            .unwrap_or_default();
        // How far each vertex puffs out: up to the loft halfway between
        // stitched lines, nothing on them or at the area's edge (sewn
        // flat there).
        let mut puff = vec![0.0f32; mesh.vertices.len()];
        for q in &quilts {
            let (Some(area), lines) = (project.quilting_outline(q), project.quilting_lines(q))
            else {
                continue;
            };
            for (v, out) in mesh.vertices.iter().zip(&mut puff) {
                if !r3d_geom::polygon::contains(&area, *v) {
                    continue;
                }
                let d = lines
                    .iter()
                    .flat_map(|l| l.windows(2))
                    .filter_map(|w| match w {
                        [a, b] => Some(r3d_geom::fit::distance_to_segment(*v, *a, *b)),
                        _ => None,
                    })
                    .fold(f64::INFINITY, f64::min)
                    .min(r3d_geom::polygon::distance_to_loop(&area, *v))
                    .min(q.spacing_mm / 2.0);
                let rise = q.loft_mm * (std::f64::consts::PI * d / q.spacing_mm).sin();
                *out = out.max((rise / 1000.0) as f32);
            }
        }
        // The lines' own vertices lie on them, even where the mesher moved
        // one a little (off the outline, onto a crossing).
        for v in quilted.iter().flatten() {
            if let Some(out) = puff.get_mut(*v as usize) {
                *out = 0.0;
            }
        }
        let mut trims: BTreeMap<(ButtonId, usize), u32> = BTreeMap::new();
        for (k, (key, _)) in ends.iter().enumerate() {
            if let Some(v) = mesh.lines.get(lines.len() + k).and_then(|l| l.first()) {
                trims.insert(*key, *v);
            }
        }
        // Each track's mesh vertices, at its samples.
        let mut tracks: BTreeMap<Track, Vec<u32>> = BTreeMap::new();
        let mut put_loop = |edges: &[EdgeId], starts: &[usize], indices: &[u32]| {
            let total = indices.len().max(1);
            for (e, start) in edges.iter().zip(starts) {
                let n = samples
                    .get(&Track::Edge(*e))
                    .map_or(1, |s| s.len().saturating_sub(1));
                let verts = (0..=n)
                    .filter_map(|k| indices.get((start + k) % total))
                    .copied()
                    .collect();
                tracks.insert(Track::Edge(*e), verts);
            }
        };
        put_loop(&panel.contour, &starts, &mesh.boundary);
        for ((hole, (_, starts)), indices) in panel.holes.iter().zip(&holes).zip(&mesh.holes) {
            put_loop(hole, starts, indices);
        }
        for ((track, _), indices) in lines.iter().zip(&mesh.lines) {
            tracks.insert(*track, indices.clone());
        }
        // The other ends: at the nearest vertex.
        for (key, at) in button_ends(project, *panel_id) {
            if trims.contains_key(&key) {
                continue;
            }
            let nearest = mesh
                .vertices
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
                .map(|(i, _)| i as u32);
            if let Some(v) = nearest {
                trims.insert(key, v);
            }
        }
        let top = mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.y));
        laid.push(Laid {
            panel: *panel_id,
            layer: worn.iter().position(|g| *g == panel.garment).unwrap_or(0),
            place,
            boundary,
            tracks,
            trims,
            mesh,
            quilted,
            puff,
            top,
        });
    }
    // A garment's circumference at a height around the body or a leg: all
    // its panels wrapped around it side by side.
    let around = |layer: usize, kind: Around, y: f32| {
        laid.iter()
            .filter(|l| l.layer == layer && l.place.around == kind)
            .map(|l| l.width_below(l.place.top_cm as f32 / 100.0 - y))
            .sum::<f32>()
    };
    // Start positions, garment by garment, each a little farther out
    // than the ones under it.
    let mut starts: Vec<Vec<Vec3>> = vec![Vec::new(); laid.len()];
    let height = form.map_or(DEFAULT_HEIGHT_M, |f| {
        f.positions.iter().map(|p| p.y).fold(0.0, f32::max)
    });
    for layer in 0..worn.len() {
        // Where the garments under this one start, around the body.
        let below = laid
            .iter()
            .zip(&starts)
            .filter(|(l, _)| {
                l.layer < layer
                    && matches!(
                        l.place.around,
                        Around::Body | Around::LeftLeg | Around::RightLeg
                    )
            })
            .flat_map(|(_, s)| s.iter().map(|p| (p.y, Vec2::new(p.x, p.z))));
        let worn_under = if layer > 0 {
            Profile::new(height, below)
        } else {
            Profile::default()
        };
        let sleeves_under = (0..layer)
            .filter(|below| {
                laid.iter().any(|l| {
                    l.layer == *below
                        && matches!(l.place.around, Around::LeftArm | Around::RightArm)
                })
            })
            .count();
        let under = Under {
            form,
            depth: layer,
            sleeves_under,
            several: worn.len() > 1,
            worn: worn_under,
        };
        let around_hips =
            |y: f32| around(layer, Around::LeftLeg, y) + around(layer, Around::RightLeg, y);
        // The lowest top of a leg's pieces (m).
        let rim = |kind: Around| {
            laid.iter()
                .filter(|l| l.layer == layer && l.place.around == kind)
                .map(|l| l.place.top_cm as f32 / 100.0)
                .fold(f32::MAX, f32::min)
        };
        for (l, start) in laid.iter().zip(&mut starts) {
            if l.layer != layer {
                continue;
            }
            *start = match l.place.around {
                Around::Body => wrap_body(l, &under, &|y| around(layer, Around::Body, y)),
                Around::LeftArm => wrap_arm(l, &under, 1.0),
                Around::RightArm => wrap_arm(l, &under, -1.0),
                Around::LeftLeg => wrap_leg(
                    l,
                    &under,
                    1.0,
                    rim(Around::LeftLeg),
                    &|y| around(layer, Around::LeftLeg, y),
                    &around_hips,
                ),
                Around::RightLeg => wrap_leg(
                    l,
                    &under,
                    -1.0,
                    rim(Around::RightLeg),
                    &|y| around(layer, Around::RightLeg, y),
                    &around_hips,
                ),
                Around::Flat => lay_flat(l),
            };
        }
    }

    // Fabrics of the cloth under details, after the garments' own.
    let mut composite: BTreeMap<(u16, Covers), u16> = BTreeMap::new();
    let mut puff: Vec<f32> = Vec::new();
    for (l, start) in laid.iter().zip(starts) {
        let Some(panel) = project.panel(l.panel) else {
            continue;
        };
        let mesh = &l.mesh;
        let offset = scene.particles.len() as u32;
        for (track, verts) in &l.tracks {
            let verts: Vec<u32> = verts.iter().map(|v| offset + v).collect();
            if let Track::Edge(e) = track {
                edge_vertices.insert(*e, verts.clone());
            }
            track_particles.insert(*track, verts);
        }
        for (key, v) in &l.trims {
            trim_particles.insert(*key, offset + v);
        }

        puff.extend(&l.puff);
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
            corners: panel
                .contour
                .iter()
                .filter_map(|e| {
                    let id = project.edge(*e)?.start;
                    let at = project.point(id)?.pos;
                    Some((
                        id,
                        Vec2::new((at.x / 1000.0) as f32, (at.y / 1000.0) as f32),
                    ))
                })
                .collect(),
        });
        let grain = (panel.grain_angle_deg as f32).to_radians();
        let warp = Vec2::new(grain.cos(), grain.sin());
        let garment = garments
            .iter()
            .position(|g| *g == panel.garment)
            .and_then(|k| u16::try_from(k).ok())
            .unwrap_or(0);
        let covers = PieceCovers::of(project, l.panel);
        // The stitched lines fold with the shell and lining alone.
        if let Some(base) = scene.fabrics.get(usize::from(garment)).copied() {
            for line in &l.quilted {
                for w in line.windows(2) {
                    if let [a, b] = w {
                        scene
                            .creases
                            .push(([offset + a, offset + b], base.quilted_crease()));
                    }
                }
            }
        }
        for t in &mesh.triangles {
            let corners = t.map(|i| mesh.vertices.get(i as usize).copied().unwrap_or_default());
            let cover = covers.covers(corners);
            let fabric = if cover == Covers::default() {
                garment
            } else {
                // A fabric of its own per garment and covers.
                match composite.get(&(garment, cover)) {
                    Some(k) => *k,
                    None => {
                        let base = scene.fabrics.get(usize::from(garment)).copied();
                        let k = u16::try_from(scene.fabrics.len()).unwrap_or(garment);
                        if let Some(base) = base {
                            scene.fabrics.push(cover.fabric(base));
                            composite.insert((garment, cover), k);
                        }
                        k
                    }
                }
            };
            scene.triangles.push(Triangle {
                vertices: t.map(|i| offset + i),
                fabric,
                garment,
                warp,
            });
        }
    }

    // A seam side's particles, in the order it is walked: its stretches'
    // samples one after the other (a corner between two edges once).
    let side_particles = |s: &SeamSide| -> Option<Vec<u32>> {
        let mut out: Vec<u32> = Vec::new();
        for span in spans(project, s)? {
            let at = samples.get(&span.track)?;
            let parts = track_particles.get(&span.track)?;
            let (i0, i1) = (sample_index(at, span.from)?, sample_index(at, span.to)?);
            for k in i0..=i1 {
                let v = *parts.get(k)?;
                if out.last() != Some(&v) {
                    out.push(v);
                }
            }
        }
        if s.reversed {
            out.reverse();
        }
        Some(out)
    };
    for seam in project.seams().values() {
        if let (Some(a), Some(b)) = (side_particles(&seam.a), side_particles(&seam.b)) {
            scene.seams.extend(
                a.iter()
                    .zip(&b)
                    .filter(|(x, y)| x != y)
                    .map(|(x, y)| [*x, *y]),
            );
        }
    }

    // Elastics, along their sides' particles.
    let mut elastics = Vec::new();
    for (id, e) in project.elastics() {
        if let Some(particles) = side_particles(&e.side) {
            elastics.push(id);
            scene.elastics.push(ElasticLine {
                particles,
                ratio: e.ratio as f32,
                stiffness: e.stiffness_n() as f32,
            });
        }
    }

    // Closures: buttoned buttons, and zippers from their bottom stop up
    // to the slider (the stop always holds, as on a real zipper).
    for (id, b) in project.buttons() {
        if let (true, Some(a), Some(h)) = (
            b.closed,
            trim_particles.get(&(*id, 0)),
            trim_particles.get(&(*id, 1)),
        ) {
            scene.closures.push(Closure {
                pair: [*a, *h],
                overlap: BUTTON_OVERLAP_M,
            });
        }
    }
    for z in project.zippers().values() {
        let (Some(a), Some(b)) = (side_particles(&z.a), side_particles(&z.b)) else {
            continue;
        };
        let pairs: Vec<[u32; 2]> = a
            .iter()
            .zip(&b)
            .filter(|(x, y)| x != y)
            .map(|(x, y)| [*x, *y])
            .collect();
        let last = pairs.len().saturating_sub(1).max(1) as f64;
        let closed = project.zipper_closed(z);
        scene.closures.extend(
            pairs
                .iter()
                .enumerate()
                .filter(|(k, _)| *k == 0 || *k as f64 / last <= closed + 1e-9)
                .map(|(_, pair)| Closure {
                    pair: *pair,
                    overlap: 0.0,
                }),
        );
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
        elastics,
        puff,
    })
}

/// Around a buttoned button and its buttonhole, the cloth within this
/// distance (m) lies on the other band on purpose: a front band buttoned
/// over the other, its buttons up to 10 cm apart. They do not repel there.
const BUTTON_OVERLAP_M: f32 = 0.08;

/// The buttons (end 0) and buttonholes (end 1) on `panel`, where the
/// closure holds them (mm).
fn button_ends(project: &Project, panel: PanelId) -> Vec<((ButtonId, usize), DVec2)> {
    let mut out = Vec::new();
    for (id, b) in project.buttons() {
        if b.panel == panel {
            out.push(((*id, 0), b.at));
        }
        if b.hole.panel == panel {
            out.push(((*id, 1), b.hole.at));
        }
    }
    out
}

/// Whether `p` lies inside `boundary`, outside every hole and at least
/// `margin` from all of them (mm).
fn clear_of(p: DVec2, boundary: &[DVec2], holes: &[Vec<DVec2>], margin: f64) -> bool {
    let far = |ring: &[DVec2]| {
        (0..ring.len()).all(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            r3d_geom::fit::distance_to_segment(p, a, b) >= margin
        })
    };
    r3d_geom::polygon::contains(boundary, p)
        && far(boundary)
        && holes
            .iter()
            .all(|h| !r3d_geom::polygon::contains(h, p) && far(h))
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

/// The cloth's normal at each particle (out of the garment), from its
/// triangles at `positions`.
fn vertex_normals(drape: &Drape, positions: &[Vec3]) -> Vec<Vec3> {
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
    normals
}

/// Internal lines are sampled this often along the pattern (mm).
const LINE_STEP_MM: f64 = 10.0;

/// A line on the draped cloth: points with the cloth's normal there.
pub type ClothLine = Vec<(Vec3, Vec3)>;

/// A topstitch on the draped cloth: each stitch's two ends (with the
/// cloth's normal), the thread's colour (`0xRRGGBB`) and width (m).
pub type ClothStitches = (Vec<[(Vec3, Vec3); 2]>, u32, f32);

/// The cloth as drawn at one frame: quilted parts puffed, with the
/// cloth's normal at each particle.
#[derive(Debug)]
pub struct ClothAt {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

impl ClothAt {
    /// `drape`'s cloth with its particles at `positions`.
    pub fn new(drape: &Drape, positions: &[Vec3]) -> Self {
        let positions = drape.puffed(positions);
        let normals = vertex_normals(drape, &positions);
        Self { positions, normals }
    }

    fn normal(&self, i: usize) -> Vec3 {
        self.normals.get(i).copied().unwrap_or(Vec3::Y)
    }

    /// The point at `anchor`, with the cloth's normal there.
    fn at(&self, (t, w): &Anchor) -> Option<(Vec3, Vec3)> {
        let [a, b, c] = t.map(|k| self.positions.get(k).copied());
        let at = a? * w.x + b? * w.y + c? * w.z;
        let [na, nb, nc] = t.map(|k| self.normal(k));
        Some((at, (na * w.x + nb * w.y + nc * w.z).normalize_or(Vec3::Y)))
    }

    fn line(&self, anchors: &[Anchor]) -> ClothLine {
        anchors.iter().filter_map(|a| self.at(a)).collect()
    }

    /// A contour edge of a simulated panel of `drape`.
    pub fn edge(&self, drape: &Drape, edge: EdgeId) -> Option<ClothLine> {
        let particles = drape.edges.get(&edge)?;
        Some(
            particles
                .iter()
                .filter_map(|i| Some((*self.positions.get(*i)?, self.normal(*i))))
                .collect(),
        )
    }
}

/// A pattern point on a drape: the triangle holding it (particles) and
/// its weights there.
type Anchor = ([usize; 3], Vec3);

/// The pattern's style lines, buttons, zippers and stitches where they lie
/// on a drape's cloth, found once for a drape and a document and then
/// placed on the cloth at every frame ([`ClothAt`]): the thousands of
/// stitches of a quilted piece are not looked up again each frame.
#[derive(Debug, Default)]
pub struct ClothMarks {
    lines: Vec<Vec<Anchor>>,
    /// Each button and its radius (m).
    buttons: Vec<(Anchor, f32)>,
    zippers: Vec<Vec<Anchor>>,
    /// Each topstitch's or quilting's stitches, its thread's colour
    /// (`0xRRGGBB`) and width (m).
    stitches: Vec<(Vec<[Anchor; 2]>, u32, f32)>,
}

impl ClothMarks {
    /// The marks of `project` on `drape`.
    pub fn new(drape: &Drape, project: &Project) -> Self {
        let project = project.as_shown();
        let locator = Locator::new(drape);
        let anchor = |panel: PanelId, p: DVec2| locator.locate(panel, (p / 1000.0).as_vec2());
        let lines = project
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
                let on: Vec<Anchor> = points
                    .iter()
                    .filter_map(|p| anchor(line.panel, *p))
                    .collect();
                (on.len() > 1).then_some(on)
            })
            .collect();
        // A buttoned button sits where it is sewn; so does an unbuttoned
        // one (its buttonhole is not drawn in 3D).
        let buttons = project
            .buttons()
            .values()
            .filter_map(|b| Some((anchor(b.panel, b.at)?, (b.diameter_mm / 2000.0) as f32)))
            .collect();
        // A zipper's closed stretch, along its first side (both sides
        // meet there).
        let zippers = project
            .zippers()
            .values()
            .filter_map(|z| {
                let panel = project.side_panel(&z.a)?;
                let side = project.side_polyline(&z.a)?;
                let total = r3d_core::seam::polyline_length(&side);
                let closed = total * project.zipper_closed(z);
                let steps = ((closed / LINE_STEP_MM).ceil() as usize).max(1);
                let along = (0..=steps).map(|k| closed * k as f64 / steps as f64);
                let line: Vec<Anchor> = r3d_core::seam::points_along(&side, along)
                    .into_iter()
                    .filter_map(|p| anchor(panel, p))
                    .collect();
                (closed > 0.0 && line.len() > 1).then_some(line)
            })
            .collect();
        let stitched = |panel: PanelId, rows: &[Vec<DVec2>], stitch_mm: f64, rgb: u32, width| {
            let dashes: Vec<[Anchor; 2]> = rows
                .iter()
                .flat_map(|row| r3d_core::seam::stitches(row, stitch_mm))
                .filter_map(|[a, b]| Some([anchor(panel, a)?, anchor(panel, b)?]))
                .collect();
            (!dashes.is_empty()).then_some((dashes, rgb, width))
        };
        let topstitches = project.topstitches().filter_map(|(_, t)| {
            let panel = project.side_panel(&t.side)?;
            let rows = project.topstitch_rows(t);
            let width = (t.thread.width_mm() / 1000.0) as f32;
            stitched(panel, &rows, t.stitch_mm, project.thread_rgb(t), width)
        });
        let quiltings = project.quiltings().filter_map(|(_, q)| {
            let rows = project.quilting_lines(q);
            let width = (r3d_core::Thread::Normal.width_mm() / 1000.0) as f32;
            stitched(q.panel, &rows, q.stitch_mm, project.quilting_rgb(q), width)
        });
        let stitches = topstitches.chain(quiltings).collect();
        Self {
            lines,
            buttons,
            zippers,
            stitches,
        }
    }

    /// Every internal line of the simulated panels on `cloth`.
    pub fn internal_lines(&self, cloth: &ClothAt) -> Vec<ClothLine> {
        self.lines.iter().map(|l| cloth.line(l)).collect()
    }

    /// The buttons on `cloth`: centre, the cloth's normal and radius (m).
    pub fn buttons(&self, cloth: &ClothAt) -> Vec<(Vec3, Vec3, f32)> {
        self.buttons
            .iter()
            .filter_map(|(a, r)| {
                let (at, n) = cloth.at(a)?;
                Some((at, n, *r))
            })
            .collect()
    }

    /// The zippers' closed stretches on `cloth`.
    pub fn zippers(&self, cloth: &ClothAt) -> Vec<ClothLine> {
        self.zippers.iter().map(|z| cloth.line(z)).collect()
    }

    /// Every topstitch and quilting on `cloth`, stitch by stitch.
    pub fn stitches(&self, cloth: &ClothAt) -> Vec<ClothStitches> {
        self.stitches
            .iter()
            .map(|(dashes, rgb, width)| {
                let on = dashes
                    .iter()
                    .filter_map(|[a, b]| Some([cloth.at(a)?, cloth.at(b)?]))
                    .collect();
                (on, *rgb, *width)
            })
            .collect()
    }
}

/// Start `new` where `old` is now (`positions`): each particle takes the
/// position of the same pattern point in the old drape, interpolated in
/// the old triangle that contains it, or extrapolated from the nearest one
/// when the panel grew. Particles of new panels, and pinned ones, keep
/// their own start. This is how pattern edits reach a running simulation
/// without a jump (architecture §7.8). A piece only moved on the board
/// (every corner by the same displacement) is looked up where it was:
/// where it lies on the board does not change the drape.
pub fn carry_over(old: &Drape, positions: &[Vec3], new: &mut Drape) {
    let locator = Locator::new(old);
    for piece in &new.pieces {
        let moved = old
            .pieces
            .iter()
            .find(|o| o.panel == piece.panel)
            .and_then(|o| moved_by(&o.corners, &piece.corners))
            .unwrap_or(Vec2::ZERO);
        for i in piece.particles.clone() {
            let Some(p) = new.scene.particles.get_mut(i) else {
                continue;
            };
            if p.pinned {
                continue;
            }
            if let Some(at) = locator.position(piece.panel, p.rest - moved, positions) {
                p.position = at;
            }
        }
    }
}

/// The displacement every corner of a piece shares from `old` to `new`
/// (the same corners, all moved alike), if any.
fn moved_by(old: &BTreeMap<PointId, Vec2>, new: &BTreeMap<PointId, Vec2>) -> Option<Vec2> {
    /// How far apart two corners' moves may be and still be one (m).
    const SAME_M: f32 = 1e-5;
    if old.len() != new.len() || old.is_empty() {
        return None;
    }
    let mut by = None;
    for (id, to) in new {
        let d = *to - *old.get(id)?;
        match by {
            None => by = Some(d),
            Some(b) if (d - b).length() > SAME_M => return None,
            Some(_) => {}
        }
    }
    by
}

/// Bin size of the carry-over grid on the pattern (m).
const CARRY_CELL_M: f32 = 0.02;

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
    /// The avatar's motion, if it moved.
    pub animation: Option<Animation>,
}

/// Simulate `frames` frames of 1/60 s from the starting positions, on the
/// CPU backend: deterministic, so a cached drape can be reproduced. With a
/// `motion`, the avatar starts moving after [`MOTION_START_S`].
pub fn simulate(
    project: &Project,
    frames: u32,
    motion: Option<Motion>,
    progress: &Progress,
) -> Result<Simulated, CommandError> {
    let form = Form::of_project(project)?;
    let mut drape = prepare(project, form.as_ref())?;
    if drape.pieces.is_empty() {
        return Err(CommandError::NothingToSimulate);
    }
    let animation = motion
        .filter(|_| form.is_some())
        .map(|m| Animation::starting(m, 0.0));
    if let (Some(f), Some(a)) = (&form, &animation) {
        drape.scene.motion = f.body_motion(a);
    }
    let mut solver = CpuSolver::new(drape.scene.clone());
    let start = Instant::now();
    for frame in 1..=frames {
        progress.check()?;
        solver
            .step(1.0 / 60.0)
            .map_err(|e| internal(e.to_string()))?;
        if frame % PROGRESS_FRAMES == 0 || frame == frames {
            progress.report(&Status {
                task: "simulate.frames",
                done: frame,
                total: frames,
                item: String::new(),
            });
        }
    }
    let ms_per_frame = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames.max(1));
    Ok(Simulated {
        positions: solver.positions().to_vec(),
        stats: solver.stats(),
        drape,
        form,
        frames,
        ms_per_frame,
        animation,
    })
}

/// How often a headless drape reports its progress (frames).
const PROGRESS_FRAMES: u32 = 10;

/// Where the last headless drape is cached, relative to the project folder.
pub const CACHE_FILE: &str = ".r3d-cache/drape.bin";
const CACHE_MAGIC: &[u8; 8] = b"R3DDRP03";
/// The cache's motion tag for a captured motion (after the built-in ones).
const CAPTURED_TAG: u32 = Motion::BUILT_IN.len() as u32 + 1;

/// Fingerprint of everything a drape depends on: the document (schema
/// JSON), the mesh spacing and the solver's substeps. FNV-1a, stable
/// across runs and platforms.
pub fn fingerprint(project: &Project) -> Result<u64, CommandError> {
    let json = r3d_core::file::to_json(project).map_err(|e| internal(e.to_string()))?;
    let tail = [
        DEFAULT_SPACING_MM.to_le_bytes(),
        u64::from(SUBSTEPS).to_le_bytes(),
    ];
    Ok(fnv(json.as_bytes().iter().chain(tail.iter().flatten())))
}

fn fnv<'a>(bytes: impl Iterator<Item = &'a u8>) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// The motion captured in the BVH file at `path` (its name), read from
/// `bytes`.
pub fn captured_motion(path: &str, bytes: &[u8]) -> Result<Motion, CommandError> {
    let motion = CapturedMotion::from_bvh(path, &String::from_utf8_lossy(bytes))?;
    Ok(Motion::Captured(Arc::new(motion)))
}

/// A cached drape: where the cloth was at `time`, and how the avatar
/// moved, if it did.
#[derive(Debug, Clone, PartialEq)]
pub struct Cached {
    pub positions: Vec<Vec3>,
    pub animation: Option<Animation>,
    /// Simulated time at the end (s).
    pub time: f32,
}

/// Encode a drape for [`CACHE_FILE`]: the fingerprint, the time, the
/// motion (0 for none, a built-in one's index in [`Motion::BUILT_IN`]
/// plus one, or a captured one's file, relative to the project folder,
/// and a hash of its bytes: `motion_file`; it started at
/// [`MOTION_START_S`]), the positions.
pub fn encode_cache(
    fingerprint: u64,
    out: &Simulated,
    motion_file: Option<(&str, &[u8])>,
) -> Vec<u8> {
    let positions = &out.positions;
    let mut bytes = Vec::with_capacity(64 + positions.len() * 12);
    bytes.extend_from_slice(CACHE_MAGIC);
    bytes.extend_from_slice(&fingerprint.to_le_bytes());
    bytes.extend_from_slice(&(out.frames as f32 / 60.0).to_le_bytes());
    let motion = out.animation.as_ref().map(|a| &a.motion);
    match (motion, motion_file) {
        (Some(Motion::Captured(_)), Some((path, file))) => {
            bytes.extend_from_slice(&CAPTURED_TAG.to_le_bytes());
            bytes.extend_from_slice(&(path.len() as u32).to_le_bytes());
            bytes.extend_from_slice(path.as_bytes());
            bytes.extend_from_slice(&fnv(file.iter()).to_le_bytes());
        }
        (m, _) => {
            let tag = m
                .and_then(|m| Motion::BUILT_IN.iter().position(|b| b == m))
                .map_or(0, |i| i as u32 + 1);
            bytes.extend_from_slice(&tag.to_le_bytes());
        }
    }
    for p in positions {
        for c in p.to_array() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
    }
    bytes
}

/// Reads a cache front to back.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, rest) = self.0.split_at_checked(n)?;
        self.0 = rest;
        Some(head)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
}

/// The cached drape in the project folder `dir`, if it was made from this
/// exact document (and motion file); `None` otherwise (stale, missing or
/// damaged cache: it is only a cache).
pub fn load_cache(dir: &Path, project: &Project, particles: usize) -> Option<Cached> {
    let bytes = std::fs::read(dir.join(CACHE_FILE)).ok()?;
    let mut r = Reader(&bytes);
    if r.take(8)? != CACHE_MAGIC || r.take(8)? != fingerprint(project).ok()?.to_le_bytes() {
        return None;
    }
    let time = f32::from_le_bytes(r.take(4)?.try_into().ok()?);
    let motion = match r.u32()? {
        0 => None,
        CAPTURED_TAG => {
            let len = r.u32()? as usize;
            let path = std::str::from_utf8(r.take(len)?).ok()?;
            let hash = r.take(8)?;
            let file = std::fs::read(dir.join(path)).ok()?;
            if fnv(file.iter()).to_le_bytes() != hash {
                return None;
            }
            Some(captured_motion(path, &file).ok()?)
        }
        i => Some(Motion::BUILT_IN.get(i as usize - 1)?.clone()),
    };
    if r.0.len() != particles * 12 {
        return None;
    }
    let positions = r
        .0
        .as_chunks::<12>()
        .0
        .iter()
        .map(|c| {
            let (xyz, _) = c.as_chunks::<4>();
            let [x, y, z] = [0, 1, 2].map(|i| xyz.get(i).map_or(0.0, |b| f32::from_le_bytes(*b)));
            Vec3::new(x, y, z)
        })
        .collect();
    Some(Cached {
        positions,
        animation: motion.map(|m| Animation::starting(m, 0.0)),
        time,
    })
}

/// The garment as it was last draped headless (renders, exports, fit):
/// the cached drape if it was made from this exact document, otherwise
/// where the simulation starts.
#[derive(Debug, Clone)]
pub struct Latest {
    pub drape: Drape,
    pub form: Option<Form>,
    /// One per particle of `drape`.
    pub positions: Vec<Vec3>,
    /// The avatar's surface at that moment (posed if it moved).
    pub body: Option<Vec<Vec3>>,
    /// The avatar's motion and the simulated time (s) at that moment.
    pub animation: Option<Animation>,
    pub time: f32,
    /// From the cache (`true`) or the starting positions.
    pub simulated: bool,
}

impl Latest {
    /// The latest drape of `project`, whose folder is `dir`.
    pub fn load(dir: &Path, project: &Project) -> Result<Self, CommandError> {
        let form = Form::of_project(project)?;
        let drape = prepare(project, form.as_ref())?;
        let start: Vec<Vec3> = drape.scene.particles.iter().map(|p| p.position).collect();
        let cached = load_cache(dir, project, start.len());
        let body = form.as_ref().map(|f| match &cached {
            Some(c) => f.posed(c.animation.as_ref(), c.time),
            None => f.positions.clone(),
        });
        let simulated = cached.is_some();
        let (animation, time) = cached
            .as_ref()
            .map_or((None, 0.0), |c| (c.animation.clone(), c.time));
        Ok(Self {
            positions: cached.map_or(start, |c| c.positions),
            drape,
            form,
            body,
            animation,
            time,
            simulated,
        })
    }

    /// `simulated` or `start`, for results.
    pub fn state(&self) -> &'static str {
        if self.simulated { "simulated" } else { "start" }
    }
}

/// The garment draped at `positions` (one per particle of `drape`), as
/// meshes to export: one per piece, named after it, with texture
/// coordinates on its garment's print (`prints`, see
/// [`crate::prints::Prints`]); the avatar too when `avatar` (its surface
/// and triangles) is given.
pub fn scene3d(
    project: &Project,
    drape: &Drape,
    positions: &[Vec3],
    avatar: Option<(&[Vec3], &[[u32; 3]])>,
    prints: &std::collections::HashMap<GarmentId, std::sync::Arc<crate::prints::PrintImage>>,
) -> r3d_io::scene3d::Scene3d {
    use r3d_io::scene3d::{Image3d, Material3d, Mesh3d, Scene3d};
    // One material per garment, named after it.
    let garments = project.garments_by_layer();
    let mut materials: Vec<Material3d> = garments
        .iter()
        .filter_map(|id| Some((project.garments().get(id)?, prints.get(id))))
        .map(|(g, print)| {
            let look = g.fabric.look();
            Material3d {
                name: g.name.clone(),
                rgb: print.map_or(g.rgb(), |p| p.mean_rgb),
                sheen: look.sheen,
                sheen_roughness: look.sheen_roughness,
                roughness: look.roughness,
                double_sided: true,
                texture: print.map(|p| Image3d {
                    bytes: p.file.clone(),
                    mime: p.mime,
                }),
            }
        })
        .collect();
    let puffed = drape.puffed(positions);
    let pieces = drape
        .pieces
        .iter()
        .filter_map(|piece| {
            let at = puffed.get(piece.particles.clone())?;
            Some(Mesh3d {
                name: project
                    .panel(piece.panel)
                    .map_or_else(|| piece.panel.to_string(), |p| p.name.clone()),
                positions: at.iter().map(|p| p.to_array()).collect(),
                uvs: crate::prints::piece_uvs(
                    project,
                    drape,
                    piece,
                    crate::prints::aspect_of(project, prints, piece),
                ),
                indices: piece.indices.clone(),
                material: project
                    .panel(piece.panel)
                    .and_then(|p| garments.iter().position(|g| *g == p.garment))
                    .unwrap_or(0),
            })
        })
        .collect();
    let avatar_material = materials.len();
    let avatar = avatar.map(|(surface, triangles)| {
        materials.push(Material3d {
            name: "Avatar".into(),
            rgb: 0xC9C6C1,
            sheen: 0.0,
            sheen_roughness: 0.0,
            roughness: 0.7,
            double_sided: false,
            texture: None,
        });
        Mesh3d {
            name: "Avatar".into(),
            positions: surface.iter().map(|p| p.to_array()).collect(),
            uvs: Vec::new(),
            indices: triangles.iter().flatten().copied().collect(),
            material: avatar_material,
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
