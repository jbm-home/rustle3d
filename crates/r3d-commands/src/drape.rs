//! The sim bridge: build a cloth simulation scene from the document.
//!
//! Every placed panel is meshed (`r3d_geom::mesh`) with its boundary
//! sampled edge by edge; the two edges of a seam get the same number of
//! samples, so their vertices pair up one to one. Panels start wrapped
//! around the body (see `start_radius`), and the cloth collides with the
//! fitted avatar and the ground.
//!
//! [`simulate`] runs a drape headless (CLI, agents); its result is cached in
//! the project folder, keyed by a fingerprint of the document, so renders
//! can show it without simulating again.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::Path;
use std::time::Instant;

use glam::{Vec2, Vec3};
use r3d_avatar::fit::{fit, fitted_positions};
use r3d_avatar::measure::girth_at;
use r3d_core::{EdgeId, Fabric, PanelId, PinId, Project};
use r3d_geom::DVec2;
use r3d_geom::mesh::mesh_polygon;
use r3d_sim::collide::StaticMesh;
use r3d_sim::scene::{Particle, Triangle};
use r3d_sim::{FabricPreset, SUBSTEPS, SimScene, Solver, StepStats};

use crate::error::CommandError;
use crate::queries::body_model;

/// Default distance between cloth vertices (mm): interactive on a laptop.
pub const DEFAULT_SPACING_MM: f64 = 25.0;
/// Grid step of the avatar's distance field (m).
const BODY_VOXEL: f32 = 0.005;
/// Body radius assumed when there is no avatar (m).
const DEFAULT_BODY_RADIUS_M: f32 = 0.16;

/// A built simulation scene.
#[derive(Debug, Clone)]
pub struct Drape {
    pub scene: SimScene,
    /// One entry per simulated panel, in particle order.
    pub pieces: Vec<Piece>,
    /// The particles each pin holds.
    pub pins: Vec<(PinId, Vec<usize>)>,
}

/// The particles and triangles of one panel, to draw it on its own.
#[derive(Debug, Clone)]
pub struct Piece {
    pub panel: PanelId,
    pub particles: Range<usize>,
    /// Triangle corners, indices local to `particles`.
    pub indices: Vec<u32>,
}

/// The dress form garments are draped on: the fitted body (m), its
/// profile and its distance field. Building one takes a fraction of a
/// second, so callers keep it for as long as the measurements do not change.
#[derive(Debug, Clone)]
pub struct Form {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    /// Equivalent radius (girth / 2π, m) every `PROFILE_STEP` from the floor.
    profile: Vec<f32>,
    collider: StaticMesh,
}

/// Height step of the body profile (m).
const PROFILE_STEP: f32 = 0.01;

impl Form {
    pub fn new(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        let height = positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let steps = (height / PROFILE_STEP).ceil() as usize;
        let profile = (0..=steps)
            .map(|k| {
                girth_at(&positions, &triangles, k as f32 * PROFILE_STEP, 0.0)
                    .map_or(0.0, |cm| cm / 100.0 / std::f32::consts::TAU)
            })
            .collect();
        let collider = StaticMesh::new(positions.clone(), triangles.clone(), BODY_VOXEL);
        Self {
            positions,
            triangles,
            profile,
            collider,
        }
    }

    /// The project's avatar fitted to its measurements, if it has one.
    pub fn of_project(project: &Project) -> Result<Option<Self>, CommandError> {
        let Some(avatar) = project.avatar() else {
            return Ok(None);
        };
        let model = body_model()?;
        let positions = fitted_positions(model, &fit(model, &avatar.body));
        Ok(Some(Self::new(positions, model.triangles().to_vec())))
    }
}

/// The widest body section between heights `top` and `top - below` (m):
/// a garment hangs outside everything above it.
fn body_radius(profile: &[f32], top: f32, below: f32) -> f32 {
    let last = profile.len().saturating_sub(1);
    let index = |y: f32| ((y / PROFILE_STEP).round().max(0.0) as usize).min(last);
    profile
        .get(index(top - below)..=index(top))
        .map_or(0.0, |s| s.iter().copied().fold(0.0, f32::max))
}

/// Horizontal span of a closed polygon at height `y` (same units).
fn span_at(polygon: &[DVec2], y: f64) -> f64 {
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        if (a.y - y) * (b.y - y) <= 0.0 && a.y != b.y {
            let x = a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y);
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    if hi >= lo { hi - lo } else { 0.0 }
}

/// A placed panel, meshed, before it is positioned in 3D.
struct Laid {
    panel: PanelId,
    place: r3d_core::Placement,
    boundary: Vec<DVec2>,
    /// Index in `boundary` of each contour edge's first sample.
    starts: Vec<usize>,
    mesh: r3d_geom::mesh::PanelMesh,
    /// Pattern height of the panel's top (mm).
    top: f64,
}

impl Laid {
    /// Width of the panel at world height `y` (m).
    fn width_at(&self, y: f32) -> f32 {
        let below_mm = f64::from(self.place.top_cm as f32 / 100.0 - y) * 1000.0;
        (span_at(&self.boundary, self.top - below_mm) / 1000.0) as f32
    }
}

pub fn fabric_preset(f: Fabric) -> FabricPreset {
    match f {
        Fabric::Cotton => FabricPreset::Cotton,
        Fabric::Denim => FabricPreset::Denim,
        Fabric::Jersey => FabricPreset::Jersey,
        Fabric::Silk => FabricPreset::Silk,
        Fabric::Wool => FabricPreset::Wool,
    }
}

fn internal(detail: impl Into<String>) -> CommandError {
    CommandError::Internal {
        detail: detail.into(),
    }
}

/// Number of boundary segments for each edge of the placed panels.
fn segment_counts(project: &Project, spacing: f64) -> BTreeMap<EdgeId, usize> {
    let count = |len: f64| ((len / spacing).ceil() as usize).max(1);
    let mut counts = BTreeMap::new();
    for panel in project.panels().values().filter(|p| p.placement.is_some()) {
        for e in &panel.contour {
            counts.insert(*e, count(project.edge_length(*e).unwrap_or_default()));
        }
    }
    // Both sides of a seam share the larger count.
    for seam in project.seams().values() {
        if let (Some(a), Some(b)) = (counts.get(&seam.a.edge), counts.get(&seam.b.edge)) {
            let n = (*a).max(*b);
            counts.insert(seam.a.edge, n);
            counts.insert(seam.b.edge, n);
        }
    }
    counts
}

/// Build the scene. Panels without a placement are left out.
pub fn build(
    project: &Project,
    form: Option<&Form>,
    spacing_mm: f64,
) -> Result<Drape, CommandError> {
    let spacing = spacing_mm.max(2.0);
    let counts = segment_counts(project, spacing);
    let garment = project
        .default_garment()
        .and_then(|g| project.garments().get(&g));
    let fabric = fabric_preset(garment.map(|g| g.fabric).unwrap_or_default()).params();
    let default_profile = [DEFAULT_BODY_RADIUS_M];
    let profile = form.map_or(&default_profile[..], |f| &f.profile[..]);

    let mut scene = SimScene {
        fabrics: vec![fabric],
        ground: true,
        ..SimScene::default()
    };
    let mut pieces = Vec::new();
    // Particle indices along each edge, start to end inclusive.
    let mut edge_vertices: BTreeMap<EdgeId, Vec<u32>> = BTreeMap::new();

    // Mesh every placed panel first: the starting radius depends on the
    // width of all of them.
    let mut laid = Vec::new();
    for (panel_id, panel) in project.panels() {
        let Some(place) = panel.placement else {
            continue;
        };
        let mut boundary: Vec<DVec2> = Vec::new();
        let mut starts = Vec::with_capacity(panel.contour.len());
        for e in &panel.contour {
            let geom = project
                .edge_geometry(*e)
                .ok_or_else(|| internal(format!("{e} has no geometry")))?;
            let len = geom.length();
            let n = counts.get(e).copied().unwrap_or(1);
            starts.push(boundary.len());
            for k in 0..n {
                boundary.push(geom.point_at_length(len * k as f64 / n as f64).0);
            }
        }
        let mesh =
            mesh_polygon(&boundary, spacing).map_err(|e| internal(format!("{panel_id}: {e}")))?;
        let top = mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.y));
        laid.push(Laid {
            panel: *panel_id,
            place,
            boundary,
            starts,
            mesh,
            top,
        });
    }
    // The garment's circumference at a height: all its panels side by side.
    let around = |y: f32| laid.iter().map(|l| l.width_at(y)).sum::<f32>();

    for l in &laid {
        let Some(panel) = project.panel(l.panel) else {
            continue;
        };
        let mesh = &l.mesh;
        let offset = scene.particles.len() as u32;
        let total = l.boundary.len();
        for (e, start) in panel.contour.iter().zip(&l.starts) {
            let n = counts.get(e).copied().unwrap_or(1);
            let verts = (0..=n)
                .filter_map(|k| mesh.boundary.get((start + k) % total))
                .map(|b| offset + b)
                .collect();
            edge_vertices.insert(*e, verts);
        }

        // Wrap around the body: the panel's horizontal centre goes to the
        // placement angle, its top to the placement height; distances along
        // the pattern are kept as arc lengths. At each height the radius is
        // the larger of the body's (plus the gap) and the garment's own
        // (circumference / 2π), so the panels start just touching.
        let lo = mesh.vertices.iter().fold(f64::MAX, |m, v| m.min(v.x));
        let hi = mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.x));
        let centre = (lo + hi) / 2.0;
        let gap = (l.place.offset_mm / 1000.0) as f32;
        let top_y = l.place.top_cm as f32 / 100.0;
        let angle0 = (l.place.angle_deg as f32).to_radians();
        for v in &mesh.vertices {
            let below = ((l.top - v.y) / 1000.0) as f32;
            let y = top_y - below;
            let r = (body_radius(profile, top_y, below) + gap)
                .max(around(y) / std::f32::consts::TAU)
                .max(1e-3);
            let theta = angle0 + ((v.x - centre) / 1000.0) as f32 / r;
            scene.particles.push(Particle {
                position: Vec3::new(r * theta.sin(), y, r * theta.cos()),
                rest: Vec2::new((v.x / 1000.0) as f32, (v.y / 1000.0) as f32),
                pinned: false,
            });
        }
        pieces.push(Piece {
            panel: l.panel,
            particles: offset as usize..scene.particles.len(),
            indices: mesh.triangles.iter().flatten().copied().collect(),
        });
        let grain = (panel.grain_angle_deg as f32).to_radians();
        let warp = Vec2::new(grain.cos(), grain.sin());
        scene
            .triangles
            .extend(mesh.triangles.iter().map(|t| Triangle {
                vertices: t.map(|i| offset + i),
                fabric: 0,
                warp,
            }));
    }

    for seam in project.seams().values() {
        let side = |s: &r3d_core::SeamSide| {
            edge_vertices.get(&s.edge).map(|v| {
                let mut v = v.clone();
                if s.reversed {
                    v.reverse();
                }
                v
            })
        };
        if let (Some(a), Some(b)) = (side(&seam.a), side(&seam.b)) {
            scene.seams.extend(
                a.iter()
                    .zip(&b)
                    .filter(|(x, y)| x != y)
                    .map(|(x, y)| [*x, *y]),
            );
        }
    }

    scene.collider = form.map(|f| f.collider.clone());
    // Pins: the cloth around each pin starts at the pin's place, keeping
    // its local shape, and does not move.
    let mut pins = Vec::new();
    let rest: Vec<Vec2> = scene.particles.iter().map(|p| p.rest).collect();
    for (id, pin) in project.pins() {
        let Some(piece) = pieces.iter().find(|p| p.panel == pin.panel) else {
            continue;
        };
        let held = pinned_by(piece, &rest, (pin.at / 1000.0).as_vec2());
        let Some(&center) = held.first() else {
            continue;
        };
        let origin = scene.particles[center].position;
        for i in &held {
            let p = &mut scene.particles[*i];
            p.position = pin.position.as_vec3() + (p.position - origin);
            p.pinned = true;
        }
        pins.push((*id, held));
    }
    Ok(Drape {
        scene,
        pieces,
        pins,
    })
}

/// Cloth within this distance of a pin (on the pattern) is held (m).
pub const PIN_RADIUS_M: f32 = 0.03;

/// The particles of `piece` a pin at `at` (pattern, m) holds: those within
/// [`PIN_RADIUS_M`] on the pattern, the closest first. `rest` is every
/// particle's pattern position.
pub fn pinned_by(piece: &Piece, rest: &[Vec2], at: Vec2) -> Vec<usize> {
    let mut near: Vec<(f32, usize)> = piece
        .particles
        .clone()
        .filter_map(|i| Some((rest.get(i)?.distance(at), i)))
        .filter(|(d, _)| *d < PIN_RADIUS_M)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.into_iter().map(|(_, i)| i).collect()
}

/// Build the scene for the project as it is, at the default spacing.
pub fn prepare(project: &Project, form: Option<&Form>) -> Result<Drape, CommandError> {
    build(project, form, DEFAULT_SPACING_MM)
}

/// Start `new` where `old` is now (`positions`): each particle takes the
/// position of the same pattern point in the old drape, interpolated in
/// the old triangle that contains it, or extrapolated from the nearest one
/// when the panel grew. Particles of new panels, and pinned ones, keep
/// their own start. This is how pattern edits reach a running simulation
/// without a jump (architecture §7.8).
pub fn carry_over(old: &Drape, positions: &[Vec3], new: &mut Drape) {
    let rest_old: Vec<Vec2> = old.scene.particles.iter().map(|p| p.rest).collect();
    let rest = |k: usize| rest_old.get(k).copied();
    for piece in &new.pieces {
        let Some(before) = old.pieces.iter().find(|p| p.panel == piece.panel) else {
            continue;
        };
        let corners: Vec<[usize; 3]> = before
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.map(|i| before.particles.start + i as usize))
            .collect();
        // Old triangles binned by their pattern bounding box, so a point
        // inside the old shape only tests a few.
        let cell = |p: Vec2| (p / CARRY_CELL_M).floor().as_ivec2().to_array();
        let mut grid: HashMap<[i32; 2], Vec<usize>> = HashMap::new();
        for (k, t) in corners.iter().enumerate() {
            let pts: Vec<Vec2> = t.iter().filter_map(|i| rest(*i)).collect();
            let lo = pts.iter().fold(Vec2::MAX, |a, b| a.min(*b));
            let hi = pts.iter().fold(Vec2::MIN, |a, b| a.max(*b));
            let (a, b) = (cell(lo), cell(hi));
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    grid.entry([x, y]).or_default().push(k);
                }
            }
        }
        let weights = |p: Vec2, t: &[usize; 3]| {
            let [a, b, c] = t.map(rest);
            barycentric(p, a?, b?, c?)
        };
        // The triangle whose smallest weight is largest: the one containing
        // the point, or the closest one outside.
        let best_of = |p: Vec2, candidates: &mut dyn Iterator<Item = &[usize; 3]>| {
            candidates
                .filter_map(|t| Some((weights(p, t)?, *t)))
                .max_by(|x, y| x.0.min_element().total_cmp(&y.0.min_element()))
        };
        for i in piece.particles.clone() {
            let Some(p) = new.scene.particles.get_mut(i) else {
                continue;
            };
            if p.pinned {
                continue;
            }
            let near = grid
                .get(&cell(p.rest))
                .map(Vec::as_slice)
                .unwrap_or_default();
            let inside = best_of(p.rest, &mut near.iter().filter_map(|k| corners.get(*k)))
                .filter(|(w, _)| w.min_element() >= -1e-4);
            let best = inside.or_else(|| best_of(p.rest, &mut corners.iter()));
            if let Some((w, t)) = best
                && let [Some(a), Some(b), Some(c)] = t.map(|k| positions.get(k).copied())
            {
                p.position = a * w.x + b * w.y + c * w.z;
            }
        }
    }
}

/// Bin size of the carry-over grid on the pattern (m).
const CARRY_CELL_M: f32 = 0.05;

/// Barycentric weights of `p` in triangle `abc` (2D); `None` if it is flat.
fn barycentric(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> Option<Vec3> {
    let (v0, v1, v2) = (b - a, c - a, p - a);
    let det = v0.perp_dot(v1);
    if det.abs() < 1e-12 {
        return None;
    }
    let v = v2.perp_dot(v1) / det;
    let w = v0.perp_dot(v2) / det;
    Some(Vec3::new(1.0 - v - w, v, w))
}

/// What a headless drape produced.
#[derive(Debug, Clone)]
pub struct Simulated {
    pub drape: Drape,
    pub form: Option<Form>,
    pub positions: Vec<Vec3>,
    pub frames: u32,
    pub ms_per_frame: f64,
    pub stats: StepStats,
}

/// Simulate `frames` frames of 1/60 s from the starting positions.
pub fn simulate(project: &Project, frames: u32) -> Result<Simulated, CommandError> {
    let form = Form::of_project(project)?;
    let drape = prepare(project, form.as_ref())?;
    if drape.pieces.is_empty() {
        return Err(CommandError::NothingToSimulate);
    }
    let mut solver = Solver::new(drape.scene.clone());
    let start = Instant::now();
    for _ in 0..frames {
        solver.step(1.0 / 60.0);
    }
    let ms_per_frame = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames.max(1));
    Ok(Simulated {
        positions: solver.positions().to_vec(),
        stats: solver.stats(),
        drape,
        form,
        frames,
        ms_per_frame,
    })
}

/// Where the last headless drape is cached, relative to the project folder.
pub const CACHE_FILE: &str = ".r3d-cache/drape.bin";
const CACHE_MAGIC: &[u8; 8] = b"R3DDRP01";

/// Fingerprint of everything a drape depends on: the document (schema
/// JSON), the mesh spacing and the solver's substeps. FNV-1a, stable
/// across runs and platforms.
pub fn fingerprint(project: &Project) -> Result<u64, CommandError> {
    let json = r3d_core::file::to_json(project).map_err(|e| internal(e.to_string()))?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let tail = [
        DEFAULT_SPACING_MM.to_le_bytes(),
        u64::from(SUBSTEPS).to_le_bytes(),
    ];
    for b in json.as_bytes().iter().chain(tail.iter().flatten()) {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    Ok(hash)
}

/// Encode a drape for [`CACHE_FILE`].
pub fn encode_cache(fingerprint: u64, positions: &[Vec3]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16 + positions.len() * 12);
    bytes.extend_from_slice(CACHE_MAGIC);
    bytes.extend_from_slice(&fingerprint.to_le_bytes());
    for p in positions {
        for c in p.to_array() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
    }
    bytes
}

/// The cached drape positions in the project folder `dir`, if they were
/// made from this exact document; `None` otherwise (stale, missing or
/// damaged cache: it is only a cache).
pub fn load_cache(dir: &Path, project: &Project, particles: usize) -> Option<Vec<Vec3>> {
    let bytes = std::fs::read(dir.join(CACHE_FILE)).ok()?;
    let (head, body) = bytes.split_at_checked(16)?;
    let (magic, print) = head.split_at(8);
    if magic != CACHE_MAGIC
        || print != fingerprint(project).ok()?.to_le_bytes()
        || body.len() != particles * 12
    {
        return None;
    }
    Some(
        body.as_chunks::<12>()
            .0
            .iter()
            .map(|c| {
                let (xyz, _) = c.as_chunks::<4>();
                let [x, y, z] =
                    [0, 1, 2].map(|i| xyz.get(i).map_or(0.0, |b| f32::from_le_bytes(*b)));
                Vec3::new(x, y, z)
            })
            .collect(),
    )
}
