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
use std::time::Instant;

use glam::{Vec2, Vec3};
use r3d_avatar::fit::{fitted_body, fitted_positions};
use r3d_avatar::motion::{EASE_IN_S, Motion, ease};
use r3d_avatar::pack::Influences;
use r3d_avatar::skeleton::{Skeleton, skin};
use r3d_core::seam::{Span, Track};
use r3d_core::{Around, EdgeId, Fabric, GarmentId, PanelId, PinId, Project, SeamSide};
use r3d_geom::DVec2;
use r3d_geom::Rigid;
use r3d_geom::mesh::mesh_panel;
use r3d_sim::collide::{Skin, StaticMesh};
use r3d_sim::motion::BodyMotion;
use r3d_sim::scene::{Particle, Triangle};
use r3d_sim::{CpuSolver, FabricPreset, SUBSTEPS, SimScene, Solver, StepStats};

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
}

/// A body's skeleton and skin weights.
#[derive(Debug, Clone)]
struct Rig {
    skeleton: Skeleton,
    weights: Vec<Influences>,
}

/// The solver gets a motion sampled this many times a second.
const MOTION_FPS: f32 = 60.0;
/// A motion starts once the garment has closed and settled on the body:
/// not before this much draping (s).
pub const MOTION_START_S: f32 = 1.5;
/// A stopped motion brings the body back to rest over this time (s).
pub const STOP_S: f32 = 1.0;

/// The avatar's motion over simulated time.
#[derive(Debug, Clone, Copy, PartialEq)]
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
            Some(a) if a.stop.is_none() && to == Some(a.motion) => Some(a),
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
            None => Some(self.motion),
            Some(_) => self.then,
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
        let half = body.iter().filter(|p| side * p.x > 0.0).map(|p| {
            let c = leg.centre(p.y);
            (p.y, Vec2::new(p.x - c.x, p.z - c.z))
        });
        leg.profile = Profile::new(height, half);
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

impl Form {
    /// A dress form that stands still.
    pub fn new(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        Self::with_rig(positions, triangles, None)
    }

    fn with_rig(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>, rig: Option<Rig>) -> Self {
        let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let arms = [1.0, -1.0].map(|side| Arm::find(&positions, height, side));
        let body: Vec<Vec3> = positions
            .iter()
            .copied()
            .filter(|p| {
                Vec2::new(p.x, p.z).length() < ARM_REACH_M
                    && !arms.iter().any(|a| a.holds(*p, height).is_some())
            })
            .collect();
        let radial = Profile::new(height, body.iter().map(|p| (p.y, Vec2::new(p.x, p.z))));
        let legs = [1.0, -1.0].map(|side| Leg::find(&body, height, side));
        let crotch = body
            .iter()
            .filter(|p| p.x.abs() < MEDIAN_POINT_M && p.y > CROTCH_ABOVE * height)
            .map(|p| p.y)
            .fold(CROTCH_FRACTION * height, f32::min);
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
        }
    }

    /// Each bone's motion at `time` (simulated, s) of `animation`; `None`
    /// for a form without a skeleton.
    fn transforms(&self, animation: &Animation, time: f32) -> Option<Vec<Rigid>> {
        let rig = self.rig.as_ref()?;
        let since = |t: f32| (t - animation.start).max(0.0);
        let pose = match animation.stop {
            Some(stop) if time >= stop => match animation.then {
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
        let (start, intro, cycle) = match (animation.stop, animation.then) {
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
    pub fn of_project(project: &Project) -> Result<Option<Self>, CommandError> {
        project.shown_body().map(|b| Self::of_body(&b)).transpose()
    }

    /// The built-in body fitted to `body`, with its skeleton (seconds the
    /// first time).
    pub fn of_body(body: &r3d_core::Body) -> Result<Self, CommandError> {
        let model = body_model()?;
        let shaped = fitted_body(model, &fitted(body)?);
        let rig = Rig {
            skeleton: Skeleton::new(model, &shaped),
            weights: model.weights().to_vec(),
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
    let arm = match project.avatar() {
        Some(avatar) => {
            let model = body_model()?;
            let positions = fitted_positions(model, &fitted(&avatar.body)?);
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
}

/// A garment starts this much farther out per garment worn under it (m),
/// so the layers start in order. Two layers' particles repel each other
/// within two particle radii (`r3d_sim::selfcollide`: 0.4 of the 25 mm
/// spacing each): at 8 mm apart they were pushed past each other at once.
/// The same distance keeps a garment from fitting closely over another:
/// a shirt cannot be tucked into fitted trousers yet (docs/physics.md).
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
    mesh: r3d_geom::mesh::PanelMesh,
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
                r = (under.radius(from, span, angle0 + arc / r) + gap).max(own) + under.layered();
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
fn wrap_leg(
    l: &Laid,
    under: &Under,
    side: f32,
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
        under.radius(y + LEG_REACH_SPAN_M, 2.0 * LEG_REACH_SPAN_M, angle)
            + LIMB_CLEARANCE_M
            + under.layered()
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
            let share = ((v.x - inner) / 1000.0) as f32 / around_leg(y).max(1e-3);
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
    let placed = |panel: Option<PanelId>| {
        panel
            .and_then(|p| project.panel(p))
            .is_some_and(|p| p.placement.is_some())
    };
    let mut fixed: BTreeMap<Track, Vec<(f64, f64, usize)>> = BTreeMap::new();
    for seam in project.seams().values() {
        if !(placed(project.side_panel(&seam.a)) && placed(project.side_panel(&seam.b))) {
            continue;
        }
        let (Some(a), Some(b)) = (spans(project, &seam.a), spans(project, &seam.b)) else {
            continue;
        };
        let need = |side: &[Span]| side.iter().map(|s| count(s.length())).sum::<usize>();
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
            let fabric = project.garments().get(id).map(|g| g.fabric);
            fabric_preset(fabric.unwrap_or_default()).params()
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
        let line_pts: Vec<Vec<DVec2>> = lines.iter().map(|(_, p)| p.clone()).collect();
        let mesh = mesh_panel(&boundary, &hole_pts, &line_pts, spacing)
            .map_err(|e| internal(format!("{panel_id}: {e}")))?;
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
        let top = mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.y));
        laid.push(Laid {
            panel: *panel_id,
            layer: worn.iter().position(|g| *g == panel.garment).unwrap_or(0),
            place,
            boundary,
            tracks,
            mesh,
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
    for layer in 0..worn.len() {
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
        };
        let around_hips =
            |y: f32| around(layer, Around::LeftLeg, y) + around(layer, Around::RightLeg, y);
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
                    &|y| around(layer, Around::LeftLeg, y),
                    &around_hips,
                ),
                Around::RightLeg => wrap_leg(
                    l,
                    &under,
                    -1.0,
                    &|y| around(layer, Around::RightLeg, y),
                    &around_hips,
                ),
                Around::Flat => lay_flat(l),
            };
        }
    }

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
        let fabric = garments
            .iter()
            .position(|g| *g == panel.garment)
            .and_then(|k| u16::try_from(k).ok())
            .unwrap_or(0);
        scene
            .triangles
            .extend(mesh.triangles.iter().map(|t| Triangle {
                vertices: t.map(|i| offset + i),
                fabric,
                warp,
            }));
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
        let project = project.as_shown();
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
const CACHE_MAGIC: &[u8; 8] = b"R3DDRP02";
/// Bytes before the positions: magic, fingerprint, motion, time.
const CACHE_HEAD: usize = 8 + 8 + 4 + 4;

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

/// A cached drape: where the cloth was at `time`, and how the avatar
/// moved, if it did.
#[derive(Debug, Clone, PartialEq)]
pub struct Cached {
    pub positions: Vec<Vec3>,
    pub animation: Option<Animation>,
    /// Simulated time at the end (s).
    pub time: f32,
}

/// Encode a drape for [`CACHE_FILE`]: the fingerprint, the motion (its
/// index in [`Motion::ALL`] plus one, 0 for none; it started at
/// [`MOTION_START_S`]), the time, the positions.
pub fn encode_cache(fingerprint: u64, out: &Simulated) -> Vec<u8> {
    let positions = &out.positions;
    let motion = out
        .animation
        .and_then(|a| Motion::ALL.iter().position(|m| *m == a.motion))
        .map_or(0, |i| i as u32 + 1);
    let time = out.frames as f32 / 60.0;
    let mut bytes = Vec::with_capacity(CACHE_HEAD + positions.len() * 12);
    bytes.extend_from_slice(CACHE_MAGIC);
    bytes.extend_from_slice(&fingerprint.to_le_bytes());
    bytes.extend_from_slice(&motion.to_le_bytes());
    bytes.extend_from_slice(&time.to_le_bytes());
    for p in positions {
        for c in p.to_array() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
    }
    bytes
}

/// The cached drape in the project folder `dir`, if it was made from this
/// exact document; `None` otherwise (stale, missing or damaged cache: it
/// is only a cache).
pub fn load_cache(dir: &Path, project: &Project, particles: usize) -> Option<Cached> {
    let bytes = std::fs::read(dir.join(CACHE_FILE)).ok()?;
    let (head, body) = bytes.split_at_checked(CACHE_HEAD)?;
    let (magic, rest) = head.split_at(8);
    let (print, rest) = rest.split_at(8);
    let (motion, time) = rest.split_at(4);
    if magic != CACHE_MAGIC
        || print != fingerprint(project).ok()?.to_le_bytes()
        || body.len() != particles * 12
    {
        return None;
    }
    let motion = u32::from_le_bytes(motion.try_into().ok()?) as usize;
    let time = f32::from_le_bytes(time.try_into().ok()?);
    let animation = match motion {
        0 => None,
        i => Some(Animation::starting(*Motion::ALL.get(i - 1)?, 0.0)),
    };
    let positions = body
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
        animation,
        time,
    })
}

/// The garment draped at `positions` (one per particle of `drape`), as
/// meshes to export: one per piece, named after it, with its pattern
/// position as texture coordinates; the avatar too when `avatar` (its
/// surface and triangles) is given.
pub fn scene3d(
    project: &Project,
    drape: &Drape,
    positions: &[Vec3],
    avatar: Option<(&[Vec3], &[[u32; 3]])>,
) -> r3d_io::scene3d::Scene3d {
    use r3d_io::scene3d::{Material3d, Mesh3d, Scene3d};
    // One material per garment, named after it.
    let garments = project.garments_by_layer();
    let mut materials: Vec<Material3d> = garments
        .iter()
        .filter_map(|id| project.garments().get(id))
        .map(|g| {
            let look = g.fabric.look();
            Material3d {
                name: g.name.clone(),
                rgb: g.rgb(),
                sheen: look.sheen,
                sheen_roughness: look.sheen_roughness,
                roughness: look.roughness,
                double_sided: true,
            }
        })
        .collect();
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
