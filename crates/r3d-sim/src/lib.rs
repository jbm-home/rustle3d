//! Cloth simulation for Rustle3D.
//!
//! XPBD with small steps ([`solver`]): anisotropic membrane (warp, weft,
//! shear) and dihedral bending per triangle pair, seams, pins, gravity,
//! collisions with a static body ([`collide`]) and the ground, friction,
//! self-collision, and thin contacts between garments ([`layers`]). A scene is compiled once into a [`Model`]; backends
//! implement [`Solver`] on it. [`CpuSolver`] is the reference backend and
//! is deterministic: the same scene always gives the same result.
//!
//! Units: metres, kilograms, seconds; `f32`; Y up. See `docs/physics.md`
//! for the method and the papers it comes from.

pub mod collide;
mod cpu;
pub mod fabric;
pub mod layers;
pub mod model;
pub mod motion;
pub mod scene;
pub mod selfcollide;
pub mod solver;

pub use cpu::CpuSolver;
pub use fabric::{FabricParams, FabricPreset};
pub use model::Model;
pub use scene::{Closure, SimScene};
pub use solver::{SUBSTEPS, Solver, StepError, StepStats};
