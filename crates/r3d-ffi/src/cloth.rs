//! What the engine keeps of the loaded simulation, outside the bridge's
//! public API (the code generator exposes everything in `api`).

use std::time::{Duration, Instant};

use r3d_core::{EdgeId, PanelId, PointId, SeamId};

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
}

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
