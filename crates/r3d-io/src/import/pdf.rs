//! PDF import (ISO 32000): the stroked and filled paths and the text of
//! the pages, read as pattern pieces by [`super::vector`]. A PDF pattern
//! is drawn at 1:1, so a unit is a point (1/72 inch, times the page's
//! `UserUnit`).
//!
//! **Tiled patterns.** A pattern printed on several pages usually draws
//! the whole pattern on every page, shifted so that the page shows its
//! tile (that is how Rustle3D exports it). The pages are put back together
//! by finding, for each page, the shift that lays its shapes onto the
//! first page's; what does not move with the pattern (page frames,
//! assembly guides, page labels) is dropped. Pages that do not repeat one
//! drawing cannot be put back together this way: import them one at a
//! time with [`ImportOptions::page`].

use std::collections::HashMap;

use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object};
use r3d_core::PathSegment;
use r3d_geom::DVec2;

use super::vector::{Drawing, Shape, Text};
use super::{ImportError, ImportOptions, ImportedPattern, Path, ScaleSource, finish};

const PT_MM: f64 = 25.4 / 72.0;
/// Form XObjects nested deeper than this are not drawn.
const MAX_FORM_DEPTH: usize = 8;
/// Shapes match across pages when their points agree within this (mm).
const MATCH_MM: f64 = 0.05;
/// Pages repeat one drawing when at least this share of their shapes
/// match the first page's.
const REPEAT_SHARE: f64 = 0.5;

fn malformed(detail: impl ToString) -> ImportError {
    ImportError::Malformed {
        format: "PDF",
        detail: detail.to_string(),
    }
}

/// Read the pattern pieces of a PDF document.
pub fn read(bytes: &[u8], options: &ImportOptions) -> Result<ImportedPattern, ImportError> {
    let doc = Document::load_mem(bytes).map_err(|e| match e {
        lopdf::Error::Decryption(_) | lopdf::Error::InvalidPassword => ImportError::Encrypted,
        other => malformed(other),
    })?;
    if doc.is_encrypted() {
        return Err(ImportError::Encrypted);
    }
    let pages: Vec<lopdf::ObjectId> = doc.get_pages().into_values().collect();
    let selected: Vec<lopdf::ObjectId> = match options.page {
        Some(page) => {
            let id =
                page.checked_sub(1)
                    .and_then(|i| pages.get(i))
                    .ok_or(ImportError::NoSuchPage {
                        page,
                        pages: pages.len(),
                    })?;
            vec![*id]
        }
        None => pages,
    };
    let (mm_per_unit, scale) = options.mm_per_unit((PT_MM, ScaleSource::File));
    let mut drawings = Vec::with_capacity(selected.len());
    for id in &selected {
        drawings.push(page_drawing(&doc, *id, mm_per_unit)?);
    }
    let drawing = match drawings.len() {
        0 => return Err(ImportError::NoPieces),
        1 => drawings.swap_remove(0),
        n => merge(&drawings).ok_or(ImportError::PdfPagesDiffer { pages: n })?,
    };
    let (pieces, ignored) = super::vector::pieces(&drawing);
    finish(pieces, ignored, mm_per_unit, scale)
}

/// An affine map `[a b c d e f]` as PDF writes it: `x' = a x + c y + e`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Matrix([f64; 6]);

impl Matrix {
    const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn apply(&self, p: DVec2) -> DVec2 {
        let [a, b, c, d, e, f] = self.0;
        DVec2::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }

    /// `inner` first, then `self`.
    fn after(&self, inner: &Matrix) -> Matrix {
        let [a, b, c, d, e, f] = inner.0;
        let o = self.0;
        Matrix([
            a * o[0] + b * o[2],
            a * o[1] + b * o[3],
            c * o[0] + d * o[2],
            c * o[1] + d * o[3],
            e * o[0] + f * o[2] + o[4],
            e * o[1] + f * o[3] + o[5],
        ])
    }

    fn scale(&self) -> f64 {
        let [a, b, c, d, _, _] = self.0;
        (a * d - b * c).abs().sqrt()
    }

    fn from_operands(ops: &[Object]) -> Option<Matrix> {
        let v: Vec<f64> = ops.iter().filter_map(num).collect();
        let m: [f64; 6] = v.try_into().ok()?;
        Some(Matrix(m))
    }
}

fn num(o: &Object) -> Option<f64> {
    o.as_float().ok().map(f64::from).filter(|v| v.is_finite())
}

#[derive(Clone, Copy)]
struct State {
    ctm: Matrix,
    dashed: bool,
}

/// Everything drawn on one page, in board millimetres.
fn page_drawing(doc: &Document, page: lopdf::ObjectId, mm: f64) -> Result<Drawing, ImportError> {
    let page_dict = doc.get_dictionary(page).map_err(malformed)?;
    let user_unit = page_dict
        .get(b"UserUnit")
        .ok()
        .and_then(num)
        .filter(|u| *u > 0.0)
        .unwrap_or(1.0);
    let base = Matrix([mm * user_unit, 0.0, 0.0, mm * user_unit, 0.0, 0.0]);
    let content = doc.get_page_content(page);
    let resources = page_resources(doc, page);
    let mut out = Drawing {
        page_area: media_box_area(doc, page_dict).map(|a| a * (mm * user_unit).powi(2)),
        ..Drawing::default()
    };
    run(doc, &content, resources.as_ref(), base, &mut out, 0)?;
    Ok(out)
}

fn media_box_area(doc: &Document, page: &Dictionary) -> Option<f64> {
    let b = page.get_deref(b"MediaBox", doc).ok()?.as_array().ok()?;
    let v: Vec<f64> = b.iter().filter_map(num).collect();
    let [x0, y0, x1, y1] = v[..] else {
        return None;
    };
    Some(((x1 - x0) * (y1 - y0)).abs())
}

fn page_resources(doc: &Document, page: lopdf::ObjectId) -> Option<Dictionary> {
    let (direct, ids) = doc.get_page_resources(page).ok()?;
    if let Some(d) = direct {
        return Some(d.clone());
    }
    ids.iter()
        .find_map(|id| doc.get_dictionary(*id).ok())
        .cloned()
}

/// Run a content stream, appending what it paints to `out`.
fn run(
    doc: &Document,
    content: &[u8],
    resources: Option<&Dictionary>,
    base: Matrix,
    out: &mut Drawing,
    depth: usize,
) -> Result<(), ImportError> {
    let ops = Content::decode(content).map_err(malformed)?.operations;
    let mut state = State {
        ctm: base,
        dashed: false,
    };
    let mut stack: Vec<State> = Vec::new();
    let mut subpaths: Vec<(Path, bool)> = Vec::new();
    let mut current: Option<Path> = None;
    // Text: matrix, line matrix, font size, leading, and the text of the
    // current BT … ET block with where it starts.
    let (mut tm, mut tlm) = (Matrix::IDENTITY, Matrix::IDENTITY);
    let (mut font_size, mut leading) = (12.0, 0.0);
    let mut shown: Option<(DVec2, f64, String)> = None;

    let pt = |ops: &[Object], i: usize| -> Option<DVec2> {
        Some(DVec2::new(num(ops.get(i)?)?, num(ops.get(i + 1)?)?))
    };
    for op in &ops {
        let o = &op.operands;
        match op.operator.as_str() {
            "q" => stack.push(state),
            "Q" => state = stack.pop().unwrap_or(state),
            "cm" => {
                if let Some(m) = Matrix::from_operands(o) {
                    state.ctm = state.ctm.after(&m);
                }
            }
            "d" => {
                state.dashed = o
                    .first()
                    .and_then(|a| a.as_array().ok())
                    .is_some_and(|a| a.iter().filter_map(num).any(|v| v > 0.0));
            }
            "m" => {
                if let Some(p) = pt(o, 0) {
                    if let Some(path) = current.take() {
                        subpaths.push((path, false));
                    }
                    current = Some(Path {
                        start: state.ctm.apply(p),
                        segments: Vec::new(),
                    });
                }
            }
            "l" => {
                if let (Some(p), Some(path)) = (pt(o, 0), current.as_mut()) {
                    path.segments.push(PathSegment::Line {
                        to: state.ctm.apply(p),
                    });
                }
            }
            "c" | "v" | "y" => {
                let Some(path) = current.as_mut() else {
                    continue;
                };
                let here = path.end();
                let m = state.ctm;
                let seg = match (op.operator.as_str(), pt(o, 0), pt(o, 2), pt(o, 4)) {
                    ("c", Some(a), Some(b), Some(p)) => Some((m.apply(a), m.apply(b), m.apply(p))),
                    ("v", Some(b), Some(p), _) => Some((here, m.apply(b), m.apply(p))),
                    ("y", Some(a), Some(p), _) => Some((m.apply(a), m.apply(p), m.apply(p))),
                    _ => None,
                };
                if let Some((c1, c2, to)) = seg {
                    path.segments.push(PathSegment::Cubic { c1, c2, to });
                }
            }
            "h" => {
                if let Some(path) = current.take() {
                    let start = path.start;
                    subpaths.push((path, true));
                    current = Some(Path {
                        start,
                        segments: Vec::new(),
                    });
                }
            }
            "re" => {
                if let (Some(p), Some(s)) = (pt(o, 0), pt(o, 2)) {
                    let m = state.ctm;
                    let corners = [
                        p,
                        p + DVec2::new(s.x, 0.0),
                        p + s,
                        p + DVec2::new(0.0, s.y),
                        p,
                    ];
                    let pts: Vec<DVec2> = corners.iter().map(|c| m.apply(*c)).collect();
                    if let Some(path) = Path::from_polyline(&pts) {
                        subpaths.push((path, true));
                    }
                }
            }
            "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                let close = matches!(op.operator.as_str(), "s" | "b" | "b*");
                if let Some(path) = current.take() {
                    subpaths.push((path, close));
                }
                if op.operator != "n" {
                    for (path, closed) in subpaths.drain(..) {
                        if !path.segments.is_empty() {
                            out.shapes.push(Shape {
                                path,
                                closed,
                                dashed: state.dashed,
                            });
                        }
                    }
                }
                subpaths.clear();
            }
            "BT" => {
                tm = Matrix::IDENTITY;
                tlm = Matrix::IDENTITY;
                shown = None;
            }
            "ET" => {
                if let Some((pos, size, text)) = shown.take() {
                    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !text.is_empty() {
                        out.texts.push(Text {
                            pos,
                            text,
                            size_mm: size,
                        });
                    }
                }
            }
            "Tf" => {
                if let Some(s) = o.get(1).and_then(num) {
                    font_size = s;
                }
            }
            "TL" => leading = o.first().and_then(num).unwrap_or(leading),
            "Td" | "TD" => {
                if let Some(t) = pt(o, 0) {
                    if op.operator == "TD" {
                        leading = -t.y;
                    }
                    tlm = tlm.after(&Matrix([1.0, 0.0, 0.0, 1.0, t.x, t.y]));
                    tm = tlm;
                }
            }
            "Tm" => {
                if let Some(m) = Matrix::from_operands(o) {
                    tlm = m;
                    tm = m;
                }
            }
            "T*" => {
                tlm = tlm.after(&Matrix([1.0, 0.0, 0.0, 1.0, 0.0, -leading]));
                tm = tlm;
            }
            "Tj" | "'" | "\"" | "TJ" => {
                let strings: Vec<&Object> = match op.operator.as_str() {
                    "TJ" => o
                        .first()
                        .and_then(|a| a.as_array().ok())
                        .map(|a| a.iter().collect())
                        .unwrap_or_default(),
                    _ => o.last().into_iter().collect(),
                };
                let mut text = String::new();
                for s in strings {
                    match s {
                        Object::String(bytes, _) => match latin1(bytes) {
                            Some(t) => text.push_str(&t),
                            None => {
                                text.clear();
                                break;
                            }
                        },
                        // A wide negative adjustment is a word space.
                        other => {
                            if num(other).is_some_and(|v| v < -200.0) {
                                text.push(' ');
                            }
                        }
                    }
                }
                let m = state.ctm.after(&tm);
                let at = m.apply(DVec2::ZERO);
                let size = font_size * m.scale();
                match shown.as_mut() {
                    Some((_, _, t)) => t.push_str(&text),
                    None => shown = Some((at, size, text)),
                }
            }
            "Do" => {
                if depth >= MAX_FORM_DEPTH {
                    continue;
                }
                let Some(name) = o.first().and_then(|n| n.as_name().ok()) else {
                    continue;
                };
                let Some(form) = resources
                    .and_then(|r| r.get_deref(b"XObject", doc).ok())
                    .and_then(|x| x.as_dict().ok())
                    .and_then(|x| x.get_deref(name, doc).ok())
                    .and_then(|s| s.as_stream().ok())
                else {
                    continue;
                };
                let is_form = form
                    .dict
                    .get(b"Subtype")
                    .and_then(Object::as_name)
                    .is_ok_and(|s| s == b"Form");
                if !is_form {
                    continue;
                }
                let matrix = form
                    .dict
                    .get(b"Matrix")
                    .ok()
                    .and_then(|m| m.as_array().ok())
                    .and_then(|m| Matrix::from_operands(m))
                    .unwrap_or(Matrix::IDENTITY);
                let own = form
                    .dict
                    .get_deref(b"Resources", doc)
                    .ok()
                    .and_then(|r| r.as_dict().ok())
                    .cloned();
                let content = form
                    .decompressed_content()
                    .unwrap_or_else(|_| form.content.clone());
                let inner = state.ctm.after(&matrix);
                run(
                    doc,
                    &content,
                    own.as_ref().or(resources),
                    inner,
                    out,
                    depth + 1,
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Text in a simple font, read as Latin-1 (WinAnsi agrees on letters);
/// `None` for two-byte encodings, which need the font's own tables.
fn latin1(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    Some(bytes.iter().map(|&b| char::from(b)).collect())
}

/// A shape's outline relative to its first point, rounded: equal for the
/// same shape drawn anywhere.
fn signature(shape: &Shape) -> Vec<i64> {
    let s = shape.path.start;
    let mut sig = vec![i64::from(shape.closed)];
    let round = |p: DVec2| {
        [
            ((p.x - s.x) / MATCH_MM).round() as i64,
            ((p.y - s.y) / MATCH_MM).round() as i64,
        ]
    };
    for seg in &shape.path.segments {
        match seg {
            PathSegment::Line { to } => sig.extend(round(*to)),
            PathSegment::Cubic { c1, c2, to } => {
                sig.push(-1);
                sig.extend(round(*c1));
                sig.extend(round(*c2));
                sig.extend(round(*to));
            }
        }
    }
    sig
}

/// The pages put back together: the first page's shapes and texts that
/// every other page repeats, shifted.
fn merge(pages: &[Drawing]) -> Option<Drawing> {
    let (first, rest) = pages.split_first()?;
    let mut by_sig: HashMap<Vec<i64>, Vec<usize>> = HashMap::new();
    for (i, s) in first.shapes.iter().enumerate() {
        by_sig.entry(signature(s)).or_default().push(i);
    }
    let key = |d: DVec2| ((d.x / 0.1).round() as i64, (d.y / 0.1).round() as i64);
    let mut keep = vec![true; first.shapes.len()];
    let mut keep_text = vec![true; first.texts.len()];
    for page in rest {
        // Vote for the shift that lays this page onto the first.
        let mut votes: HashMap<(i64, i64), (usize, DVec2)> = HashMap::new();
        let sigs: Vec<Vec<i64>> = page.shapes.iter().map(signature).collect();
        for (s, sig) in page.shapes.iter().zip(&sigs) {
            for &i in by_sig.get(sig).into_iter().flatten().take(32) {
                let d = first.shapes[i].path.start - s.path.start;
                let v = votes.entry(key(d)).or_insert((0, d));
                v.0 += 1;
            }
        }
        let (_, (_, shift)) = votes.into_iter().max_by_key(|(_, (n, _))| *n)?;
        let mut matched = vec![false; first.shapes.len()];
        for (s, sig) in page.shapes.iter().zip(&sigs) {
            let at = s.path.start + shift;
            if let Some(&i) = by_sig
                .get(sig)
                .into_iter()
                .flatten()
                .find(|&&i| first.shapes[i].path.start.distance(at) < 2.0 * MATCH_MM)
            {
                matched[i] = true;
            }
        }
        let count = matched.iter().filter(|m| **m).count();
        let smaller = first.shapes.len().min(page.shapes.len());
        if smaller == 0 || (count as f64) < REPEAT_SHARE * smaller as f64 {
            return None;
        }
        for (k, m) in keep.iter_mut().zip(&matched) {
            *k &= *m;
        }
        for (k, t) in keep_text.iter_mut().zip(&first.texts) {
            *k &= page
                .texts
                .iter()
                .any(|u| u.text == t.text && (u.pos + shift).distance(t.pos) < 0.5);
        }
    }
    Some(Drawing {
        shapes: first
            .shapes
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(s, _)| s.clone())
            .collect(),
        texts: first
            .texts
            .iter()
            .zip(&keep_text)
            .filter(|(_, k)| **k)
            .map(|(t, _)| t.clone())
            .collect(),
        page_area: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices_compose_like_pdf() {
        // cm 2 0 0 2 10 0 then cm 1 0 0 1 5 5: a point (1, 1) goes to
        // ((1 + 5) * 2 + 10, (1 + 5) * 2).
        let ctm = Matrix::IDENTITY
            .after(&Matrix([2.0, 0.0, 0.0, 2.0, 10.0, 0.0]))
            .after(&Matrix([1.0, 0.0, 0.0, 1.0, 5.0, 5.0]));
        assert_eq!(ctm.apply(DVec2::new(1.0, 1.0)), DVec2::new(22.0, 12.0));
    }

    #[test]
    fn not_a_pdf() {
        assert!(matches!(
            read(b"hello", &ImportOptions::default()),
            Err(ImportError::Malformed { .. })
        ));
    }
}
