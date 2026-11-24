//! The walk: on the spot, as on a treadmill, at an easy pace.
//!
//! A planted foot moves back with the belt at one steady speed and does
//! not slip: it lands on its heel, rolls flat about it, stays flat, then
//! rolls up onto the ball of the foot and pushes off ([`Gait::planted`]).
//! The swinging foot follows the swing of published joint angle curves
//! (hip, knee and ankle in the sagittal plane, every 5 % of the cycle),
//! bent to leave exactly where the foot pushed off and to land exactly
//! where it strikes ([`Gait::swing`]). The legs reach the feet by inverse
//! kinematics. The pelvis drops on the side of the swinging leg, turns
//! with it, sways over the loaded foot and rises and falls twice a cycle;
//! the thorax turns against it, the head stays level, and the arms swing
//! against the legs, the elbow bending as the arm comes forward.
//!
//! Sources (general shapes and ranges, rounded; no data set is copied):
//! the sagittal joint angles of natural cadence in D. A. Winter, *The
//! Biomechanics and Motor Control of Human Gait*, University of Waterloo
//! Press, 2nd ed. 1991, and M. P. Kadaba, H. K. Ramakrishnan, M. E.
//! Wootten, "Measurement of lower extremity kinematics during level
//! walking", Journal of Orthopaedic Research 8(3), 1990 (also the
//! pelvis's obliquity and rotation); the phases of the cycle (heel
//! strike, foot flat, heel off, toe off at about 62 %) in J. Perry, *Gait
//! Analysis: Normal and Pathological Function*, 1992; the pelvis's
//! vertical and lateral travel, the thorax turning against it and the
//! arm swing in M. P. Murray, A. B. Drought, R. C. Kory, "Walking
//! patterns of normal men", Journal of Bone and Joint Surgery 46-A(2),
//! 1964; the knee and the pelvis smoothing the body's path in J. B.
//! Saunders, V. T. Inman, H. D. Eberhart, "The major determinants in
//! normal and pathological gait", same journal, 35-A(3), 1953. The
//! amplitudes are kept at the low end of those ranges, for a calm walk.

use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};
use r3d_geom::Rigid;

use crate::legs::{Leg, foot_at};
use crate::motion::{Builder, SIDES, Stance, drop_at, ease, periodic};
use crate::skeleton::Pose;

/// One stride (two steps) at about 105 steps a minute (s).
pub(crate) const PERIOD_S: f32 = 1.15;

/// Where the cycle of a leg, from its heel strike, reaches foot flat, heel
/// off and toe off.
const FOOT_FLAT: f32 = 0.08;
const HEEL_OFF: f32 = 0.40;
const TOE_OFF: f32 = 0.62;
/// Stride length over the height of the hip joint.
const STRIDE: f32 = 1.2;
/// Where the feet are put across, as a share of the hip joints' width.
const WIDTH: f32 = 0.85;
/// Each foot turned out (degrees).
const TOE_OUT: f32 = 6.0;
/// The foot's angle to the floor at heel strike and at toe off (degrees,
/// toes up). The first is the shank's lean in the curves below.
const STRIKE_PITCH: f32 = 18.0;
const PUSH_PITCH: f32 = -32.0;
/// Pelvis: drop on the swing side and turn (degrees), sway over the
/// loaded foot (m). It rises and falls as the legs let it
/// ([`drop_at`]).
const OBLIQUITY: f32 = 4.0;
const ROTATION: f32 = 4.5;
const SWAY_M: f32 = 0.02;
/// The thorax turning against the pelvis (degrees, from the rest pose)
/// and leaning forward.
const THORAX: f32 = 3.0;
const LEAN: f32 = 2.0;
/// Arms: forward swing about a mean a little back (degrees), and the
/// elbow's added bend.
const ARM_SWING: f32 = 15.0;
const ARM_MEAN: f32 = -3.0;
const ELBOW: f32 = 10.0;
/// The swinging foot's lowest point clears the floor by this at mid
/// swing (m).
const CLEARANCE_M: f32 = 0.012;

/// Thigh forward from the vertical, knee bend and ankle dorsiflexion over
/// the cycle from heel strike, every 5 % (degrees).
const THIGH: [f32; 20] = [
    21.0, 20.0, 18.0, 14.0, 10.0, 6.0, 2.0, -1.0, -4.0, -7.0, -10.0, -12.0, -11.0, -6.0, 2.0, 10.0,
    16.0, 20.0, 22.0, 22.0,
];
const KNEE: [f32; 20] = [
    3.0, 12.0, 17.0, 18.0, 15.0, 10.0, 6.0, 4.0, 4.0, 6.0, 10.0, 20.0, 35.0, 50.0, 60.0, 58.0,
    45.0, 28.0, 12.0, 5.0,
];
const ANKLE: [f32; 20] = [
    0.0, -5.0, -3.0, 1.0, 4.0, 6.0, 8.0, 9.0, 10.0, 10.0, 7.0, 0.0, -12.0, -17.0, -12.0, -6.0,
    -2.0, 0.0, 1.0, 1.0,
];

/// The walk of one body at one stage of its start (`grown`, 0 at rest to
/// 1).
struct Gait<'a> {
    legs: &'a [Leg; 2],
    grown: f32,
    stride: f32,
    /// Where each ankle is across (m) and each foot's heading (radians).
    across: [f32; 2],
    heading: [f32; 2],
    /// Where the heel strikes, forward (m).
    strike: f32,
}

impl<'a> Gait<'a> {
    fn new(legs: &'a [Leg; 2], grown: f32) -> Self {
        let hip_height = 0.5 * (legs[0].hip().y + legs[1].hip().y);
        let mut across = [0.0; 2];
        let mut heading = [0.0; 2];
        for (i, (leg, (_, sign))) in legs.iter().zip(SIDES).enumerate() {
            across[i] = leg.ankle.x + (WIDTH * leg.hip().x - leg.ankle.x) * grown;
            heading[i] = (sign * TOE_OUT * grown).to_radians();
        }
        let stride = STRIDE * hip_height * grown;
        // A quarter into the cycle the ankle passes under the hip, so that
        // at heel strike the leading and the trailing feet are about as
        // far from it.
        let leg = &legs[0];
        let strike = leg.hip().z - (leg.ankle.z - leg.heel.z) + 0.25 * stride;
        Self {
            legs,
            grown,
            stride,
            across,
            heading,
            strike: leg.heel.z + (strike - leg.heel.z) * grown,
        }
    }

    /// The phase of leg `i` at the walk's phase `u`: the left heel
    /// strikes at 0, the right one half a cycle later.
    fn phase(i: usize, u: f32) -> f32 {
        (u - 0.5 * i as f32).rem_euclid(1.0)
    }

    /// The pelvis's turn about the vertical at phase `u` (radians): the
    /// left hip forward as the left heel strikes.
    fn yaw(&self, u: f32) -> f32 {
        (-ROTATION * self.grown * (TAU * u).cos()).to_radians()
    }

    /// The pelvis's turn and the body's offset at phase `u`.
    fn pelvis(&self, u: f32) -> (Quat, Vec3) {
        let g = self.grown;
        let turn = Quat::from_rotation_y(self.yaw(u))
            * Quat::from_rotation_z((OBLIQUITY * g * (TAU * (u + 0.05)).sin()).to_radians());
        let offset = Vec3::X * SWAY_M * g * (TAU * u).sin();
        (turn, offset)
    }

    /// Where the walk puts the body at phase `u`, feet in the air left
    /// out.
    fn stance(&self, u: f32) -> Stance {
        let (pelvis, offset) = self.pelvis(u);
        let mut feet = [None, None];
        for (i, slot) in feet.iter_mut().enumerate() {
            let v = Self::phase(i, u);
            if v <= TOE_OFF {
                *slot = Some(self.planted(i, v));
            }
        }
        Stance {
            pelvis,
            offset,
            feet,
        }
    }

    /// Foot `i` on the floor at phase `v` of its stance: its heel lands
    /// at [`Gait::strike`] and goes back with the belt; the foot rolls
    /// about the heel to flat, then about its front up to toe off.
    fn planted(&self, i: usize, v: f32) -> Rigid {
        let Some(leg) = self.legs.get(i) else {
            return Rigid::IDENTITY;
        };
        let heading = self.heading.get(i).copied().unwrap_or(0.0);
        let across = self.across.get(i).copied().unwrap_or(leg.ankle.x);
        let v = v.clamp(0.0, TOE_OFF);
        let heel = Vec3::new(
            across - (leg.ankle.x - leg.heel.x),
            leg.heel.y,
            self.strike - self.stride * v,
        );
        let flat = foot_at(heading, 0.0, leg.heel, heel);
        let g = self.grown;
        let mut foot = if v < FOOT_FLAT {
            let pitch = STRIKE_PITCH * g * (1.0 - ease(v / FOOT_FLAT));
            foot_at(heading, pitch.to_radians(), leg.heel, heel)
        } else if v < HEEL_OFF {
            flat
        } else {
            let x = (v - HEEL_OFF) / (TOE_OFF - HEEL_OFF);
            let pitch = PUSH_PITCH * g * x * x;
            foot_at(heading, pitch.to_radians(), leg.toe, flat.apply(leg.toe))
        };
        // On the floor: the point it rolls about may be a little above
        // the sole's lowest one, and a rounded heel or toe dips below it.
        foot.translation.y -= leg.lowest(&foot);
        foot
    }

    /// The ankle relative to the hip and the foot's angle to the floor
    /// (degrees) that the published curves give at phase `v`.
    fn curves(&self, i: usize, v: f32) -> (Vec3, f32) {
        let g = self.grown;
        let (thigh, knee) = (periodic(&THIGH, v) * g, periodic(&KNEE, v) * g);
        let shank = thigh - knee;
        let Some(leg) = self.legs.get(i) else {
            return (Vec3::ZERO, 0.0);
        };
        let (upper, lower) = leg.lengths();
        let along = |deg: f32, length: f32| {
            let a = deg.to_radians();
            Vec3::new(0.0, -a.cos(), a.sin()) * length
        };
        (
            along(thigh, upper) + along(shank, lower),
            shank + periodic(&ANKLE, v) * g,
        )
    }

    /// Foot `i` in the air at phase `v` of its cycle, the hip at `hip`;
    /// `hip_at(v)` says where the hip is at another phase of this leg.
    fn swing(&self, i: usize, v: f32, hip: Vec3, hip_at: impl Fn(f32) -> Vec3) -> Rigid {
        let Some(leg) = self.legs.get(i) else {
            return Rigid::IDENTITY;
        };
        let heading = self.heading.get(i).copied().unwrap_or(0.0);
        let g = self.grown;
        let lift = self.planted(i, TOE_OFF);
        let land = self.planted(i, 0.0);
        let (at_lift, pitch_lift) = self.curves(i, TOE_OFF);
        let (at_land, pitch_land) = self.curves(i, 1.0);
        let off_lift = lift.apply(leg.ankle) - hip_at(TOE_OFF) - at_lift;
        let off_land = land.apply(leg.ankle) - hip_at(1.0) - at_land;
        let x = ((v - TOE_OFF) / (1.0 - TOE_OFF)).clamp(0.0, 1.0);
        // The difference at toe off fades over the first third of the
        // swing, the one at heel strike grows over its second half: in
        // between the foot follows the curves.
        let (leave, reach) = (1.0 - ease(x / 0.35), ease((x - 0.5) / 0.5));
        let (at, pitch) = self.curves(i, v);
        let ankle = hip + at + off_lift * leave + off_land * reach;
        let pitch =
            pitch + leave * (PUSH_PITCH * g - pitch_lift) + reach * (STRIKE_PITCH * g - pitch_land);
        let mut foot = foot_at(heading, pitch.to_radians(), leg.ankle, ankle);
        let clear = CLEARANCE_M * g * (PI * x).sin();
        let low = leg.lowest(&foot);
        if low < clear {
            foot.translation.y += clear - low;
        }
        foot
    }
}

/// The walk at phase `u` of its cycle (the left heel striking at 0),
/// grown from the rest pose by `grown`.
pub(crate) fn walk(b: &mut Builder<'_>, u: f32, grown: f32) {
    b.relax_arms(grown);
    let Some(legs) = Leg::both(b.skeleton) else {
        return;
    };
    let gait = Gait::new(&legs, grown);
    let drop = drop_at(b.skeleton, &legs, PERIOD_S, u, |v| gait.stance(v));
    let now = gait.stance(u);
    b.set_pelvis(&now, drop);
    let g = grown;
    // The spine takes back the pelvis's drop (the shoulders stay level)
    // and its turn: the thorax turns the other way.
    let obliquity = OBLIQUITY * g * (TAU * (u + 0.05)).sin();
    b.turn("spine1", Vec3::Z, -0.7 * obliquity);
    b.turn("spine2", Vec3::Z, -0.3 * obliquity);
    b.turn("spine1", Vec3::X, LEAN * g);
    let twist = (THORAX + ROTATION) * g * (TAU * u).cos();
    for bone in ["spine2", "spine3", "chest"] {
        b.turn(bone, Vec3::Y, twist / 3.0);
    }
    // Each arm swings with the other side's leg, a little late; the elbow
    // bends most just after the arm is forward, the wrist follows.
    for (i, (side, _)) in SIDES.iter().enumerate() {
        let a = Gait::phase(i, u);
        let forward = g * (ARM_MEAN - ARM_SWING * (TAU * (a - 0.04)).cos());
        b.turn(&format!("upper_arm_{side}"), Vec3::X, -forward);
        b.elbow(side, g * ELBOW * 0.5 * (1.0 - (TAU * (a - 0.58)).cos()));
        b.wrist(side, g * 3.0 * (TAU * (a - 0.62)).sin());
    }
    b.steady_head(Quat::IDENTITY);
    // The feet: planted ones where they are, swinging ones from where the
    // hips are now and were when they left.
    let skeleton = b.skeleton;
    let hip_at = |i: usize, u: f32| {
        let s = gait.stance(u);
        let mut pose = Pose::rest(skeleton.bones.len());
        if let Some(q) = skeleton
            .find("pelvis")
            .and_then(|p| pose.rotations.get_mut(p))
        {
            *q = s.pelvis;
        }
        pose.offset = s.offset + Vec3::Y * drop;
        let world = skeleton.forward(&pose);
        legs.get(i)
            .map_or(Vec3::ZERO, |l| l.hip_at(skeleton, &world))
    };
    for (i, leg) in legs.iter().enumerate() {
        let v = Gait::phase(i, u);
        let foot = match now.feet.get(i).copied().flatten() {
            Some(foot) => foot,
            None => gait.swing(i, v, hip_at(i, u), |p| hip_at(i, u + p - v)),
        };
        let heading = gait.heading.get(i).copied().unwrap_or(0.0);
        leg.place(skeleton, &mut b.pose, &foot, 0.5 * (heading + gait.yaw(u)));
    }
}
