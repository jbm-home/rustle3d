//! Give an edge an exact length, typed in the inspector
//! (docs/specs/13-3d-to-2d-editing.md).

use std::collections::{BTreeMap, BTreeSet};

use r3d_core::{Edge, EdgeCurve, EdgeId, PointId, Project, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::mirror::{mirror_twin, mirrored_source};
use super::{check_outlines, move_points, outline_states};
use crate::error::{CommandError, Invalid, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Scale an edge about its middle to a given length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeSetLength {
    pub edge: EdgeId,
    /// The new length along the edge (mm).
    pub length_mm: f64,
}

impl Described for EdgeSetLength {
    const SUMMARY: &'static str = "Give an edge a new length (mm): it is scaled about its middle, curve included, so its end points move along it. A sewn edge's partner is scaled in the same ratio so the seam stays matched, and so are the symmetric seams of linked mirror copies (which follow their source).";
}

/// The edge itself, if it is on an ordinary panel, or the source edge it
/// mirrors.
fn source(p: &Project, edge: EdgeId) -> EdgeId {
    mirrored_source(p, edge).map_or(edge, |(src, _)| src)
}

/// The other side of the seam `edge` is on, if it is sewn.
fn partner(p: &Project, edge: EdgeId) -> Option<EdgeId> {
    let seam = p.seams().get(&p.seam_of(edge)?)?;
    Some(if seam.a.edge == edge {
        seam.b.edge
    } else {
        seam.a.edge
    })
}

/// The source edges to scale with `start` so that every seam stays
/// matched: across seams, and across linked mirrors.
fn linked_edges(p: &Project, start: EdgeId) -> BTreeSet<EdgeId> {
    let mut found = BTreeSet::new();
    let mut todo = vec![start];
    while let Some(edge) = todo.pop() {
        let edge = source(p, edge);
        if !found.insert(edge) {
            continue;
        }
        let twin = mirror_twin(p, edge).map(|(twin, _)| twin);
        todo.extend(
            [Some(edge), twin]
                .into_iter()
                .flatten()
                .filter_map(|e| partner(p, e)),
        );
    }
    found
}

impl CommandSpec for EdgeSetLength {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        positive("length_mm", self.length_mm)?;
        let p = tx.project();
        let current = p
            .edge_length(self.edge)
            .ok_or_else(|| CommandError::not_found(self.edge))?;
        if current <= 0.0 {
            return Err(CommandError::invalid("edge", Invalid::NotPositive));
        }
        let ratio = self.length_mm / current;

        let mut moves: BTreeMap<PointId, DVec2> = BTreeMap::new();
        let mut scaled: Vec<(EdgeId, Edge)> = Vec::new();
        let mut panels = BTreeSet::new();
        for id in linked_edges(p, self.edge) {
            let Some(edge) = p.edge(id).cloned() else {
                continue;
            };
            let [a, b] =
                [edge.start, edge.end].map(|pt| p.point(pt).map(|x| x.pos).unwrap_or_default());
            let middle = (a + b) / 2.0;
            let scale = |x: DVec2| middle + (x - middle) * ratio;
            moves.insert(edge.start, scale(a) - a);
            moves.insert(edge.end, scale(b) - b);
            let curve = match edge.curve {
                EdgeCurve::Line => EdgeCurve::Line,
                EdgeCurve::Cubic { c1, c2 } => EdgeCurve::Cubic {
                    c1: scale(c1),
                    c2: scale(c2),
                },
            };
            panels.insert(edge.panel);
            scaled.push((id, Edge { curve, ..edge }));
        }

        let before = outline_states(tx.project(), &panels);
        // Moving the end points carries the neighbouring edges' control
        // points along; the scaled edges then get their own shape.
        move_points(tx, &moves);
        for (id, edge) in scaled {
            tx.put_edge(id, edge);
        }
        check_outlines(tx, &before, "length_mm")?;
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("edge.set_length:{}", self.edge))
    }
}
