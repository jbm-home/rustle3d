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
    const SUMMARY: &'static str = "Project name, display unit, the garments (fabric, colour, notes, print), the size range (names, base, shown; null without one), entity counts and a summary of every panel.";
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
    garments: Vec<Value>,
    grading: Value,
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
            // Next to the body first.
            garments: p
                .garments_by_layer()
                .into_iter()
                .filter_map(|id| Some((id, p.garments().get(&id)?)))
                .map(|(id, g)| {
                    serde_json::json!({
                        "id": id, "name": g.name, "fabric": g.fabric, "color": g.color,
                        "layer": g.layer, "notes": g.notes, "print": g.print,
                        "source": g.source.as_ref().map(|s| &s.file),
                    })
                })
                .collect(),
            grading: p.grading().map_or(Value::Null, |g| {
                let name = |i: usize| g.sizes.get(i).map(|s| s.name.clone());
                serde_json::json!({
                    "sizes": g.sizes.iter().map(|s| &s.name).collect::<Vec<_>>(),
                    "base": name(g.base),
                    "shown": name(g.shown),
                })
            }),
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
    /// A size of the project's range: the geometry in that size. Default:
    /// as drawn (the base size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

impl Described for PanelGet {
    const SUMMARY: &'static str = "Full geometry of a panel: its edges in contour order with point IDs, positions, curve and length; graded points have their `grade`, their offset in each size of the range (grading.get's order). With `size`, the geometry in that size.";
}

#[derive(Serialize)]
struct PointOut {
    id: PointId,
    pos: Vec2,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    grade: Vec<Vec2>,
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
    /// On the reflected half of a symmetric piece: it follows the drawn
    /// half and cannot be edited.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    derived: bool,
}

#[derive(Serialize)]
struct SymmetryOut {
    axis_origin: Vec2,
    axis_dir: Vec2,
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
struct PrintPlacementOut {
    offset_mm: Vec2,
    angle_deg: f64,
}

#[derive(Serialize)]
struct PanelOut {
    #[serde(flatten)]
    summary: PanelSummary,
    grain_angle_deg: f64,
    seam_allowance_mm: f64,
    /// How the piece shifts and turns its garment's print.
    print: PrintPlacementOut,
    /// Where it sits around the body for the simulation; null when the
    /// panel is not simulated.
    placement: Option<r3d_core::Placement>,
    /// Present on linked mirror copies, which cannot be edited directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    mirror_of: Option<MirrorOut>,
    /// Present on symmetric pieces (panel.unfold).
    #[serde(skip_serializing_if = "Option::is_none")]
    symmetry: Option<SymmetryOut>,
    edges: Vec<EdgeOut>,
    notches: Vec<NotchOut>,
    internal_lines: Vec<InternalLineOut>,
}

impl QuerySpec for PanelGet {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        if let Some(size) = &self.size {
            let sized = p.at_size(crate::commands::size_index(p, size.trim())?);
            return PanelGet {
                panel: self.panel,
                size: None,
            }
            .run(&sized);
        }
        let panel = p.panel(self.panel).ok_or_else(|| missing(self.panel))?;
        let grade = |id: PointId| -> Vec<Vec2> {
            p.point(id)
                .map(|pt| pt.grade.iter().map(|g| r3v(*g)).collect())
                .unwrap_or_default()
        };
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
                    grade: grade(e.start),
                },
                end: PointOut {
                    id: e.end,
                    pos: r3v(b),
                    grade: grade(e.end),
                },
                curve,
                length_mm: r3(geom.length()),
                derived: panel.mirror_of.is_none() && p.is_derived_edge(*eid),
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
            print: PrintPlacementOut {
                offset_mm: panel.print.offset_mm.to_array(),
                angle_deg: panel.print.angle_deg,
            },
            placement: panel.placement,
            mirror_of: panel.mirror_of.as_ref().map(|l| MirrorOut {
                source: l.source,
                axis_origin: r3v(l.origin),
                axis_dir: r3v(l.dir),
                offset: r3v(l.offset),
            }),
            symmetry: panel.symmetry.as_ref().map(|s| SymmetryOut {
                axis_origin: r3v(s.origin),
                axis_dir: r3v(s.dir),
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

/// The built-in body fitted to `body` and the measurements typed besides
/// (`typed`, cm; only those the body follows count). Fitting takes a few
/// seconds, so
/// the last bodies fitted are kept (a graded template drafts each size,
/// the drape fits the shown size again), and a body asked for while it
/// is being fitted waits for that fit instead of doing it again (the app
/// fits on a thread of its own while a step may need the same body).
/// Different bodies fit in parallel.
pub fn fitted(
    body: &r3d_core::Body,
    typed: &r3d_core::Measures,
) -> Result<r3d_avatar::fit::Fit, CommandError> {
    type Key = (r3d_core::Body, r3d_core::Measures);
    type Cell = std::sync::Arc<std::sync::OnceLock<r3d_avatar::fit::Fit>>;
    const KEPT: usize = 32;
    static FITS: std::sync::Mutex<Vec<(Key, Cell)>> = std::sync::Mutex::new(Vec::new());
    let model = body_model()?;
    let followed: r3d_core::Measures = typed
        .iter()
        .filter(|(m, _)| r3d_avatar::model::FOLLOWED.iter().any(|(f, _)| f == *m))
        .map(|(m, v)| (*m, *v))
        .collect();
    let key = (*body, followed);
    let cell: Cell = {
        let mut fits = FITS.lock().map_err(|_| CommandError::Internal {
            detail: "fit cache lock poisoned".into(),
        })?;
        match fits.iter().find(|(k, _)| *k == key) {
            Some((_, cell)) => cell.clone(),
            None => {
                if fits.len() >= KEPT {
                    fits.remove(0);
                }
                let cell = Cell::default();
                fits.push((key.clone(), cell.clone()));
                cell
            }
        }
    };
    Ok(cell
        .get_or_init(|| r3d_avatar::fit::fit_typed(model, &key.0, &key.1))
        .clone())
}

/// The avatar and what the fitted body actually measures.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarGet {}

impl Described for AvatarGet {
    const SUMMARY: &'static str = "The avatar's requested measurements, its preset, the measurements the fitted 3D body actually reaches (they can differ slightly for extreme sizes), and the heights above the floor (cm) of its waist (narrowest girth), hips (largest girth) and crotch, to draft and place skirts and trousers (panel.place top_cm). `measures`: every other measurement (neck, shoulder_width, arm_length…, as formulas name them) with its group (torso, arms, legs), its value in cm, whether it is `estimated` from the four (else typed with avatar.set_measures), whether the 3D body `follows` it, and what the built-in body `reached_cm` (null on an imported body). Null when there is no avatar.";
}

impl QuerySpec for AvatarGet {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let Some(avatar) = p.avatar() else {
            return Ok(Value::Null);
        };
        let measures = |reached: Option<&r3d_avatar::tape::Taped>| {
            r3d_core::Measure::ALL
                .iter()
                .map(|m| {
                    let v = avatar.measure(*m);
                    serde_json::json!({
                        "name": m.name(),
                        "group": m.group(),
                        "cm": v.cm,
                        "estimated": v.estimated,
                        "follows": r3d_avatar::model::FOLLOWED.iter().any(|(f, _)| f == m),
                        "reached_cm": reached.map(|t| (f64::from(t.get(*m)) * 10.0).round() / 10.0),
                    })
                })
                .collect::<Vec<_>>()
        };
        if let Some(mesh) = &avatar.mesh {
            // Taken on the imported body at its points.
            let cm = |k: r3d_core::BodyPoint| {
                mesh.points
                    .get(&k)
                    .map(|v| (v[1] / 10.0 * 10.0).round() / 10.0)
            };
            return Ok(serde_json::json!({
                "body": avatar.body,
                "preset": avatar.preset,
                "achieved": avatar.body,
                "heights_cm": {
                    "waist": cm(r3d_core::BodyPoint::Waist),
                    "hips": cm(r3d_core::BodyPoint::Hips),
                    "crotch": cm(r3d_core::BodyPoint::Crotch),
                },
                "imported": mesh,
                "measures": measures(None),
            }));
        }
        let model = body_model()?;
        let fit = fitted(&avatar.body, &avatar.measures)?;
        let a = fit.achieved;
        let shaped = r3d_avatar::fit::fitted_body(model, &fit);
        let h = r3d_avatar::measure::heights(&shaped.positions, model.triangles());
        let taped = r3d_avatar::tape::tape(model, &shaped);
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
            "heights_cm": {
                "waist": r1(a.waist_height_cm),
                "hips": r1(h.hips * 100.0),
                "crotch": h.crotch.map(|c| r1(c * 100.0)),
            },
            "measures": measures(taped.as_ref()),
        }))
    }
}

/// The check of the imported body (spec 27).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarCheck {}

impl Described for AvatarCheck {
    const SUMMARY: &'static str = "Check the imported body before draping: `ready` (no error), triangle and vertex counts, height, units/up axis/turn used, whether the file has a skeleton, `items` (each a stable `code`, a `severity` error/warning/info and `args`: file.unavailable, mesh.open, mesh.non_manifold, mesh.pieces, mesh.heavy, mesh.coarse, mesh.reoriented, mesh.inside_out, mesh.degenerate, scale.height, orientation.up, orientation.facing, points.outside, points.side, measurements.clamped, motion.unavailable) and the arrangement points with their distance to the surface (mm, negative inside). Fails when the avatar has no imported body.";
}

impl QuerySpec for AvatarCheck {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let mesh = p
            .avatar()
            .and_then(|a| a.mesh.as_ref())
            .ok_or(CommandError::NoImportedBody)?;
        to_value(&crate::bodies::check(mesh))
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

/// Every seam with the lengths of its two sides.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeamList {}

impl Described for SeamList {
    const SUMMARY: &'static str = "Every seam: its two sides (an `edge`, or `edges` / a `line` with `from_mm` and `to_mm` for part of a path; with their panels, lengths in mm and direction), the length mismatch in mm beyond its ease, its ease, whether it is gathered and its gathering ratio (longer side / shorter side).";
}

impl QuerySpec for SeamList {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let side = |s: &r3d_core::SeamSide| -> Result<Value, CommandError> {
            let panel = p.side_panel(s).ok_or_else(|| CommandError::Internal {
                detail: "a seam side names nothing".into(),
            })?;
            let mut out = serde_json::json!({
                "panel": panel,
                "panel_name": p.panel(panel).map(|panel| panel.name.clone()),
                "reversed": s.reversed,
                "length_mm": r3(p.side_length(s).unwrap_or_default()),
            });
            // A whole edge is named as before; other paths as stored.
            match s.whole_edge() {
                Some(e) => out["edge"] = serde_json::json!(e),
                None => {
                    if !s.edges.is_empty() {
                        out["edges"] = serde_json::json!(s.edges);
                    }
                    if let Some(l) = s.line {
                        out["line"] = serde_json::json!(l);
                    }
                    if let Some(f) = s.from_mm {
                        out["from_mm"] = serde_json::json!(f);
                    }
                    if let Some(t) = s.to_mm {
                        out["to_mm"] = serde_json::json!(t);
                    }
                }
            }
            Ok(out)
        };
        let seams = p
            .seams()
            .iter()
            .map(|(id, s)| {
                let (a, b) = (side(&s.a)?, side(&s.b)?);
                let (la, lb) = (
                    a["length_mm"].as_f64().unwrap_or_default(),
                    b["length_mm"].as_f64().unwrap_or_default(),
                );
                let ratio = if la.min(lb) > 0.0 {
                    la.max(lb) / la.min(lb)
                } else {
                    1.0
                };
                Ok(serde_json::json!({
                    "id": id, "a": a, "b": b,
                    "mismatch_mm": r3((la - lb - s.ease_mm).abs()),
                    "ease_mm": s.ease_mm,
                    "gathered": s.gathered,
                    "gathering_ratio": r3(ratio),
                }))
            })
            .collect::<Result<Vec<_>, CommandError>>()?;
        Ok(Value::Array(seams))
    }
}

/// Every pin.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinList {}

impl Described for PinList {
    const SUMMARY: &'static str = "Every pin: its panel, the pinned point on the pattern (mm) and where it is held in 3D (m).";
}

impl QuerySpec for PinList {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        Ok(Value::Array(
            p.pins()
                .iter()
                .map(|(id, pin)| {
                    let name = p.panel(pin.panel).map(|panel| panel.name.clone());
                    serde_json::json!({
                        "id": id,
                        "panel": pin.panel,
                        "panel_name": name,
                        "at": [r3(pin.at.x), r3(pin.at.y)],
                        "position": [r3(pin.position.x), r3(pin.position.y), r3(pin.position.z)],
                    })
                })
                .collect(),
        ))
    }
}

/// Every button and zipper.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrimList {}

impl Described for TrimList {
    const SUMMARY: &'static str = "Every trim: buttons (the piece and point they are sewn at, diameter, the buttonhole's piece, the point where the button sits and the slit's ends, buttoned or not) and zippers (their sides, length, how much is open from the top, the share closed; mm).";
}

impl QuerySpec for TrimList {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let name = |id: PanelId| p.panel(id).map(|panel| panel.name.clone());
        let buttons: Vec<Value> = p
            .buttons()
            .iter()
            .map(|(id, b)| {
                let [s0, s1] = b.slit();
                serde_json::json!({
                    "id": id,
                    "panel": b.panel,
                    "panel_name": name(b.panel),
                    "at": r3v(b.at),
                    "diameter_mm": r3(b.diameter_mm),
                    "hole_panel": b.hole.panel,
                    "hole_panel_name": name(b.hole.panel),
                    "hole_at": r3v(b.hole.at),
                    "hole_angle_deg": r3(b.hole.angle_deg),
                    "slit": [r3v(s0), r3v(s1)],
                    "closed": b.closed,
                })
            })
            .collect();
        let zippers: Vec<Value> = p
            .zippers()
            .iter()
            .map(|(id, z)| {
                serde_json::json!({
                    "id": id,
                    "a": z.a,
                    "b": z.b,
                    "panel_a": p.side_panel(&z.a),
                    "panel_b": p.side_panel(&z.b),
                    "length_mm": p.zipper_length(z).map(r3),
                    "open_mm": r3(z.open_mm),
                    "closed": r3(p.zipper_closed(z)),
                })
            })
            .collect();
        Ok(serde_json::json!({ "buttons": buttons, "zippers": zippers }))
    }
}

/// Every construction detail of spec 31.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DetailList {}

impl Described for DetailList {
    const SUMMARY: &'static str = "Every construction detail: elastics (their side, piece, ratio, width, the side's length and the relaxed length in each size of the range, or in the drawn size) topstitches (their side, piece, rows, distance, gap, stitch length, thread and its colour, the length of each row on the piece) interfacings (their piece, area, fabric, weight, and the area they cover in mm²) and quiltings (their piece and line, pattern, angle, spacing, padding, stitch, thread colour, the area quilted in mm² and the stitched lines' length); mm.";
}

impl QuerySpec for DetailList {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let lengths = crate::commands::elastic_lengths(p);
        let elastics: Vec<Value> = p
            .elastics()
            .map(|(id, e)| {
                let panel = p.side_panel(&e.side);
                let sizes: Vec<Value> = lengths
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .map(|(size, len)| serde_json::json!({ "size": size, "relaxed_mm": r3(*len) }))
                    .collect();
                serde_json::json!({
                    "id": id,
                    "side": e.side,
                    "panel": panel,
                    "panel_name": panel.and_then(|id| p.panel(id)).map(|panel| panel.name.clone()),
                    "ratio": r3(e.ratio),
                    "width_mm": r3(e.width_mm),
                    "side_mm": p.side_length(&e.side).map(r3),
                    "relaxed_mm": p.elastic_length(e).map(r3),
                    "sizes": sizes,
                })
            })
            .collect();
        let topstitches: Vec<Value> = p
            .topstitches()
            .map(|(id, t)| {
                let panel = p.side_panel(&t.side);
                let rows: Vec<f64> = p
                    .topstitch_rows(t)
                    .iter()
                    .map(|r| r3(r3d_core::seam::polyline_length(r)))
                    .collect();
                serde_json::json!({
                    "id": id,
                    "side": t.side,
                    "panel": panel,
                    "panel_name": panel.and_then(|id| p.panel(id)).map(|panel| panel.name.clone()),
                    "rows": t.rows,
                    "offset_mm": r3(t.offset_mm),
                    "gap_mm": r3(t.gap_mm),
                    "stitch_mm": r3(t.stitch_mm),
                    "color": format!("#{:06X}", p.thread_rgb(t)),
                    "own_color": t.color.is_some(),
                    "thread": t.thread,
                    "rows_mm": rows,
                })
            })
            .collect();
        let interfacings: Vec<Value> = p
            .interfacings()
            .map(|(id, i)| {
                let area = p
                    .interfacing_outline(i)
                    .map(|o| r3(r3d_geom::polygon::signed_area(&o).abs()));
                serde_json::json!({
                    "id": id,
                    "panel": i.panel,
                    "panel_name": p.panel(i.panel).map(|panel| panel.name.clone()),
                    "area": i.area,
                    "fabric": i.fabric,
                    "weight": i.weight,
                    "area_mm2": area,
                })
            })
            .collect();
        let quiltings: Vec<Value> = p
            .quiltings()
            .map(|(id, q)| {
                let area = p
                    .quilting_outline(q)
                    .map(|o| r3(r3d_geom::polygon::signed_area(&o).abs()));
                let stitched: f64 = p
                    .quilting_lines(q)
                    .iter()
                    .map(|l| r3d_core::seam::polyline_length(l))
                    .sum();
                serde_json::json!({
                    "id": id,
                    "panel": q.panel,
                    "panel_name": p.panel(q.panel).map(|panel| panel.name.clone()),
                    "line": q.line,
                    "pattern": q.pattern,
                    "angle_deg": r3(q.angle_deg),
                    "spacing_mm": r3(q.spacing_mm),
                    "loft_mm": r3(q.loft_mm),
                    "padding_gsm": r3(q.padding_gsm),
                    "stitch_mm": r3(q.stitch_mm),
                    "color": format!("#{:06X}", p.quilting_rgb(q)),
                    "area_mm2": area,
                    "stitched_mm": r3(stitched),
                })
            })
            .collect();
        Ok(serde_json::json!({
            "elastics": elastics,
            "topstitches": topstitches,
            "interfacings": interfacings,
            "quiltings": quiltings,
        }))
    }
}

/// The starter garments and their default sizes.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemplateList {}

impl Described for TemplateList {
    const SUMMARY: &'static str = "The starter garments template.apply can draft, each with the sizes it reads (cm), filled in from the avatar's measurements (or a standard size without one).";
}

impl QuerySpec for TemplateList {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        to_value(&crate::templates::list(p)?)
    }
}

/// The project's size range.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradingGet {}

impl Described for GradingGet {
    const SUMMARY: &'static str = "The project's size range: each size with how its body differs from the base size's (cm) and its body (the avatar's measurements changed by those differences; null without an avatar), the base size the pattern is drawn in, the size shown and draped in 3D, and how many points are graded. Null when the project has no range.";
}

impl QuerySpec for GradingGet {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let Some(g) = p.grading() else {
            return Ok(Value::Null);
        };
        let sizes: Vec<Value> = g
            .sizes
            .iter()
            .enumerate()
            .map(|(i, s)| {
                serde_json::json!({
                    "name": s.name,
                    "height_cm": s.height_cm,
                    "bust_cm": s.bust_cm,
                    "waist_cm": s.waist_cm,
                    "hips_cm": s.hips_cm,
                    "body": p.size_body(i),
                })
            })
            .collect();
        let name = |i: usize| g.sizes.get(i).map(|s| s.name.clone());
        Ok(serde_json::json!({
            "sizes": sizes,
            "base": name(g.base),
            "shown": name(g.shown),
            "graded_points": p.points().values().filter(|pt| !pt.grade.is_empty()).count(),
        }))
    }
}

/// Seams that no longer match in a size, and pieces that cross themselves.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradingCheck {
    /// Report seams whose two sides differ by more than this (mm, default
    /// 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_mm: Option<f64>,
}

impl Described for GradingCheck {
    const SUMMARY: &'static str = "Check the grading in every size of the range: the seams (not gathered) whose two sides differ by more than `tolerance_mm` (default 1), with both lengths, and the pieces whose outline crosses itself. `ok` is true when no size has either.";
}

impl QuerySpec for GradingCheck {
    fn run(&self, p: &Project) -> Result<Value, CommandError> {
        let tolerance = self.tolerance_mm.unwrap_or(1.0);
        crate::error::finite("tolerance_mm", &[tolerance])?;
        crate::error::non_negative("tolerance_mm", tolerance)?;
        let g = crate::commands::grading(p)?;
        let mut ok = true;
        let sizes: Vec<Value> = g
            .sizes
            .iter()
            .enumerate()
            .map(|(i, size)| {
                let q = p.at_size(i);
                let seams: Vec<Value> = q
                    .seams()
                    .iter()
                    .filter(|(_, s)| !s.gathered)
                    .filter_map(|(id, s)| {
                        let (a, b) = (q.side_length(&s.a)?, q.side_length(&s.b)?);
                        let mismatch = (a - b - s.ease_mm).abs();
                        (mismatch > tolerance).then(|| {
                            serde_json::json!({
                                "seam": id, "a_mm": r3(a), "b_mm": r3(b),
                                "mismatch_mm": r3(mismatch),
                            })
                        })
                    })
                    .collect();
                let crossed: Vec<PanelId> = q
                    .panels()
                    .keys()
                    .filter(|id| q.panel_is_simple(**id) == Some(false))
                    .copied()
                    .collect();
                ok &= seams.is_empty() && crossed.is_empty();
                serde_json::json!({
                    "name": size.name, "seams": seams, "crossed_panels": crossed,
                })
            })
            .collect();
        Ok(serde_json::json!({ "ok": ok, "sizes": sizes }))
    }
}

/// The built-in size charts.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradingCharts {}

impl Described for GradingCharts {
    const SUMMARY: &'static str = "The built-in size charts for grading.use_chart: each chart's sizes with their bodies (cm).";
}

impl QuerySpec for GradingCharts {
    fn run(&self, _: &Project) -> Result<Value, CommandError> {
        use crate::commands::SizeChart;
        let charts: Vec<Value> = [SizeChart::Women, SizeChart::Men]
            .into_iter()
            .map(|c| {
                let sizes: Vec<Value> = c
                    .sizes()
                    .into_iter()
                    .map(|(name, body)| serde_json::json!({"name": name, "body": body}))
                    .collect();
                serde_json::json!({ "chart": c, "sizes": sizes })
            })
            .collect();
        Ok(Value::Array(charts))
    }
}
