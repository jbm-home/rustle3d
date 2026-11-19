//! The game-ready export (spec 15, M15): one mesh per garment and level
//! of detail, an atlas of the pieces, wrinkles baked into normal maps,
//! back faces or a thin shell, and skinning to the avatar's skeleton with
//! its motions as animations.
//!
//! Levels of detail come from [`r3d_geom::decimate`] (quadric error
//! metrics, Garland & Heckbert 1997) and their normal maps from
//! [`r3d_geom::bake`] (through the atlas they share, Cohen, Olano &
//! Manocha 1998). Skin weights are transferred from the body: each
//! garment vertex takes the weights interpolated at the closest point of
//! the body's surface (the matching stage of Abdrashitov et al., "Robust
//! Skin Weights Transfer via Weight Inpainting", SIGGRAPH Asia 2023
//! Technical Communications), then a few Laplacian smoothing passes over
//! the garment instead of their inpainting, and the four heaviest bones
//! are kept (linear blend skinning, Magnenat-Thalmann et al. 1988).

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat3, Mat4, Quat, Vec2, Vec3};
use r3d_avatar::motion::{EASE_IN_S, Motion};
use r3d_avatar::pack::Influences;
use r3d_avatar::skeleton::Skeleton;
use r3d_core::{GarmentId, Project, Texture};
use r3d_geom::Rigid;
use r3d_geom::bake::{Sheet, bake_colour, bake_normals, tangents};
use r3d_geom::decimate::decimate;
use r3d_io::game::{Clip, GameImage, GameMaterial, GameMesh, GameScene, Joint, Track};
use r3d_io::scene3d::Image3d;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::{ActionSpec, export_stem};
use crate::drape::{Animation, Drape, Form, Latest};
use crate::error::{CommandError, Invalid};
use crate::prints::PrintImage;
use crate::spec::Described;
use crate::workspace::{Workspace, WorkspaceError};

/// Most levels of detail in one export.
pub const MAX_LODS: usize = 5;
/// Animations are sampled this many times a second.
const CLIP_FPS: f32 = 30.0;
/// Laplacian smoothing passes over the transferred weights.
const SMOOTHING_PASSES: usize = 3;
/// Texels of padding around each piece in the maps.
const PADDING: u32 = 8;
/// Size of the base-colour atlas when no normal map sets it.
const COLOUR_SIZE: u32 = 1024;

/// The engine an export is made for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GamePreset {
    #[default]
    Generic,
    Godot,
    Unity,
    Unreal,
}

/// How the cloth's back is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sides {
    /// One surface, its material drawn from both sides.
    #[default]
    DoubleSided,
    /// An inner surface `thickness_mm` inside, facing inwards, and a rim
    /// along the pieces' edges; a one-sided material.
    Shell,
}

impl GamePreset {
    /// Its default LOD ratios.
    pub fn lods(self) -> &'static [f64] {
        match self {
            GamePreset::Unreal => &[1.0, 0.5, 0.25, 0.125],
            _ => &[1.0, 0.5, 0.2],
        }
    }

    /// Its default normal map size.
    pub fn normal_map(self) -> u32 {
        match self {
            GamePreset::Generic => 1024,
            _ => 2048,
        }
    }

    fn name(self) -> &'static str {
        match self {
            GamePreset::Generic => "generic",
            GamePreset::Godot => "godot",
            GamePreset::Unity => "unity",
            GamePreset::Unreal => "unreal",
        }
    }

    fn mesh_name(self, garment: &str, lod: usize) -> String {
        match self {
            GamePreset::Unreal => format!("SK_{garment}_LOD{lod}"),
            _ => format!("{garment}_LOD{lod}"),
        }
    }

    fn material_name(self, garment: &str, lod: usize) -> String {
        let base = match self {
            GamePreset::Unreal => format!("M_{garment}"),
            _ => garment.to_owned(),
        };
        if lod == 0 {
            base
        } else {
            format!("{base}_LOD{lod}")
        }
    }

    fn normal_name(self, garment: &str, lod: usize) -> String {
        match self {
            GamePreset::Unreal => format!("T_{garment}_N_LOD{lod}"),
            _ => format!("{garment}_normal_LOD{lod}"),
        }
    }

    fn colour_name(self, garment: &str) -> String {
        match self {
            GamePreset::Unreal => format!("T_{garment}_D"),
            _ => format!("{garment}_colour"),
        }
    }
}

fn yes() -> bool {
    true
}

/// Export the draped garment for a game engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportGame {
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>-<preset>.glb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `generic` (default), `godot`, `unity` or `unreal`: sets the
    /// defaults below and the names.
    #[serde(default)]
    pub preset: GamePreset,
    /// Levels of detail as ratios of the full triangle count: the first
    /// 1, then decreasing, above 0, at most five (default from the preset:
    /// 1, 0.5, 0.2; Unreal 1, 0.5, 0.25, 0.125).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lods: Option<Vec<f64>>,
    /// Size of the baked normal maps (pixels a side): 0 for none, or a
    /// power of two from 256 to 4096 (default from the preset: 1024,
    /// 2048 for the engines).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_map: Option<u32>,
    /// `double_sided` (default) or `shell`.
    #[serde(default)]
    pub sides: Sides,
    /// The shell's thickness (mm, 0.2–20, default 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thickness_mm: Option<f64>,
    /// Skin the garment to the avatar's skeleton (default true; refused
    /// for a body without one).
    #[serde(default = "yes")]
    pub skin: bool,
    /// With `skin`, the avatar's motions as animations (default true).
    #[serde(default = "yes")]
    pub animations: bool,
    /// Include the avatar's body (default false).
    #[serde(default)]
    pub avatar: bool,
}

impl Default for ExportGame {
    fn default() -> Self {
        Self {
            path: None,
            preset: GamePreset::Generic,
            lods: None,
            normal_map: None,
            sides: Sides::DoubleSided,
            thickness_mm: None,
            skin: true,
            animations: true,
            avatar: false,
        }
    }
}

impl Described for ExportGame {
    const SUMMARY: &'static str = "Export the draped garment ready for a game engine as glTF 2.0 binary (GLB): per garment one mesh per level of detail (`lods`, ratios of the triangle count; simplified by quadric error metrics, the pieces' edges kept), texture coordinates in an atlas of the pieces, the full drape's wrinkles baked into a tangent-space normal map for each lower LOD (`normal_map` pixels), tangents, the print baked into a base-colour atlas, `sides` double_sided or a `shell` of `thickness_mm`, and with `skin` the avatar's 24-bone skeleton, weights transferred from the body (closest point, smoothed, four bones) and the walk, turn and raise_arms motions as animations; `preset` godot, unity or unreal sets the defaults and names. Metres, Y up. Uses the cached drape from `simulate` if it is up to date. An imported body has no skeleton: use skin false. Returns the path and each LOD's triangles.";
}

/// The options with the preset's defaults filled in, checked.
#[derive(Debug, Clone, PartialEq)]
struct Settings {
    preset: GamePreset,
    lods: Vec<f64>,
    normal_map: u32,
    sides: Sides,
    thickness_m: f32,
    skin: bool,
    animations: bool,
    avatar: bool,
}

impl ExportGame {
    fn settings(&self) -> Result<Settings, CommandError> {
        let lods = self
            .lods
            .clone()
            .unwrap_or_else(|| self.preset.lods().to_vec());
        let ordered = lods.first() == Some(&1.0)
            && lods.windows(2).all(|w| w[1] < w[0])
            && lods.iter().all(|r| r.is_finite() && *r > 0.0)
            && lods.len() <= MAX_LODS;
        if !ordered {
            return Err(CommandError::invalid("lods", Invalid::Lods));
        }
        let normal_map = self.normal_map.unwrap_or(self.preset.normal_map());
        if normal_map != 0 && !(normal_map.is_power_of_two() && (256..=4096).contains(&normal_map))
        {
            return Err(CommandError::invalid("normal_map", Invalid::TextureSize));
        }
        let thickness = self.thickness_mm.unwrap_or(1.0);
        if !(0.2..=20.0).contains(&thickness) {
            return Err(CommandError::invalid(
                "thickness_mm",
                Invalid::Range(0.2, 20.0),
            ));
        }
        Ok(Settings {
            preset: self.preset,
            lods,
            normal_map,
            sides: self.sides,
            thickness_m: (thickness / 1000.0) as f32,
            skin: self.skin,
            animations: self.skin && self.animations,
            avatar: self.avatar,
        })
    }
}

/// The draped garment and what it is draped on, as an export sees it.
#[derive(Debug, Clone, Copy)]
pub struct Dressed<'a> {
    pub project: &'a Project,
    pub drape: &'a Drape,
    /// One per particle of `drape`.
    pub positions: &'a [Vec3],
    pub form: Option<&'a Form>,
    /// The avatar's motion and the simulated time of `positions`.
    pub animation: Option<&'a Animation>,
    pub time: f32,
    pub prints: &'a HashMap<GarmentId, Arc<PrintImage>>,
}

/// One garment's pieces merged into one mesh.
struct Merged {
    name: String,
    garment: GarmentId,
    /// Draped (then rest-pose) positions.
    positions: Vec<Vec3>,
    /// Atlas coordinates.
    atlas: Vec<Vec2>,
    /// Coordinates on the print (repeats), when it has one.
    print: Vec<Vec2>,
    triangles: Vec<[u32; 3]>,
    /// Four bones and weights per vertex, when skinned.
    skin: Vec<Influences>,
}

/// What `export.game` wrote, for its result.
#[derive(Debug, Clone, PartialEq)]
pub struct GameReport {
    /// Per LOD: its ratio and the triangles of all garments.
    pub lods: Vec<(f64, usize)>,
    pub normal_map: u32,
    pub skinned: bool,
    pub animations: Vec<&'static str>,
}

impl GameReport {
    pub fn to_json(&self) -> Value {
        json!({
            "lods": self.lods.iter().map(|(r, t)| json!({ "ratio": r, "triangles": t })).collect::<Vec<_>>(),
            "normal_map": self.normal_map,
            "skinned": self.skinned,
            "animations": self.animations,
        })
    }
}

/// The game-ready scene of `dressed` with `request`'s options.
pub fn game_scene(
    dressed: &Dressed<'_>,
    request: &ExportGame,
) -> Result<(GameScene, GameReport), CommandError> {
    let s = request.settings()?;
    let rig = if s.skin {
        Some(
            dressed
                .form
                .and_then(Form::rig)
                .ok_or(CommandError::NoSkeleton)?,
        )
    } else {
        None
    };
    let form = dressed.form;
    // The body where the garment is, and each bone's motion to it.
    let moved = match (dressed.animation, form) {
        (Some(a), Some(f)) => f.transforms(a, dressed.time),
        _ => None,
    };
    let posed_body = form.map(|f| f.posed(dressed.animation, dressed.time));

    let mut garments = merge(dressed);
    if let (Some((_, weights)), Some(f), Some(body)) = (rig, form, &posed_body) {
        let finder = Nearest::new(body, &f.triangles);
        for g in &mut garments {
            g.skin = transfer(&g.positions, &g.triangles, &finder, weights);
            if let Some(t) = &moved {
                g.positions = unpose(&g.positions, &g.skin, t);
            }
        }
    }

    let mut scene = GameScene {
        generator: format!("Rustle3D {}", env!("CARGO_PKG_VERSION")),
        name: dressed.project.name().to_owned(),
        meshes: Vec::new(),
        materials: Vec::new(),
        images: Vec::new(),
        joints: Vec::new(),
        clips: Vec::new(),
        extras: json!({
            "preset": s.preset.name(),
            "lods": s.lods,
            "normal_map": s.normal_map,
            "sides": match s.sides { Sides::DoubleSided => "double_sided", Sides::Shell => "shell" },
            "skinned": s.skin,
        }),
    };
    let mut lod_triangles = vec![0usize; s.lods.len()];
    for g in &garments {
        add_garment(&mut scene, g, dressed, &s, &mut lod_triangles);
    }

    let mut animations = Vec::new();
    if let Some((skeleton, weights)) = rig {
        let (joints, shift) = joints(skeleton);
        scene.joints = joints;
        if shift > 0 {
            for m in &mut scene.meshes {
                for (j, w) in m.joints.iter_mut().zip(&m.weights) {
                    for (b, x) in j.iter_mut().zip(w) {
                        if *x > 0.0 {
                            *b = b.saturating_add(shift);
                        }
                    }
                }
            }
        }
        if s.animations {
            for motion in Motion::ALL {
                scene.clips.push(clip(skeleton, motion, shift));
                animations.push(motion_name(motion));
            }
        }
        if s.avatar
            && let Some(f) = form
        {
            add_avatar(
                &mut scene,
                &f.positions,
                &f.triangles,
                Some((weights, shift)),
            );
        }
    } else if s.avatar
        && let (Some(f), Some(body)) = (form, &posed_body)
    {
        add_avatar(&mut scene, body, &f.triangles, None);
    }
    let report = GameReport {
        lods: s.lods.iter().copied().zip(lod_triangles).collect(),
        normal_map: s.normal_map,
        skinned: s.skin,
        animations,
    };
    Ok((scene, report))
}

fn motion_name(m: Motion) -> &'static str {
    match m {
        Motion::Walk => "walk",
        Motion::Turn => "turn",
        Motion::RaiseArms => "raise_arms",
    }
}

/// Each garment's pieces as one mesh, with the pieces packed in an atlas.
fn merge(d: &Dressed<'_>) -> Vec<Merged> {
    let project = d.project;
    let particles = &d.drape.scene.particles;
    project
        .garments_by_layer()
        .into_iter()
        .filter_map(|id| {
            let garment = project.garments().get(&id)?;
            let pieces: Vec<_> = d
                .drape
                .pieces
                .iter()
                .filter(|p| {
                    project
                        .panel(p.panel)
                        .is_some_and(|panel| panel.garment == id)
                })
                .collect();
            if pieces.is_empty() {
                return None;
            }
            let print = d.prints.get(&id);
            let mut m = Merged {
                name: safe_name(&garment.name),
                garment: id,
                positions: Vec::new(),
                atlas: Vec::new(),
                print: Vec::new(),
                triangles: Vec::new(),
                skin: Vec::new(),
            };
            let mut flats: Vec<(usize, Vec<Vec2>)> = Vec::new();
            for piece in pieces {
                let (Some(at), Some(rest)) = (
                    d.positions.get(piece.particles.clone()),
                    particles.get(piece.particles.clone()),
                ) else {
                    continue;
                };
                let base = m.positions.len() as u32;
                m.positions.extend_from_slice(at);
                flats.push((
                    m.positions.len() - at.len(),
                    rest.iter().map(|p| p.rest).collect(),
                ));
                if print.is_some() {
                    let aspect = print.map_or(1.0, |p| p.aspect);
                    let uvs = crate::prints::piece_uvs(project, d.drape, piece, aspect);
                    if uvs.len() == at.len() {
                        m.print.extend(uvs.iter().map(|uv| Vec2::from(*uv)));
                    } else {
                        m.print.extend(std::iter::repeat_n(Vec2::ZERO, at.len()));
                    }
                }
                m.triangles.extend(
                    piece
                        .indices
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .filter(|t| t.iter().all(|i| (*i as usize) < at.len()))
                        .map(|t| t.map(|i| i + base)),
                );
            }
            m.atlas = pack(&flats, m.positions.len());
            Some(m)
        })
        .collect()
}

/// A name usable in every engine: letters, digits, `_` and `-`.
fn safe_name(name: &str) -> String {
    let out: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        "Garment".into()
    } else {
        out
    }
}

/// Lay the pieces' flat shapes (pattern, m; each from vertex `start`)
/// side by side in the unit square at one scale, upright, `v` down: rows
/// of pieces, tallest first, a gap between them, the row width that
/// fills the square best. Atlas coordinates of `n` vertices.
fn pack(flats: &[(usize, Vec<Vec2>)], n: usize) -> Vec<Vec2> {
    let boxes: Vec<(Vec2, Vec2)> = flats
        .iter()
        .map(|(_, pts)| {
            pts.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            })
        })
        .collect();
    let area: f32 = boxes
        .iter()
        .map(|(lo, hi)| (*hi - *lo).max(Vec2::ZERO).element_product())
        .sum();
    let gap = 0.03 * area.max(1e-6).sqrt();
    let widest = boxes.iter().map(|(lo, hi)| hi.x - lo.x).fold(0.0, f32::max);
    let all: f32 = boxes.iter().map(|(lo, hi)| hi.x - lo.x + gap).sum();
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|a, b| {
        let h = |i: &usize| boxes.get(*i).map_or(0.0, |(lo, hi)| hi.y - lo.y);
        h(b).total_cmp(&h(a)).then(a.cmp(b))
    });
    let shelves = |row_width: f32| -> (Vec<Vec2>, f32) {
        let mut offsets = vec![Vec2::ZERO; boxes.len()];
        let (mut x, mut y, mut row) = (gap, gap, 0.0f32);
        let mut width = 0.0f32;
        for i in &order {
            let Some((lo, hi)) = boxes.get(*i) else {
                continue;
            };
            let size = *hi - *lo;
            if x > gap && x + size.x + gap > row_width {
                x = gap;
                y += row + gap;
                row = 0.0;
            }
            if let Some(o) = offsets.get_mut(*i) {
                *o = Vec2::new(x, y);
            }
            x += size.x + gap;
            width = width.max(x);
            row = row.max(size.y);
        }
        (offsets, width.max(y + row + gap).max(1e-6))
    };
    // Row widths from the widest piece to all in one row.
    let (offsets, side) = (0..=32)
        .map(|k| {
            let w = widest + 2.0 * gap;
            shelves(w + (all + gap - w).max(0.0) * k as f32 / 32.0)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or_else(|| shelves(widest + 2.0 * gap));
    let mut out = vec![Vec2::ZERO; n];
    for ((start, pts), ((lo, hi), offset)) in flats.iter().zip(boxes.iter().zip(&offsets)) {
        for (k, p) in pts.iter().enumerate() {
            if let Some(slot) = out.get_mut(start + k) {
                // Pattern y points up; the image's v points down.
                *slot = Vec2::new(offset.x + p.x - lo.x, offset.y + hi.y - p.y) / side;
            }
        }
    }
    out
}

/// Body triangles binned in a grid, for closest-point queries.
struct Nearest<'a> {
    positions: &'a [Vec3],
    triangles: &'a [[u32; 3]],
    cell: f32,
    grid: HashMap<[i32; 3], Vec<u32>>,
}

impl<'a> Nearest<'a> {
    fn new(positions: &'a [Vec3], triangles: &'a [[u32; 3]]) -> Self {
        let cell = 0.04;
        let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        let key = |p: Vec3| (p / cell).floor().as_ivec3().to_array();
        for (i, t) in triangles.iter().enumerate() {
            let Some(c) = corners(positions, *t) else {
                continue;
            };
            let (a, b) = (key(c[0].min(c[1]).min(c[2])), key(c[0].max(c[1]).max(c[2])));
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        grid.entry([x, y, z]).or_default().push(i as u32);
                    }
                }
            }
        }
        Self {
            positions,
            triangles,
            cell,
            grid,
        }
    }

    /// The triangle closest to `p` and the closest point's barycentric
    /// weights in it.
    fn find(&self, p: Vec3) -> Option<([u32; 3], Vec3)> {
        let centre = (p / self.cell).floor().as_ivec3();
        let mut best: Option<(f32, [u32; 3], Vec3)> = None;
        for ring in 0..64i32 {
            // Every cell at Chebyshev distance `ring` from the centre.
            for x in -ring..=ring {
                for y in -ring..=ring {
                    for z in -ring..=ring {
                        if x.abs().max(y.abs()).max(z.abs()) != ring {
                            continue;
                        }
                        let key = (centre + glam::IVec3::new(x, y, z)).to_array();
                        for t in self.grid.get(&key).into_iter().flatten() {
                            let Some(tri) = self.triangles.get(*t as usize) else {
                                continue;
                            };
                            let Some(c) = corners(self.positions, *tri) else {
                                continue;
                            };
                            let (q, _) = r3d_sim::collide::closest_feature(p, c[0], c[1], c[2]);
                            let d = q.distance_squared(p);
                            if best.as_ref().is_none_or(|b| d < b.0) {
                                best = Some((d, *tri, barycentric(q, c)));
                            }
                        }
                    }
                }
            }
            // Cells beyond this ring are at least `ring` cells away.
            if let Some((d, ..)) = best
                && d.sqrt() <= ring as f32 * self.cell
            {
                break;
            }
        }
        best.map(|(_, t, w)| (t, w))
    }
}

fn corners<T: Copy>(values: &[T], t: [u32; 3]) -> Option<[T; 3]> {
    Some([
        *values.get(t[0] as usize)?,
        *values.get(t[1] as usize)?,
        *values.get(t[2] as usize)?,
    ])
}

/// Barycentric weights of `q` (in the triangle's plane) in `c`.
fn barycentric(q: Vec3, c: [Vec3; 3]) -> Vec3 {
    let (v0, v1, v2) = (c[1] - c[0], c[2] - c[0], q - c[0]);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-20 {
        return Vec3::new(1.0, 0.0, 0.0);
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    Vec3::new(1.0 - v - w, v, w)
}

/// The garment's skin weights from the body's: at each vertex those of
/// the closest body point, smoothed over the garment's edges, the four
/// heaviest kept and normalized.
fn transfer(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    finder: &Nearest<'_>,
    weights: &[Influences],
) -> Vec<Influences> {
    let bones = weights
        .iter()
        .flat_map(|w| w.bones)
        .map(|b| usize::from(b) + 1)
        .max()
        .unwrap_or(1);
    let mut dense: Vec<Vec<f32>> = positions
        .iter()
        .map(|p| {
            let mut out = vec![0.0f32; bones];
            if let Some((t, w)) = finder.find(*p) {
                for (corner, share) in t.iter().zip([w.x, w.y, w.z]) {
                    if let Some(inf) = weights.get(*corner as usize) {
                        for (b, x) in inf.bones.iter().zip(inf.weights) {
                            if let Some(slot) = out.get_mut(usize::from(*b)) {
                                *slot += x * share.max(0.0);
                            }
                        }
                    }
                }
            }
            out
        })
        .collect();
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); positions.len()];
    for t in triangles {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            if let Some(n) = neighbours.get_mut(a as usize) {
                n.push(b);
            }
            if let Some(n) = neighbours.get_mut(b as usize) {
                n.push(a);
            }
        }
    }
    for n in &mut neighbours {
        n.sort_unstable();
        n.dedup();
    }
    for _ in 0..SMOOTHING_PASSES {
        dense = dense
            .iter()
            .zip(&neighbours)
            .map(|(own, around)| {
                if around.is_empty() {
                    return own.clone();
                }
                let mut avg = vec![0.0f32; bones];
                for v in around {
                    if let Some(w) = dense.get(*v as usize) {
                        for (a, x) in avg.iter_mut().zip(w) {
                            *a += x;
                        }
                    }
                }
                let k = around.len() as f32;
                own.iter()
                    .zip(&avg)
                    .map(|(o, a)| 0.5 * o + 0.5 * a / k)
                    .collect()
            })
            .collect();
    }
    dense.iter().map(|w| strongest(w)).collect()
}

/// The four heaviest bones of `dense`, their weights summing to 1
/// (unused slots: bone 0, weight 0).
fn strongest(dense: &[f32]) -> Influences {
    let mut order: Vec<usize> = (0..dense.len()).collect();
    order.sort_by(|a, b| {
        let w = |i: &usize| dense.get(*i).copied().unwrap_or(0.0);
        w(b).total_cmp(&w(a)).then(a.cmp(b))
    });
    let mut out = Influences::default();
    let top: Vec<(usize, f32)> = order
        .into_iter()
        .take(4)
        .map(|i| (i, dense.get(i).copied().unwrap_or(0.0)))
        .filter(|(_, w)| *w > 1e-4)
        .collect();
    let total: f32 = top.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        out.weights[0] = 1.0;
        return out;
    }
    for (k, (b, w)) in top.iter().enumerate() {
        if let (Some(slot), Some(weight)) = (out.bones.get_mut(k), out.weights.get_mut(k)) {
            *slot = u8::try_from(*b).unwrap_or(0);
            *weight = w / total;
        }
    }
    // Exactly 1: the rounding left on the heaviest.
    let rest: f32 = out.weights.iter().skip(1).sum();
    out.weights[0] = 1.0 - rest;
    out
}

/// The blended bone motion of a vertex with `skin`, as a matrix and a
/// translation.
fn blended(skin: &Influences, transforms: &[Rigid]) -> (Mat3, Vec3) {
    let mut m = Mat3::ZERO;
    let mut t = Vec3::ZERO;
    for (b, w) in skin.bones.iter().zip(skin.weights) {
        if w > 0.0
            && let Some(r) = transforms.get(usize::from(*b))
        {
            m += Mat3::from_quat(r.rotation) * w;
            t += r.translation * w;
        }
    }
    (m, t)
}

/// Positions carried back to the rest pose from where `transforms`
/// moved the body: each vertex by the inverse of its blended motion.
fn unpose(positions: &[Vec3], skin: &[Influences], transforms: &[Rigid]) -> Vec<Vec3> {
    positions
        .iter()
        .zip(skin)
        .map(|(p, s)| {
            let (m, t) = blended(s, transforms);
            if m.determinant().abs() < 1e-6 {
                *p
            } else {
                m.inverse() * (*p - t)
            }
        })
        .collect()
}

/// Smooth vertex normals (area-weighted).
fn normals(positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<Vec3> {
    let mut n = vec![Vec3::ZERO; positions.len()];
    for t in triangles {
        let Some(c) = corners(positions, *t) else {
            continue;
        };
        let f = (c[1] - c[0]).cross(c[2] - c[0]);
        for v in t {
            if let Some(a) = n.get_mut(*v as usize) {
                *a += f;
            }
        }
    }
    n.iter()
        .map(|v| v.try_normalize().unwrap_or(Vec3::Y))
        .collect()
}

/// The vertices `triangles` use, in order, and the triangles renumbered.
fn compact(triangles: &[[u32; 3]], n: usize) -> (Vec<u32>, Vec<[u32; 3]>) {
    let mut map = vec![u32::MAX; n];
    let mut kept = Vec::new();
    let tris: Vec<[u32; 3]> = triangles
        .iter()
        .map(|t| {
            t.map(|v| match map.get_mut(v as usize) {
                Some(m) if *m != u32::MAX => *m,
                Some(m) => {
                    *m = kept.len() as u32;
                    kept.push(v);
                    *m
                }
                None => 0,
            })
        })
        .collect();
    let mut order: Vec<u32> = (0..kept.len() as u32).collect();
    // Keep the vertices in their original order.
    order.sort_by_key(|i| kept.get(*i as usize).copied().unwrap_or(0));
    let mut rank = vec![0u32; kept.len()];
    for (r, i) in order.iter().enumerate() {
        if let Some(slot) = rank.get_mut(*i as usize) {
            *slot = r as u32;
        }
    }
    let tris: Vec<[u32; 3]> = tris
        .iter()
        .map(|t| t.map(|v| rank.get(v as usize).copied().unwrap_or(0)))
        .collect();
    let vertices = order
        .iter()
        .map(|i| kept.get(*i as usize).copied().unwrap_or(0))
        .collect();
    (vertices, tris)
}

/// One LOD's surface: vertices (indices into the garment's), triangles.
struct Surface {
    vertices: Vec<u32>,
    triangles: Vec<[u32; 3]>,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    atlas: Vec<Vec2>,
    tangents: Vec<[f32; 4]>,
}

impl Surface {
    fn of(g: &Merged, vertices: Vec<u32>, triangles: Vec<[u32; 3]>) -> Self {
        let pick = |v: &u32| *v as usize;
        let positions: Vec<Vec3> = vertices
            .iter()
            .map(|v| g.positions.get(pick(v)).copied().unwrap_or_default())
            .collect();
        let atlas: Vec<Vec2> = vertices
            .iter()
            .map(|v| g.atlas.get(pick(v)).copied().unwrap_or_default())
            .collect();
        let normals = normals(&positions, &triangles);
        let tangents = tangents(&Sheet {
            positions: &positions,
            normals: &normals,
            uvs: &atlas,
            triangles: &triangles,
        });
        Self {
            vertices,
            triangles,
            positions,
            normals,
            atlas,
            tangents,
        }
    }

    fn sheet(&self) -> Sheet<'_> {
        Sheet {
            positions: &self.positions,
            normals: &self.normals,
            uvs: &self.atlas,
            triangles: &self.triangles,
        }
    }
}

/// `surface` as a mesh; as a shell `thickness` thick when asked: an
/// inner copy offset along the normals, facing in, and a rim along the
/// border edges.
fn mesh_of(
    surface: &Surface,
    g: &Merged,
    shell: Option<f32>,
    name: String,
    material: usize,
) -> GameMesh {
    let skin = |v: &u32| g.skin.get(*v as usize).copied();
    let mut positions = surface.positions.clone();
    let mut atlas = surface.atlas.clone();
    let mut origin: Vec<u32> = surface.vertices.clone();
    let mut triangles = surface.triangles.clone();
    if let Some(t) = shell {
        let n = positions.len() as u32;
        let inner: Vec<Vec3> = surface
            .positions
            .iter()
            .zip(&surface.normals)
            .map(|(p, nrm)| *p - *nrm * t)
            .collect();
        positions.extend(&inner);
        atlas.extend_from_slice(&surface.atlas);
        origin.extend_from_slice(&surface.vertices);
        triangles.extend(
            surface
                .triangles
                .iter()
                .map(|[a, b, c]| [a + n, c + n, b + n]),
        );
        // Border edges, in the direction their triangle runs them.
        let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
        for tri in &surface.triangles {
            for k in 0..3 {
                let (a, b) = (tri[k], tri[(k + 1) % 3]);
                *uses.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let mut rim: HashMap<u32, (u32, u32)> = HashMap::new();
        for tri in &surface.triangles {
            for k in 0..3 {
                let (a, b) = (tri[k], tri[(k + 1) % 3]);
                if uses.get(&(a.min(b), a.max(b))) != Some(&1) {
                    continue;
                }
                let mut copy = |v: u32| -> (u32, u32) {
                    *rim.entry(v).or_insert_with(|| {
                        let at = positions.len() as u32;
                        for p in [surface.positions.get(v as usize), inner.get(v as usize)] {
                            positions.push(p.copied().unwrap_or_default());
                            atlas.push(surface.atlas.get(v as usize).copied().unwrap_or_default());
                            origin.push(surface.vertices.get(v as usize).copied().unwrap_or(0));
                        }
                        (at, at + 1)
                    })
                };
                let (ao, ai) = copy(a);
                let (bo, bi) = copy(b);
                triangles.push([ao, ai, bi]);
                triangles.push([ao, bi, bo]);
            }
        }
    }
    let normals = normals(&positions, &triangles);
    let tangents = if shell.is_some() {
        tangents(&Sheet {
            positions: &positions,
            normals: &normals,
            uvs: &atlas,
            triangles: &triangles,
        })
    } else {
        surface.tangents.clone()
    };
    let skins: Vec<Influences> = origin.iter().filter_map(skin).collect();
    let skinned = skins.len() == origin.len() && !skins.is_empty();
    GameMesh {
        name,
        positions: positions.iter().map(|p| p.to_array()).collect(),
        normals: normals.iter().map(|n| n.to_array()).collect(),
        tangents,
        uvs: atlas.iter().map(|uv| uv.to_array()).collect(),
        joints: if skinned {
            skins.iter().map(|s| s.bones).collect()
        } else {
            Vec::new()
        },
        weights: if skinned {
            skins.iter().map(|s| s.weights).collect()
        } else {
            Vec::new()
        },
        indices: triangles.iter().flatten().copied().collect(),
        material,
    }
}

fn png(size: u32, rgba: Vec<u8>) -> Image3d {
    Image3d {
        bytes: r3d_io::texture::encode_png(&Texture {
            width: size,
            height: size,
            rgba,
        }),
        mime: "image/png",
    }
}

/// The print's colour at `uv` (repeats, `v` down the image), bilinear,
/// repeating.
fn sample(t: &Texture, uv: Vec2) -> [u8; 4] {
    let (w, h) = (t.width.max(1) as f32, t.height.max(1) as f32);
    let x = uv.x.rem_euclid(1.0) * w - 0.5;
    let y = uv.y.rem_euclid(1.0) * h - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let px = |xi: f32, yi: f32| -> [f32; 4] {
        let xi = (xi as i64).rem_euclid(i64::from(t.width.max(1))) as usize;
        let yi = (yi as i64).rem_euclid(i64::from(t.height.max(1))) as usize;
        let i = (yi * t.width as usize + xi) * 4;
        match t.rgba.get(i..i + 4) {
            Some(c) => [c[0], c[1], c[2], 255].map(f32::from),
            None => [0.0, 0.0, 0.0, 255.0],
        }
    };
    let (a, b, c, d) = (
        px(x0, y0),
        px(x0 + 1.0, y0),
        px(x0, y0 + 1.0),
        px(x0 + 1.0, y0 + 1.0),
    );
    let mut out = [0u8; 4];
    for (k, o) in out.iter_mut().enumerate() {
        let top = a[k] + (b[k] - a[k]) * fx;
        let bottom = c[k] + (d[k] - c[k]) * fx;
        *o = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// One garment's LODs, materials and maps.
fn add_garment(
    scene: &mut GameScene,
    g: &Merged,
    d: &Dressed<'_>,
    s: &Settings,
    lod_triangles: &mut [usize],
) {
    let Some(garment) = d.project.garments().get(&g.garment) else {
        return;
    };
    let print = d.prints.get(&g.garment);
    let look = garment.fabric.look();
    let full = Surface::of(
        g,
        (0..g.positions.len() as u32).collect(),
        g.triangles.clone(),
    );
    let base_colour = print.map(|p| {
        let size = if s.normal_map > 0 {
            s.normal_map
        } else {
            COLOUR_SIZE
        };
        let rgba = bake_colour(
            &g.atlas,
            &g.triangles,
            &g.print,
            size,
            PADDING,
            [0, 0, 0, 255],
            |uv| sample(&p.texture, uv),
        );
        scene.images.push(GameImage {
            name: s.preset.colour_name(&g.name),
            image: png(size, rgba),
        });
        scene.images.len() - 1
    });
    let shell = (s.sides == Sides::Shell).then_some(s.thickness_m);
    for (lod, ratio) in s.lods.iter().enumerate() {
        let surface = if lod == 0 {
            None
        } else {
            let target = (g.triangles.len() as f64 * ratio).ceil() as usize;
            let kept = decimate(&g.positions, &g.atlas, &g.triangles, target);
            let (vertices, triangles) = compact(&kept, g.positions.len());
            Some(Surface::of(g, vertices, triangles))
        };
        let surface = surface.as_ref().unwrap_or(&full);
        let normal_map = (lod > 0 && s.normal_map > 0).then(|| {
            let rgba = bake_normals(
                &full.sheet(),
                &surface.sheet(),
                &surface.tangents,
                s.normal_map,
                PADDING,
            );
            scene.images.push(GameImage {
                name: s.preset.normal_name(&g.name, lod),
                image: png(s.normal_map, rgba),
            });
            scene.images.len() - 1
        });
        scene.materials.push(GameMaterial {
            name: s.preset.material_name(&g.name, lod),
            rgb: print.map_or(garment.rgb(), |p| p.mean_rgb),
            sheen: look.sheen,
            sheen_roughness: look.sheen_roughness,
            roughness: look.roughness,
            double_sided: shell.is_none(),
            base_colour,
            normal_map,
        });
        if let Some(count) = lod_triangles.get_mut(lod) {
            *count += surface.triangles.len();
        }
        scene.meshes.push(mesh_of(
            surface,
            g,
            shell,
            s.preset.mesh_name(&g.name, lod),
            scene.materials.len() - 1,
        ));
    }
}

/// The avatar's body: at rest and skinned with its own weights (bones
/// shifted by `shift`), or as it stands without a skeleton.
fn add_avatar(
    scene: &mut GameScene,
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    skin: Option<(&[Influences], u8)>,
) {
    scene.materials.push(GameMaterial {
        name: "Avatar".into(),
        rgb: 0xC9C6C1,
        sheen: 0.0,
        sheen_roughness: 0.0,
        roughness: 0.7,
        double_sided: false,
        base_colour: None,
        normal_map: None,
    });
    let (joints, weights) = match skin {
        Some((w, shift)) if w.len() == positions.len() => (
            w.iter()
                .map(|i| i.bones.map(|b| b.saturating_add(shift)))
                .collect(),
            w.iter().map(|i| i.weights).collect::<Vec<_>>(),
        ),
        _ => (Vec::new(), Vec::new()),
    };
    let (joints, weights) = clean_slots(joints, weights);
    scene.meshes.push(GameMesh {
        name: "Avatar".into(),
        positions: positions.iter().map(|p| p.to_array()).collect(),
        normals: normals(positions, triangles)
            .iter()
            .map(|n| n.to_array())
            .collect(),
        tangents: Vec::new(),
        uvs: Vec::new(),
        joints,
        weights,
        indices: triangles.iter().flatten().copied().collect(),
        material: scene.materials.len() - 1,
    });
}

/// Slots of zero weight on joint 0, and weights summing to exactly 1.
fn clean_slots(joints: Vec<[u8; 4]>, weights: Vec<[f32; 4]>) -> (Vec<[u8; 4]>, Vec<[f32; 4]>) {
    joints
        .into_iter()
        .zip(weights)
        .map(|(mut j, mut w)| {
            let total: f32 = w.iter().map(|x| x.max(0.0)).sum();
            if total <= 0.0 {
                return ([j[0], 0, 0, 0], [1.0, 0.0, 0.0, 0.0]);
            }
            for (b, x) in j.iter_mut().zip(w.iter_mut()) {
                *x = x.max(0.0) / total;
                if *x == 0.0 {
                    *b = 0;
                }
            }
            let heaviest = (0..4)
                .max_by(|a, b| w[*a].total_cmp(&w[*b]).then(b.cmp(a)))
                .unwrap_or(0);
            let rest: f32 = (0..4).filter(|k| *k != heaviest).map(|k| w[k]).sum();
            w[heaviest] = 1.0 - rest;
            (j, w)
        })
        .unzip()
}

/// The skeleton as joints at rest (no rotation, each at its head joint),
/// under one root: a `root` joint at the origin is added when the
/// skeleton has several. Also how much the bone indices shift.
fn joints(skeleton: &Skeleton) -> (Vec<Joint>, u8) {
    let roots = skeleton.bones.iter().filter(|b| b.parent.is_none()).count();
    let shift = usize::from(roots > 1);
    let mut out = Vec::with_capacity(skeleton.bones.len() + shift);
    if shift == 1 {
        out.push(Joint {
            name: "root".into(),
            parent: None,
            translation: [0.0; 3],
            inverse_bind: Mat4::IDENTITY.to_cols_array(),
        });
    }
    for bone in &skeleton.bones {
        let parent_head = bone
            .parent
            .and_then(|p| skeleton.bones.get(p))
            .map_or(Vec3::ZERO, |p| p.head);
        out.push(Joint {
            name: bone.name.clone(),
            parent: bone.parent.map(|p| p + shift).or((shift == 1).then_some(0)),
            translation: (bone.head - parent_head).to_array(),
            inverse_bind: Mat4::from_translation(-bone.head).to_cols_array(),
        });
    }
    (out, shift as u8)
}

/// One cycle of `motion` at full amplitude, from phase 0, as local joint
/// rotations (and the root bones' translations), looping.
fn clip(skeleton: &Skeleton, motion: Motion, shift: u8) -> Clip {
    let period = motion.period();
    let frames = (period * CLIP_FPS).round().max(2.0) as usize;
    // The first cycle start after the motion has fully grown in.
    let t0 = (EASE_IN_S / period).ceil() * period;
    let n = skeleton.bones.len();
    let mut rotations: Vec<Vec<[f32; 4]>> = vec![Vec::with_capacity(frames + 1); n];
    let mut translations: Vec<Vec<[f32; 3]>> = vec![Vec::new(); n];
    let mut times = Vec::with_capacity(frames + 1);
    for k in 0..=frames {
        let dt = period * k as f32 / frames as f32;
        times.push(dt);
        let transforms = skeleton.transforms(&motion.pose(skeleton, t0 + dt));
        // A joint's place in the scene: its bone's motion applied to the
        // rest joint (which has no rotation).
        let world: Vec<Rigid> = skeleton
            .bones
            .iter()
            .zip(&transforms)
            .map(|(b, t)| Rigid {
                rotation: t.rotation,
                translation: t.apply(b.head),
            })
            .collect();
        for (i, bone) in skeleton.bones.iter().enumerate() {
            let Some(w) = world.get(i) else {
                continue;
            };
            let local = match bone.parent.and_then(|p| world.get(p)) {
                Some(parent) => parent.inverse().after(w),
                None => *w,
            };
            if let Some(track) = rotations.get_mut(i) {
                let mut q: Quat = local.rotation.normalize();
                // The shorter way from the previous sample.
                if let Some(prev) = track.last()
                    && Quat::from_array(*prev).dot(q) < 0.0
                {
                    q = -q;
                }
                track.push(q.to_array());
            }
            if bone.parent.is_none()
                && let Some(track) = translations.get_mut(i)
            {
                track.push(local.translation.to_array());
            }
        }
    }
    let mut channels = Vec::new();
    for (i, track) in rotations.into_iter().enumerate() {
        channels.push((i + usize::from(shift), Track::Rotation(track)));
    }
    for (i, track) in translations.into_iter().enumerate() {
        if !track.is_empty() {
            channels.push((i + usize::from(shift), Track::Translation(track)));
        }
    }
    Clip {
        name: motion_name(motion).into(),
        times,
        channels,
    }
}

impl ActionSpec for ExportGame {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let project = ws.session().project();
        let latest = Latest::load(ws.dir(), project)?;
        let prints = crate::prints::Prints::default().all(project, Some(ws.dir()));
        let dressed = Dressed {
            project,
            drape: &latest.drape,
            positions: &latest.positions,
            form: latest.form.as_ref(),
            animation: latest.animation.as_ref(),
            time: latest.time,
            prints: &prints,
        };
        let (scene, report) = game_scene(&dressed, self)?;
        let path = self
            .path
            .clone()
            .unwrap_or_else(|| format!("exports/{}-{}.glb", export_stem(ws), self.preset.name()));
        let written = ws.write(&path, &r3d_io::game::to_glb(&scene))?;
        let mut out = report.to_json();
        out["path"] = json!(written);
        out["drape"] = json!(latest.state());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn settings_follow_the_preset_and_are_checked() {
        let s = ExportGame {
            preset: GamePreset::Unreal,
            ..ExportGame::default()
        }
        .settings()
        .unwrap();
        assert_eq!(s.lods, vec![1.0, 0.5, 0.25, 0.125]);
        assert_eq!(s.normal_map, 2048);
        for lods in [vec![0.5], vec![1.0, 0.5, 0.6], vec![1.0, 0.0], vec![1.0; 6]] {
            let bad = ExportGame {
                lods: Some(lods),
                ..ExportGame::default()
            };
            assert!(bad.settings().is_err());
        }
        for size in [100, 1000, 8192] {
            let bad = ExportGame {
                normal_map: Some(size),
                ..ExportGame::default()
            };
            assert!(bad.settings().is_err());
        }
        assert!(
            ExportGame {
                normal_map: Some(0),
                ..ExportGame::default()
            }
            .settings()
            .is_ok()
        );
    }

    #[test]
    fn the_strongest_four_bones_sum_to_one() {
        let w = strongest(&[0.1, 0.0, 0.3, 0.2, 0.25, 0.15]);
        assert_eq!(w.bones, [2, 4, 3, 5]);
        assert!((w.weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(w.weights[0] > w.weights[3]);
        let none = strongest(&[0.0; 4]);
        assert_eq!(none.weights, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn pieces_pack_apart_in_the_unit_square() {
        let square = |x: f32, s: f32| {
            vec![
                Vec2::new(x, 0.0),
                Vec2::new(x + s, 0.0),
                Vec2::new(x + s, s),
                Vec2::new(x, s),
            ]
        };
        // Two pieces drawn on top of each other on the board.
        let flats = vec![(0, square(0.0, 0.5)), (4, square(0.1, 0.3))];
        let uv = pack(&flats, 8);
        assert!(
            uv.iter()
                .all(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= 1.0 && p.y <= 1.0)
        );
        let bounds = |r: std::ops::Range<usize>| {
            uv[r].iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            })
        };
        let (a, b) = (bounds(0..4), bounds(4..8));
        let apart = a.1.x < b.0.x || b.1.x < a.0.x || a.1.y < b.0.y || b.1.y < a.0.y;
        assert!(apart, "{a:?} {b:?}");
        // One scale for all: sizes keep their ratio.
        assert!(((a.1.x - a.0.x) / (b.1.x - b.0.x) - 0.5 / 0.3).abs() < 1e-4);
        // Upright, v down: the pattern's top is at the smaller v.
        assert!(uv[3].y < uv[0].y);
    }

    #[test]
    fn compacting_keeps_the_vertex_order() {
        let (v, t) = compact(&[[5, 2, 7], [7, 2, 9]], 10);
        assert_eq!(v, vec![2, 5, 7, 9]);
        assert_eq!(t, vec![[1, 0, 2], [2, 0, 3]]);
    }

    #[test]
    fn unposing_undoes_the_blended_motion() {
        let t = [
            Rigid::about(Vec3::ZERO, Quat::from_rotation_z(0.6)),
            Rigid {
                rotation: Quat::from_rotation_x(-0.3),
                translation: Vec3::new(0.1, 0.0, 0.2),
            },
        ];
        let skin = Influences {
            bones: [0, 1, 0, 0],
            weights: [0.3, 0.7, 0.0, 0.0],
        };
        let rest = Vec3::new(0.2, 1.1, -0.1);
        let (m, tr) = blended(&skin, &t);
        let posed = m * rest + tr;
        let back = unpose(&[posed], &[skin], &t);
        assert!(back[0].distance(rest) < 1e-5);
    }
}
