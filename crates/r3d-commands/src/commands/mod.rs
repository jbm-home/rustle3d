//! Command implementations, grouped by entity.

mod avatar;
mod edge;
mod garment;
mod internal_line;
mod mirror;
mod notch;
mod panel;
mod pin;
mod point;
mod project;
mod seam;

pub use avatar::*;
pub use edge::*;
pub use garment::*;
pub use internal_line::*;
pub use mirror::*;
pub use notch::*;
pub use panel::*;
pub use pin::*;
pub use point::*;
pub use project::*;
pub use seam::*;

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

/// Refuse to change the shape of a sewn edge (seams are whole edges).
pub(crate) fn not_sewn(tx: &Transaction<'_>, edge: EdgeId) -> Result<(), CommandError> {
    match tx.project().seam_of(edge) {
        Some(seam) => Err(CommandError::Sewn {
            edge: edge.into(),
            seam: seam.into(),
        }),
        None => Ok(()),
    }
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
