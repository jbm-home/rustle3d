//! Cloth simulation for Rustle3D.
//!
//! XPBD with small steps ([`solver`]): anisotropic membrane (warp, weft,
//! shear) and dihedral bending per triangle pair, seams, pins, gravity,
//! collisions with a static body ([`collide`]) and the ground, friction.
//! Deterministic: the same scene always gives the same result.
//!
//! Units: metres, kilograms, seconds; `f32`; Y up. See `docs/physics.md`
//! for the method and the papers it comes from.

pub mod collide;
pub mod fabric;
pub mod scene;
pub mod solver;

pub use fabric::{FabricParams, FabricPreset};
pub use scene::SimScene;
pub use solver::Solver;
