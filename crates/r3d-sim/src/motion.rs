//! A body that moves: its parts (the avatar's bones) follow rigid motions
//! over time, and the cloth collides with it (docs/physics.md §5).
//!
//! The body's distance field is built once, in the rest pose, with the
//! parts each node's closest surface belongs to ([`StaticMesh::with_parts`]).
//! A point is taken back to the rest pose by the inverse motion of each
//! part in turn; where the field there is near that same part's surface,
//! the part touches the point, at the distance read there (a rigid motion
//! keeps distances), along the normal turned by the part's motion. Of the
//! parts that touch, the deepest wins. The body is thus a union of rigid
//! pieces: around a bent joint they overlap where linear blend skinning
//! folds the skin, so the cloth stays out of the visible body; no part
//! hides behind another's rest position, since a part only counts where
//! the field's closest surface is its own. (Our own scheme, on top of the
//! distance field of `collide`.)

use glam::Vec3;
use r3d_geom::Rigid;

use crate::collide::{Contact, StaticMesh};

/// How the body's parts move: for each part, its rigid motion from the
/// rest pose, sampled `fps` times a second from `start` (simulated time,
/// s); first `intro` once, then `cycle` over and over. Before `start`
/// the body rests.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyMotion {
    pub start: f32,
    pub fps: f32,
    pub intro: Vec<Vec<Rigid>>,
    pub cycle: Vec<Vec<Rigid>>,
}

impl BodyMotion {
    /// Number of parts.
    pub fn parts(&self) -> usize {
        self.intro
            .first()
            .or(self.cycle.first())
            .map_or(0, Vec::len)
    }

    fn frame(&self, i: usize) -> Option<&Vec<Rigid>> {
        match self.intro.get(i) {
            Some(f) => Some(f),
            None if self.cycle.is_empty() => self.intro.last(),
            None => self
                .cycle
                .get((i - self.intro.len()) % self.cycle.len().max(1)),
        }
    }

    /// Each part's motion at time `t`, between the two nearest samples.
    pub fn at(&self, t: f32) -> Vec<Rigid> {
        let k = (t - self.start) * self.fps;
        let rest = vec![Rigid::IDENTITY; self.parts()];
        if k.is_nan() || k <= 0.0 || !k.is_finite() {
            return rest;
        }
        let i = k.floor() as usize;
        let f = k - i as f32;
        match (self.frame(i), self.frame(i + 1)) {
            (Some(a), Some(b)) => a.iter().zip(b).map(|(a, b)| a.lerp(b, f)).collect(),
            (Some(a), None) => a.clone(),
            _ => rest,
        }
    }
}

/// The body's parts during one substep: where they are at its end, the
/// way back to the rest pose, and where they were at its start.
#[derive(Debug, Clone)]
pub struct Posed {
    now: Vec<Rigid>,
    back: Vec<Rigid>,
    before: Vec<Rigid>,
}

impl Posed {
    pub fn new(before: Vec<Rigid>, now: Vec<Rigid>) -> Self {
        Self {
            back: now.iter().map(Rigid::inverse).collect(),
            now,
            before,
        }
    }

    pub fn now(&self) -> &[Rigid] {
        &self.now
    }

    pub fn before(&self) -> &[Rigid] {
        &self.before
    }
}

/// The body seen from `p`: the contact (world frame), and how far the
/// body's surface there moved during the substep (zero for a body at
/// rest). `posed` is `None` for a body that does not move.
pub fn touch(body: &StaticMesh, posed: Option<&Posed>, p: Vec3) -> Option<(Contact, Vec3)> {
    let Some(posed) = posed else {
        return body.sample(p).map(|c| (c, Vec3::ZERO));
    };
    let mut best: Option<(Contact, Vec3)> = None;
    for (part, (lo, hi)) in body.part_boxes().iter().enumerate() {
        let (Some(back), Some(now), Some(before)) = (
            posed.back.get(part),
            posed.now.get(part),
            posed.before.get(part),
        ) else {
            break;
        };
        let q = back.apply(p);
        if q.cmplt(*lo).any() || q.cmpgt(*hi).any() {
            continue;
        }
        let Some(c) = body.sample(q) else {
            continue;
        };
        if best.is_some_and(|(b, _)| b.distance <= c.distance)
            || body.parts_at(q) & (1 << part) == 0
        {
            continue;
        }
        let contact = Contact {
            distance: c.distance,
            normal: now.rotation * c.normal,
        };
        best = Some((contact, p - before.apply(q)));
    }
    best
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use glam::Quat;

    /// Two boxes side by side along x, each a part: [-0.2, 0] and [0, 0.2]
    /// (a closed mesh of two cubes touching at x = 0 would share a face;
    /// they are 2 cm apart).
    fn two_parts() -> StaticMesh {
        let mut positions = Vec::new();
        let mut triangles = Vec::new();
        let mut parts = Vec::new();
        for (k, x0) in [(0u32, -0.21f32), (1, 0.01)] {
            let base = positions.len() as u32;
            for i in 0..8 {
                positions.push(Vec3::new(
                    if i & 1 == 0 { x0 } else { x0 + 0.2 },
                    if i & 2 == 0 { -0.1 } else { 0.1 },
                    if i & 4 == 0 { -0.1 } else { 0.1 },
                ));
                parts.push(1 << k);
            }
            let quads = [
                [0, 2, 3, 1],
                [4, 5, 7, 6],
                [0, 1, 5, 4],
                [2, 6, 7, 3],
                [0, 4, 6, 2],
                [1, 3, 7, 5],
            ];
            for [a, b, c, d] in quads {
                triangles.push([base + a, base + b, base + c]);
                triangles.push([base + a, base + c, base + d]);
            }
        }
        StaticMesh::with_parts(positions, triangles, 0.005, &parts)
    }

    #[test]
    fn samples_between_frames_and_loops() {
        let shift = |x: f32| Rigid {
            rotation: Quat::IDENTITY,
            translation: Vec3::X * x,
        };
        let m = BodyMotion {
            start: 1.0,
            fps: 10.0,
            intro: vec![vec![shift(0.0)], vec![shift(1.0)]],
            cycle: vec![vec![shift(2.0)], vec![shift(3.0)]],
        };
        let x = |t: f32| m.at(t)[0].translation.x;
        assert_eq!(x(0.5), 0.0);
        assert!((x(1.05) - 0.5).abs() < 1e-5);
        assert!((x(1.25) - 2.5).abs() < 1e-5);
        // After the cycle's last sample comes its first again.
        assert!((x(1.35) - 2.5).abs() < 1e-5);
        assert!((x(1.45) - 2.5).abs() < 1e-5);
    }

    #[test]
    fn each_part_carries_its_own_surface() {
        let body = two_parts();
        assert_eq!(body.part_boxes().len(), 2);
        // The right box (part 1) moves up 5 cm; the left one stays.
        let up = Rigid {
            rotation: Quat::IDENTITY,
            translation: Vec3::Y * 0.05,
        };
        let posed = Posed::new(vec![Rigid::IDENTITY; 2], vec![Rigid::IDENTITY, up]);
        // Just above the moved right box's top: outside, 2 mm.
        let (c, moved) = touch(&body, Some(&posed), Vec3::new(0.1, 0.152, 0.0)).unwrap();
        assert!((c.distance - 0.002).abs() < 1e-3, "{}", c.distance);
        assert!(c.normal.dot(Vec3::Y) > 0.99);
        assert!((moved - Vec3::Y * 0.05).length() < 1e-5);
        // Where its top used to be, nothing touches any more.
        assert!(touch(&body, Some(&posed), Vec3::new(0.1, 0.102, 0.0)).is_none());
        // The left box's top is still where it was, and did not move.
        let (c, moved) = touch(&body, Some(&posed), Vec3::new(-0.1, 0.098, 0.0)).unwrap();
        assert!((c.distance + 0.002).abs() < 1e-3);
        assert_eq!(moved, Vec3::ZERO);
        // At rest the parts answer as the plain field does.
        let rest = Posed::new(vec![Rigid::IDENTITY; 2], vec![Rigid::IDENTITY; 2]);
        let p = Vec3::new(0.15, 0.03, 0.105);
        let a = touch(&body, Some(&rest), p).unwrap().0;
        let b = body.sample(p).unwrap();
        assert!((a.distance - b.distance).abs() < 1e-6);
    }
}
