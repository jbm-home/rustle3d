//! What the engine keeps of the loaded simulation, outside the bridge's
//! public API (the code generator exposes everything in `api`).

use std::time::{Duration, Instant};

use r3d_core::{EdgeId, PanelId};

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
}

/// A cloth particle under the cursor.
pub(crate) struct Hit {
    pub particle: usize,
    pub position: glam::Vec3,
    pub panel: Option<PanelId>,
}
