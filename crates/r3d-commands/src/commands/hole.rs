//! Holes in pieces and booleans between pieces (spec 21).

use r3d_core::measure::FLATTEN_TOLERANCE_MM;
use r3d_core::{AnyId, Edge, EdgeCurve, EdgeId, Panel, PanelId, PathSegment, Point, Transaction};
use r3d_geom::boolean::{Op, Region, boolean};
use r3d_geom::fit::{Fitted, fit_polyline};
use r3d_geom::{CubicBez, DVec2, polygon};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::internal_line::path_segments;
use super::{SegmentSpec, Vec2, editable_panel, get_panel, v};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// How closely a boolean's result follows the pieces' curves (mm).
const FIT_TOLERANCE_MM: f64 = 0.2;

/// A piece that may be reshaped: not a linked mirror copy, not mirrored,
/// not symmetric; and, when its outline is redrawn (`outline`), not sewn.
fn reshapable(tx: &Transaction<'_>, id: PanelId, outline: bool) -> Result<Panel, CommandError> {
    let panel = editable_panel(tx, id)?;
    let p = tx.project();
    if p.panels()
        .values()
        .any(|c| c.mirror_of.as_ref().is_some_and(|l| l.source == id))
    {
        return Err(CommandError::invalid("panel", Invalid::Mirrored));
    }
    if panel.symmetry.is_some() {
        return Err(CommandError::invalid("panel", Invalid::Symmetric));
    }
    let edges = panel.contour.iter().chain(panel.holes.iter().flatten());
    for e in edges.filter(|_| outline) {
        if let Some(seam) = p.seam_of(*e) {
            return Err(CommandError::Sewn {
                edge: (*e).into(),
                seam: seam.into(),
            });
        }
    }
    Ok(panel)
}

/// A piece's region: its outline and holes, flattened.
fn region(tx: &Transaction<'_>, panel: &Panel) -> Result<Region, CommandError> {
    let p = tx.project();
    let ring = |edges: &[EdgeId]| -> Result<Vec<DVec2>, CommandError> {
        let mut out = Vec::new();
        for e in edges {
            let line = p
                .edge_polyline(*e)
                .ok_or_else(|| CommandError::not_found(*e))?;
            out.extend(line.iter().take(line.len().saturating_sub(1)));
        }
        Ok(out)
    };
    Ok(Region {
        outline: ring(&panel.contour)?,
        holes: panel
            .holes
            .iter()
            .map(|h| ring(h))
            .collect::<Result<_, _>>()?,
    })
}

/// Put a closed loop of fitted pieces into `panel`: new points and edges,
/// graded like the piece's points around them. Returns the edges.
fn put_loop(
    tx: &mut Transaction<'_>,
    panel: PanelId,
    pieces: &[Fitted],
    grades: &dyn Fn(DVec2) -> Vec<DVec2>,
) -> Vec<EdgeId> {
    let points: Vec<_> = pieces
        .iter()
        .map(|f| {
            let id = tx.new_point_id();
            tx.put_point(
                id,
                Point {
                    grade: grades(f.start()),
                    ..Point::new(panel, f.start())
                },
            );
            id
        })
        .collect();
    let n = points.len();
    pieces
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let (a, b) = (*points.get(i)?, *points.get((i + 1) % n)?);
            let curve = match f {
                Fitted::Line(..) => EdgeCurve::Line,
                Fitted::Cubic(c) => EdgeCurve::Cubic { c1: c.p1, c2: c.p2 },
            };
            let id = tx.new_edge_id();
            tx.put_edge(id, Edge::new(panel, a, b, curve));
            Some(id)
        })
        .collect()
}

/// Give `panel` a new outline and holes (counter-clockwise outline,
/// clockwise holes, as polylines), fitted to curves; its old points and
/// edges go, with their notches.
fn reshape(
    tx: &mut Transaction<'_>,
    id: PanelId,
    outline: &[DVec2],
    holes: &[Vec<DVec2>],
) -> Result<(), CommandError> {
    let panel = get_panel(tx.project(), id)?;
    // Grades of the new points: as the piece's points around them moved.
    let before = tx.project().clone();
    let grades = |at: DVec2| before.grade_at(id, at);
    let old: Vec<EdgeId> = panel
        .contour
        .iter()
        .chain(panel.holes.iter().flatten())
        .copied()
        .collect();
    let notches: Vec<_> = tx
        .project()
        .notches()
        .iter()
        .filter(|(_, n)| old.contains(&n.edge))
        .map(|(nid, _)| *nid)
        .collect();
    for n in notches {
        tx.remove_notch(n);
    }
    for e in &old {
        if let Some(edge) = tx.remove_edge(*e) {
            tx.remove_point(edge.start);
        }
    }
    let fit = |ring: &[DVec2]| fit_polyline(ring, true, FIT_TOLERANCE_MM);
    let contour = put_loop(tx, id, &fit(outline), &grades);
    let holes = holes
        .iter()
        .map(|h| put_loop(tx, id, &fit(h), &grades))
        .collect();
    tx.put_panel(
        id,
        Panel {
            contour,
            holes,
            ..panel
        },
    );
    Ok(())
}

/// Cut a hole in a piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelAddHole {
    pub panel: PanelId,
    /// First point of the hole's outline (mm).
    pub start: Vec2,
    /// Its edges in order (straight or cubic), back to `start` (a straight
    /// edge closes it otherwise).
    pub segments: Vec<SegmentSpec>,
}

impl Described for PanelAddHole {
    const SUMMARY: &'static str = "Cut a hole in a piece (a buttonhole opening, a cut-out): a closed path of straight and cubic edges (mm) inside the piece, not crossing its outline or its other holes. The hole is left out of the cloth; its edges can be sewn and notched like the outline's. Returns the hole's points, then its edges.";
}

impl CommandSpec for PanelAddHole {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let panel = reshapable(tx, self.panel, false)?;
        finite("start", &self.start)?;
        let segments = path_segments("segments", &self.segments)?;
        let start = v(self.start);
        // The loop: each point with the edge from it.
        let mut ring: Vec<(DVec2, EdgeCurve)> = Vec::new();
        let mut at = start;
        for seg in &segments {
            let curve = match seg {
                PathSegment::Line { .. } => EdgeCurve::Line,
                PathSegment::Cubic { c1, c2, .. } => EdgeCurve::Cubic { c1: *c1, c2: *c2 },
            };
            ring.push((at, curve));
            at = seg.end();
        }
        if at.distance(start) > 1e-3 {
            ring.push((at, EdgeCurve::Line));
        }
        let mut flat = Vec::new();
        for (i, (a, curve)) in ring.iter().enumerate() {
            flat.push(*a);
            if let (EdgeCurve::Cubic { c1, c2 }, Some((b, _))) =
                (curve, ring.get((i + 1) % ring.len()))
            {
                CubicBez::new(*a, *c1, *c2, *b).flatten_into(FLATTEN_TOLERANCE_MM, &mut flat);
                flat.pop();
            }
        }
        if flat.len() < 3 || !polygon::is_simple(&flat) {
            return Err(CommandError::invalid("segments", Invalid::NotSimple));
        }
        // Inside the piece, clear of its outline and other holes.
        let r = region(tx, &panel)?;
        let crosses = |ring: &[DVec2]| {
            (0..flat.len()).any(|i| {
                let (a, b) = (flat[i], flat[(i + 1) % flat.len()]);
                (0..ring.len())
                    .any(|j| polygon::segments_intersect(a, b, ring[j], ring[(j + 1) % ring.len()]))
            })
        };
        let inside = flat.iter().all(|p| polygon::contains(&r.outline, *p))
            && !crosses(&r.outline)
            && r.holes.iter().all(|h| {
                !crosses(h)
                    && !flat.iter().any(|p| polygon::contains(h, *p))
                    && !h.iter().any(|p| polygon::contains(&flat, *p))
            });
        if !inside {
            return Err(CommandError::invalid("segments", Invalid::NotAcross));
        }
        // Holes are walked clockwise.
        if polygon::signed_area(&flat) > 0.0 {
            let n = ring.len();
            ring = (0..n)
                .map(|j| {
                    let (pos, _) = ring[n - 1 - j];
                    let (_, curve) = &ring[(2 * n - 2 - j) % n];
                    let curve = match curve {
                        EdgeCurve::Line => EdgeCurve::Line,
                        EdgeCurve::Cubic { c1, c2 } => EdgeCurve::Cubic { c1: *c2, c2: *c1 },
                    };
                    (pos, curve)
                })
                .collect();
        }
        let before = tx.project().clone();
        let points: Vec<_> = ring
            .iter()
            .map(|(pos, _)| {
                let id = tx.new_point_id();
                tx.put_point(
                    id,
                    Point {
                        grade: before.grade_at(self.panel, *pos),
                        ..Point::new(self.panel, *pos)
                    },
                );
                id
            })
            .collect();
        let n = points.len();
        let edges: Vec<EdgeId> = (0..n)
            .map(|i| {
                let id = tx.new_edge_id();
                tx.put_edge(
                    id,
                    Edge::new(
                        self.panel,
                        points[i],
                        points[(i + 1) % n],
                        ring[i].1.clone(),
                    ),
                );
                id
            })
            .collect();
        let mut panel = panel;
        panel.holes.push(edges.clone());
        tx.put_panel(self.panel, panel);
        let mut created: Vec<AnyId> = points.into_iter().map(AnyId::from).collect();
        created.extend(edges.into_iter().map(AnyId::from));
        Ok(CommandOutput { created })
    }
}

/// Remove a hole from a piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelRemoveHole {
    /// Any edge of the hole.
    pub edge: EdgeId,
}

impl Described for PanelRemoveHole {
    const SUMMARY: &'static str = "Fill a hole of a piece again: its points and edges go, with the notches and seams on them.";
}

impl CommandSpec for PanelRemoveHole {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let edge = tx
            .project()
            .edge(self.edge)
            .cloned()
            .ok_or_else(|| CommandError::not_found(self.edge))?;
        let mut panel = editable_panel(tx, edge.panel)?;
        let k = panel
            .holes
            .iter()
            .position(|h| h.contains(&self.edge))
            .ok_or_else(|| CommandError::invalid("edge", Invalid::NotSeamSide))?;
        let hole = panel.holes.remove(k);
        let p = tx.project();
        let seams: Vec<_> = p
            .seams()
            .iter()
            .filter(|(_, s)| hole.iter().any(|e| s.touches(*e)))
            .map(|(id, _)| *id)
            .collect();
        let notches: Vec<_> = p
            .notches()
            .iter()
            .filter(|(_, n)| hole.contains(&n.edge))
            .map(|(id, _)| *id)
            .collect();
        for s in seams {
            tx.remove_seam(s);
        }
        for n in notches {
            tx.remove_notch(n);
        }
        for e in &hole {
            if let Some(edge) = tx.remove_edge(*e) {
                tx.remove_point(edge.start);
            }
        }
        tx.put_panel(edge.panel, panel);
        Ok(CommandOutput::default())
    }
}

/// Join two pieces into one, or cut one with another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelBoolean {
    /// The piece changed.
    pub panel: PanelId,
    /// The other piece: joined to `panel` (`union`), or cut out of it
    /// (`difference`). It goes, unless `keep`.
    pub with: PanelId,
    pub op: BooleanOp,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep: bool,
}

/// What a boolean keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BooleanOp {
    /// Both pieces' cloth: one piece (they must overlap or touch).
    Union,
    /// The first piece without the second: a notch cut in its outline, or
    /// a hole when the second lies inside.
    Difference,
}

impl Described for PanelBoolean {
    const SUMMARY: &'static str = "Join two overlapping pieces into one (`union`), or cut a piece with another (`difference`: a bite out of its outline, or a hole when the other lies inside it). The result is fitted with straight and curved edges; the other piece is removed unless `keep`. Pieces that are sewn, symmetric or mirrored must be unpicked or unlinked first; notches on the changed outline go. Fails if the result would be several pieces or nothing.";
}

impl CommandSpec for PanelBoolean {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if self.panel == self.with {
            return Err(CommandError::invalid("with", Invalid::SamePoint));
        }
        let a = reshapable(tx, self.panel, true)?;
        let b = if self.keep {
            get_panel(tx.project(), self.with)?
        } else {
            reshapable(tx, self.with, true)?
        };
        let (ra, rb) = (region(tx, &a)?, region(tx, &b)?);
        let op = match self.op {
            BooleanOp::Union => Op::Union,
            BooleanOp::Difference => Op::Difference,
        };
        let loops = boolean(&ra, &rb, op).ok_or_else(|| CommandError::Internal {
            detail: "boolean: the outlines cannot be triangulated".into(),
        })?;
        let (outlines, holes): (Vec<_>, Vec<_>) = loops
            .into_iter()
            .partition(|l| polygon::signed_area(l) > 0.0);
        let [outline] = outlines.as_slice() else {
            return Err(CommandError::invalid("with", Invalid::Apart));
        };
        reshape(tx, self.panel, outline, &holes)?;
        if !self.keep {
            super::PanelDelete {
                panel: self.with,
                confirm: true,
            }
            .apply(tx)?;
        }
        Ok(CommandOutput::default())
    }
}
