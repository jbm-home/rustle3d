//! The diagnostic report of a drape (spec 34): where it goes wrong and
//! why, for an agent to read. Faces chosen by regions (on the pattern or
//! on the drape), or the worst faces of the whole drape, each with its
//! stretch, its area against the pattern's, its distance to the body,
//! the nearest cloth that is not its own neighbourhood, and the gaps of
//! the seams at its corners; every seam's gap; what to replay it from.
//!
//! Like the fit maps (spec 24) it reads positions only: the drape from
//! the CPU or the GPU solver, cached headless or live.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use glam::{Vec2, Vec3};
use r3d_core::{PanelId, Project, SeamId};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::ActionSpec;
use crate::drape::{Latest, fingerprint};
use crate::error::{CommandError, Invalid};
use crate::fit::{Surface, stretch};
use crate::spec::Described;
use crate::workspace::{Workspace, WorkspaceError};

/// Other cloth this close to a face is reported as its contact (m): a
/// few times the thickness and the solver's skin margin (docs/physics.md
/// §5), so layers lying on each other show, not only those crossing.
const CONTACT_M: f32 = 0.01;
/// Cloth within this distance of a face along the pattern (m, seams
/// crossed at no cost) is its own neighbourhood, never a contact.
const ALONG_M: f32 = 0.03;
/// The body is searched this far from the cloth when it moves (m).
const BODY_SEARCH_M: f32 = 0.1;
/// A particle deeper than this inside the body counts as inside (m): the
/// tolerance of the simulation tests (CLAUDE.md, Tests).
const INSIDE_M: f32 = 0.002;
/// Faces listed per list by default, and at most.
const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 200;
/// Region radii accepted (mm).
const MAX_RADIUS_MM: f64 = 2000.0;

/// Where to look.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Region {
    /// A disc on a piece of the pattern: its faces whose centre lies
    /// within `radius_mm` of `at_mm` (the piece's coordinates, base size,
    /// as `fit.report` gives its tight spots).
    Pattern {
        panel: PanelId,
        at_mm: [f64; 2],
        radius_mm: f64,
    },
    /// A ball on the drape: its faces whose centre lies within
    /// `radius_mm` of `point_m` (m, x right, y up, z forward, as
    /// `fit.tape` takes its points).
    Drape { point_m: [f64; 3], radius_mm: f64 },
}

/// The diagnostic report of the drape.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrapeDiagnose {
    /// Regions to report on; omitted: the whole drape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<Region>,
    /// Faces listed per list (1–200, default 20).
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    DEFAULT_LIMIT
}

impl Described for DrapeDiagnose {
    const SUMMARY: &'static str = "Where the drape goes wrong and why, read from the cached drape (`simulate` first; says `drape`: simulated or start). Per face: piece, place on the pattern (mm) and on the drape (m), strain (% stretch beyond the pattern), area against the pattern's (below 1: crumpled), signed distance to the body (mm, negative inside; unsigned while the body moves), the nearest other cloth within 10 mm that is not the face's own neighbourhood (a fold or another layer: piece and distance), and the largest gap of a seam at its corners. With `regions` (a disc on a piece's pattern, or a ball on the drape): each region's summary and its faces, the most strained first. Always: counts (faces, particles that are not numbers, particles more than 2 mm inside the body), every seam's largest and mean gap (widest first), the worst faces of the whole drape (most strained, most crumpled, deepest in the body), at most `limit` (default 20) per list, and `replay`: the project version (`fingerprint`), simulated time and motion this drape comes from.";
}

/// What every face's report is built from.
struct Facts<'a> {
    project: &'a Project,
    latest: &'a Latest,
    /// Each particle's piece (index into `latest.drape.pieces`).
    piece_of: Vec<usize>,
    /// Each triangle's index among its piece's.
    local: Vec<usize>,
    /// Each particle's neighbours along the cloth and the rest length to
    /// them (m; 0 across a seam).
    links: Vec<Vec<(u32, f32)>>,
    /// Particles bucketed by cells of `CONTACT_M`.
    grid: HashMap<[i32; 3], Vec<u32>>,
    /// The largest gap of the seam pairs at each particle (m).
    seam_gap: HashMap<u32, f32>,
    /// Each particle's distance to the body (m), signed when the body
    /// stands still; `None` without a body.
    body: Option<Vec<f32>>,
    signed: bool,
}

fn cell(p: Vec3) -> [i32; 3] {
    (p / CONTACT_M).floor().as_ivec3().to_array()
}

fn r1(v: f32) -> f64 {
    (f64::from(v) * 10.0).round() / 10.0
}

fn r3(v: f32) -> f64 {
    (f64::from(v) * 1000.0).round() / 1000.0
}

impl<'a> Facts<'a> {
    fn new(project: &'a Project, latest: &'a Latest) -> Self {
        let drape = &latest.drape;
        let particles = &drape.scene.particles;
        let n = particles.len();
        let mut piece_of = vec![0; n];
        for (k, piece) in drape.pieces.iter().enumerate() {
            for slot in piece_of
                .get_mut(piece.particles.clone())
                .unwrap_or_default()
            {
                *slot = k;
            }
        }
        let mut count = vec![0; drape.pieces.len()];
        let local = drape
            .scene
            .triangles
            .iter()
            .map(|t| {
                let k = piece_of.get(t.vertices[0] as usize).copied().unwrap_or(0);
                let slot = count.get_mut(k);
                slot.map_or(0, |c| {
                    *c += 1;
                    *c - 1
                })
            })
            .collect();
        let rest = |i: u32| particles.get(i as usize).map_or(Vec2::ZERO, |p| p.rest);
        let mut links: Vec<Vec<(u32, f32)>> = vec![Vec::new(); n];
        let mut link = |a: u32, b: u32, d: f32| {
            for (x, y) in [(a, b), (b, a)] {
                if let Some(l) = links.get_mut(x as usize) {
                    l.push((y, d));
                }
            }
        };
        for t in &drape.scene.triangles {
            let [a, b, c] = t.vertices;
            for (x, y) in [(a, b), (b, c), (c, a)] {
                link(x, y, rest(x).distance(rest(y)));
            }
        }
        let at = |i: u32| {
            latest
                .positions
                .get(i as usize)
                .copied()
                .unwrap_or_default()
        };
        let mut seam_gap: HashMap<u32, f32> = HashMap::new();
        for [a, b] in &drape.scene.seams {
            link(*a, *b, 0.0);
            let gap = at(*a).distance(at(*b));
            for v in [*a, *b] {
                let g = seam_gap.entry(v).or_insert(0.0);
                *g = g.max(gap);
            }
        }
        let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (i, p) in latest.positions.iter().enumerate() {
            if p.is_finite() {
                grid.entry(cell(*p)).or_default().push(i as u32);
            }
        }
        let moving = latest.animation.is_some();
        let body = match (&drape.scene.collider, &latest.body, &latest.form) {
            (Some(collider), _, _) if !moving => Some(
                latest
                    .positions
                    .iter()
                    .map(|p| collider.sample(*p).map_or(BODY_SEARCH_M, |c| c.distance))
                    .collect(),
            ),
            (_, Some(body), Some(form)) => {
                let surface = Surface::new((body, &form.triangles));
                Some(
                    latest
                        .positions
                        .iter()
                        .map(|p| surface.distance(*p, BODY_SEARCH_M))
                        .collect(),
                )
            }
            _ => None,
        };
        Self {
            project,
            latest,
            piece_of,
            local,
            links,
            grid,
            seam_gap,
            body,
            signed: !moving,
        }
    }

    fn position(&self, i: u32) -> Vec3 {
        self.latest
            .positions
            .get(i as usize)
            .copied()
            .unwrap_or_default()
    }

    fn corners(&self, t: usize) -> [u32; 3] {
        self.latest
            .drape
            .scene
            .triangles
            .get(t)
            .map_or([0; 3], |t| t.vertices)
    }

    fn centre(&self, t: usize) -> Vec3 {
        self.corners(t)
            .map(|v| self.position(v))
            .iter()
            .sum::<Vec3>()
            / 3.0
    }

    /// Strain (ratio) and area on the drape over area on the pattern.
    fn shape(&self, t: usize) -> (f32, f32) {
        let particles = &self.latest.drape.scene.particles;
        let c = self.corners(t);
        let rest = c.map(|v| particles.get(v as usize).map_or(Vec2::ZERO, |p| p.rest));
        let at = c.map(|v| self.position(v));
        let rest_area = (rest[1] - rest[0]).perp_dot(rest[2] - rest[0]).abs() / 2.0;
        let area = (at[1] - at[0]).cross(at[2] - at[0]).length() / 2.0;
        let strain = stretch(rest, at).unwrap_or(0.0).max(0.0);
        let ratio = if rest_area > 1e-12 {
            area / rest_area
        } else {
            1.0
        };
        (strain, if ratio.is_finite() { ratio } else { 0.0 })
    }

    fn piece(&self, t: usize) -> Option<&crate::drape::Piece> {
        let k = self.piece_of.get(self.corners(t)[0] as usize)?;
        self.latest.drape.pieces.get(*k)
    }

    fn name(&self, panel: PanelId) -> String {
        self.project
            .panel(panel)
            .map_or_else(|| panel.to_string(), |p| p.name.clone())
    }

    /// The face's centre on its piece's pattern (mm, base size).
    fn on_pattern(&self, t: usize) -> Option<(PanelId, DVec2)> {
        let piece = self.piece(t)?;
        let particles = &self.latest.drape.scene.particles;
        let rest = self
            .corners(t)
            .map(|v| particles.get(v as usize).map_or(Vec2::ZERO, |p| p.rest))
            .iter()
            .sum::<Vec2>()
            / 3.0;
        let shown = DVec2::new(f64::from(rest.x), f64::from(rest.y)) * 1000.0;
        Some((piece.panel, self.project.from_shown(piece.panel, shown)))
    }

    /// Particles within `ALONG_M` of the face along the cloth.
    fn neighbourhood(&self, t: usize) -> HashSet<u32> {
        let mut best: HashMap<u32, f32> = HashMap::new();
        let mut heap = BinaryHeap::new();
        for v in self.corners(t) {
            best.insert(v, 0.0);
            heap.push(Reverse((0u32, v)));
        }
        // Non-negative floats order as their bits.
        while let Some(Reverse((bits, v))) = heap.pop() {
            let d = f32::from_bits(bits);
            if best.get(&v).is_some_and(|b| *b < d) {
                continue;
            }
            for (w, len) in self.links.get(v as usize).map_or(&[][..], |l| &l[..]) {
                let nd = d + len;
                if nd <= ALONG_M && best.get(w).is_none_or(|b| nd < *b) {
                    best.insert(*w, nd);
                    heap.push(Reverse((nd.to_bits(), *w)));
                }
            }
        }
        best.into_keys().collect()
    }

    /// The nearest particle within `CONTACT_M` of the face's centre that
    /// is not its neighbourhood, and its distance (m).
    fn contact(&self, t: usize) -> Option<(u32, f32)> {
        let c = self.centre(t);
        let own = self.neighbourhood(t);
        let [x, y, z] = cell(c);
        let mut best: Option<(u32, f32)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let Some(list) = self.grid.get(&[x + dx, y + dy, z + dz]) else {
                        continue;
                    };
                    for i in list {
                        let d = self.position(*i).distance(c);
                        if d <= CONTACT_M && !own.contains(i) && best.is_none_or(|(_, b)| d < b) {
                            best = Some((*i, d));
                        }
                    }
                }
            }
        }
        best
    }

    /// The face's distance to the body (m): its closest corner.
    fn body_distance(&self, t: usize) -> Option<f32> {
        let body = self.body.as_ref()?;
        self.corners(t)
            .iter()
            .filter_map(|v| body.get(*v as usize).copied())
            .reduce(f32::min)
    }

    fn report(&self, t: usize) -> Value {
        let (strain, ratio) = self.shape(t);
        let centre = self.centre(t);
        let (panel, at) = self.on_pattern(t).unzip();
        let seam_gap = self
            .corners(t)
            .iter()
            .filter_map(|v| self.seam_gap.get(v).copied())
            .reduce(f32::max);
        let contact = self.contact(t).map(|(i, d)| {
            let piece = self
                .piece_of
                .get(i as usize)
                .and_then(|k| self.latest.drape.pieces.get(*k));
            json!({
                "panel": piece.map(|p| p.panel),
                "name": piece.map(|p| self.name(p.panel)),
                "distance_mm": r1(d * 1000.0),
            })
        });
        json!({
            "panel": panel,
            "name": panel.map(|p| self.name(p)),
            "triangle": self.local.get(t),
            "at_mm": at.map(|a| [(a.x * 10.0).round() / 10.0, (a.y * 10.0).round() / 10.0]),
            "point_m": [r3(centre.x), r3(centre.y), r3(centre.z)],
            "height_cm": r1(centre.y * 100.0),
            "strain_pct": r1(strain * 100.0),
            "area_ratio": r3(ratio),
            "body_mm": self.body_distance(t).map(|d| r1(d * 1000.0)),
            "contact": contact,
            "seam_gap_mm": seam_gap.map(|g| r1(g * 1000.0)),
        })
    }

    fn in_region(&self, t: usize, region: &Region) -> bool {
        match region {
            Region::Pattern {
                panel,
                at_mm,
                radius_mm,
            } => self.on_pattern(t).is_some_and(|(p, at)| {
                p == *panel && at.distance(DVec2::from_array(*at_mm)) <= *radius_mm
            }),
            Region::Drape { point_m, radius_mm } => {
                let p = Vec3::new(point_m[0] as f32, point_m[1] as f32, point_m[2] as f32);
                f64::from(self.centre(t).distance(p)) * 1000.0 <= *radius_mm
            }
        }
    }

    fn seams(&self, limit: usize) -> Vec<Value> {
        let pairs = &self.latest.drape.scene.seams;
        let mut out: Vec<(SeamId, f32, f32, Vec<PanelId>)> = self
            .latest
            .drape
            .seams
            .iter()
            .filter_map(|(id, range)| {
                let gaps: Vec<f32> = pairs
                    .get(range.clone())?
                    .iter()
                    .map(|[a, b]| self.position(*a).distance(self.position(*b)))
                    .collect();
                let max = gaps.iter().copied().reduce(f32::max)?;
                let mean = gaps.iter().sum::<f32>() / gaps.len() as f32;
                let mut panels: Vec<PanelId> = pairs
                    .get(range.clone())?
                    .iter()
                    .flatten()
                    .filter_map(|v| self.piece_of.get(*v as usize))
                    .filter_map(|k| self.latest.drape.pieces.get(*k).map(|p| p.panel))
                    .collect();
                panels.sort();
                panels.dedup();
                Some((*id, max, mean, panels))
            })
            .collect();
        out.sort_by(|a, b| b.1.total_cmp(&a.1));
        out.truncate(limit);
        out.into_iter()
            .map(|(id, max, mean, panels)| {
                json!({
                    "seam": id,
                    "panels": panels.iter().map(|p| self.name(*p)).collect::<Vec<_>>(),
                    "max_gap_mm": r1(max * 1000.0),
                    "mean_gap_mm": r1(mean * 1000.0),
                })
            })
            .collect()
    }
}

/// `faces` ordered by `key`, the largest first, at most `limit`.
fn worst(faces: &[usize], limit: usize, key: impl Fn(usize) -> Option<f32>) -> Vec<usize> {
    let mut keyed: Vec<(usize, f32)> = faces
        .iter()
        .filter_map(|t| key(*t).map(|k| (*t, k)))
        .collect();
    keyed.sort_by(|a, b| b.1.total_cmp(&a.1));
    keyed.truncate(limit);
    keyed.into_iter().map(|(t, _)| t).collect()
}

fn check(region: &Region) -> Result<(), CommandError> {
    let (radius, finite) = match region {
        Region::Pattern {
            at_mm, radius_mm, ..
        } => (*radius_mm, at_mm.iter().all(|v| v.is_finite())),
        Region::Drape { point_m, radius_mm } => (*radius_mm, point_m.iter().all(|v| v.is_finite())),
    };
    if !finite {
        return Err(CommandError::invalid("regions", Invalid::NotFinite));
    }
    if !(radius > 0.0 && radius <= MAX_RADIUS_MM) {
        return Err(CommandError::invalid(
            "radius_mm",
            Invalid::Range(0.0, MAX_RADIUS_MM),
        ));
    }
    Ok(())
}

impl ActionSpec for DrapeDiagnose {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        if !(1..=MAX_LIMIT).contains(&self.limit) {
            return Err(
                CommandError::invalid("limit", Invalid::Range(1.0, f64::from(MAX_LIMIT))).into(),
            );
        }
        let project = ws.session().project();
        for region in &self.regions {
            check(region)?;
            if let Region::Pattern { panel, .. } = region
                && project.panel(*panel).is_none()
            {
                return Err(CommandError::not_found(*panel).into());
            }
        }
        let latest = Latest::load(ws.dir(), project)?;
        if latest.drape.pieces.is_empty() {
            return Err(CommandError::NothingToSimulate.into());
        }
        let facts = Facts::new(project, &latest);
        let limit = self.limit as usize;
        let all: Vec<usize> = (0..latest.drape.scene.triangles.len()).collect();
        let list = |faces: Vec<usize>| -> Vec<Value> {
            faces.into_iter().map(|t| facts.report(t)).collect()
        };
        let regions: Vec<Value> = self
            .regions
            .iter()
            .map(|region| {
                let faces: Vec<usize> = all
                    .iter()
                    .copied()
                    .filter(|t| facts.in_region(*t, region))
                    .collect();
                let shapes: Vec<(f32, f32)> = faces.iter().map(|t| facts.shape(*t)).collect();
                let max = shapes.iter().map(|s| s.0).reduce(f32::max);
                let mean = (!shapes.is_empty())
                    .then(|| shapes.iter().map(|s| s.0).sum::<f32>() / shapes.len() as f32);
                let body = faces
                    .iter()
                    .filter_map(|t| facts.body_distance(*t))
                    .reduce(f32::min);
                let gap = faces
                    .iter()
                    .flat_map(|t| facts.corners(*t))
                    .filter_map(|v| facts.seam_gap.get(&v).copied())
                    .reduce(f32::max);
                json!({
                    "region": region,
                    "faces": faces.len(),
                    "max_strain_pct": max.map(|s| r1(s * 100.0)),
                    "mean_strain_pct": mean.map(|s| r1(s * 100.0)),
                    "min_body_mm": body.map(|d| r1(d * 1000.0)),
                    "max_seam_gap_mm": gap.map(|g| r1(g * 1000.0)),
                    "listed": list(worst(&faces, limit, |t| Some(facts.shape(t).0))),
                })
            })
            .collect();
        let non_finite = latest.positions.iter().filter(|p| !p.is_finite()).count();
        let inside = facts
            .body
            .as_ref()
            .filter(|_| facts.signed)
            .map(|b| b.iter().filter(|d| **d < -INSIDE_M).count());
        let deepest = if facts.signed {
            worst(&all, limit, |t| {
                facts
                    .body_distance(t)
                    .filter(|d| *d < -INSIDE_M)
                    .map(|d| -d)
            })
        } else {
            Vec::new()
        };
        Ok(json!({
            "drape": latest.state(),
            "replay": {
                "fingerprint": format!("{:016x}", fingerprint(project)?),
                "time_s": r3(latest.time),
                "motion": latest.animation.as_ref().map(|a| a.motion.name().to_owned()),
            },
            "body_signed": facts.body.is_some().then_some(facts.signed),
            "counts": {
                "faces": all.len(),
                "particles": latest.positions.len(),
                "non_finite": non_finite,
                "inside_body": inside,
            },
            "regions": regions,
            "seams": facts.seams(limit),
            "worst": {
                "strained": list(worst(&all, limit, |t| Some(facts.shape(t).0))),
                "crumpled": list(worst(&all, limit, |t| {
                    let ratio = facts.shape(t).1;
                    (ratio < 1.0).then_some(1.0 - ratio)
                })),
                "inside_body": list(deepest),
            },
        }))
    }
}
