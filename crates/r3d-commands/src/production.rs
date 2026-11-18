//! Production (M30, `docs/specs/23-production.md`): indicators per piece
//! and per garment, and the marker, the pieces of a garment in one or
//! several sizes nested on the fabric with its yardage and cost.

use std::borrow::Cow;
use std::f64::consts::PI;

use r3d_core::{GarmentId, PanelId, Project};
use r3d_geom::nest::{self, NestError, Orientation};
use r3d_geom::{DVec2, polygon};
use r3d_io::marker::{Sheet, SheetPiece, marker_drawing};
use r3d_io::pdf::{Paper, to_pdf};
use r3d_io::svg::to_svg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::{ActionSpec, PaperSpec, export_stem};
use crate::commands::{Vec2, size_index};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{Described, QuerySpec};
use crate::workspace::{Workspace, WorkspaceError};

/// Fabric width of a marker when neither it nor the garment gives one
/// (mm): the most common width of woven fabric.
pub const DEFAULT_WIDTH_MM: f64 = 1500.0;
/// Raster cell of the nesting (mm).
pub const DEFAULT_STEP_MM: f64 = 5.0;

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// The project in size `size` of its range, or as drawn.
fn in_size<'a>(project: &'a Project, size: Option<&str>) -> Result<Cow<'a, Project>, CommandError> {
    Ok(match size {
        Some(name) => Cow::Owned(project.at_size(size_index(project, name.trim())?)),
        None => Cow::Borrowed(project),
    })
}

/// A piece as it is cut: its sewing line (counter-clockwise), its cutting
/// line (the sewing line plus the seam allowance) and its holes.
pub(crate) struct Cut {
    pub(crate) sew: Vec<DVec2>,
    pub(crate) cut: Vec<DVec2>,
    pub(crate) holes: Vec<Vec<DVec2>>,
    allowance: bool,
}

impl Cut {
    pub(crate) fn of(project: &Project, id: PanelId) -> Option<Cut> {
        let panel = project.panel(id)?;
        let mut sew = project.panel_outline(id)?;
        if sew.len() < 3 {
            return None;
        }
        if polygon::signed_area(&sew) < 0.0 {
            sew.reverse();
        }
        let sa = panel.seam_allowance_mm;
        let cut = if sa > 0.0 {
            polygon::offset_outward(&sew, sa)
        } else {
            sew.clone()
        };
        Some(Cut {
            sew,
            cut,
            holes: project.hole_outlines(id),
            allowance: sa > 0.0,
        })
    }

    fn holes_area(&self) -> f64 {
        self.holes
            .iter()
            .map(|h| polygon::signed_area(h).abs())
            .sum()
    }

    /// Cloth inside the sewing line (mm²).
    pub(crate) fn area(&self) -> f64 {
        polygon::signed_area(&self.sew).abs() - self.holes_area()
    }

    /// Cloth cut, seam allowance included (mm²).
    pub(crate) fn cut_area(&self) -> f64 {
        polygon::signed_area(&self.cut).abs() - self.holes_area()
    }
}

/// Mass of `area_mm2` of fabric weighing `gsm` (grams).
pub(crate) fn mass_g(area_mm2: f64, gsm: f64) -> f64 {
    area_mm2 * 1e-6 * gsm
}

/// Area, cut area, mass of each piece and totals per garment.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductionIndicators {
    /// A size of the range, by name; omitted: the pattern as drawn (the
    /// base size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

impl Described for ProductionIndicators {
    const SUMMARY: &'static str = "Production indicators, per garment (next to the body first): the weight in use (g/m²) and the fabric details as set (weight_gsm, width_mm, price_per_m), each piece's area (sewing line, holes left out, mm²), cut area (with its seam allowance), perimeter (mm) and mass (cut area × weight, g), and the garment's totals. In one size of the range with `size`.";
}

#[derive(Serialize)]
struct PieceFigures {
    panel: PanelId,
    name: String,
    area_mm2: f64,
    cut_area_mm2: f64,
    perimeter_mm: f64,
    mass_g: f64,
}

impl QuerySpec for ProductionIndicators {
    fn run(&self, project: &Project) -> Result<Value, CommandError> {
        let p = in_size(project, self.size.as_deref())?;
        let garments: Vec<Value> = p
            .garments_by_layer()
            .into_iter()
            .filter_map(|gid| Some((gid, p.garments().get(&gid)?)))
            .map(|(gid, g)| {
                let gsm = g.weight_gsm();
                let pieces: Vec<PieceFigures> = p
                    .panels()
                    .iter()
                    .filter(|(_, panel)| panel.garment == gid)
                    .filter_map(|(id, panel)| {
                        let cut = Cut::of(&p, *id)?;
                        Some(PieceFigures {
                            panel: *id,
                            name: panel.name.clone(),
                            area_mm2: r3(cut.area()),
                            cut_area_mm2: r3(cut.cut_area()),
                            perimeter_mm: r3(p.panel_perimeter(*id).unwrap_or_default()),
                            mass_g: r2(mass_g(cut.cut_area(), gsm)),
                        })
                    })
                    .collect();
                let area: f64 = pieces.iter().map(|f| f.area_mm2).sum();
                let cut_area: f64 = pieces.iter().map(|f| f.cut_area_mm2).sum();
                json!({
                    "id": gid,
                    "name": g.name,
                    "fabric": g.fabric,
                    "weight_gsm": gsm,
                    "fabric_details": g.fabric_details,
                    "pieces": pieces,
                    "totals": {
                        "pieces": pieces.len(),
                        "area_mm2": r3(area),
                        "cut_area_mm2": r3(cut_area),
                        "mass_g": r2(mass_g(cut_area, gsm)),
                    },
                })
            })
            .collect();
        Ok(json!({ "size": self.size, "garments": garments }))
    }
}

/// How many times a piece is cut in a marker, and how.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PieceQuantity {
    pub panel: PanelId,
    /// Copies per size (0–50; 0 leaves the piece out). Default 1.
    #[serde(default = "one")]
    pub count: u32,
    /// Every second copy is turned over (a left and a right).
    #[serde(default)]
    pub pairs: bool,
    /// The piece has no grain to follow (felt, interfacing, a bias
    /// piece): it may lie at every 45°.
    #[serde(default)]
    pub free_grain: bool,
}

fn one() -> u32 {
    1
}

/// What to lay out, and on what.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkerSpec {
    /// The garment whose pieces are laid out; omitted: the project's only
    /// garment (required when there are several).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garment: Option<GarmentId>,
    /// Sizes of the range to cut, by name (each piece once per size);
    /// omitted: the pattern as drawn (the base size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<String>>,
    /// Usable fabric width (mm, 300–3000); omitted: the garment's fabric
    /// width, else 1500.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
    /// Pieces cut another number of times than once, turned over in
    /// pairs, or with a free grain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quantities: Vec<PieceQuantity>,
    /// Raster cell of the nesting (mm, 2–20, default 5): smaller is
    /// tighter and slower.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_mm: Option<f64>,
}

/// One piece where it lies on the marker (marker mm: x along the length,
/// y across the width from the bottom selvedge).
#[derive(Clone, Debug, PartialEq)]
pub struct MarkedPiece {
    pub panel: PanelId,
    pub name: String,
    /// The size's name; `None` without a range.
    pub size: Option<String>,
    /// Which of the marker's sizes (0 = the first).
    pub size_rank: usize,
    /// Which copy of the piece in its size (0 = the first).
    pub copy: u32,
    /// A board point `p` lies at `rotate(p, rotation_deg)`, then turned
    /// over (y → −y) when `mirrored`, then moved by `offset`.
    pub rotation_deg: f64,
    pub mirrored: bool,
    pub offset: DVec2,
    pub cut: Vec<DVec2>,
    pub sew: Option<Vec<DVec2>>,
    pub holes: Vec<Vec<DVec2>>,
    /// The grainline's direction on the fabric (degrees).
    pub grain_angle_deg: f64,
    /// Cloth of the piece as cut (mm²).
    pub area_mm2: f64,
}

/// A computed marker.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub garment: GarmentId,
    pub garment_name: String,
    /// The sizes laid out (empty without a range).
    pub sizes: Vec<String>,
    pub width_mm: f64,
    pub length_mm: f64,
    /// Cloth of the pieces over the fabric used (0–1).
    pub efficiency: f64,
    /// Fabric to buy (m, the length rounded up to the centimetre).
    pub yardage_m: f64,
    pub price_per_m: Option<f64>,
    pub cost: Option<f64>,
    pub pieces: Vec<MarkedPiece>,
}

impl Marker {
    /// The marker as JSON; each piece's outlines (marker mm) with
    /// `outlines`.
    pub fn to_json(&self, outlines: bool) -> Value {
        let pts = |v: &[DVec2]| -> Vec<Vec2> { v.iter().map(|p| [r3(p.x), r3(p.y)]).collect() };
        let pieces: Vec<Value> = self
            .pieces
            .iter()
            .map(|m| {
                let mut v = json!({
                    "panel": m.panel,
                    "name": m.name,
                    "size": m.size,
                    "copy": m.copy,
                    "rotation_deg": r3(m.rotation_deg),
                    "mirrored": m.mirrored,
                    "offset": [r3(m.offset.x), r3(m.offset.y)],
                });
                if outlines {
                    v["size_rank"] = json!(m.size_rank);
                    v["cut"] = json!(pts(&m.cut));
                    v["sew"] = json!(m.sew.as_deref().map(pts));
                    v["holes"] = json!(m.holes.iter().map(|h| pts(h)).collect::<Vec<_>>());
                    v["grain_angle_deg"] = json!(r3(m.grain_angle_deg));
                }
                v
            })
            .collect();
        json!({
            "garment": self.garment,
            "sizes": self.sizes,
            "width_mm": r3(self.width_mm),
            "length_mm": r3(self.length_mm),
            "efficiency_pct": (self.efficiency * 1000.0).round() / 10.0,
            "yardage_m": self.yardage_m,
            "price_per_m": self.price_per_m,
            "cost": self.cost,
            "pieces": pieces,
        })
    }

    /// The marker's drawing, captioned with figures (no words: the file
    /// is read in any language).
    pub fn sheet(&self, project_name: &str) -> Sheet {
        let mut caption = vec![format!(
            "{} - {}{}",
            project_name,
            self.garment_name,
            if self.sizes.is_empty() {
                String::new()
            } else {
                format!(" - {}", self.sizes.join(" "))
            }
        )];
        caption.push(format!(
            "{:.0} x {:.0} mm   {:.1} %",
            self.width_mm,
            self.length_mm,
            self.efficiency * 100.0
        ));
        if let (Some(price), Some(cost)) = (self.price_per_m, self.cost) {
            caption.push(format!(
                "{:.2} m x {:.2} = {:.2}",
                self.yardage_m, price, cost
            ));
        } else {
            caption.push(format!("{:.2} m", self.yardage_m));
        }
        Sheet {
            width_mm: self.width_mm,
            length_mm: self.length_mm,
            pieces: self
                .pieces
                .iter()
                .map(|m| SheetPiece {
                    cut: m.cut.clone(),
                    sew: m.sew.clone(),
                    holes: m.holes.clone(),
                    grain_angle_deg: m.grain_angle_deg,
                    label: match &m.size {
                        Some(s) => format!("{} {s}", m.name),
                        None => m.name.clone(),
                    },
                    size_rank: m.size_rank,
                })
                .collect(),
            caption,
        }
    }
}

/// The garment a marker is for.
fn marker_garment(project: &Project, id: Option<GarmentId>) -> Result<GarmentId, CommandError> {
    match id {
        Some(id) if project.garments().contains_key(&id) => Ok(id),
        Some(id) => Err(CommandError::not_found(id)),
        None => match project.garments().keys().collect::<Vec<_>>().as_slice() {
            [only] => Ok(**only),
            _ => Err(CommandError::invalid("garment", Invalid::Empty)),
        },
    }
}

/// Lay the garment's pieces out on the fabric (spec 23).
pub fn compute(project: &Project, spec: &MarkerSpec) -> Result<Marker, CommandError> {
    let gid = marker_garment(project, spec.garment)?;
    let garment = project
        .garments()
        .get(&gid)
        .ok_or_else(|| CommandError::not_found(gid))?;
    let width = spec
        .width_mm
        .or(garment.fabric_details.width_mm)
        .unwrap_or(DEFAULT_WIDTH_MM);
    finite("width_mm", &[width])?;
    if !(300.0..=3000.0).contains(&width) {
        return Err(CommandError::invalid(
            "width_mm",
            Invalid::Range(300.0, 3000.0),
        ));
    }
    let step = spec.step_mm.unwrap_or(DEFAULT_STEP_MM);
    finite("step_mm", &[step])?;
    if !(2.0..=20.0).contains(&step) {
        return Err(CommandError::invalid("step_mm", Invalid::Range(2.0, 20.0)));
    }
    for q in &spec.quantities {
        match project.panel(q.panel) {
            Some(p) if p.garment == gid => {}
            Some(_) => return Err(CommandError::invalid("quantities", Invalid::OtherPanel)),
            None => return Err(CommandError::not_found(q.panel)),
        }
        if q.count > 50 {
            return Err(CommandError::invalid("count", Invalid::Range(0.0, 50.0)));
        }
    }
    // The sizes, by index into the range, in the range's order.
    let sizes: Vec<Option<(usize, String)>> = match &spec.sizes {
        None => vec![None],
        Some(names) => {
            if names.is_empty() {
                return Err(CommandError::invalid("sizes", Invalid::Empty));
            }
            let mut idx = names
                .iter()
                .map(|n| size_index(project, n.trim()))
                .collect::<Result<Vec<_>, _>>()?;
            idx.sort_unstable();
            idx.dedup();
            let g = project.grading();
            idx.into_iter()
                .map(|k| {
                    let name = g.and_then(|g| g.sizes.get(k)).map(|s| s.name.clone());
                    Some((k, name.unwrap_or_default()))
                })
                .collect()
        }
    };

    // Every copy to place, with what it is.
    struct Copy {
        panel: PanelId,
        name: String,
        size: Option<String>,
        size_rank: usize,
        copy: u32,
        grain_deg: f64,
        cut: Cut,
    }
    let mut copies: Vec<Copy> = Vec::new();
    let mut pieces: Vec<nest::Piece> = Vec::new();
    for (rank, size) in sizes.iter().enumerate() {
        let sized = match size {
            Some((k, _)) => Cow::Owned(project.at_size(*k)),
            None => Cow::Borrowed(project),
        };
        for (id, panel) in sized.panels().iter().filter(|(_, p)| p.garment == gid) {
            let q = spec.quantities.iter().find(|q| q.panel == *id);
            let count = q.map_or(1, |q| q.count);
            let (pairs, free) = q.map_or((false, false), |q| (q.pairs, q.free_grain));
            for copy in 0..count {
                let Some(cut) = Cut::of(&sized, *id) else {
                    continue;
                };
                let mirrored = pairs && copy % 2 == 1;
                let base = -panel.grain_angle_deg.to_radians();
                let turns: Vec<f64> = if free {
                    (0..8).map(|k| base + k as f64 * PI / 4.0).collect()
                } else {
                    vec![base, base + PI]
                };
                pieces.push(nest::Piece {
                    outline: cut.cut.clone(),
                    orientations: turns
                        .into_iter()
                        .map(|angle| Orientation { angle, mirrored })
                        .collect(),
                });
                copies.push(Copy {
                    panel: *id,
                    name: panel.name.clone(),
                    size: size.as_ref().map(|(_, n)| n.clone()),
                    size_rank: rank,
                    copy,
                    grain_deg: panel.grain_angle_deg,
                    cut,
                });
            }
        }
    }
    if pieces.is_empty() {
        return Err(CommandError::invalid("garment", Invalid::Empty));
    }
    let layout = nest::nest(&pieces, width, step).map_err(|e| match e {
        NestError::TooWide(i) => match copies.get(i) {
            Some(c) => CommandError::MarkerTooNarrow {
                panel: c.panel.into(),
                width_mm: width,
            },
            None => CommandError::Internal {
                detail: "marker piece out of range".into(),
            },
        },
        NestError::BadGrid => CommandError::invalid("step_mm", Invalid::Range(2.0, 20.0)),
    })?;

    let mut used = 0.0;
    let mut marked = Vec::with_capacity(copies.len());
    for ((c, piece), at) in copies.into_iter().zip(&pieces).zip(&layout.placed) {
        let o = piece
            .orientations
            .get(at.orientation)
            .copied()
            .ok_or_else(|| CommandError::Internal {
                detail: "marker orientation".into(),
            })?;
        let place =
            |v: &[DVec2]| -> Vec<DVec2> { v.iter().map(|p| o.apply(*p) + at.offset).collect() };
        let area = c.cut.cut_area();
        used += area;
        let turned = (c.grain_deg + o.angle.to_degrees()).rem_euclid(360.0);
        marked.push(MarkedPiece {
            panel: c.panel,
            name: c.name,
            size: c.size,
            size_rank: c.size_rank,
            copy: c.copy,
            rotation_deg: o.angle.to_degrees().rem_euclid(360.0),
            mirrored: o.mirrored,
            offset: at.offset,
            sew: c.cut.allowance.then(|| place(&c.cut.sew)),
            holes: c.cut.holes.iter().map(|h| place(h)).collect(),
            cut: place(&c.cut.cut),
            grain_angle_deg: if o.mirrored {
                (-turned).rem_euclid(360.0)
            } else {
                turned
            },
            area_mm2: area,
        });
    }
    let length = layout.length;
    let yardage_m = (length / 10.0).ceil() / 100.0;
    let price = garment.fabric_details.price_per_m;
    Ok(Marker {
        garment: gid,
        garment_name: garment.name.clone(),
        sizes: sizes.into_iter().flatten().map(|(_, n)| n).collect(),
        width_mm: width,
        length_mm: length,
        efficiency: if length > 0.0 {
            used / (length * width)
        } else {
            0.0
        },
        yardage_m,
        price_per_m: price,
        cost: price.map(|p| r2(p * yardage_m)),
        pieces: marked,
    })
}

/// Lay out the pieces of a garment on the fabric (a marker).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkerCompute {
    /// What to lay out, and on what.
    #[serde(default)]
    pub marker: MarkerSpec,
    /// Include each piece's outlines (cutting and sewing lines, holes, in
    /// marker mm) in the result.
    #[serde(default)]
    pub outlines: bool,
}

impl Described for MarkerCompute {
    const SUMMARY: &'static str = "Lay the pieces of a garment, in one or several sizes, out on a strip of fabric (a marker; raster bottom-left-fill, deterministic): each piece by its cutting line, its grainline along the fabric's length (0° or 180°; every 45° with free_grain), once per size unless `quantities` says otherwise. Returns the length (mm), efficiency (% of the fabric used), yardage (m), cost (from the garment's price per metre) and where each piece lies. Fails with marker_too_narrow when a piece is wider than the fabric.";
}

impl QuerySpec for MarkerCompute {
    fn run(&self, project: &Project) -> Result<Value, CommandError> {
        Ok(compute(project, &self.marker)?.to_json(self.outlines))
    }
}

/// File format of an exported marker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MarkerFormat {
    #[default]
    Svg,
    Pdf,
}

/// The marker as a file: its bytes, and the marker.
pub fn marker_file(
    project: &Project,
    spec: &MarkerSpec,
    format: MarkerFormat,
    paper: Paper,
) -> Result<(Vec<u8>, Marker), CommandError> {
    let marker = compute(project, spec)?;
    let drawing = marker_drawing(&marker.sheet(project.name()));
    let bytes = match format {
        MarkerFormat::Svg => to_svg(&drawing).into_bytes(),
        MarkerFormat::Pdf => to_pdf(&drawing, paper),
    };
    Ok((bytes, marker))
}

/// Export a marker as SVG or PDF at 1:1.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportMarker {
    /// What to lay out, and on what (as in marker.compute).
    #[serde(default)]
    pub marker: MarkerSpec,
    /// `svg` (default) or `pdf`.
    #[serde(default)]
    pub format: MarkerFormat,
    /// PDF pages: `single` (default, one page as long as the marker, for
    /// a plotter), `a4` or `letter` tiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper: Option<PaperSpec>,
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>-marker.<ext>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Described for ExportMarker {
    const SUMMARY: &'static str = "Lay out a marker (as marker.compute) and write it at 1:1 as SVG or PDF: the fabric's edges, each piece's cutting and sewing lines, grainline, name and size, the width, length and efficiency, and the 100 mm test square. Returns the written path and the marker's length, efficiency, yardage and cost.";
}

impl ActionSpec for ExportMarker {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        let paper = Paper::from(self.paper.unwrap_or(PaperSpec::Single));
        let (bytes, marker) =
            marker_file(ws.session().project(), &self.marker, self.format, paper)?;
        let ext = match self.format {
            MarkerFormat::Svg => "svg",
            MarkerFormat::Pdf => "pdf",
        };
        let path = self
            .path
            .clone()
            .unwrap_or_else(|| format!("exports/{}-marker.{ext}", export_stem(ws)));
        let written = ws.write(&path, &bytes)?;
        let mut out = marker.to_json(false);
        if let Some(o) = out.as_object_mut() {
            o.remove("pieces");
            o.insert("pieces".into(), json!(marker.pieces.len()));
            o.insert("path".into(), json!(written));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mass_of_a_square_metre() {
        assert!((mass_g(1e6, 200.0) - 200.0).abs() < 1e-9);
    }
}
