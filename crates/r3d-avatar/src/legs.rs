//! Legs placed by inverse kinematics: a motion says where each foot is
//! (planted on the floor, or in the air) and the thigh and the shin turn
//! so that the leg reaches it, the knee bending in the plane of the hip,
//! the ankle and the way the knee points. Two bones and a hinge, solved in
//! closed form by the law of cosines (as in Tolani, Goswami, Badler,
//! "Real-time inverse kinematics techniques for anthropomorphic limbs",
//! Graphical Models 62(5), 2000, §3).

use glam::{Mat3, Quat, Vec3};
use r3d_geom::Rigid;

use crate::skeleton::{Pose, Skeleton};

/// Sole points this close to the lowest one in the rest pose bear the
/// weight (m); the heel and the ball of the foot are the back and the
/// front of them.
const CONTACT_M: f32 = 0.005;

/// A leg of a skeleton, in the rest pose.
#[derive(Debug, Clone)]
pub(crate) struct Leg {
    thigh: usize,
    shin: usize,
    foot: usize,
    hip: Vec3,
    knee: Vec3,
    pub(crate) ankle: Vec3,
    /// The farthest the ankle may be from the hip: as at rest, so the
    /// knee never straightens past its rest bend.
    reach: f32,
    /// The way the knee points at rest, square to the hip–ankle line.
    bend: Vec3,
    /// The back and the front of the part of the sole that bears weight.
    pub(crate) heel: Vec3,
    pub(crate) toe: Vec3,
    sole: Vec<Vec3>,
}

impl Leg {
    /// The leg on `side` (`l` or `r`); `None` when the skeleton has no
    /// such leg or its foot no sole.
    pub(crate) fn new(skeleton: &Skeleton, side: &str) -> Option<Self> {
        let thigh = skeleton.find(&format!("thigh_{side}"))?;
        let shin = skeleton.find(&format!("shin_{side}"))?;
        let foot = skeleton.find(&format!("foot_{side}"))?;
        let head = |i: usize| skeleton.bones.get(i).map(|b| b.head);
        let (hip, knee, ankle) = (head(thigh)?, head(shin)?, head(foot)?);
        let sole: Vec<Vec3> = skeleton.sole(foot).collect();
        let ground = sole.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let contact = || sole.iter().filter(|p| p.y < ground + CONTACT_M);
        let heel = *contact().min_by(|a, b| a.z.total_cmp(&b.z))?;
        let toe = *contact().max_by(|a, b| a.z.total_cmp(&b.z))?;
        let line = (ankle - hip).normalize_or(Vec3::NEG_Y);
        let off = knee - hip;
        let bend = (off - line * off.dot(line)).normalize_or(Vec3::Z);
        Some(Self {
            bend,
            thigh,
            shin,
            foot,
            hip,
            knee,
            ankle,
            reach: hip.distance(ankle),
            heel,
            toe,
            sole,
        })
    }

    /// Both legs, left then right.
    pub(crate) fn both(skeleton: &Skeleton) -> Option<[Self; 2]> {
        Some([Self::new(skeleton, "l")?, Self::new(skeleton, "r")?])
    }

    /// The hip joint at rest.
    pub(crate) fn hip(&self) -> Vec3 {
        self.hip
    }

    /// The thigh's and the shin's lengths.
    pub(crate) fn lengths(&self) -> (f32, f32) {
        (self.hip.distance(self.knee), self.knee.distance(self.ankle))
    }

    /// The hip joint where the bones' motions `world` put it.
    pub(crate) fn hip_at(&self, skeleton: &Skeleton, world: &[Rigid]) -> Vec3 {
        skeleton
            .bones
            .get(self.thigh)
            .and_then(|b| b.parent)
            .and_then(|p| world.get(p))
            .map_or(self.hip, |g| g.apply(self.hip))
    }

    /// How far a hip at `hip` may go up (negative: how far it must come
    /// down) for the leg to reach the ankle of a foot moved by `foot`.
    pub(crate) fn room(&self, hip: Vec3, foot: &Rigid) -> f32 {
        let ankle = foot.apply(self.ankle);
        let across = (ankle.x - hip.x).hypot(ankle.z - hip.z);
        ankle.y - hip.y + (self.reach * self.reach - across * across).max(0.0).sqrt()
    }

    /// The height of the lowest point of the sole of a foot moved by
    /// `foot`.
    pub(crate) fn lowest(&self, foot: &Rigid) -> f32 {
        self.sole
            .iter()
            .map(|p| foot.apply(*p).y)
            .fold(f32::INFINITY, f32::min)
    }

    /// Turns the thigh, the shin and the foot of `pose` so that the foot
    /// moves by `foot`, the knee pointing as at rest turned by `heading`
    /// (about the vertical). Bones above the thigh keep their turns. A
    /// foot out of reach is reached for.
    pub(crate) fn place(&self, skeleton: &Skeleton, pose: &mut Pose, foot: &Rigid, heading: f32) {
        let forward = Quat::from_rotation_y(heading) * self.bend;
        let world = skeleton.forward(pose);
        let parent = skeleton
            .bones
            .get(self.thigh)
            .and_then(|b| b.parent)
            .and_then(|p| world.get(p).copied())
            .unwrap_or(Rigid::IDENTITY);
        let hip = parent.apply(self.hip);
        let (thigh, shin) = self.lengths();
        let to = foot.apply(self.ankle) - hip;
        let d = to.length().clamp((thigh - shin).abs() + 1e-4, self.reach);
        let down = to.try_normalize().unwrap_or(Vec3::NEG_Y);
        let ahead = (forward - down * forward.dot(down))
            .try_normalize()
            .unwrap_or(self.bend);
        let cos = ((thigh * thigh + d * d - shin * shin) / (2.0 * thigh * d)).clamp(-1.0, 1.0);
        let knee = hip + thigh * (cos * down + (1.0 - cos * cos).max(0.0).sqrt() * ahead);
        let ankle = hip + down * d;
        let hinge = ahead.cross(down);
        let rest_hinge = self
            .bend
            .cross((self.ankle - self.hip).normalize_or(Vec3::NEG_Y));
        let thigh_turn = aim(self.knee - self.hip, knee - hip, rest_hinge, hinge);
        let shin_turn = aim(self.ankle - self.knee, ankle - knee, rest_hinge, hinge);
        for (bone, q) in [
            (self.thigh, parent.rotation.inverse() * thigh_turn),
            (self.shin, thigh_turn.inverse() * shin_turn),
            (self.foot, shin_turn.inverse() * foot.rotation),
        ] {
            if let Some(slot) = pose.rotations.get_mut(bone) {
                *slot = q.normalize();
            }
        }
    }
}

/// The rotation taking the direction `from` to `to`, and the axis
/// `from_hinge` (made square to `from`) to `hinge` (made square to `to`).
fn aim(from: Vec3, to: Vec3, from_hinge: Vec3, hinge: Vec3) -> Quat {
    let frame = |along: Vec3, across: Vec3| {
        let a = along.normalize_or(Vec3::NEG_Y);
        let c = (across - a * across.dot(a)).normalize_or(Vec3::X);
        Mat3::from_cols(a, c, a.cross(c))
    };
    Quat::from_mat3(&(frame(to, hinge) * frame(from, from_hinge).transpose())).normalize()
}

/// A foot's motion from rest: turned by `heading` about the vertical and
/// by `pitch` about its across axis (toes up for a positive pitch, both
/// in radians), its rest point `pivot` brought to `at`.
pub(crate) fn foot_at(heading: f32, pitch: f32, pivot: Vec3, at: Vec3) -> Rigid {
    let rotation = Quat::from_rotation_y(heading) * Quat::from_rotation_x(-pitch);
    Rigid {
        rotation,
        translation: at - rotation * pivot,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::model::{BodyModel, BodyShape};

    #[test]
    fn a_foot_is_reached_where_it_is_put() {
        let model = BodyModel::builtin().unwrap();
        let body = model.shaped(&BodyShape::default());
        let skeleton = Skeleton::new(&model, &body);
        let [left, _] = Leg::both(&skeleton).unwrap();
        assert!(left.heel.z < left.ankle.z && left.toe.z > left.ankle.z + 0.1);
        let foot_bone = skeleton.find("foot_l").unwrap();
        let shin_bone = skeleton.find("shin_l").unwrap();
        // The pelvis lowered 6 cm, the foot stepped forward 20 cm and its
        // toes raised: the leg reaches it, the knee bent forward.
        let mut pose = Pose::rest(skeleton.bones.len());
        pose.offset = Vec3::new(0.0, -0.06, 0.0);
        let foot = foot_at(0.1, 0.2, left.heel, left.heel + Vec3::new(0.0, 0.0, 0.2));
        pose.offset.y = pose.offset.y.min(left.room(left.hip + pose.offset, &foot));
        left.place(&skeleton, &mut pose, &foot, 0.0);
        let world = skeleton.forward(&pose);
        let ankle = world[foot_bone].apply(left.ankle);
        assert!(ankle.distance(foot.apply(left.ankle)) < 1e-4, "{ankle}");
        assert!(world[foot_bone].rotation.angle_between(foot.rotation) < 1e-3);
        let knee = world[shin_bone].apply(skeleton.bones[shin_bone].head);
        assert!(knee.z > left.hip.z + 0.05, "{knee}");
        // At rest the leg rests.
        let mut rest = Pose::rest(skeleton.bones.len());
        left.place(&skeleton, &mut rest, &Rigid::IDENTITY, 0.0);
        assert!(
            rest.rotations
                .iter()
                .all(|q| q.angle_between(Quat::IDENTITY) < 1e-3)
        );
        assert!(left.room(left.hip, &Rigid::IDENTITY).abs() < 1e-5);
    }
}
