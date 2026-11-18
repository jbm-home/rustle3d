//! Take in or let out a seam: the 3D *Fit* tool
//! (docs/specs/13-3d-to-2d-editing.md).

use std::collections::{BTreeMap, BTreeSet};

use r3d_core::{Edge, EdgeId, PanelId, PointId, Project, SeamId, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::mirror::{mirror_twin, mirrored_source};
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
    const SUMMARY: &'static str = "Take in (mm > 0) or let out (mm < 0) a seam: both sides move inward by half of mm, perpendicular to the seam, so they stay matched. With `at` (an end point of the seam) only that end moves, tapering the change (e.g. take in the waist only). With linked mirror copies the symmetric seam is taken in too, so every seam stays matched.";
}

/// At the seam end where `at` is: the end point of each side, [a, b].
fn matched_ends(p: &Project, seam: SeamId, at: PointId) -> Option<[PointId; 2]> {
    let s = p.seams().get(&seam)?;
    let ((ea, ra), (eb, rb)) = s.across(s.a.whole_edge()?)?;
    let (a, b) = (p.edge(ea)?, p.edge(eb)?);
    // Is `at` the first point of its side's walk (or the last)?
    let first = if at == a.start || at == a.end {
        (at == a.start) != ra
    } else if at == b.start || at == b.end {
        (at == b.start) != rb
    } else {
        return None;
    };
    let end = |e: &Edge, reversed: bool| if first != reversed { e.start } else { e.end };
    Some([end(a, ra), end(b, rb)])
}

/// Point offsets that take `seam` in by `mm` (at the end where `at` is,
/// or along all of it), and the panels they change. Sides on linked
/// mirror copies are moved through their source.
fn seam_moves(
    p: &Project,
    seam_id: SeamId,
    at: Option<PointId>,
    mm: f64,
) -> Result<(BTreeMap<PointId, DVec2>, BTreeSet<PanelId>), CommandError> {
    let seam = p
        .seams()
        .get(&seam_id)
        .ok_or_else(|| CommandError::not_found(seam_id))?;
    let sides = [seam.a.whole_edge(), seam.b.whole_edge()];
    let [Some(side_a), Some(side_b)] = sides else {
        return Err(CommandError::invalid("seam", Invalid::NotWholeEdges));
    };
    let ends = match at {
        Some(at) => Some(
            matched_ends(p, seam_id, at)
                .ok_or_else(|| CommandError::invalid("at", Invalid::NotSeamEnd))?,
        ),
        None => None,
    };
    let mut moves = BTreeMap::new();
    let mut panels = BTreeSet::new();
    for (k, side) in [side_a, side_b].into_iter().enumerate() {
        let (edge_id, point_map): (EdgeId, [(PointId, PointId); 2]) = match mirrored_source(p, side)
        {
            Some(found) => found,
            None => {
                let e = p.edge(side).ok_or_else(|| CommandError::not_found(side))?;
                (side, [(e.start, e.start), (e.end, e.end)])
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
        let d = inward * (mm / 2.0);
        for (own, moved) in point_map {
            if ends.is_none_or(|ends| ends[k] == own) {
                moves.insert(moved, d);
            }
        }
        panels.insert(edge.panel);
    }
    Ok((moves, panels))
}

/// The seam mirroring `seam` across a linked mirror (e.g. the left side
/// seam for the right one), and the point matching `at` on it.
fn mirror_seam(
    p: &Project,
    seam_id: SeamId,
    at: Option<PointId>,
) -> Option<(SeamId, Option<PointId>)> {
    let seam = p.seams().get(&seam_id)?;
    let first = seam.a.whole_edge()?;
    for side in [first, seam.b.whole_edge()?] {
        let Some((twin, map)) = mirror_twin(p, side) else {
            continue;
        };
        let Some(other) = p.seam_of(twin) else {
            continue;
        };
        if other == seam_id {
            continue;
        }
        let at = match at {
            None => None,
            Some(at) => {
                // `at` may be on either side of the seam: bring it to this
                // side first, then across the mirror.
                let here = matched_ends(p, seam_id, at)?;
                let k = usize::from(side != first);
                map.iter().find(|(own, _)| *own == here[k]).map(|(_, m)| *m)
            }
        };
        return Some((other, at));
    }
    None
}

impl CommandSpec for SeamTakeIn {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("mm", &[self.mm])?;
        let p = tx.project();
        let (mut moves, mut panels) = seam_moves(p, self.seam, self.at, self.mm)?;
        // With a linked mirror the symmetric seam changes as well, on both
        // sides, so every seam stays matched.
        if let Some((twin, at)) = mirror_seam(p, self.seam, self.at) {
            let (more, touched) = seam_moves(p, twin, at, self.mm)?;
            moves.extend(more);
            panels.extend(touched);
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
