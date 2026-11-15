use r3d_core::{EdgeId, Seam, SeamId, SeamSide, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::not_sewn;
use crate::error::{CommandError, Invalid};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// One side of a new seam.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamSideSpec {
    pub edge: EdgeId,
    /// Walk this edge from its end point instead of its start. Omitted:
    /// `false` for side `a`; for side `b`, the direction that walks both
    /// sides the same way on the pattern (what sewing two panels laid side
    /// by side needs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversed: Option<bool>,
}

/// Sew two edges together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamAdd {
    pub a: SeamSideSpec,
    pub b: SeamSideSpec,
}

impl Described for SeamAdd {
    const SUMMARY: &'static str = "Sew two whole edges together. Points are matched along the edges, start to start unless a side is reversed; leave b.reversed out to have it chosen (both sides walked the same way on the pattern), and use seam.flip if the seam looks twisted. An edge can be in one seam only. Returns the seam.";
}

impl CommandSpec for SeamAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        for side in [&self.a, &self.b] {
            if tx.project().edge(side.edge).is_none() {
                return Err(CommandError::not_found(side.edge));
            }
            not_sewn(tx, side.edge)?;
        }
        if self.a.edge == self.b.edge {
            return Err(CommandError::invalid("b", Invalid::SamePoint));
        }
        let a_reversed = self.a.reversed.unwrap_or(false);
        let b_reversed = match self.b.reversed {
            Some(r) => r,
            None => {
                let p = tx.project();
                let chord = |e: EdgeId| -> Option<r3d_geom::DVec2> {
                    let edge = p.edge(e)?;
                    Some(p.point(edge.end)?.pos - p.point(edge.start)?.pos)
                };
                let walked_a =
                    chord(self.a.edge).unwrap_or_default() * if a_reversed { -1.0 } else { 1.0 };
                chord(self.b.edge).unwrap_or_default().dot(walked_a) < 0.0
            }
        };
        let id = tx.new_seam_id();
        tx.put_seam(
            id,
            Seam {
                a: SeamSide {
                    edge: self.a.edge,
                    reversed: a_reversed,
                },
                b: SeamSide {
                    edge: self.b.edge,
                    reversed: b_reversed,
                },
            },
        );
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

fn get_seam(tx: &Transaction<'_>, id: SeamId) -> Result<Seam, CommandError> {
    tx.project()
        .seams()
        .get(&id)
        .copied()
        .ok_or_else(|| CommandError::not_found(id))
}

/// Unpick a seam.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamRemove {
    pub seam: SeamId,
}

impl Described for SeamRemove {
    const SUMMARY: &'static str = "Remove a seam (the edges are no longer sewn).";
}

impl CommandSpec for SeamRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_seam(tx, self.seam)?;
        tx.remove_seam(self.seam);
        Ok(CommandOutput::default())
    }
}

/// Flip the direction of a seam's second side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamFlip {
    pub seam: SeamId,
}

impl Described for SeamFlip {
    const SUMMARY: &'static str =
        "Reverse how a seam matches its two edges (fixes a twisted seam).";
}

impl CommandSpec for SeamFlip {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let mut seam = get_seam(tx, self.seam)?;
        seam.b.reversed = !seam.b.reversed;
        tx.put_seam(self.seam, seam);
        Ok(CommandOutput::default())
    }
}
