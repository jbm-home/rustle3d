//! 2D geometry for Rustle3D.
//!
//! Pure math, no document or GPU knowledge: points and vectors, cubic Bézier
//! curves, curves fitted to digitised points ([`fit`]), polylines and
//! polygon operations (offsets for seam allowances), and the conversion of
//! a closed panel contour into a triangle mesh with controllable particle
//! spacing (used by the simulation), and pieces nested on a strip of
//! fabric ([`nest`], a marker). [`rigid`] holds the one 3D type: the
//! rigid motion of a body's bone (metres, `f32`); [`trimesh`] and
//! [`winding`] check a 3D surface (an imported body) and tell its inside
//! from its outside.
//!
//! Units: millimetres, `f64`. See `docs/architecture.md`.

pub mod bezier;
pub mod boolean;
pub mod fit;
pub mod mesh;
pub mod nest;
pub mod polygon;
pub mod rigid;
pub mod solve;
pub mod transform;
pub mod trimesh;
pub mod winding;

pub use bezier::CubicBez;
pub use glam::DVec2;
pub use polygon::Aabb;
pub use rigid::Rigid;
pub use transform::Mirror;
