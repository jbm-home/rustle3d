//! The tech pack (M30, `docs/specs/23-production.md`): the project's
//! production file as a PDF: views of the garment, colourways, the bill of
//! materials with the fabric each garment takes, the measurements (size
//! chart, pieces, pieces by size), the pattern and the sewing notes.
//!
//! The words in the file are labels with stable keys and English
//! defaults; a front-end passes its own (the app, in the user's
//! language). Pictures of the garment are rendered by the front-ends
//! (this crate has no GPU) and passed in, or read from PNG files in the
//! project folder.

use std::collections::BTreeMap;

use r3d_core::{GarmentId, PanelId, Project};
use r3d_geom::DVec2;
pub use r3d_io::techpack::Image;
use r3d_io::techpack::{
    Block, Column, PageSize, Section, Swatch, TechPack, to_pdf as techpack_pdf,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::{ActionSpec, export_stem};
use crate::error::{CommandError, Invalid};
use crate::production::{self, Cut, MarkerSpec, mass_g};
use crate::spec::Described;
use crate::workspace::{Workspace, WorkspaceError};

/// The labels and their English text. A key missing from a request's
/// `labels` takes this text.
pub const LABELS: &[(&str, &str)] = &[
    ("decimal", "."),
    ("sizes", "Sizes"),
    ("sample", "sample"),
    ("views", "Views"),
    ("front", "Front"),
    ("back", "Back"),
    ("colourways", "Colourways"),
    ("materials", "Bill of materials"),
    ("fabrics", "Fabrics"),
    ("garment", "Garment"),
    ("fabric", "Fabric"),
    ("colour", "Colour"),
    ("weight", "Weight (g/m²)"),
    ("width", "Width (cm)"),
    ("price", "Price per metre"),
    ("consumption", "Fabric per garment (m)"),
    ("cost", "Cost per garment"),
    ("consumption_by_size", "Fabric per garment by size (m)"),
    ("trims", "Trims"),
    ("no_trims", "No trims."),
    ("trim", "Trim"),
    ("trim_size", "Size"),
    ("quantity", "Quantity"),
    ("placement", "On"),
    ("button", "Button"),
    ("zipper", "Zipper"),
    ("measurements", "Measurements"),
    ("size_chart", "Size chart (body, cm)"),
    ("size", "Size"),
    ("height", "Height"),
    ("bust", "Bust"),
    ("waist", "Waist"),
    ("hips", "Hips"),
    ("pieces", "Pieces"),
    ("piece", "Piece"),
    ("length", "Length (cm)"),
    ("piece_width", "Width (cm)"),
    ("perimeter", "Sewing line (cm)"),
    ("area", "Area (cm²)"),
    ("cut_area", "Cut area (cm²)"),
    ("allowance", "Allowance (mm)"),
    ("mass", "Mass (g)"),
    ("pieces_by_size", "Pieces by size: length x width (cm)"),
    ("pattern", "Pattern"),
    ("sewing", "Sewing notes"),
    ("seams", "Seams"),
    ("side_a", "Side A"),
    ("side_b", "Side B"),
    ("length_a", "Length A (mm)"),
    ("length_b", "Length B (mm)"),
    ("ease", "Ease (mm)"),
    ("gathered", "Gathered"),
    ("yes", "yes"),
    ("no_seams", "No seams yet."),
    ("notes", "Notes"),
    ("cotton", "Cotton"),
    ("denim", "Denim"),
    ("jersey", "Jersey"),
    ("silk", "Silk"),
    ("wool", "Wool"),
];

/// Page size of a tech pack (landscape).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TechPackPaper {
    #[default]
    A4,
    Letter,
}

/// A picture to put in the tech pack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TechPackImage {
    /// A PNG file inside the project folder (a render, a photo).
    pub path: String,
    /// Written under it; omitted: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

/// Write the project's tech pack as a PDF.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportTechPack {
    /// Pictures of the garment (PNG files in the project folder), shown
    /// side by side first. The CLI and the MCP server render a front
    /// and a back view of their own before these (`views`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<TechPackImage>,
    /// The words of the file, by key (see `docs/specs/23-production.md`;
    /// `decimal` is the decimal separator). Omitted ones are in English.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    /// `a4` (default) or `letter`, landscape.
    #[serde(default)]
    pub paper: TechPackPaper,
    /// Output path relative to the project folder. Defaults to
    /// `exports/<project name>-techpack.pdf`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Described for ExportTechPack {
    const SUMMARY: &'static str = "Write the project's tech pack as a PDF (landscape A4 or Letter): pictures of the garment, colourways (each garment's colour and fabric), the bill of materials (each garment's fabric with weight, width and price, the fabric one garment takes from its marker in the sample size, and by size when graded; trims), the measurements (size chart, every piece's length along the grain, width, sewing line, areas, allowance and mass, and by size), the pattern to scale, and the sewing notes (every seam with its sides' lengths, ease, gathering; the garments' notes). Returns the path and the fabric per garment.";
}

/// Text of a label: the request's, or the English default.
fn label<'a>(labels: &'a BTreeMap<String, String>, key: &'a str) -> &'a str {
    if let Some(v) = labels.get(key) {
        return v;
    }
    LABELS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(key, |(_, v)| *v)
}

/// What one garment's marker takes in the sample size.
#[derive(Clone, Debug, PartialEq)]
struct Consumption {
    garment: GarmentId,
    yardage_m: Option<f64>,
    cost: Option<f64>,
}

/// Builds the content.
struct Writer<'a> {
    project: &'a Project,
    labels: &'a BTreeMap<String, String>,
}

impl Writer<'_> {
    fn t(&self, key: &'static str) -> String {
        label(self.labels, key).to_owned()
    }

    /// `v` with `digits` decimals and the labels' decimal separator.
    fn num(&self, v: f64, digits: usize) -> String {
        let s = format!("{v:.digits$}");
        let sep = label(self.labels, "decimal");
        if sep == "." { s } else { s.replace('.', sep) }
    }

    fn garments(&self) -> Vec<GarmentId> {
        self.project.garments_by_layer()
    }

    fn garment_name(&self, id: GarmentId) -> String {
        self.project
            .garments()
            .get(&id)
            .map(|g| g.name.clone())
            .unwrap_or_default()
    }

    fn fabric_name(&self, id: GarmentId) -> String {
        let key = self
            .project
            .garments()
            .get(&id)
            .and_then(|g| serde_json::to_value(g.fabric).ok())
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        self.labels
            .get(&key)
            .cloned()
            .or_else(|| {
                LABELS
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| (*v).to_owned())
            })
            .unwrap_or(key)
    }

    /// A piece named for the table: with its garment when there are
    /// several.
    fn piece_name(&self, project: &Project, id: PanelId) -> String {
        let name = project
            .panel(id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        if project.garments().len() > 1
            && let Some(g) = project.garment_of(id)
        {
            return format!("{} - {name}", g.name);
        }
        name
    }

    fn sizes(&self) -> Vec<String> {
        self.project
            .grading()
            .map(|g| g.sizes.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default()
    }

    fn subtitle(&self) -> String {
        let names: Vec<String> = self
            .garments()
            .into_iter()
            .map(|g| self.garment_name(g))
            .collect();
        let mut s = names.join(", ");
        if let Some(g) = self.project.grading() {
            let first = g.sizes.first().map(|s| s.name.as_str()).unwrap_or("");
            let last = g.sizes.last().map(|s| s.name.as_str()).unwrap_or("");
            let base = g.sizes.get(g.base).map(|s| s.name.as_str()).unwrap_or("");
            s.push_str(&format!(
                " - {} {first}-{last} ({} {base})",
                self.t("sizes"),
                self.t("sample")
            ));
        }
        s
    }

    fn colourways(&self) -> Section {
        let swatches = self
            .garments()
            .into_iter()
            .filter_map(|id| {
                let g = self.project.garments().get(&id)?;
                Some(Swatch {
                    rgb: g.rgb(),
                    lines: vec![
                        g.name.clone(),
                        self.fabric_name(id),
                        format!("#{:06X}", g.rgb()),
                    ],
                })
            })
            .collect();
        Section {
            title: self.t("colourways"),
            blocks: vec![Block::Swatches(swatches)],
        }
    }

    /// The fabric one garment takes: its marker in the sample size, or
    /// in size `size`.
    fn consumption(&self, id: GarmentId, size: Option<&str>) -> Consumption {
        let spec = MarkerSpec {
            garment: Some(id),
            sizes: size.map(|s| vec![s.to_owned()]),
            ..MarkerSpec::default()
        };
        match production::compute(self.project, &spec) {
            Ok(m) => Consumption {
                garment: id,
                yardage_m: Some(m.yardage_m),
                cost: m.cost,
            },
            Err(_) => Consumption {
                garment: id,
                yardage_m: None,
                cost: None,
            },
        }
    }

    fn materials(&self, consumption: &[Consumption]) -> Section {
        let dash = || "-".to_owned();
        let rows = consumption
            .iter()
            .filter_map(|c| {
                let g = self.project.garments().get(&c.garment)?;
                let d = g.fabric_details;
                Some(vec![
                    g.name.clone(),
                    self.fabric_name(c.garment),
                    format!("#{:06X}", g.rgb()),
                    self.num(g.weight_gsm(), 0),
                    d.width_mm
                        .or(Some(production::DEFAULT_WIDTH_MM))
                        .map_or_else(dash, |w| self.num(w / 10.0, 0)),
                    d.price_per_m.map_or_else(dash, |p| self.num(p, 2)),
                    c.yardage_m.map_or_else(dash, |y| self.num(y, 2)),
                    c.cost.map_or_else(dash, |v| self.num(v, 2)),
                ])
            })
            .collect();
        let mut blocks = vec![
            Block::Subheading(self.t("fabrics")),
            Block::Table {
                columns: vec![
                    Column::text(self.t("garment"), 2.0),
                    Column::text(self.t("fabric"), 1.5),
                    Column::text(self.t("colour"), 1.2),
                    Column::number(self.t("weight"), 1.3),
                    Column::number(self.t("width"), 1.1),
                    Column::number(self.t("price"), 1.3),
                    Column::number(self.t("consumption"), 1.8),
                    Column::number(self.t("cost"), 1.4),
                ],
                rows,
            },
        ];
        let sizes = self.sizes();
        if !sizes.is_empty() {
            let mut columns = vec![Column::text(self.t("garment"), 2.0)];
            columns.extend(sizes.iter().map(|s| Column::number(s.clone(), 1.0)));
            let rows = self
                .garments()
                .into_iter()
                .map(|id| {
                    let mut row = vec![self.garment_name(id)];
                    row.extend(sizes.iter().map(|s| {
                        self.consumption(id, Some(s))
                            .yardage_m
                            .map_or_else(dash, |y| self.num(y, 2))
                    }));
                    row
                })
                .collect();
            blocks.push(Block::Subheading(self.t("consumption_by_size")));
            blocks.push(Block::Table { columns, rows });
        }
        blocks.push(Block::Subheading(self.t("trims")));
        let trims = self.trims();
        if trims.is_empty() {
            blocks.push(Block::Text(self.t("no_trims")));
        } else {
            blocks.push(Block::Table {
                columns: vec![
                    Column::text(self.t("garment"), 2.0),
                    Column::text(self.t("trim"), 1.5),
                    Column::number(self.t("trim_size"), 1.2),
                    Column::number(self.t("quantity"), 1.2),
                    Column::text(self.t("placement"), 4.0),
                ],
                rows: trims,
            });
        }
        Section {
            title: self.t("materials"),
            blocks,
        }
    }

    /// The trims one garment takes: buttons by diameter (with their
    /// buttonholes), zippers by length; where they go.
    fn trims(&self) -> Vec<Vec<String>> {
        let p = self.project;
        let piece = |id: PanelId| p.panel(id).map(|x| x.name.clone()).unwrap_or_default();
        let garment = |id: PanelId| p.panel(id).map(|x| x.garment);
        // (garment, kind, size in mm) → pieces, count.
        let mut rows: std::collections::BTreeMap<(GarmentId, u8, i64), (Vec<String>, usize)> =
            Default::default();
        let mut add = |g: GarmentId, kind: u8, size: f64, pieces: Vec<String>| {
            let e = rows
                .entry((g, kind, (size * 10.0).round() as i64))
                .or_default();
            for name in pieces {
                if !e.0.contains(&name) {
                    e.0.push(name);
                }
            }
            e.1 += 1;
        };
        for b in p.buttons().values() {
            if let Some(g) = garment(b.panel) {
                add(
                    g,
                    0,
                    b.diameter_mm,
                    vec![piece(b.panel), piece(b.hole.panel)],
                );
            }
        }
        for z in p.zippers().values() {
            let (Some(a), Some(len)) = (p.side_panel(&z.a), p.zipper_length(z)) else {
                continue;
            };
            let Some(g) = garment(a) else {
                continue;
            };
            let mut pieces = vec![piece(a)];
            pieces.extend(p.side_panel(&z.b).map(piece));
            // Zippers are bought by the centimetre.
            add(g, 1, (len / 10.0).ceil() * 10.0, pieces);
        }
        rows.into_iter()
            .map(|((g, kind, size), (pieces, count))| {
                let size = size as f64 / 10.0;
                vec![
                    self.garment_name(g),
                    self.t(if kind == 0 { "button" } else { "zipper" }),
                    if kind == 0 {
                        format!("{} mm", self.num(size, 0))
                    } else {
                        format!("{} cm", self.num(size / 10.0, 0))
                    },
                    count.to_string(),
                    pieces.join(", "),
                ]
            })
            .collect()
    }

    /// A piece's extent along its grainline and across it (mm).
    fn extent(project: &Project, id: PanelId) -> Option<(f64, f64)> {
        let panel = project.panel(id)?;
        let outline = project.panel_outline(id)?;
        let a = -panel.grain_angle_deg.to_radians();
        let (s, c) = a.sin_cos();
        let turned: Vec<DVec2> = outline
            .iter()
            .map(|p| DVec2::new(p.x * c - p.y * s, p.x * s + p.y * c))
            .collect();
        let bb = r3d_geom::polygon::bounding_box(&turned)?;
        let size = bb.max - bb.min;
        Some((size.x, size.y))
    }

    fn measurements(&self) -> Section {
        let p = self.project;
        let mut blocks = Vec::new();
        let sizes = self.sizes();
        if let Some(g) = p.grading()
            && p.avatar().is_some()
        {
            let rows = (0..g.sizes.len())
                .filter_map(|i| {
                    let body = p.size_body(i)?;
                    let mut name = g.sizes.get(i)?.name.clone();
                    if i == g.base {
                        name.push_str(" *");
                    }
                    Some(vec![
                        name,
                        self.num(body.height_cm, 1),
                        self.num(body.bust_cm, 1),
                        self.num(body.waist_cm, 1),
                        self.num(body.hips_cm, 1),
                    ])
                })
                .collect();
            blocks.push(Block::Subheading(self.t("size_chart")));
            blocks.push(Block::Table {
                columns: vec![
                    Column::text(self.t("size"), 1.0),
                    Column::number(self.t("height"), 1.0),
                    Column::number(self.t("bust"), 1.0),
                    Column::number(self.t("waist"), 1.0),
                    Column::number(self.t("hips"), 1.0),
                ],
                rows,
            });
        }
        let ids: Vec<PanelId> = self
            .garments()
            .into_iter()
            .flat_map(|g| {
                p.panels()
                    .iter()
                    .filter(move |(_, panel)| panel.garment == g)
                    .map(|(id, _)| *id)
            })
            .collect();
        let rows = ids
            .iter()
            .filter_map(|id| {
                let panel = p.panel(*id)?;
                let cut = Cut::of(p, *id)?;
                let (len, wid) = Self::extent(p, *id)?;
                let gsm = p.garment_of(*id).map_or(0.0, |g| g.weight_gsm());
                Some(vec![
                    self.piece_name(p, *id),
                    self.num(len / 10.0, 1),
                    self.num(wid / 10.0, 1),
                    self.num(p.panel_perimeter(*id).unwrap_or_default() / 10.0, 1),
                    self.num(cut.area() / 100.0, 0),
                    self.num(cut.cut_area() / 100.0, 0),
                    self.num(panel.seam_allowance_mm, 0),
                    self.num(mass_g(cut.cut_area(), gsm), 1),
                ])
            })
            .collect();
        let mut title = self.t("pieces");
        if let Some(g) = p.grading()
            && let Some(base) = g.sizes.get(g.base)
        {
            title = format!("{title} - {}", base.name);
        }
        blocks.push(Block::Subheading(title));
        blocks.push(Block::Table {
            columns: vec![
                Column::text(self.t("piece"), 2.6),
                Column::number(self.t("length"), 1.0),
                Column::number(self.t("piece_width"), 1.0),
                Column::number(self.t("perimeter"), 1.2),
                Column::number(self.t("area"), 1.0),
                Column::number(self.t("cut_area"), 1.1),
                Column::number(self.t("allowance"), 1.1),
                Column::number(self.t("mass"), 0.9),
            ],
            rows,
        });
        if !sizes.is_empty() {
            let sized: Vec<Project> = (0..sizes.len()).map(|i| p.at_size(i)).collect();
            let mut columns = vec![Column::text(self.t("piece"), 2.2)];
            columns.extend(sizes.iter().map(|s| Column::number(s.clone(), 1.0)));
            let rows = ids
                .iter()
                .map(|id| {
                    let mut row = vec![self.piece_name(p, *id)];
                    row.extend(sized.iter().map(|sp| {
                        Self::extent(sp, *id).map_or_else(
                            || "-".to_owned(),
                            |(l, w)| {
                                format!("{} x {}", self.num(l / 10.0, 1), self.num(w / 10.0, 1))
                            },
                        )
                    }));
                    row
                })
                .collect();
            blocks.push(Block::Subheading(self.t("pieces_by_size")));
            blocks.push(Block::Table { columns, rows });
        }
        Section {
            title: self.t("measurements"),
            blocks,
        }
    }

    fn pattern(&self) -> Section {
        let p = self.project;
        let every: Vec<usize> = p
            .grading()
            .map(|g| (0..g.sizes.len()).collect())
            .unwrap_or_default();
        let mut blocks = Vec::new();
        if !every.is_empty() {
            let colors = r3d_io::drawing::size_colors(p, &every);
            blocks.push(Block::Legend(
                colors.into_iter().zip(self.sizes()).collect(),
            ));
        }
        blocks.push(Block::Drawing {
            drawing: r3d_io::pieces_drawing(p, &every, None),
            caption: self.t("pattern"),
        });
        Section {
            title: self.t("pattern"),
            blocks,
        }
    }

    fn sewing(&self) -> Section {
        let p = self.project;
        let side = |s: &r3d_core::SeamSide| -> (String, String) {
            let name = p
                .side_panel(s)
                .map(|id| self.piece_name(p, id))
                .unwrap_or_default();
            let len = p.side_length(s).unwrap_or_default();
            (name, self.num(len, 1))
        };
        let rows: Vec<Vec<String>> = p
            .seams()
            .values()
            .enumerate()
            .map(|(i, s)| {
                let (a, la) = side(&s.a);
                let (b, lb) = side(&s.b);
                vec![
                    format!("{}", i + 1),
                    a,
                    b,
                    la,
                    lb,
                    if s.ease_mm == 0.0 {
                        String::new()
                    } else {
                        self.num(s.ease_mm, 1)
                    },
                    if s.gathered {
                        self.t("yes")
                    } else {
                        String::new()
                    },
                ]
            })
            .collect();
        let mut blocks = vec![Block::Subheading(self.t("seams"))];
        if rows.is_empty() {
            blocks.push(Block::Text(self.t("no_seams")));
        } else {
            blocks.push(Block::Table {
                columns: vec![
                    Column::number("#", 0.4),
                    Column::text(self.t("side_a"), 2.2),
                    Column::text(self.t("side_b"), 2.2),
                    Column::number(self.t("length_a"), 1.2),
                    Column::number(self.t("length_b"), 1.2),
                    Column::number(self.t("ease"), 0.9),
                    Column::text(self.t("gathered"), 0.9),
                ],
                rows,
            });
        }
        for id in self.garments() {
            if let Some(g) = p.garments().get(&id)
                && !g.notes.trim().is_empty()
            {
                let title = if p.garments().len() > 1 {
                    format!("{} - {}", self.t("notes"), g.name)
                } else {
                    self.t("notes")
                };
                blocks.push(Block::Subheading(title));
                blocks.push(Block::Text(g.notes.clone()));
            }
        }
        Section {
            title: self.t("sewing"),
            blocks,
        }
    }
}

/// The tech pack of `project` as PDF bytes, with `images` (renders of
/// the garment) first, and what each garment takes of its fabric.
pub fn techpack(
    project: &Project,
    labels: &BTreeMap<String, String>,
    paper: TechPackPaper,
    images: Vec<Image>,
) -> Result<(Vec<u8>, Value), CommandError> {
    if project.panels().is_empty() {
        return Err(CommandError::invalid("panels", Invalid::Empty));
    }
    let w = Writer { project, labels };
    let consumption: Vec<Consumption> = w
        .garments()
        .into_iter()
        .filter(|g| project.panels().values().any(|p| p.garment == *g))
        .map(|g| w.consumption(g, None))
        .collect();
    let mut sections = Vec::new();
    if !images.is_empty() {
        sections.push(Section {
            title: w.t("views"),
            blocks: vec![Block::Images(images)],
        });
    }
    sections.push(w.colourways());
    sections.push(w.materials(&consumption));
    sections.push(w.measurements());
    sections.push(w.pattern());
    sections.push(w.sewing());
    let tp = TechPack {
        title: project.name().to_owned(),
        subtitle: w.subtitle(),
        sections,
        page: match paper {
            TechPackPaper::A4 => PageSize::A4,
            TechPackPaper::Letter => PageSize::Letter,
        },
    };
    let fabric: Vec<Value> = consumption
        .iter()
        .map(|c| json!({ "garment": c.garment, "yardage_m": c.yardage_m, "cost": c.cost }))
        .collect();
    Ok((techpack_pdf(&tp), json!({ "fabric": fabric })))
}

impl ExportTechPack {
    /// The text of label `key` in this request.
    pub fn label<'a>(&'a self, key: &'a str) -> &'a str {
        label(&self.labels, key)
    }

    /// Write the tech pack with `views` (renders the caller made) before
    /// the pictures of `images`.
    pub fn write(&self, ws: &Workspace, views: Vec<Image>) -> Result<Value, WorkspaceError> {
        let mut pictures = views;
        for im in &self.images {
            let bytes = ws.read(&im.path)?;
            let caption = im.caption.clone().unwrap_or_default();
            pictures.push(Image::from_png(&bytes, caption)?);
        }
        let count = pictures.len();
        let (bytes, mut out) =
            techpack(ws.session().project(), &self.labels, self.paper, pictures)?;
        let path = self
            .path
            .clone()
            .unwrap_or_else(|| format!("exports/{}-techpack.pdf", export_stem(ws)));
        let written = ws.write(&path, &bytes)?;
        out["path"] = json!(written);
        out["images"] = json!(count);
        Ok(out)
    }
}

impl ActionSpec for ExportTechPack {
    fn run(&self, ws: &mut Workspace) -> Result<Value, WorkspaceError> {
        self.write(ws, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_fall_back_to_english() {
        let mut labels = BTreeMap::new();
        labels.insert("views".to_owned(), "Vues".to_owned());
        assert_eq!(label(&labels, "views"), "Vues");
        assert_eq!(label(&labels, "front"), "Front");
        let keys: Vec<&str> = LABELS.iter().map(|(k, _)| *k).collect();
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(keys.len(), unique.len());
    }
}
