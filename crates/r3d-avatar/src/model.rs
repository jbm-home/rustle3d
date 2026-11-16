//! The parametric body: base mesh plus weighted morph targets.

use glam::Vec3;

use crate::measure::Landmarks;
use crate::pack::{BodyPack, PackError, Target};

/// The built-in body pack (MakeHuman CC0 assets, see `assets/avatars/`).
static BUILTIN_PACK: &[u8] = include_bytes!("../../../assets/avatars/makehuman-body.r3dbody");

/// Circumferences the model can adjust directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Girth {
    Bust,
    Waist,
    Hips,
}

impl Girth {
    pub const ALL: [Girth; 3] = [Girth::Bust, Girth::Waist, Girth::Hips];

    fn target_stem(self) -> &'static str {
        match self {
            Girth::Bust => "bust-circ",
            Girth::Waist => "waist-circ",
            Girth::Hips => "hips-circ",
        }
    }
}

/// Shape parameters. All are dimensionless; `0` is the neutral base body.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodyShape {
    /// 0 = feminine, 1 = masculine.
    pub masculinity: f32,
    /// -1 = slim, 1 = heavy (up to [`MAX_EXTRAPOLATION`]).
    pub weight: f32,
    /// -1 = short proportions, 1 = tall proportions (before exact scaling).
    pub stature: f32,
    /// Local girth adjustments, -1..1 (up to [`MAX_EXTRAPOLATION`]), in
    /// [`Girth::ALL`] order.
    pub girths: [f32; 3],
}

/// Targets are linear, so they can be pushed moderately past their authored
/// range to reach larger sizes.
pub const MAX_EXTRAPOLATION: f32 = 1.8;

/// Pairs of targets selected by a signed parameter.
struct Signed<'a> {
    negative: &'a Target,
    positive: &'a Target,
}

/// The body model, with its targets resolved by name once.
#[derive(Debug, Clone)]
pub struct BodyModel {
    pack: BodyPack,
    landmarks: Landmarks,
    /// Vertices smoothed like a dress form's (around the bust apexes):
    /// each with its smoothing weight (0..1) and its neighbours.
    smoothed: Vec<(u32, f32, Vec<u32>)>,
}

/// Radius of the smoothed area around each bust apex (m). The nipples are
/// narrower than the cloth's vertex spacing, so they would poke through
/// any garment; dress forms are smooth there too.
const SMOOTH_RADIUS_M: f32 = 0.05;
const SMOOTH_PASSES: usize = 30;

impl BodyModel {
    /// The built-in MakeHuman-based body.
    pub fn builtin() -> Result<Self, PackError> {
        Self::from_pack(BodyPack::from_bytes(BUILTIN_PACK)?)
    }

    pub fn from_pack(pack: BodyPack) -> Result<Self, PackError> {
        let mut required = vec!["gender-female".to_owned(), "gender-male".to_owned()];
        for sex in ["female", "male"] {
            for end in ["min", "max"] {
                required.push(format!("weight-{sex}-{end}"));
                required.push(format!("height-{sex}-{end}"));
            }
        }
        for g in Girth::ALL {
            for dir in ["decr", "incr"] {
                required.push(format!("measure-{}-{dir}", g.target_stem()));
            }
        }
        if let Some(missing) = required.iter().find(|n| pack.target(n).is_none()) {
            return Err(PackError::Invalid(format!("missing target {missing}")));
        }
        let mut model = Self {
            pack,
            landmarks: Landmarks { bust: [0, 0] },
            smoothed: Vec::new(),
        };
        let base = model.positions(&BodyShape::default());
        model.landmarks = Landmarks::find(&base);
        // The nipples stand out on the feminine body: find them there (the
        // topology is shared, so the same vertices are smoothed on every
        // body).
        let feminine = model.positions(&BodyShape {
            masculinity: 0.0,
            ..BodyShape::default()
        });
        let apexes: Vec<Vec3> = Landmarks::find(&feminine)
            .bust
            .iter()
            .filter_map(|i| feminine.get(*i as usize).copied())
            .collect();
        let mut neighbours = vec![Vec::new(); base.len()];
        for t in &model.pack.triangles {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                for (from, to) in [(a, b), (b, a)] {
                    if let Some(list) = neighbours.get_mut(from as usize)
                        && !list.contains(&to)
                    {
                        list.push(to);
                    }
                }
            }
        }
        model.smoothed = feminine
            .iter()
            .zip(neighbours)
            .enumerate()
            .filter_map(|(i, (p, near))| {
                let d = apexes
                    .iter()
                    .map(|a| a.distance(*p))
                    .fold(f32::MAX, f32::min);
                let w = 1.0 - d / SMOOTH_RADIUS_M;
                (w > 0.0).then_some((i as u32, w, near))
            })
            .collect();
        Ok(model)
    }

    pub fn landmarks(&self) -> &Landmarks {
        &self.landmarks
    }

    pub fn provenance(&self) -> &str {
        &self.pack.provenance
    }

    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.pack.triangles
    }

    fn get(&self, name: &str) -> Option<&Target> {
        self.pack.target(name)
    }

    fn signed(&self, negative: &str, positive: &str) -> Option<Signed<'_>> {
        Some(Signed {
            negative: self.get(negative)?,
            positive: self.get(positive)?,
        })
    }

    /// The targets a shape applies, with their weights.
    fn weighted_targets(&self, shape: &BodyShape) -> Vec<(&Target, f32)> {
        let m = shape.masculinity.clamp(0.0, 1.0);
        let mut list = Vec::new();
        for (sex, w) in [("female", 1.0 - m), ("male", m)] {
            list.extend(self.get(&format!("gender-{sex}")).map(|t| (t, w)));
            let weight = self.signed(&format!("weight-{sex}-min"), &format!("weight-{sex}-max"));
            list.extend(pick(weight, shape.weight, w));
            let stature = self.signed(&format!("height-{sex}-min"), &format!("height-{sex}-max"));
            list.extend(pick(stature, shape.stature, w));
        }
        for (g, value) in Girth::ALL.iter().zip(shape.girths) {
            let stem = g.target_stem();
            let pair = self.signed(
                &format!("measure-{stem}-decr"),
                &format!("measure-{stem}-incr"),
            );
            list.extend(pick(pair, value, 1.0));
        }
        list.retain(|(_, w)| *w != 0.0);
        list
    }

    /// Vertex positions for a shape, with the feet on the ground (y = 0).
    pub fn positions(&self, shape: &BodyShape) -> Vec<Vec3> {
        let mut out = self.pack.positions.clone();
        for (t, w) in self.weighted_targets(shape) {
            for (v, o) in t.vertices.iter().zip(&t.offsets) {
                if let Some(p) = out.get_mut(*v as usize) {
                    *p += *o * w;
                }
            }
        }
        for _ in 0..SMOOTH_PASSES {
            let moves: Vec<(usize, Vec3)> = self
                .smoothed
                .iter()
                .filter_map(|(i, w, near)| {
                    let p = *out.get(*i as usize)?;
                    let sum: Vec3 = near.iter().filter_map(|n| out.get(*n as usize)).sum();
                    let mean = sum / near.len().max(1) as f32;
                    Some((*i as usize, p + (mean - p) * 0.5 * w))
                })
                .collect();
            for (i, p) in moves {
                if let Some(slot) = out.get_mut(i) {
                    *slot = p;
                }
            }
        }
        let floor = out.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        if floor.is_finite() {
            for p in &mut out {
                p.y -= floor;
            }
        }
        out
    }
}

/// The target a signed parameter selects, weighted by its magnitude.
fn pick<'a>(pair: Option<Signed<'a>>, value: f32, scale: f32) -> Option<(&'a Target, f32)> {
    let pair = pair?;
    let v = value.clamp(-1.0, MAX_EXTRAPOLATION);
    let t = if v < 0.0 {
        pair.negative
    } else {
        pair.positive
    };
    Some((t, v.abs() * scale))
}

/// Area-weighted vertex normals.
pub fn vertex_normals(positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for t in triangles {
        let [a, b, c] = t.map(|i| i as usize);
        let (Some(pa), Some(pb), Some(pc)) = (positions.get(a), positions.get(b), positions.get(c))
        else {
            continue;
        };
        let n = (*pb - *pa).cross(*pc - *pa);
        for i in [a, b, c] {
            normals[i] += n;
        }
    }
    normals.iter().map(|n| n.normalize_or(Vec3::Y)).collect()
}
