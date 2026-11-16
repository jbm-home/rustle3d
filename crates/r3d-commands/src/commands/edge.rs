use r3d_core::measure::{EdgeGeometry, LENGTH_TOLERANCE_MM};
use r3d_core::{Edge, EdgeCurve, EdgeId, Notch, Panel, Point, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use super::{Vec2, editable_edge_panel, move_points, not_sewn, v};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Shape to give an edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CurveSpec {
    /// Straight segment.
    Line,
    /// Cubic Bézier; control points in absolute board coordinates (mm).
    Cubic { c1: Vec2, c2: Vec2 },
}

/// Turn an edge into a straight line or a cubic curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeSetCurve {
    pub edge: EdgeId,
    pub curve: CurveSpec,
}

impl Described for EdgeSetCurve {
    const SUMMARY: &'static str =
        "Make an edge a straight line or a cubic Bézier curve (absolute control points in mm).";
}

impl CommandSpec for EdgeSetCurve {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let (edge, _) = editable_edge_panel(tx, self.edge)?;
        let curve = match &self.curve {
            CurveSpec::Line => EdgeCurve::Line,
            CurveSpec::Cubic { c1, c2 } => {
                finite("curve", &[c1[0], c1[1], c2[0], c2[1]])?;
                EdgeCurve::Cubic {
                    c1: v(*c1),
                    c2: v(*c2),
                }
            }
        };
        tx.put_edge(self.edge, Edge { curve, ..edge });
        Ok(CommandOutput::default())
    }
}

/// Insert a point on an edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeSplit {
    pub edge: EdgeId,
    /// Distance from the edge's start point, along the edge (mm), strictly
    /// inside the edge.
    pub at_mm: f64,
}

impl Described for EdgeSplit {
    const SUMMARY: &'static str = "Insert a point on an edge at a distance (mm) along it; the curve shape is kept. Returns the new point, then the new edge (the second half).";
}

impl CommandSpec for EdgeSplit {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("at_mm", &[self.at_mm])?;
        let (edge, panel) = editable_edge_panel(tx, self.edge)?;
        not_sewn(tx, self.edge)?;
        let geom = tx
            .project()
            .edge_geometry(self.edge)
            .ok_or_else(|| CommandError::not_found(self.edge))?;
        let len = geom.length();
        if !(self.at_mm > 0.0 && self.at_mm < len) {
            return Err(CommandError::invalid("at_mm", Invalid::OutsideEdge(len)));
        }
        let (pos, first, second) = match geom {
            EdgeGeometry::Line(a, b) => (
                a.lerp(b, self.at_mm / len),
                EdgeCurve::Line,
                EdgeCurve::Line,
            ),
            EdgeGeometry::Cubic(c) => {
                let (l, r) = c.split(c.param_at_length(self.at_mm, LENGTH_TOLERANCE_MM));
                (
                    l.p3,
                    EdgeCurve::Cubic { c1: l.p1, c2: l.p2 },
                    EdgeCurve::Cubic { c1: r.p1, c2: r.p2 },
                )
            }
        };
        let point = tx.new_point_id();
        tx.put_point(
            point,
            Point {
                panel: edge.panel,
                pos,
            },
        );
        let new_edge = tx.new_edge_id();
        tx.put_edge(
            new_edge,
            Edge {
                panel: edge.panel,
                start: point,
                end: edge.end,
                curve: second,
            },
        );
        tx.put_edge(
            self.edge,
            Edge {
                end: point,
                curve: first,
                ..edge.clone()
            },
        );
        let mut contour = panel.contour.clone();
        let at = contour
            .iter()
            .position(|e| *e == self.edge)
            .unwrap_or(contour.len());
        contour.insert((at + 1).min(contour.len()), new_edge);
        tx.put_panel(edge.panel, Panel { contour, ..panel });

        // Notches past the split point move to the second half.
        let moved: Vec<_> = tx
            .project()
            .notches()
            .iter()
            .filter(|(_, n)| n.edge == self.edge && n.offset_mm > self.at_mm)
            .map(|(id, n)| (*id, n.offset_mm))
            .collect();
        for (id, offset) in moved {
            tx.put_notch(
                id,
                Notch {
                    edge: new_edge,
                    offset_mm: offset - self.at_mm,
                },
            );
        }
        Ok(CommandOutput {
            created: vec![point.into(), new_edge.into()],
        })
    }
}

/// Move an edge: both its end points, by the same offset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeMove {
    pub edge: EdgeId,
    /// Offset in mm.
    pub by: Vec2,
}

impl Described for EdgeMove {
    const SUMMARY: &'static str = "Move an edge by an offset in mm: both its end points move, with the curve handles attached to them, so the neighbouring edges stretch to follow. A sewn edge's partner does not move (the seam may then mismatch; `seam.list` reports it). The panel may end up self-intersecting; `panel.get` reports it.";
}

impl CommandSpec for EdgeMove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("by", &self.by)?;
        let (edge, _) = editable_edge_panel(tx, self.edge)?;
        let by = v(self.by);
        move_points(tx, &BTreeMap::from([(edge.start, by), (edge.end, by)]));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("edge.move:{}", self.edge))
    }
}
