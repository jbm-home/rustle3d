//! 2D geometry for Rustle3D.
//!
//! Pure math, no document or GPU knowledge: points and vectors, cubic Bézier
//! curves, polylines, offsets, polygon operations, and the conversion of a
//! closed panel contour into a triangle mesh with controllable particle
//! spacing (used by the simulation).
//!
//! Units: millimetres, `f64`. See `docs/architecture.md`.
