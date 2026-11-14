//! Command implementations, grouped by entity.

mod edge;
mod internal_line;
mod mirror;
mod notch;
mod panel;
mod point;
mod project;

pub use edge::*;
pub use internal_line::*;
pub use mirror::*;
pub use notch::*;
pub use panel::*;
pub use point::*;
pub use project::*;

use r3d_core::{EdgeId, Panel, PanelId, Project, Transaction};
use r3d_geom::DVec2;

use crate::error::CommandError;

/// A 2D position or vector in millimetres, as `[x, y]`.
pub type Vec2 = [f64; 2];

pub(crate) fn v(p: Vec2) -> DVec2 {
    DVec2::new(p[0], p[1])
}

pub(crate) fn get_panel(p: &Project, id: PanelId) -> Result<Panel, CommandError> {
    p.panel(id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))
}

/// A panel whose geometry may be edited directly: not a linked mirror copy.
pub(crate) fn editable_panel(tx: &Transaction<'_>, id: PanelId) -> Result<Panel, CommandError> {
    let panel = get_panel(tx.project(), id)?;
    if panel.mirror_of.is_some() {
        return Err(CommandError::LinkedMirror { panel: id.into() });
    }
    Ok(panel)
}

/// The panel owning `edge`, if editable.
pub(crate) fn editable_edge_panel(
    tx: &Transaction<'_>,
    edge: EdgeId,
) -> Result<(r3d_core::Edge, Panel), CommandError> {
    let e = tx
        .project()
        .edge(edge)
        .cloned()
        .ok_or_else(|| CommandError::not_found(edge))?;
    let panel = editable_panel(tx, e.panel)?;
    Ok((e, panel))
}
