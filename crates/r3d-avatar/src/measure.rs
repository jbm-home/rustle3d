//! Body measurements taken like a tailor would: stature from the floor, and
//! circumferences as a tape around the torso at a given height (the convex
//! hull of the horizontal section).

use std::collections::HashMap;

use glam::Vec3;
use r3d_geom::{DVec2, polygon};
use serde::{Deserialize, Serialize};

/// Measurements in centimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Measurements {
    pub height_cm: f32,
    pub bust_cm: f32,
    pub waist_cm: f32,
    pub hips_cm: f32,
    /// Height of the waist (the narrowest girth) above the floor.
    pub waist_height_cm: f32,
}

/// Bands of the torso, as fractions of the stature, where the waist
/// (smallest) and hips (largest) girths are searched.
const WAIST_BAND: (f32, f32) = (0.57, 0.66);
const HIPS_BAND: (f32, f32) = (0.47, 0.56);
const SAMPLES: usize = 20;

/// Torso girth (cm) at height `y`: the tape length around the section
/// loop that surrounds the body's vertical axis (arms and hands are
/// separate loops and are ignored).
pub fn girth_at(positions: &[Vec3], triangles: &[[u32; 3]], y: f32, axis_x: f32) -> Option<f32> {
    // Each crossed mesh edge gives one section point; triangles link them.
    let key = |a: u32, b: u32| if a < b { (a, b) } else { (b, a) };
    let mut point: HashMap<(u32, u32), DVec2> = HashMap::new();
    let mut links: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
    for t in triangles {
        let mut crossed = Vec::with_capacity(2);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (pa, pb) = (positions.get(a as usize)?, positions.get(b as usize)?);
            if (pa.y > y) != (pb.y > y) {
                let s = (y - pa.y) / (pb.y - pa.y);
                let p = pa.lerp(*pb, s);
                let k = key(a, b);
                point.insert(k, DVec2::new(p.x as f64, p.z as f64));
                crossed.push(k);
            }
        }
        if let [k1, k2] = crossed[..] {
            links.entry(k1).or_default().push(k2);
            links.entry(k2).or_default().push(k1);
        }
    }
    // Walk connected components into loops of points.
    let mut seen = std::collections::HashSet::new();
    let mut best: Option<(f64, f32)> = None;
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
            if let Some(p) = point.get(&k) {
                pts.push(*p);
            }
            stack.extend(links.get(&k).into_iter().flatten().copied());
        }
        if pts.len() < 3 {
            continue;
        }
        let cx = pts.iter().map(|p| p.x).sum::<f64>() / pts.len() as f64;
        let hull = polygon::convex_hull(&pts);
        // The torso loop is the one around the vertical axis.
        let inside = polygon::contains(&hull, DVec2::new(axis_x as f64, centroid_z(&hull)));
        if inside {
            let distance = (cx - axis_x as f64).abs();
            let girth = polygon::perimeter(&hull) as f32;
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, girth));
            }
        }
    }
    best.map(|(_, g)| g * 100.0)
}

fn centroid_z(hull: &[DVec2]) -> f64 {
    hull.iter().map(|p| p.y).sum::<f64>() / hull.len().max(1) as f64
}

fn extreme_girth(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    height: f32,
    band: (f32, f32),
    largest: bool,
) -> (f32, f32) {
    // (girth, height) of the extreme section.
    let girths = (0..SAMPLES).filter_map(|i| {
        let y = (band.0 + (band.1 - band.0) * i as f32 / (SAMPLES - 1) as f32) * height;
        girth_at(positions, triangles, y, 0.0).map(|g| (g, y))
    });
    if largest {
        girths.fold((0.0, 0.0), |a, b| if b.0 > a.0 { b } else { a })
    } else {
        girths.fold((f32::MAX, 0.0), |a, b| if b.0 < a.0 { b } else { a })
    }
}

/// Anatomical reference points of the mesh, found once on the base body;
/// morph targets move them along with the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landmarks {
    /// The bust apex (most forward point of the chest) on each side.
    pub bust: [u32; 2],
}

impl Landmarks {
    /// Locate the landmarks on a neutral body (metres, feet at y = 0).
    pub fn find(positions: &[Vec3]) -> Self {
        let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let apex = |side: f32| {
            positions
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.x * side > 0.02
                        && p.x * side < 0.15
                        && p.y > 0.64 * height
                        && p.y < 0.76 * height
                })
                .max_by(|(_, a), (_, b)| a.z.total_cmp(&b.z))
                .map_or(0, |(i, _)| i as u32)
        };
        Self {
            bust: [apex(1.0), apex(-1.0)],
        }
    }
}

/// Measure a body (positions in metres, feet at y = 0). The bust is taken
/// over the bust apex, like a tailor's tape.
pub fn measure(positions: &[Vec3], triangles: &[[u32; 3]], landmarks: &Landmarks) -> Measurements {
    let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
    let bust_y = landmarks
        .bust
        .iter()
        .filter_map(|i| positions.get(*i as usize))
        .map(|p| p.y)
        .sum::<f32>()
        / 2.0;
    let waist = extreme_girth(positions, triangles, height, WAIST_BAND, false);
    Measurements {
        height_cm: height * 100.0,
        bust_cm: girth_at(positions, triangles, bust_y, 0.0).unwrap_or_default(),
        waist_cm: waist.0,
        hips_cm: extreme_girth(positions, triangles, height, HIPS_BAND, true).0,
        waist_height_cm: waist.1 * 100.0,
    }
}
