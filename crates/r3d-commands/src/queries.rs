//! Read-only queries. Lengths in mm and areas in mm², rounded to 0.001.

use r3d_core::measure::EdgeGeometry;
use r3d_core::{
    AnyId, DisplayUnit, EdgeId, GarmentId, InternalLineId, NotchId, PanelId, PathSegment, PointId,
    Project,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::Vec2;
use crate::error::{CommandError, Invalid};
use crate::spec::{Described, QuerySpec, to_value};

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn r3v(p: r3d_geom::DVec2) -> Vec2 {
    [r3(p.x), r3(p.y)]
}

fn missing(id: impl Into<AnyId>) -> CommandError {
    CommandError::not_found(id)
}

/// Bounding box `{min: [x, y], max: [x, y]}` in mm.
#[derive(Clone, Debug, Serialize)]
pub struct BBox {
    pub min: Vec2,
    pub max: Vec2,
}

/// Overview of one panel.
#[derive(Clone, Debug, Serialize)]
pub struct PanelSummary {
    pub id: PanelId,
    pub name: String,
    pub garment: GarmentId,
    pub point_count: usize,
    pub area_mm2: f64,
    pub perimeter_mm: f64,
    pub bbox: Option<BBox>,
    /// `false` if the contour crosses itself.
    pub is_simple: bool,
}

fn summarize(p: &Project, id: PanelId) -> Result<PanelSummary, CommandError> {
    let panel = p.panel(id).ok_or_else(|| missing(id))?;
    Ok(PanelSummary {
        id,
        name: panel.name.clone(),
        garment: panel.garment,
        point_count: panel.contour.len(),
        area_mm2: r3(p.panel_area(id).unwrap_or_default()),
        perimeter_mm: r3(p.panel_perimeter(id).unwrap_or_default()),
        bbox: p.panel_bounding_box(id).map(|b| BBox {
            min: r3v(b.min),
            max: r3v(b.max),
        }),
        is_simple: p.panel_is_simple(id).unwrap_or(false),
    })
}

/// Project name, units, counts and a summary of every panel.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectSummary {}

impl Described for ProjectSummary {
    const SUMMARY: &'static str =
        "Project name, display unit, entity counts and a summary of every panel.";
}

#[derive(Serialize)]
struct Counts {
    garments: usize,
    panels: usize,
    points: usize,
    edges: usize,
}

#[derive(Serialize)]
struct SummaryOut {
    name: String,
    display_unit: DisplayUnit,
    counts: Counts,
    panels: Vec<PanelSummary>,
}

impl QuerySpec for ProjectSummary {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let panels = p
            .panels()
            .keys()
            .map(|id| summarize(p, *id))
            .collect::<Result<_, _>>()?;
        to_value(&SummaryOut {
            name: p.name().to_owned(),
            display_unit: p.display_unit(),
            counts: Counts {
                garments: p.garments().len(),
                panels: p.panels().len(),
                points: p.points().len(),
                edges: p.edges().len(),
            },
            panels,
        })
    }
}

/// List panels with area, perimeter and bounding box.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelList {}

impl Described for PanelList {
    const SUMMARY: &'static str = "List panels with area, perimeter, bounding box and validity.";
}

impl QuerySpec for PanelList {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let list: Vec<_> = p
            .panels()
            .keys()
            .map(|id| summarize(p, *id))
            .collect::<Result<_, _>>()?;
        to_value(&list)
    }
}

/// Full geometry of one panel.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelGet {
    pub panel: PanelId,
}

impl Described for PanelGet {
    const SUMMARY: &'static str = "Full geometry of a panel: its edges in contour order with point IDs, positions, curve and length.";
}

#[derive(Serialize)]
struct PointOut {
    id: PointId,
    pos: Vec2,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CurveOut {
    Line,
    Cubic { c1: Vec2, c2: Vec2 },
}

#[derive(Serialize)]
struct EdgeOut {
    id: EdgeId,
    start: PointOut,
    end: PointOut,
    curve: CurveOut,
    length_mm: f64,
}

#[derive(Serialize)]
struct MirrorOut {
    source: PanelId,
    axis_origin: Vec2,
    axis_dir: Vec2,
    offset: Vec2,
}

#[derive(Serialize)]
struct NotchOut {
    id: NotchId,
    edge: EdgeId,
    offset_mm: f64,
    pos: Vec2,
    /// The offset exceeds the edge length; the notch sits at the edge end.
    clamped: bool,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SegmentOut {
    Line { to: Vec2 },
    Cubic { c1: Vec2, c2: Vec2, to: Vec2 },
}

#[derive(Serialize)]
struct InternalLineOut {
    id: InternalLineId,
    start: Vec2,
    segments: Vec<SegmentOut>,
}

#[derive(Serialize)]
struct PanelOut {
    #[serde(flatten)]
    summary: PanelSummary,
    grain_angle_deg: f64,
    seam_allowance_mm: f64,
    /// Present on linked mirror copies, which cannot be edited directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    mirror_of: Option<MirrorOut>,
    edges: Vec<EdgeOut>,
    notches: Vec<NotchOut>,
    internal_lines: Vec<InternalLineOut>,
}

impl QuerySpec for PanelGet {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let panel = p.panel(self.panel).ok_or_else(|| missing(self.panel))?;
        let mut edges = Vec::with_capacity(panel.contour.len());
        for eid in &panel.contour {
            let e = p.edge(*eid).ok_or_else(|| missing(*eid))?;
            let geom = p.edge_geometry(*eid).ok_or_else(|| missing(*eid))?;
            let (a, b, curve) = match geom {
                EdgeGeometry::Line(a, b) => (a, b, CurveOut::Line),
                EdgeGeometry::Cubic(c) => (
                    c.p0,
                    c.p3,
                    CurveOut::Cubic {
                        c1: r3v(c.p1),
                        c2: r3v(c.p2),
                    },
                ),
            };
            edges.push(EdgeOut {
                id: *eid,
                start: PointOut {
                    id: e.start,
                    pos: r3v(a),
                },
                end: PointOut {
                    id: e.end,
                    pos: r3v(b),
                },
                curve,
                length_mm: r3(geom.length()),
            });
        }
        let notches = p
            .notches()
            .iter()
            .filter(|(_, n)| panel.contour.contains(&n.edge))
            .filter_map(|(id, n)| {
                let at = p.notch_placement(*id)?;
                Some(NotchOut {
                    id: *id,
                    edge: n.edge,
                    offset_mm: n.offset_mm,
                    pos: r3v(at.pos),
                    clamped: at.clamped,
                })
            })
            .collect();
        let internal_lines = p
            .internal_lines()
            .iter()
            .filter(|(_, l)| l.panel == self.panel)
            .map(|(id, l)| InternalLineOut {
                id: *id,
                start: r3v(l.start),
                segments: l
                    .segments
                    .iter()
                    .map(|s| match s {
                        PathSegment::Line { to } => SegmentOut::Line { to: r3v(*to) },
                        PathSegment::Cubic { c1, c2, to } => SegmentOut::Cubic {
                            c1: r3v(*c1),
                            c2: r3v(*c2),
                            to: r3v(*to),
                        },
                    })
                    .collect(),
            })
            .collect();
        to_value(&PanelOut {
            summary: summarize(p, self.panel)?,
            grain_angle_deg: panel.grain_angle_deg,
            seam_allowance_mm: panel.seam_allowance_mm,
            mirror_of: panel.mirror_of.as_ref().map(|l| MirrorOut {
                source: l.source,
                axis_origin: r3v(l.origin),
                axis_dir: r3v(l.dir),
                offset: r3v(l.offset),
            }),
            edges,
            notches,
            internal_lines,
        })
    }
}

/// Straight-line distance between two points.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeasureDistance {
    pub a: PointId,
    pub b: PointId,
}

impl Described for MeasureDistance {
    const SUMMARY: &'static str = "Straight-line distance between two points, in mm.";
}

impl QuerySpec for MeasureDistance {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        if self.a == self.b {
            return Err(CommandError::invalid("b", Invalid::SamePoint));
        }
        let a = p.point(self.a).ok_or_else(|| missing(self.a))?;
        let b = p.point(self.b).ok_or_else(|| missing(self.b))?;
        Ok(serde_json::json!({ "distance_mm": r3(a.pos.distance(b.pos)) }))
    }
}

/// Length along a panel contour between two of its points.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeasureContourLength {
    /// Start point.
    pub from: PointId,
    /// End point, on the same panel.
    pub to: PointId,
}

impl Described for MeasureContourLength {
    const SUMMARY: &'static str = "Length (mm) along a panel contour from one point to another, walking in contour order (counter-clockwise). Useful to compare seam lengths.";
}

impl QuerySpec for MeasureContourLength {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        if self.from == self.to {
            return Err(CommandError::invalid("to", Invalid::SamePoint));
        }
        let from = p.point(self.from).ok_or_else(|| missing(self.from))?;
        let to = p.point(self.to).ok_or_else(|| missing(self.to))?;
        if from.panel != to.panel {
            return Err(CommandError::invalid("to", Invalid::OtherPanel));
        }
        let panel = p.panel(from.panel).ok_or_else(|| missing(from.panel))?;
        let n = panel.contour.len();
        let start = panel
            .contour
            .iter()
            .position(|e| p.edge(*e).is_some_and(|e| e.start == self.from))
            .ok_or_else(|| missing(self.from))?;
        let mut length = 0.0;
        let mut edges = Vec::new();
        for k in 0..n {
            let id = panel.contour[(start + k) % n];
            let edge = p.edge(id).ok_or_else(|| missing(id))?;
            length += p.edge_length(id).unwrap_or_default();
            edges.push(id);
            if edge.end == self.to {
                break;
            }
        }
        Ok(serde_json::json!({ "length_mm": r3(length), "edges": edges }))
    }
}

/// The built-in body model, loaded once (it embeds a few MB of geometry).
pub fn body_model() -> Result<&'static r3d_avatar::model::BodyModel, CommandError> {
    static MODEL: std::sync::OnceLock<Result<r3d_avatar::model::BodyModel, String>> =
        std::sync::OnceLock::new();
    MODEL
        .get_or_init(|| r3d_avatar::model::BodyModel::builtin().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| CommandError::Internal { detail: e.clone() })
}

/// The avatar and what the fitted body actually measures.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarGet {}

impl Described for AvatarGet {
    const SUMMARY: &'static str = "The avatar's requested measurements, its preset, and the measurements the fitted 3D body actually reaches (they can differ slightly for extreme sizes). Null when there is no avatar.";
}

impl QuerySpec for AvatarGet {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let Some(avatar) = p.avatar() else {
            return Ok(Value::Null);
        };
        let fit = r3d_avatar::fit::fit(body_model()?, &avatar.body);
        let a = fit.achieved;
        let r1 = |v: f32| (v as f64 * 10.0).round() / 10.0;
        Ok(serde_json::json!({
            "body": avatar.body,
            "preset": avatar.preset,
            "achieved": {
                "height_cm": r1(a.height_cm),
                "bust_cm": r1(a.bust_cm),
                "waist_cm": r1(a.waist_cm),
                "hips_cm": r1(a.hips_cm),
            },
        }))
    }
}

/// The built-in body presets (standard sizes).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarPresets {}

impl Described for AvatarPresets {
    const SUMMARY: &'static str = "Built-in body presets: standard women's and men's sizes, as {name, body} pairs usable with avatar.set.";
}

impl QuerySpec for AvatarPresets {
    fn run(&self, _p: &Project) -> Result<Value, CommandError> {
        to_value(&r3d_avatar::presets::builtin())
    }
}
