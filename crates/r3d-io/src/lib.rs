//! Import/export through open formats only.
//!
//! M2: patterns as SVG and PDF at 1:1 scale (PDF tiled on A4/Letter or one
//! large page). Both render the same format-independent [`drawing::Drawing`],
//! so they always agree. M9: draped garments as glTF (GLB) and OBJ, from the
//! same format-independent [`scene3d::Scene3d`]. M13: patterns as DXF-AAMA
//! ([`dxf`], both ways), and pattern import from DXF, SVG and PDF
//! ([`import`]). M30: the marker, pieces laid out on the fabric
//! ([`marker`]) and the tech pack ([`techpack`]). This crate does no file I/O: it takes and returns bytes,
//! and the caller decides where they come from and go.

pub mod drawing;
pub mod dxf;
pub mod gltf;
pub mod import;
pub mod marker;
pub mod obj;
pub mod pdf;
pub mod scene3d;
pub mod svg;
pub mod techpack;
pub mod texture;

pub use drawing::{Drawing, graded_drawing, pattern_drawing, pieces_drawing};
