//! Motions captured on real people, read from BVH files and played by the
//! avatar.
//!
//! **Retargeting.** Each of our bones that has a matching captured joint
//! takes that joint's rotation from its rest pose, in world axes, after a
//! fixed turn that lays our bone's rest direction onto the captured
//! joint's (so a captured T pose and our A pose line up, and the bone
//! points where the captured one points). A bone without a direction of
//! its own (the hand, the head) keeps its parent's fixed turn. The spine
//! bones between the hips and the chest share the captured spine's
//! rotations along the chain (spherical interpolation). Poses are made
//! for each body when played, so a captured motion follows every size.
//! Some files leave their rest pose lying (every bone along one axis, a
//! leg up and the other down) and turn each joint upright in every frame;
//! for those, the sampled frame that stands best stands in for the rest
//! pose, each joint's rotation at that frame undone.
//!
//! **Looping.** The motion repeats the longest stretch whose end matches
//! its start, poses and speeds compared over a few frames, within a
//! tolerance of the best match; its last quarter second blends into the
//! frames before its start, so the loop has no jump. The hips' heading
//! drifts over a capture (the person walks a slight curve): its linear
//! trend over the stretch is turned away, so the avatar faces forward
//! and the loop closes. The avatar moves on the spot: the captured
//! travel is dropped, and the skeleton keeps the lowest sole on the
//! floor ([`Skeleton::transforms`]).

use std::f32::consts::{PI, TAU};

use glam::{Mat3, Quat, Vec3};
use r3d_core::{Localizable, Message};

use crate::bvh::Bvh;
use crate::motion::ease;
use crate::skeleton::{Pose, Skeleton};

/// Captures are sampled at most this many times a second.
const MAX_FPS: f32 = 60.0;
/// Only the first this many seconds of a capture are used.
const MAX_CAPTURE_S: f32 = 30.0;
/// The shortest loop (s).
const MIN_LOOP_S: f32 = 1.0;
/// The end of the loop blends into its start over this time (s).
const BLEND_S: f32 = 0.25;
/// Frames compared on each side of a loop's start and end, so that the
/// speeds match as well as the poses.
const WINDOW: usize = 3;
/// A loop may be longer than the best matching one if its mismatch is
/// worse by no more than this share of the gap between the best match
/// and an average pair of frames.
const LOOP_TOLERANCE: f32 = 0.1;

/// Why a file cannot be played as a motion.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MotionError {
    #[error("not a readable BVH file: {detail}")]
    Format { detail: String },
    #[error("the captured skeleton has no joint for the {bone}")]
    Skeleton { bone: &'static str },
    #[error("the motion is too short: it needs at least {seconds} s")]
    TooShort { seconds: f32 },
}

impl Localizable for MotionError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Format { detail } => Message::new("motion.format", text).arg("detail", detail),
            Self::Skeleton { bone } => Message::new("motion.skeleton", text).arg("bone", bone),
            Self::TooShort { seconds } => {
                Message::new("motion.too_short", text).arg("seconds", format!("{seconds:.1}"))
            }
        }
    }
}

/// One of our bones and the captured joint that drives it.
#[derive(Debug, Clone, PartialEq)]
struct Driven {
    bone: String,
    /// The captured bone's rest direction, in our axes; `None`: the bone
    /// keeps its parent's fixed turn.
    direction: Option<Vec3>,
}

/// A captured motion, ready for any body.
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedMotion {
    pub name: String,
    fps: f32,
    driven: Vec<Driven>,
    /// Per frame of the loop, each driven bone's rotation from the
    /// captured rest pose, in our axes.
    frames: Vec<Vec<Quat>>,
}

/// A joint name as compared: what follows a namespace (`rig:Hips`), in
/// lower case, letters and digits only.
fn key(name: &str) -> String {
    name.rsplit(':')
        .next()
        .unwrap_or(name)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Our limb bones (without the side), the names the matching joint goes
/// by in common capture skeletons (without the side), and whether a file
/// must have it.
const LIMBS: [(&str, &[&str], bool); 8] = [
    ("clavicle", &["clavicle", "collar", "shoulder"], false),
    ("upper_arm", &["upperarm", "arm", "uparm", "shldr"], true),
    ("forearm", &["forearm", "lowerarm"], true),
    ("hand", &["hand"], true),
    ("thigh", &["upleg", "thigh", "upperleg"], true),
    ("shin", &["leg", "shin", "calf", "lowerleg"], true),
    ("foot", &["foot"], true),
    ("toe", &["toebase", "toe", "toes"], false),
];

/// Where a driven bone's rotation comes from.
#[derive(Debug, Clone, Copy)]
enum Source {
    Joint(usize),
    /// A place along the captured spine, 0 at the hips, 1 at the chest.
    Spine(f32),
}

/// Our spine, hips to chest.
const SPINE: [&str; 6] = [
    "pelvis",
    "spine1",
    "spine2",
    "spine3",
    "chest",
    "upper_chest",
];

struct Mapping {
    driven: Vec<Driven>,
    sources: Vec<Source>,
    spine: Vec<usize>,
    /// The joints of the left and right thighs, for the heading.
    thighs: [usize; 2],
}

fn map(bvh: &Bvh, basis: Quat, rest: &[Vec3]) -> Result<Mapping, MotionError> {
    let keys: Vec<String> = bvh.joints.iter().map(|j| key(&j.name)).collect();
    let find = |names: &[String]| names.iter().find_map(|n| keys.iter().position(|k| k == n));
    let owned = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
    let hips =
        find(&owned(&["hips", "hip", "pelvis"])).ok_or(MotionError::Skeleton { bone: "hips" })?;
    let head = find(&owned(&["head"])).ok_or(MotionError::Skeleton { bone: "head" })?;
    let neck = find(&owned(&["neck1", "neck"]));
    let limb = |stems: &[&str], side: usize| {
        let (word, letter) = [("left", "l"), ("right", "r")][side.min(1)];
        let names: Vec<String> = stems
            .iter()
            .flat_map(|s| {
                [
                    format!("{word}{s}"),
                    format!("{letter}{s}"),
                    format!("{s}{letter}"),
                    format!("{s}{word}"),
                ]
            })
            .collect();
        find(&names)
    };
    let mut limbs = [[None; 8]; 2];
    for side in 0..2 {
        for (k, (bone, stems, required)) in LIMBS.iter().enumerate() {
            let found = limb(stems, side);
            if *required && found.is_none() {
                return Err(MotionError::Skeleton { bone });
            }
            if let Some(slot) = limbs.get_mut(side).and_then(|l| l.get_mut(k)) {
                *slot = found;
            }
        }
    }
    // The captured spine: the hips up to the joint the neck (or the head)
    // hangs from.
    let top = bvh
        .joints
        .get(neck.unwrap_or(head))
        .and_then(|j| j.parent)
        .ok_or(MotionError::Skeleton { bone: "spine" })?;
    let mut spine = vec![top];
    while let Some(&j) = spine.last()
        && j != hips
    {
        let parent = bvh.joints.get(j).and_then(|j| j.parent);
        match parent {
            Some(p) if spine.len() < bvh.joints.len() => spine.push(p),
            _ => return Err(MotionError::Skeleton { bone: "spine" }),
        }
    }
    spine.reverse();

    let direction = |from: usize, to: Option<usize>| {
        let (a, b) = (rest.get(from)?, rest.get(to?)?);
        (basis * (*b - *a)).try_normalize()
    };
    let mut driven = Vec::new();
    let mut sources = Vec::new();
    let mut add = |bone: String, source: Source, direction: Option<Vec3>| {
        driven.push(Driven { bone, direction });
        sources.push(source);
    };
    for (k, bone) in SPINE.iter().enumerate() {
        add((*bone).to_owned(), Source::Spine(k as f32 / 5.0), None);
    }
    if let Some(n) = neck {
        add("neck".into(), Source::Joint(n), None);
    }
    add("head".into(), Source::Joint(head), None);
    for (side, letter) in ["l", "r"].iter().enumerate() {
        let joint = |k: usize| limbs.get(side).and_then(|l| l.get(k)).copied().flatten();
        // Our bone, its joint and the joint at its tail.
        for (k, tail) in [
            (0, Some(1)),
            (1, Some(2)),
            (2, Some(3)),
            (3, None),
            (4, Some(5)),
            (5, Some(6)),
            (6, Some(7)),
        ] {
            let (Some(j), Some((bone, _, _))) = (joint(k), LIMBS.get(k)) else {
                continue;
            };
            add(
                format!("{bone}_{letter}"),
                Source::Joint(j),
                direction(j, tail.and_then(joint)),
            );
        }
    }
    let thigh = |side: usize| limbs.get(side).and_then(|l| l.get(4)).copied().flatten();
    let (Some(left), Some(right)) = (thigh(0), thigh(1)) else {
        return Err(MotionError::Skeleton { bone: "thigh" });
    };
    Ok(Mapping {
        driven,
        sources,
        spine,
        thighs: [left, right],
    })
}

/// Frames sampled to find how the body stands in the motion.
const AXIS_SAMPLES: usize = 32;
/// The rest pose stands if its hips-to-head direction is within 45° of
/// the motion's up.
const STANDING_COS: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// The file's axis (±X, ±Y or ±Z) nearest to `v`.
fn nearest_axis(v: Vec3) -> Vec3 {
    let a = v.abs();
    if a.x >= a.y && a.x >= a.z {
        Vec3::X * v.x.signum()
    } else if a.y >= a.z {
        Vec3::Y * v.y.signum()
    } else {
        Vec3::Z * v.z.signum()
    }
}

/// The rotation whose columns are the body's left, up and forward, from
/// `up` and a `left` made square to it.
fn frame(up: Vec3, left: Vec3) -> Mat3 {
    let left = (left - up * left.dot(up)).normalize_or(up.any_orthonormal_vector());
    Mat3::from_cols(left, up, left.cross(up))
}

/// The body's up and left in `positions`: up from the hips to the head,
/// left from the right thigh to the left one (not normalized).
fn body_axes(positions: &[Vec3], hips: usize, head: usize, thighs: [usize; 2]) -> (Vec3, Vec3) {
    let at = |i: usize| positions.get(i).copied().unwrap_or(Vec3::ZERO);
    (at(head) - at(hips), at(thighs[0]) - at(thighs[1]))
}

/// The joints that tell how a body stands.
struct Stance {
    hips: usize,
    head: usize,
    thighs: [usize; 2],
    /// Each thigh's child joint.
    knees: [usize; 2],
}

impl Stance {
    /// How well `positions` stand along `up`: the cosine of the worst of
    /// the trunk (hips to head, along up) and the thighs (down).
    fn standing(&self, positions: &[Vec3], up: Vec3) -> f32 {
        let at = |i: usize| positions.get(i).copied().unwrap_or(Vec3::ZERO);
        let along =
            |from: usize, to: usize, dir: Vec3| (at(to) - at(from)).normalize_or_zero().dot(dir);
        along(self.hips, self.head, up)
            .min(along(self.thighs[0], self.knees[0], -up))
            .min(along(self.thighs[1], self.knees[1], -up))
    }
}

/// The frame to retarget from, when the file's rest pose does not stand:
/// some files leave every bone along one axis (a leg up, the other down)
/// and turn them in every frame; rotations from such a rest pose would
/// lay our avatar down and twist its limbs. Then the sampled frame that
/// stands best serves as the rest pose. Up in the motion is the file axis
/// nearest to the hips-to-head direction over the capture.
fn reference_frame(bvh: &Bvh, rest: &[Vec3], stance: &Stance) -> Option<usize> {
    let step = (bvh.frames() / AXIS_SAMPLES).max(1);
    let sampled: Vec<(usize, Vec<Vec3>)> = (0..bvh.frames())
        .step_by(step)
        .map(|f| (f, bvh.posed_positions(f)))
        .collect();
    let up = sampled
        .iter()
        .map(|(_, p)| {
            body_axes(p, stance.hips, stance.head, stance.thighs)
                .0
                .normalize_or_zero()
        })
        .sum::<Vec3>();
    let up = nearest_axis(up.normalize_or(Vec3::Y));
    if stance.standing(rest, up) > STANDING_COS {
        return None;
    }
    sampled
        .iter()
        .map(|(f, p)| (*f, stance.standing(p, up)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(f, _)| f)
}

/// The turn from the file's axes to ours (X to the body's left, Y up, Z
/// forward), found from its (standing) rest pose.
fn basis(rest: &[Vec3], stance: &Stance) -> Quat {
    let (up, left) = body_axes(rest, stance.hips, stance.head, stance.thighs);
    // Rows: the file's left, up and forward, so that they map to X, Y, Z.
    Quat::from_mat3(&frame(up.normalize_or(Vec3::Y), left).transpose()).normalize()
}

/// Angle between two rotations (radians).
fn angle(a: Quat, b: Quat) -> f32 {
    2.0 * a.dot(b).abs().min(1.0).acos()
}

impl CapturedMotion {
    /// The motion in a BVH file's `text`, named `name`.
    pub fn from_bvh(name: &str, text: &str) -> Result<Self, MotionError> {
        let bvh = Bvh::parse(text)?;
        let rest = bvh.rest_positions();
        // A first mapping finds the joints that give the file's axes.
        let first = map(&bvh, Quat::IDENTITY, &rest)?;
        let hips = first.spine.first().copied().unwrap_or(0);
        let head_key = key("head");
        let head = bvh
            .joints
            .iter()
            .position(|j| key(&j.name) == head_key)
            .unwrap_or(0);
        let child = |j: usize| bvh.joints.iter().position(|c| c.parent == Some(j));
        let stance = Stance {
            hips,
            head,
            thighs: first.thighs,
            knees: first.thighs.map(|t| child(t).unwrap_or(t)),
        };
        // Rotations are taken from the rest pose, or from the reference
        // frame's pose (each joint's rotation at that frame undone).
        let reference = reference_frame(&bvh, &rest, &stance);
        let (rest, undo): (Vec<Vec3>, Vec<Quat>) = match reference {
            Some(f) => (
                bvh.posed_positions(f),
                bvh.world_rotations(f).iter().map(|q| q.inverse()).collect(),
            ),
            None => (rest, Vec::new()),
        };
        let basis = basis(&rest, &stance);
        let mapping = map(&bvh, basis, &rest)?;

        let step = ((1.0 / (bvh.frame_time * MAX_FPS)).round() as usize).max(1);
        let fps = 1.0 / (bvh.frame_time * step as f32);
        let count = bvh
            .frames()
            .min((MAX_CAPTURE_S / bvh.frame_time) as usize + 1);
        let inverse = basis.inverse();
        let lateral = rest
            .get(mapping.thighs[0])
            .zip(rest.get(mapping.thighs[1]))
            .map_or(Vec3::X, |(l, r)| (basis * (*l - *r)).normalize_or(Vec3::X));
        let mut frames: Vec<Vec<Quat>> = Vec::new();
        let mut headings: Vec<f32> = Vec::new();
        for f in (0..count).step_by(step) {
            let world: Vec<Quat> = bvh
                .world_rotations(f)
                .into_iter()
                .enumerate()
                .map(|(j, q)| {
                    let q = q * undo.get(j).copied().unwrap_or(Quat::IDENTITY);
                    (basis * q * inverse).normalize()
                })
                .collect();
            let at = |j: usize| world.get(j).copied().unwrap_or(Quat::IDENTITY);
            let spine = |s: f32| {
                let x = s * mapping.spine.len().saturating_sub(1) as f32;
                let i = x.floor() as usize;
                let a = mapping.spine.get(i).map_or(Quat::IDENTITY, |j| at(*j));
                let b = mapping.spine.get(i + 1).map_or(a, |j| at(*j));
                a.slerp(b, x - i as f32)
            };
            frames.push(
                mapping
                    .sources
                    .iter()
                    .map(|s| match s {
                        Source::Joint(j) => at(*j),
                        Source::Spine(s) => spine(*s),
                    })
                    .collect(),
            );
            let l = at(hips) * lateral;
            headings.push((-l.z).atan2(l.x));
        }
        // Unwrapped, so that a turn past ±180° is not a jump.
        for i in 1..headings.len() {
            if let (Some(a), Some(b)) = (headings.get(i - 1).copied(), headings.get_mut(i)) {
                *b = a + (*b - a + PI).rem_euclid(TAU) - PI;
            }
        }
        let (start, end) = best_loop(&frames, &headings, fps)?;
        let blend = ((BLEND_S * fps).round() as usize).max(1);

        // Turn the heading's trend away over the loop and its blend.
        let span: Vec<usize> = (start..=end).collect();
        let n = span.len() as f32;
        let mean_f = span.iter().map(|f| *f as f32).sum::<f32>() / n;
        let mean_h = span.iter().filter_map(|f| headings.get(*f)).sum::<f32>() / n;
        let (num, den) = span.iter().fold((0.0, 0.0), |(num, den), f| {
            let x = *f as f32 - mean_f;
            let h = headings.get(*f).copied().unwrap_or(mean_h);
            (num + x * (h - mean_h), den + x * x)
        });
        let slope = if den > 0.0 { num / den } else { 0.0 };
        let straight = |f: usize| -> Vec<Quat> {
            let turn = Quat::from_rotation_y(-(mean_h + slope * (f as f32 - mean_f)));
            frames
                .get(f)
                .map(|row| row.iter().map(|q| (turn * *q).normalize()).collect())
                .unwrap_or_default()
        };
        let mut looped: Vec<Vec<Quat>> = (start..end).map(straight).collect();
        let len = looped.len();
        for k in 0..blend.min(len) {
            let before = straight(start + k - blend.min(start));
            let w = ease((k + 1) as f32 / (blend + 1) as f32);
            if let Some(row) = looped.get_mut(len - blend.min(len) + k) {
                for (q, b) in row.iter_mut().zip(before) {
                    *q = q.slerp(b, w).normalize();
                }
            }
        }
        Ok(Self {
            name: name.to_owned(),
            fps,
            driven: mapping.driven,
            frames: looped,
        })
    }

    /// Length of one loop (s).
    pub fn period(&self) -> f32 {
        self.frames.len() as f32 / self.fps
    }

    /// The pose of `skeleton` `t` seconds into the loop.
    pub fn pose(&self, skeleton: &Skeleton, t: f32) -> Pose {
        let n = skeleton.bones.len();
        let mut pose = Pose::rest(n);
        let len = self.frames.len();
        if len == 0 {
            return pose;
        }
        let x = (t.max(0.0) / self.period()).fract() * len as f32;
        let i = (x.floor() as usize).min(len - 1);
        let w = x - i as f32;
        let (Some(a), Some(b)) = (self.frames.get(i), self.frames.get((i + 1) % len)) else {
            return pose;
        };
        // Each bone's rotation in world axes and its fixed turn.
        let mut world: Vec<Quat> = Vec::with_capacity(n);
        let mut fixed: Vec<Quat> = Vec::with_capacity(n);
        for (i, bone) in skeleton.bones.iter().enumerate() {
            let parent = bone.parent.and_then(|p| world.get(p).zip(fixed.get(p)));
            let (pw, pf) = parent.map_or((Quat::IDENTITY, Quat::IDENTITY), |(w, f)| (*w, *f));
            let driven = self.driven.iter().position(|d| d.bone == bone.name);
            let (rotation, turn) =
                match driven.and_then(|k| Some((self.driven.get(k)?, a.get(k)?, b.get(k)?))) {
                    Some((d, qa, qb)) => {
                        let turn = d
                            .direction
                            .map_or(pf, |to| Quat::from_rotation_arc(bone.direction(), to));
                        ((qa.slerp(*qb, w) * turn).normalize(), turn)
                    }
                    None => (pw, pf),
                };
            if let Some(r) = pose.rotations.get_mut(i) {
                *r = (pw.inverse() * rotation).normalize();
            }
            world.push(rotation);
            fixed.push(turn);
        }
        pose
    }
}

/// The loop's first frame and the frame it comes back to.
fn best_loop(
    frames: &[Vec<Quat>],
    headings: &[f32],
    fps: f32,
) -> Result<(usize, usize), MotionError> {
    let n = frames.len();
    let blend = ((BLEND_S * fps).round() as usize).max(1);
    let shortest = ((MIN_LOOP_S * fps).round() as usize).max(2);
    let lo = blend.max(WINDOW);
    let needed = (lo + shortest + WINDOW + 1) as f32 / fps;
    if n < lo + shortest + WINDOW + 1 {
        return Err(MotionError::TooShort {
            seconds: (needed * 10.0).ceil() / 10.0,
        });
    }
    let hi = n - 1 - WINDOW;
    // Poses without their heading, to compare.
    let level: Vec<Vec<Quat>> = frames
        .iter()
        .zip(headings)
        .map(|(row, h)| {
            let turn = Quat::from_rotation_y(-h);
            row.iter().map(|q| turn * *q).collect()
        })
        .collect();
    let bones = frames.first().map_or(1, Vec::len).max(1);
    let distance = |a: usize, b: usize| -> f32 {
        match (level.get(a), level.get(b)) {
            (Some(x), Some(y)) => x.iter().zip(y).map(|(p, q)| angle(*p, *q)).sum(),
            _ => f32::MAX,
        }
    };
    let cost = |i: usize, j: usize| -> f32 {
        (0..=2 * WINDOW)
            .map(|k| distance(i + k - WINDOW, j + k - WINDOW))
            .sum::<f32>()
            / (bones * (2 * WINDOW + 1)) as f32
    };
    // For each start, the ends where the match is best locally: a loop a
    // few frames longer or shorter than a good one only looks good.
    let mut costs = Vec::new();
    for i in lo..=hi {
        let row: Vec<f32> = (i + shortest..=hi).map(|j| cost(i, j)).collect();
        for (k, c) in row.iter().enumerate() {
            let before = k.checked_sub(1).and_then(|k| row.get(k));
            let after = row.get(k + 1);
            if before.is_none_or(|b| c <= b) && after.is_none_or(|a| c <= a) {
                costs.push((i, i + shortest + k, *c));
            }
        }
    }
    let best = costs.iter().map(|c| c.2).fold(f32::MAX, f32::min);
    let mean = costs.iter().map(|c| c.2).sum::<f32>() / costs.len().max(1) as f32;
    let worst = best + LOOP_TOLERANCE * (mean - best).max(0.0);
    costs
        .into_iter()
        .filter(|c| c.2 <= worst)
        .max_by(|a, b| (a.1 - a.0).cmp(&(b.1 - b.0)).then(b.2.total_cmp(&a.2)))
        .map(|(i, j, _)| (i, j))
        .ok_or(MotionError::TooShort {
            seconds: (needed * 10.0).ceil() / 10.0,
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::fmt::Write;

    use super::*;
    use crate::model::{BodyModel, BodyShape};
    use crate::skeleton::skin;

    /// A capture skeleton like the common ones, standing in a T pose;
    /// every joint with three rotation channels, `Zrotation Xrotation
    /// Yrotation`.
    const JOINTS: [(&str, Option<usize>, [f32; 3]); 16] = [
        ("Hips", None, [0.0, 0.0, 0.0]),
        ("Spine", Some(0), [0.0, 10.0, 0.0]),
        ("Neck", Some(1), [0.0, 30.0, 0.0]),
        ("Head", Some(2), [0.0, 10.0, 0.0]),
        ("LeftArm", Some(1), [18.0, 25.0, 0.0]),
        ("LeftForeArm", Some(4), [28.0, 0.0, 0.0]),
        ("LeftHand", Some(5), [25.0, 0.0, 0.0]),
        ("RightArm", Some(1), [-18.0, 25.0, 0.0]),
        ("RightForeArm", Some(7), [-28.0, 0.0, 0.0]),
        ("RightHand", Some(8), [-25.0, 0.0, 0.0]),
        ("LeftUpLeg", Some(0), [9.0, -5.0, 0.0]),
        ("LeftLeg", Some(10), [0.0, -42.0, 0.0]),
        ("LeftFoot", Some(11), [0.0, -40.0, 0.0]),
        ("RightUpLeg", Some(0), [-9.0, -5.0, 0.0]),
        ("RightLeg", Some(13), [0.0, -42.0, 0.0]),
        ("RightFoot", Some(14), [0.0, -40.0, 0.0]),
    ];

    /// A BVH file of `JOINTS`, `seconds` long at 120 frames a second,
    /// each frame's rotations given by `angles(t)` (joint index, channel
    /// Z/X/Y, degrees), with the file's up axis along `up`.
    fn capture(
        seconds: f32,
        z_up: bool,
        angles: impl Fn(f32) -> Vec<(usize, usize, f32)>,
    ) -> String {
        fn emit(s: &mut String, i: usize, z_up: bool) {
            let (name, parent, v) = JOINTS[i];
            let o = if z_up { [v[0], -v[2], v[1]] } else { v };
            // Z up: the file's Y is our −Z and its Z our Y; the channels
            // turn about our Z, X, Y in that order either way.
            let turns = if z_up {
                "Yrotation Xrotation Zrotation"
            } else {
                "Zrotation Xrotation Yrotation"
            };
            let (kind, channels) = match parent {
                None => (
                    "ROOT",
                    format!("CHANNELS 6 Xposition Yposition Zposition {turns}"),
                ),
                Some(_) => ("JOINT", format!("CHANNELS 3 {turns}")),
            };
            writeln!(
                s,
                "{kind} {name} {{ OFFSET {} {} {} {channels}",
                o[0], o[1], o[2]
            )
            .unwrap();
            for c in (0..JOINTS.len()).filter(|c| JOINTS[*c].1 == Some(i)) {
                emit(s, c, z_up);
            }
            s.push_str("}\n");
        }
        let mut s = String::from("HIERARCHY\n");
        emit(&mut s, 0, z_up);
        let frames = (seconds * 120.0) as usize;
        writeln!(s, "MOTION\nFrames: {frames}\nFrame Time: {}", 1.0 / 120.0).unwrap();
        for f in 0..frames {
            let mut row = vec![0.0f32; 3 + 3 * JOINTS.len()];
            row[1] = 90.0;
            for (j, c, v) in angles(f as f32 / 120.0) {
                // The rotation channels of joint j start at 3 + 3j; about
                // our Z is about the Z-up file's −Y.
                let sign = if z_up && c == 0 { -1.0 } else { 1.0 };
                row[3 + 3 * j + c] = v * sign;
            }
            let line: Vec<String> = row.iter().map(|v| v.to_string()).collect();
            writeln!(s, "{}", line.join(" ")).unwrap();
        }
        s
    }

    /// A walk of one step cycle a second: the thighs swing (X, forward
    /// is negative), the arms hang down (Z) and swing against the legs.
    fn walk(t: f32) -> Vec<(usize, usize, f32)> {
        let phase = (TAU * t).sin();
        vec![
            (10, 1, -25.0 * phase),
            (13, 1, 25.0 * phase),
            (4, 0, -75.0),
            (7, 0, 75.0),
            (4, 1, 15.0 * phase),
            (7, 1, -15.0 * phase),
        ]
    }

    fn body() -> (BodyModel, crate::model::Shaped, Skeleton) {
        let model = BodyModel::builtin().unwrap();
        let body = model.shaped(&BodyShape::default());
        let skeleton = Skeleton::new(&model, &body);
        (model, body, skeleton)
    }

    fn joint(skeleton: &Skeleton, pose: &Pose, bone: &str, tail: bool) -> Vec3 {
        let i = skeleton.find(bone).unwrap();
        let b = &skeleton.bones[i];
        skeleton.transforms(pose)[i].apply(if tail { b.tail } else { b.head })
    }

    #[test]
    fn a_captured_walk_loops_and_moves_our_bones_like_the_captured_ones() {
        let (model, body, skeleton) = body();
        let motion = CapturedMotion::from_bvh("walk", &capture(3.5, false, walk)).unwrap();
        // One or more whole step cycles.
        let cycles = motion.period();
        assert!(
            (cycles - cycles.round()).abs() < 0.05 && cycles >= 0.95,
            "{cycles}"
        );
        // The loop has no jump.
        let a = motion.pose(&skeleton, 0.0);
        let b = motion.pose(&skeleton, motion.period() - 0.001);
        let gap = a
            .rotations
            .iter()
            .zip(&b.rotations)
            .map(|(p, q)| angle(*p, *q))
            .fold(0.0, f32::max);
        assert!(gap < 0.05, "{gap}");
        for k in 0..20 {
            let t = k as f32 * motion.period() / 20.0;
            let pose = motion.pose(&skeleton, t);
            // The arms hang along the body, whatever our rest pose.
            let hand = joint(&skeleton, &pose, "hand_l", false);
            let shoulder = joint(&skeleton, &pose, "upper_arm_l", false);
            assert!(hand.y < shoulder.y - 0.4, "{t}: {hand} {shoulder}");
            assert!((hand.x - shoulder.x).abs() < 0.15, "{t}: {hand} {shoulder}");
            let posed = skin(
                &body.positions,
                model.weights(),
                &skeleton.transforms(&pose),
            );
            let floor = posed.iter().map(|p| p.y).fold(f32::MAX, f32::min);
            assert!(posed.iter().all(|p| p.is_finite()) && floor.abs() < 0.01);
        }
        // Where the captured left thigh swings forward, ours does, and
        // the right one back.
        let forward = (0..40)
            .map(|k| k as f32 * motion.period() / 40.0)
            .map(|t| {
                let pose = motion.pose(&skeleton, t);
                let knee = |side: &str| joint(&skeleton, &pose, &format!("thigh_{side}"), true).z;
                knee("l") - knee("r")
            })
            .fold((f32::MAX, f32::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)));
        assert!(forward.0 < -0.15 && forward.1 > 0.15, "{forward:?}");
    }

    #[test]
    fn the_file_axes_and_a_heading_do_not_change_the_motion() {
        let (_, _, skeleton) = body();
        let y_up = CapturedMotion::from_bvh("a", &capture(3.0, false, walk)).unwrap();
        let z_up = CapturedMotion::from_bvh("b", &capture(3.0, true, walk)).unwrap();
        let turned = CapturedMotion::from_bvh(
            "c",
            &capture(3.0, false, |t| {
                let mut a = walk(t);
                a.push((0, 2, 70.0));
                a
            }),
        )
        .unwrap();
        for other in [&z_up, &turned] {
            assert!((other.period() - y_up.period()).abs() < 0.02);
            for k in 0..10 {
                let t = k as f32 * 0.1;
                let a = y_up.pose(&skeleton, t);
                let b = other.pose(&skeleton, t);
                for bone in ["hand_l", "foot_r", "head"] {
                    let (p, q) = (
                        joint(&skeleton, &a, bone, true),
                        joint(&skeleton, &b, bone, true),
                    );
                    assert!(
                        p.distance(q) < 0.01,
                        "{} {bone} at {t}: {p} {q}",
                        other.name
                    );
                }
            }
        }
    }

    /// The same capture as `capture(seconds, false, angles)`, from a
    /// skeleton whose joints each have a frame of their own: the rest
    /// pose lies along X, a leg up and the other down, and every frame
    /// turns it upright (as some exporters write files).
    fn lying_capture(seconds: f32, angles: impl Fn(f32) -> Vec<(usize, usize, f32)>) -> String {
        use glam::EulerRot;
        use std::f32::consts::FRAC_PI_2;
        let orient = |i: usize| -> Quat {
            let name = JOINTS[i].0;
            let turn = if name.starts_with("Left") {
                FRAC_PI_2
            } else {
                -FRAC_PI_2
            };
            Quat::from_rotation_z(turn) * Quat::from_rotation_x(0.3 * i as f32)
        };
        let parent_orient = |i: usize| JOINTS[i].1.map_or(Quat::IDENTITY, orient);
        let turns = "Zrotation Xrotation Yrotation";
        fn emit(s: &mut String, i: usize, offset: &dyn Fn(usize) -> Vec3, turns: &str) {
            let (name, parent, _) = JOINTS[i];
            let o = offset(i);
            let (kind, channels) = match parent {
                None => (
                    "ROOT",
                    format!("CHANNELS 6 Xposition Yposition Zposition {turns}"),
                ),
                Some(_) => ("JOINT", format!("CHANNELS 3 {turns}")),
            };
            writeln!(
                s,
                "{kind} {name} {{ OFFSET {} {} {} {channels}",
                o.x, o.y, o.z
            )
            .unwrap();
            for c in (0..JOINTS.len()).filter(|c| JOINTS[*c].1 == Some(i)) {
                emit(s, c, offset, turns);
            }
            s.push_str(
                "}
",
            );
        }
        // A joint's offset is in its parent's frame.
        let offset = |i: usize| parent_orient(i).inverse() * Vec3::from(JOINTS[i].2);
        let mut s = String::from(
            "HIERARCHY
",
        );
        emit(&mut s, 0, &offset, turns);
        let frames = (seconds * 120.0) as usize;
        writeln!(
            s,
            "MOTION
Frames: {frames}
Frame Time: {}",
            1.0 / 120.0
        )
        .unwrap();
        for f in 0..frames {
            let mut euler = [[0.0f32; 3]; JOINTS.len()];
            for (j, c, v) in angles(f as f32 / 120.0) {
                euler[j][c] = v.to_radians();
            }
            let mut row = vec![0.0, 90.0, 0.0];
            for (j, [z, x, y]) in euler.iter().enumerate() {
                // The upright local rotation, seen from the joints' frames.
                let local = Quat::from_euler(EulerRot::ZXY, *z, *x, *y);
                let lying = parent_orient(j).inverse() * local * orient(j);
                let (z, x, y) = lying.to_euler(EulerRot::ZXY);
                row.extend([z.to_degrees(), x.to_degrees(), y.to_degrees()]);
            }
            let line: Vec<String> = row.iter().map(|v| v.to_string()).collect();
            writeln!(s, "{}", line.join(" ")).unwrap();
        }
        s
    }

    #[test]
    fn a_lying_rest_pose_still_walks_upright() {
        let (_, _, skeleton) = body();
        let upright = CapturedMotion::from_bvh("a", &capture(3.0, false, walk)).unwrap();
        let lying = CapturedMotion::from_bvh("b", &lying_capture(3.0, walk)).unwrap();
        assert!((lying.period() - upright.period()).abs() < 0.02);
        // The loops may start at another point of the step cycle.
        let ankle =
            |m: &CapturedMotion, t: f32| joint(&skeleton, &m.pose(&skeleton, t), "foot_l", false);
        let shift = (0..120)
            .map(|k| k as f32 / 120.0 * lying.period())
            .min_by(|x, y| {
                let d = |s: f32| ankle(&upright, 0.0).distance(ankle(&lying, s));
                d(*x).total_cmp(&d(*y))
            })
            .unwrap();
        for k in 0..10 {
            let t = k as f32 * 0.1;
            let a = upright.pose(&skeleton, t);
            let b = lying.pose(&skeleton, t + shift);
            // Wrists, ankles and the neck land where they do from the
            // upright file (twists around the bones may differ).
            for bone in ["hand_l", "hand_r", "foot_l", "foot_r", "head"] {
                let (p, q) = (
                    joint(&skeleton, &a, bone, false),
                    joint(&skeleton, &b, bone, false),
                );
                assert!(p.distance(q) < 0.02, "{bone} at {t}: {p} {q}");
            }
        }
    }

    #[test]
    fn files_that_cannot_be_played_are_refused() {
        let short = capture(0.5, false, walk);
        assert!(matches!(
            CapturedMotion::from_bvh("s", &short),
            Err(MotionError::TooShort { .. })
        ));
        let armless = capture(2.0, false, walk).replace("LeftForeArm", "Tentacle");
        assert_eq!(
            CapturedMotion::from_bvh("a", &armless),
            Err(MotionError::Skeleton { bone: "forearm" })
        );
        assert!(matches!(
            CapturedMotion::from_bvh("x", "HIERARCHY"),
            Err(MotionError::Format { .. })
        ));
    }
}
