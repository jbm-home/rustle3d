//! 2D geometry for Rustle3D.
//!
//! Pure math, no document or GPU knowledge: points and vectors, cubic Bézier
//! curves, curves fitted to digitised points ([`fit`]), polylines and
//! polygon operations (offsets for seam allowances), and the conversion of
//! a closed panel contour into a triangle mesh with controllable particle
//! spacing (used by the simulation).
//!
//! Units: millimetres, `f64`. See `docs/architecture.md`.

pub mod bezier;
pub mod boolean;
pub mod fit;
pub mod mesh;
pub mod polygon;
pub mod solve;
pub mod transform;

pub use bezier::CubicBez;
pub use glam::DVec2;
pub use polygon::Aabb;
pub use transform::Mirror;
