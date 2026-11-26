//! What a view's frame is drawn from. A view whose key has not changed
//! since its last frame is not drawn again: an idle app costs nothing.

use glam::Vec3;
use r3d_commands::fit::FitMap;
use r3d_core::{EdgeId, GarmentId, PanelId};
use r3d_render::board::BoardView;
use r3d_render::{OrbitCamera, Theme};

/// Every input of a frame: the same key, the same picture.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FrameKey {
    /// The document's revision.
    pub revision: u64,
    pub size: (u32, u32),
    pub theme: Theme,
    pub selected: Vec<PanelId>,
    pub edges: Vec<EdgeId>,
    /// Each garment's print, by the key that changes with its pixels.
    pub prints: Vec<(GarmentId, u64)>,
    pub shown: Shown,
}

/// What only one kind of view is drawn from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Shown {
    Board {
        /// `None` fits the pattern (which the revision covers).
        view: Option<BoardView>,
        sizes: bool,
    },
    Garment {
        camera: OrbitCamera,
        /// The simulation's publication serial: the cloth, the time (so
        /// the moving body), the loaded drape.
        sim: u64,
        /// The fitted avatar drawn (`FormKey::id`), `None` while fitting.
        avatar: Option<u64>,
        fit_map: Option<FitMap>,
        tape: Option<Vec<(Vec3, Vec3)>>,
        /// A finished photo shown, by its samples.
        photo: Option<u32>,
    },
}
