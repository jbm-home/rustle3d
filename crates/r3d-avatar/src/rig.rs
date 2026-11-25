//! Our skeleton on a body that is not ours: an imported one (spec 27).
//!
//! The motions are written for our skeleton standing in its rest (A)
//! pose. A body from a file stands as its author left it (an A or a T
//! pose, arms closer or wider) and has its own proportions. So the
//! skeleton of the built-in body fitted to its measurements (the
//! reference) is laid onto it: each joint is placed on the body
//! ([`Anchors`]: from the file's own skeleton, found by its joints'
//! names, or from the body's arrangement points), the bones between
//! follow the reference's proportions ([`Targets`]), and [`bind`] splits
//! the result into
//!
//! - a rest skeleton with the body's bone lengths and the reference's
//!   bone directions, so every motion plays on it as on ours;
//! - the bind pose, which turns that rest skeleton into the body as it
//!   stands in the file.
//!
//! A motion then moves the body by each bone's motion from the bind pose
//! to the motion's pose. (Our own scheme.)

use glam::{Quat, Vec3};

use crate::captured::{MotionError, limb_bone, name_joints};
use crate::skeleton::{Pose, Skeleton, SkeletonBone};

/// A bone shorter than this has no direction of its own (m).
const POINT_M: f32 = 1e-3;

/// Where an arm's or a leg's joints are on a body (m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limb {
    /// The shoulder or the hip.
    pub top: Vec3,
    /// The elbow or the knee.
    pub middle: Vec3,
    /// The wrist or the ankle.
    pub end: Vec3,
}

/// The joints of a body that place our skeleton on it (m, the body's
/// frame: Y up, facing +Z, its left at +x).
#[derive(Debug, Clone, PartialEq)]
pub struct Anchors {
    /// The pelvis bone's head, between the hips.
    pub pelvis: Vec3,
    /// The bottom of the neck, if known.
    pub neck: Option<Vec3>,
    /// The bottom of the head.
    pub head: Vec3,
    /// Per side, left first: the inner end of the clavicle, if known.
    pub clavicles: [Option<Vec3>; 2],
    pub arms: [Limb; 2],
    pub legs: [Limb; 2],
}

/// The head and tail of each of the reference skeleton's bones, on a
/// body (m).
#[derive(Debug, Clone, PartialEq)]
pub struct Targets(pub Vec<[Vec3; 2]>);

/// Our anchors found in a file's skeleton, and for each of its joints the
/// bone of ours whose motion it takes (its own, or its nearest
/// ancestor's that has one; the pelvis's otherwise).
#[derive(Debug, Clone, PartialEq)]
pub struct FileSkeleton {
    pub anchors: Anchors,
    pub bones: Vec<usize>,
}

/// `p` carried by the similarity (turn, uniform scale, shift) that takes
/// segment `from` onto segment `to`.
fn similar(from: [Vec3; 2], to: [Vec3; 2], p: Vec3) -> Vec3 {
    let (a, b) = (from[1] - from[0], to[1] - to[0]);
    let (la, lb) = (a.length(), b.length());
    if la < POINT_M || lb < POINT_M {
        return to[0] + (p - from[0]);
    }
    let turn = Quat::from_rotation_arc(a / la, b / lb);
    to[0] + turn * (p - from[0]) * (lb / la)
}

/// Points found the same way on the reference body and on another
/// (the arrangement points' guess, spec 27), m.
pub struct Guides<F: Fn(f32) -> Vec3> {
    /// The middle of the torso at a height.
    pub middle: F,
    /// Per arm, left first: shoulder, elbow and wrist.
    pub arms: [[Vec3; 3]; 2],
    /// Per leg, left first: two points of its axis, the higher first.
    pub legs: [[Vec3; 2]; 2],
}

impl<F: Fn(f32) -> Vec3> std::fmt::Debug for Guides<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guides")
            .field("arms", &self.arms)
            .field("legs", &self.legs)
            .finish_non_exhaustive()
    }
}

impl Anchors {
    /// The anchors of a body from points found on it (`body`) and the
    /// same points on the `reference` body: each joint of the reference
    /// keeps its place relative to the points around it. The trunk's
    /// joints keep their offset from the torso's middle at their height;
    /// a limb's joints follow the similarity that takes the reference's
    /// limb points onto the body's.
    pub fn from_points<F: Fn(f32) -> Vec3, G: Fn(f32) -> Vec3>(
        reference: &Skeleton,
        on_reference: &Guides<F>,
        body: &Guides<G>,
    ) -> Self {
        let head_of = |name: &str| {
            reference
                .find(name)
                .and_then(|b| reference.bones.get(b))
                .map_or(Vec3::ZERO, |b| b.head)
        };
        let trunk = |name: &str| {
            let joint = head_of(name);
            (body.middle)(joint.y) + (joint - (on_reference.middle)(joint.y))
        };
        let arm = |r: [Vec3; 3], b: [Vec3; 3], letter: &str| {
            let upper = |j: Vec3| similar([r[0], r[1]], [b[0], b[1]], j);
            let fore = |j: Vec3| similar([r[1], r[2]], [b[1], b[2]], j);
            Limb {
                top: upper(head_of(&format!("upper_arm_{letter}"))),
                middle: upper(head_of(&format!("forearm_{letter}"))),
                end: fore(head_of(&format!("hand_{letter}"))),
            }
        };
        let leg = |r: [Vec3; 2], b: [Vec3; 2], letter: &str| {
            let along = |j: Vec3| similar(r, b, j);
            Limb {
                top: along(head_of(&format!("thigh_{letter}"))),
                middle: along(head_of(&format!("shin_{letter}"))),
                end: along(head_of(&format!("foot_{letter}"))),
            }
        };
        let ([ral, rar], [bal, bar]) = (on_reference.arms, body.arms);
        let ([rll, rlr], [bll, blr]) = (on_reference.legs, body.legs);
        Self {
            pelvis: trunk("pelvis"),
            neck: Some(trunk("neck")),
            head: trunk("head"),
            clavicles: [None, None],
            arms: [arm(ral, bal, "l"), arm(rar, bar, "r")],
            legs: [leg(rll, bll, "l"), leg(rlr, blr, "r")],
        }
    }
}

impl FileSkeleton {
    /// Our joints found in a file's skeleton by their names (as for a
    /// captured motion, spec 22): `joints` are (name, parent, position).
    /// Fails when one we need is missing.
    pub fn find(
        reference: &Skeleton,
        joints: &[(String, Option<usize>, Vec3)],
    ) -> Result<Self, MotionError> {
        let names: Vec<String> = joints.iter().map(|j| j.0.clone()).collect();
        let parents: Vec<Option<usize>> = joints.iter().map(|j| j.1).collect();
        let named = name_joints(&names, &parents)?;
        let at = |j: Option<usize>| j.and_then(|j| joints.get(j)).map(|j| j.2);
        let limb = |side: usize, k: usize| named.limbs.get(side).and_then(|l| l.get(k)).copied();
        let required = |side: usize, k: usize| {
            at(limb(side, k).flatten()).ok_or(MotionError::Skeleton {
                bone: limb_bone(k).unwrap_or("limb"),
            })
        };
        let mut arms = Vec::new();
        let mut legs = Vec::new();
        for side in 0..2 {
            arms.push(Limb {
                top: required(side, 1)?,
                middle: required(side, 2)?,
                end: required(side, 3)?,
            });
            legs.push(Limb {
                top: required(side, 4)?,
                middle: required(side, 5)?,
                end: required(side, 6)?,
            });
        }
        let pair = |v: &[Limb]| match v {
            [l, r] => Ok([*l, *r]),
            _ => Err(MotionError::Skeleton { bone: "limb" }),
        };
        let anchors = Anchors {
            pelvis: at(Some(named.hips)).unwrap_or_default(),
            neck: at(named.neck),
            head: at(Some(named.head)).unwrap_or_default(),
            clavicles: [at(limb(0, 0).flatten()), at(limb(1, 0).flatten())],
            arms: pair(&arms)?,
            legs: pair(&legs)?,
        };

        // Each joint's bone of ours.
        let mut own: Vec<Option<usize>> = vec![None; joints.len()];
        let mut set = |j: Option<usize>, bone: &str| {
            if let (Some(slot), Some(b)) = (j.and_then(|j| own.get_mut(j)), reference.find(bone)) {
                *slot = Some(b);
            }
        };
        set(Some(named.hips), "pelvis");
        // The spine's joints share our spine bones along it.
        const SPINE: [&str; 6] = [
            "pelvis",
            "spine1",
            "spine2",
            "spine3",
            "chest",
            "upper_chest",
        ];
        let last = named.spine.len().saturating_sub(1).max(1);
        for (k, j) in named.spine.iter().enumerate().skip(1) {
            let bone = SPINE
                .get((k * (SPINE.len() - 1)).div_ceil(last))
                .copied()
                .unwrap_or("upper_chest");
            set(Some(*j), bone);
        }
        set(named.neck, "neck");
        set(Some(named.head), "head");
        for (side, letter) in ["l", "r"].iter().enumerate() {
            for k in 0..7 {
                if let Some(bone) = limb_bone(k) {
                    set(limb(side, k).flatten(), &format!("{bone}_{letter}"));
                }
            }
            // A toe moves with the foot.
            set(limb(side, 7).flatten(), &format!("foot_{letter}"));
        }
        let pelvis = reference.find("pelvis").unwrap_or(0);
        let bones = (0..joints.len())
            .map(|j| {
                let mut at = Some(j);
                for _ in 0..joints.len() {
                    let Some(k) = at else { break };
                    if let Some(b) = own.get(k).copied().flatten() {
                        return b;
                    }
                    at = joints.get(k).and_then(|j| j.1);
                }
                pelvis
            })
            .collect();
        Ok(Self { anchors, bones })
    }
}

impl Targets {
    /// The reference skeleton's joints on a body with these anchors: the
    /// anchored joints where they are, the others carried along with the
    /// anchors around them, as the reference has them.
    pub fn new(reference: &Skeleton, a: &Anchors) -> Self {
        let bones = &reference.bones;
        let head_of = |name: &str| {
            reference
                .find(name)
                .and_then(|b| bones.get(b))
                .map_or(Vec3::ZERO, |b| b.head)
        };
        let (r_pelvis, r_neck, r_head) = (head_of("pelvis"), head_of("neck"), head_of("head"));
        let neck = a
            .neck
            .unwrap_or_else(|| similar([r_pelvis, r_head], [a.pelvis, a.head], r_neck));
        let trunk = |p: Vec3| similar([r_pelvis, r_neck], [a.pelvis, neck], p);
        let mut out: Vec<[Vec3; 2]> = Vec::with_capacity(bones.len());
        for bone in bones {
            let [head, tail] = [bone.head, bone.tail];
            let side = match bone.name.rsplit('_').next() {
                Some("l") => Some(0),
                Some("r") => Some(1),
                _ => None,
            };
            let stem = bone
                .name
                .strip_suffix("_l")
                .or_else(|| bone.name.strip_suffix("_r"))
                .unwrap_or(&bone.name);
            let letter = if side == Some(0) { "l" } else { "r" };
            let named = |s: &str| head_of(&format!("{s}_{letter}"));
            let arm = side.and_then(|s| a.arms.get(s));
            let leg = side.and_then(|s| a.legs.get(s));
            // The segment of the reference this bone lies along, and
            // where that segment is on the body.
            let along: Option<([Vec3; 2], [Vec3; 2])> = match (stem, arm, leg) {
                ("pelvis" | "spine1" | "spine2" | "spine3" | "chest" | "upper_chest", ..) => {
                    Some(([r_pelvis, r_neck], [a.pelvis, neck]))
                }
                ("neck" | "head", ..) => Some(([r_neck, r_head], [neck, a.head])),
                ("clavicle", Some(arm), _) => {
                    let inner = side
                        .and_then(|s| a.clavicles.get(s).copied().flatten())
                        .unwrap_or_else(|| trunk(head));
                    Some(([head, named("upper_arm")], [inner, arm.top]))
                }
                ("shoulder", Some(arm), _) => {
                    let clavicle = named("clavicle");
                    let inner = side
                        .and_then(|s| a.clavicles.get(s).copied().flatten())
                        .unwrap_or_else(|| trunk(clavicle));
                    Some(([clavicle, named("upper_arm")], [inner, arm.top]))
                }
                ("upper_arm", Some(arm), _) => {
                    Some(([head, named("forearm")], [arm.top, arm.middle]))
                }
                ("forearm", Some(arm), _) => Some(([head, named("hand")], [arm.middle, arm.end])),
                ("hand", Some(arm), _) => Some(([named("forearm"), head], [arm.middle, arm.end])),
                ("thigh", _, Some(leg)) => Some(([head, named("shin")], [leg.top, leg.middle])),
                ("shin", _, Some(leg)) => Some(([head, named("foot")], [leg.middle, leg.end])),
                ("foot", _, Some(leg)) => Some(([named("shin"), head], [leg.middle, leg.end])),
                _ => None,
            };
            out.push(match along {
                Some((from, to)) => [similar(from, to, head), similar(from, to, tail)],
                None => [trunk(head), trunk(tail)],
            });
        }
        Self(out)
    }
}

/// A bone's direction for laying it: from its head to its tail, or to
/// its first child's head when it has no length.
fn aim(bones: &[SkeletonBone], joints: &[[Vec3; 2]], i: usize) -> Option<Vec3> {
    let [head, tail] = *joints.get(i)?;
    if head.distance(tail) >= POINT_M {
        return (tail - head).try_normalize();
    }
    let child = bones.iter().position(|b| b.parent == Some(i))?;
    let [c, _] = *joints.get(child)?;
    (c.distance(head) >= POINT_M)
        .then(|| (c - head).try_normalize())
        .flatten()
}

/// Our skeleton on a body: the rest skeleton (the body's bone lengths,
/// the `reference`'s directions) and the bind pose that turns it into
/// the body as it stands (`targets`).
pub fn bind(reference: &Skeleton, targets: &Targets) -> (Vec<SkeletonBone>, Pose) {
    let bones = &reference.bones;
    let at_rest: Vec<[Vec3; 2]> = bones.iter().map(|b| [b.head, b.tail]).collect();
    let mut world: Vec<Quat> = Vec::with_capacity(bones.len());
    let mut rest: Vec<SkeletonBone> = Vec::with_capacity(bones.len());
    let mut pose = Pose::rest(bones.len());
    for (i, bone) in bones.iter().enumerate() {
        let [head, tail] = targets.0.get(i).copied().unwrap_or([bone.head, bone.tail]);
        let parent = bone
            .parent
            .and_then(|p| Some((world.get(p)?, rest.get(p)?)));
        let parent_turn = parent.map_or(Quat::IDENTITY, |(q, _)| *q);
        let turn = match (aim(bones, &at_rest, i), aim(bones, &targets.0, i)) {
            (Some(from), Some(to)) => Quat::from_rotation_arc(from, to),
            _ => parent_turn,
        };
        let rest_head = match (parent, bone.parent) {
            (Some((_, p)), Some(pi)) => {
                let parent_head = targets.0.get(pi).map_or(p.head, |t| t[0]);
                p.head + parent_turn.inverse() * (head - parent_head)
            }
            _ => head,
        };
        if let Some(r) = pose.rotations.get_mut(i) {
            *r = (parent_turn.inverse() * turn).normalize();
        }
        world.push(turn);
        rest.push(SkeletonBone {
            name: bone.name.clone(),
            parent: bone.parent,
            head: rest_head,
            tail: rest_head + turn.inverse() * (tail - head),
        });
    }
    (rest, pose)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::model::{BodyModel, BodyShape};

    fn reference() -> Skeleton {
        let model = BodyModel::builtin().unwrap();
        let body = model.shaped(&BodyShape::default());
        Skeleton::new(&model, &body)
    }

    /// The anchors of a skeleton's own joints.
    fn anchors(s: &Skeleton, pose: &Pose) -> Anchors {
        let t = s.forward(pose);
        let at = |name: &str| {
            let b = s.find(name).unwrap();
            t[b].apply(s.bones[b].head)
        };
        let limb = |a: &str, b: &str, c: &str, side: &str| Limb {
            top: at(&format!("{a}_{side}")),
            middle: at(&format!("{b}_{side}")),
            end: at(&format!("{c}_{side}")),
        };
        Anchors {
            pelvis: at("pelvis"),
            neck: Some(at("neck")),
            head: at("head"),
            clavicles: [Some(at("clavicle_l")), Some(at("clavicle_r"))],
            arms: ["l", "r"].map(|s| limb("upper_arm", "forearm", "hand", s)),
            legs: ["l", "r"].map(|s| limb("thigh", "shin", "foot", s)),
        }
    }

    #[test]
    fn a_posed_body_binds_back_to_its_rest_skeleton() {
        let s = reference();
        // Arms raised to a T, a knee bent: the body as a file has it.
        let mut posed = Pose::rest(s.bones.len());
        for (name, q) in [
            ("upper_arm_l", Quat::from_rotation_z(1.0)),
            ("upper_arm_r", Quat::from_rotation_z(-1.0)),
            ("shin_l", Quat::from_rotation_x(0.4)),
        ] {
            posed.rotations[s.find(name).unwrap()] = q;
        }
        let a = anchors(&s, &posed);
        let targets = Targets::new(&s, &a);
        let (rest, bind_pose) = bind(&s, &targets);
        // The rest skeleton is the reference's.
        for (r, b) in rest.iter().zip(&s.bones) {
            assert!(
                r.head.distance(b.head) < 2e-3,
                "{} {:?} {:?}",
                b.name,
                r.head,
                b.head
            );
            assert!(r.tail.distance(b.tail) < 2e-3, "{}", b.name);
        }
        // The bind pose carries it onto the body.
        let bound = Skeleton::from_parts(rest, Vec::new());
        let t = bound.forward(&bind_pose);
        for (i, [head, tail]) in targets.0.iter().enumerate() {
            let b = &bound.bones[i];
            assert!(t[i].apply(b.head).distance(*head) < 1e-4, "{}", b.name);
            assert!(t[i].apply(b.tail).distance(*tail) < 1e-4, "{}", b.name);
        }
        let hand = s.find("hand_l").unwrap();
        assert!(t[hand].apply(bound.bones[hand].head).y > 1.3);
    }

    #[test]
    fn a_longer_arm_keeps_its_length_at_rest() {
        let s = reference();
        let mut a = anchors(&s, &Pose::rest(s.bones.len()));
        let arm = &mut a.arms[0];
        let longer = (arm.end - arm.middle) * 1.2;
        arm.end = arm.middle + longer;
        let (rest, _) = bind(&s, &Targets::new(&s, &a));
        let fore = s.find("forearm_l").unwrap();
        let hand = s.find("hand_l").unwrap();
        let length = rest[hand].head.distance(rest[fore].head);
        assert!((length - longer.length()).abs() < 1e-4);
        // Along the reference's forearm.
        let r = (s.bones[hand].head - s.bones[fore].head).normalize();
        let d = (rest[hand].head - rest[fore].head).normalize();
        assert!(r.dot(d) > 0.9999);
    }

    #[test]
    fn a_file_skeleton_is_found_by_its_names() {
        let s = reference();
        let rest = Pose::rest(s.bones.len());
        let a = anchors(&s, &rest);
        let names = [
            ("mixamorig:Hips", None, a.pelvis),
            ("mixamorig:Spine", Some(0), a.pelvis + Vec3::Y * 0.1),
            ("mixamorig:Spine1", Some(1), a.pelvis + Vec3::Y * 0.2),
            ("mixamorig:Neck", Some(2), a.neck.unwrap()),
            ("mixamorig:Head", Some(3), a.head),
            ("mixamorig:LeftShoulder", Some(2), a.clavicles[0].unwrap()),
            ("mixamorig:LeftArm", Some(5), a.arms[0].top),
            ("mixamorig:LeftForeArm", Some(6), a.arms[0].middle),
            ("mixamorig:LeftHand", Some(7), a.arms[0].end),
            ("mixamorig:LeftHandIndex1", Some(8), a.arms[0].end),
            ("mixamorig:RightShoulder", Some(2), a.clavicles[1].unwrap()),
            ("mixamorig:RightArm", Some(10), a.arms[1].top),
            ("mixamorig:RightForeArm", Some(11), a.arms[1].middle),
            ("mixamorig:RightHand", Some(12), a.arms[1].end),
            ("mixamorig:LeftUpLeg", Some(0), a.legs[0].top),
            ("mixamorig:LeftLeg", Some(14), a.legs[0].middle),
            ("mixamorig:LeftFoot", Some(15), a.legs[0].end),
            (
                "mixamorig:LeftToeBase",
                Some(16),
                a.legs[0].end + Vec3::Z * 0.1,
            ),
            ("mixamorig:RightUpLeg", Some(0), a.legs[1].top),
            ("mixamorig:RightLeg", Some(18), a.legs[1].middle),
            ("mixamorig:RightFoot", Some(19), a.legs[1].end),
        ];
        let joints: Vec<(String, Option<usize>, Vec3)> = names
            .iter()
            .map(|(n, p, at)| ((*n).to_owned(), *p, *at))
            .collect();
        let file = FileSkeleton::find(&s, &joints).unwrap();
        assert_eq!(file.anchors.arms, a.arms);
        let bone = |j: usize| s.bones[file.bones[j]].name.as_str();
        assert_eq!(bone(0), "pelvis");
        assert_eq!(bone(2), "upper_chest");
        assert_eq!(bone(9), "hand_l");
        assert_eq!(bone(17), "foot_l");
        assert_eq!(bone(19), "shin_r");
        // Without a forearm it cannot be used.
        let mut broken = joints.clone();
        broken[7].0 = "mixamorig:LeftElbowThing".into();
        assert!(FileSkeleton::find(&s, &broken).is_err());
    }
}
