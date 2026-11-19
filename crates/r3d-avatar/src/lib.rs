//! Parametric body model: an avatar built from body measurements.
//!
//! The geometry (a base body mesh and morph targets) comes from MakeHuman's
//! assets, released under CC0 (see `assets/avatars/README.md`). Only the
//! data is used: the model, the measurement method and the fitting below
//! are our own (clean-room; MakeHuman's code is AGPL and is not used).
//!
//! - [`pack`]: the compact binary body pack (`assets/avatars/*.r3dbody`).
//! - [`model`]: shape parameters → vertex positions.
//! - [`measure`]: stature and girths, measured like a tailor would.
//! - [`fit`]: the shape that matches given measurements.
//! - [`presets`]: built-in sizes and the user's own presets.
//! - [`skeleton`]: the body's bones, poses and skinning.
//! - [`motion`]: walking, turning, raising the arms, captured motions.
//! - [`bvh`], [`captured`]: motions captured on real people, from BVH
//!   files, played by our skeleton.
//! - [`motion_library`]: the user's captured motions, shared by every
//!   project.

pub mod bvh;
pub mod captured;
pub mod fit;
pub mod measure;
pub mod model;
pub mod motion;
pub mod motion_library;
pub mod pack;
pub mod presets;
pub mod skeleton;
