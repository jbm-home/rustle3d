//! Bodies imported from a file (spec 27): read once and kept by their
//! fingerprint, turned into the avatar's frame (metres, Y up, facing +Z,
//! feet on the floor), checked, measured, given arrangement points, and
//! made into the dress form the drape uses.
//!
//! A project names its body's file and fingerprint; the file is read
//! when the project folder is opened ([`load`]) or the body imported
//! ([`register`]), so commands and queries, which see only the document,
//! find it here by fingerprint.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use glam::{DVec3, Vec2, Vec3};
use r3d_core::imported::{fingerprint, model_path_ok};
use r3d_core::{AvatarMesh, Body, BodyPoint, Project, UpAxis};
use r3d_geom::trimesh::{self, Oriented, Topology};
use r3d_geom::winding::Winding;
use r3d_io::model::{Model, ModelError, ModelFormat};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::commands::{BUST_RANGE, HEIGHT_RANGE, HIPS_RANGE, WAIST_RANGE};
use crate::drape::{Form, Parts};
use crate::error::CommandError;

/// A model file as read.
#[derive(Debug)]
pub struct Loaded {
    /// Its fingerprint ([`r3d_core::imported::fingerprint`]).
    pub hash: String,
    pub model: Model,
    pub format: ModelFormat,
}

/// How many files, placed bodies and dress forms are kept.
const KEPT: usize = 4;

static LOADED: Mutex<Vec<Arc<Loaded>>> = Mutex::new(Vec::new());
static PLACED: Mutex<Vec<(String, Orientation, Arc<Placed>)>> = Mutex::new(Vec::new());
static FORMS: Mutex<Vec<(AvatarMesh, Arc<Form>)>> = Mutex::new(Vec::new());

fn lock<T>(m: &Mutex<T>) -> Result<MutexGuard<'_, T>, CommandError> {
    m.lock().map_err(|_| CommandError::Internal {
        detail: "imported body cache lock poisoned".into(),
    })
}

/// Keep `item`, dropping the oldest beyond [`KEPT`].
fn keep<T>(list: &mut Vec<T>, item: T) {
    if list.len() >= KEPT {
        list.remove(0);
    }
    list.push(item);
}

/// Read a model file and keep it by its fingerprint.
pub fn register(bytes: &[u8], format: ModelFormat) -> Result<Arc<Loaded>, ModelError> {
    let hash = fingerprint(bytes);
    if let Some(l) = loaded(&hash) {
        return Ok(l);
    }
    let model = r3d_io::model::read(bytes, format)?;
    let l = Arc::new(Loaded {
        hash,
        model,
        format,
    });
    if let Ok(mut list) = LOADED.lock() {
        keep(&mut list, l.clone());
    }
    Ok(l)
}

/// The model file with this fingerprint, if it was read.
pub fn loaded(hash: &str) -> Option<Arc<Loaded>> {
    LOADED.lock().ok()?.iter().find(|l| l.hash == hash).cloned()
}

/// Read the imported body of `project` from its folder `dir`, if it has
/// one and the file is still the one imported. A missing or changed file
/// is left to the check (`file.unavailable`).
pub fn load(dir: &Path, project: &Project) {
    let Some(mesh) = project.avatar().and_then(|a| a.mesh.as_ref()) else {
        return;
    };
    if loaded(&mesh.hash).is_some() || !model_path_ok(&mesh.file) {
        return;
    }
    let Some(format) = ModelFormat::of_name(&mesh.file) else {
        return;
    };
    if let Ok(bytes) = std::fs::read(dir.join(&mesh.file))
        && fingerprint(&bytes) == mesh.hash
    {
        register(&bytes, format).ok();
    }
}

/// How a model's coordinates become the avatar's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orientation {
    /// Metres per file unit.
    pub scale: f64,
    pub up: UpAxis,
    /// Degrees about the vertical (right-handed), after the up axis.
    pub turn_deg: f64,
}

impl Orientation {
    pub fn of(mesh: &AvatarMesh) -> Self {
        Self {
            scale: mesh.scale,
            up: mesh.up,
            turn_deg: mesh.turn_deg,
        }
    }
}

/// A file point with `up` turned to +Y (a rotation, never a mirror).
fn upright(p: [f32; 3], up: UpAxis) -> DVec3 {
    let [x, y, z] = p.map(f64::from);
    match up {
        UpAxis::Y => DVec3::new(x, y, z),
        UpAxis::Z => DVec3::new(x, z, -y),
        UpAxis::X => DVec3::new(-y, x, z),
        UpAxis::NegY => DVec3::new(x, -y, -z),
        UpAxis::NegZ => DVec3::new(x, -z, y),
        UpAxis::NegX => DVec3::new(y, -x, z),
    }
}

/// `v` turned by `deg` about +Y.
fn turned(v: DVec3, deg: f64) -> DVec3 {
    let (s, c) = deg.to_radians().sin_cos();
    DVec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

/// A body in the avatar's frame, ready to check and drape.
#[derive(Debug)]
pub struct Placed {
    /// Metres, Y up, facing +Z, feet at y = 0, centred on its vertical
    /// axis.
    pub positions: Vec<Vec3>,
    /// Facing outward, without flat triangles.
    pub triangles: Vec<[u32; 3]>,
    /// As in the file.
    pub topology: Topology,
    /// What turning the triangles outward changed.
    pub oriented: Oriented,
    pub height: f32,
    /// The file has a skeleton (not used).
    pub skeleton: bool,
}

/// The hips' band (fractions of the height), where the body's front and
/// back are centred.
const HIPS_BAND: (f32, f32) = (0.47, 0.56);

fn place(l: &Loaded, o: Orientation) -> Placed {
    let mut positions: Vec<Vec3> = l
        .model
        .positions
        .iter()
        .map(|p| turned(upright(*p, o.up) * o.scale, o.turn_deg).as_vec3())
        .collect();
    let (lo, hi) = bounds(positions.iter());
    let height = (hi.y - lo.y).max(0.0);
    let hips = positions.iter().filter(|p| {
        let f = (p.y - lo.y) / height.max(1e-6);
        f >= HIPS_BAND.0 && f <= HIPS_BAND.1
    });
    let (hips_lo, hips_hi) = bounds(hips);
    let z_mid = if hips_lo.z <= hips_hi.z {
        (hips_lo.z + hips_hi.z) / 2.0
    } else {
        (lo.z + hi.z) / 2.0
    };
    let shift = Vec3::new((lo.x + hi.x) / 2.0, lo.y, z_mid);
    for p in &mut positions {
        *p -= shift;
    }
    let mut triangles = l.model.triangles.clone();
    let topology = trimesh::topology(&positions, &triangles);
    let oriented = trimesh::orient(&positions, &mut triangles);
    let triangles = trimesh::without_degenerate(&positions, &triangles);
    Placed {
        positions,
        triangles,
        topology,
        oriented,
        height,
        skeleton: l.model.skeleton,
    }
}

fn bounds<'a>(points: impl Iterator<Item = &'a Vec3>) -> (Vec3, Vec3) {
    points.fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
        (lo.min(*p), hi.max(*p))
    })
}

/// The model with fingerprint `hash` placed as `o` says (kept).
fn placed_as(hash: &str, file: &str, o: Orientation) -> Result<Arc<Placed>, CommandError> {
    if let Some((_, _, p)) = lock(&PLACED)?.iter().find(|(h, k, _)| h == hash && *k == o) {
        return Ok(p.clone());
    }
    let l = loaded(hash).ok_or_else(|| CommandError::BodyUnavailable {
        file: file.to_owned(),
    })?;
    let p = Arc::new(place(&l, o));
    keep(&mut *lock(&PLACED)?, (hash.to_owned(), o, p.clone()));
    Ok(p)
}

/// The imported body of `mesh`, placed.
pub fn placed(mesh: &AvatarMesh) -> Result<Arc<Placed>, CommandError> {
    placed_as(&mesh.hash, &mesh.file, Orientation::of(mesh))
}

/// Units a model may be in, as metres per unit: metres, centimetres,
/// millimetres, inches.
const UNITS: [f64; 4] = [1.0, 0.01, 0.001, 0.0254];
/// Heights a body is taken to have when its units are guessed (m).
const PLAUSIBLE_M: (f64, f64) = (1.0, 2.3);
/// The usual height, when none of the units is plausible (m).
const USUAL_M: f64 = 1.7;
/// An axis this close to the longest is taken as up first (a T pose is
/// as wide as it is tall).
const UP_TIE: f64 = 0.9;
/// The ends of the up axis compared for the feet (fraction of length).
const END_SLAB: f64 = 0.05;
/// A turn within this of a quarter turn is snapped to it (degrees).
const SNAP_DEG: f64 = 10.0;

/// Guess the units, the up axis and the facing of a model (spec 27).
pub fn detect(l: &Loaded) -> Orientation {
    let points: Vec<DVec3> = l
        .model
        .positions
        .iter()
        .map(|p| DVec3::from(p.map(f64::from)))
        .collect();
    let (lo, hi) = points.iter().fold((DVec3::MAX, DVec3::MIN), |(lo, hi), p| {
        (lo.min(*p), hi.max(*p))
    });
    let extent = (hi - lo).max(DVec3::ZERO);
    let longest = extent.max_element();
    // glTF is Y-up by its specification; otherwise prefer Y, then Z, then
    // X among the axes about as long as the longest.
    let axis = if matches!(l.format, ModelFormat::Gltf | ModelFormat::Glb) {
        1
    } else {
        [1, 2, 0]
            .into_iter()
            .find(|i| extent[*i] >= UP_TIE * longest)
            .unwrap_or(1)
    };
    // The floor is the end with the larger footprint (two feet, not a
    // head).
    let footprint = |at_top: bool| {
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        let slab = points.iter().filter(|p| {
            let d = if at_top {
                hi[axis] - p[axis]
            } else {
                p[axis] - lo[axis]
            };
            d <= END_SLAB * extent[axis]
        });
        let (s_lo, s_hi) = slab.fold((DVec3::MAX, DVec3::MIN), |(lo, hi), p| {
            (lo.min(*p), hi.max(*p))
        });
        ((s_hi[a] - s_lo[a]) * (s_hi[b] - s_lo[b])).max(0.0)
    };
    let down_is_top = footprint(true) > footprint(false);
    let up = match (axis, down_is_top) {
        (2, false) => UpAxis::Z,
        (2, true) => UpAxis::NegZ,
        (0, false) => UpAxis::X,
        (0, true) => UpAxis::NegX,
        (_, false) => UpAxis::Y,
        (_, true) => UpAxis::NegY,
    };
    let height = extent[axis];
    let scale = UNITS
        .into_iter()
        .find(|u| (PLAUSIBLE_M.0..=PLAUSIBLE_M.1).contains(&(height * u)))
        .unwrap_or_else(|| {
            UNITS
                .into_iter()
                .min_by(|a, b| {
                    let off = |u: f64| (height * u / USUAL_M).ln().abs();
                    off(*a).total_cmp(&off(*b))
                })
                .unwrap_or(1.0)
        });
    let o = Orientation {
        scale,
        up,
        turn_deg: 0.0,
    };
    let body = place(l, o);
    let turn_deg = facing(&body.positions, body.height)
        .map_or(0.0, |f| snap(f64::from((-f.x).atan2(f.y)).to_degrees()));
    Orientation { turn_deg, ..o }
}

/// A turn in degrees, in (−180, 180], snapped to a quarter turn when
/// close to one.
fn snap(deg: f64) -> f64 {
    let quarter = (deg / 90.0).round() * 90.0;
    let d = if (deg - quarter).abs() <= SNAP_DEG {
        quarter
    } else {
        deg.round()
    };
    let d = d.rem_euclid(360.0);
    if d > 180.0 { d - 360.0 } else { d }
}

/// The direction the body faces on the floor (x, z), if it can be told:
/// across the line of the shoulders, on the side the feet point to.
fn facing(positions: &[Vec3], height: f32) -> Option<Vec2> {
    let slab = |from: f32, to: f32| -> Vec<Vec2> {
        positions
            .iter()
            .filter(|p| p.y >= from * height && p.y <= to * height)
            .map(|p| Vec2::new(p.x, p.z))
            .collect()
    };
    let mean =
        |v: &[Vec2]| (!v.is_empty()).then(|| v.iter().copied().sum::<Vec2>() / v.len() as f32);
    let shoulders = slab(0.76, 0.84);
    let c = mean(&shoulders)?;
    let (mut xx, mut xz, mut zz) = (0.0, 0.0, 0.0);
    for p in &shoulders {
        let d = *p - c;
        xx += d.x * d.x;
        xz += d.x * d.y;
        zz += d.y * d.y;
    }
    let phi = 0.5 * (2.0 * xz).atan2(xx - zz);
    let across = Vec2::new(-phi.sin(), phi.cos());
    let toes = match (mean(&slab(0.0, 0.04)), mean(&slab(0.06, 0.10))) {
        (Some(feet), Some(ankles)) => feet - ankles,
        _ => Vec2::Y,
    };
    let dot = across.dot(toes);
    Some(if dot < 0.0 || (dot == 0.0 && across.y < 0.0) {
        -across
    } else {
        across
    })
}

/// The heights (m) of the chest (over the bust points), the waist (the
/// narrowest girth) and the hips (the largest), as the built-in body is
/// measured.
fn levels_of(p: &Placed) -> [f32; 3] {
    use r3d_avatar::measure::{Landmarks, heights, measure};
    let landmarks = Landmarks::find(&p.positions);
    let chest = landmarks
        .bust
        .iter()
        .filter_map(|i| p.positions.get(*i as usize))
        .map(|v| v.y)
        .sum::<f32>()
        / 2.0;
    let waist = measure(&p.positions, &p.triangles, &landmarks).waist_height_cm / 100.0;
    [chest, waist, heights(&p.positions, &p.triangles).hips]
}

/// The middle of the torso at height `y` (x on the axis).
fn torso_middle(p: &Placed, y: f32) -> Vec3 {
    let band = p
        .positions
        .iter()
        .filter(|v| (v.y - y).abs() < TORSO_SLICE_M && v.x.abs() < TORSO_HALF_M);
    let (lo, hi) = bounds(band);
    let z = if lo.z <= hi.z {
        (lo.z + hi.z) / 2.0
    } else {
        0.0
    };
    Vec3::new(0.0, y, z)
}

/// Half the height of the slice the torso's middle is found in (m).
const TORSO_SLICE_M: f32 = 0.01;
/// Points this far from the axis are the torso, not the arms (m).
const TORSO_HALF_M: f32 = 0.12;
/// The crotch point is this far above the lowest point between the legs,
/// inside the body (m).
const CROTCH_INSIDE_M: f32 = 0.01;

/// Guess the arrangement points of a placed body (mm).
pub fn guess_points(p: &Placed) -> BTreeMap<BodyPoint, [f64; 3]> {
    let parts = Parts::find(&p.positions);
    let [chest, waist, hips] = levels_of(p);
    // Just above where the legs part (as the built-in body is measured),
    // at the depth of the body there.
    let crotch_y = r3d_avatar::measure::heights(&p.positions, &p.triangles)
        .crotch
        .unwrap_or(parts.crotch);
    let low: Vec<f32> = p
        .positions
        .iter()
        .filter(|v| v.x.abs() < 0.01 && (v.y - crotch_y).abs() < 0.01)
        .map(|v| v.z)
        .collect();
    let z = if low.is_empty() {
        torso_middle(p, crotch_y).z
    } else {
        low.iter().sum::<f32>() / low.len() as f32
    };
    let crotch = Vec3::new(0.0, crotch_y + CROTCH_INSIDE_M, z);
    let mm = |v: Vec3| {
        v.as_dvec3()
            .to_array()
            .map(|c| (c * 10_000.0).round() / 10.0)
    };
    let [[ls, le, lw], [rs, re, rw]] = parts.arms;
    let [[lt, lsh], [rt, rsh]] = parts.legs;
    [
        (BodyPoint::Chest, torso_middle(p, chest)),
        (BodyPoint::Waist, torso_middle(p, waist)),
        (BodyPoint::Hips, torso_middle(p, hips)),
        (BodyPoint::Crotch, crotch),
        (BodyPoint::LeftShoulder, ls),
        (BodyPoint::LeftElbow, le),
        (BodyPoint::LeftWrist, lw),
        (BodyPoint::RightShoulder, rs),
        (BodyPoint::RightElbow, re),
        (BodyPoint::RightWrist, rw),
        (BodyPoint::LeftThigh, lt),
        (BodyPoint::LeftShin, lsh),
        (BodyPoint::RightThigh, rt),
        (BodyPoint::RightShin, rsh),
    ]
    .into_iter()
    .map(|(k, v)| (k, mm(v)))
    .collect()
}

/// The parts and levels the points give (m); a point missing from the
/// file is guessed.
fn parts_of(p: &Placed, points: &BTreeMap<BodyPoint, [f64; 3]>) -> (Parts, [f32; 3]) {
    let guessed = guess_points(p);
    let at = |k: BodyPoint| {
        let v = points
            .get(&k)
            .or_else(|| guessed.get(&k))
            .copied()
            .unwrap_or_default();
        (DVec3::from(v) / 1000.0).as_vec3()
    };
    use BodyPoint as B;
    let parts = Parts {
        arms: [
            [at(B::LeftShoulder), at(B::LeftElbow), at(B::LeftWrist)],
            [at(B::RightShoulder), at(B::RightElbow), at(B::RightWrist)],
        ],
        legs: [
            [at(B::LeftThigh), at(B::LeftShin)],
            [at(B::RightThigh), at(B::RightShin)],
        ],
        crotch: at(B::Crotch).y - CROTCH_INSIDE_M,
    };
    (parts, [at(B::Chest).y, at(B::Waist).y, at(B::Hips).y])
}

/// Waist-to-hips ratios read as fully feminine and fully masculine.
const WHR: (f64, f64) = (0.72, 0.90);

/// The measurements of a placed body at its points (cm), and those that
/// had to be brought into the accepted ranges.
pub fn measure(p: &Placed, points: &BTreeMap<BodyPoint, [f64; 3]>) -> (Body, Vec<&'static str>) {
    let (_, [chest, waist, hips]) = parts_of(p, points);
    let girth = |y: f32| {
        f64::from(r3d_avatar::measure::girth_at(&p.positions, &p.triangles, y, 0.0).unwrap_or(0.0))
    };
    let mut clamped = Vec::new();
    let mut fit = |name: &'static str, v: f64, (lo, hi): (f64, f64)| {
        let r = (v * 10.0).round() / 10.0;
        if (lo..=hi).contains(&r) {
            r
        } else {
            clamped.push(name);
            r.clamp(lo, hi)
        }
    };
    let height_cm = fit("height_cm", f64::from(p.height) * 100.0, HEIGHT_RANGE);
    let bust_cm = fit("bust_cm", girth(chest), BUST_RANGE);
    let waist_cm = fit("waist_cm", girth(waist), WAIST_RANGE);
    let hips_cm = fit("hips_cm", girth(hips), HIPS_RANGE);
    let masculinity = ((waist_cm / hips_cm - WHR.0) / (WHR.1 - WHR.0)).clamp(0.0, 1.0);
    let body = Body {
        masculinity: (masculinity * 100.0).round() / 100.0,
        height_cm,
        bust_cm,
        waist_cm,
        hips_cm,
    };
    (body, clamped)
}

/// The dress form of an imported body (kept: building its distance field
/// takes a fraction of a second, more for a large scan).
pub fn form(mesh: &AvatarMesh) -> Result<Arc<Form>, CommandError> {
    if let Some((_, f)) = lock(&FORMS)?.iter().find(|(m, _)| m == mesh) {
        return Ok(f.clone());
    }
    let p = placed(mesh)?;
    let (parts, levels) = parts_of(&p, &mesh.points);
    let f = Arc::new(Form::imported(
        p.positions.clone(),
        p.triangles.clone(),
        &parts,
        levels,
    ));
    keep(&mut *lock(&FORMS)?, (mesh.clone(), f.clone()));
    Ok(f)
}

/// How much an item of the check matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The body cannot be draped.
    Error,
    /// It drapes, but check it.
    Warning,
    /// Fixed or noted.
    Info,
}

/// One finding of the check: a stable code and its arguments.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    pub code: &'static str,
    pub severity: Severity,
    pub args: BTreeMap<&'static str, Value>,
}

/// An arrangement point and where it lies.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PointCheck {
    pub point: BodyPoint,
    pub at_mm: [f64; 3],
    /// Distance to the surface (mm): negative inside.
    pub distance_mm: f64,
}

/// The check of an imported body (spec 27).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub file: String,
    /// No item is an error: the body can be draped.
    pub ready: bool,
    pub triangles: usize,
    pub vertices: usize,
    pub height_m: f64,
    pub scale: f64,
    pub up: UpAxis,
    pub turn_deg: f64,
    pub skeleton: bool,
    pub items: Vec<Item>,
    pub points: Vec<PointCheck>,
}

impl Report {
    /// The warnings' codes.
    pub fn warnings(&self) -> Vec<&'static str> {
        self.items
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .map(|i| i.code)
            .collect()
    }
}

/// More triangles than this make a body slow (warning).
const HEAVY: usize = 300_000;
/// Fewer than this make it coarse (warning).
const COARSE: usize = 2_000;
/// Heights a body is checked against (m).
const HEIGHT_OK_M: (f64, f64) = (1.2, 2.2);
/// A point farther out than this is outside the body (mm).
const OUTSIDE_MM: f64 = 5.0;

fn item(code: &'static str, severity: Severity, args: &[(&'static str, Value)]) -> Item {
    Item {
        code,
        severity,
        args: args.iter().cloned().collect(),
    }
}

/// Signed distance (mm) from `q` to the surface: negative inside.
fn signed_distance(p: &Placed, winding: &Winding, q: Vec3) -> f64 {
    let d = p
        .triangles
        .iter()
        .filter_map(|t| {
            let [a, b, c] = t.map(|v| p.positions.get(v as usize).copied());
            let (closest, _) = r3d_sim::collide::closest_feature(q, a?, b?, c?);
            Some(closest.distance(q))
        })
        .fold(f32::MAX, f32::min);
    let sign = if winding.inside(q) { -1.0 } else { 1.0 };
    (sign * f64::from(d) * 1000.0 * 10.0).round() / 10.0
}

/// Check the imported body of `mesh`.
pub fn check(mesh: &AvatarMesh) -> Report {
    let mut report = Report {
        file: mesh.file.clone(),
        ready: true,
        triangles: 0,
        vertices: 0,
        height_m: 0.0,
        scale: mesh.scale,
        up: mesh.up,
        turn_deg: mesh.turn_deg,
        skeleton: false,
        items: Vec::new(),
        points: Vec::new(),
    };
    let Ok(p) = placed(mesh) else {
        report.ready = false;
        report.items.push(item(
            "file.unavailable",
            Severity::Error,
            &[("file", json!(mesh.file))],
        ));
        return report;
    };
    use Severity::{Info, Warning};
    let items = &mut report.items;
    let t = p.topology;
    report.triangles = p.triangles.len();
    report.vertices = p.positions.len();
    report.height_m = (f64::from(p.height) * 1000.0).round() / 1000.0;
    report.skeleton = p.skeleton;
    if p.triangles.len() > HEAVY {
        items.push(item(
            "mesh.heavy",
            Warning,
            &[("triangles", json!(p.triangles.len()))],
        ));
    }
    if p.triangles.len() < COARSE {
        items.push(item(
            "mesh.coarse",
            Warning,
            &[("triangles", json!(p.triangles.len()))],
        ));
    }
    if t.boundary_edges > 0 {
        items.push(item(
            "mesh.open",
            Warning,
            &[
                ("holes", json!(t.holes)),
                ("edges", json!(t.boundary_edges)),
            ],
        ));
    }
    if t.non_manifold_edges > 0 {
        items.push(item(
            "mesh.non_manifold",
            Warning,
            &[("edges", json!(t.non_manifold_edges))],
        ));
    }
    if t.pieces > 1 {
        items.push(item(
            "mesh.pieces",
            Warning,
            &[("pieces", json!(t.pieces - 1))],
        ));
    }
    if p.oriented.turned > 0 {
        items.push(item(
            "mesh.reoriented",
            Info,
            &[("triangles", json!(p.oriented.turned))],
        ));
    }
    if p.oriented.inside_out {
        items.push(item("mesh.inside_out", Info, &[]));
    }
    if t.degenerate > 0 {
        items.push(item(
            "mesh.degenerate",
            Info,
            &[("triangles", json!(t.degenerate))],
        ));
    }
    if !(HEIGHT_OK_M.0..=HEIGHT_OK_M.1).contains(&report.height_m) {
        items.push(item(
            "scale.height",
            Warning,
            &[("height_m", json!(report.height_m))],
        ));
    }
    let (lo, hi) = bounds(p.positions.iter());
    let extent = hi - lo;
    if extent.y < 0.8 * extent.x.max(extent.z) {
        items.push(item("orientation.up", Warning, &[]));
    }
    if facing(&p.positions, p.height).is_some_and(|f| f.y < -0.5) {
        items.push(item("orientation.facing", Warning, &[]));
    }
    let winding = Winding::new(&p.positions, &p.triangles);
    for (point, at) in &mesh.points {
        let q = (DVec3::from(*at) / 1000.0).as_vec3();
        let distance_mm = signed_distance(&p, &winding, q);
        if distance_mm > OUTSIDE_MM {
            items.push(item("points.outside", Warning, &[("point", json!(point))]));
        }
        if point.side() * at[0] < 0.0 {
            items.push(item("points.side", Warning, &[("point", json!(point))]));
        }
        report.points.push(PointCheck {
            point: *point,
            at_mm: *at,
            distance_mm,
        });
    }
    for name in measure(&p, &mesh.points).1 {
        items.push(item(
            "measurements.clamped",
            Warning,
            &[("measurement", json!(name))],
        ));
    }
    items.push(item(
        "motion.unavailable",
        Info,
        &[("skeleton", json!(p.skeleton))],
    ));
    report
}
