//! Lengthen or shorten a garment at an open edge (hem, sleeve end,
//! neckline): the 3D *Length* tool (docs/specs/13-3d-to-2d-editing.md).

use std::collections::{BTreeMap, BTreeSet};

use r3d_core::{EdgeId, PanelId, PointId, Project, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{centre, check_outlines, move_points, not_sewn, outline_states};
use crate::error::{CommandError, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Move an open edge, and the line it continues across seams, along the
/// grain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeLengthen {
    /// An open (unsewn) edge: a hem, a sleeve end, a neckline.
    pub edge: EdgeId,
    /// How far to move it (mm): positive makes the garment longer.
    pub mm: f64,
}

impl Described for EdgeLengthen {
    const SUMMARY: &'static str = "Lengthen (mm > 0) or shorten (mm < 0) the garment at an open edge such as a hem: the edge moves along its panel's grain, and so do the edges it continues into across seams (the whole hemline), so sewn sides stay matched. Linked mirror copies follow their source.";
}

/// The other contour edge of `panel` at `point`.
fn neighbour(p: &Project, panel: PanelId, point: PointId, not: EdgeId) -> Option<EdgeId> {
    p.panel(panel)?.contour.iter().copied().find(|e| {
        *e != not
            && p.edge(*e)
                .is_some_and(|edge| edge.start == point || edge.end == point)
    })
}

/// Across the seam of `side` (an edge at `point`): the matching point on
/// the other edge, and that edge.
fn across(p: &Project, side: EdgeId, point: PointId) -> Option<(PointId, EdgeId)> {
    let seam = p.seams().values().find(|s| s.touches(side))?;
    let ((_, mine_reversed), (other, other_reversed)) = seam.across(side)?;
    let edge = p.edge(side)?;
    // Seams pair the two sides walked from their starts (or ends if
    // reversed): the k-th point of one matches the k-th of the other.
    let first = (edge.start == point) != mine_reversed;
    let o = p.edge(other)?;
    let matched = if first != other_reversed {
        o.start
    } else {
        o.end
    };
    Some((matched, other))
}

/// From `current` at its end `point`: the edge the line continues into
/// across the seam there, and the matching point (where it starts).
fn next(p: &Project, current: EdgeId, point: PointId) -> Option<(EdgeId, PointId)> {
    let panel = p.edge(current)?.panel;
    let side = neighbour(p, panel, point, current)?;
    let (matched, other_side) = across(p, side, point)?;
    let other_panel = p.edge(other_side)?.panel;
    Some((neighbour(p, other_panel, matched, other_side)?, matched))
}

/// The line `edge` belongs to: itself, then the edges it continues into
/// across seams, in both directions, until the line closes or ends.
fn line(p: &Project, start: EdgeId) -> Vec<EdgeId> {
    let mut found = vec![start];
    let Some(first) = p.edge(start) else {
        return found;
    };
    for end in [first.end, first.start] {
        let (mut current, mut point) = (start, end);
        while let Some((edge, matched)) = next(p, current, point)
            && !found.contains(&edge)
            && let Some(e) = p.edge(edge)
        {
            found.push(edge);
            point = if e.start == matched { e.end } else { e.start };
            current = edge;
        }
    }
    found
}

impl CommandSpec for EdgeLengthen {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("mm", &[self.mm])?;
        if tx.project().edge(self.edge).is_none() {
            return Err(CommandError::not_found(self.edge));
        }
        not_sewn(tx, self.edge)?;
        let p = tx.project();
        let edges = line(p, self.edge);

        // Linked mirror copies follow their source: move the source edges.
        let mut moves: BTreeMap<PointId, DVec2> = BTreeMap::new();
        let mut panels = BTreeSet::new();
        for e in &edges {
            let Some(edge) = p.edge(*e) else { continue };
            let Some(panel) = p.panel(edge.panel) else {
                continue;
            };
            if panel.mirror_of.is_some() {
                continue;
            }
            // Outward along the grain: away from the panel's centre.
            let grain = DVec2::from_angle(panel.grain_angle_deg.to_radians());
            let centre = centre(p, edge.panel);
            let ends =
                [edge.start, edge.end].map(|id| p.point(id).map(|pt| pt.pos).unwrap_or_default());
            let side = ((ends[0] + ends[1]) / 2.0 - centre).dot(grain).signum();
            let d = grain * side * self.mm;
            moves.insert(edge.start, d);
            moves.insert(edge.end, d);
            panels.insert(edge.panel);
        }
        if moves.is_empty() {
            return Err(CommandError::LinkedMirror {
                panel: p
                    .edge(self.edge)
                    .map(|e| e.panel.into())
                    .ok_or_else(|| CommandError::not_found(self.edge))?,
            });
        }

        let before = outline_states(tx.project(), &panels);
        move_points(tx, &moves);
        check_outlines(tx, &before, "mm")?;
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("edge.lengthen:{}", self.edge))
    }
}
