//! The avatar's motions: walking on the spot, standing at rest, turning
//! around, raising the arms, and motions captured on real people
//! ([`crate::captured`]).
//!
//! The built-in ones are our own procedural motions. The feet are placed
//! first, on the floor or in the air, and the legs reach them by inverse
//! kinematics (the `legs` module), so a planted foot neither slides nor
//! lifts; the pelvis comes down only as far as the legs need, along a
//! smooth path ([`drop_at`]), so it rises and falls without a hop. The
//! trunk
//! turns against the pelvis, the head is kept level and looking ahead,
//! and the arms hang relaxed. The walk follows published gait studies
//! (the `gait` module); standing, turning and raising the arms follow
//! general knowledge of human movement (the weight shifted over the
//! loaded foot, a turn led by the head and made in steps, the shoulder
//! girdle rising with the arm). A motion's cycle starts with it and
//! repeats every [`Motion::period`]; it starts from the rest pose and
//! grows over the first [`EASE_IN_S`].

use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use glam::{Quat, Vec3};
use r3d_geom::Rigid;

use crate::captured::CapturedMotion;
use crate::gait;
use crate::legs::{Leg, foot_at};
use crate::skeleton::{Pose, Skeleton};

/// Time over which a motion grows from the rest pose (s).
pub const EASE_IN_S: f32 = 1.0;

/// A motion of the avatar.
#[derive(Debug, Clone, PartialEq)]
pub enum Motion {
    /// Walking on the spot, as on a treadmill.
    Walk,
    /// Standing at rest: shifting the weight from one leg to the other,
    /// breathing, looking around a little.
    Idle,
    /// Turning half round and back, in steps.
    Turn,
    /// Raising both arms overhead and lowering them.
    RaiseArms,
    /// A motion captured on a real person, from a file.
    Captured(Arc<CapturedMotion>),
}

/// How far the arms come down from the rest (A) pose to hang relaxed, and
/// how far the elbows are bent then (degrees).
const ARMS_DOWN: f32 = 30.0;
const RELAXED_ELBOW: f32 = 15.0;

/// Samples of a cycle searched for how low the pelvis must come.
const SAMPLES: usize = 120;

impl Motion {
    pub const BUILT_IN: [Motion; 4] = [Motion::Walk, Motion::Turn, Motion::RaiseArms, Motion::Idle];

    /// The name agents and the app call it by: `walk`, `turn`,
    /// `raise_arms`, `idle`, or a captured motion's own.
    pub fn name(&self) -> &str {
        match self {
            Motion::Walk => "walk",
            Motion::Idle => "idle",
            Motion::Turn => "turn",
            Motion::RaiseArms => "raise_arms",
            Motion::Captured(m) => &m.name,
        }
    }

    /// Length of one cycle (s).
    pub fn period(&self) -> f32 {
        match self {
            Motion::Walk => gait::PERIOD_S,
            Motion::Idle => 8.0,
            Motion::Turn => 6.0,
            Motion::RaiseArms => 4.0,
            Motion::Captured(m) => m.period(),
        }
    }

    /// The pose `t` seconds after the motion started.
    pub fn pose(&self, skeleton: &Skeleton, t: f32) -> Pose {
        let t = t.max(0.0);
        let grown = ease(t / EASE_IN_S);
        if let Motion::Captured(m) = self {
            return m.pose(skeleton, t).scaled(grown);
        }
        let mut b = Builder {
            skeleton,
            pose: Pose::rest(skeleton.bones.len()),
        };
        let u = (t / self.period()).fract();
        match self {
            Motion::Walk => gait::walk(&mut b, u, grown),
            Motion::Idle => idle(&mut b, u, grown),
            Motion::Turn => turn(&mut b, u, grown),
            Motion::RaiseArms => raise_arms(&mut b, u),
            Motion::Captured(_) => {}
        }
        b.pose
    }
}

/// Smooth 0 → 1 over 0 → 1 (smoothstep).
pub fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Left (+x) and right sides, with the sign that mirrors an angle about
/// the vertical or forward axis.
pub(crate) const SIDES: [(&str, f32); 2] = [("l", 1.0), ("r", -1.0)];

/// Where a motion puts the body at one moment: the pelvis's turn about its
/// head joint, the whole body's offset, and each foot's motion (left,
/// right; `None` for a foot the motion places after the pelvis, from
/// where the hip is).
#[derive(Debug, Clone)]
pub(crate) struct Stance {
    pub(crate) pelvis: Quat,
    pub(crate) offset: Vec3,
    pub(crate) feet: [Option<Rigid>; 2],
}

/// How far the pelvis may go up (negative: must come down) for every
/// leg to reach its foot in `stance`: the least room a leg has.
fn room(skeleton: &Skeleton, legs: &[Leg; 2], stance: &Stance) -> f32 {
    let mut pose = Pose::rest(skeleton.bones.len());
    if let Some(q) = skeleton
        .find("pelvis")
        .and_then(|i| pose.rotations.get_mut(i))
    {
        *q = stance.pelvis;
    }
    pose.offset = stance.offset;
    let world = skeleton.forward(&pose);
    legs.iter()
        .zip(&stance.feet)
        .filter_map(|(leg, foot)| Some(leg.room(leg.hip_at(skeleton, &world), foot.as_ref()?)))
        .fold(0.0, f32::min)
}

/// How sharply the pelvis's path may bend upward where it comes down for
/// the legs and goes back up (m/s²).
const PELVIS_ACCELERATION: f32 = 2.0;
/// The half-width of the smoothing of the pelvis's path (s).
const PELVIS_SMOOTHING_S: f32 = 0.08;

/// How far the pelvis comes down at phase `u` of a motion's cycle of
/// `period` seconds whose stances `stance` gives by phase: never above
/// the rest height, low enough for the legs at every phase, and smooth.
/// The legs' limit, sampled over the cycle, is first eroded by parabolas
/// of [`PELVIS_ACCELERATION`] (its lower envelope under them, as a
/// distance transform), so the pelvis starts down early enough; that
/// envelope is then smoothed by a raised cosine of half-width
/// [`PELVIS_SMOOTHING_S`] (rounding the corners where the parabolas
/// meet) and lowered where that took it above the limit, by a smoothed
/// bound of the excess around. Between samples a leg may lack a few
/// micrometres of reach.
pub(crate) fn drop_at(
    skeleton: &Skeleton,
    legs: &[Leg; 2],
    period: f32,
    u: f32,
    stance: impl Fn(f32) -> Stance,
) -> f32 {
    let n = SAMPLES;
    let step = period / n as f32;
    let limit: Vec<f32> = (0..n)
        .map(|k| room(skeleton, legs, &stance(k as f32 / n as f32)))
        .collect();
    // Time from sample `i` to sample `j` round the cycle (s).
    let apart = |i: usize, j: usize| {
        let d = i.abs_diff(j);
        d.min(n - d) as f32 * step
    };
    let eroded: Vec<f32> = (0..n)
        .map(|i| {
            limit
                .iter()
                .enumerate()
                .map(|(j, r)| r + 0.5 * PELVIS_ACCELERATION * apart(i, j).powi(2))
                .fold(0.0, f32::min)
        })
        .collect();
    let weight = |t: f32| {
        if t < PELVIS_SMOOTHING_S {
            1.0 + (PI * t / PELVIS_SMOOTHING_S).cos()
        } else {
            0.0
        }
    };
    let smoothed = |values: &[f32]| -> Vec<f32> {
        (0..n)
            .map(|i| {
                let (sum, total) = values
                    .iter()
                    .enumerate()
                    .fold((0.0, 0.0), |(s, w), (j, e)| {
                        let x = weight(apart(i, j));
                        (s + x * e, w + x)
                    });
                if total > 0.0 { sum / total } else { 0.0 }
            })
            .collect()
    };
    let rounded = smoothed(&eroded);
    // Where the rounding went above the limit: that much more, over the
    // smoothing's width around it, smoothed in turn (it then covers the
    // excess everywhere and is nothing far from it).
    let excess: Vec<f32> = rounded
        .iter()
        .zip(&limit)
        .map(|(h, r)| (h - r).max(0.0))
        .collect();
    let widened: Vec<f32> = (0..n)
        .map(|i| {
            excess
                .iter()
                .enumerate()
                .filter(|(j, _)| apart(i, *j) <= PELVIS_SMOOTHING_S)
                .map(|(_, e)| *e)
                .fold(0.0, f32::max)
        })
        .collect();
    let smooth: Vec<f32> = rounded
        .iter()
        .zip(smoothed(&widened))
        .map(|(h, c)| h - c)
        .collect();
    periodic(&smooth, u)
}

/// A curve over a cycle at phase `v` (cycles), through its `samples`
/// (evenly spaced from phase 0) by a periodic Catmull-Rom spline.
pub(crate) fn periodic(samples: &[f32], v: f32) -> f32 {
    let n = samples.len();
    if n == 0 {
        return 0.0;
    }
    let x = v.rem_euclid(1.0) * n as f32;
    let i = x.floor() as usize;
    let f = x - i as f32;
    let at = |k: usize| samples.get((i + k + n - 1) % n).copied().unwrap_or(0.0);
    let (p0, p1, p2, p3) = (at(0), at(1), at(2), at(3));
    0.5 * (2.0 * p1
        + (p2 - p0) * f
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * f * f
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * f * f * f)
}

pub(crate) struct Builder<'a> {
    pub(crate) skeleton: &'a Skeleton,
    pub(crate) pose: Pose,
}

impl Builder<'_> {
    /// Turn `bone` by `degrees` about `axis` (rest axes), after the turns
    /// already given to it. Unknown bones are skipped.
    pub(crate) fn turn(&mut self, bone: &str, axis: Vec3, degrees: f32) {
        let Some(i) = self.skeleton.find(bone) else {
            return;
        };
        if let (Some(q), Some(axis)) = (self.pose.rotations.get_mut(i), axis.try_normalize()) {
            *q = Quat::from_axis_angle(axis, degrees.to_radians()) * *q;
        }
    }

    fn direction(&self, bone: &str) -> Vec3 {
        self.skeleton
            .find(bone)
            .and_then(|i| self.skeleton.bones.get(i))
            .map_or(Vec3::NEG_Y, |b| b.direction())
    }

    /// The turn of bone `i` from the rest pose, its parents' included.
    fn world_rotation(&self, i: usize) -> Quat {
        let mut q = Quat::IDENTITY;
        let mut at = Some(i);
        while let Some(b) = at {
            q = self
                .pose
                .rotations
                .get(b)
                .copied()
                .unwrap_or(Quat::IDENTITY)
                * q;
            at = self.skeleton.bones.get(b).and_then(|bone| bone.parent);
        }
        q
    }

    /// Bend the elbow on `side` forward by `degrees`.
    pub(crate) fn elbow(&mut self, side: &str, degrees: f32) {
        let upper = self.direction(&format!("upper_arm_{side}"));
        self.turn(&format!("forearm_{side}"), upper.cross(Vec3::Z), degrees);
    }

    /// Bend the wrist on `side` as the elbow bends, by `degrees`.
    pub(crate) fn wrist(&mut self, side: &str, degrees: f32) {
        let upper = self.direction(&format!("upper_arm_{side}"));
        self.turn(&format!("hand_{side}"), upper.cross(Vec3::Z), degrees);
    }

    /// How far the elbow on `side` is bent at rest (degrees): the A pose
    /// holds the forearms forward.
    fn rest_elbow(&self, side: &str) -> f32 {
        self.direction(&format!("upper_arm_{side}"))
            .angle_between(self.direction(&format!("forearm_{side}")))
            .to_degrees()
    }

    /// Bend the elbow on `side` from its rest bend toward `degrees`, by
    /// `amount` of the way.
    pub(crate) fn elbow_to(&mut self, side: &str, degrees: f32, amount: f32) {
        let rest = self.rest_elbow(side);
        self.elbow(side, (degrees - rest) * amount);
    }

    /// Bring both arms down from the A pose to hang relaxed, the elbows
    /// and the wrists a little bent.
    pub(crate) fn relax_arms(&mut self, amount: f32) {
        for (side, sign) in SIDES {
            self.turn(
                &format!("upper_arm_{side}"),
                Vec3::Z * sign,
                -ARMS_DOWN * amount,
            );
            self.elbow_to(side, RELAXED_ELBOW, amount);
            self.wrist(side, 6.0 * amount);
        }
    }

    /// Turn the neck and the head, sharing it, so that the head faces
    /// `facing` (from the rest pose), whatever the trunk below does.
    pub(crate) fn steady_head(&mut self, facing: Quat) {
        let (Some(neck), Some(head)) = (self.skeleton.find("neck"), self.skeleton.find("head"))
        else {
            return;
        };
        let below = self
            .skeleton
            .bones
            .get(neck)
            .and_then(|b| b.parent)
            .map_or(Quat::IDENTITY, |p| self.world_rotation(p));
        let half = Quat::IDENTITY.slerp((below.inverse() * facing).normalize(), 0.5);
        for i in [neck, head] {
            if let Some(q) = self.pose.rotations.get_mut(i) {
                *q = half;
            }
        }
    }

    /// Set the pelvis's turn and the body's offset of `stance`, the body
    /// lowered by `drop`.
    pub(crate) fn set_pelvis(&mut self, stance: &Stance, drop: f32) {
        if let Some(q) = self
            .skeleton
            .find("pelvis")
            .and_then(|i| self.pose.rotations.get_mut(i))
        {
            *q = stance.pelvis;
        }
        self.pose.offset = stance.offset + Vec3::Y * drop;
    }

    /// Turn the upper arm on `side` so that it points along `toward`
    /// (world), by `amount` of the way from its present turn.
    fn aim_arm(&mut self, side: &str, toward: Vec3, amount: f32) {
        let Some(i) = self.skeleton.find(&format!("upper_arm_{side}")) else {
            return;
        };
        let rest = self.direction(&format!("upper_arm_{side}"));
        let parent = self
            .skeleton
            .bones
            .get(i)
            .and_then(|b| b.parent)
            .map_or(Quat::IDENTITY, |p| self.world_rotation(p));
        let Some(q) = self.pose.rotations.get_mut(i) else {
            return;
        };
        let now = parent * *q;
        let aimed = Quat::from_rotation_arc(now * rest, toward.normalize_or(Vec3::Y)) * now;
        *q = q.slerp((parent.inverse() * aimed).normalize(), amount);
    }
}

/// Standing at rest over eight seconds: the weight goes onto the left leg,
/// over to the right one and back. The hips move about 3 cm over the
/// loaded foot, its side of the pelvis up; the other knee eases and its
/// heel lifts a little; the feet stay where they are. Two breaths lift the
/// chest and the shoulders, the head looks a little around, the arms hang
/// relaxed. Every term repeats within the cycle, so it loops without a
/// jump; it grows from the rest pose by `grown`.
fn idle(b: &mut Builder<'_>, u: f32, grown: f32) {
    b.relax_arms(grown);
    let Some(legs) = Leg::both(b.skeleton) else {
        return;
    };
    let stance = |u: f32| {
        let weight = grown * (TAU * u).sin();
        let mut feet = [None, None];
        for (i, (leg, (_, sign))) in legs.iter().zip(SIDES).enumerate() {
            let unloaded = (-sign * weight).max(0.0);
            let lift = foot_at(0.0, -(4.0 * unloaded).to_radians(), leg.toe, leg.toe);
            if let Some(slot) = feet.get_mut(i) {
                *slot = Some(lift);
            }
        }
        Stance {
            pelvis: Quat::from_rotation_z((1.5 * weight).to_radians()),
            offset: Vec3::X * 0.03 * weight,
            feet,
        }
    };
    let drop = drop_at(b.skeleton, &legs, Motion::Idle.period(), u, stance);
    let now = stance(u);
    b.set_pelvis(&now, drop);
    let weight = grown * (TAU * u).sin();
    // The spine keeps the trunk upright over the tilted pelvis.
    b.turn("spine1", Vec3::Z, -weight);
    b.turn("spine2", Vec3::Z, -0.5 * weight);
    let breath = grown * (2.0 * TAU * u).sin();
    b.turn("chest", Vec3::X, -0.8 * breath);
    b.turn("upper_chest", Vec3::X, -0.5 * breath);
    for (side, sign) in SIDES {
        b.turn(&format!("clavicle_{side}"), Vec3::Z * sign, breath);
        b.turn(
            &format!("upper_arm_{side}"),
            Vec3::X,
            2.0 * grown * (TAU * u + 0.7).sin(),
        );
    }
    let look = Quat::from_rotation_y((5.0 * grown * (TAU * u + 1.0).sin()).to_radians())
        * Quat::from_rotation_x((1.5 * grown * (2.0 * TAU * u + 0.5).sin()).to_radians());
    b.steady_head(look);
    for (leg, foot) in legs.iter().zip(&now.feet) {
        if let Some(foot) = foot {
            leg.place(b.skeleton, &mut b.pose, foot, 0.0);
        }
    }
}

/// A step of the turn: from `start` (s into the cycle) for [`STEP_S`], the
/// foot (0 left, 1 right) goes from where the body turned by `from` puts
/// it to where the body turned by `to` does (degrees about the vertical).
struct Step {
    start: f32,
    foot: usize,
    from: f32,
    to: f32,
}

const STEP_S: f32 = 0.45;
/// How high a foot is lifted in a step of the turn (m).
const STEP_LIFT_M: f32 = 0.035;

/// Turning left: the left foot opens first; back: the right one. Each
/// step takes the foot a third of the way round, the last of each turn
/// brings the feet side by side.
const TURN_STEPS: [Step; 8] = [
    Step {
        start: 0.55,
        foot: 0,
        from: 0.0,
        to: 60.0,
    },
    Step {
        start: 1.05,
        foot: 1,
        from: 0.0,
        to: 120.0,
    },
    Step {
        start: 1.55,
        foot: 0,
        from: 60.0,
        to: 180.0,
    },
    Step {
        start: 2.05,
        foot: 1,
        from: 120.0,
        to: 180.0,
    },
    Step {
        start: 3.05,
        foot: 1,
        from: 180.0,
        to: 120.0,
    },
    Step {
        start: 3.55,
        foot: 0,
        from: 180.0,
        to: 60.0,
    },
    Step {
        start: 4.05,
        foot: 1,
        from: 120.0,
        to: 0.0,
    },
    Step {
        start: 4.55,
        foot: 0,
        from: 60.0,
        to: 0.0,
    },
];

/// The body's heading `s` seconds into the turn (radians): still for half
/// a second, half round to the left over two, a pause, back over two, a
/// pause.
fn turn_heading(s: f32) -> f32 {
    PI * if s < 0.5 {
        0.0
    } else if s < 2.5 {
        ease((s - 0.5) / 2.0)
    } else if s < 3.0 {
        1.0
    } else if s < 5.0 {
        1.0 - ease((s - 3.0) / 2.0)
    } else {
        0.0
    }
}

/// Where foot `foot` is `s` seconds into the turn: its heading (degrees)
/// and, in a step, how far through it is.
fn turn_foot(foot: usize, s: f32) -> (f32, Option<f32>) {
    let mut heading = 0.0;
    for step in TURN_STEPS.iter().filter(|st| st.foot == foot) {
        if s < step.start {
            break;
        }
        let x = (s - step.start) / STEP_S;
        if x < 1.0 {
            return (step.from + (step.to - step.from) * ease(x), Some(x));
        }
        heading = step.to;
    }
    (heading, None)
}

/// Half a turn to the left and back over six seconds, about the vertical
/// between the feet, in steps: the head leads, the trunk follows, then the
/// pelvis; the feet step round in turn, the weight shifting over the foot
/// that stays.
fn turn(b: &mut Builder<'_>, u: f32, grown: f32) {
    let Some(legs) = Leg::both(b.skeleton) else {
        return;
    };
    let Some(head) = b
        .skeleton
        .find("pelvis")
        .and_then(|i| b.skeleton.bones.get(i))
        .map(|p| p.head)
    else {
        return;
    };
    let centre = (legs[0].ankle + legs[1].ankle) / 2.0;
    let pivot = Vec3::new(centre.x, 0.0, centre.z);
    let stance = |s: f32| {
        let heading = turn_heading(s);
        let turned = Quat::from_rotation_y(heading);
        let mut feet = [None, None];
        let mut shift = Vec3::ZERO;
        let mut tilt = 0.0;
        for (i, (_, sign)) in SIDES.iter().enumerate() {
            let (h, step) = turn_foot(i, s);
            let mut foot = Rigid::about(pivot, Quat::from_rotation_y(h.to_radians()));
            if let Some(x) = step {
                let up = (PI * x).sin();
                foot.translation.y += STEP_LIFT_M * up;
                // The weight over the other foot; its side of the pelvis up.
                let other = &legs[1 - i];
                let (oh, _) = turn_foot(1 - i, s);
                let stays = Rigid::about(pivot, Quat::from_rotation_y(oh.to_radians()));
                let over = stays.apply(other.ankle) - pivot;
                shift += Vec3::new(over.x, 0.0, over.z) * 0.3 * up;
                tilt -= sign * 2.0 * up;
            }
            if let Some(slot) = feet.get_mut(i) {
                *slot = Some(foot);
            }
        }
        Stance {
            pelvis: turned * Quat::from_rotation_z(tilt.to_radians()),
            // Turning about the pelvis's head, then shifted: as if about
            // `pivot`.
            offset: turned * (head - pivot) + pivot - head + shift,
            feet,
        }
    };
    let period = Motion::Turn.period();
    let drop = drop_at(b.skeleton, &legs, period, u, |v| stance(v * period));
    let s = u * period;
    let now = stance(s);
    b.set_pelvis(&now, drop);
    b.relax_arms(grown);
    // The trunk turns ahead of the pelvis, the head ahead of the trunk.
    let lead = turn_heading(s + 0.15) - turn_heading(s);
    for bone in ["spine2", "spine3", "chest"] {
        b.turn(bone, Vec3::Y, lead.to_degrees() / 3.0);
    }
    b.steady_head(Quat::from_rotation_y(turn_heading(s + 0.3)));
    for (i, (leg, foot)) in legs.iter().zip(&now.feet).enumerate() {
        if let Some(foot) = foot {
            let (h, _) = turn_foot(i, s);
            let knee = 0.5 * (h.to_radians() + turn_heading(s));
            leg.place(b.skeleton, &mut b.pose, foot, knee);
        }
    }
}

/// Both arms raised high over 1.5 s, in a V above the head (about 55°
/// above the horizontal), held, lowered over 1.5 s, held. The clavicles
/// rise a little with the arms, the elbows and the wrists bend a little,
/// the chest opens and the head stays level. Straight overhead the
/// shoulders' skin carries a T-shirt's neckline in and up and it rolls
/// over at the chest: the V keeps garments as they would sit.
fn raise_arms(b: &mut Builder<'_>, u: f32) {
    let s = u * 4.0;
    let amount = if s < 1.5 {
        ease(s / 1.5)
    } else if s < 2.0 {
        1.0
    } else if s < 3.5 {
        1.0 - ease((s - 2.0) / 1.5)
    } else {
        0.0
    };
    b.turn("chest", Vec3::X, -2.0 * amount);
    b.turn("upper_chest", Vec3::X, -1.5 * amount);
    for (side, sign) in SIDES {
        b.turn(&format!("clavicle_{side}"), Vec3::Z * sign, 8.0 * amount);
        b.aim_arm(side, Vec3::new(0.58 * sign, 0.8, 0.15), amount);
        b.elbow_to(side, 10.0, amount);
        b.wrist(side, 8.0 * amount);
    }
    b.steady_head(Quat::IDENTITY);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::model::{BodyModel, BodyShape, Shaped};
    use crate::skeleton::skin;

    fn body() -> (BodyModel, Shaped, Skeleton) {
        let model = BodyModel::builtin().unwrap();
        let body = model.shaped(&BodyShape::default());
        let skeleton = Skeleton::new(&model, &body);
        (model, body, skeleton)
    }

    /// Where the rest point `p` of bone `bone` is at `t` of `motion`.
    fn at(skeleton: &Skeleton, motion: &Motion, bone: &str, p: Vec3, t: f32) -> Vec3 {
        let i = skeleton.find(bone).unwrap();
        skeleton.transforms(&motion.pose(skeleton, t))[i].apply(p)
    }

    #[test]
    fn motions_stay_on_the_floor_and_move_the_body() {
        let (model, body, skeleton) = body();
        let wrist = |pose: &Pose, side: &str| {
            let i = skeleton.find(&format!("hand_{side}")).unwrap();
            skeleton.transforms(pose)[i].apply(skeleton.bones[i].head)
        };
        for motion in Motion::BUILT_IN {
            let still = motion.pose(&skeleton, 0.0);
            assert!(
                still
                    .rotations
                    .iter()
                    .all(|q| q.abs_diff_eq(Quat::IDENTITY, 1e-5))
                    && still.offset.length() < 1e-6,
                "{motion:?}"
            );
            for k in 0..60 {
                let t = k as f32 * 0.1;
                let pose = motion.pose(&skeleton, t);
                let posed = skin(
                    &body.positions,
                    model.weights(),
                    &skeleton.transforms(&pose),
                );
                let floor = posed.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                assert!(
                    posed.iter().all(|p| p.is_finite()) && floor.abs() < 0.01,
                    "{motion:?} at {t}: floor {floor}"
                );
            }
        }
        // Walking: the left arm is back when the left heel strikes.
        let at_strike = Motion::Walk.pose(&skeleton, 2.0 * Motion::Walk.period());
        assert!(wrist(&at_strike, "l").z < wrist(&at_strike, "r").z - 0.1);
        // Raising the arms: the hands go above the top of the head.
        let up = Motion::RaiseArms.pose(&skeleton, 1.75);
        let top = skeleton.bones[skeleton.find("head").unwrap()].tail.y;
        assert!(
            wrist(&up, "l").y > top && wrist(&up, "r").y > top,
            "{} {top}",
            wrist(&up, "l")
        );
        // Turning: half round, the left hand is on the right.
        let round = Motion::Turn.pose(&skeleton, 2.75);
        assert!(wrist(&round, "l").x < -0.1);
    }

    #[test]
    fn a_planted_foot_moves_with_the_belt_and_the_pelvis_does_not_hop() {
        let (_, _, skeleton) = body();
        let legs = Leg::both(&skeleton).unwrap();
        let walk = Motion::Walk;
        let period = walk.period();
        let steps = 240;
        let t0 = 3.0 * period;
        let dt = period / steps as f32;
        // The heel of a flat foot, the front of one rolling onto it.
        for (leg, (side, _)) in legs.iter().zip(SIDES) {
            let bone = format!("foot_{side}");
            let start = if side == "l" { 0.0 } else { 0.5 };
            let track = |p: Vec3, from: f32, to: f32| -> Vec<Vec3> {
                let n = ((to - from) * steps as f32) as usize;
                (0..=n)
                    .map(|k| {
                        let t = t0 + (start + from) * period + k as f32 * dt;
                        at(&skeleton, &walk, &bone, p, t)
                    })
                    .collect()
            };
            let flat = track(leg.heel, 0.1, 0.38);
            let rolling = track(leg.toe, 0.42, 0.6);
            let speed = (flat[0].z - flat[flat.len() - 1].z) / ((flat.len() - 1) as f32 * dt);
            assert!(speed > 0.6 && speed < 1.3, "belt {speed} m/s");
            for track in [&flat, &rolling] {
                for (k, p) in track.iter().enumerate() {
                    let belt = track[0].z - speed * k as f32 * dt;
                    assert!(
                        (p.z - belt).abs() < 0.003 && (p.x - track[0].x).abs() < 0.002,
                        "{side} slides: {p} vs {belt}"
                    );
                    // The front of the foot is rounded: its lowest point
                    // comes down a little as it rolls.
                    assert!(
                        (p.y - track[0].y).abs() < 0.003,
                        "{side} lifts at {k}: {p} from {}",
                        track[0]
                    );
                }
            }
        }
        // The pelvis rises and falls smoothly, twice a cycle, by a few
        // centimetres: no hop.
        let pelvis = skeleton.bones[skeleton.find("pelvis").unwrap()].head;
        let heights: Vec<f32> = (0..=steps)
            .map(|k| at(&skeleton, &walk, "pelvis", pelvis, t0 + k as f32 * dt).y)
            .collect();
        let (lo, hi) = heights
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), h| (a.min(*h), b.max(*h)));
        assert!(hi - lo > 0.01 && hi - lo < 0.045, "travel {}", hi - lo);
        let jerk = heights
            .windows(3)
            .map(|w| (w[2] - 2.0 * w[1] + w[0]).abs() / (dt * dt))
            .fold(0.0, f32::max);
        assert!(jerk < 4.0, "vertical acceleration {jerk} m/s2");
    }

    #[test]
    fn the_walk_bends_the_joints_as_people_do_and_keeps_the_head_steady() {
        let (_, _, skeleton) = body();
        let walk = Motion::Walk;
        let period = walk.period();
        let (thigh, shin, head) = (
            skeleton.find("thigh_l").unwrap(),
            skeleton.find("shin_l").unwrap(),
            skeleton.find("head").unwrap(),
        );
        let rest_bend = skeleton.bones[thigh]
            .direction()
            .angle_between(skeleton.bones[shin].direction());
        let mut knee = Vec::new();
        let mut tilt: f32 = 0.0;
        for k in 0..100 {
            let pose = walk.pose(&skeleton, 3.0 * period + k as f32 * period / 100.0);
            let world = skeleton.transforms(&pose);
            let dir = |b: usize| world[b].rotation * skeleton.bones[b].direction();
            knee.push((dir(thigh).angle_between(dir(shin)) - rest_bend).to_degrees());
            tilt = tilt.max(
                world[head]
                    .rotation
                    .angle_between(Quat::IDENTITY)
                    .to_degrees(),
            );
        }
        let stance = knee[..50].iter().copied().fold(0.0, f32::max);
        let swing = knee[60..].iter().copied().fold(0.0, f32::max);
        assert!((5.0..25.0).contains(&stance), "stance knee {stance}");
        assert!((45.0..70.0).contains(&swing), "swing knee {swing}");
        assert!(tilt < 1.5, "head {tilt}");
    }

    #[test]
    fn standing_and_turning_keep_the_feet_still_but_for_steps() {
        let (_, _, skeleton) = body();
        let legs = Leg::both(&skeleton).unwrap();
        for k in 0..80 {
            let t = k as f32 * 0.1;
            for (leg, (side, _)) in legs.iter().zip(SIDES) {
                let toe = at(
                    &skeleton,
                    &Motion::Idle,
                    &format!("foot_{side}"),
                    leg.toe,
                    t,
                );
                assert!(toe.distance(leg.toe) < 0.002, "idle {side} at {t}: {toe}");
            }
        }
        // Turning: a foot that is not stepping stays where it is.
        for k in 0..600 {
            let s = k as f32 * 0.01;
            for (i, (leg, (side, _))) in legs.iter().zip(SIDES).enumerate() {
                let (_, step) = turn_foot(i, s);
                let (_, next) = turn_foot(i, s + 0.01);
                if step.is_some() || next.is_some() {
                    continue;
                }
                let bone = format!("foot_{side}");
                let a = at(&skeleton, &Motion::Turn, &bone, leg.heel, s);
                let b = at(&skeleton, &Motion::Turn, &bone, leg.heel, s + 0.01);
                assert!(a.distance(b) < 0.001, "turn {side} at {s}: {a} -> {b}");
            }
        }
    }
}
