//! The avatar's motions: walking on the spot, standing at rest, turning
//! around, raising the arms, and motions captured on real people
//! ([`crate::captured`]).
//!
//! The built-in ones are our own procedural motions: joint angles over a
//! cycle, written from general knowledge of human gait and posture (hip,
//! knee and ankle ranges of a normal walk, the pelvis dropping on the
//! swing side, the arms swinging against the legs, the pelvis and the
//! chest turning against each other, a standing body shifting its weight
//! and breathing). The walk's ranges were checked against a slow walk of
//! the CMU motion capture database (mocap.cs.cmu.edu, subject 7, trial 4)
//! played on our skeleton, and kept smaller; no capture data is used. A motion's cycle starts with it and repeats every
//! [`Motion::period`]; over the first [`EASE_IN_S`] it grows from the rest
//! pose.

use std::f32::consts::TAU;
use std::sync::Arc;

use glam::{Quat, Vec3};

use crate::captured::CapturedMotion;
use crate::skeleton::{Pose, Skeleton};

/// Time over which a motion grows from the rest pose (s).
pub const EASE_IN_S: f32 = 1.0;

/// A motion of the avatar.
#[derive(Debug, Clone, PartialEq)]
pub enum Motion {
    /// Walking on the spot.
    Walk,
    /// Standing at rest: shifting the weight from one leg to the other,
    /// breathing, looking around a little.
    Idle,
    /// Turning half round and back.
    Turn,
    /// Raising both arms overhead and lowering them.
    RaiseArms,
    /// A motion captured on a real person, from a file.
    Captured(Arc<CapturedMotion>),
}

/// One step cycle of the walk (s): about 105 steps a minute, an easy pace.
const WALK_PERIOD_S: f32 = 1.15;

/// How far the arms come down from the rest (A) pose to hang relaxed
/// (degrees).
const ARMS_DOWN: f32 = 30.0;

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
            Motion::Walk => WALK_PERIOD_S,
            Motion::Idle => 8.0,
            Motion::Turn => 6.0,
            Motion::RaiseArms => 4.0,
            Motion::Captured(m) => m.period(),
        }
    }

    /// The pose `t` seconds after the motion started.
    pub fn pose(&self, skeleton: &Skeleton, t: f32) -> Pose {
        if let Motion::Captured(m) = self {
            return m.pose(skeleton, t.max(0.0)).scaled(ease(t / EASE_IN_S));
        }
        let mut b = Builder {
            skeleton,
            pose: Pose::rest(skeleton.bones.len()),
        };
        let u = (t.max(0.0) / self.period()).fract();
        match self {
            Motion::Walk => walk(&mut b, u),
            Motion::Idle => idle(&mut b, u),
            Motion::Turn => turn(&mut b, u),
            Motion::RaiseArms => raise_arms(&mut b, u),
            Motion::Captured(_) => {}
        }
        b.pose.scaled(ease(t / EASE_IN_S))
    }
}

/// Smooth 0 → 1 over 0 → 1 (smoothstep).
pub fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A bump of height 1 at phase `centre`, `width` wide (phases in cycles,
/// wrapping around).
fn bump(u: f32, centre: f32, width: f32) -> f32 {
    let d = (u - centre + 0.5).rem_euclid(1.0) - 0.5;
    (-(d / width).powi(2)).exp()
}

/// Left (+x) and right sides, with the sign that mirrors an angle about
/// the vertical or forward axis.
const SIDES: [(&str, f32); 2] = [("l", 1.0), ("r", -1.0)];

struct Builder<'a> {
    skeleton: &'a Skeleton,
    pose: Pose,
}

impl Builder<'_> {
    /// Turn `bone` by `degrees` about `axis` (rest axes), after the turns
    /// already given to it. Unknown bones are skipped.
    fn turn(&mut self, bone: &str, axis: Vec3, degrees: f32) {
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

    /// Bend the elbow on `side` forward by `degrees`.
    fn elbow(&mut self, side: &str, degrees: f32) {
        let upper = self.direction(&format!("upper_arm_{side}"));
        self.turn(&format!("forearm_{side}"), upper.cross(Vec3::Z), degrees);
    }

    /// Bring both arms down from the A pose to hang relaxed.
    fn relax_arms(&mut self) {
        for (side, sign) in SIDES {
            self.turn(&format!("upper_arm_{side}"), Vec3::Z * sign, -ARMS_DOWN);
            self.elbow(side, 10.0);
        }
    }
}

/// A walk on the spot, the left heel striking at phase 0. Angles in
/// degrees. The thigh swings from 22 forward at heel strike to 16 back at
/// toe-off (not forward on average, which reads as sitting); the knee
/// takes the load (about 18) after heel strike, straightens through
/// stance, bends before the toe leaves (about 30) and most in swing
/// (about 58); the ankle lowers the foot after heel strike, flexes over
/// stance, pushes off and lifts the toe in swing. The pelvis drops on the
/// swing side and turns with the swinging leg; the spine takes both back,
/// so the shoulders stay level and turn against the pelvis, and the head
/// looks ahead. The arms swing against the legs, a little behind, the
/// elbow bending as the arm comes forward.
fn walk(b: &mut Builder<'_>, u: f32) {
    b.relax_arms();
    let drop = 4.0 * (TAU * u).sin();
    for (side, sign) in SIDES {
        let v = if sign > 0.0 { u } else { (u + 0.5).fract() };
        let hip = 3.0 + 19.0 * (TAU * (v - 0.04)).cos();
        let knee = 4.0
            + 14.0 * bump(v, 0.13, 0.07)
            + 12.0 * bump(v, 0.6, 0.05)
            + 54.0 * bump(v, 0.72, 0.11);
        let ankle = -4.0 * bump(v, 0.06, 0.04) + 8.0 * bump(v, 0.42, 0.12)
            - 14.0 * bump(v, 0.61, 0.06)
            + 3.0 * bump(v, 0.82, 0.08);
        // The pelvis's drop is taken back at the hip: the leg stays
        // upright.
        b.turn(&format!("thigh_{side}"), Vec3::X, -hip);
        b.turn(&format!("thigh_{side}"), Vec3::Z, -drop);
        b.turn(&format!("shin_{side}"), Vec3::X, knee);
        b.turn(&format!("foot_{side}"), Vec3::X, -ankle);
        // The arm swings forward while the other side's leg does.
        let swing = -15.0 * (TAU * v).cos();
        b.turn(&format!("upper_arm_{side}"), Vec3::X, 2.0 - swing);
        b.elbow(side, 6.0 + 14.0 * swing.max(0.0) / 15.0);
    }
    // Left hip up while the left foot is on the ground (drop > 0).
    b.turn("pelvis", Vec3::Z, drop);
    b.turn("spine1", Vec3::Z, -0.7 * drop);
    b.turn("spine2", Vec3::Z, -0.3 * drop);
    let twist = (TAU * u).cos();
    b.turn("pelvis", Vec3::Y, -5.0 * twist);
    for bone in ["spine3", "chest", "upper_chest"] {
        b.turn(bone, Vec3::Y, 3.0 * twist);
    }
    // The chest ends up turned 4° against the pelvis: the head turns back.
    b.turn("neck", Vec3::Y, -2.0 * twist);
    b.turn("head", Vec3::Y, -2.0 * twist);
}

/// Standing at rest over eight seconds: the weight goes onto the left leg,
/// over to the right one and back (the hips move about 3 cm over the
/// loaded foot, its side of the pelvis up, the other knee easing), two
/// breaths lift the chest and the shoulders, the head looks a little
/// around, the arms hang relaxed. Every term repeats within the cycle, so
/// it loops without a jump.
fn idle(b: &mut Builder<'_>, u: f32) {
    b.relax_arms();
    let weight = (TAU * u).sin();
    // Both legs lean under the hips, and the body moves back over the
    // feet: the hips, not the feet, go to the side.
    let lean = 2.0 * weight;
    let height = |name: &str| {
        b.skeleton
            .find(name)
            .and_then(|i| b.skeleton.bones.get(i))
            .map_or(0.0, |bone| bone.head.y)
    };
    let length = height("thigh_l") - height("foot_l");
    for (side, sign) in SIDES {
        // The unloaded leg's knee eases forward.
        let ease_off = 6.0 * (-sign * weight).max(0.0);
        b.turn(&format!("thigh_{side}"), Vec3::X, -0.5 * ease_off);
        b.turn(&format!("thigh_{side}"), Vec3::Z, -lean - 1.5 * weight);
        b.turn(&format!("shin_{side}"), Vec3::X, ease_off);
        b.turn(&format!("foot_{side}"), Vec3::X, -0.5 * ease_off);
    }
    b.pose.offset.x = length * lean.to_radians().sin();
    // The loaded side of the pelvis up; the spine keeps the trunk upright.
    b.turn("pelvis", Vec3::Z, 1.5 * weight);
    b.turn("spine1", Vec3::Z, -weight);
    b.turn("spine2", Vec3::Z, -0.5 * weight);
    let breath = (2.0 * TAU * u).sin();
    b.turn("chest", Vec3::X, -0.8 * breath);
    b.turn("upper_chest", Vec3::X, -0.5 * breath);
    for (side, sign) in SIDES {
        b.turn(&format!("clavicle_{side}"), Vec3::Z * sign, breath);
        b.turn(
            &format!("upper_arm_{side}"),
            Vec3::X,
            2.0 * (TAU * u + 0.7).sin(),
        );
    }
    b.turn("neck", Vec3::Y, 2.0 * (TAU * u + 1.0).sin());
    b.turn("head", Vec3::Y, 3.0 * (TAU * u + 1.0).sin());
    b.turn("head", Vec3::X, 1.5 * (2.0 * TAU * u + 0.5).sin());
}

/// Half a turn to the left over two seconds, a pause, back over two
/// seconds, a pause; about the vertical between the feet.
fn turn(b: &mut Builder<'_>, u: f32) {
    b.relax_arms();
    let s = u * 6.0;
    let amount = if s < 2.0 {
        ease(s / 2.0)
    } else if s < 3.0 {
        1.0
    } else if s < 5.0 {
        1.0 - ease((s - 3.0) / 2.0)
    } else {
        0.0
    };
    let Some(pelvis) = b.skeleton.find("pelvis") else {
        return;
    };
    let Some(head) = b.skeleton.bones.get(pelvis).map(|p| p.head) else {
        return;
    };
    let feet: Vec<Vec3> = ["foot_l", "foot_r"]
        .iter()
        .filter_map(|n| Some(b.skeleton.bones.get(b.skeleton.find(n)?)?.head))
        .collect();
    let centre = feet.iter().sum::<Vec3>() / feet.len().max(1) as f32;
    let pivot = Vec3::new(centre.x, head.y, centre.z);
    let q = Quat::from_rotation_y((180.0 * amount).to_radians());
    b.turn("pelvis", Vec3::Y, 180.0 * amount);
    // Turning about the pelvis's head, then shifted: as if about `pivot`.
    b.pose.offset = q * (head - pivot) + pivot - head;
}

/// Both arms raised over 1.5 s to about 40° above the horizontal, held,
/// lowered over 1.5 s, held. Higher, linear blend skinning folds the skin
/// of the armpit onto itself (MakeHuman's weights are made for the A
/// pose), and cloth caught in the fold is squeezed out.
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
    for (side, sign) in SIDES {
        b.turn(&format!("clavicle_{side}"), Vec3::Z * sign, 15.0 * amount);
        b.turn(&format!("upper_arm_{side}"), Vec3::Z * sign, 70.0 * amount);
        b.elbow(side, 10.0 * amount);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::model::{BodyModel, BodyShape};
    use crate::skeleton::skin;

    #[test]
    fn motions_stay_on_the_floor_and_move_the_body() {
        let model = BodyModel::builtin().unwrap();
        let body = model.shaped(&BodyShape::default());
        let skeleton = Skeleton::new(&model, &body);
        let still = Motion::Walk.pose(&skeleton, 0.0);
        assert!(
            still
                .rotations
                .iter()
                .all(|q| q.abs_diff_eq(Quat::IDENTITY, 1e-6))
        );
        let wrist = |pose: &Pose, side: &str| {
            let i = skeleton.find(&format!("hand_{side}")).unwrap();
            skeleton.transforms(pose)[i].apply(skeleton.bones[i].head)
        };
        for motion in Motion::BUILT_IN {
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
        // Raising the arms: the hands go above the shoulders, beside the
        // head.
        let up = Motion::RaiseArms.pose(&skeleton, 1.75);
        let neck = skeleton.bones[skeleton.find("neck").unwrap()].head.y;
        assert!(wrist(&up, "l").y > neck + 0.1 && wrist(&up, "r").y > neck + 0.1);
        // Turning: half round, the left hand is on the right.
        let round = Motion::Turn.pose(&skeleton, 2.5);
        assert!(wrist(&round, "l").x < -0.1);
    }
}
