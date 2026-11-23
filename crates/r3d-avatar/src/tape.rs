//! Every body measurement of a shaped body, taken like a tailor would
//! (`docs/specs/29-body-measurements.md`): girths as a tape around a
//! limb or the neck (the convex hull of a section), lengths between
//! points found on the skeleton and the skin. Our own method.

use glam::Vec3;
use r3d_core::Measure;
use r3d_geom::{DVec2, polygon};

use crate::measure::{girth_at, heights};
use crate::model::{BodyModel, Shaped};
use crate::skeleton::Skeleton;

/// From the shoulder joint out to the shoulder's tip (m).
pub const JOINT_TO_TIP_M: f32 = 0.02;

/// Measurements of a body (cm), by [`Measure`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Taped(pub [f32; Measure::ALL.len()]);

impl Taped {
    pub fn get(&self, m: Measure) -> f32 {
        self.0.get(m.index()).copied().unwrap_or_default()
    }
}

/// The girth (m) of the section of the mesh by the plane through
/// `origin` with normal `normal`: the tape around the section loop that
/// surrounds `origin` (other loops, another limb or the torso, are
/// ignored).
pub fn section_girth(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    origin: Vec3,
    normal: Vec3,
) -> Option<f32> {
    let n = normal.try_normalize()?;
    let u = n.any_orthonormal_vector();
    let v = n.cross(u);
    let side = |p: Vec3| (p - origin).dot(n);
    let flat = |p: Vec3| DVec2::new((p - origin).dot(u) as f64, (p - origin).dot(v) as f64);
    let key = |a: u32, b: u32| if a < b { (a, b) } else { (b, a) };
    let mut point = std::collections::HashMap::new();
    let mut links: std::collections::HashMap<(u32, u32), Vec<(u32, u32)>> =
        std::collections::HashMap::new();
    for t in triangles {
        let mut crossed = Vec::with_capacity(2);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (pa, pb) = (*positions.get(a as usize)?, *positions.get(b as usize)?);
            let (da, db) = (side(pa), side(pb));
            if (da > 0.0) != (db > 0.0) {
                let p = pa.lerp(pb, da / (da - db));
                let k = key(a, b);
                point.insert(k, flat(p));
                crossed.push(k);
            }
        }
        if let [k1, k2] = crossed[..] {
            links.entry(k1).or_default().push(k2);
            links.entry(k2).or_default().push(k1);
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut best: Option<(f64, f64)> = None;
    for start in links.keys() {
        if seen.contains(start) {
            continue;
        }
        let mut stack = vec![*start];
        let mut pts = Vec::new();
        while let Some(k) = stack.pop() {
            if !seen.insert(k) {
                continue;
            }
            pts.extend(point.get(&k).copied());
            stack.extend(links.get(&k).into_iter().flatten().copied());
        }
        if pts.len() < 3 {
            continue;
        }
        let hull = polygon::convex_hull(&pts);
        if polygon::contains(&hull, DVec2::ZERO) {
            let centre = pts.iter().fold(DVec2::ZERO, |s, p| s + *p) / pts.len() as f64;
            let d = centre.length();
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, polygon::perimeter(&hull)));
            }
        }
    }
    best.map(|(_, g)| g as f32)
}

/// A section of a limb this much larger than the one nearest the
/// limb's end took in the torso too (a heavy arm touching it).
const WITH_TORSO: f32 = 1.6;

/// The largest (or smallest) girth across a bone from `from` to `to`
/// (fractions of its length from its head), the tape square to the bone
/// (m). Sections that took in the torso are left out.
fn limb_girth(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    (head, tail): (Vec3, Vec3),
    (from, to): (f32, f32),
    largest: bool,
) -> Option<f32> {
    const SAMPLES: usize = 12;
    let girths: Vec<f32> = (0..SAMPLES)
        .filter_map(|i| {
            let t = from + (to - from) * i as f32 / (SAMPLES - 1) as f32;
            section_girth(positions, triangles, head.lerp(tail, t), tail - head)
        })
        .collect();
    let end = *girths.last()?;
    let limb = girths.into_iter().filter(|g| *g < WITH_TORSO * end);
    if largest {
        limb.reduce(f32::max)
    } else {
        limb.reduce(f32::min)
    }
}

/// How far the waist is searched for around its place on the skeleton
/// (m).
const WAIST_SEARCH_M: f32 = 0.03;

/// The waist's height (m): the narrowest girth near the place the
/// skeleton gives it, a third of the way up the lowest spine bone that
/// reaches above the pelvis. The narrowest girth over the whole torso
/// wandered by several centimetres on a man with a belly.
fn waist_level(p: &[Vec3], t: &[[u32; 3]], (head, tail): (Vec3, Vec3)) -> Option<f32> {
    const SAMPLES: usize = 13;
    let at = head.y + (tail.y - head.y) / 3.0;
    (0..SAMPLES)
        .filter_map(|i| {
            let y = at - WAIST_SEARCH_M + 2.0 * WAIST_SEARCH_M * i as f32 / (SAMPLES - 1) as f32;
            Some((girth_at(p, t, y, 0.0)?, y))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, y)| y)
}

/// The point of the section at height `y` nearest the median plane
/// (|x| < 2 cm) that is furthest back (`back`) or forward, near `x`.
fn profile_z(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    y: f32,
    x: f32,
    back: bool,
) -> Option<f32> {
    let mut best: Option<f32> = None;
    for t in triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (pa, pb) = (*positions.get(a as usize)?, *positions.get(b as usize)?);
            if (pa.y > y) != (pb.y > y) {
                let p = pa.lerp(pb, (y - pa.y) / (pb.y - pa.y));
                if (p.x - x).abs() < 0.02 {
                    let better = |z: f32| if back { p.z < z } else { p.z > z };
                    if best.is_none_or(better) {
                        best = Some(p.z);
                    }
                }
            }
        }
    }
    best
}

/// A shaped body being measured, with its skeleton.
struct Taping<'a> {
    p: &'a [Vec3],
    t: &'a [[u32; 3]],
    model: &'a BodyModel,
    skeleton: Skeleton,
}

impl Taping<'_> {
    fn bone(&self, name: &str) -> Option<(Vec3, Vec3)> {
        let b = self.skeleton.bones.get(self.skeleton.find(name)?)?;
        Some((b.head, b.tail))
    }

    fn waist(&self) -> Option<f32> {
        waist_level(self.p, self.t, self.bone("spine3")?)
    }

    fn crotch(&self) -> Option<f32> {
        heights(self.p, self.t).crotch
    }

    fn neck_girth(&self) -> Option<f32> {
        Some(girth_at(self.p, self.t, self.bone("neck")?.0.y, 0.0)? / 100.0)
    }

    /// Where the neck meets the shoulder, on the left.
    fn neck_side(&self) -> Option<Vec3> {
        let (neck, _) = self.bone("neck")?;
        Some(Vec3::new(
            self.neck_girth()? / std::f32::consts::TAU,
            neck.y,
            neck.z,
        ))
    }

    /// The left shoulder's tip, out from its joint at the shoulder's top.
    fn shoulder_tip(&self) -> Option<Vec3> {
        let (joint, _) = self.bone("upper_arm_l")?;
        let top = self
            .p
            .iter()
            .filter(|v| (v.x - joint.x).abs() < 0.015 && (v.z - joint.z).abs() < 0.04)
            .map(|v| v.y)
            .fold(joint.y, f32::max);
        Some(Vec3::new(joint.x + JOINT_TO_TIP_M, top, joint.z))
    }

    /// Down the back's profile from the nape to the waist, every 5 mm.
    fn back_length(&self) -> Option<f32> {
        let (nape_y, waist_y) = (self.bone("neck")?.0.y, self.waist()?);
        let steps = ((nape_y - waist_y) / 0.005).ceil().max(1.0) as usize;
        let mut length = 0.0;
        let mut last: Option<Vec3> = None;
        for i in 0..=steps {
            let y = nape_y + (waist_y - nape_y) * i as f32 / steps as f32;
            let q = Vec3::new(0.0, y, profile_z(self.p, self.t, y, 0.0, true)?);
            if let Some(l) = last {
                length += l.distance(q);
            }
            last = Some(q);
        }
        Some(length)
    }

    /// From the side of the neck over the bust apex down to the waist.
    fn front_length(&self) -> Option<f32> {
        let waist_y = self.waist()?;
        let apex = self
            .p
            .get(self.model.landmarks().bust[0] as usize)
            .copied()?;
        let z = profile_z(self.p, self.t, waist_y, apex.x, false)?;
        let waist = Vec3::new(apex.x, waist_y, z);
        Some(self.neck_side()?.distance(apex) + apex.distance(waist))
    }

    fn horizontal_girth(&self, at: Vec3) -> Option<f32> {
        section_girth(self.p, self.t, at, Vec3::Y)
    }

    /// One measurement (m).
    fn take(&self, m: Measure) -> Option<f32> {
        let limb = |bone: &str, range, largest| {
            limb_girth(self.p, self.t, self.bone(bone)?, range, largest)
        };
        Some(match m {
            Measure::Neck => self.neck_girth()?,
            Measure::ShoulderWidth => 2.0 * self.shoulder_tip()?.x,
            Measure::ShoulderLength => self.neck_side()?.distance(self.shoulder_tip()?),
            Measure::ArmLength => {
                let (_, elbow) = self.bone("upper_arm_l")?;
                let (_, wrist) = self.bone("forearm_l")?;
                self.shoulder_tip()?.distance(elbow) + elbow.distance(wrist)
            }
            Measure::UpperArm => limb("upper_arm_l", (0.3, 0.8), true)?,
            Measure::Wrist => limb("forearm_l", (0.88, 0.95), false)?,
            Measure::BackWaistLength => self.back_length()?,
            Measure::FrontWaistLength => self.front_length()?,
            Measure::WaistHeight => self.waist()?,
            Measure::HipHeight => heights(self.p, self.t).hips,
            Measure::CrotchDepth => self.waist()? - self.crotch()?,
            Measure::InsideLeg => self.crotch()?,
            Measure::Thigh => {
                let (head, _) = self.bone("thigh_l")?;
                self.horizontal_girth(Vec3::new(head.x, self.crotch()? - 0.03, head.z))?
            }
            Measure::Knee => self.horizontal_girth(self.bone("shin_l")?.0)?,
            Measure::Ankle => {
                let (_, ankle) = self.bone("shin_l")?;
                (0..8)
                    .filter_map(|i| {
                        let y = ankle.y + 0.03 + 0.01 * i as f32;
                        self.horizontal_girth(Vec3::new(ankle.x, y, ankle.z))
                    })
                    .reduce(f32::min)?
            }
        })
    }
}

/// One measurement of `shaped`, a shape of `model` (fitted or not;
/// metres, feet on the floor), in cm. `None` if that part of the body
/// was not found (a mesh far from a human shape).
pub fn take(model: &BodyModel, shaped: &Shaped, m: Measure) -> Option<f32> {
    Taping {
        p: &shaped.positions,
        t: model.triangles(),
        model,
        skeleton: Skeleton::new(model, shaped),
    }
    .take(m)
    .map(|v| v * 100.0)
}

/// Every measurement of `shaped` (see [`take`]).
pub fn tape(model: &BodyModel, shaped: &Shaped) -> Option<Taped> {
    let taping = Taping {
        p: &shaped.positions,
        t: model.triangles(),
        model,
        skeleton: Skeleton::new(model, shaped),
    };
    let mut out = [0.0; Measure::ALL.len()];
    for (slot, m) in out.iter_mut().zip(Measure::ALL) {
        *slot = taping.take(m)? * 100.0;
    }
    Some(Taped(out))
}
