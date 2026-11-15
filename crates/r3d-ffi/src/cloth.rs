//! What the engine keeps of the loaded simulation, outside the bridge's
//! public API (the code generator exposes everything in `api`).

use std::collections::HashMap;

use r3d_commands::drape::Piece;
use r3d_core::{PanelId, PinId};

/// What the engine knows of the loaded simulation.
#[derive(Default)]
pub(crate) struct Cloth {
    pub pieces: Vec<Piece>,
    /// Each particle's position on its pattern (m).
    pub rest: Vec<glam::Vec2>,
    /// The particles each pin holds.
    pub pins: HashMap<PinId, Vec<usize>>,
}

/// Pick radius on the cloth (physical pixels).
pub(crate) const PICK_PX: f32 = 24.0;

/// Cloth under a pressed pointer. The grab starts on the first move, so a
/// simple click only selects.
#[derive(Clone, Copy)]
pub(crate) struct Held {
    pub particle: usize,
    /// Where it was when pressed: the pulling plane goes through it.
    pub anchor: glam::Vec3,
    pub pulling: bool,
}

/// A cloth particle under the cursor.
pub(crate) struct Hit {
    pub particle: usize,
    pub position: glam::Vec3,
    pub panel: Option<PanelId>,
}
