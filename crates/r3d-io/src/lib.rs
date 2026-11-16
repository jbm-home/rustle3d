//! Import/export through open formats only.
//!
//! M2: patterns as SVG and PDF at 1:1 scale (PDF tiled on A4/Letter or one
//! large page). Both render the same format-independent [`drawing::Drawing`],
//! so they always agree. M9: draped garments as glTF (GLB) and OBJ, from the
//! same format-independent [`scene3d::Scene3d`]. Later: DXF-AAMA. This crate
//! does no file I/O: it returns bytes, and the caller decides where they go.

pub mod drawing;
pub mod gltf;
pub mod obj;
pub mod pdf;
pub mod scene3d;
pub mod svg;

pub use drawing::{Drawing, pattern_drawing};
