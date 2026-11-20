use r3d_core::measure::FLATTEN_TOLERANCE_MM;
use r3d_core::{
    AnyId, Around, Edge, EdgeCurve, MirrorLink, Panel, PanelId, PathSegment, Placement, Point,
    Transaction,
};
use r3d_geom::{CubicBez, DVec2, polygon};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::internal_line::path_segments;
use super::{SegmentSpec, Vec2, editable_panel, get_panel, v};
use crate::error::{CommandError, Invalid, finite, non_blank, non_negative, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Default grainline: vertical on the board, the usual layout for garments.
pub const DEFAULT_GRAIN_ANGLE_DEG: f64 = 90.0;

pub(crate) fn default_grain() -> f64 {
    DEFAULT_GRAIN_ANGLE_DEG
}

/// A closed outline: each point with the shape of the edge from it to the
/// next point (the last edge returns to the first point).
pub(crate) type Outline = Vec<(DVec2, EdgeCurve)>;

/// The outline flattened into a polygon (mm).
fn flattened(outline: &Outline) -> Vec<DVec2> {
    let mut out = Vec::new();
    for (i, (a, curve)) in outline.iter().enumerate() {
        out.push(*a);
        if let (EdgeCurve::Cubic { c1, c2 }, Some((b, _))) =
            (curve, outline.get((i + 1) % outline.len()))
        {
            CubicBez::new(*a, *c1, *c2, *b).flatten_into(FLATTEN_TOLERANCE_MM, &mut out);
            out.pop(); // the next point comes with the next edge
        }
    }
    out
}

/// The same outline walked the other way: the points in reverse order,
/// each edge reversed.
fn reversed(outline: Outline) -> Outline {
    let n = outline.len();
    (0..n)
        .map(|j| {
            let (pos, _) = &outline[n - 1 - j];
            // The edge from here to the next point was the edge before it.
            let (_, curve) = &outline[(2 * n - 2 - j) % n];
            let curve = match curve {
                EdgeCurve::Line => EdgeCurve::Line,
                EdgeCurve::Cubic { c1, c2 } => EdgeCurve::Cubic { c1: *c2, c2: *c1 },
            };
            (*pos, curve)
        })
        .collect()
}

/// Create a panel from a closed outline. Shared by the rectangle, polygon
/// and path commands.
pub(crate) fn create_panel(
    tx: &mut Transaction<'_>,
    name: Option<&str>,
    mut outline: Outline,
    grain_angle_deg: f64,
) -> Result<CommandOutput, CommandError> {
    finite("grain_angle_deg", &[grain_angle_deg])?;
    if let Some(n) = name {
        non_blank("name", n)?;
    }
    let curves_finite = outline.iter().all(|(p, c)| {
        p.is_finite()
            && match c {
                EdgeCurve::Line => true,
                EdgeCurve::Cubic { c1, c2 } => c1.is_finite() && c2.is_finite(),
            }
    });
    if !curves_finite {
        return Err(CommandError::invalid("points", Invalid::NotFinite));
    }
    let flat = flattened(&outline);
    if flat.len() < 3 {
        return Err(CommandError::invalid("points", Invalid::TooFewPoints(3)));
    }
    // A simple polygon (no crossing, overlapping or repeated edges, which
    // also rules out collinear outlines) always has a non-zero area.
    if !polygon::is_simple(&flat) {
        return Err(CommandError::invalid("points", Invalid::NotSimple));
    }
    if polygon::signed_area(&flat) < 0.0 {
        outline = reversed(outline); // store contours counter-clockwise
    }
    let garment = tx
        .project()
        .default_garment()
        .ok_or_else(|| CommandError::Internal {
            detail: "project has no garment".into(),
        })?;
    // English default name (i18n: see docs/architecture.md §7.7).
    let name = match name {
        Some(n) => n.trim().to_owned(),
        None => format!("Panel {}", tx.project().panels().len() + 1),
    };

    let panel = tx.new_panel_id();
    let point_ids: Vec<_> = outline
        .iter()
        .map(|&(pos, _)| {
            let id = tx.new_point_id();
            tx.put_point(id, Point::new(panel, pos));
            id
        })
        .collect();
    let n = point_ids.len();
    let edge_ids: Vec<_> = (0..n)
        .map(|i| {
            let id = tx.new_edge_id();
            tx.put_edge(
                id,
                Edge::new(
                    panel,
                    point_ids[i],
                    point_ids[(i + 1) % n],
                    outline[i].1.clone(),
                ),
            );
            id
        })
        .collect();
    tx.put_panel(
        panel,
        Panel {
            name,
            garment,
            contour: edge_ids.clone(),
            grain_angle_deg,
            seam_allowance_mm: 0.0,
            mirror_of: None,
            placement: None,
            symmetry: None,
            holes: Vec::new(),
            print: Default::default(),
        },
    );
    let mut created: Vec<AnyId> = vec![panel.into()];
    created.extend(point_ids.into_iter().map(AnyId::from));
    created.extend(edge_ids.into_iter().map(AnyId::from));
    Ok(CommandOutput { created })
}

/// Create a rectangular panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCreateRect {
    /// Panel name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Bottom-left corner on the board, in mm. Defaults to `[0, 0]`.
    #[serde(default)]
    pub origin: Vec2,
    /// Width in mm (> 0).
    pub width: f64,
    /// Height in mm (> 0).
    pub height: f64,
    /// Grainline angle in degrees from +X. Defaults to 90 (vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
}

impl Described for PanelCreateRect {
    const SUMMARY: &'static str = "Create a rectangular panel. Returns the panel, then its 4 points (counter-clockwise from the origin), then its 4 edges.";
}

impl CommandSpec for PanelCreateRect {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("origin", &self.origin)?;
        positive("width", self.width)?;
        positive("height", self.height)?;
        let o = v(self.origin);
        let pts = [
            o,
            o + DVec2::new(self.width, 0.0),
            o + DVec2::new(self.width, self.height),
            o + DVec2::new(0.0, self.height),
        ];
        let outline = pts.into_iter().map(|p| (p, EdgeCurve::Line)).collect();
        create_panel(tx, self.name.as_deref(), outline, self.grain_angle_deg)
    }
}

/// Create a panel from a closed polygon.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCreatePolygon {
    /// Panel name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// At least 3 vertices in mm; the closing edge is implicit. Must form a
    /// simple outline (no crossing or overlapping edges). Stored
    /// counter-clockwise.
    pub points: Vec<Vec2>,
    /// Grainline angle in degrees from +X. Defaults to 90 (vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
    /// The points draw half a piece: the closing edge, from the last point
    /// back to the first, is its axis, and the piece is unfolded across it
    /// (as panel.unfold does).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub symmetric: bool,
}

impl Described for PanelCreatePolygon {
    const SUMMARY: &'static str = "Create a panel from a closed polygon. With `symmetric`, the points draw half a piece against an axis, the closing edge: the other half follows (panel.unfold). Returns the panel, then its points, then its edges (then, symmetric, the reflected half's points and edges).";
}

/// Unfold a piece just drawn (`create_panel`'s output) across its closing
/// edge, the one back to the first point (`symmetric` drawing), adding the
/// reflected half to `out`.
fn unfold_closing(
    tx: &mut Transaction<'_>,
    mut out: CommandOutput,
) -> Result<CommandOutput, CommandError> {
    // Created last, whichever way round the outline was stored.
    let (Some(AnyId::Panel(panel)), Some(AnyId::Edge(edge))) =
        (out.created.first().copied(), out.created.last().copied())
    else {
        return Ok(out);
    };
    if !matches!(
        tx.project().edge(edge).map(|e| &e.curve),
        Some(EdgeCurve::Line)
    ) {
        return Err(CommandError::invalid("symmetric", Invalid::NotStraight));
    }
    let unfolded = super::PanelUnfold { panel, edge }.apply(tx)?;
    out.created.retain(|id| *id != AnyId::Edge(edge));
    out.created.extend(unfolded.created);
    Ok(out)
}

impl CommandSpec for PanelCreatePolygon {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let outline = self
            .points
            .iter()
            .map(|p| (v(*p), EdgeCurve::Line))
            .collect();
        let out = create_panel(tx, self.name.as_deref(), outline, self.grain_angle_deg)?;
        if self.symmetric {
            unfold_closing(tx, out)
        } else {
            Ok(out)
        }
    }
}

/// Create a panel from a closed path of straight and curved edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCreatePath {
    /// Panel name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// First point of the outline (mm).
    pub start: Vec2,
    /// The edges in order, each starting where the previous one ended. The
    /// last one should end at `start`; if it does not, a straight edge
    /// closes the outline.
    pub segments: Vec<SegmentSpec>,
    /// Grainline angle in degrees from +X. Defaults to 90 (vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
    /// The path draws half a piece: the straight edge back to `start` is
    /// its axis, and the piece is unfolded across it (as panel.unfold
    /// does).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub symmetric: bool,
}

impl Described for PanelCreatePath {
    const SUMMARY: &'static str = "Create a panel from a closed path of straight and cubic Bézier edges (mm), like internal_line.add but closed. Must form a simple outline; stored counter-clockwise. With `symmetric`, the path draws half a piece against an axis, the straight edge back to `start`: the other half follows (panel.unfold). Returns the panel, then its points, then its edges (then, symmetric, the reflected half's points and edges).";
}

/// Two outline points closer than this are the same point (mm).
const SAME_POINT_MM: f64 = 1e-3;

impl CommandSpec for PanelCreatePath {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("start", &self.start)?;
        let segments = path_segments("segments", &self.segments)?;
        let start = v(self.start);
        let mut outline: Outline = Vec::with_capacity(segments.len() + 1);
        let mut at = start;
        for seg in &segments {
            let curve = match seg {
                PathSegment::Line { .. } => EdgeCurve::Line,
                PathSegment::Cubic { c1, c2, .. } => EdgeCurve::Cubic { c1: *c1, c2: *c2 },
            };
            outline.push((at, curve));
            at = seg.end();
        }
        if at.distance(start) > SAME_POINT_MM {
            outline.push((at, EdgeCurve::Line));
        }
        let out = create_panel(tx, self.name.as_deref(), outline, self.grain_angle_deg)?;
        if self.symmetric {
            unfold_closing(tx, out)
        } else {
            Ok(out)
        }
    }
}

/// Rename a panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelRename {
    pub panel: PanelId,
    /// New name.
    pub name: String,
}

impl Described for PanelRename {
    const SUMMARY: &'static str = "Rename a panel.";
}

impl CommandSpec for PanelRename {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        non_blank("name", &self.name)?;
        let mut p = get_panel(tx.project(), self.panel)?;
        p.name = self.name.trim().to_owned();
        tx.put_panel(self.panel, p);
        Ok(CommandOutput::default())
    }
}

/// Move a whole panel on the board.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelTranslate {
    pub panel: PanelId,
    /// Displacement in mm.
    pub delta: Vec2,
}

impl Described for PanelTranslate {
    const SUMMARY: &'static str = "Move a whole panel on the board by a displacement in mm. Works on linked mirror copies too (only the copy moves).";
}

impl CommandSpec for PanelTranslate {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("delta", &self.delta)?;
        let d = v(self.delta);
        let panel = get_panel(tx.project(), self.panel)?;
        // A linked copy is placed by its link's offset; the sync pass then
        // moves its geometry.
        if let Some(link) = &panel.mirror_of {
            let mut moved = panel.clone();
            moved.mirror_of = Some(MirrorLink {
                offset: link.offset + d,
                ..link.clone()
            });
            if let Some(place) = &mut moved.placement {
                place.centre_x_mm = place.centre_x_mm.map(|x| x + d.x);
            }
            tx.put_panel(self.panel, moved);
            return Ok(CommandOutput::default());
        }
        super::transform::map_panel(tx, self.panel, panel, glam::DAffine2::from_translation(d))?;
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.translate:{}", self.panel))
    }
}

/// Set the grainline direction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelSetGrain {
    pub panel: PanelId,
    /// Angle in degrees from the board's +X axis (90 = vertical).
    pub angle_deg: f64,
}

impl Described for PanelSetGrain {
    const SUMMARY: &'static str = "Set a panel's grainline (warp direction), in degrees from +X.";
}

impl CommandSpec for PanelSetGrain {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("angle_deg", &[self.angle_deg])?;
        let mut p = editable_panel(tx, self.panel)?;
        p.grain_angle_deg = self.angle_deg;
        tx.put_panel(self.panel, p);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.set_grain:{}", self.panel))
    }
}

/// Set the seam allowance drawn around the panel on export.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelSetSeamAllowance {
    pub panel: PanelId,
    /// Width in mm (≥ 0; 0 = no allowance).
    pub mm: f64,
}

impl Described for PanelSetSeamAllowance {
    const SUMMARY: &'static str =
        "Set the seam allowance (mm, 0 = none) drawn around a panel when exporting.";
}

impl CommandSpec for PanelSetSeamAllowance {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        non_negative("mm", self.mm)?;
        let mut p = editable_panel(tx, self.panel)?;
        p.seam_allowance_mm = self.mm;
        tx.put_panel(self.panel, p);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.set_seam_allowance:{}", self.panel))
    }
}

/// Delete a panel with everything attached to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelDelete {
    pub panel: PanelId,
    /// Must be `true`: deleting is destructive (still undoable).
    #[serde(default)]
    pub confirm: bool,
}

impl Described for PanelDelete {
    const SUMMARY: &'static str = "Delete a panel with its points, edges, notches and internal lines. Linked mirror copies of it become independent. Destructive: requires \"confirm\": true.";
}

impl CommandSpec for PanelDelete {
    const DESTRUCTIVE: bool = true;

    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let panel = get_panel(tx.project(), self.panel)?;
        let p = tx.project();
        let notches: Vec<_> = p
            .notches()
            .iter()
            .filter(|(_, n)| panel.contour.contains(&n.edge))
            .map(|(id, _)| *id)
            .collect();
        let lines: Vec<_> = p
            .internal_lines()
            .iter()
            .filter(|(_, l)| l.panel == self.panel)
            .map(|(id, _)| *id)
            .collect();
        let pins: Vec<_> = p
            .pins()
            .iter()
            .filter(|(_, pin)| pin.panel == self.panel)
            .map(|(id, _)| *id)
            .collect();
        let seams: Vec<_> = p
            .seams()
            .iter()
            .filter(|(_, s)| panel.contour.iter().any(|e| s.touches(*e)))
            .map(|(id, _)| *id)
            .collect();
        let copies: Vec<_> = p
            .panels()
            .iter()
            .filter(|(_, c)| c.mirror_of.as_ref().is_some_and(|l| l.source == self.panel))
            .map(|(id, c)| (*id, c.clone()))
            .collect();
        for id in notches {
            tx.remove_notch(id);
        }
        for id in seams {
            tx.remove_seam(id);
        }
        for id in pins {
            tx.remove_pin(id);
        }
        for id in lines {
            tx.remove_internal_line(id);
        }
        for (id, copy) in copies {
            tx.put_panel(
                id,
                Panel {
                    mirror_of: None,
                    ..copy
                },
            );
        }
        for eid in &panel.contour {
            if let Some(edge) = tx.remove_edge(*eid) {
                tx.remove_point(edge.start);
            }
        }
        tx.remove_panel(self.panel);
        Ok(CommandOutput::default())
    }

    fn confirmed(&self) -> bool {
        self.confirm
    }
}

/// Place a panel around the body for the simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelPlace {
    pub panel: PanelId,
    /// What the panel is wrapped around: `body` (default), `left_arm` or
    /// `right_arm` (sleeves), `left_leg` or `right_leg` (trouser legs:
    /// around the leg, and above the crotch around that half of the body),
    /// or `flat` (not wrapped: a flat sheet facing outward, `offset_mm`
    /// from the body's axis).
    #[serde(default)]
    pub around: Around,
    /// Angle around the body (degrees): 0 front, 90 the body's left side,
    /// 180 back, 270 right side. Around an arm: 0 the outside of the arm,
    /// 180 the side facing the body. Around a leg, as around the body,
    /// centred on the leg.
    pub angle_deg: f64,
    /// Height of the panel's top above the floor (cm). Around an arm, the
    /// top goes to the arm at that height and the panel runs down the arm.
    pub top_cm: f64,
    /// Distance from the body at the start of the simulation (mm,
    /// default 30). Around an arm or a leg a panel starts as a closed tube
    /// instead (around a leg, just outside it if the leg is thicker).
    #[serde(default = "default_offset")]
    pub offset_mm: f64,
    /// The pattern abscissa (mm) that goes to the angle. Defaults to the
    /// panel's horizontal centre; the pieces of a cut panel keep the
    /// whole's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre_x_mm: Option<f64>,
}

fn default_offset() -> f64 {
    30.0
}

impl Described for PanelPlace {
    const SUMMARY: &'static str = "Place a panel around the body (or around an arm, for sleeves) for the 3D simulation: angle (0 front, 90 left side, 180 back, 270 right side; around an arm 0 is its outside), height of its top in cm, distance from the body in mm. Unplaced panels are not simulated.";
}

impl CommandSpec for PanelPlace {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("angle_deg", &[self.angle_deg])?;
        finite(
            "centre_x_mm",
            &self.centre_x_mm.into_iter().collect::<Vec<_>>(),
        )?;
        positive("top_cm", self.top_cm)?;
        non_negative("offset_mm", self.offset_mm)?;
        let mut p = get_panel(tx.project(), self.panel)?;
        p.placement = Some(Placement {
            around: self.around,
            angle_deg: self.angle_deg.rem_euclid(360.0),
            top_cm: self.top_cm,
            offset_mm: self.offset_mm,
            centre_x_mm: self.centre_x_mm,
        });
        tx.put_panel(self.panel, p);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("panel.place:{}", self.panel))
    }
}
