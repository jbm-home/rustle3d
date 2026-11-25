//! Arms kept clear of the body while it moves.
//!
//! A captured motion is played by a body of other proportions: arms that
//! hung close to the captured person's hips go through wider hips, and
//! cloth caught between a hand and a hip is squeezed out at many metres a
//! second, or dragged along by the hand. Each pose is therefore checked:
//! where the forearm or the hand comes closer to the rest of the body than
//! [`CLEARANCE_M`] (seen through the skinning, past the arms:
//! [`touch_parts`]), the upper arm is turned out from the body about the
//! forward axis by the smallest angle that clears it (bisection), up to
//! [`MAX_TURN`]. A pose that is clear is left as it is. (Our own scheme.)

use glam::{Quat, Vec3};
use r3d_avatar::pack::Influences;
use r3d_avatar::skeleton::{Pose, Skeleton, skin};
use r3d_geom::Rigid;
use r3d_sim::collide::StaticMesh;
use r3d_sim::motion::{Posed, touch_parts};

/// Room left between a forearm or a hand and the body (m): a layer or two
/// of cloth, each kept a few millimetres off the skin on both sides.
pub(super) const CLEARANCE_M: f32 = 0.015;
/// The farthest an arm is turned out (rad, about 29°).
const MAX_TURN: f32 = 0.5;
/// Halvings of the turn: 0.5 / 2⁷ ≈ 0.2°.
const BISECTIONS: usize = 7;
/// Skin points checked per arm, spread over its forearm and hand.
const ARM_POINTS: usize = 64;

/// One arm: its upper arm bone, the way out from the body (+1 for the
/// left arm, turning about +Z), and points of its forearm and hand.
#[derive(Debug, Clone)]
struct Arm {
    upper: usize,
    out: f32,
    points: Vec<Vec3>,
    weights: Vec<Influences>,
}

/// The arms of a rigged body and the parts that are not arms.
#[derive(Debug, Clone)]
pub(super) struct Arms {
    arms: Vec<Arm>,
    /// Bit `k` for bone `k` when it is not an arm bone.
    body: u32,
}

impl Arms {
    /// The arms of a body with `skeleton`, skin `positions` (as it
    /// stands, m) and their `weights`.
    pub(super) fn new(skeleton: &Skeleton, positions: &[Vec3], weights: &[Influences]) -> Self {
        let mut arm_bones = 0u32;
        let mut arms = Vec::new();
        for (letter, out) in [("l", 1.0), ("r", -1.0)] {
            let bone = |name: &str| skeleton.find(&format!("{name}_{letter}"));
            let (Some(upper), Some(forearm), Some(hand)) =
                (bone("upper_arm"), bone("forearm"), bone("hand"))
            else {
                continue;
            };
            for b in [upper, forearm, hand] {
                if b < 32 {
                    arm_bones |= 1 << b;
                }
            }
            let heaviest = |w: &Influences| {
                w.bones
                    .iter()
                    .zip(w.weights)
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(b, _)| usize::from(*b))
            };
            let chosen: Vec<(Vec3, Influences)> = positions
                .iter()
                .zip(weights)
                .filter(|(_, w)| heaviest(w).is_some_and(|b| b == forearm || b == hand))
                .map(|(p, w)| (*p, *w))
                .collect();
            let stride = chosen.len().div_ceil(ARM_POINTS).max(1);
            let (points, weights) = chosen.into_iter().step_by(stride).unzip();
            arms.push(Arm {
                upper,
                out,
                points,
                weights,
            });
        }
        let all = match u32::try_from(skeleton.bones.len()) {
            Ok(n) if n < 32 => (1u32 << n) - 1,
            _ => u32::MAX,
        };
        Self {
            arms,
            body: all & !arm_bones,
        }
    }

    /// `pose` with each arm turned out as far as its forearm and hand
    /// need to clear the rest of the body: the distance field `body` of
    /// the body as it stands, seen through its skinning, `transforms`
    /// giving each bone's motion from there to a pose.
    pub(super) fn clear(
        &self,
        body: &StaticMesh,
        pose: &mut Pose,
        transforms: impl Fn(&Pose) -> Vec<Rigid>,
    ) {
        for arm in &self.arms {
            let Some(rotation) = pose.rotations.get(arm.upper).copied() else {
                continue;
            };
            let turned = |angle: f32| {
                let mut p = pose.clone();
                if let Some(r) = p.rotations.get_mut(arm.upper) {
                    *r = (Quat::from_rotation_z(arm.out * angle) * rotation).normalize();
                }
                p
            };
            let is_clear = |p: &Pose| {
                let t = transforms(p);
                let points = skin(&arm.points, &arm.weights, &t);
                let posed = Posed::new(t.clone(), t);
                points.iter().all(|q| {
                    touch_parts(body, &posed, *q, self.body)
                        .is_none_or(|(c, _)| c.distance >= CLEARANCE_M)
                })
            };
            if is_clear(pose) {
                continue;
            }
            let (mut lo, mut hi) = (0.0, MAX_TURN);
            for _ in 0..BISECTIONS {
                let mid = 0.5 * (lo + hi);
                if is_clear(&turned(mid)) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            *pose = turned(hi);
        }
    }
}
