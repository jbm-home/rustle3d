//! Construction details: darts and pleats (spec 21), drawn, folded and
//! sewn in one command.

use r3d_core::{AnyId, Edge, EdgeCurve, EdgeId, InternalLineId, Panel, Point, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    EdgeSplit, InternalLineAdd, NotchAdd, SeamAdd, SeamSideSpec, SegmentSpec, editable_edge_panel,
    not_sewn,
};
use crate::error::{CommandError, Invalid, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Keep this far from an edge's ends (mm).
const END_MARGIN_MM: f64 = 1.0;

/// A point `s` mm along an edge, and the direction into its piece there
/// (contours turn counter-clockwise: the inside is on the left).
fn along(tx: &Transaction<'_>, edge: EdgeId, s: f64) -> Result<(DVec2, DVec2), CommandError> {
    let (at, tangent) = tx
        .project()
        .edge_geometry(edge)
        .ok_or_else(|| CommandError::not_found(edge))?
        .point_at_length(s);
    Ok((at, tangent.perp()))
}

/// Add a dart on an edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DartAdd {
    pub edge: EdgeId,
    /// Where the dart's middle is along the edge (mm from its start).
    pub at_mm: f64,
    /// How much the dart takes in at the edge (mm, > 0).
    pub width_mm: f64,
    /// How far into the piece its point is (mm, > 0).
    pub length_mm: f64,
}

impl Described for DartAdd {
    const SUMMARY: &'static str = "Draw a dart on an outline edge, fold it and sew it, in one step: the edge is opened `width_mm` wide around `at_mm`, the dart's point goes `length_mm` into the piece, and its two legs are sewn together (the cloth closes when it drapes). Returns the dart's point, its two legs and the seam. Constraints on the edge are removed.";
}

impl CommandSpec for DartAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        positive("width_mm", self.width_mm)?;
        positive("length_mm", self.length_mm)?;
        let (edge, _) = editable_edge_panel(tx, self.edge)?;
        not_sewn(tx, self.edge)?;
        let before = super::outline_states(tx.project(), &[edge.panel].into());
        let len = tx.project().edge_length(self.edge).unwrap_or_default();
        let (l, r) = (
            self.at_mm - self.width_mm / 2.0,
            self.at_mm + self.width_mm / 2.0,
        );
        if !(l >= END_MARGIN_MM && r <= len - END_MARGIN_MM) {
            return Err(CommandError::invalid("at_mm", Invalid::OutsideEdge(len)));
        }
        let (mid, inward) = along(tx, self.edge, self.at_mm)?;
        let apex_at = mid + inward * self.length_mm;
        // Open the edge: [start, L] [L, R] [R, end], the middle part
        // becoming the two legs L → apex → R.
        let right = EdgeSplit {
            edge: self.edge,
            at_mm: r,
        }
        .apply(tx)?;
        let left = EdgeSplit {
            edge: self.edge,
            at_mm: l,
        }
        .apply(tx)?;
        let (Some(AnyId::Edge(middle)), Some(AnyId::Point(r_point))) =
            (left.created.get(1).copied(), right.created.first().copied())
        else {
            return Err(CommandError::Internal {
                detail: "dart: split".into(),
            });
        };
        let apex = tx.new_point_id();
        tx.put_point(
            apex,
            Point {
                grade: tx.project().grade_at(edge.panel, apex_at),
                ..Point::new(edge.panel, apex_at)
            },
        );
        let leg1 = middle;
        let first = tx
            .project()
            .edge(leg1)
            .cloned()
            .ok_or_else(|| CommandError::not_found(leg1))?;
        tx.put_edge(
            leg1,
            Edge {
                end: apex,
                curve: EdgeCurve::Line,
                grade: Vec::new(),
                ..first
            },
        );
        let leg2 = tx.new_edge_id();
        tx.put_edge(leg2, Edge::new(edge.panel, apex, r_point, EdgeCurve::Line));
        let panel = tx
            .project()
            .panel(edge.panel)
            .cloned()
            .ok_or_else(|| CommandError::not_found(edge.panel))?;
        let mut contour = panel.contour.clone();
        let at = contour
            .iter()
            .position(|e| *e == leg1)
            .map_or(contour.len(), |i| i + 1);
        contour.insert(at.min(contour.len()), leg2);
        tx.put_panel(edge.panel, Panel { contour, ..panel });
        // The outline must stay simple: the point inside the piece.
        super::check_outlines(tx, &before, "length_mm")?;
        // Fold and sew: L meets R, the point itself.
        let seam = SeamAdd {
            a: SeamSideSpec {
                edge: Some(leg1),
                reversed: Some(false),
                ..SeamSideSpec::default()
            },
            b: SeamSideSpec {
                edge: Some(leg2),
                reversed: Some(true),
                ..SeamSideSpec::default()
            },
            ease_mm: None,
        }
        .apply(tx)?;
        let mut created: Vec<AnyId> = vec![apex.into(), leg1.into(), leg2.into()];
        created.extend(seam.created);
        Ok(CommandOutput { created })
    }
}

/// How a pleat folds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PleatKind {
    /// One fold, all the same way.
    #[default]
    Knife,
    /// Two folds facing away from each other.
    Box,
}

/// Add a pleat on an edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PleatAdd {
    pub edge: EdgeId,
    /// Where the pleat starts along the edge (mm from its start).
    pub at_mm: f64,
    /// How deep each fold is (mm, > 0): a knife pleat takes up twice that
    /// along the edge, a box pleat four times.
    pub depth_mm: f64,
    #[serde(default)]
    pub kind: PleatKind,
    /// Stitched down this far into the piece (mm); 0 or omitted: held at
    /// the edge only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stitched_mm: Option<f64>,
}

impl Described for PleatAdd {
    const SUMMARY: &'static str = "Fold and sew a pleat on an outline edge (a skirt's waist): from `at_mm`, a knife pleat folds `depth_mm` of cloth under (the edge's stretch of 2 × depth is sewn to itself, folded in the middle), a box pleat two such folds facing away from each other; with `stitched_mm`, the folds are also stitched down that far into the piece (along internal lines). Notches mark the fold lines. Returns the seams, then the lines and notches.";
}

impl CommandSpec for PleatAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        positive("depth_mm", self.depth_mm)?;
        let stitched = self.stitched_mm.unwrap_or(0.0);
        crate::error::non_negative("stitched_mm", stitched)?;
        let (edge, _) = editable_edge_panel(tx, self.edge)?;
        let len = tx.project().edge_length(self.edge).unwrap_or_default();
        let d = self.depth_mm;
        let folds: Vec<f64> = match self.kind {
            PleatKind::Knife => vec![self.at_mm],
            PleatKind::Box => vec![self.at_mm, self.at_mm + 2.0 * d],
        };
        let end = folds.last().copied().unwrap_or(self.at_mm) + 2.0 * d;
        if !(self.at_mm >= 0.0 && end <= len + 1e-6) {
            return Err(CommandError::invalid("at_mm", Invalid::OutsideEdge(len)));
        }
        let mut seams = Vec::new();
        let mut rest = Vec::new();
        // Where the fold lines meet the edge, marked with notches.
        let mut marks: Vec<f64> = Vec::new();
        for (k, p0) in folds.iter().enumerate() {
            let (p1, p2) = (p0 + d, p0 + 2.0 * d);
            // The stretch [p0, p1] folded onto [p1, p2]: p0 meets p2. A box
            // pleat's second fold turns the other way.
            let (a, b) = if k == 0 {
                ((*p0, p1), (p1, p2))
            } else {
                ((p1, p2), (*p0, p1))
            };
            let out = SeamAdd {
                a: SeamSideSpec {
                    edge: Some(self.edge),
                    from_mm: Some(a.0),
                    to_mm: Some(a.1),
                    reversed: Some(false),
                    ..SeamSideSpec::default()
                },
                b: SeamSideSpec {
                    edge: Some(self.edge),
                    from_mm: Some(b.0),
                    to_mm: Some(b.1),
                    reversed: Some(true),
                    ..SeamSideSpec::default()
                },
                ease_mm: None,
            }
            .apply(tx)?;
            seams.extend(out.created);
            for s in [*p0, p1, p2] {
                if !marks.iter().any(|m: &f64| (m - s).abs() < 1e-9) {
                    marks.push(s);
                }
            }
        }
        if stitched > 0.0 {
            // The outer fold lines meet: stitched together down the piece.
            let mut line = |s: f64| -> Result<InternalLineId, CommandError> {
                let (at, inward) = along(tx, self.edge, s)?;
                let to = at + inward * stitched;
                let out = InternalLineAdd {
                    panel: edge.panel,
                    start: [at.x, at.y],
                    segments: vec![SegmentSpec::Line { to: [to.x, to.y] }],
                }
                .apply(tx)?;
                match out.created.first() {
                    Some(AnyId::InternalLine(l)) => Ok(*l),
                    _ => Err(CommandError::Internal {
                        detail: "pleat: line".into(),
                    }),
                }
            };
            let (la, lb) = (line(self.at_mm)?, line(end)?);
            let out = SeamAdd {
                a: SeamSideSpec {
                    line: Some(la),
                    reversed: Some(false),
                    ..SeamSideSpec::default()
                },
                b: SeamSideSpec {
                    line: Some(lb),
                    reversed: Some(false),
                    ..SeamSideSpec::default()
                },
                ease_mm: None,
            }
            .apply(tx)?;
            rest.push(la.into());
            rest.push(lb.into());
            seams.extend(out.created);
        }
        for s in marks {
            let n = NotchAdd {
                edge: self.edge,
                offset_mm: s,
            }
            .apply(tx)?;
            rest.extend(n.created);
        }
        seams.extend(rest);
        Ok(CommandOutput { created: seams })
    }
}
