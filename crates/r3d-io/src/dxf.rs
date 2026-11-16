//! DXF-AAMA (ASTM D6673) pattern exchange: ASCII DXF where each piece is
//! a block, and layers say what an entity is:
//!
//! | Layer | Meaning |
//! |---|---|
//! | 1 | boundary (cutting line), piece texts |
//! | 2 | turn points (corners of the boundary) |
//! | 3 | curve points |
//! | 4, 80–83 | notches |
//! | 7 | grainline |
//! | 8 | internal lines |
//! | 11 | internal cutouts |
//! | 14 | sewing line |
//!
//! Piece texts are `Piece Name: …` and `Quantity: …`. Curves are
//! polylines; on import they are fitted back to cubics (between turn
//! points when the file has them). Units come from `$INSUNITS` or a
//! `Units: METRIC|ENGLISH` text. A DXF without AAMA layers is read as a
//! plain drawing ([`crate::import`]'s vector rules).

use std::fmt::Write;

use r3d_core::measure::FLATTEN_TOLERANCE_MM;
use r3d_core::{EdgeCurve, Project};
use r3d_geom::fit::fit_polyline;
use r3d_geom::{CubicBez, DVec2, polygon};

use crate::import::vector::{Drawing, Shape, Text};
use crate::import::{
    ImportError, ImportOptions, ImportedPattern, ImportedPiece, Path, ScaleSource,
    distance_to_polyline, finish, nearest_on, seam_allowance, simplify,
};

/// Max distance between an exported polyline and the curve (mm).
const EXPORT_TOLERANCE_MM: f64 = 0.05;
/// A point of the contour is a turn point when the outline turns there by
/// more than this (degrees).
const TURN_DEG: f64 = 1.0;

const BOUNDARY: &str = "1";
const TURN: &str = "2";
const CURVE: &str = "3";
const NOTCH: &str = "4";
const GRAIN: &str = "7";
const INTERNAL: &str = "8";
const CUTOUT: &str = "11";
const SEW: &str = "14";
const NOTCH_TYPES: [&str; 4] = ["80", "81", "82", "83"];

// ---------------------------------------------------------------- export

struct Writer(String);

impl Writer {
    fn pair(&mut self, code: i32, value: impl std::fmt::Display) {
        let _ = write!(self.0, "{code}\n{value}\n");
    }

    fn num(&mut self, code: i32, v: f64) {
        // Four decimals (0.1 µm), without trailing zeros.
        let s = format!("{v:.4}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        let s = if s == "-0" { "0" } else { s };
        self.pair(code, s);
    }

    fn point(&mut self, layer: &str, p: DVec2) {
        self.pair(0, "POINT");
        self.pair(8, layer);
        self.num(10, p.x);
        self.num(20, p.y);
        self.num(30, 0.0);
    }

    fn line(&mut self, layer: &str, a: DVec2, b: DVec2) {
        self.pair(0, "LINE");
        self.pair(8, layer);
        self.num(10, a.x);
        self.num(20, a.y);
        self.num(30, 0.0);
        self.num(11, b.x);
        self.num(21, b.y);
        self.num(31, 0.0);
    }

    fn polyline(&mut self, layer: &str, pts: &[DVec2], closed: bool) {
        self.pair(0, "POLYLINE");
        self.pair(8, layer);
        self.pair(66, 1);
        self.num(10, 0.0);
        self.num(20, 0.0);
        self.num(30, 0.0);
        self.pair(70, i32::from(closed));
        for p in pts {
            self.pair(0, "VERTEX");
            self.pair(8, layer);
            self.num(10, p.x);
            self.num(20, p.y);
            self.num(30, 0.0);
        }
        self.pair(0, "SEQEND");
        self.pair(8, layer);
    }

    fn text(&mut self, layer: &str, at: DVec2, height: f64, text: &str) {
        self.pair(0, "TEXT");
        self.pair(8, layer);
        self.num(10, at.x);
        self.num(20, at.y);
        self.num(30, 0.0);
        self.num(40, height);
        // DXF text is one line.
        self.pair(1, text.replace(['\n', '\r'], " "));
    }
}

/// The pattern as DXF-AAMA, in millimetres. Each panel is a block drawn
/// where it is on the board: the boundary is the cutting line (the
/// sewing line offset by the seam allowance), with the sewing line on its
/// own layer when they differ.
pub fn to_dxf(project: &Project) -> String {
    let mut w = Writer(String::new());
    w.pair(0, "SECTION");
    w.pair(2, "HEADER");
    w.pair(9, "$ACADVER");
    w.pair(1, "AC1009");
    w.pair(9, "$INSUNITS");
    w.pair(70, 4); // millimetres
    w.pair(9, "$MEASUREMENT");
    w.pair(70, 1); // metric
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "TABLES");
    w.pair(0, "TABLE");
    w.pair(2, "LAYER");
    let layers = ["0", BOUNDARY, TURN, CURVE, NOTCH, GRAIN, INTERNAL, SEW];
    w.pair(70, layers.len());
    for l in layers {
        w.pair(0, "LAYER");
        w.pair(2, l);
        w.pair(70, 0);
        w.pair(62, 7);
        w.pair(6, "CONTINUOUS");
    }
    w.pair(0, "ENDTAB");
    w.pair(0, "ENDSEC");

    let mut names = Vec::new();
    w.pair(0, "SECTION");
    w.pair(2, "BLOCKS");
    for (k, (id, panel)) in project.panels().iter().enumerate() {
        let Some(sew) = sewing_line(project, *id) else {
            continue;
        };
        let block = format!("P{}", k + 1);
        w.pair(0, "BLOCK");
        w.pair(8, "0");
        w.pair(2, &block);
        w.pair(70, 0);
        w.num(10, 0.0);
        w.num(20, 0.0);
        w.num(30, 0.0);
        w.pair(3, &block);
        let sa = panel.seam_allowance_mm;
        let flat: Vec<DVec2> = sew.iter().map(|(p, _)| *p).collect();
        if sa > 0.0 {
            w.polyline(BOUNDARY, &polygon::offset_outward(&flat, sa), true);
            w.polyline(SEW, &flat, true);
        } else {
            w.polyline(BOUNDARY, &flat, true);
            for (p, kind) in &sew {
                match kind {
                    Vertex::Turn => w.point(TURN, *p),
                    Vertex::Curve => w.point(CURVE, *p),
                    Vertex::Between => {}
                }
            }
        }
        for (nid, notch) in project.notches() {
            if !panel.contour.contains(&notch.edge) {
                continue;
            }
            if let Some(at) = project.notch_placement(*nid) {
                let outward = DVec2::new(at.tangent.y, -at.tangent.x);
                w.point(NOTCH, at.pos + outward * sa);
            }
        }
        if let Some((a, b)) = crate::drawing::grainline(&flat, panel.grain_angle_deg) {
            w.line(GRAIN, a, b);
            let side = (a + b) * 0.5 + (b - a).normalize_or_zero().perp() * 15.0;
            w.text(BOUNDARY, side, 5.0, &format!("Piece Name: {}", panel.name));
            w.text(BOUNDARY, side - DVec2::new(0.0, 8.0), 5.0, "Quantity: 1");
        }
        for (lid, _) in project
            .internal_lines()
            .iter()
            .filter(|(_, l)| l.panel == *id)
        {
            if let Some(pts) = project.internal_line_polyline(*lid) {
                w.polyline(INTERNAL, &thin(&pts), false);
            }
        }
        w.pair(0, "ENDBLK");
        w.pair(8, "0");
        names.push(block);
    }
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "ENTITIES");
    w.text(
        "1",
        DVec2::ZERO,
        5.0,
        &format!("Style Name: {}", project.name()),
    );
    w.text("1", DVec2::new(0.0, -8.0), 5.0, "Units: METRIC");
    for block in names {
        w.pair(0, "INSERT");
        w.pair(8, "1");
        w.pair(2, block);
        w.num(10, 0.0);
        w.num(20, 0.0);
        w.num(30, 0.0);
    }
    w.pair(0, "ENDSEC");
    w.pair(0, "EOF");
    w.0
}

#[derive(Clone, Copy, PartialEq)]
enum Vertex {
    /// A contour point where the outline turns.
    Turn,
    /// A contour point on a smooth curve.
    Curve,
    /// A point added to draw a curve.
    Between,
}

/// The panel's sewing line as a polygon, each vertex marked.
fn sewing_line(project: &Project, panel: r3d_core::PanelId) -> Option<Vec<(DVec2, Vertex)>> {
    let contour = &project.panel(panel)?.contour;
    let n = contour.len();
    let mut out = Vec::new();
    for (i, e) in contour.iter().enumerate() {
        let edge = project.edge(*e)?;
        let a = project.point(edge.start)?.pos;
        let b = project.point(edge.end)?.pos;
        let prev = project.edge(contour[(i + n - 1) % n])?;
        let prev_a = project.point(prev.start)?.pos;
        let incoming = end_tangent(prev_a, &prev.curve, a);
        let outgoing = start_tangent(a, &edge.curve, b);
        let kind = if incoming.angle_to(outgoing).abs().to_degrees() > TURN_DEG {
            Vertex::Turn
        } else {
            Vertex::Curve
        };
        out.push((a, kind));
        if let EdgeCurve::Cubic { c1, c2 } = &edge.curve {
            let mut pts = Vec::new();
            CubicBez::new(a, *c1, *c2, b).flatten_into(EXPORT_TOLERANCE_MM, &mut pts);
            pts.pop(); // `b` comes with the next edge
            out.extend(pts.into_iter().map(|p| (p, Vertex::Between)));
        }
    }
    Some(out)
}

fn start_tangent(a: DVec2, curve: &EdgeCurve, b: DVec2) -> DVec2 {
    match curve {
        EdgeCurve::Line => b - a,
        EdgeCurve::Cubic { c1, c2 } => CubicBez::new(a, *c1, *c2, b).tangent(0.0),
    }
}

fn end_tangent(a: DVec2, curve: &EdgeCurve, b: DVec2) -> DVec2 {
    match curve {
        EdgeCurve::Line => b - a,
        EdgeCurve::Cubic { c1, c2 } => CubicBez::new(a, *c1, *c2, b).tangent(1.0),
    }
}

/// A polyline flattened finely by the document, with points dropped while
/// it stays within the export tolerance of the original.
fn thin(pts: &[DVec2]) -> Vec<DVec2> {
    let mut out: Vec<DVec2> = Vec::new();
    let mut anchor = 0;
    for i in 1..pts.len() {
        let straight = pts[anchor + 1..i].iter().all(|p| {
            r3d_geom::fit::distance_to_segment(*p, pts[anchor], pts[i])
                <= EXPORT_TOLERANCE_MM - FLATTEN_TOLERANCE_MM
        });
        if !straight {
            out.push(pts[anchor]);
            anchor = i - 1;
        }
    }
    out.push(pts[anchor]);
    if let Some(last) = pts.last()
        && out.last() != Some(last)
    {
        out.push(*last);
    }
    out
}

// ---------------------------------------------------------------- import

#[derive(Clone, Debug, PartialEq)]
enum Geometry {
    Polyline(Vec<DVec2>, bool),
    Point(DVec2),
    Text(DVec2, f64, String),
}

#[derive(Clone, Debug, PartialEq)]
struct Entity {
    layer: String,
    geometry: Geometry,
}

/// One entity's group codes, in order.
type Fields = Vec<(i32, String)>;

fn get(f: &Fields, code: i32) -> Option<&str> {
    f.iter().find(|(c, _)| *c == code).map(|(_, v)| v.as_str())
}

fn getf(f: &Fields, code: i32) -> Option<f64> {
    get(f, code)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn malformed(detail: impl ToString) -> ImportError {
    ImportError::Malformed {
        format: "DXF",
        detail: detail.to_string(),
    }
}

/// An INSERT: which block, and how it is placed.
struct Insert {
    block: String,
    at: DVec2,
    scale: DVec2,
    rotation_deg: f64,
}

struct Block {
    base: DVec2,
    entities: Vec<Entity>,
}

/// Read the pattern pieces of a DXF-AAMA file (or of a plain DXF drawing).
pub fn read(bytes: &[u8], options: &ImportOptions) -> Result<ImportedPattern, ImportError> {
    if bytes.starts_with(b"AutoCAD Binary DXF") {
        return Err(ImportError::BinaryDxf);
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t.to_owned(),
        // Older files are in a Windows code page; Latin-1 keeps the digits.
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    };
    let mut lines = text.lines();
    let mut pairs: Vec<(i32, String)> = Vec::new();
    while let Some(code) = lines.next() {
        let Some(value) = lines.next() else {
            break;
        };
        let code = code
            .trim()
            .parse::<i32>()
            .map_err(|_| malformed(format!("group code expected, found `{}`", code.trim())))?;
        pairs.push((code, value.trim_end().to_owned()));
    }
    if pairs.is_empty() {
        return Err(malformed("empty file"));
    }

    // Sections → entity lists.
    let mut insunits: Option<i64> = None;
    let mut blocks: Vec<(String, Block)> = Vec::new();
    let mut loose: Vec<Entity> = Vec::new();
    let mut inserts: Vec<Insert> = Vec::new();
    let mut section = String::new();
    let mut i = 0;
    while i < pairs.len() {
        let (code, value) = &pairs[i];
        if *code == 0 && value == "SECTION" {
            section = pairs
                .get(i + 1)
                .filter(|(c, _)| *c == 2)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            i += 2;
            continue;
        }
        if section == "HEADER" && *code == 9 && value == "$INSUNITS" {
            insunits = pairs
                .get(i + 1)
                .and_then(|(_, v)| v.trim().parse::<i64>().ok());
        }
        if *code != 0 || !(section == "BLOCKS" || section == "ENTITIES") {
            i += 1;
            continue;
        }
        // An entity: its fields up to the next code 0.
        let kind = value.clone();
        let mut fields: Fields = Vec::new();
        i += 1;
        while i < pairs.len() && pairs[i].0 != 0 {
            fields.push(pairs[i].clone());
            i += 1;
        }
        match kind.as_str() {
            "BLOCK" => {
                let name = get(&fields, 2).unwrap_or_default().to_owned();
                let base = DVec2::new(
                    getf(&fields, 10).unwrap_or(0.0),
                    getf(&fields, 20).unwrap_or(0.0),
                );
                blocks.push((
                    name,
                    Block {
                        base,
                        entities: Vec::new(),
                    },
                ));
            }
            "ENDBLK" | "ENDSEC" | "SEQEND" | "VERTEX" => {}
            "INSERT" if section == "ENTITIES" => inserts.push(Insert {
                block: get(&fields, 2).unwrap_or_default().to_owned(),
                at: DVec2::new(
                    getf(&fields, 10).unwrap_or(0.0),
                    getf(&fields, 20).unwrap_or(0.0),
                ),
                scale: DVec2::new(
                    getf(&fields, 41).unwrap_or(1.0),
                    getf(&fields, 42).unwrap_or(1.0),
                ),
                rotation_deg: getf(&fields, 50).unwrap_or(0.0),
            }),
            _ => {
                // A POLYLINE's vertices follow it as their own entities.
                let mut vertices: Vec<Fields> = Vec::new();
                if kind == "POLYLINE" {
                    while i < pairs.len() && pairs[i].1 == "VERTEX" && pairs[i].0 == 0 {
                        let mut vf: Fields = Vec::new();
                        i += 1;
                        while i < pairs.len() && pairs[i].0 != 0 {
                            vf.push(pairs[i].clone());
                            i += 1;
                        }
                        vertices.push(vf);
                    }
                }
                if let Some(e) = entity(&kind, &fields, &vertices) {
                    match (section.as_str(), blocks.last_mut()) {
                        ("BLOCKS", Some((_, b))) => b.entities.push(e),
                        _ => loose.push(e),
                    }
                }
            }
        }
    }

    // Units.
    let all_texts = blocks
        .iter()
        .flat_map(|(_, b)| b.entities.iter())
        .chain(loose.iter())
        .filter_map(|e| match &e.geometry {
            Geometry::Text(_, _, t) => Some(t.to_lowercase()),
            _ => None,
        });
    let mut stated: Option<f64> = match insunits {
        Some(1) => Some(25.4),
        Some(4) => Some(1.0),
        Some(5) => Some(10.0),
        Some(6) => Some(1000.0),
        _ => None,
    };
    for t in all_texts {
        if let Some(u) = t.strip_prefix("units:") {
            let u = u.trim();
            if stated.is_none() {
                stated = match u {
                    "metric" | "mm" => Some(1.0),
                    "english" | "inch" | "inches" => Some(25.4),
                    "cm" => Some(10.0),
                    _ => None,
                };
            }
        }
    }
    let (mm_per_unit, scale) = options.mm_per_unit(match stated {
        Some(mm) => (mm, ScaleSource::File),
        None => (1.0, ScaleSource::Assumed),
    });

    // Groups of entities: one per inserted block, plus the loose ones.
    let mut groups: Vec<Vec<Entity>> = Vec::new();
    for ins in &inserts {
        let Some((_, block)) = blocks.iter().find(|(n, _)| *n == ins.block) else {
            continue;
        };
        let rot = DVec2::from_angle(ins.rotation_deg.to_radians());
        let place = |p: DVec2| ins.at + rot.rotate((p - block.base) * ins.scale);
        groups.push(
            block
                .entities
                .iter()
                .map(|e| transformed(e, &place))
                .collect(),
        );
    }
    if !loose.is_empty() {
        groups.push(loose);
    }
    let to_mm = |p: DVec2| p * mm_per_unit;
    let groups: Vec<Vec<Entity>> = groups
        .into_iter()
        .map(|g| g.iter().map(|e| transformed(e, &to_mm)).collect())
        .collect();

    let aama = groups
        .iter()
        .flatten()
        .any(|e| e.layer == BOUNDARY && matches!(e.geometry, Geometry::Polyline(_, true)));
    let (raw, ignored) = if aama {
        aama_pieces(&groups)
    } else {
        let mut d = Drawing::default();
        for e in groups.iter().flatten() {
            match &e.geometry {
                Geometry::Polyline(pts, closed) => {
                    if let Some(path) = Path::from_polyline(pts) {
                        d.shapes.push(Shape {
                            path,
                            closed: *closed,
                            dashed: false,
                        });
                    }
                }
                Geometry::Text(pos, size, text) => d.texts.push(Text {
                    pos: *pos,
                    text: text.clone(),
                    size_mm: *size,
                }),
                Geometry::Point(_) => {}
            }
        }
        crate::import::vector::pieces(&d)
    };
    finish(raw, ignored, mm_per_unit, scale)
}

fn transformed(e: &Entity, f: &impl Fn(DVec2) -> DVec2) -> Entity {
    let geometry = match &e.geometry {
        Geometry::Polyline(pts, closed) => {
            Geometry::Polyline(pts.iter().map(|p| f(*p)).collect(), *closed)
        }
        Geometry::Point(p) => Geometry::Point(f(*p)),
        Geometry::Text(p, h, t) => {
            let scale = f(DVec2::X).distance(f(DVec2::ZERO));
            Geometry::Text(f(*p), h * scale, t.clone())
        }
    };
    Entity {
        layer: e.layer.clone(),
        geometry,
    }
}

/// The geometry of one entity, in drawing units.
fn entity(kind: &str, f: &Fields, vertices: &[Fields]) -> Option<Entity> {
    let layer = get(f, 8).unwrap_or("0").trim().to_owned();
    let xy = |cx: i32, cy: i32| Some(DVec2::new(getf(f, cx)?, getf(f, cy)?));
    let geometry = match kind {
        "LINE" => Geometry::Polyline(vec![xy(10, 20)?, xy(11, 21)?], false),
        "POINT" => Geometry::Point(xy(10, 20)?),
        "TEXT" | "MTEXT" => Geometry::Text(
            xy(10, 20)?,
            getf(f, 40).unwrap_or(1.0),
            get(f, 1)?.to_owned(),
        ),
        "POLYLINE" => {
            let closed = get(f, 70)
                .and_then(|v| v.trim().parse::<i64>().ok())
                .unwrap_or(0)
                & 1
                == 1;
            let pts: Vec<(DVec2, f64)> = vertices
                .iter()
                .filter_map(|v| {
                    Some((
                        DVec2::new(getf(v, 10)?, getf(v, 20)?),
                        getf(v, 42).unwrap_or(0.0),
                    ))
                })
                .collect();
            Geometry::Polyline(with_bulges(&pts, closed), closed)
        }
        "LWPOLYLINE" => {
            let closed = get(f, 70)
                .and_then(|v| v.trim().parse::<i64>().ok())
                .unwrap_or(0)
                & 1
                == 1;
            // Bulges (42) follow the vertex they belong to.
            let mut pts: Vec<(DVec2, f64)> = Vec::new();
            let mut x = None;
            for (code, value) in f {
                let v = value.trim().parse::<f64>().ok().filter(|v| v.is_finite());
                match (code, v) {
                    (10, Some(v)) => x = Some(v),
                    (20, Some(y)) => {
                        if let Some(x) = x.take() {
                            pts.push((DVec2::new(x, y), 0.0));
                        }
                    }
                    (42, Some(b)) => {
                        if let Some(last) = pts.last_mut() {
                            last.1 = b;
                        }
                    }
                    _ => {}
                }
            }
            Geometry::Polyline(with_bulges(&pts, closed), closed)
        }
        "CIRCLE" | "ARC" => {
            let c = xy(10, 20)?;
            let r = getf(f, 40)?;
            let (a0, a1) = if kind == "CIRCLE" {
                (0.0, 360.0)
            } else {
                let a0 = getf(f, 50)?;
                let mut a1 = getf(f, 51)?;
                if a1 <= a0 {
                    a1 += 360.0;
                }
                (a0, a1)
            };
            let steps = ((a1 - a0) / 5.0).ceil().max(2.0) as usize;
            let mut pts: Vec<DVec2> = (0..=steps)
                .map(|k| {
                    c + DVec2::from_angle((a0 + (a1 - a0) * k as f64 / steps as f64).to_radians())
                        * r
                })
                .collect();
            let closed = kind == "CIRCLE";
            if closed {
                pts.pop();
            }
            Geometry::Polyline(pts, closed)
        }
        _ => return None,
    };
    Some(Entity { layer, geometry })
}

/// A polyline's points, with arcs (`bulge` = tan(angle / 4)) sampled.
fn with_bulges(pts: &[(DVec2, f64)], closed: bool) -> Vec<DVec2> {
    let n = pts.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (a, bulge) = pts[i];
        out.push(a);
        let last = i + 1 == n;
        if bulge.abs() < 1e-9 || (last && !closed) {
            continue;
        }
        let b = pts[(i + 1) % n].0;
        let angle = 4.0 * bulge.atan();
        let chord = b - a;
        let len = chord.length();
        if len <= 0.0 {
            continue;
        }
        // Centre: on the chord's perpendicular bisector.
        let r = len / (2.0 * (angle / 2.0).sin());
        let h = r * (angle / 2.0).cos();
        let mid = (a + b) * 0.5;
        let centre = mid + chord.perp().normalize() * h;
        let steps = (angle.abs().to_degrees() / 5.0).ceil().max(2.0) as usize;
        let start = (a - centre).to_angle();
        for k in 1..steps {
            let t = start + angle * k as f64 / steps as f64;
            out.push(centre + DVec2::from_angle(t) * r.abs());
        }
    }
    out
}

/// Pieces from AAMA layers: each group (block) holds one piece per
/// boundary; loose entities go to the boundary that holds them.
fn aama_pieces(groups: &[Vec<Entity>]) -> (Vec<(ImportedPiece, Path)>, usize) {
    let mut out = Vec::new();
    let mut ignored = 0;
    for group in groups {
        let boundaries: Vec<&Vec<DVec2>> = group
            .iter()
            .filter(|e| e.layer == BOUNDARY)
            .filter_map(|e| match &e.geometry {
                Geometry::Polyline(p, true) if p.len() >= 3 => Some(p),
                _ => None,
            })
            .collect();
        let owner = |p: DVec2| -> Option<usize> {
            if boundaries.len() == 1 {
                return Some(0);
            }
            boundaries
                .iter()
                .position(|b| polygon::contains(b, p) || distance_to_polyline(p, b, true) < 1.0)
        };
        let mut drafts: Vec<Vec<&Entity>> = vec![Vec::new(); boundaries.len()];
        for e in group {
            let at = match &e.geometry {
                Geometry::Polyline(p, _) => p.first().copied(),
                Geometry::Point(p) | Geometry::Text(p, _, _) => Some(*p),
            };
            match at.and_then(owner) {
                Some(k) => drafts[k].push(e),
                None if e.layer != BOUNDARY => ignored += 1,
                None => {}
            }
        }
        for (cut, entities) in boundaries.iter().zip(drafts) {
            out.push(aama_piece(cut, &entities));
        }
    }
    (out, ignored)
}

fn aama_piece(cut: &[DVec2], entities: &[&Entity]) -> (ImportedPiece, Path) {
    let on = |layers: &[&str]| -> Vec<&Entity> {
        entities
            .iter()
            .copied()
            .filter(|e| layers.contains(&e.layer.as_str()))
            .collect()
    };
    let sew = on(&[SEW]).into_iter().find_map(|e| match &e.geometry {
        Geometry::Polyline(p, true) if p.len() >= 3 => Some(p.clone()),
        _ => None,
    });
    let (outline_pts, sa, turns) = match sew
        .as_ref()
        .and_then(|s| seam_allowance(cut, s).map(|sa| (s, sa)))
    {
        Some((s, sa)) => (s.clone(), sa, Vec::new()),
        None => {
            let turns: Vec<DVec2> = on(&[TURN])
                .into_iter()
                .filter_map(|e| match e.geometry {
                    Geometry::Point(p) => Some(p),
                    _ => None,
                })
                .collect();
            (cut.to_vec(), 0.0, turns)
        }
    };
    let mut raw_pts = outline_pts.clone();
    raw_pts.push(outline_pts[0]);
    let raw = Path::from_polyline(&raw_pts).unwrap_or(Path {
        start: outline_pts[0],
        segments: Vec::new(),
    });
    let outline = fit_between_turns(&outline_pts, &turns).unwrap_or_else(|| simplify(&raw, true));

    let mut notch_layers = vec![NOTCH];
    notch_layers.extend(NOTCH_TYPES);
    // Notches sit on the cutting line; they go on the sewing line.
    let notches = on(&notch_layers)
        .into_iter()
        .filter_map(|e| match &e.geometry {
            Geometry::Point(p) => Some(*p),
            // A notch drawn as a tick: the end on the cutting line.
            Geometry::Polyline(p, _) => p.iter().copied().min_by(|a, b| {
                distance_to_polyline(*a, cut, true).total_cmp(&distance_to_polyline(*b, cut, true))
            }),
            Geometry::Text(..) => None,
        })
        .map(|p| nearest_on(p, &outline_pts))
        .collect();
    let grain = on(&[GRAIN]).into_iter().find_map(|e| match &e.geometry {
        Geometry::Polyline(p, _) if p.len() >= 2 => Some((p[0], p[p.len() - 1])),
        _ => None,
    });
    let internal_lines = on(&[INTERNAL, CUTOUT])
        .into_iter()
        .filter_map(|e| match &e.geometry {
            Geometry::Polyline(p, closed) if p.len() >= 2 => {
                let mut pts = p.clone();
                if *closed {
                    pts.push(p[0]);
                }
                Path::from_polyline(&pts).map(|path| simplify(&path, false))
            }
            _ => None,
        })
        .collect();
    let name = entities.iter().find_map(|e| match &e.geometry {
        Geometry::Text(_, _, t) => {
            let label = "piece name:";
            let head = t.get(..label.len())?;
            head.eq_ignore_ascii_case(label)
                .then(|| t.get(label.len()..).unwrap_or_default().trim().to_owned())
                .filter(|n| !n.is_empty())
        }
        _ => None,
    });
    (
        ImportedPiece {
            name,
            outline,
            seam_allowance_mm: sa,
            grain,
            notches,
            internal_lines,
        },
        raw,
    )
}

/// The outline fitted run by run between its turn points.
fn fit_between_turns(pts: &[DVec2], turns: &[DVec2]) -> Option<Path> {
    let n = pts.len();
    let corners: Vec<usize> = (0..n)
        .filter(|&i| turns.iter().any(|t| t.distance(pts[i]) < 0.01))
        .collect();
    let first = *corners.first()?;
    let mut segments = Vec::new();
    for (k, &from) in corners.iter().enumerate() {
        let to = corners[(k + 1) % corners.len()];
        let mut run = vec![pts[from]];
        let mut i = from;
        loop {
            i = (i + 1) % n;
            run.push(pts[i]);
            if i == to {
                break;
            }
        }
        let fitted = fit_polyline(&run, false, crate::import::FIT_TOLERANCE_MM);
        segments.extend(fitted.iter().map(|f| match f {
            r3d_geom::fit::Fitted::Line(_, b) => r3d_core::PathSegment::Line { to: *b },
            r3d_geom::fit::Fitted::Cubic(c) => r3d_core::PathSegment::Cubic {
                c1: c.p1,
                c2: c.p2,
                to: c.p3,
            },
        }));
    }
    Some(Path {
        start: pts[first],
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AAMA: &str = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n1\n0\nENDSEC\n\
0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n8\n0\n2\nFRONT\n70\n0\n10\n0\n20\n0\n\
0\nPOLYLINE\n8\n1\n66\n1\n70\n1\n\
0\nVERTEX\n8\n1\n10\n0\n20\n0\n0\nVERTEX\n8\n1\n10\n10\n20\n0\n\
0\nVERTEX\n8\n1\n10\n10\n20\n12\n0\nVERTEX\n8\n1\n10\n0\n20\n12\n0\nSEQEND\n8\n1\n\
0\nPOINT\n8\n2\n10\n0\n20\n0\n0\nPOINT\n8\n2\n10\n10\n20\n0\n\
0\nPOINT\n8\n2\n10\n10\n20\n12\n0\nPOINT\n8\n2\n10\n0\n20\n12\n\
0\nPOINT\n8\n4\n10\n10\n20\n6\n\
0\nLINE\n8\n7\n10\n5\n20\n2\n11\n5\n21\n10\n\
0\nTEXT\n8\n1\n10\n2\n20\n5\n40\n0.2\n1\nPiece Name: Front\n\
0\nENDBLK\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n0\nINSERT\n8\n1\n2\nFRONT\n10\n0\n20\n0\n0\nENDSEC\n0\nEOF\n";

    #[test]
    fn an_aama_piece_in_inches() {
        let out = read(AAMA.as_bytes(), &ImportOptions::default()).unwrap();
        assert_eq!(out.scale, ScaleSource::File);
        assert_eq!(out.mm_per_unit, 25.4);
        let p = &out.pieces[0];
        assert_eq!(p.name.as_deref(), Some("Front"));
        assert_eq!(p.outline.segments.len(), 4);
        assert_eq!(p.notches.len(), 1);
        assert!(p.notches[0].distance(DVec2::new(254.0, 152.4)) < 1e-9);
        assert_eq!(crate::import::grain_angle_deg(p.grain.unwrap()), 90.0);
    }

    #[test]
    fn binary_and_garbage() {
        assert_eq!(
            read(b"AutoCAD Binary DXF\r\n\x1a\0", &ImportOptions::default()),
            Err(ImportError::BinaryDxf)
        );
        assert!(matches!(
            read(b"hello\nworld\n", &ImportOptions::default()),
            Err(ImportError::Malformed { .. })
        ));
    }

    #[test]
    fn a_plain_drawing_without_units() {
        let dxf = "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\nOUTLINE\n70\n1\n\
10\n0\n20\n0\n10\n100\n20\n0\n42\n0.4142135\n10\n100\n20\n100\n10\n0\n20\n100\n0\nENDSEC\n0\nEOF\n";
        let out = read(dxf.as_bytes(), &ImportOptions::default()).unwrap();
        assert_eq!(out.scale, ScaleSource::Assumed);
        assert_eq!(out.pieces.len(), 1);
        // The bulge (a quarter circle) became one curve.
        let curves = out.pieces[0]
            .outline
            .segments
            .iter()
            .filter(|s| matches!(s, r3d_core::PathSegment::Cubic { .. }))
            .count();
        assert!(curves >= 1);
    }

    #[test]
    fn thinning_keeps_the_shape() {
        let pts: Vec<DVec2> = (0..=1000)
            .map(|i| DVec2::from_angle(f64::from(i) * 0.09f64.to_radians()) * 100.0)
            .collect();
        let t = thin(&pts);
        assert!(t.len() < 100);
        assert_eq!(t.first(), pts.first());
        assert_eq!(t.last(), pts.last());
        assert!(
            pts.iter()
                .all(|p| distance_to_polyline(*p, &t, false) <= EXPORT_TOLERANCE_MM)
        );
    }
}
