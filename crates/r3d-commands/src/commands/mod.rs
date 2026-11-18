//! Command implementations, grouped by entity.

mod arrange;
mod avatar;
mod clip;
mod constraint;
mod cut;
mod detail;
mod edge;
mod garment;
mod grading;
mod hole;
mod import;
mod internal_line;
mod lengthen;
mod mirror;
mod notch;
mod panel;
mod pin;
mod point;
mod project;
mod seam;
mod set_length;
mod shape;
mod symmetry;
mod take_in;
mod template;

pub use arrange::*;
pub use avatar::*;
pub use clip::*;
pub use constraint::*;
pub use cut::*;
pub use detail::*;
pub use edge::*;
pub use garment::*;
pub use grading::*;
pub use hole::*;
pub use import::*;
pub use internal_line::*;
pub use lengthen::*;
pub use mirror::*;
pub use notch::*;
pub use panel::*;
pub use pin::*;
pub use point::*;
pub use project::*;
pub use seam::*;
pub use set_length::*;
pub use shape::*;
pub use symmetry::*;
pub use take_in::*;
pub use template::*;

use std::collections::{BTreeMap, BTreeSet};

use r3d_core::{Edge, EdgeCurve, EdgeId, Panel, PanelId, Point, PointId, Project, Transaction};
use r3d_geom::DVec2;

use crate::error::{CommandError, Invalid};

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
    if tx.project().is_derived_edge(edge) {
        return Err(CommandError::Derived { id: edge.into() });
    }
    Ok((e, panel))
}

/// A point whose position may be edited directly: not derived from other
/// geometry.
pub(crate) fn editable_point(tx: &Transaction<'_>, id: PointId) -> Result<Point, CommandError> {
    let point = tx
        .project()
        .point(id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))?;
    editable_panel(tx, point.panel)?;
    if tx.project().is_derived_point(id) {
        return Err(CommandError::Derived { id: id.into() });
    }
    Ok(point)
}

/// Move contour points by the given offsets (mm); curve handles stay
/// attached to their points.
pub(crate) fn move_points(tx: &mut Transaction<'_>, moves: &BTreeMap<PointId, DVec2>) {
    let p = tx.project();
    let edges: Vec<(EdgeId, Edge)> = p
        .edges()
        .iter()
        .filter(|(_, e)| moves.contains_key(&e.start) || moves.contains_key(&e.end))
        .map(|(id, e)| (*id, e.clone()))
        .collect();
    let points: Vec<(PointId, Point)> = moves
        .keys()
        .filter_map(|id| Some((*id, p.point(*id)?.clone())))
        .collect();
    let at = |pt: PointId| moves.get(&pt).copied().unwrap_or_default();
    for (id, edge) in edges {
        if let EdgeCurve::Cubic { c1, c2 } = edge.curve {
            let curve = EdgeCurve::Cubic {
                c1: c1 + at(edge.start),
                c2: c2 + at(edge.end),
            };
            tx.put_edge(id, Edge { curve, ..edge });
        }
    }
    for (id, point) in points {
        tx.put_point(
            id,
            Point {
                pos: point.pos + at(id),
                ..point
            },
        );
    }
}

/// Whether each panel's outline is simple and counter-clockwise (right
/// way out).
pub(crate) fn outline_states(
    p: &Project,
    panels: &BTreeSet<PanelId>,
) -> BTreeMap<PanelId, (bool, bool)> {
    panels
        .iter()
        .map(|id| {
            let outline = p.panel_outline(*id).unwrap_or_default();
            let simple = r3d_geom::polygon::is_simple(&outline);
            (
                *id,
                (simple, r3d_geom::polygon::signed_area(&outline) > 0.0),
            )
        })
        .collect()
}

/// Refuse a change that made a panel cross itself or turn inside out
/// (compared with `before`, from [`outline_states`]; a panel that was
/// already invalid can still be edited, e.g. to fix it).
pub(crate) fn check_outlines(
    tx: &Transaction<'_>,
    before: &BTreeMap<PanelId, (bool, bool)>,
    field: &'static str,
) -> Result<(), CommandError> {
    let panels = before.keys().copied().collect();
    for (id, (simple, right_way)) in outline_states(tx.project(), &panels) {
        let (was_simple, was_right_way) = before.get(&id).copied().unwrap_or((false, false));
        if was_simple && !simple {
            return Err(CommandError::invalid(field, Invalid::NotSimple));
        }
        if was_right_way && !right_way {
            return Err(CommandError::invalid(field, Invalid::Inverted));
        }
    }
    Ok(())
}

/// The centre of a panel's outline (mm).
pub(crate) fn centre(p: &Project, panel: PanelId) -> DVec2 {
    let outline = p.panel_outline(panel).unwrap_or_default();
    outline.iter().fold(DVec2::ZERO, |a, b| a + *b) / outline.len().max(1) as f64
}
