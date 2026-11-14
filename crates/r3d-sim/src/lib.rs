//! Cloth simulation for Rustle3D.
//!
//! An XPBD solver on the CPU (parallelized with rayon) behind a `Solver`
//! trait, so that GPU or alternative methods (e.g. Vertex Block Descent) can
//! be added later. Includes avatar collision (SDF), self-collision (spatial
//! hashing) and a deterministic mode for regression tests.
//!
//! Units: metres, `f32`, Y-up. See `docs/physics.md`.
