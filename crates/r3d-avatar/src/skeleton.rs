//! The skeleton of a shaped body, its poses and skinning.
//!
//! A pose turns each bone about its head joint; rotations are written in
//! the rest pose's axes (X to the body's left, Y up, Z forward) and carried
//! by the parent bones' motion (forward kinematics). The skin follows by
//! linear blend skinning (Magnenat-Thalmann, Laperrière, Thalmann, "Joint-
//! dependent local deformations for hand animation and object grasping",
//! Graphics Interface 1988): each vertex moves with the weighted average of
//! its bones' motions.

use glam::{Quat, Vec3};
use r3d_geom::Rigid;

use crate::model::{BodyModel, Shaped};
use crate::pack::Influences;

/// Vertices of a foot this close to the floor in the rest pose are its
/// sole (m).
const SOLE_HEIGHT_M: f32 = 0.02;

/// A bone of a shaped body.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonBone {
    pub name: String,
    /// Always lower than the bone's own index.
    pub parent: Option<usize>,
    /// The joint the bone turns about (m).
    pub head: Vec3,
    /// The joint at its other end (m).
    pub tail: Vec3,
}

impl SkeletonBone {
    /// Unit direction from head to tail.
    pub fn direction(&self) -> Vec3 {
        (self.tail - self.head).normalize_or(Vec3::NEG_Y)
    }
}

/// A shaped body's skeleton.
#[derive(Debug, Clone)]
pub struct Skeleton {
    pub bones: Vec<SkeletonBone>,
    /// Points under the feet (bone, rest position): poses keep the lowest
    /// of them on the floor.
    soles: Vec<(usize, Vec3)>,
}

/// Each bone's rotation about its head joint, in the rest pose's axes,
/// and an offset of the whole body.
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    pub rotations: Vec<Quat>,
    pub offset: Vec3,
}

impl Pose {
    /// The rest pose of a skeleton with `bones` bones.
    pub fn rest(bones: usize) -> Self {
        Self {
            rotations: vec![Quat::IDENTITY; bones],
            offset: Vec3::ZERO,
        }
    }

    /// Between the rest pose (`w` = 0) and this one (`w` = 1).
    pub fn scaled(&self, w: f32) -> Self {
        Self {
            rotations: self
                .rotations
                .iter()
                .map(|q| Quat::IDENTITY.slerp(*q, w))
                .collect(),
            offset: self.offset * w,
        }
    }
}

impl Skeleton {
    /// The skeleton of `body`, a shape of `model` (fitted or not).
    pub fn new(model: &BodyModel, body: &Shaped) -> Self {
        let joint = |i: u32| body.joints.get(i as usize).copied().unwrap_or_default();
        let bones: Vec<SkeletonBone> = model
            .bones()
            .iter()
            .map(|b| SkeletonBone {
                name: b.name.clone(),
                parent: b.parent.map(|p| p as usize),
                head: joint(b.head),
                tail: joint(b.tail),
            })
            .collect();
        let feet: Vec<usize> = bones
            .iter()
            .enumerate()
            .filter(|(_, b)| b.name.starts_with("foot"))
            .map(|(i, _)| i)
            .collect();
        let soles = body
            .positions
            .iter()
            .zip(model.weights())
            .filter(|(p, _)| p.y < SOLE_HEIGHT_M)
            .filter_map(|(p, w)| {
                let bone = usize::from(w.bones[0]);
                feet.contains(&bone).then_some((bone, *p))
            })
            .collect();
        Self { bones, soles }
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }

    /// Each bone's motion from the rest pose to `pose`, the body then
    /// raised or lowered so that its lowest sole point is on the floor,
    /// as in the rest pose.
    pub fn transforms(&self, pose: &Pose) -> Vec<Rigid> {
        let mut out: Vec<Rigid> = Vec::with_capacity(self.bones.len());
        for (i, bone) in self.bones.iter().enumerate() {
            let turn = pose.rotations.get(i).copied().unwrap_or(Quat::IDENTITY);
            let local = Rigid::about(bone.head, turn.normalize());
            let parent = match bone.parent.and_then(|p| out.get(p)) {
                Some(p) => *p,
                None => Rigid {
                    rotation: Quat::IDENTITY,
                    translation: pose.offset,
                },
            };
            out.push(parent.after(&local));
        }
        let lowest = self
            .soles
            .iter()
            .filter_map(|(b, p)| Some(out.get(*b)?.apply(*p).y))
            .fold(f32::INFINITY, f32::min);
        if lowest.is_finite() {
            for t in &mut out {
                t.translation.y -= lowest;
            }
        }
        out
    }
}

/// The skin moved by its bones: each vertex by the weighted average of
/// its bones' motions (linear blend skinning).
pub fn skin(positions: &[Vec3], weights: &[Influences], transforms: &[Rigid]) -> Vec<Vec3> {
    positions
        .iter()
        .zip(weights)
        .map(|(p, w)| {
            let mut sum = Vec3::ZERO;
            let mut total = 0.0;
            for (b, x) in w.bones.iter().zip(w.weights) {
                if x > 0.0
                    && let Some(t) = transforms.get(usize::from(*b))
                {
                    sum += t.apply(*p) * x;
                    total += x;
                }
            }
            if total > 0.0 { sum / total } else { *p }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::model::BodyShape;

    #[test]
    fn rest_pose_moves_nothing_and_bones_keep_their_length() {
        let model = BodyModel::builtin().unwrap();
        let body = model.shaped(&BodyShape::default());
        let skeleton = Skeleton::new(&model, &body);
        assert_eq!(skeleton.bones.len(), 24);
        assert!(skeleton.soles.len() > 20, "{}", skeleton.soles.len());
        let rest = skeleton.transforms(&Pose::rest(skeleton.bones.len()));
        let skinned = skin(&body.positions, model.weights(), &rest);
        let moved = skinned
            .iter()
            .zip(&body.positions)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0, f32::max);
        assert!(moved < 1e-5, "{moved}");

        // Bend the left knee: the shin turns, the thigh stays.
        let mut pose = Pose::rest(skeleton.bones.len());
        let shin = skeleton.find("shin_l").unwrap();
        let thigh = skeleton.find("thigh_l").unwrap();
        pose.rotations[shin] = Quat::from_rotation_x(1.0);
        let t = skeleton.transforms(&pose);
        let b = &skeleton.bones[shin];
        let length = t[shin].apply(b.tail).distance(t[shin].apply(b.head));
        assert!((length - b.head.distance(b.tail)).abs() < 1e-5);
        assert!(t[thigh].apply(b.head).distance(b.head) < 1e-5);
        assert!(t[shin].apply(b.tail).z < b.tail.z - 0.05);
    }
}
