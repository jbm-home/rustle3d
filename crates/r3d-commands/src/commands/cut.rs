//! Cutting a panel in two along a line (docs/specs/16-cut.md).

use r3d_core::measure::{EdgeGeometry, FLATTEN_TOLERANCE_MM, LENGTH_TOLERANCE_MM};
use r3d_core::{
    AnyId, Edge, EdgeCurve, EdgeId, InternalLine, Notch, Panel, PanelId, PathSegment, Pin,
    Placement, Point, PointId, Seam, SeamSide, Transaction,
};
use r3d_geom::{CubicBez, DVec2, polygon};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::internal_line::path_segments;
use super::{SegmentSpec, Vec2, editable_panel, get_panel, v};
use crate::error::{CommandError, Invalid, finite, non_blank};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// A crossing this close to an outline point is at that point (mm), and
/// two crossings this close are one.
const SNAP_MM: f64 = 0.5;

/// The new piece is moved this far from the other (mm), so both show.
const GAP_MM: f64 = 20.0;

/// Cut a panel in two along a straight or curved line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCut {
    /// The panel to cut. If omitted, the one panel the line crosses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<PanelId>,
    /// Start of the cut line (mm), outside the panel.
    pub start: Vec2,
    /// The cut line's segments; it ends outside the panel too.
    pub segments: Vec<SegmentSpec>,
    /// Name of the new piece. Defaults to the panel's name followed by " 2".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Described for PanelCut {
    const SUMMARY: &'static str = "Cut a panel in two along an open path of straight and cubic segments (mm) that crosses its outline exactly twice, its ends outside the panel (without `panel`, the one panel it crosses). The two pieces are sewn together along the cut. The larger piece keeps the panel's ID, its seams and pins; the other is a new panel with the same garment, grain and seam allowance, moved 20 mm away on the board and placed where it was around the body. Notches, internal lines and pins go to the piece they are on; a sewn edge that is cut is split with its partner, and the seam with them. Returns the new panel, then the seams along the cut.";
}

/// Where the cut line crosses the outline.
#[derive(Clone, Copy, Debug)]
struct Crossing {
    /// Cut segment and parameter on it.
    seg: usize,
    t: f64,
    pos: DVec2,
    at: At,
}

#[derive(Clone, Copy, Debug)]
enum At {
    Point(PointId),
    Edge(EdgeId, f64),
}

fn not_across() -> CommandError {
    CommandError::invalid("segments", Invalid::NotAcross)
}

fn cubic(g: EdgeGeometry) -> CubicBez {
    match g {
        EdgeGeometry::Line(a, b) => CubicBez::line(a, b),
        EdgeGeometry::Cubic(c) => c,
    }
}

fn geometry(tx: &Transaction<'_>, edge: EdgeId) -> Result<EdgeGeometry, CommandError> {
    tx.project()
        .edge_geometry(edge)
        .ok_or_else(|| CommandError::not_found(edge))
}

impl CommandSpec for PanelCut {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if let Some(n) = &self.name {
            non_blank("name", n)?;
        }
        finite("start", &self.start)?;
        if self.segments.is_empty() {
            return Err(CommandError::invalid("segments", Invalid::Empty));
        }
        let path = cut_path(v(self.start), &path_segments("segments", &self.segments)?);

        // 1. The panel, and the two crossings in the cut's direction.
        let (panel_id, [a, b]) = match self.panel {
            Some(id) => {
                let panel = editable_panel(tx, id)?;
                if has_mirror_copy(tx, id) {
                    return Err(CommandError::invalid("panel", Invalid::Mirrored));
                }
                if panel.symmetry.is_some() {
                    return Err(CommandError::invalid("panel", Invalid::Symmetric));
                }
                if !panel.holes.is_empty() {
                    return Err(CommandError::invalid("panel", Invalid::HasHoles));
                }
                (id, crossings(tx, &panel, &path)?)
            }
            None => {
                let crossed: Vec<_> = tx
                    .project()
                    .panels()
                    .iter()
                    .filter_map(|(id, p)| Some((*id, crossings(tx, p, &path).ok()?)))
                    .collect();
                match crossed.as_slice() {
                    [(id, found)] => {
                        // Say why the one piece crossed cannot be cut.
                        let panel = editable_panel(tx, *id)?;
                        if has_mirror_copy(tx, *id) {
                            return Err(CommandError::invalid("panel", Invalid::Mirrored));
                        }
                        if panel.symmetry.is_some() {
                            return Err(CommandError::invalid("panel", Invalid::Symmetric));
                        }
                        if !panel.holes.is_empty() {
                            return Err(CommandError::invalid("panel", Invalid::HasHoles));
                        }
                        (*id, *found)
                    }
                    _ => return Err(not_across()),
                }
            }
        };
        let panel = get_panel(tx.project(), panel_id)?;
        let whole = tx
            .project()
            .panel_outline(panel_id)
            .ok_or_else(|| CommandError::not_found(panel_id))?;
        let inner = inner_path(&path, &a, &b);
        if inner.is_empty() {
            return Err(not_across());
        }
        if let Some((c, _)) = inner.get(inner.len() / 2)
            && !polygon::contains(&whole, c.eval(0.5))
        {
            return Err(not_across());
        }

        // 2. Points on the outline where it is cut.
        let (pa, pb) = split_outline(tx, a.at, b.at)?;
        if pa == pb {
            return Err(not_across());
        }

        // 3. The two halves: outline A → B then the cut back, and
        //    outline B → A then the cut forward. Both stay counter-clockwise.
        let contour = get_panel(tx.project(), panel_id)?.contour;
        let starts: Vec<PointId> = contour
            .iter()
            .map(|e| tx.project().edge(*e).map(|e| e.start))
            .collect::<Option<_>>()
            .ok_or_else(|| CommandError::not_found(panel_id))?;
        let ia = starts
            .iter()
            .position(|p| *p == pa)
            .ok_or_else(not_across)?;
        let ib = starts
            .iter()
            .position(|p| *p == pb)
            .ok_or_else(not_across)?;
        let run = |from: usize, to: usize| -> Vec<EdgeId> {
            let n = contour.len();
            let len = (to + n - from) % n;
            (0..len).map(|k| contour[(from + k) % n]).collect()
        };
        let (ab, ba) = (run(ia, ib), run(ib, ia));
        let pos_a = point_pos(tx, pa)?;
        let pos_b = point_pos(tx, pb)?;
        let mut forward = inner;
        if let Some((first, _)) = forward.first_mut() {
            first.p0 = pos_a;
        }
        if let Some((last, _)) = forward.last_mut() {
            last.p3 = pos_b;
        }
        let back: Vec<(CubicBez, bool)> = forward
            .iter()
            .rev()
            .map(|(c, l)| (c.reversed(), *l))
            .collect();
        let half_ab = outline(tx, &ab, &back)?;
        let half_ba = outline(tx, &ba, &forward)?;
        for half in [&half_ab, &half_ba] {
            if !polygon::is_simple(half) || polygon::signed_area(half) <= 0.0 {
                return Err(CommandError::invalid("segments", Invalid::NotSimple));
            }
        }

        // 4. The larger half keeps the panel; the other becomes a new one.
        //    Its outline runs from `x` to `y`, its cut from `y` back to `x`.
        let keep_ab = polygon::signed_area(&half_ab) >= polygon::signed_area(&half_ba);
        let (kept_edges, kept_cut, new_edges, new_cut, new_outline, x, y) = if keep_ab {
            (ab, back, ba, forward, half_ba, pb, pa)
        } else {
            (ba, forward, ab, back, half_ab, pa, pb)
        };
        let new_panel = tx.new_panel_id();
        let x_new = copy_point(tx, x, new_panel)?;
        let y_new = copy_point(tx, y, new_panel)?;
        for (k, e) in new_edges.iter().enumerate() {
            let mut edge = tx
                .project()
                .edge(*e)
                .cloned()
                .ok_or_else(|| CommandError::not_found(*e))?;
            edge.panel = new_panel;
            if k == 0 {
                edge.start = x_new;
            } else {
                let mut point = tx
                    .project()
                    .point(edge.start)
                    .cloned()
                    .ok_or_else(|| CommandError::not_found(edge.start))?;
                point.panel = new_panel;
                tx.put_point(edge.start, point);
            }
            if k + 1 == new_edges.len() {
                edge.end = y_new;
            }
            tx.put_edge(*e, edge);
        }
        // The kept half's cut runs from its outline's end (x) to its start (y).
        let kept_cut_edges = cut_edges(tx, panel_id, &kept_cut, x, y);
        let new_cut_edges = cut_edges(tx, new_panel, &new_cut, y_new, x_new);

        let mut kept = panel.clone();
        kept.contour = kept_edges.iter().chain(&kept_cut_edges).copied().collect();
        let kept_outline = outline(tx, &kept_edges, &kept_cut)?;
        let (kept_place, new_place) = match panel.placement {
            Some(place) => (
                Some(shifted(place, &whole, &kept_outline)),
                Some(shifted(place, &whole, &new_outline)),
            ),
            None => (None, None),
        };
        kept.placement = kept_place;
        tx.put_panel(panel_id, kept);
        tx.put_panel(
            new_panel,
            Panel {
                name: match &self.name {
                    Some(n) => n.trim().to_owned(),
                    // English default (i18n: docs/architecture.md §7.7).
                    None => format!("{} 2", panel.name),
                },
                contour: new_edges.iter().chain(&new_cut_edges).copied().collect(),
                placement: new_place,
                mirror_of: None,
                ..panel
            },
        );

        // 5. Sew the halves along the cut: the new half's edge k pairs with
        //    the kept half's edge walked backwards.
        let mut created: Vec<AnyId> = vec![new_panel.into()];
        for (k, e) in new_cut_edges.iter().enumerate() {
            let Some(partner) = kept_cut_edges.iter().rev().nth(k) else {
                continue;
            };
            let seam = tx.new_seam_id();
            tx.put_seam(
                seam,
                Seam::whole(SeamSide::edge(*e, false), SeamSide::edge(*partner, true)),
            );
            created.push(seam.into());
        }

        // 6. Internal lines and pins go with the piece they start on.
        let lines: Vec<_> = tx
            .project()
            .internal_lines()
            .iter()
            .filter(|(_, l)| l.panel == panel_id && polygon::contains(&new_outline, l.start))
            .map(|(id, l)| (*id, l.clone()))
            .collect();
        for (id, line) in lines {
            tx.put_internal_line(
                id,
                InternalLine {
                    panel: new_panel,
                    ..line
                },
            );
        }
        let pins: Vec<_> = tx
            .project()
            .pins()
            .iter()
            .filter(|(_, p)| p.panel == panel_id && polygon::contains(&new_outline, p.at))
            .map(|(id, p)| (*id, *p))
            .collect();
        // Buttons and buttonholes on the half that leaves go with it.
        let moving =
            |panel: PanelId, at: DVec2| panel == panel_id && polygon::contains(&new_outline, at);
        let buttons: Vec<_> = tx
            .project()
            .buttons()
            .iter()
            .filter(|(_, b)| moving(b.panel, b.at) || moving(b.hole.panel, b.hole.at))
            .map(|(id, b)| (*id, b.clone()))
            .collect();
        for (id, mut b) in buttons {
            if moving(b.panel, b.at) {
                b.panel = new_panel;
            }
            if moving(b.hole.panel, b.hole.at) {
                b.hole.panel = new_panel;
            }
            tx.put_button(id, b);
        }
        for (id, pin) in pins {
            tx.put_pin(
                id,
                Pin {
                    panel: new_panel,
                    ..pin
                },
            );
        }
        // 7. Move the new piece a little away from the other.
        if let (Some(k), Some(n)) = (
            polygon::bounding_box(&kept_outline),
            polygon::bounding_box(&new_outline),
        ) {
            let away = ((n.min + n.max) - (k.min + k.max))
                .try_normalize()
                .unwrap_or(DVec2::X)
                * GAP_MM;
            super::PanelTranslate {
                panel: new_panel,
                delta: [away.x, away.y],
            }
            .apply(tx)?;
        }
        Ok(CommandOutput { created })
    }
}

fn has_mirror_copy(tx: &Transaction<'_>, panel: PanelId) -> bool {
    tx.project()
        .panels()
        .values()
        .any(|p| p.mirror_of.as_ref().is_some_and(|l| l.source == panel))
}

/// The cut as cubics, each flagged when it is a straight segment.
fn cut_path(start: DVec2, segments: &[PathSegment]) -> Vec<(CubicBez, bool)> {
    let mut at = start;
    segments
        .iter()
        .map(|s| {
            let c = match s {
                PathSegment::Line { to } => (CubicBez::line(at, *to), true),
                PathSegment::Cubic { c1, c2, to } => (CubicBez::new(at, *c1, *c2, *to), false),
            };
            at = s.end();
            c
        })
        .collect()
}

/// The cut's two crossings with the panel's outline, in the cut's order.
fn crossings(
    tx: &Transaction<'_>,
    panel: &Panel,
    path: &[(CubicBez, bool)],
) -> Result<[Crossing; 2], CommandError> {
    let p = tx.project();
    let mut found: Vec<Crossing> = Vec::new();
    for e in &panel.contour {
        let edge = p.edge(*e).ok_or_else(|| CommandError::not_found(*e))?;
        let g = cubic(geometry(tx, *e)?);
        for (seg, (c, _)) in path.iter().enumerate() {
            for (t, u) in c.crossings(&g, FLATTEN_TOLERANCE_MM) {
                let pos = g.eval(u);
                let at = if pos.distance(g.p0) < SNAP_MM {
                    At::Point(edge.start)
                } else if pos.distance(g.p3) < SNAP_MM {
                    At::Point(edge.end)
                } else {
                    At::Edge(*e, u)
                };
                let pos = match at {
                    At::Point(pt) => point_pos(tx, pt)?,
                    At::Edge(..) => pos,
                };
                found.push(Crossing { seg, t, pos, at });
            }
        }
    }
    found.sort_by(|a, b| {
        (a.seg, a.t)
            .partial_cmp(&(b.seg, b.t))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut merged: Vec<Crossing> = Vec::new();
    for c in found {
        if !merged.iter().any(|m| m.pos.distance(c.pos) < SNAP_MM) {
            merged.push(c);
        }
    }
    match merged.as_slice() {
        [a, b] => Ok([*a, *b]),
        _ => Err(not_across()),
    }
}

/// The part of the cut between its two crossings, as cubics (with their
/// straight flags).
fn inner_path(path: &[(CubicBez, bool)], a: &Crossing, b: &Crossing) -> Vec<(CubicBez, bool)> {
    (a.seg..=b.seg)
        .filter_map(|k| {
            let (c, line) = path.get(k)?;
            let lo = if k == a.seg { a.t } else { 0.0 };
            let hi = if k == b.seg { b.t } else { 1.0 };
            let sub = c.sub(lo, hi);
            (sub.p0.distance(sub.p3) + sub.p0.distance(sub.p1) + sub.p2.distance(sub.p3) > 1e-6)
                .then_some((sub, *line))
        })
        .collect()
}

fn point_pos(tx: &Transaction<'_>, id: PointId) -> Result<DVec2, CommandError> {
    tx.project()
        .point(id)
        .map(|p| p.pos)
        .ok_or_else(|| CommandError::not_found(id))
}

/// A new point of `panel` where `id` is.
fn copy_point(
    tx: &mut Transaction<'_>,
    id: PointId,
    panel: PanelId,
) -> Result<PointId, CommandError> {
    let pos = point_pos(tx, id)?;
    let grade = tx
        .project()
        .point(id)
        .map(|p| p.grade.clone())
        .unwrap_or_default();
    let new = tx.new_point_id();
    tx.put_point(new, Point { panel, pos, grade });
    Ok(new)
}

/// Outline points for the cut: existing points, or edges split there. When
/// both crossings are on one edge, the later one is split first so the
/// other's parameter still holds.
fn split_outline(
    tx: &mut Transaction<'_>,
    a: At,
    b: At,
) -> Result<(PointId, PointId), CommandError> {
    match (a, b) {
        (At::Edge(ea, ta), At::Edge(eb, tb)) if ea == eb => {
            if ta < tb {
                let pb = split_edge(tx, eb, tb)?;
                Ok((split_edge(tx, ea, ta)?, pb))
            } else {
                let pa = split_edge(tx, ea, ta)?;
                Ok((pa, split_edge(tx, eb, tb)?))
            }
        }
        (At::Edge(ea, _), At::Edge(eb, _))
            if tx.project().seam_of(ea).is_some()
                && tx.project().seam_of(ea) == tx.project().seam_of(eb) =>
        {
            // Both sides of one seam: splitting one would split the other.
            let seam = tx.project().seam_of(ea).ok_or_else(not_across)?;
            Err(CommandError::Sewn {
                edge: eb.into(),
                seam: seam.into(),
            })
        }
        _ => Ok((at_point(tx, a)?, at_point(tx, b)?)),
    }
}

fn at_point(tx: &mut Transaction<'_>, at: At) -> Result<PointId, CommandError> {
    match at {
        At::Point(p) => Ok(p),
        At::Edge(e, t) => split_edge(tx, e, t),
    }
}

/// Split an edge at parameter `t`; a sewn edge's partner is split at the
/// same fraction of the seam, and the seam in two. Returns the new point.
fn split_edge(tx: &mut Transaction<'_>, edge: EdgeId, t: f64) -> Result<PointId, CommandError> {
    let geom = geometry(tx, edge)?;
    let len = geom.length();
    let (point, second, first_len) = split_at(tx, edge, t)?;
    let Some(seam_id) = tx.project().seam_of(edge) else {
        return Ok(point);
    };
    let seam = tx
        .project()
        .seams()
        .get(&seam_id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(seam_id))?;
    // A seam on part of the edge, or on more than one, cannot follow a
    // cut across it: unpick it first.
    let Some(((_, mine_reversed), (partner, theirs_reversed))) = seam.across(edge) else {
        return Err(CommandError::Sewn {
            edge: edge.into(),
            seam: seam_id.into(),
        });
    };
    // How far along the seam the cut is, then where that is on the partner.
    let along = if len > 0.0 { first_len / len } else { 0.5 };
    let walked = if mine_reversed { 1.0 - along } else { along };
    let partner_frac = if theirs_reversed {
        1.0 - walked
    } else {
        walked
    };
    super::editable_edge_panel(tx, partner)?;
    let pg = geometry(tx, partner)?;
    let u = match pg {
        EdgeGeometry::Line(..) => partner_frac,
        EdgeGeometry::Cubic(c) => c.param_at_length(
            partner_frac * c.arc_length(LENGTH_TOLERANCE_MM),
            LENGTH_TOLERANCE_MM,
        ),
    };
    let (_, partner_second, _) = split_at(tx, partner, u)?;
    // Each side's halves in walking order, then one seam per pair.
    let walk = |reversed: bool, first: EdgeId, second: EdgeId| {
        if reversed {
            [second, first]
        } else {
            [first, second]
        }
    };
    let m = walk(mine_reversed, edge, second);
    let o = walk(theirs_reversed, partner, partner_second);
    let mine_is_a = seam.a.whole_edge() == Some(edge);
    let make = |k: usize| {
        let (mine, theirs) = (
            SeamSide::edge(m[k], mine_reversed),
            SeamSide::edge(o[k], theirs_reversed),
        );
        let (a, b) = if mine_is_a {
            (mine, theirs)
        } else {
            (theirs, mine)
        };
        Seam {
            a,
            b,
            gathered: seam.gathered,
            ease_mm: seam.ease_mm / 2.0,
        }
    };
    tx.put_seam(seam_id, make(0));
    let more = tx.new_seam_id();
    tx.put_seam(more, make(1));
    Ok(point)
}

/// Split an edge at parameter `t` (the shape is kept): returns the new
/// point, the new second edge and the first half's length. Notches past
/// the point move to the second edge.
fn split_at(
    tx: &mut Transaction<'_>,
    id: EdgeId,
    t: f64,
) -> Result<(PointId, EdgeId, f64), CommandError> {
    let edge = tx
        .project()
        .edge(id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))?;
    let mut panel = get_panel(tx.project(), edge.panel)?;
    let (pos, first, second, first_len) = match geometry(tx, id)? {
        EdgeGeometry::Line(a, b) => (
            a.lerp(b, t),
            EdgeCurve::Line,
            EdgeCurve::Line,
            a.distance(b) * t,
        ),
        EdgeGeometry::Cubic(c) => {
            let (l, r) = c.split(t);
            (
                l.p3,
                EdgeCurve::Cubic { c1: l.p1, c2: l.p2 },
                EdgeCurve::Cubic { c1: r.p1, c2: r.p2 },
                l.arc_length(LENGTH_TOLERANCE_MM),
            )
        }
    };
    let total = tx.project().edge_length(id).unwrap_or_default();
    let fraction = if total > 0.0 { first_len / total } else { 0.0 };
    let grade = tx.project().grade_along(id, fraction);
    let point = tx.new_point_id();
    tx.put_point(
        point,
        Point {
            panel: edge.panel,
            pos,
            grade,
        },
    );
    let new_edge = tx.new_edge_id();
    tx.put_edge(new_edge, Edge::new(edge.panel, point, edge.end, second));
    tx.put_edge(
        id,
        Edge {
            end: point,
            curve: first,
            grade: Vec::new(),
            ..edge
        },
    );
    let at = panel
        .contour
        .iter()
        .position(|e| *e == id)
        .map_or(panel.contour.len(), |i| i + 1);
    panel.contour.insert(at, new_edge);
    tx.put_panel(edge.panel, panel);
    let moved: Vec<_> = tx
        .project()
        .notches()
        .iter()
        .filter(|(_, n)| n.edge == id && n.offset_mm > first_len)
        .map(|(nid, n)| (*nid, n.offset_mm))
        .collect();
    for (nid, offset) in moved {
        tx.put_notch(
            nid,
            Notch {
                edge: new_edge,
                offset_mm: offset - first_len,
            },
        );
    }
    Ok((point, new_edge, first_len))
}

/// The outline of a half: its outline edges, then its cut (mm).
fn outline(
    tx: &Transaction<'_>,
    edges: &[EdgeId],
    cut: &[(CubicBez, bool)],
) -> Result<Vec<DVec2>, CommandError> {
    let mut out = Vec::new();
    for e in edges {
        if out.is_empty() {
            out.push(cubic(geometry(tx, *e)?).p0);
        }
        geometry(tx, *e)?.flatten_into(&mut out);
    }
    for (c, line) in cut {
        if out.is_empty() {
            out.push(c.p0);
        }
        if *line {
            out.push(c.p3);
        } else {
            c.flatten_into(FLATTEN_TOLERANCE_MM, &mut out);
        }
    }
    out.pop(); // back at the start
    Ok(out)
}

/// New edges of `panel` along `cut`, from point `from` to point `to`,
/// with new points in between.
fn cut_edges(
    tx: &mut Transaction<'_>,
    panel: PanelId,
    cut: &[(CubicBez, bool)],
    from: PointId,
    to: PointId,
) -> Vec<EdgeId> {
    let mut start = from;
    let n = cut.len();
    cut.iter()
        .enumerate()
        .map(|(k, (c, line))| {
            let end = if k + 1 == n {
                to
            } else {
                let grade = tx.project().grade_at(panel, c.p3);
                let id = tx.new_point_id();
                tx.put_point(
                    id,
                    Point {
                        panel,
                        pos: c.p3,
                        grade,
                    },
                );
                id
            };
            let id = tx.new_edge_id();
            let curve = if *line {
                EdgeCurve::Line
            } else {
                EdgeCurve::Cubic { c1: c.p1, c2: c.p2 }
            };
            tx.put_edge(id, Edge::new(panel, start, end, curve));
            start = end;
            id
        })
        .collect()
}

/// The placement of a half so it starts where it was in the whole panel:
/// the same angle for the same pattern abscissa (the whole's), and its top
/// lowered by as much as it sits below the whole's top.
fn shifted(place: Placement, whole: &[DVec2], half: &[DVec2]) -> Placement {
    let (Some(w), Some(h)) = (polygon::bounding_box(whole), polygon::bounding_box(half)) else {
        return place;
    };
    Placement {
        top_cm: place.top_cm - (w.max.y - h.max.y) / 10.0,
        centre_x_mm: Some(place.centre_x_mm.unwrap_or((w.min.x + w.max.x) / 2.0)),
        ..place
    }
}
