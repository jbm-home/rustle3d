//! Fit maps and the virtual tape measure (spec 24): how much the draped
//! cloth is stretched compared with its pattern, how far it stands off the
//! body, and circumferences of the garment and the body at a height.
//!
//! Everything here reads positions only, so it works the same on a drape
//! from the CPU or the GPU solver, live in the app or cached headless.

use std::collections::{BTreeMap, HashMap};

use glam::{Vec2, Vec3};
use r3d_core::{PanelId, Project};
use r3d_geom::{DVec2, polygon};
use r3d_sim::collide::closest_feature;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::ActionSpec;
use crate::drape::{Drape, Form, Latest};
use crate::error::{CommandError, Invalid};
use crate::spec::Described;
use crate::workspace::{Workspace, WorkspaceError};

/// What a fit map shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FitMap {
    /// How much the cloth is stretched beyond its pattern (%).
    Strain,
    /// How far the cloth stands off the body (mm).
    Gap,
}

/// Strain shown at the tight end of the scale (%).
const STRAIN_TIGHT_PCT: f32 = 20.0;
/// Gap shown at the loose end of the scale (mm).
const GAP_LOOSE_MM: f32 = 50.0;
/// Gaps are searched this far from the cloth (m); farther reads as this.
const GAP_SEARCH_M: f32 = 0.1;
/// Cloth this close to the body touches it (m): its thickness and the
/// solver's skin margin (docs/physics.md §5) leave a few millimetres.
const TOUCHING_M: f32 = 0.005;
/// Ends of a cut through the cloth this close together close the loop (m).
const CLOSE_LOOP_M: f32 = 0.02;
/// Tight spots reported, the strongest first.
const MAX_SPOTS: usize = 10;

impl FitMap {
    pub const ALL: [FitMap; 2] = [FitMap::Strain, FitMap::Gap];

    pub const fn name(self) -> &'static str {
        match self {
            FitMap::Strain => "strain",
            FitMap::Gap => "gap",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    /// The values at the loose (dark) and the tight (bright) ends of the
    /// colour scale, in [`Self::unit`]s.
    pub const fn range(self) -> (f32, f32) {
        match self {
            FitMap::Strain => (0.0, STRAIN_TIGHT_PCT),
            FitMap::Gap => (GAP_LOOSE_MM, 0.0),
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            FitMap::Strain => "%",
            FitMap::Gap => "mm",
        }
    }

    /// Where `value` lies on the colour scale: 0 loose, 1 tight.
    pub fn level(self, value: f32) -> f32 {
        let (loose, tight) = self.range();
        ((value - loose) / (tight - loose)).clamp(0.0, 1.0)
    }

    /// The map's value at every particle of `drape` (in [`Self::unit`]s):
    /// the cloth at `positions`, the body's surface and triangles `body`.
    /// Without a body the gap reads its search limit everywhere.
    pub fn values(self, drape: &Drape, positions: &[Vec3], body: Option<Body<'_>>) -> Vec<f32> {
        match self {
            FitMap::Strain => vertex_strains(drape, positions)
                .into_iter()
                .map(|s| s * 100.0)
                .collect(),
            FitMap::Gap => gaps(positions, body)
                .into_iter()
                .map(|g| g * 1000.0)
                .collect(),
        }
    }
}

/// A body surface: vertex positions (m) and triangles.
pub type Body<'a> = (&'a [Vec3], &'a [[u32; 3]]);

/// The largest principal stretch of a triangle, minus one (0.1 = 10 %):
/// the deformation gradient `F` maps its rest shape on the pattern
/// (`rest`, m) onto its place in space (`at`); the stretches are the
/// square roots of the eigenvalues of `FᵀF` (Bonet & Wood 1997, §3; as
/// in Müller et al., Strain Based Dynamics, 2014). `None` for a
/// degenerate rest triangle.
pub fn stretch(rest: [Vec2; 3], at: [Vec3; 3]) -> Option<f32> {
    let (r1, r2) = (rest[1] - rest[0], rest[2] - rest[0]);
    let det = r1.perp_dot(r2);
    if det.abs() < 1e-12 {
        return None;
    }
    let (e1, e2) = (at[1] - at[0], at[2] - at[0]);
    // The columns of F = [e1 e2] [r1 r2]⁻¹.
    let fu = (e1 * r2.y - e2 * r1.y) / det;
    let fv = (e2 * r1.x - e1 * r2.x) / det;
    let (a, b, d) = (fu.dot(fu), fu.dot(fv), fv.dot(fv));
    let largest = (a + d) / 2.0 + (((a - d) / 2.0).powi(2) + b * b).sqrt();
    let s = largest.sqrt() - 1.0;
    s.is_finite().then_some(s)
}

/// Each triangle's strain (ratio) and rest area (m²), in the order of the
/// scene's triangles; a degenerate one reads 0.
fn triangle_strains(drape: &Drape, positions: &[Vec3]) -> Vec<(f32, f32)> {
    let particles = &drape.scene.particles;
    drape
        .scene
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.vertices.map(|v| v as usize);
            let rest = [a, b, c].map(|i| particles.get(i).map_or(Vec2::ZERO, |p| p.rest));
            let at = [a, b, c].map(|i| positions.get(i).copied().unwrap_or(Vec3::ZERO));
            let area = (rest[1] - rest[0]).perp_dot(rest[2] - rest[0]).abs() / 2.0;
            (stretch(rest, at).unwrap_or(0.0).max(0.0), area)
        })
        .collect()
}

/// Each particle's strain (ratio): the area-weighted mean of its
/// triangles'.
fn vertex_strains(drape: &Drape, positions: &[Vec3]) -> Vec<f32> {
    let mut sum = vec![(0.0f32, 0.0f32); drape.scene.particles.len()];
    for (t, (s, area)) in drape
        .scene
        .triangles
        .iter()
        .zip(triangle_strains(drape, positions))
    {
        for v in t.vertices {
            if let Some((ws, w)) = sum.get_mut(v as usize) {
                *ws += s * area;
                *w += area;
            }
        }
    }
    sum.into_iter()
        .map(|(ws, w)| if w > 0.0 { ws / w } else { 0.0 })
        .collect()
}

/// Each point's distance to the body (m), at most [`GAP_SEARCH_M`].
fn gaps(points: &[Vec3], body: Option<Body<'_>>) -> Vec<f32> {
    match body {
        None => vec![GAP_SEARCH_M; points.len()],
        Some(body) => {
            let surface = Surface::new(body);
            points
                .iter()
                .map(|p| surface.distance(*p, GAP_SEARCH_M))
                .collect()
        }
    }
}

/// A body surface with its triangles bucketed in a grid, for the
/// distance from points to it.
#[derive(Debug)]
pub struct Surface<'a> {
    positions: &'a [Vec3],
    triangles: &'a [[u32; 3]],
    cells: HashMap<[i32; 3], Vec<u32>>,
}

/// Grid cell of [`Surface`] (m).
const CELL_M: f32 = 0.04;

fn cell_of(p: Vec3) -> [i32; 3] {
    (p / CELL_M).floor().as_ivec3().to_array()
}

impl<'a> Surface<'a> {
    pub fn new((positions, triangles): Body<'a>) -> Self {
        let mut cells: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (i, t) in triangles.iter().enumerate() {
            let corners = t.map(|v| positions.get(v as usize).copied());
            let [Some(a), Some(b), Some(c)] = corners else {
                continue;
            };
            let (lo, hi) = (cell_of(a.min(b).min(c)), cell_of(a.max(b).max(c)));
            for x in lo[0]..=hi[0] {
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        cells.entry([x, y, z]).or_default().push(i as u32);
                    }
                }
            }
        }
        Self {
            positions,
            triangles,
            cells,
        }
    }

    /// Distance from `p` to the surface (m), or `limit` when farther.
    /// Searches shells of cells outwards: a triangle in shell `r + 1` is
    /// at least `r` cells away.
    pub fn distance(&self, p: Vec3, limit: f32) -> f32 {
        let home = cell_of(p);
        let mut best = limit;
        let shells = (limit / CELL_M).ceil() as i32 + 1;
        for r in 0..=shells {
            for x in -r..=r {
                for y in -r..=r {
                    for z in -r..=r {
                        if x.abs().max(y.abs()).max(z.abs()) != r {
                            continue;
                        }
                        let key = [home[0] + x, home[1] + y, home[2] + z];
                        for t in self.cells.get(&key).into_iter().flatten() {
                            if let Some(d) = self.to_triangle(p, *t as usize) {
                                best = best.min(d);
                            }
                        }
                    }
                }
            }
            if best <= r as f32 * CELL_M {
                break;
            }
        }
        best
    }

    fn to_triangle(&self, p: Vec3, t: usize) -> Option<f32> {
        let [a, b, c] = self
            .triangles
            .get(t)?
            .map(|v| self.positions.get(v as usize));
        let (q, _) = closest_feature(p, *a?, *b?, *c?);
        Some(q.distance(p))
    }
}

/// How one piece fits.
#[derive(Clone, Debug, Serialize)]
pub struct PieceFit {
    pub panel: PanelId,
    pub name: String,
    pub max_strain_pct: f64,
    /// Area-weighted.
    pub mean_strain_pct: f64,
    /// Share of its area at or above the tight threshold.
    pub tight_area_pct: f64,
    /// `None` without a body.
    pub mean_gap_mm: Option<f64>,
    /// Share of its vertices touching the body.
    pub touching_pct: Option<f64>,
}

/// Neighbouring triangles of one piece, all at or above the threshold.
#[derive(Clone, Debug, Serialize)]
pub struct TightSpot {
    pub panel: PanelId,
    pub name: String,
    pub strain_pct: f64,
    pub area_cm2: f64,
    /// Above the floor.
    pub height_cm: f64,
    /// The spot's centre on the pattern (mm, base size).
    pub at_mm: [f64; 2],
}

fn r1(v: impl Into<f64>) -> f64 {
    (v.into() * 10.0).round() / 10.0
}

/// Every piece's fit and the tight spots, the strongest first.
pub fn pieces(
    project: &Project,
    drape: &Drape,
    positions: &[Vec3],
    body: Option<Body<'_>>,
    threshold: f32,
) -> (Vec<PieceFit>, Vec<TightSpot>) {
    let strains = triangle_strains(drape, positions);
    let gaps = body.map(|b| gaps(positions, Some(b)));
    let triangles = &drape.scene.triangles;
    let mut fits = Vec::new();
    let mut spots = Vec::new();
    for piece in &drape.pieces {
        let name = project
            .panel(piece.panel)
            .map_or_else(|| piece.panel.to_string(), |p| p.name.clone());
        let mine: Vec<usize> = triangles
            .iter()
            .enumerate()
            .filter(|(_, t)| piece.particles.contains(&(t.vertices[0] as usize)))
            .map(|(i, _)| i)
            .collect();
        let of = |i: &usize| strains.get(*i).copied().unwrap_or_default();
        let area: f32 = mine.iter().map(|i| of(i).1).sum();
        let max = mine.iter().map(|i| of(i).0).fold(0.0, f32::max);
        let mean = mine.iter().map(|i| of(i).0 * of(i).1).sum::<f32>() / area.max(1e-12);
        let tight: Vec<usize> = mine
            .iter()
            .copied()
            .filter(|i| of(i).0 >= threshold)
            .collect();
        let tight_area: f32 = tight.iter().map(|i| of(i).1).sum();
        let piece_gaps = gaps
            .as_ref()
            .and_then(|g| g.get(piece.particles.clone()))
            .filter(|g| !g.is_empty());
        fits.push(PieceFit {
            panel: piece.panel,
            name: name.clone(),
            max_strain_pct: r1(max * 100.0),
            mean_strain_pct: r1(mean * 100.0),
            tight_area_pct: r1(tight_area / area.max(1e-12) * 100.0),
            mean_gap_mm: piece_gaps.map(|g| r1(g.iter().sum::<f32>() / g.len() as f32 * 1000.0)),
            touching_pct: piece_gaps.map(|g| {
                r1(g.iter().filter(|d| **d <= TOUCHING_M).count() as f32 / g.len() as f32 * 100.0)
            }),
        });
        for group in groups(&tight, triangles) {
            let (mut peak, mut area, mut centre, mut rest) =
                (0.0f32, 0.0f32, Vec3::ZERO, Vec2::ZERO);
            for i in &group {
                let (s, a) = of(i);
                peak = peak.max(s);
                area += a;
                let Some(t) = triangles.get(*i) else { continue };
                let pts = t
                    .vertices
                    .map(|v| positions.get(v as usize).copied().unwrap_or_default());
                let rs = t.vertices.map(|v| {
                    drape
                        .scene
                        .particles
                        .get(v as usize)
                        .map_or(Vec2::ZERO, |p| p.rest)
                });
                centre += (pts[0] + pts[1] + pts[2]) / 3.0 * a;
                rest += (rs[0] + rs[1] + rs[2]) / 3.0 * a;
            }
            let (centre, rest) = (centre / area.max(1e-12), rest / area.max(1e-12));
            let at = project.from_shown(
                piece.panel,
                DVec2::new(f64::from(rest.x), f64::from(rest.y)) * 1000.0,
            );
            spots.push(TightSpot {
                panel: piece.panel,
                name: name.clone(),
                strain_pct: r1(peak * 100.0),
                area_cm2: r1(area * 1e4),
                height_cm: r1(centre.y * 100.0),
                at_mm: [r1(at.x), r1(at.y)],
            });
        }
    }
    spots.sort_by(|a, b| {
        b.strain_pct
            .total_cmp(&a.strain_pct)
            .then(b.area_cm2.total_cmp(&a.area_cm2))
    });
    spots.truncate(MAX_SPOTS);
    (fits, spots)
}

/// The triangles `chosen` grouped into neighbours (sharing a vertex).
fn groups(chosen: &[usize], triangles: &[r3d_sim::scene::Triangle]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..chosen.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while let Some(&p) = parent.get(i) {
            if p == i {
                break;
            }
            let grand = parent.get(p).copied().unwrap_or(p);
            if let Some(slot) = parent.get_mut(i) {
                *slot = grand;
            }
            i = grand;
        }
        i
    }
    let mut first: HashMap<u32, usize> = HashMap::new();
    for (k, t) in chosen.iter().enumerate() {
        let Some(t) = triangles.get(*t) else { continue };
        for v in t.vertices {
            match first.get(&v) {
                Some(&other) => {
                    let (a, b) = (root(&mut parent, k), root(&mut parent, other));
                    if let Some(slot) = parent.get_mut(a) {
                        *slot = b;
                    }
                }
                None => {
                    first.insert(v, k);
                }
            }
        }
    }
    let mut out: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, t) in chosen.iter().enumerate() {
        let r = root(&mut parent, k);
        out.entry(r).or_default().push(*t);
    }
    out.into_values().collect()
}

/// A cut through a surface by a horizontal plane.
#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    /// In order along the cut (m).
    pub points: Vec<Vec3>,
    /// The last point joins the first.
    pub closed: bool,
}

impl Loop {
    /// Length along the points (m), the closing segment included.
    pub fn length(&self) -> f32 {
        let open: f32 = self.points.windows(2).map(|w| w[0].distance(w[1])).sum();
        let closing = match (self.closed, self.points.first(), self.points.last()) {
            (true, Some(a), Some(b)) => a.distance(*b),
            _ => 0.0,
        };
        open + closing
    }

    fn plan(&self) -> Vec<DVec2> {
        self.points
            .iter()
            .map(|p| DVec2::new(f64::from(p.x), f64::from(p.z)))
            .collect()
    }

    /// The convex outline seen from above: what a tape around it reads.
    fn hull(&self) -> Vec<DVec2> {
        polygon::convex_hull(&self.plan())
    }

    /// Length of a tape around it (m).
    pub fn tape(&self) -> f32 {
        polygon::perimeter(&self.hull()) as f32
    }

    fn centre(&self) -> DVec2 {
        let plan = self.plan();
        plan.iter().copied().sum::<DVec2>() / plan.len().max(1) as f64
    }

    /// Distance from `p` to the nearest point of the loop's segments (m).
    fn distance(&self, p: Vec3) -> f32 {
        let n = self.points.len();
        let segments = if self.closed { n } else { n.saturating_sub(1) };
        (0..segments)
            .filter_map(|i| {
                let a = *self.points.get(i)?;
                let b = *self.points.get((i + 1) % n)?;
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
                Some((a + ab * t).distance(p))
            })
            .fold(f32::MAX, f32::min)
    }
}

/// Cut the triangles of `positions` by the plane at height `y` into
/// loops. `same` names the vertex a vertex is sewn to (one name for the
/// whole group), so cuts through two pieces join across their seam.
/// Loops whose ends are close are closed ([`CLOSE_LOOP_M`]).
pub fn slice(
    positions: &[Vec3],
    triangles: impl Iterator<Item = [u32; 3]>,
    same: &dyn Fn(u32) -> u32,
    y: f32,
) -> Vec<Loop> {
    type Key = (u32, u32);
    let key = |a: u32, b: u32| {
        let (a, b) = (same(a), same(b));
        if a < b { (a, b) } else { (b, a) }
    };
    // Each crossed edge gives a point (both sides of a seam averaged);
    // each crossed triangle links two of them.
    let mut points: BTreeMap<Key, (Vec3, u32)> = BTreeMap::new();
    let mut links: BTreeMap<Key, Vec<Key>> = BTreeMap::new();
    for t in triangles {
        let mut crossed: Vec<Key> = Vec::with_capacity(2);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (Some(pa), Some(pb)) = (positions.get(a as usize), positions.get(b as usize))
            else {
                continue;
            };
            if (pa.y > y) == (pb.y > y) {
                continue;
            }
            let k = key(a, b);
            if k.0 == k.1 {
                continue;
            }
            let p = pa.lerp(*pb, (y - pa.y) / (pb.y - pa.y));
            let slot = points.entry(k).or_insert((Vec3::ZERO, 0));
            slot.0 += p;
            slot.1 += 1;
            crossed.push(k);
        }
        if let [k1, k2] = crossed[..] {
            links.entry(k1).or_default().push(k2);
            links.entry(k2).or_default().push(k1);
        }
    }
    let at = |k: &Key| points.get(k).map(|(sum, n)| *sum / (*n).max(1) as f32);
    let mut seen = std::collections::BTreeSet::new();
    let mut loops = Vec::new();
    // Open chains from their ends first, then what is left (cycles).
    let ends: Vec<Key> = links
        .iter()
        .filter(|(_, l)| l.len() == 1)
        .map(|(k, _)| *k)
        .chain(links.keys().copied())
        .collect();
    for start in ends {
        if seen.contains(&start) {
            continue;
        }
        let mut chain = vec![start];
        seen.insert(start);
        let mut current = start;
        let closed = loop {
            let next = links
                .get(&current)
                .into_iter()
                .flatten()
                .find(|k| !seen.contains(*k))
                .copied();
            match next {
                Some(k) => {
                    seen.insert(k);
                    chain.push(k);
                    current = k;
                }
                None => {
                    break chain.len() > 2
                        && links.get(&current).is_some_and(|l| l.contains(&start));
                }
            }
        };
        let points: Vec<Vec3> = chain.iter().filter_map(at).collect();
        if points.len() < 2 {
            continue;
        }
        let closed = closed
            || (points.len() > 2
                && points
                    .first()
                    .zip(points.last())
                    .is_some_and(|(a, b)| a.distance(*b) < CLOSE_LOOP_M));
        loops.push(Loop { points, closed });
    }
    join(loops)
}

/// Join open loops whose ends meet (a seam not sewn point for point,
/// a cut through a piece left open at its seam) into longer ones.
fn join(mut loops: Vec<Loop>) -> Vec<Loop> {
    loop {
        let open: Vec<usize> = (0..loops.len())
            .filter(|i| loops.get(*i).is_some_and(|l| !l.closed))
            .collect();
        let mut pair = None;
        'search: for (n, &i) in open.iter().enumerate() {
            for &j in open.iter().skip(n + 1) {
                let (Some(a), Some(b)) = (loops.get(i), loops.get(j)) else {
                    continue;
                };
                let ends = |l: &Loop| (l.points.first().copied(), l.points.last().copied());
                let ((Some(a0), Some(a1)), (Some(b0), Some(b1))) = (ends(a), ends(b)) else {
                    continue;
                };
                // Which ends meet: a's last with b's first, and so on.
                for (flip_a, flip_b, d) in [
                    (false, false, a1.distance(b0)),
                    (false, true, a1.distance(b1)),
                    (true, false, a0.distance(b0)),
                    (true, true, a0.distance(b1)),
                ] {
                    if d < CLOSE_LOOP_M {
                        pair = Some((i, j, flip_a, flip_b));
                        break 'search;
                    }
                }
            }
        }
        let Some((i, j, flip_a, flip_b)) = pair else {
            return loops;
        };
        let mut b = loops.remove(j);
        let Some(a) = loops.get_mut(i) else {
            return loops;
        };
        if flip_a {
            a.points.reverse();
        }
        if flip_b {
            b.points.reverse();
        }
        a.points.extend(b.points);
        a.closed = a.points.len() > 2
            && a.points
                .first()
                .zip(a.points.last())
                .is_some_and(|(p, q)| p.distance(*q) < CLOSE_LOOP_M);
    }
}

/// Where to take the tape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Probe {
    /// Around the body at this height (m).
    Height(f32),
    /// Around the cloth passing nearest to this point, at its height.
    Near(Vec3),
}

/// A tape measure's reading.
#[derive(Clone, Debug, PartialEq)]
pub struct Girth {
    pub height: f32,
    /// The garment's loop, if the cut found one.
    pub garment: Option<Loop>,
    /// Around the body inside the garment (or around the torso), by tape
    /// (m).
    pub body: Option<f32>,
    /// With a [`Probe::Near`] and a body: the cloth's distance to it (m).
    pub gap: Option<f32>,
}

impl Girth {
    /// Garment circumference minus body circumference (m), when both are
    /// known and the garment's loop is closed.
    pub fn ease(&self) -> Option<f32> {
        let garment = self.garment.as_ref().filter(|l| l.closed)?;
        Some(garment.length() - self.body?)
    }

    /// The reading for agents (cm, mm for the gap), named `at`.
    pub fn to_json(&self, at: &str) -> Value {
        let cm = |v: f32| r1(v * 100.0);
        json!({
            "at": at,
            "height_cm": cm(self.height),
            "garment_cm": self.garment.as_ref().map(|l| cm(l.length())),
            "garment_tape_cm": self.garment.as_ref().filter(|l| l.closed).map(|l| cm(l.tape())),
            "closed": self.garment.as_ref().is_some_and(|l| l.closed),
            "body_cm": self.body.map(cm),
            "ease_cm": self.ease().map(cm),
            "gap_mm": self.gap.map(|g| r1(g * 1000.0)),
        })
    }
}

/// Take the tape on the cloth of `drape` at `positions`, around `body` if
/// there is one.
pub fn girth(drape: &Drape, positions: &[Vec3], body: Option<Body<'_>>, probe: Probe) -> Girth {
    let height = match probe {
        Probe::Height(y) => y,
        Probe::Near(p) => p.y,
    };
    let sewn = sewn_to(drape);
    let same = |v: u32| sewn.get(&v).copied().unwrap_or(v);
    let cloth = slice(
        positions,
        drape.scene.triangles.iter().map(|t| t.vertices),
        &same,
        height,
    );
    let bodies = body
        .map(|(p, t)| {
            slice(p, t.iter().copied(), &|v| v, height)
                .into_iter()
                .filter(|l| l.closed)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let garment = match probe {
        Probe::Near(p) => cloth
            .into_iter()
            .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p))),
        Probe::Height(_) => around_axis(cloth.into_iter().filter(|l| l.closed)),
    };
    // The body's loop inside the garment's (the torso in a shirt, an arm
    // in a sleeve), the largest; without a garment loop, the torso.
    let inner = match garment.as_ref().filter(|l| l.closed) {
        Some(g) => {
            let outline = g.hull();
            bodies
                .iter()
                .filter(|b| polygon::contains(&outline, b.centre()))
                .max_by(|a, b| a.tape().total_cmp(&b.tape()))
                .cloned()
        }
        None => around_axis(bodies.iter().cloned()),
    };
    Girth {
        height,
        garment,
        body: inner.map(|l| l.tape()),
        gap: match (probe, body) {
            (Probe::Near(p), Some(b)) => Some(Surface::new(b).distance(p, 1.0)),
            _ => None,
        },
    }
}

/// The loop around the body's vertical axis (x = 0), nearest to it.
fn around_axis(loops: impl Iterator<Item = Loop>) -> Option<Loop> {
    loops
        .filter(|l| {
            let c = l.centre();
            polygon::contains(&l.hull(), DVec2::new(0.0, c.y))
        })
        .min_by(|a, b| a.centre().x.abs().total_cmp(&b.centre().x.abs()))
}

/// For each sewn particle, one particle standing for its whole group
/// (the seams' union–find), so cuts join across seams.
fn sewn_to(drape: &Drape) -> HashMap<u32, u32> {
    let mut parent: HashMap<u32, u32> = HashMap::new();
    fn root(parent: &HashMap<u32, u32>, mut v: u32) -> u32 {
        while let Some(&p) = parent.get(&v) {
            if p == v {
                break;
            }
            v = p;
        }
        v
    }
    for [a, b] in &drape.scene.seams {
        let (ra, rb) = (root(&parent, *a), root(&parent, *b));
        parent.entry(ra).or_insert(ra);
        parent.entry(rb).or_insert(rb);
        if ra != rb {
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            parent.insert(hi, lo);
        }
    }
    let keys: Vec<u32> = parent.keys().copied().collect();
    keys.into_iter().map(|v| (v, root(&parent, v))).collect()
}

/// Heights (m) of the body's chest (over the bust points), waist
/// (narrowest girth) and hips (largest), for the tape.
pub fn landmarks(form: &Form) -> Result<[(&'static str, f32); 3], CommandError> {
    let model = crate::queries::body_model()?;
    let (p, t) = (&form.positions, &form.triangles[..]);
    let chest = model
        .landmarks()
        .bust
        .iter()
        .filter_map(|i| p.get(*i as usize))
        .map(|v| v.y)
        .sum::<f32>()
        / 2.0;
    let measured = r3d_avatar::measure::measure(p, t, model.landmarks());
    let hips = r3d_avatar::measure::heights(p, t).hips;
    Ok([
        ("chest", chest),
        ("waist", measured.waist_height_cm / 100.0),
        ("hips", hips),
    ])
}

/// The latest drape of the workspace's project, refused when nothing is
/// placed.
fn latest(ws: &Workspace) -> Result<Latest, WorkspaceError> {
    let latest = Latest::load(ws.dir(), ws.session().project())?;
    if latest.drape.pieces.is_empty() {
        return Err(CommandError::NothingToSimulate.into());
    }
    Ok(latest)
}

fn height_arg(name: &'static str, cm: f64) -> Result<f32, CommandError> {
    if !(0.0..=300.0).contains(&cm) {
        return Err(CommandError::invalid(name, Invalid::Range(0.0, 300.0)));
    }
    Ok(cm as f32 / 100.0)
}

fn default_threshold() -> f64 {
    5.0
}

/// How the draped garment fits.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FitReport {
    /// Strain (%) from which the cloth counts as tight (default 5).
    #[serde(default = "default_threshold")]
    pub threshold_pct: f64,
    /// More heights above the floor (cm) to take the tape at, besides the
    /// chest, waist and hips.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heights_cm: Vec<f64>,
}

impl Described for FitReport {
    const SUMMARY: &'static str = "How the draped garment fits, read from the cached drape (`simulate` first; says `drape`: simulated or start): per piece its largest and mean strain (% stretch beyond the pattern), the share of its area at or above `threshold_pct` (default 5), its mean gap to the body (mm) and the share touching it; the tight spots (neighbouring cloth at or above the threshold, strongest first, at most 10: piece, peak strain, area in cm², height above the floor in cm, centre on the pattern in mm); and the ease (garment circumference minus body circumference, cm) at the chest, waist and hips and at any `heights_cm`.";
}

impl ActionSpec for FitReport {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        if !(0.0..=100.0).contains(&self.threshold_pct) {
            return Err(CommandError::invalid("threshold_pct", Invalid::Range(0.0, 100.0)).into());
        }
        let extra = self
            .heights_cm
            .iter()
            .map(|h| height_arg("heights_cm", *h).map(|y| (format!("{h} cm"), y)))
            .collect::<Result<Vec<_>, _>>()?;
        let latest = latest(ws)?;
        let project = ws.session().project();
        let body = latest
            .body
            .as_deref()
            .zip(latest.form.as_ref().map(|f| &f.triangles[..]));
        let (pieces, spots) = pieces(
            project,
            &latest.drape,
            &latest.positions,
            body,
            self.threshold_pct as f32 / 100.0,
        );
        let mut heights: Vec<(String, f32)> = match &latest.form {
            Some(f) => landmarks(f)?
                .into_iter()
                .map(|(n, y)| (n.to_owned(), y))
                .collect(),
            None => Vec::new(),
        };
        heights.extend(extra);
        let ease: Vec<Value> = heights
            .iter()
            .map(|(name, y)| {
                girth(&latest.drape, &latest.positions, body, Probe::Height(*y)).to_json(name)
            })
            .collect();
        Ok(json!({
            "drape": latest.state(),
            "threshold_pct": self.threshold_pct,
            "pieces": pieces,
            "tight_spots": spots,
            "ease": ease,
        }))
    }
}

/// A tape measure on the drape.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FitTape {
    /// Height above the floor (cm): the loop around the body there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_cm: Option<f64>,
    /// A point on the drape (m, x right, y up, z forward): the loop
    /// through it at its height (a sleeve, the body…), and the gap to the
    /// body there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub near: Option<[f64; 3]>,
}

impl Described for FitTape {
    const SUMMARY: &'static str = "A tape measure on the cached drape: at `height_cm`, the garment's circumference around the body (cloth length along the horizontal cut, folds included), the body's (a tape around its section) and the ease between them (cm); or, at a point `near` on the drape (m), the same for the loop passing there (a sleeve, the body) plus the gap between the cloth and the body there (mm). `closed` is false when the cut does not go round (an open front, a hem): no ease then.";
}

impl ActionSpec for FitTape {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let probe = match (self.near, self.height_cm) {
            (Some(p), _) => {
                if p.iter().any(|c| !c.is_finite()) {
                    return Err(CommandError::invalid("near", Invalid::NotFinite).into());
                }
                Probe::Near(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32))
            }
            (None, Some(h)) => Probe::Height(height_arg("height_cm", h)?),
            (None, None) => return Err(CommandError::invalid("height_cm", Invalid::Empty).into()),
        };
        let latest = latest(ws)?;
        let body = latest
            .body
            .as_deref()
            .zip(latest.form.as_ref().map(|f| &f.triangles[..]));
        let mut out = girth(&latest.drape, &latest.positions, body, probe).to_json("tape");
        out["drape"] = json!(latest.state());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_stretched_triangle_reads_its_stretch_in_any_orientation() {
        let rest = [Vec2::ZERO, Vec2::new(0.1, 0.0), Vec2::new(0.0, 0.1)];
        let flat = rest.map(|r| Vec3::new(r.x, r.y, 0.0));
        assert!(stretch(rest, flat).unwrap().abs() < 1e-6);
        // 10 % along x, turned and lifted into space.
        let turn = glam::Quat::from_euler(glam::EulerRot::XYZ, 0.4, 1.1, -0.3);
        let at = rest.map(|r| turn * Vec3::new(r.x * 1.1, r.y, 0.0) + Vec3::ONE);
        assert!((stretch(rest, at).unwrap() - 0.1).abs() < 1e-5);
        // Squashed: no stretch beyond the pattern.
        let squashed = rest.map(|r| Vec3::new(r.x * 0.8, r.y * 0.9, 0.0));
        assert!(stretch(rest, squashed).unwrap() < 0.0);
        assert!(stretch([Vec2::ZERO; 3], flat).is_none());
    }

    /// A tube of `n` × `rows` quads, radius `r`, from y = 0 to 1.
    fn tube(r: f32, n: u32, rows: u32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let mut p = Vec::new();
        for j in 0..=rows {
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                p.push(Vec3::new(r * a.cos(), j as f32 / rows as f32, r * a.sin()));
            }
        }
        let mut t = Vec::new();
        for j in 0..rows {
            for i in 0..n {
                let (a, b) = (j * n + i, j * n + (i + 1) % n);
                t.push([a, b, b + n]);
                t.push([a, b + n, a + n]);
            }
        }
        (p, t)
    }

    #[test]
    fn a_tube_cuts_into_one_closed_loop() {
        let (p, t) = tube(0.2, 64, 4);
        let loops = slice(&p, t.iter().copied(), &|v| v, 0.37);
        assert_eq!(loops.len(), 1);
        let l = &loops[0];
        assert!(l.closed);
        let polygon = 2.0 * 64.0 * 0.2 * (std::f32::consts::PI / 64.0).sin();
        assert!((l.length() - polygon).abs() < 1e-4, "{}", l.length());
        assert!((l.tape() - polygon).abs() < 1e-4);
        assert!(slice(&p, t.iter().copied(), &|v| v, 1.5).is_empty());
    }

    #[test]
    fn two_halves_join_across_their_ends() {
        let cut = |r: f32, gone: &[u32]| {
            let (p, t) = tube(r, 64, 4);
            let kept: Vec<[u32; 3]> = t
                .iter()
                .copied()
                .filter(|tri| !gone.contains(&(tri[0] % 64)))
                .collect();
            slice(&p, kept.iter().copied(), &|v| v, 0.5)
        };
        // Two opposite gaps of 4 cm: two open halves.
        let halves = cut(0.2, &[0, 1, 32, 33]);
        assert_eq!(halves.len(), 2);
        assert!(halves.iter().all(|l| !l.closed));
        // One gap of 1 cm (a seam not quite closed): one loop.
        let joined = cut(0.1, &[0]);
        assert_eq!(joined.len(), 1);
        assert!(joined[0].closed);
    }

    #[test]
    fn the_tape_reads_the_ease_and_the_gap() {
        let (body, bt) = tube(0.15, 64, 8);
        let (cloth, ct) = tube(0.17, 48, 8);
        let ease = |g: &Girth| g.ease().unwrap();
        let body_c = 2.0 * 64.0 * 0.15 * (std::f32::consts::PI / 64.0).sin();
        let cloth_c = 2.0 * 48.0 * 0.17 * (std::f32::consts::PI / 48.0).sin();
        let lone = slice(&cloth, ct.iter().copied(), &|v| v, 0.5);
        assert_eq!(lone.len(), 1);
        let bodies = slice(&body, bt.iter().copied(), &|v| v, 0.5);
        let outline = lone[0].hull();
        assert!(polygon::contains(&outline, bodies[0].centre()));
        let surface = Surface::new((&body, &bt));
        let gap = surface.distance(Vec3::new(0.17, 0.5, 0.0), 1.0);
        assert!((gap - 0.02).abs() < 1e-3, "{gap}");
        let g = Girth {
            height: 0.5,
            garment: lone.into_iter().next(),
            body: Some(bodies[0].tape()),
            gap: Some(gap),
        };
        assert!((ease(&g) - (cloth_c - body_c)).abs() < 1e-4);
        assert_eq!(g.to_json("x")["closed"], json!(true));
    }

    #[test]
    fn levels_run_from_loose_to_tight() {
        assert_eq!(FitMap::Strain.level(0.0), 0.0);
        assert_eq!(FitMap::Strain.level(25.0), 1.0);
        assert_eq!(FitMap::Gap.level(0.0), 1.0);
        assert_eq!(FitMap::Gap.level(80.0), 0.0);
        assert!((FitMap::Gap.level(25.0) - 0.5).abs() < 1e-6);
        assert_eq!(FitMap::from_name("gap"), Some(FitMap::Gap));
        assert_eq!(FitMap::from_name("stress"), None);
    }

    #[test]
    fn neighbouring_triangles_group_together() {
        let tri = |a, b, c| r3d_sim::scene::Triangle {
            vertices: [a, b, c],
            fabric: 0,
            warp: Vec2::X,
        };
        let ts = [tri(0, 1, 2), tri(2, 3, 4), tri(5, 6, 7), tri(7, 8, 9)];
        let mut g = groups(&[0, 1, 2, 3], &ts);
        g.iter_mut().for_each(|v| v.sort());
        g.sort();
        assert_eq!(g, vec![vec![0, 1], vec![2, 3]]);
    }
}
