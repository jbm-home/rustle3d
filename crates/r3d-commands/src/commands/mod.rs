//! Command implementations, grouped by entity.

mod edge;
mod panel;
mod point;
mod project;

pub use edge::*;
pub use panel::*;
pub use point::*;
pub use project::*;

use r3d_geom::DVec2;

/// A 2D position or vector in millimetres, as `[x, y]`.
pub type Vec2 = [f64; 2];

pub(crate) fn v(p: Vec2) -> DVec2 {
    DVec2::new(p[0], p[1])
}
