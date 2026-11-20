//! What the engine keeps of the loaded simulation, outside the bridge's
//! public API (the code generator exposes everything in `api`).

use std::time::{Duration, Instant};

use r3d_core::{EdgeId, PanelId, PointId, SeamId};
use r3d_geom::DVec2;

/// Pattern edits not yet carried into the running simulation.
pub(crate) struct Sync {
    pub pending: bool,
    pub last: Instant,
}

impl Default for Sync {
    fn default() -> Self {
        Self {
            pending: false,
            last: Instant::now(),
        }
    }
}

/// How often pattern edits are carried into the running simulation at
/// most: each rebuild remeshes and restarts the solver.
pub(crate) const RESYNC_EVERY: Duration = Duration::from_millis(100);

/// Pick radius on the cloth (physical pixels).
pub(crate) const PICK_PX: f32 = 24.0;

/// Cloth under a pressed pointer, and what dragging it does.
#[derive(Clone, Copy)]
pub(crate) struct Held {
    pub particle: usize,
    /// Where it was when pressed: the dragging plane goes through it.
    pub anchor: glam::Vec3,
    pub tool: Tool,
}

#[derive(Clone, Copy)]
pub(crate) enum Tool {
    /// Pull the cloth. The grab starts on the first move, so a simple
    /// click only selects.
    Pull { pulling: bool },
    /// Lengthen or shorten the garment at an open edge (`edge.lengthen`):
    /// `applied` mm so far.
    Length { edge: EdgeId, applied: f64 },
    /// Take in or let out a seam (`seam.take_in`), at one end (`at`) or
    /// along all of it: `applied` mm so far.
    Fit {
        seam: SeamId,
        at: Option<PointId>,
        applied: f64,
    },
    /// Move a piece around the body (`panel.place`): its placement when
    /// the drag started, and whether the drape was running.
    Move {
        panel: PanelId,
        from: r3d_core::Placement,
        running: bool,
    },
    /// On the board: move a corner (`point.move`) from where it was; `grab`
    /// is where the pointer pressed (mm).
    Corner {
        point: PointId,
        from: DVec2,
        grab: DVec2,
    },
    /// On the board: move an edge (`edge.move`), `applied` mm so far.
    Edge {
        edge: EdgeId,
        grab: DVec2,
        applied: DVec2,
    },
    /// On the board: move a whole piece (`panel.translate`), `applied` mm
    /// so far.
    Piece {
        panel: PanelId,
        grab: DVec2,
        applied: DVec2,
    },
}

/// On the board, corners within this distance of the pointer are picked
/// before edges (px).
pub(crate) const CORNER_PICK_PX: f64 = 10.0;
/// How close (pixels) a dragged corner must come to something to snap.
pub(crate) const SNAP_PX: f64 = 8.0;

/// What ⇧-dragging a particle on a contour edge adjusts.
#[derive(Clone, Copy)]
pub(crate) enum Adjust {
    Length(EdgeId),
    Fit(SeamId, Option<PointId>),
}

/// Along a seam, grabbing within this fraction of an end adjusts that end
/// only.
pub(crate) const SEAM_END: f32 = 0.3;

/// A cloth particle under the cursor.
pub(crate) struct Hit {
    pub particle: usize,
    pub position: glam::Vec3,
    pub panel: Option<PanelId>,
}

/// The body's surface as it is now (posed while it moves), with its
/// form.
pub(crate) type LiveBody = (Vec<glam::Vec3>, std::sync::Arc<r3d_commands::drape::Form>);

/// A click near a contour edge selects it: on the board (physical pixels)
/// and on the garment (mm on the pattern).
pub(crate) const EDGE_PICK_PX: f64 = 10.0;
pub(crate) const EDGE_PICK_MM: f64 = 20.0;

/// Points of a line drawn on the garment are kept this far apart on the
/// pattern (mm).
pub(crate) const STROKE_STEP_MM: f64 = 8.0;

/// Where a ray meets the cloth.
pub(crate) struct ClothHit {
    pub panel: PanelId,
    /// The point on the panel's pattern (mm).
    pub at: r3d_geom::DVec2,
    /// The point in space (m).
    pub point: glam::Vec3,
}

/// Ray / triangle intersection (Möller & Trumbore, "Fast, minimum storage
/// ray-triangle intersection", 1997): distance along the ray and the
/// barycentric weights of `b` and `c`. Both faces count.
pub(crate) fn ray_triangle(
    origin: glam::Vec3,
    dir: glam::Vec3,
    a: glam::Vec3,
    b: glam::Vec3,
    c: glam::Vec3,
) -> Option<(f32, f32, f32)> {
    let (e1, e2) = (b - a, c - a);
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let w = dir.dot(q) * inv;
    if w < 0.0 || u + w > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 0.0).then_some((t, u, w))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn rays_hit_triangles_with_barycentric_weights() {
        let (a, b, c) = (Vec3::ZERO, Vec3::X, Vec3::Y);
        let hit = ray_triangle(Vec3::new(0.25, 0.5, 1.0), -Vec3::Z, a, b, c);
        let (t, u, w) = hit.unwrap_or_default();
        assert!((t - 1.0).abs() < 1e-6 && (u - 0.25).abs() < 1e-6 && (w - 0.5).abs() < 1e-6);
        assert!(ray_triangle(Vec3::new(0.8, 0.8, 1.0), -Vec3::Z, a, b, c).is_none());
        assert!(ray_triangle(Vec3::new(0.2, 0.2, 1.0), Vec3::Z, a, b, c).is_none());
    }
}

/// One piece of the cloth as the views draw it: its panel, its mesh (with
/// texture coordinates when its garment has a print) and that print.
#[derive(Debug)]
pub(crate) struct ClothPiece {
    pub panel: PanelId,
    pub mesh: r3d_render::Mesh,
    pub print: Option<std::sync::Arc<r3d_commands::prints::PrintImage>>,
}

impl ClothPiece {
    pub(crate) fn printed(&self) -> Option<r3d_render::Printed<'_>> {
        self.print.as_ref().map(|p| r3d_render::Printed {
            key: p.key,
            texture: &p.texture,
        })
    }

    /// Its fabric's look in the 3D view, the print over it.
    pub(crate) fn material(&self, project: &r3d_core::Project) -> r3d_render::Material {
        let garment = project.garment_of(self.panel);
        let material = r3d_render::Material::fabric(
            garment.map(|g| g.fabric).unwrap_or_default(),
            garment.and_then(|g| g.color.as_deref()),
        );
        match &self.print {
            Some(p) => material.printed(p.mean_rgb),
            None => material,
        }
    }

    /// Its fabric's surface in photos, coloured as the print on average.
    pub(crate) fn surface(&self, project: &r3d_core::Project) -> r3d_render::photo::Surface {
        let garment = project.garment_of(self.panel);
        let mean = self.print.as_ref().map(|p| format!("#{:06X}", p.mean_rgb));
        r3d_render::photo::Surface::fabric(
            garment.map(|g| g.fabric).unwrap_or_default(),
            mean.as_deref().or(garment.and_then(|g| g.color.as_deref())),
        )
    }
}

/// The garments' prints as the board fills pieces with them.
pub(crate) fn board_prints(
    prints: &std::collections::HashMap<
        r3d_core::GarmentId,
        std::sync::Arc<r3d_commands::prints::PrintImage>,
    >,
) -> Vec<r3d_render::board::BoardPrint> {
    prints
        .iter()
        .map(|(garment, p)| r3d_render::board::BoardPrint {
            garment: *garment,
            key: p.key,
            texture: p.texture.clone(),
            aspect: p.aspect,
        })
        .collect()
}
