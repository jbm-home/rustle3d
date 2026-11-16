//! SVG import (W3C SVG 1.1): the shapes of the drawing, with their
//! transforms, read as pattern pieces by [`super::vector`].
//!
//! Scale: the document's `width` in a physical unit (mm, cm, in, pt, pc)
//! together with its `viewBox` says how long a drawing unit is. Without
//! one (a size in pixels, or none) a unit is a CSS pixel, 1/96 inch, and
//! the result says the scale was assumed.

use r3d_core::PathSegment;
use r3d_geom::DVec2;
use roxmltree::Node;
use svgtypes::{Length, LengthUnit, SimplePathSegment, SimplifyingPathParser};

use super::vector::{Drawing, Shape, Text};
use super::{ImportError, ImportOptions, ImportedPattern, Path, ScaleSource, finish};

const CSS_PX_MM: f64 = 25.4 / 96.0;
/// Handle length of a quarter-circle cubic, as a share of the radius.
const KAPPA: f64 = 0.552_284_749_830_793_4;
/// Default font size (CSS px).
const DEFAULT_FONT_PX: f64 = 16.0;

/// Elements whose content is never drawn as it stands.
const NOT_DRAWN: [&str; 14] = [
    "defs",
    "clipPath",
    "mask",
    "symbol",
    "marker",
    "pattern",
    "metadata",
    "title",
    "desc",
    "style",
    "script",
    "linearGradient",
    "radialGradient",
    "filter",
];

/// An affine map `[a c e; b d f]`, as SVG writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Affine([f64; 6]);

impl Affine {
    const IDENTITY: Affine = Affine([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn apply(&self, p: DVec2) -> DVec2 {
        let [a, b, c, d, e, f] = self.0;
        DVec2::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }

    /// `self` after `inner`: `self(inner(p))`.
    fn then(&self, inner: &Affine) -> Affine {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = inner.0;
        Affine([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }

    /// How much the map scales lengths (geometric mean of the axes).
    fn scale(&self) -> f64 {
        let [a, b, c, d, _, _] = self.0;
        (a * d - b * c).abs().sqrt()
    }
}

fn malformed(detail: impl ToString) -> ImportError {
    ImportError::Malformed {
        format: "SVG",
        detail: detail.to_string(),
    }
}

/// Read the pattern pieces of an SVG document.
pub fn read(bytes: &[u8], options: &ImportOptions) -> Result<ImportedPattern, ImportError> {
    let text = std::str::from_utf8(bytes).map_err(malformed)?;
    let doc = roxmltree::Document::parse(text).map_err(malformed)?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(malformed("the root element is not <svg>"));
    }
    let (mm_per_unit, scale) = options.mm_per_unit(document_scale(&root));
    // Board millimetres, y up.
    let to_board = Affine([mm_per_unit, 0.0, 0.0, -mm_per_unit, 0.0, 0.0]);
    let mut drawing = Drawing::default();
    walk(&root, &to_board, &mut drawing);
    let (pieces, ignored) = super::vector::pieces(&drawing);
    finish(pieces, ignored, mm_per_unit, scale)
}

/// Millimetres per user unit from the root's size and viewBox.
fn document_scale(root: &Node) -> (f64, ScaleSource) {
    let width = root
        .attribute("width")
        .and_then(|w| w.parse::<Length>().ok());
    let view_w = root
        .attribute("viewBox")
        .and_then(|v| v.parse::<svgtypes::ViewBox>().ok())
        .map(|v| v.w)
        .filter(|w| *w > 0.0);
    let physical = |l: &Length| -> Option<f64> {
        let mm = match l.unit {
            LengthUnit::Mm => 1.0,
            LengthUnit::Cm => 10.0,
            LengthUnit::In => 25.4,
            LengthUnit::Pt => 25.4 / 72.0,
            LengthUnit::Pc => 25.4 / 6.0,
            _ => return None,
        };
        Some(l.number * mm)
    };
    match width {
        Some(w) => {
            let units = view_w.unwrap_or(w.number);
            match (physical(&w), units > 0.0) {
                (Some(mm), true) if view_w.is_some() => (mm / units, ScaleSource::File),
                // A physical size without a viewBox: user units are pixels.
                (Some(_), _) => (CSS_PX_MM, ScaleSource::File),
                (None, true) if matches!(w.unit, LengthUnit::Px | LengthUnit::None) => {
                    (CSS_PX_MM * w.number / units, ScaleSource::Assumed)
                }
                _ => (CSS_PX_MM, ScaleSource::Assumed),
            }
        }
        None => (CSS_PX_MM, ScaleSource::Assumed),
    }
}

/// A style property: from the `style` attribute, then the attribute of
/// that name; looked up on ancestors too when `inherited`.
fn property<'a>(node: &Node<'a, '_>, name: &str, inherited: bool) -> Option<&'a str> {
    let mut at = Some(*node);
    while let Some(n) = at {
        if n.is_element() {
            let from_style = n.attribute("style").and_then(|s| {
                s.split(';').find_map(|decl| {
                    let (k, v) = decl.split_once(':')?;
                    (k.trim() == name).then(|| v.trim())
                })
            });
            if let Some(v) = from_style.or_else(|| n.attribute(name)) {
                return Some(v);
            }
        }
        if !inherited {
            return None;
        }
        at = n.parent();
    }
    None
}

fn number(node: &Node, name: &str) -> f64 {
    node.attribute(name)
        .and_then(|v| v.parse::<Length>().ok())
        .map_or(0.0, |l| l.number)
}

fn walk(node: &Node, outer: &Affine, out: &mut Drawing) {
    for child in node.children().filter(Node::is_element) {
        let tag = child.tag_name().name();
        if NOT_DRAWN.contains(&tag) || property(&child, "display", false) == Some("none") {
            continue;
        }
        let local = child
            .attribute("transform")
            .and_then(|t| t.parse::<svgtypes::Transform>().ok())
            .map_or(Affine::IDENTITY, |t| Affine([t.a, t.b, t.c, t.d, t.e, t.f]));
        let m = outer.then(&local);
        if tag == "text" {
            text(&child, &m, out);
            continue;
        }
        let hidden = matches!(
            property(&child, "visibility", true),
            Some("hidden" | "collapse")
        );
        let dashed = property(&child, "stroke-dasharray", true).is_some_and(|d| {
            d != "none" && svgtypes::NumberListParser::from(d).any(|n| n.is_ok_and(|v| v > 0.0))
        });
        let subpaths = if hidden {
            Vec::new()
        } else {
            shapes(&child, tag)
        };
        for (path, closed) in subpaths {
            out.shapes.push(Shape {
                path: path.mapped(|p| m.apply(p)),
                closed,
                dashed,
            });
        }
        walk(&child, &m, out);
    }
}

fn text(node: &Node, m: &Affine, out: &mut Drawing) {
    let first = |name: &str| {
        node.attribute(name)
            .and_then(|v| svgtypes::NumberListParser::from(v).next())
            .and_then(Result::ok)
            .unwrap_or(0.0)
    };
    let content: String = node
        .descendants()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect::<Vec<_>>()
        .join("");
    let content = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if content.is_empty() {
        return;
    }
    let size = property(node, "font-size", true)
        .and_then(|v| v.parse::<Length>().ok())
        .map_or(DEFAULT_FONT_PX, |l| l.number);
    out.texts.push(Text {
        pos: m.apply(DVec2::new(first("x"), first("y"))),
        text: content,
        size_mm: size * m.scale(),
    });
}

/// The subpaths of a basic shape or path element, in its own units, and
/// whether each is closed.
fn shapes(node: &Node, tag: &str) -> Vec<(Path, bool)> {
    let p = |x: f64, y: f64| DVec2::new(x, y);
    let poly = |pts: Vec<DVec2>| Path::from_polyline(&pts);
    match tag {
        "path" => node.attribute("d").map(path_data).unwrap_or_default(),
        "rect" => {
            let (x, y) = (number(node, "x"), number(node, "y"));
            let (w, h) = (number(node, "width"), number(node, "height"));
            if w <= 0.0 || h <= 0.0 {
                return Vec::new();
            }
            poly(vec![
                p(x, y),
                p(x + w, y),
                p(x + w, y + h),
                p(x, y + h),
                p(x, y),
            ])
            .map(|path| vec![(path, true)])
            .unwrap_or_default()
        }
        "circle" | "ellipse" => {
            let (cx, cy) = (number(node, "cx"), number(node, "cy"));
            let (rx, ry) = if tag == "circle" {
                (number(node, "r"), number(node, "r"))
            } else {
                (number(node, "rx"), number(node, "ry"))
            };
            if rx <= 0.0 || ry <= 0.0 {
                return Vec::new();
            }
            vec![(ellipse(p(cx, cy), rx, ry), true)]
        }
        "line" => poly(vec![
            p(number(node, "x1"), number(node, "y1")),
            p(number(node, "x2"), number(node, "y2")),
        ])
        .map(|path| vec![(path, false)])
        .unwrap_or_default(),
        "polyline" | "polygon" => {
            let mut pts: Vec<DVec2> = node
                .attribute("points")
                .map(|s| {
                    svgtypes::PointsParser::from(s)
                        .map(|(x, y)| p(x, y))
                        .collect()
                })
                .unwrap_or_default();
            let closed = tag == "polygon";
            if closed && let Some(first) = pts.first().copied() {
                pts.push(first);
            }
            poly(pts)
                .map(|path| vec![(path, closed)])
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn ellipse(c: DVec2, rx: f64, ry: f64) -> Path {
    let (kx, ky) = (KAPPA * rx, KAPPA * ry);
    let pt = |x: f64, y: f64| c + DVec2::new(x, y);
    Path {
        start: pt(rx, 0.0),
        segments: vec![
            PathSegment::Cubic {
                c1: pt(rx, ky),
                c2: pt(kx, ry),
                to: pt(0.0, ry),
            },
            PathSegment::Cubic {
                c1: pt(-kx, ry),
                c2: pt(-rx, ky),
                to: pt(-rx, 0.0),
            },
            PathSegment::Cubic {
                c1: pt(-rx, -ky),
                c2: pt(-kx, -ry),
                to: pt(0.0, -ry),
            },
            PathSegment::Cubic {
                c1: pt(kx, -ry),
                c2: pt(rx, -ky),
                to: pt(rx, 0.0),
            },
        ],
    }
}

/// Subpaths of path data; a malformed tail is dropped, as browsers do.
fn path_data(d: &str) -> Vec<(Path, bool)> {
    let mut out = Vec::new();
    let mut current: Option<Path> = None;
    let flush = |current: &mut Option<Path>, closed: bool, out: &mut Vec<(Path, bool)>| {
        if let Some(path) = current.take()
            && !path.segments.is_empty()
        {
            out.push((path, closed));
        }
    };
    for seg in SimplifyingPathParser::from(d) {
        let Ok(seg) = seg else {
            break;
        };
        match seg {
            SimplePathSegment::MoveTo { x, y } => {
                flush(&mut current, false, &mut out);
                current = Some(Path {
                    start: DVec2::new(x, y),
                    segments: Vec::new(),
                });
            }
            SimplePathSegment::LineTo { x, y } => {
                if let Some(path) = current.as_mut() {
                    path.segments.push(PathSegment::Line {
                        to: DVec2::new(x, y),
                    });
                }
            }
            SimplePathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                if let Some(path) = current.as_mut() {
                    path.segments.push(PathSegment::Cubic {
                        c1: DVec2::new(x1, y1),
                        c2: DVec2::new(x2, y2),
                        to: DVec2::new(x, y),
                    });
                }
            }
            SimplePathSegment::Quadratic { x1, y1, x, y } => {
                if let Some(path) = current.as_mut() {
                    let (p0, q, p) = (path.end(), DVec2::new(x1, y1), DVec2::new(x, y));
                    path.segments.push(PathSegment::Cubic {
                        c1: p0 + (q - p0) * (2.0 / 3.0),
                        c2: p + (q - p) * (2.0 / 3.0),
                        to: p,
                    });
                }
            }
            SimplePathSegment::ClosePath => {
                let start = current.as_ref().map(|p| p.start);
                flush(&mut current, true, &mut out);
                // A path may go on from where the closed one started.
                current = start.map(|start| Path {
                    start,
                    segments: Vec::new(),
                });
            }
        }
    }
    flush(&mut current, false, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skirt_panel_in_millimetres() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400mm" height="300mm" viewBox="0 0 40 30">
          <g transform="translate(5 5)">
            <path d="M0 0 H20 V20 C 15 22, 5 22, 0 20 Z" fill="none" stroke="black"/>
            <line x1="10" y1="3" x2="10" y2="15" stroke="black"/>
            <polyline points="9.5,4 10,3 10.5,4" stroke="black"/>
            <text x="12" y="10" font-size="1">Back</text>
          </g>
        </svg>"#;
        let out = read(svg.as_bytes(), &ImportOptions::default()).unwrap();
        assert_eq!(out.scale, ScaleSource::File);
        assert!((out.mm_per_unit - 10.0).abs() < 1e-12);
        assert_eq!(out.pieces.len(), 1);
        let p = &out.pieces[0];
        assert_eq!(p.name.as_deref(), Some("Back"));
        assert!(p.grain.is_some());
        assert_eq!(p.outline.segments.len(), 4);
        assert!(matches!(p.outline.segments[2], PathSegment::Cubic { .. }));
        let bb = r3d_geom::polygon::bounding_box(&p.outline.polyline()).unwrap();
        assert!((bb.max.x - bb.min.x - 200.0).abs() < 1e-9);
    }

    #[test]
    fn pixels_are_assumed_and_can_be_overridden() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
            <rect x="10" y="10" width="96" height="96" style="fill:none;stroke:#000"/>
            <circle cx="150" cy="150" r="30"/>
        </svg>"#;
        let out = read(svg.as_bytes(), &ImportOptions::default()).unwrap();
        assert_eq!(out.scale, ScaleSource::Assumed);
        assert_eq!(out.pieces.len(), 2);
        let given = ImportOptions {
            mm_per_unit: Some(1.0),
            ..ImportOptions::default()
        };
        let out = read(svg.as_bytes(), &given).unwrap();
        assert_eq!(out.scale, ScaleSource::Given);
        let bb = r3d_geom::polygon::bounding_box(&out.pieces[0].outline.polyline()).unwrap();
        assert!((bb.max.x - bb.min.x - 96.0).abs() < 1e-9);
    }

    #[test]
    fn not_svg() {
        assert!(matches!(
            read(b"<html/>", &ImportOptions::default()),
            Err(ImportError::Malformed { .. })
        ));
        assert_eq!(
            read(
                br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
                &ImportOptions::default()
            ),
            Err(ImportError::NoPieces)
        );
    }
}
