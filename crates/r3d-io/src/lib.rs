//! Import/export through open formats only.
//!
//! M2: patterns as SVG and PDF at 1:1 scale (PDF tiled on A4/Letter or one
//! large page). Both render the same format-independent [`drawing::Drawing`],
//! so they always agree. Later: OBJ/glTF (avatars, draped garments) and
//! DXF-AAMA. This crate does no file I/O: it returns bytes, and the caller
//! decides where they go.

pub mod drawing;
pub mod pdf;
pub mod svg;

pub use drawing::{Drawing, pattern_drawing};
