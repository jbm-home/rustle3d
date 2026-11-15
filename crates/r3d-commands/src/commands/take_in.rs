//! Take in or let out a seam: the 3D *Fit* tool
//! (docs/specs/13-3d-to-2d-editing.md).

use std::collections::{BTreeMap, BTreeSet};

use r3d_core::{Edge, EdgeId, PointId, Project, SeamId, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::mirror::mirrored_source;
use super::{centre, check_outlines, move_points, outline_states};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Take in (mm > 0) or let out (mm < 0) a seam.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamTakeIn {
    pub seam: SeamId,
    /// Total width taken in (mm): each side moves half of it inward.
    /// Negative lets the seam out.
    pub mm: f64,
    /// An end point of the seam (on either side) to take in at that end
    /// only, tapering to nothing at the other end. Omitted: the whole seam.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<PointId>,
}

impl Described for SeamTakeIn {
    const SUMMARY: &'static str = "Take in (mm > 0) or let out (mm < 0) a seam: both sides move inward by half of mm, perpendicular to the seam, so they stay matched. With `at` (an end point of the seam) only that end moves, tapering the change (e.g. take in the waist only). With a linked mirror copy, the symmetric seam changes too.";
}

/// At the seam end where `at` is: the end point of each side, [a, b].
fn matched_ends(p: &Project, seam: SeamId, at: PointId) -> Option<[PointId; 2]> {
    let s = p.seams().get(&seam)?;
    let (a, b) = (p.edge(s.a.edge)?, p.edge(s.b.edge)?);
    // Is `at` the first point of its side's walk (or the last)?
    let first = if at == a.start || at == a.end {
        (at == a.start) != s.a.reversed
    } else if at == b.start || at == b.end {
        (at == b.start) != s.b.reversed
    } else {
        return None;
    };
    let end = |e: &Edge, reversed: bool| if first != reversed { e.start } else { e.end };
    Some([end(a, s.a.reversed), end(b, s.b.reversed)])
}

impl CommandSpec for SeamTakeIn {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("mm", &[self.mm])?;
        let p = tx.project();
        let seam = p
            .seams()
            .get(&self.seam)
            .copied()
            .ok_or_else(|| CommandError::not_found(self.seam))?;
        let ends = match self.at {
            Some(at) => Some(
                matched_ends(p, self.seam, at)
                    .ok_or_else(|| CommandError::invalid("at", Invalid::NotSeamEnd))?,
            ),
            None => None,
        };
        let mut moves: BTreeMap<PointId, DVec2> = BTreeMap::new();
        let mut panels = BTreeSet::new();
        for (k, side) in [seam.a, seam.b].iter().enumerate() {
            // A side on a linked mirror copy moves through its source.
            let (edge_id, point_map): (EdgeId, [(PointId, PointId); 2]) =
                match mirrored_source(p, side.edge) {
                    Some(found) => found,
                    None => {
                        let e = p
                            .edge(side.edge)
                            .ok_or_else(|| CommandError::not_found(side.edge))?;
                        (side.edge, [(e.start, e.start), (e.end, e.end)])
                    }
                };
            let edge = p
                .edge(edge_id)
                .ok_or_else(|| CommandError::not_found(edge_id))?;
            let [a, b] =
                [edge.start, edge.end].map(|id| p.point(id).map(|pt| pt.pos).unwrap_or_default());
            let normal = (b - a).perp().normalize_or_zero();
            let towards_centre = (centre(p, edge.panel) - (a + b) / 2.0).dot(normal) >= 0.0;
            let inward = if towards_centre { normal } else { -normal };
            let d = inward * (self.mm / 2.0);
            for (own, moved) in point_map {
                if ends.is_none_or(|ends| ends[k] == own) {
                    moves.insert(moved, d);
                }
            }
            panels.insert(edge.panel);
        }
        let before = outline_states(tx.project(), &panels);
        move_points(tx, &moves);
        check_outlines(tx, &before, "mm")?;
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("seam.take_in:{}", self.seam))
    }
}
