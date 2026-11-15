//! The sim bridge: build a cloth simulation scene from the document.
//!
//! Every placed panel is meshed (`r3d_geom::mesh`) with its boundary
//! sampled edge by edge; the two edges of a seam get the same number of
//! samples, so their vertices pair up one to one. Panels start wrapped on a
//! vertical cylinder around the body (see `Placement`), and the cloth
//! collides with the fitted avatar and the ground.

use std::collections::BTreeMap;

use glam::{Vec2, Vec3};
use r3d_avatar::measure::girth_at;
use r3d_core::{EdgeId, Fabric, PanelId, Project};
use r3d_geom::DVec2;
use r3d_geom::mesh::mesh_polygon;
use r3d_sim::collide::StaticMesh;
use r3d_sim::scene::{Particle, Triangle};
use r3d_sim::{FabricPreset, SimScene};

use crate::error::CommandError;

/// Default distance between cloth vertices (mm): interactive on a laptop.
pub const DEFAULT_SPACING_MM: f64 = 25.0;
/// Body radius assumed when there is no avatar (m).
const DEFAULT_BODY_RADIUS_M: f32 = 0.16;
/// Panels start on a cone that widens downwards (radius gained per metre
/// below the panel's top), so flared pieces do not overlap before the
/// seams pull them together.
const FLARE: f32 = 0.35;

/// A built simulation scene.
#[derive(Debug, Clone)]
pub struct Drape {
    pub scene: SimScene,
    /// Which panel each particle belongs to.
    pub particle_panels: Vec<PanelId>,
}

/// The body to collide with: positions (m) and triangles.
#[derive(Debug, Clone, Copy)]
pub struct BodyMesh<'a> {
    pub positions: &'a [Vec3],
    pub triangles: &'a [[u32; 3]],
}

/// Height step of the body profile (m).
const PROFILE_STEP: f32 = 0.01;

/// The body's equivalent radius (girth / 2π, m) every `PROFILE_STEP` from
/// the floor, or the default radius where there is no body.
fn body_profile(body: Option<BodyMesh<'_>>) -> Vec<f32> {
    let Some(b) = body else {
        return vec![DEFAULT_BODY_RADIUS_M];
    };
    let height = b.positions.iter().map(|p| p.y).fold(0.0, f32::max);
    let steps = (height / PROFILE_STEP).ceil() as usize;
    (0..=steps)
        .map(|k| {
            girth_at(b.positions, b.triangles, k as f32 * PROFILE_STEP, 0.0)
                .map_or(0.0, |cm| cm / 100.0 / std::f32::consts::TAU)
        })
        .collect()
}

/// Radius at which a panel whose top is at `top` starts, `below` metres
/// under its top: the widest body section between the two heights (a
/// garment hangs outside everything above it), plus the offset and the
/// flare.
fn start_radius(profile: &[f32], top: f32, below: f32, offset: f32) -> f32 {
    let index = |y: f32| ((y / PROFILE_STEP).round().max(0.0) as usize).min(profile.len() - 1);
    let (lo, hi) = (index(top - below), index(top));
    let body = profile[lo..=hi].iter().copied().fold(0.0, f32::max);
    body + offset + FLARE * below
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
    body: Option<BodyMesh<'_>>,
    spacing_mm: f64,
) -> Result<Drape, CommandError> {
    let spacing = spacing_mm.max(2.0);
    let counts = segment_counts(project, spacing);
    let garment = project
        .default_garment()
        .and_then(|g| project.garments().get(&g));
    let fabric = fabric_preset(garment.map(|g| g.fabric).unwrap_or_default()).params();
    let profile = body_profile(body);

    let mut scene = SimScene {
        fabrics: vec![fabric],
        ground: true,
        ..SimScene::default()
    };
    let mut particle_panels = Vec::new();
    // Particle indices along each edge, start to end inclusive.
    let mut edge_vertices: BTreeMap<EdgeId, Vec<u32>> = BTreeMap::new();

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
        let offset = scene.particles.len() as u32;
        let total = boundary.len();
        for (i, e) in panel.contour.iter().enumerate() {
            let n = counts.get(e).copied().unwrap_or(1);
            let verts = (0..=n)
                .map(|k| offset + mesh.boundary[(starts[i] + k) % total])
                .collect();
            edge_vertices.insert(*e, verts);
        }

        // Wrap on the cone: the panel's horizontal centre goes to the
        // placement angle, its top to the placement height; distances along
        // the pattern are kept as arc lengths.
        let lo = mesh.vertices.iter().fold(f64::MAX, |m, v| m.min(v.x));
        let hi = mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.x));
        let top = mesh.vertices.iter().fold(f64::MIN, |m, v| m.max(v.y));
        let centre = (lo + hi) / 2.0;
        let gap = (place.offset_mm / 1000.0) as f32;
        let top_y = place.top_cm as f32 / 100.0;
        let angle0 = (place.angle_deg as f32).to_radians();
        for v in &mesh.vertices {
            let below = ((top - v.y) / 1000.0) as f32;
            let r = start_radius(&profile, top_y, below, gap);
            let theta = angle0 + ((v.x - centre) / 1000.0) as f32 / r;
            let y = top_y - below;
            scene.particles.push(Particle {
                position: Vec3::new(r * theta.sin(), y, r * theta.cos()),
                rest: Vec2::new((v.x / 1000.0) as f32, (v.y / 1000.0) as f32),
                pinned: false,
            });
            particle_panels.push(*panel_id);
        }
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

    if let Some(b) = body {
        scene.collider = Some(StaticMesh::new(
            b.positions.to_vec(),
            b.triangles.to_vec(),
            0.04,
        ));
    }
    Ok(Drape {
        scene,
        particle_panels,
    })
}
