use r3d_core::{Edge, EdgeCurve, Point, PointId, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, v};
use crate::error::{CommandError, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

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
        let point = tx
            .project()
            .point(self.point)
            .cloned()
            .ok_or_else(|| CommandError::not_found(self.point))?;
        let to = v(self.to);
        let d = to - point.pos;
        // Keep the handles attached to this point rigidly attached.
        let attached: Vec<_> = tx
            .project()
            .edges()
            .iter()
            .filter(|(_, e)| {
                e.panel == point.panel && (e.start == self.point || e.end == self.point)
            })
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
