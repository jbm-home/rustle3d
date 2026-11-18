//! Symmetric pieces: half of a piece is drawn against an axis, the other
//! half follows (see `r3d_core::Symmetry`). [`sync_symmetry`] recomputes
//! the reflected half after every step, keeping its IDs, so seams on it
//! stay; notches on the drawn half are reflected too.

use std::collections::{BTreeMap, BTreeSet};

use r3d_core::{
    AnyId, Edge, EdgeCurve, EdgeId, Notch, NotchId, Panel, PanelId, Point, PointId, Symmetry,
    Transaction,
};
use r3d_geom::Mirror;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{editable_panel, get_panel, move_points, not_sewn};
use crate::error::{CommandError, Invalid};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Unfold a half piece on one of its edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelUnfold {
    pub panel: PanelId,
    /// A straight edge of the piece: the axis (a centre front on the fold).
    pub edge: EdgeId,
}

impl Described for PanelUnfold {
    const SUMMARY: &'static str = "Make a half piece whole and symmetric: it is unfolded across one of its straight edges (the axis, e.g. a centre front), which disappears. The other half follows every later change of the drawn half (points, curves, notches) and cannot be edited itself; seams and notches can go on it. panel.unlink makes the halves independent. Returns the new points and edges of the reflected half.";
}

impl CommandSpec for PanelUnfold {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let panel = editable_panel(tx, self.panel)?;
        if panel.symmetry.is_some() {
            return Err(CommandError::invalid("panel", Invalid::Symmetric));
        }
        let k = panel
            .contour
            .iter()
            .position(|e| *e == self.edge)
            .ok_or_else(|| CommandError::invalid("edge", Invalid::OtherPanel))?;
        let edge = tx
            .project()
            .edge(self.edge)
            .cloned()
            .ok_or_else(|| CommandError::not_found(self.edge))?;
        if !matches!(edge.curve, EdgeCurve::Line) {
            return Err(CommandError::invalid("edge", Invalid::NotStraight));
        }
        not_sewn(tx, self.edge)?;
        let p = tx.project();
        let pos = |id: PointId| p.point(id).map(|p| p.pos).unwrap_or_default();
        let (origin, to) = (pos(edge.start), pos(edge.end));
        let dir = to - origin;
        if dir.length_squared() == 0.0 {
            return Err(CommandError::invalid("edge", Invalid::ZeroVector));
        }
        // The half must lie on one side of the axis (its left: contours
        // turn counter-clockwise), or the whole would cross itself.
        let outline = p
            .panel_outline(self.panel)
            .ok_or_else(|| CommandError::not_found(self.panel))?;
        let unit = dir.normalize();
        if outline.iter().any(|q| unit.perp_dot(*q - origin) < -1e-6) {
            return Err(CommandError::invalid("edge", Invalid::NotSimple));
        }
        let n = panel.contour.len();
        let half: Vec<EdgeId> = (1..n)
            .filter_map(|i| panel.contour.get((k + i) % n).copied())
            .collect();
        let notches: Vec<NotchId> = p
            .notches()
            .iter()
            .filter(|(_, nt)| nt.edge == self.edge)
            .map(|(id, _)| *id)
            .collect();
        for id in notches {
            tx.remove_notch(id);
        }
        tx.remove_edge(self.edge);
        tx.put_panel(
            self.panel,
            Panel {
                contour: half,
                symmetry: Some(Symmetry {
                    origin,
                    dir,
                    start: edge.end,
                    end: edge.start,
                    points: Vec::new(),
                    edges: Vec::new(),
                }),
                ..panel
            },
        );
        sync_panel(tx, self.panel)?;
        let sym = get_panel(tx.project(), self.panel)?
            .symmetry
            .unwrap_or_else(|| Symmetry {
                origin,
                dir,
                start: edge.end,
                end: edge.start,
                points: Vec::new(),
                edges: Vec::new(),
            });
        let mut created: Vec<AnyId> = sym.points.iter().map(|[_, r]| (*r).into()).collect();
        created.extend(sym.edges.iter().map(|[_, r]| AnyId::from(*r)));
        Ok(CommandOutput { created })
    }
}

/// Recompute every symmetric piece's reflected half.
pub(crate) fn sync_symmetry(tx: &mut Transaction<'_>) -> Result<(), CommandError> {
    let panels: Vec<PanelId> = tx
        .project()
        .panels()
        .iter()
        .filter(|(_, p)| p.symmetry.is_some())
        .map(|(id, _)| *id)
        .collect();
    for id in panels {
        sync_panel(tx, id)?;
    }
    Ok(())
}

fn internal(detail: String) -> CommandError {
    CommandError::Internal { detail }
}

/// The drawn half: the contour edges from the one leaving `start` to the
/// one arriving at `end`.
fn drawn_half(
    tx: &Transaction<'_>,
    panel: &Panel,
    sym: &Symmetry,
) -> Result<Vec<EdgeId>, CommandError> {
    let p = tx.project();
    let reflected: BTreeSet<EdgeId> = sym.edges.iter().map(|[_, r]| *r).collect();
    let n = panel.contour.len();
    let first = panel
        .contour
        .iter()
        .position(|e| !reflected.contains(e) && p.edge(*e).is_some_and(|e| e.start == sym.start))
        .ok_or_else(|| internal(format!("{}: no edge leaves the axis", sym.start)))?;
    let mut half = Vec::new();
    for i in 0..n {
        let id = panel
            .contour
            .get((first + i) % n)
            .copied()
            .ok_or_else(|| internal("contour".into()))?;
        if reflected.contains(&id) {
            return Err(internal(format!("{id} is reflected and drawn")));
        }
        half.push(id);
        if p.edge(id).is_some_and(|e| e.end == sym.end) {
            return Ok(half);
        }
    }
    Err(internal(format!("{}: no edge reaches the axis", sym.end)))
}

/// Write `value` only if it differs, so unchanged halves record nothing.
macro_rules! put_if_changed {
    ($tx:expr, $get:ident, $put:ident, $id:expr, $value:expr) => {{
        let value = $value;
        if $tx.project().$get($id) != Some(&value) {
            $tx.$put($id, value);
        }
    }};
}

fn sync_panel(tx: &mut Transaction<'_>, id: PanelId) -> Result<(), CommandError> {
    let panel = get_panel(tx.project(), id)?;
    let Some(sym) = panel.symmetry.clone() else {
        return Ok(());
    };
    let m = Mirror::new(sym.origin, sym.dir)
        .ok_or_else(|| internal(format!("{id} has a degenerate axis")))?;
    let half = drawn_half(tx, &panel, &sym)?;

    // 1. The drawn half's ends stay on the axis.
    let unit = sym.dir.normalize_or_zero();
    let mut onto = BTreeMap::new();
    for end in [sym.start, sym.end] {
        let at = tx
            .project()
            .point(end)
            .ok_or_else(|| CommandError::not_found(end))?
            .pos;
        let on = sym.origin + unit * unit.dot(at - sym.origin);
        if on.distance(at) > 1e-9 {
            onto.insert(end, on - at);
        }
    }
    if !onto.is_empty() {
        move_points(tx, &onto);
    }

    // 2. The reflection: edge for edge backwards, with their twins' IDs.
    let old_points: BTreeMap<PointId, PointId> = sym.points.iter().map(|[a, b]| (*a, *b)).collect();
    let old_edges: BTreeMap<EdgeId, EdgeId> = sym.edges.iter().map(|[a, b]| (*a, *b)).collect();
    let edges: Vec<Edge> = half
        .iter()
        .map(|e| {
            tx.project()
                .edge(*e)
                .cloned()
                .ok_or_else(|| CommandError::not_found(*e))
        })
        .collect::<Result<_, _>>()?;
    let mut point_twins: Vec<[PointId; 2]> = Vec::new();
    for e in edges.iter().skip(1) {
        let kept = old_points
            .get(&e.start)
            .copied()
            .filter(|t| tx.project().point(*t).is_some());
        let twin = match kept {
            Some(t) => t,
            None => tx.new_point_id(),
        };
        point_twins.push([e.start, twin]);
    }
    let twin_of = |pt: PointId| -> PointId {
        point_twins
            .iter()
            .find(|[a, _]| *a == pt)
            .map_or(pt, |[_, b]| *b)
    };
    for [a, b] in &point_twins {
        let src = tx
            .project()
            .point(*a)
            .cloned()
            .ok_or_else(|| CommandError::not_found(*a))?;
        let point = Point {
            panel: id,
            pos: m.apply(src.pos),
            grade: src.grade.iter().map(|g| m.apply_vector(*g)).collect(),
        };
        put_if_changed!(tx, point, put_point, *b, point);
    }
    let mut edge_twins: Vec<[EdgeId; 2]> = Vec::new();
    for e in &half {
        let kept = old_edges
            .get(e)
            .copied()
            .filter(|t| tx.project().edge(*t).is_some());
        let twin = match kept {
            Some(t) => t,
            None => tx.new_edge_id(),
        };
        edge_twins.push([*e, twin]);
    }
    for (e, [_, twin]) in edges.iter().zip(&edge_twins) {
        let edge = Edge {
            panel: id,
            start: twin_of(e.end),
            end: twin_of(e.start),
            curve: match &e.curve {
                EdgeCurve::Line => EdgeCurve::Line,
                EdgeCurve::Cubic { c1, c2 } => EdgeCurve::Cubic {
                    c1: m.apply(*c2),
                    c2: m.apply(*c1),
                },
            },
            grade: e
                .grade
                .iter()
                .map(|[g1, g2]| [m.apply_vector(*g2), m.apply_vector(*g1)])
                .collect(),
        };
        put_if_changed!(tx, edge, put_edge, *twin, edge);
    }

    // 3. What the old reflection had and the new one does not: gone, with
    //    the seams and notches on it.
    let kept_edges: BTreeSet<EdgeId> = edge_twins.iter().map(|[_, t]| *t).collect();
    let kept_points: BTreeSet<PointId> = point_twins.iter().map(|[_, t]| *t).collect();
    for t in old_edges.values().filter(|t| !kept_edges.contains(t)) {
        let p = tx.project();
        let seams: Vec<_> = p
            .seams()
            .iter()
            .filter(|(_, s)| s.touches(*t))
            .map(|(id, _)| *id)
            .collect();
        let notches: Vec<_> = p
            .notches()
            .iter()
            .filter(|(_, n)| n.edge == *t)
            .map(|(id, _)| *id)
            .collect();
        for s in seams {
            tx.remove_seam(s);
        }
        for n in notches {
            tx.remove_notch(n);
        }
        tx.remove_edge(*t);
    }
    for t in old_points.values().filter(|t| !kept_points.contains(t)) {
        tx.remove_point(*t);
    }

    // 4. The contour: the drawn half, then its reflection backwards.
    let mut contour = half.clone();
    contour.extend(edge_twins.iter().rev().map(|[_, t]| *t));
    put_if_changed!(
        tx,
        panel,
        put_panel,
        id,
        Panel {
            contour,
            symmetry: Some(Symmetry {
                points: point_twins,
                edges: edge_twins.clone(),
                ..sym
            }),
            ..panel
        }
    );

    // 5. Notches: those of the drawn half, reflected onto the twins (as
    //    far from the axis as the originals).
    let p = tx.project();
    let mut wanted: Vec<Notch> = Vec::new();
    for [e, twin] in &edge_twins {
        let len = p.edge_length(*e).unwrap_or_default();
        for n in p.notches().values().filter(|n| n.edge == *e) {
            wanted.push(Notch {
                edge: *twin,
                offset_mm: (len - n.offset_mm).max(0.0),
            });
        }
    }
    let existing: Vec<NotchId> = p
        .notches()
        .iter()
        .filter(|(_, n)| kept_edges.contains(&n.edge))
        .map(|(id, _)| *id)
        .collect();
    for (k, notch) in wanted.iter().enumerate() {
        match existing.get(k) {
            Some(nid) => put_if_changed!(tx, notch, put_notch, *nid, notch.clone()),
            None => {
                let nid = tx.new_notch_id();
                tx.put_notch(nid, notch.clone());
            }
        }
    }
    for nid in existing.iter().skip(wanted.len()) {
        tx.remove_notch(*nid);
    }
    Ok(())
}
