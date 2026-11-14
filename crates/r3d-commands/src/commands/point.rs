use r3d_core::{Edge, EdgeCurve, Notch, Panel, Point, PointId, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, editable_panel, v};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

fn get_point(tx: &Transaction<'_>, id: PointId) -> Result<Point, CommandError> {
    tx.project()
        .point(id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))
}

/// Move a contour point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointMove {
    pub point: PointId,
    /// New position in mm.
    pub to: Vec2,
}

impl Described for PointMove {
    const SUMMARY: &'static str = "Move a contour point to a new position in mm. Curve handles attached to the point move with it. The panel may end up self-intersecting; `panel.get` reports it.";
}

impl CommandSpec for PointMove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("to", &self.to)?;
        let point = get_point(tx, self.point)?;
        editable_panel(tx, point.panel)?;
        let to = v(self.to);
        let d = to - point.pos;
        // Keep the handles attached to this point rigidly attached.
        let attached: Vec<_> = tx
            .project()
            .edges()
            .iter()
            .filter(|(_, e)| e.start == self.point || e.end == self.point)
            .map(|(id, e)| (*id, e.clone()))
            .collect();
        for (id, edge) in attached {
            if let EdgeCurve::Cubic { mut c1, mut c2 } = edge.curve {
                if edge.start == self.point {
                    c1 += d;
                }
                if edge.end == self.point {
                    c2 += d;
                }
                tx.put_edge(
                    id,
                    Edge {
                        curve: EdgeCurve::Cubic { c1, c2 },
                        ..edge
                    },
                );
            }
        }
        tx.put_point(self.point, Point { pos: to, ..point });
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("point.move:{}", self.point))
    }
}

/// Remove a contour point, joining its two edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointRemove {
    pub point: PointId,
}

impl Described for PointRemove {
    const SUMMARY: &'static str = "Remove a contour point and join its two edges into one (a curve if either was curved, keeping the outer handles). The panel must keep at least 3 points.";
}

impl CommandSpec for PointRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let point = get_point(tx, self.point)?;
        let panel = editable_panel(tx, point.panel)?;
        if panel.contour.len() <= 3 {
            return Err(CommandError::invalid("point", Invalid::TooFewPoints(3)));
        }
        let p = tx.project();
        let find = |pred: &dyn Fn(&Edge) -> bool| {
            panel
                .contour
                .iter()
                .filter_map(|id| p.edge(*id).map(|e| (*id, e.clone())))
                .find(|(_, e)| pred(e))
                .ok_or_else(|| CommandError::Internal {
                    detail: format!("{} is not on its panel contour", self.point),
                })
        };
        let (a_id, a) = find(&|e| e.end == self.point)?;
        let (b_id, b) = find(&|e| e.start == self.point)?;
        let a_len = p.edge_length(a_id).unwrap_or_default();
        let pos = |id| p.point(id).map(|pt| pt.pos).unwrap_or_default();
        let curve = match (&a.curve, &b.curve) {
            (EdgeCurve::Line, EdgeCurve::Line) => EdgeCurve::Line,
            (ac, bc) => EdgeCurve::Cubic {
                c1: match ac {
                    EdgeCurve::Cubic { c1, .. } => *c1,
                    EdgeCurve::Line => pos(a.start),
                },
                c2: match bc {
                    EdgeCurve::Cubic { c2, .. } => *c2,
                    EdgeCurve::Line => pos(b.end),
                },
            },
        };
        let moved: Vec<_> = p
            .notches()
            .iter()
            .filter(|(_, n)| n.edge == b_id)
            .map(|(id, n)| (*id, n.offset_mm))
            .collect();
        for (id, offset) in moved {
            tx.put_notch(
                id,
                Notch {
                    edge: a_id,
                    offset_mm: offset + a_len,
                },
            );
        }
        tx.put_edge(
            a_id,
            Edge {
                end: b.end,
                curve,
                ..a
            },
        );
        tx.remove_edge(b_id);
        tx.remove_point(self.point);
        let contour = panel
            .contour
            .iter()
            .copied()
            .filter(|e| *e != b_id)
            .collect();
        tx.put_panel(point.panel, Panel { contour, ..panel });
        Ok(CommandOutput::default())
    }
}
