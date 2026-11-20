//! Pattern import from open formats: DXF-AAMA ([`crate::dxf`]), SVG
//! ([`svg`]) and PDF ([`pdf`]). Every reader produces the same
//! [`ImportedPattern`]: pieces in board millimetres (y up) with their
//! outline, seam allowance, notches, internal lines, grainline and name.
//! Outlines drawn as polylines are turned back into straight edges and
//! curves ([`r3d_geom::fit`]), so imported pieces are as easy to edit as
//! drawn ones.

pub mod pdf;
pub mod svg;
pub mod val;
pub(crate) mod vector;

use r3d_core::{Localizable, Message, PathSegment};
use r3d_geom::fit::{Fitted, fit_polyline};
use r3d_geom::{Aabb, CubicBez, DVec2, polygon};

/// Max distance between an imported outline and the points it was read
/// from, once fitted with curves (mm).
pub const FIT_TOLERANCE_MM: f64 = 0.1;
/// Flattening tolerance used to test imported shapes (mm).
const FLATTEN_MM: f64 = 0.02;
/// Spacing of the samples that measure a seam allowance (mm).
const SA_SAMPLE_MM: f64 = 5.0;
/// Share of those samples that must agree on the allowance.
const SA_EVEN_SHARE: f64 = 0.8;
/// Gap between pieces laid out side by side (mm).
const LAYOUT_GAP_MM: f64 = 50.0;

/// A path in board millimetres: a start point and segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub start: DVec2,
    pub segments: Vec<PathSegment>,
}

impl Path {
    pub fn end(&self) -> DVec2 {
        self.segments.last().map_or(self.start, PathSegment::end)
    }

    /// The path as a polyline (curves flattened).
    pub fn polyline(&self) -> Vec<DVec2> {
        let mut out = vec![self.start];
        let mut at = self.start;
        for s in &self.segments {
            match s {
                PathSegment::Line { to } => out.push(*to),
                PathSegment::Cubic { c1, c2, to } => {
                    CubicBez::new(at, *c1, *c2, *to).flatten_into(FLATTEN_MM, &mut out);
                }
            }
            at = s.end();
        }
        out
    }

    /// The same path moved by `d`.
    pub fn translated(&self, d: DVec2) -> Path {
        self.mapped(|p| p + d)
    }

    pub(crate) fn mapped(&self, f: impl Fn(DVec2) -> DVec2) -> Path {
        Path {
            start: f(self.start),
            segments: self
                .segments
                .iter()
                .map(|s| match s {
                    PathSegment::Line { to } => PathSegment::Line { to: f(*to) },
                    PathSegment::Cubic { c1, c2, to } => PathSegment::Cubic {
                        c1: f(*c1),
                        c2: f(*c2),
                        to: f(*to),
                    },
                })
                .collect(),
        }
    }

    /// A polyline as a path of straight segments.
    pub(crate) fn from_polyline(points: &[DVec2]) -> Option<Path> {
        let (first, rest) = points.split_first()?;
        Some(Path {
            start: *first,
            segments: rest.iter().map(|p| PathSegment::Line { to: *p }).collect(),
        })
    }
}

/// One imported piece.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedPiece {
    pub name: Option<String>,
    /// The sewing line, closed: the last segment ends at `start`.
    pub outline: Path,
    pub seam_allowance_mm: f64,
    /// Grainline, from one end to the other.
    pub grain: Option<(DVec2, DVec2)>,
    /// Notch positions, on or near the outline.
    pub notches: Vec<DVec2>,
    pub internal_lines: Vec<Path>,
}

impl ImportedPiece {
    fn bounds(&self) -> Option<Aabb> {
        polygon::bounding_box(&self.outline.polyline())
    }

    fn translate(&mut self, d: DVec2) {
        self.outline = self.outline.translated(d);
        self.grain = self.grain.map(|(a, b)| (a + d, b + d));
        for n in &mut self.notches {
            *n += d;
        }
        for l in &mut self.internal_lines {
            *l = l.translated(d);
        }
    }
}

/// Where the size of a drawing unit came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleSource {
    /// Stated in the file (DXF units, SVG size in mm, PDF points).
    File,
    /// Given by the caller.
    Given,
    /// Not stated: a default was used (millimetres for DXF, CSS pixels
    /// for SVG). The result says so, to check the size.
    Assumed,
}

impl ScaleSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ScaleSource::File => "file",
            ScaleSource::Given => "given",
            ScaleSource::Assumed => "assumed",
        }
    }
}

/// What a reader found.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedPattern {
    pub pieces: Vec<ImportedPiece>,
    /// Shapes that are not part of any piece, or that could not be read
    /// as one (open outlines, crossing outlines, unsupported entities).
    pub ignored: usize,
    /// Millimetres per drawing unit.
    pub mm_per_unit: f64,
    pub scale: ScaleSource,
}

/// How to read a file.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImportOptions {
    /// Millimetres per drawing unit, overriding what the file says.
    pub mm_per_unit: Option<f64>,
    /// PDF only: import this page (1-based) instead of the whole document.
    pub page: Option<usize>,
    /// Size factor on top of the units (2 for a pattern drawn at half
    /// size); 1 when `None`.
    pub scale: Option<f64>,
}

impl ImportOptions {
    /// Millimetres per unit: given, else what the file says, times the
    /// size factor.
    pub(crate) fn mm_per_unit(&self, from_file: (f64, ScaleSource)) -> (f64, ScaleSource) {
        let (mm, source) = match self.mm_per_unit {
            Some(mm) => (mm, ScaleSource::Given),
            None => from_file,
        };
        (mm * self.scale.unwrap_or(1.0), source)
    }
}

/// Why a file could not be imported.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ImportError {
    #[error("this {format} file cannot be read: {detail}")]
    Malformed {
        format: &'static str,
        detail: String,
    },
    #[error("binary DXF is not supported; save the file as ASCII DXF")]
    BinaryDxf,
    #[error("no pattern piece found: no closed outline")]
    NoPieces,
    #[error(
        "the PDF pages do not repeat one drawing, so they cannot be put back together; import one page with `page`"
    )]
    PdfPagesDiffer { pages: usize },
    #[error("page {page} does not exist (the document has {pages})")]
    NoSuchPage { page: usize, pages: usize },
    #[error("the PDF is encrypted")]
    Encrypted,
    #[error(transparent)]
    Val(#[from] val::ValError),
}

impl Localizable for ImportError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Malformed { format, detail } => Message::new("import_malformed", text)
                .arg("format", format)
                .arg("detail", detail),
            Self::BinaryDxf => Message::new("import_binary_dxf", text),
            Self::NoPieces => Message::new("import_no_pieces", text),
            Self::PdfPagesDiffer { pages } => {
                Message::new("import_pdf_pages_differ", text).arg("pages", pages)
            }
            Self::NoSuchPage { page, pages } => Message::new("import_no_such_page", text)
                .arg("page", page)
                .arg("pages", pages),
            Self::Encrypted => Message::new("import_encrypted", text),
            Self::Val(e) => e.message(),
        }
    }
}

/// Fit straight edges and curves to the runs of straight segments of a
/// path (a polyline from another program), keeping its curves.
pub(crate) fn simplify(path: &Path, closed: bool) -> Path {
    let all_lines = path
        .segments
        .iter()
        .all(|s| matches!(s, PathSegment::Line { .. }));
    if all_lines {
        let mut pts = vec![path.start];
        pts.extend(path.segments.iter().map(PathSegment::end));
        return from_fitted(
            &fit_polyline(&pts, closed, FIT_TOLERANCE_MM),
            path.start,
            closed,
        )
        .unwrap_or_else(|| path.clone());
    }
    // Mixed: fit each run of three or more straight segments on its own.
    let mut out = Path {
        start: path.start,
        segments: Vec::new(),
    };
    let mut run = vec![path.start];
    let flush = |run: &mut Vec<DVec2>, out: &mut Path| {
        if run.len() >= 4 {
            for f in fit_polyline(run, false, FIT_TOLERANCE_MM) {
                out.segments.push(segment(&f));
            }
        } else {
            out.segments
                .extend(run.iter().skip(1).map(|p| PathSegment::Line { to: *p }));
        }
        let last = run.last().copied();
        run.clear();
        run.extend(last);
    };
    for s in &path.segments {
        match s {
            PathSegment::Line { to } => run.push(*to),
            cubic => {
                flush(&mut run, &mut out);
                out.segments.push(cubic.clone());
                run.clear();
                run.push(cubic.end());
            }
        }
    }
    flush(&mut run, &mut out);
    out
}

fn segment(f: &Fitted) -> PathSegment {
    match f {
        Fitted::Line(_, b) => PathSegment::Line { to: *b },
        Fitted::Cubic(c) => PathSegment::Cubic {
            c1: c.p1,
            c2: c.p2,
            to: c.p3,
        },
    }
}

/// Fitted pieces as a path; a closed one gets its closing segment.
fn from_fitted(fitted: &[Fitted], fallback_start: DVec2, closed: bool) -> Option<Path> {
    let first = fitted.first()?;
    let mut path = Path {
        start: first.start(),
        segments: fitted.iter().map(segment).collect(),
    };
    if closed && path.end().distance(path.start) > 1e-9 {
        path.segments.push(PathSegment::Line { to: path.start });
    }
    if !closed && path.start.distance(fallback_start) > 1e-9 {
        return None;
    }
    Some(path)
}

/// Close a path: make the last segment end exactly at the start (adding a
/// straight one if it ends elsewhere).
pub(crate) fn closed(mut path: Path) -> Path {
    let start = path.start;
    let gap = path.end().distance(start);
    if gap > 1e-6 {
        path.segments.push(PathSegment::Line { to: start });
    } else if let Some(last) = path.segments.last_mut() {
        match last {
            PathSegment::Line { to } | PathSegment::Cubic { to, .. } => *to = start,
        }
    }
    path
}

/// Last checks before handing pieces over: outlines must be simple (the
/// fitted one, else the raw one, else the piece is dropped); pieces that
/// overlap are laid out side by side; the whole set starts at the origin.
pub(crate) fn finish(
    raw: Vec<(ImportedPiece, Path)>,
    mut ignored: usize,
    mm_per_unit: f64,
    scale: ScaleSource,
) -> Result<ImportedPattern, ImportError> {
    let mut pieces = Vec::new();
    for (mut piece, raw_outline) in raw {
        if is_simple(&piece.outline) {
            pieces.push(piece);
        } else if is_simple(&raw_outline) {
            piece.outline = raw_outline;
            pieces.push(piece);
        } else {
            ignored += 1;
        }
    }
    if pieces.is_empty() {
        return Err(ImportError::NoPieces);
    }
    lay_out(&mut pieces);
    Ok(ImportedPattern {
        pieces,
        ignored,
        mm_per_unit,
        scale,
    })
}

pub(crate) fn is_simple(outline: &Path) -> bool {
    let mut pts = outline.polyline();
    if pts.len() > 1 && pts[0].distance(pts[pts.len() - 1]) < 1e-6 {
        pts.pop();
    }
    pts.len() >= 3 && polygon::is_simple(&pts)
}

/// Side by side, in reading order, if any two pieces overlap (DXF-AAMA
/// pieces are often all drawn at the origin); then move the set so its
/// bounding box starts at the origin.
fn lay_out(pieces: &mut [ImportedPiece]) {
    let boxes: Vec<Option<Aabb>> = pieces.iter().map(ImportedPiece::bounds).collect();
    let overlap = |a: &Aabb, b: &Aabb| {
        a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
    };
    let crowded = boxes.iter().enumerate().any(|(i, a)| {
        boxes.iter().skip(i + 1).any(|b| match (a, b) {
            (Some(a), Some(b)) => overlap(a, b),
            _ => false,
        })
    });
    if crowded {
        let mut x = 0.0;
        for (piece, bb) in pieces.iter_mut().zip(&boxes) {
            if let Some(bb) = bb {
                piece.translate(DVec2::new(x - bb.min.x, -bb.min.y));
                x += bb.max.x - bb.min.x + LAYOUT_GAP_MM;
            }
        }
    }
    let all: Vec<DVec2> = pieces.iter().flat_map(|p| p.outline.polyline()).collect();
    if let Some(bb) = polygon::bounding_box(&all) {
        for piece in pieces.iter_mut() {
            piece.translate(-bb.min);
        }
    }
}

/// Distance from `p` to a polyline.
pub(crate) fn distance_to_polyline(p: DVec2, line: &[DVec2], closed: bool) -> f64 {
    let n = line.len();
    let segs = if closed { n } else { n.saturating_sub(1) };
    (0..segs)
        .map(|i| r3d_geom::fit::distance_to_segment(p, line[i], line[(i + 1) % n]))
        .fold(f64::INFINITY, f64::min)
}

/// The point of a closed polyline nearest to `p`.
pub(crate) fn nearest_on(p: DVec2, line: &[DVec2]) -> DVec2 {
    let n = line.len();
    let mut best = (f64::INFINITY, p);
    for i in 0..n {
        let (a, b) = (line[i], line[(i + 1) % n]);
        let ab = b - a;
        let t = if ab.length_squared() > 0.0 {
            ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = a + ab * t;
        let d = q.distance(p);
        if d < best.0 {
            best = (d, q);
        }
    }
    best.1
}

/// The seam allowance when `inner` is the sewing line of the cutting
/// line `outer`: the distance between them, if it is the same along most
/// of the line (corners stand further off) and at most 50 mm.
pub(crate) fn seam_allowance(outer: &[DVec2], inner: &[DVec2]) -> Option<f64> {
    if inner.len() < 3 || !inner.iter().all(|p| polygon::contains(outer, *p)) {
        return None;
    }
    // Sample the sewing line every few millimetres.
    let n = inner.len();
    let mut d: Vec<f64> = Vec::new();
    for i in 0..n {
        let (a, b) = (inner[i], inner[(i + 1) % n]);
        let k = (a.distance(b) / SA_SAMPLE_MM).ceil().max(1.0) as usize;
        for j in 0..k {
            d.push(distance_to_polyline(
                a.lerp(b, j as f64 / k as f64),
                outer,
                true,
            ));
        }
    }
    let mut sorted = d.clone();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    let tol = 0.5_f64.max(0.05 * median);
    let even = d.iter().filter(|x| (**x - median).abs() <= tol).count();
    (median > 0.1 && median <= 50.0 && even as f64 >= SA_EVEN_SHARE * d.len() as f64)
        .then(|| (median * 10.0).round() / 10.0)
}

/// Grainline angle in degrees (0 ≤ a < 180), as the document stores it.
pub fn grain_angle_deg((a, b): (DVec2, DVec2)) -> f64 {
    let deg = (b - a).to_angle().to_degrees().rem_euclid(180.0);
    // Round off float noise (89.9999999 → 90).
    let r = (deg * 1e6).round() / 1e6;
    if r >= 180.0 { 0.0 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(at: DVec2, side: f64) -> Path {
        let p = |x: f64, y: f64| at + DVec2::new(x, y);
        Path::from_polyline(&[
            p(0.0, 0.0),
            p(side, 0.0),
            p(side, side),
            p(0.0, side),
            p(0.0, 0.0),
        ])
        .unwrap()
    }

    fn piece(outline: Path) -> (ImportedPiece, Path) {
        (
            ImportedPiece {
                name: None,
                outline: outline.clone(),
                seam_allowance_mm: 0.0,
                grain: None,
                notches: vec![],
                internal_lines: vec![],
            },
            outline,
        )
    }

    #[test]
    fn overlapping_pieces_are_laid_out() {
        let out = finish(
            vec![
                piece(square(DVec2::ZERO, 100.0)),
                piece(square(DVec2::splat(10.0), 50.0)),
            ],
            0,
            1.0,
            ScaleSource::File,
        )
        .unwrap();
        let b0 = out.pieces[0].bounds().unwrap();
        let b1 = out.pieces[1].bounds().unwrap();
        assert_eq!(b0.min, DVec2::ZERO);
        assert!((b1.min.x - 150.0).abs() < 1e-9);
    }

    #[test]
    fn a_crossed_outline_is_dropped() {
        let bow = Path::from_polyline(&[
            DVec2::ZERO,
            DVec2::new(10.0, 10.0),
            DVec2::new(10.0, 0.0),
            DVec2::new(0.0, 10.0),
            DVec2::ZERO,
        ])
        .unwrap();
        assert_eq!(
            finish(vec![piece(bow)], 0, 1.0, ScaleSource::File),
            Err(ImportError::NoPieces)
        );
    }

    #[test]
    fn seam_allowance_of_an_offset_outline() {
        let inner = square(DVec2::ZERO, 100.0).polyline();
        let outer = polygon::offset_outward(&inner[..4], 15.0);
        assert_eq!(seam_allowance(&outer, &inner[..4]), Some(15.0));
        assert_eq!(seam_allowance(&inner[..4], &outer), None);
    }

    #[test]
    fn grain_angles() {
        assert_eq!(grain_angle_deg((DVec2::ZERO, DVec2::new(0.0, -5.0))), 90.0);
        assert_eq!(grain_angle_deg((DVec2::ZERO, DVec2::new(-5.0, 0.0))), 0.0);
        assert!((grain_angle_deg((DVec2::ZERO, DVec2::new(1.0, 1.0))) - 45.0).abs() < 1e-9);
    }

    #[test]
    fn a_flattened_curve_comes_back_as_curves() {
        let c = CubicBez::new(
            DVec2::ZERO,
            DVec2::new(0.0, 100.0),
            DVec2::new(200.0, 100.0),
            DVec2::new(200.0, 0.0),
        );
        let mut pts = vec![c.p0];
        c.flatten_into(0.01, &mut pts);
        let s = simplify(&Path::from_polyline(&pts).unwrap(), false);
        assert!(s.segments.len() <= 2, "{}", s.segments.len());
        assert_eq!(s.end(), c.p3);
    }
}
