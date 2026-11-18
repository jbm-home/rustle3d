//! A body that moves: its bones follow rigid motions over time, its skin
//! follows its bones (linear blend skinning), and the cloth collides with
//! the skin (docs/physics.md §5).
//!
//! The body's distance field is built once, in the rest pose, with each
//! node's nearest vertex and so its skin weights
//! ([`StaticMesh::with_skin`]). To see the body from a point `p`, the
//! point is taken back to the rest pose: the rest point `q` whose skinned
//! position `Σ wₖ Gₖ(q)` is `p`. With the weights held constant around
//! `q` this is one linear solve, `q = M⁻¹ (p − T)` where `M = Σ wₖ Rₖ` and
//! `T = Σ wₖ tₖ`; it is repeated with the weights found there. Each bone
//! whose rest box holds `G⁻¹(p)` gives a starting point (the bone's own
//! inverse motion), so every limb near `p` is tried; a start is kept only
//! if the skin it finds really passes through `p`. The distance read at
//! `q` is mapped to the posed skin by `M` (to first order), and of the
//! points found, the deepest wins. The cloth thus collides with the very
//! skin the views draw. (Our own scheme, on top of the distance field of
//! `collide`.)

use glam::{Mat3, Vec3};
use r3d_geom::Rigid;

use crate::collide::{Contact, Skin, StaticMesh};

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
}

/// Solves of the inverse skinning per starting point.
pub const ITERATIONS: usize = 3;
/// A rest point counts when its skinned position is this close to the
/// point looked from (m).
pub const ON_SKIN: f32 = 0.002;

/// The skinning of a vertex with weights `skin` under `bones`, as
/// `p ↦ M p + T`; `None` when it squashes space flat (`M` singular).
fn blend(skin: &Skin, bones: &[Rigid]) -> Option<(Mat3, Vec3)> {
    let mut m = Mat3::ZERO;
    let mut t = Vec3::ZERO;
    for (b, w) in skin.bones.iter().zip(skin.weights) {
        if w > 0.0 {
            let g = bones.get(usize::from(*b))?;
            m += Mat3::from_quat(g.rotation) * w;
            t += g.translation * w;
        }
    }
    (m.determinant().abs() > 1e-3).then_some((m, t))
}

/// The body seen from `p`: the contact (world frame), and how far the
/// body's surface there moved during the substep (zero for a body at
/// rest). `posed` is `None` for a body that does not move.
pub fn touch(body: &StaticMesh, posed: Option<&Posed>, p: Vec3) -> Option<(Contact, Vec3)> {
    let Some(posed) = posed else {
        return body.sample(p).map(|c| (c, Vec3::ZERO));
    };
    let mut best: Option<(Contact, Vec3)> = None;
    for (bone, (lo, hi)) in body.part_boxes().iter().enumerate() {
        let Some(back) = posed.back.get(bone) else {
            break;
        };
        let mut q = back.apply(p);
        if q.cmplt(*lo).any() || q.cmpgt(*hi).any() {
            continue;
        }
        if body
            .skin_at(q)
            .is_none_or(|s| s.leading() & (1 << bone) == 0)
        {
            continue;
        }
        let mut solved = true;
        for _ in 0..ITERATIONS {
            let Some((m, t)) = body.skin_at(q).and_then(|s| blend(s, &posed.now)) else {
                solved = false;
                break;
            };
            q = m.inverse() * (p - t);
        }
        let Some(skin) = body.skin_at(q).filter(|_| solved) else {
            continue;
        };
        let (Some((m, t)), Some((mb, tb))) = (blend(skin, &posed.now), blend(skin, &posed.before))
        else {
            continue;
        };
        if (m * q + t).distance(p) > ON_SKIN {
            continue;
        }
        let Some(c) = body.sample(q) else {
            continue;
        };
        // A rest offset d·n becomes M d n: along the posed normal
        // M⁻ᵀ n / |M⁻ᵀ n|, that is d / |M⁻ᵀ n|.
        let n = m.inverse().transpose() * c.normal;
        let stretch = n.length();
        if stretch.is_nan() || stretch <= 1e-6 {
            continue;
        }
        let distance = c.distance / stretch;
        if best.is_some_and(|(b, _)| b.distance <= distance) {
            continue;
        }
        let contact = Contact {
            distance,
            normal: n / stretch,
        };
        best = Some((contact, p - (mb * q + tb)));
    }
    best
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use glam::Quat;

    /// Two boxes side by side along x, 2 cm apart, each led by its own
    /// bone: [-0.21, -0.01] and [0.01, 0.21].
    fn two_bones() -> StaticMesh {
        let mut positions = Vec::new();
        let mut triangles = Vec::new();
        let mut skin = Vec::new();
        for (bone, x0) in [(0u8, -0.21f32), (1, 0.01)] {
            let base = positions.len() as u32;
            for i in 0..8 {
                positions.push(Vec3::new(
                    if i & 1 == 0 { x0 } else { x0 + 0.2 },
                    if i & 2 == 0 { -0.1 } else { 0.1 },
                    if i & 4 == 0 { -0.1 } else { 0.1 },
                ));
                skin.push(Skin {
                    bones: [bone, 0, 0, 0],
                    weights: [1.0, 0.0, 0.0, 0.0],
                });
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
        StaticMesh::with_skin(positions, triangles, 0.005, skin)
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
    fn each_bone_carries_its_own_skin() {
        let body = two_bones();
        assert_eq!(body.part_boxes().len(), 2);
        // The right box (bone 1) moves up 5 cm and turns a little; the
        // left one stays.
        let up = Rigid {
            rotation: Quat::from_rotation_y(0.2),
            translation: Vec3::Y * 0.05,
        };
        let posed = Posed::new(vec![Rigid::IDENTITY; 2], vec![Rigid::IDENTITY, up]);
        // Just above the moved right box's top: outside, 2 mm.
        let p = up.apply(Vec3::new(0.11, 0.102, 0.0));
        let (c, moved) = touch(&body, Some(&posed), p).unwrap();
        assert!((c.distance - 0.002).abs() < 1e-3, "{}", c.distance);
        assert!(c.normal.dot(Vec3::Y) > 0.99);
        assert!((moved - (p - Vec3::new(0.11, 0.102, 0.0))).length() < 1e-4);
        // Where its top used to be, nothing touches any more.
        assert!(touch(&body, Some(&posed), Vec3::new(0.1, 0.102, 0.0)).is_none());
        // The left box's top is still where it was, and did not move.
        let (c, moved) = touch(&body, Some(&posed), Vec3::new(-0.1, 0.098, 0.0)).unwrap();
        assert!((c.distance + 0.002).abs() < 1e-3);
        assert_eq!(moved, Vec3::ZERO);
        // At rest the skin answers as the plain field does.
        let rest = Posed::new(vec![Rigid::IDENTITY; 2], vec![Rigid::IDENTITY; 2]);
        let p = Vec3::new(0.15, 0.03, 0.105);
        let a = touch(&body, Some(&rest), p).unwrap().0;
        let b = body.sample(p).unwrap();
        assert!((a.distance - b.distance).abs() < 1e-6);
    }

    #[test]
    fn a_blended_skin_is_followed_where_it_is() {
        // One box whose skin is half bone 0, half bone 1: moving bone 1 up
        // by 4 cm moves the skin up by 2 cm, not 4 (nor 0).
        let positions: Vec<Vec3> = (0..8)
            .map(|i| {
                Vec3::new(
                    if i & 1 == 0 { -0.1 } else { 0.1 },
                    if i & 2 == 0 { -0.1 } else { 0.1 },
                    if i & 4 == 0 { -0.1 } else { 0.1 },
                )
            })
            .collect();
        let quads = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let triangles = quads
            .iter()
            .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
            .collect();
        let half = Skin {
            bones: [0, 1, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        };
        let body = StaticMesh::with_skin(positions, triangles, 0.005, vec![half; 8]);
        let up = Rigid {
            rotation: Quat::IDENTITY,
            translation: Vec3::Y * 0.04,
        };
        let posed = Posed::new(vec![Rigid::IDENTITY; 2], vec![Rigid::IDENTITY, up]);
        let (c, moved) = touch(&body, Some(&posed), Vec3::new(0.0, 0.123, 0.0)).unwrap();
        assert!((c.distance - 0.003).abs() < 1e-3, "{}", c.distance);
        assert!((moved - Vec3::Y * 0.02).length() < 1e-4, "{moved}");
    }
}
