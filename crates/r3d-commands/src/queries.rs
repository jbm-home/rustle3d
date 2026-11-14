//! Read-only queries. Lengths in mm and areas in mm², rounded to 0.001.

use r3d_core::measure::EdgeGeometry;
use r3d_core::{AnyId, DisplayUnit, EdgeId, GarmentId, PanelId, PointId, Project};
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
struct PanelOut {
    #[serde(flatten)]
    summary: PanelSummary,
    grain_angle_deg: f64,
    edges: Vec<EdgeOut>,
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
        to_value(&PanelOut {
            summary: summarize(p, self.panel)?,
            grain_angle_deg: panel.grain_angle_deg,
            edges,
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
