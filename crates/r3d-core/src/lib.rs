//! Document model for Rustle3D.
//!
//! Projects, garments, panels, points and edges, with stable typed IDs and
//! versioned JSON serialization (schema version + migrations). No GUI, no
//! GPU, no simulation.
//!
//! The document can only be modified through an [`edit::Transaction`], which
//! records every entity change so that undo/redo is generic and cannot drift
//! from what a command actually did. See `docs/architecture.md` §3–4.

pub mod edit;
pub mod file;
pub mod id;
pub mod integrity;
pub mod measure;
pub mod message;
pub mod model;
pub mod project_dir;

pub use edit::{ChangeSet, Transaction};
pub use id::{AnyId, EdgeId, GarmentId, PanelId, PointId};
pub use message::{Localizable, Message};
pub use model::{DisplayUnit, Edge, EdgeCurve, Garment, Panel, Point, Project};
