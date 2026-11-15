//! Linked mirror copies.
//!
//! A copy is the source panel reflected across an axis, then moved by an
//! offset (see `r3d_core::MirrorLink`). Its geometry is derived data:
//! [`sync_mirrors`] recomputes it after every command, reusing the copy's
//! entity IDs when the source topology is unchanged so that unchanged copies
//! record no change at all.

use r3d_core::{
    AnyId, Edge, EdgeCurve, EdgeId, InternalLineId, MirrorLink, Notch, NotchId, Panel, PanelId,
    Point, PointId, Project, Transaction,
};
use r3d_geom::{DVec2, Mirror};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, editable_panel, get_panel, v};
use crate::error::{CommandError, Invalid, finite, non_blank};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Gap between a panel and its mirror copy when no axis is given (mm).
pub const DEFAULT_MIRROR_GAP_MM: f64 = 50.0;

/// Create a linked mirror copy of a panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelMirror {
    pub panel: PanelId,
    /// Name of the copy. Defaults to "<source name> (mirror)".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A point on the mirror axis (mm). Defaults to a vertical axis placed
    /// so the copy sits to the right of the source with a 50 mm gap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_origin: Option<Vec2>,
    /// Direction of the mirror axis. Defaults to vertical `[0, 1]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_dir: Option<Vec2>,
}

impl Described for PanelMirror {
    const SUMMARY: &'static str = "Create a linked mirror copy of a panel (e.g. left/right pieces). The copy follows every later change of its source; move it with panel.translate, or make it independent with panel.unlink. Returns the copy, then its points, then its edges.";
}

impl CommandSpec for PanelMirror {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let source = editable_panel(tx, self.panel)?;
        if let Some(n) = &self.name {
            non_blank("name", n)?;
        }
        let dir = v(self.axis_dir.unwrap_or([0.0, 1.0]));
        finite("axis_dir", &[dir.x, dir.y])?;
        if dir.length_squared() == 0.0 {
            return Err(CommandError::invalid("axis_dir", Invalid::ZeroVector));
        }
        let origin = match self.axis_origin {
            Some(o) => {
                finite("axis_origin", &o)?;
                v(o)
            }
            None => {
                let max_x = tx
                    .project()
                    .panel_bounding_box(self.panel)
                    .map(|b| b.max.x)
                    .unwrap_or_default();
                DVec2::new(max_x + DEFAULT_MIRROR_GAP_MM / 2.0, 0.0)
            }
        };
        let copy = tx.new_panel_id();
        tx.put_panel(
            copy,
            Panel {
                name: self
                    .name
                    .as_deref()
                    .map(str::trim)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("{} (mirror)", source.name)),
                contour: Vec::new(),
                mirror_of: Some(MirrorLink {
                    source: self.panel,
                    origin,
                    dir,
                    offset: DVec2::ZERO,
                }),
                ..source
            },
        );
        sync_panel(tx, copy)?;
        let synced = get_panel(tx.project(), copy)?;
        let p = tx.project();
        let mut created: Vec<AnyId> = vec![copy.into()];
        created.extend(
            synced
                .contour
                .iter()
                .filter_map(|e| p.edge(*e).map(|e| AnyId::from(e.start))),
        );
        created.extend(synced.contour.iter().map(|e| AnyId::from(*e)));
        Ok(CommandOutput { created })
    }
}

/// Make a linked mirror copy independent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelUnlink {
    pub panel: PanelId,
}

impl Described for PanelUnlink {
    const SUMMARY: &'static str = "Make a linked mirror copy independent: it keeps its current shape and can then be edited directly.";
}

impl CommandSpec for PanelUnlink {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let panel = get_panel(tx.project(), self.panel)?;
        if panel.mirror_of.is_none() {
            return Err(CommandError::invalid("panel", Invalid::NotLinked));
        }
        tx.put_panel(
            self.panel,
            Panel {
                mirror_of: None,
                ..panel
            },
        );
        Ok(CommandOutput::default())
    }
}

/// For an edge of a linked mirror copy: the source edge it mirrors, and a
/// point map (the copy's start mirrors the source's end, as reflection
/// reverses the contour). `None` for edges of ordinary panels.
pub(crate) fn mirrored_source(
    p: &Project,
    edge: EdgeId,
) -> Option<(EdgeId, [(PointId, PointId); 2])> {
    let e = p.edge(edge)?;
    let copy = p.panel(e.panel)?;
    let source = p.panel(copy.mirror_of.as_ref()?.source)?;
    let k = copy.contour.iter().position(|c| *c == edge)?;
    let n = copy.contour.len();
    let src_id = *source.contour.get(n.checked_sub(1 + k)?)?;
    let s = p.edge(src_id)?;
    Some((src_id, [(e.start, s.end), (e.end, s.start)]))
}

/// The edge mirroring `edge` across a linked mirror, either way (copy →
/// source, or source → its first copy), with the matching points.
pub(crate) fn mirror_twin(p: &Project, edge: EdgeId) -> Option<(EdgeId, [(PointId, PointId); 2])> {
    if let Some(found) = mirrored_source(p, edge) {
        return Some(found);
    }
    let panel = p.edge(edge)?.panel;
    let copy = p
        .panels()
        .values()
        .find(|c| c.mirror_of.as_ref().is_some_and(|l| l.source == panel))?;
    let j = p.panel(panel)?.contour.iter().position(|e| *e == edge)?;
    let twin = *copy.contour.get(copy.contour.len().checked_sub(1 + j)?)?;
    let (e, t) = (p.edge(edge)?, p.edge(twin)?);
    Some((twin, [(e.start, t.end), (e.end, t.start)]))
}

/// Recompute every linked mirror copy from its source.
pub(crate) fn sync_mirrors(tx: &mut Transaction<'_>) -> Result<(), CommandError> {
    let copies: Vec<PanelId> = tx
        .project()
        .panels()
        .iter()
        .filter(|(_, p)| p.mirror_of.is_some())
        .map(|(id, _)| *id)
        .collect();
    for copy in copies {
        sync_panel(tx, copy)?;
    }
    Ok(())
}

fn internal(detail: String) -> CommandError {
    CommandError::Internal { detail }
}

/// Mirror angle in degrees, rounded to 1e-9° to avoid float noise.
fn mirrored_angle(m: &Mirror, deg: f64) -> f64 {
    let g = DVec2::from_angle(deg.to_radians());
    let r = m.apply(g) - m.apply(DVec2::ZERO);
    (r.y.atan2(r.x).to_degrees() * 1e9).round() / 1e9
}

/// Existing IDs of the copy's notches and internal lines, oldest first.
fn derived_ids(
    p: &Project,
    copy: PanelId,
    contour: &[EdgeId],
) -> (Vec<NotchId>, Vec<InternalLineId>) {
    let notches = p
        .notches()
        .iter()
        .filter(|(_, n)| contour.contains(&n.edge))
        .map(|(id, _)| *id)
        .collect();
    let lines = p
        .internal_lines()
        .iter()
        .filter(|(_, l)| l.panel == copy)
        .map(|(id, _)| *id)
        .collect();
    (notches, lines)
}

/// Write `value` only if it differs, so unchanged copies record nothing.
macro_rules! put_if_changed {
    ($tx:expr, $get:ident, $put:ident, $id:expr, $value:expr) => {{
        let value = $value;
        if $tx.project().$get($id) != Some(&value) {
            $tx.$put($id, value);
        }
    }};
}

fn sync_panel(tx: &mut Transaction<'_>, copy_id: PanelId) -> Result<(), CommandError> {
    let copy = get_panel(tx.project(), copy_id)?;
    let Some(link) = copy.mirror_of.clone() else {
        return Ok(());
    };
    let source = get_panel(tx.project(), link.source)?;
    let m = Mirror::new(link.origin, link.dir)
        .ok_or_else(|| internal(format!("{copy_id} has a degenerate mirror axis")))?;
    let f = |p: DVec2| m.apply(p) + link.offset;

    // 1. Desired geometry. Reflection reverses orientation, so the loop is
    //    walked backwards to stay counter-clockwise: copy edge i is source
    //    edge n-1-i reversed, and copy point i is the end of that edge.
    let p = tx.project();
    let src_edges: Vec<Edge> = source
        .contour
        .iter()
        .map(|e| {
            p.edge(*e)
                .cloned()
                .ok_or_else(|| CommandError::not_found(*e))
        })
        .collect::<Result<_, _>>()?;
    let n = src_edges.len();
    let mut positions = Vec::with_capacity(n);
    let mut curves = Vec::with_capacity(n);
    for i in 0..n {
        let e = &src_edges[n - 1 - i];
        let end = p
            .point(e.end)
            .ok_or_else(|| CommandError::not_found(e.end))?;
        positions.push(f(end.pos));
        curves.push(match &e.curve {
            EdgeCurve::Line => EdgeCurve::Line,
            EdgeCurve::Cubic { c1, c2 } => EdgeCurve::Cubic {
                c1: f(*c2),
                c2: f(*c1),
            },
        });
    }
    // Notches: (copy edge index, offset from that edge's start).
    let notch_specs: Vec<(usize, f64)> = p
        .notches()
        .values()
        .filter_map(|nt| {
            let j = source.contour.iter().position(|e| *e == nt.edge)?;
            let len = p.edge_length(nt.edge)?;
            Some((n - 1 - j, (len - nt.offset_mm).max(0.0)))
        })
        .collect();
    let lines: Vec<_> = p
        .internal_lines()
        .values()
        .filter(|l| l.panel == link.source)
        .map(|l| super::internal_line::mapped(l, f))
        .collect();

    // 2. IDs: reuse the copy's entities when the topology is unchanged,
    //    otherwise replace them.
    let (point_ids, edge_ids): (Vec<PointId>, Vec<EdgeId>) = if copy.contour.len() == n {
        let starts = copy
            .contour
            .iter()
            .map(|e| {
                p.edge(*e)
                    .map(|e| e.start)
                    .ok_or_else(|| CommandError::not_found(*e))
            })
            .collect::<Result<_, _>>()?;
        (starts, copy.contour.clone())
    } else {
        let (old_notches, _) = derived_ids(p, copy_id, &copy.contour);
        // The copy's edges are replaced: seams on them cannot follow.
        let old_seams: Vec<_> = p
            .seams()
            .iter()
            .filter(|(_, s)| copy.contour.iter().any(|e| s.touches(*e)))
            .map(|(id, _)| *id)
            .collect();
        for id in old_notches {
            tx.remove_notch(id);
        }
        for id in old_seams {
            tx.remove_seam(id);
        }
        for e in &copy.contour {
            if let Some(edge) = tx.remove_edge(*e) {
                tx.remove_point(edge.start);
            }
        }
        let pts = (0..n).map(|_| tx.new_point_id()).collect();
        let edges = (0..n).map(|_| tx.new_edge_id()).collect();
        (pts, edges)
    };

    // 3. Write what changed.
    for (id, pos) in point_ids.iter().zip(&positions) {
        put_if_changed!(
            tx,
            point,
            put_point,
            *id,
            Point {
                panel: copy_id,
                pos: *pos
            }
        );
    }
    for (i, (id, curve)) in edge_ids.iter().zip(curves).enumerate() {
        let edge = Edge {
            panel: copy_id,
            start: point_ids[i],
            end: point_ids[(i + 1) % n],
            curve,
        };
        put_if_changed!(tx, edge, put_edge, *id, edge);
    }
    put_if_changed!(
        tx,
        panel,
        put_panel,
        copy_id,
        Panel {
            contour: edge_ids.clone(),
            grain_angle_deg: mirrored_angle(&m, source.grain_angle_deg),
            seam_allowance_mm: source.seam_allowance_mm,
            ..copy
        }
    );

    let (old_notches, old_lines) = derived_ids(tx.project(), copy_id, &edge_ids);
    for (k, (i, offset_mm)) in notch_specs.iter().enumerate() {
        let notch = Notch {
            edge: edge_ids[*i],
            offset_mm: *offset_mm,
        };
        match old_notches.get(k) {
            Some(id) => put_if_changed!(tx, notch, put_notch, *id, notch),
            None => {
                let id = tx.new_notch_id();
                tx.put_notch(id, notch);
            }
        }
    }
    for id in old_notches.iter().skip(notch_specs.len()) {
        tx.remove_notch(*id);
    }
    let line_count = lines.len();
    for (k, line) in lines.into_iter().enumerate() {
        let line = r3d_core::InternalLine {
            panel: copy_id,
            ..line
        };
        match old_lines.get(k) {
            Some(id) => put_if_changed!(tx, internal_line, put_internal_line, *id, line),
            None => {
                let id = tx.new_internal_line_id();
                tx.put_internal_line(id, line);
            }
        }
    }
    for id in old_lines.iter().skip(line_count) {
        tx.remove_internal_line(*id);
    }
    Ok(())
}
