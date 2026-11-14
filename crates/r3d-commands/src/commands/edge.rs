use r3d_core::{Edge, EdgeCurve, EdgeId, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, v};
use crate::error::{CommandError, finite};
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
        let edge = tx
            .project()
            .edge(self.edge)
            .cloned()
            .ok_or_else(|| CommandError::not_found(self.edge))?;
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
