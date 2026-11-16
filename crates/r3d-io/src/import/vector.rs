//! Reading pattern pieces out of a plain vector drawing (SVG, PDF), where
//! nothing says what a line is. The rules follow how patterns are drawn:
//!
//! - an outermost closed shape is a piece (its cutting line);
//! - a closed shape inside it at an even distance is its sewing line, and
//!   that distance is the seam allowance;
//! - a short straight tick (≤ 15 mm) touching the outline is a notch;
//! - a straight line with arrowheads (short strokes at its ends) is the
//!   grainline;
//! - any other line or closed shape entirely inside the piece is an
//!   internal line;
//! - the largest text inside the piece is its name;
//! - a calibration square (25.4, 50, 50.8 or 100 mm) holding only its own
//!   caption, and page frames, are not pieces.

use r3d_geom::{DVec2, polygon};

use super::{
    ImportedPiece, Path, closed, distance_to_polyline, nearest_on, seam_allowance, simplify,
};

/// A stroke of the drawing, in board millimetres (y up).
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub path: Path,
    /// Closed by the file (`Z`, `h`, polygon), or ending where it starts.
    pub closed: bool,
    pub dashed: bool,
}

/// A piece of text, in board millimetres.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub pos: DVec2,
    pub text: String,
    pub size_mm: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    pub shapes: Vec<Shape>,
    pub texts: Vec<Text>,
    /// Page area (mm²), when the drawing is a page: frames covering most
    /// of it are not pieces.
    pub page_area: Option<f64>,
}

/// Longest notch tick (mm).
const NOTCH_MAX_MM: f64 = 15.0;
/// How close a tick must come to the outline to be a notch (mm).
const ON_OUTLINE_MM: f64 = 1.0;
/// Shortest grainline (mm).
const GRAIN_MIN_MM: f64 = 30.0;
/// Arrowheads are at most this share of their grainline's length.
const WING_SHARE: f64 = 0.3;
/// Calibration squares, by side (mm).
const TEST_SQUARES_MM: [f64; 4] = [25.4, 50.0, 50.8, 100.0];
/// A closed shape covering this much of the page is a frame.
const FRAME_SHARE: f64 = 0.6;

struct Closed {
    shape: Shape,
    poly: Vec<DVec2>,
    area: f64,
}

/// Pieces found in the drawing, with their raw (unfitted) outline, and
/// the number of shapes that belong to none.
pub fn pieces(d: &Drawing) -> (Vec<(ImportedPiece, Path)>, usize) {
    let mut closed_shapes: Vec<Closed> = Vec::new();
    let mut open: Vec<(Shape, Vec<DVec2>)> = Vec::new();
    for s in &d.shapes {
        let mut poly = s.path.polyline();
        let ends_meet = poly.len() > 2 && poly[0].distance(poly[poly.len() - 1]) < 0.01;
        if (s.closed || ends_meet) && poly.len() >= 3 {
            if ends_meet {
                poly.pop();
            }
            let area = polygon::signed_area(&poly).abs();
            if area > 1.0 {
                let shape = Shape {
                    path: closed(s.path.clone()),
                    closed: true,
                    dashed: s.dashed,
                };
                closed_shapes.push(Closed { shape, poly, area });
                continue;
            }
        }
        if poly.len() >= 2 {
            open.push((s.clone(), poly));
        }
    }
    let mut ignored = 0;

    // The same outline drawn twice (fill then stroke) counts once.
    closed_shapes.sort_by(|a, b| b.area.total_cmp(&a.area));
    let mut unique: Vec<Closed> = Vec::new();
    for c in closed_shapes {
        let dup = unique.iter().any(|u| {
            (u.area - c.area).abs() <= 1e-3 * u.area
                && u.poly
                    .iter()
                    .zip(&c.poly)
                    .all(|(a, b)| a.distance(*b) < 0.05)
        });
        if !dup {
            unique.push(c);
        }
    }

    // Page frames and calibration squares.
    let inside = |outer: &[DVec2], p: DVec2| {
        polygon::contains(outer, p) || distance_to_polyline(p, outer, true) <= ON_OUTLINE_MM
    };
    let mut texts_used = vec![false; d.texts.len()];
    unique.retain(|c| {
        if d.page_area.is_some_and(|page| c.area >= FRAME_SHARE * page) {
            return false;
        }
        if is_calibration_square(&c.poly) {
            let caption: Vec<usize> = (0..d.texts.len())
                .filter(|&i| polygon::contains(&c.poly, d.texts[i].pos))
                .collect();
            let lines_inside = open
                .iter()
                .any(|(_, l)| l.iter().all(|p| polygon::contains(&c.poly, *p)));
            let captioned = caption.iter().any(|&i| mentions_size(&d.texts[i].text));
            if captioned && !lines_inside {
                for i in caption {
                    texts_used[i] = true;
                }
                return false;
            }
        }
        true
    });

    // Outermost closed shapes are pieces; each other closed shape goes to
    // the smallest piece holding it.
    let n = unique.len();
    let holds =
        |a: &Closed, b: &Closed| a.area > b.area && b.poly.iter().all(|p| inside(&a.poly, *p));
    let is_piece: Vec<bool> = (0..n)
        .map(|i| !(0..n).any(|j| j != i && holds(&unique[j], &unique[i])))
        .collect();
    let owner = |poly: &[DVec2]| -> Option<usize> {
        (0..n)
            .filter(|&j| is_piece[j] && poly.iter().all(|p| inside(&unique[j].poly, *p)))
            .min_by(|&a, &b| unique[a].area.total_cmp(&unique[b].area))
    };

    struct Draft {
        cut: usize,
        sew: Option<(usize, f64)>,
        inner: Vec<Path>,
        open: Vec<(Shape, Vec<DVec2>)>,
    }
    let mut drafts: Vec<Draft> = (0..n)
        .filter(|&i| is_piece[i])
        .map(|i| Draft {
            cut: i,
            sew: None,
            inner: Vec::new(),
            open: Vec::new(),
        })
        .collect();
    let draft_of = |piece: usize, drafts: &[Draft]| drafts.iter().position(|d| d.cut == piece);

    for i in (0..n).filter(|&i| !is_piece[i]) {
        let Some(p) = owner(&unique[i].poly).and_then(|p| draft_of(p, &drafts)) else {
            ignored += 1;
            continue;
        };
        let draft = &mut drafts[p];
        let sa = seam_allowance(&unique[draft.cut].poly, &unique[i].poly);
        match (sa, draft.sew) {
            // The largest even offset is the sewing line (shapes are
            // sorted by decreasing area).
            (Some(sa), None) => draft.sew = Some((i, sa)),
            _ => draft.inner.push(unique[i].shape.path.clone()),
        }
    }
    for (shape, poly) in open {
        let piece = owner(&poly).or_else(|| touching(&unique, &is_piece, &poly));
        match piece.and_then(|p| draft_of(p, &drafts)) {
            Some(p) => drafts[p].open.push((shape, poly)),
            None => ignored += 1,
        }
    }

    let mut out = Vec::new();
    for draft in drafts {
        let cut = &unique[draft.cut];
        let (outline_raw, sa, sew_poly) = match draft.sew {
            Some((i, sa)) => (unique[i].shape.path.clone(), sa, unique[i].poly.clone()),
            None => (cut.shape.path.clone(), 0.0, cut.poly.clone()),
        };
        let (grain, wings) = grainline(&draft.open);
        let mut notches = Vec::new();
        let mut internal_lines: Vec<Path> = draft.inner.iter().map(|p| simplify(p, true)).collect();
        for (k, (shape, poly)) in draft.open.iter().enumerate() {
            if wings.contains(&k) || grain.is_some_and(|(g, _)| g == k) {
                continue;
            }
            if let Some(at) = notch(poly, &cut.poly, &sew_poly) {
                notches.push(at);
            } else {
                internal_lines.push(simplify(&shape.path, false));
            }
        }
        let name = d
            .texts
            .iter()
            .enumerate()
            .filter(|(i, t)| !texts_used[*i] && polygon::contains(&cut.poly, t.pos))
            .max_by(|a, b| a.1.size_mm.total_cmp(&b.1.size_mm))
            .map(|(_, t)| t.text.trim().to_owned())
            .filter(|t| !t.is_empty());
        let piece = ImportedPiece {
            name,
            outline: simplify(&outline_raw, true),
            seam_allowance_mm: sa,
            grain: grain.map(|(_, line)| line),
            notches,
            internal_lines,
        };
        out.push((piece, outline_raw));
    }
    (out, ignored)
}

/// A short open line not inside any piece but touching one's outline (a
/// notch drawn outwards): that piece.
fn touching(unique: &[Closed], is_piece: &[bool], poly: &[DVec2]) -> Option<usize> {
    if length(poly) > NOTCH_MAX_MM {
        return None;
    }
    let (Some(a), Some(b)) = (poly.first(), poly.last()) else {
        return None;
    };
    (0..unique.len()).find(|&j| {
        is_piece[j]
            && [a, b]
                .iter()
                .any(|p| distance_to_polyline(**p, &unique[j].poly, true) <= ON_OUTLINE_MM)
    })
}

fn length(poly: &[DVec2]) -> f64 {
    poly.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// A closed axis-aligned square of a calibration size.
fn is_calibration_square(poly: &[DVec2]) -> bool {
    let Some(bb) = polygon::bounding_box(poly) else {
        return false;
    };
    let size = bb.max - bb.min;
    let sized = TEST_SQUARES_MM
        .into_iter()
        .any(|s| (size.x - s).abs() <= 0.01 * s && (size.y - s).abs() <= 0.01 * s);
    let area = polygon::signed_area(poly).abs();
    sized && (area - size.x * size.y).abs() <= 0.02 * area
}

/// Whether a caption names a size: "100 mm", "10 cm", "2 in", "2\"".
fn mentions_size(text: &str) -> bool {
    let t = text.to_lowercase();
    t.chars().any(|c| c.is_ascii_digit()) && ["mm", "cm", "in", "\""].iter().any(|u| t.contains(u))
}

/// The grainline: the longest straight line with arrowheads (other short
/// strokes touching one of its ends). Returns its index in `open` and
/// its ends, and the indices of its arrowheads.
#[allow(clippy::type_complexity)]
fn grainline(open: &[(Shape, Vec<DVec2>)]) -> (Option<(usize, (DVec2, DVec2))>, Vec<usize>) {
    let mut best: Option<(usize, (DVec2, DVec2), Vec<usize>, f64)> = None;
    for (k, (_, poly)) in open.iter().enumerate() {
        let [a, b] = poly[..] else {
            continue;
        };
        let len = a.distance(b);
        if len < GRAIN_MIN_MM || best.as_ref().is_some_and(|x| x.3 >= len) {
            continue;
        }
        let wings: Vec<usize> = open
            .iter()
            .enumerate()
            .filter(|(j, (_, w))| {
                *j != k
                    && length(w) <= WING_SHARE * len
                    && w.iter().any(|p| p.distance(a) < 0.5 || p.distance(b) < 0.5)
            })
            .map(|(j, _)| j)
            .collect();
        if !wings.is_empty() {
            best = Some((k, (a, b), wings, len));
        }
    }
    match best {
        Some((k, line, wings, _)) => (Some((k, line)), wings),
        None => (None, Vec::new()),
    }
}

/// A notch: a short straight tick with an end on the cutting or sewing
/// line, roughly across it. Returns where it meets the sewing line.
fn notch(poly: &[DVec2], cut: &[DVec2], sew: &[DVec2]) -> Option<DVec2> {
    let [a, b] = poly[..] else {
        return None;
    };
    if a.distance(b) > NOTCH_MAX_MM {
        return None;
    }
    let on = |p: DVec2| {
        distance_to_polyline(p, cut, true) <= ON_OUTLINE_MM
            || distance_to_polyline(p, sew, true) <= ON_OUTLINE_MM
    };
    let end = if on(a) {
        a
    } else if on(b) {
        b
    } else {
        return None;
    };
    Some(nearest_on(end, sew))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(points: &[(f64, f64)], closed: bool, dashed: bool) -> Shape {
        let pts: Vec<DVec2> = points.iter().map(|&(x, y)| DVec2::new(x, y)).collect();
        Shape {
            path: Path::from_polyline(&pts).unwrap(),
            closed,
            dashed,
        }
    }

    fn text(x: f64, y: f64, s: &str, size: f64) -> Text {
        Text {
            pos: DVec2::new(x, y),
            text: s.into(),
            size_mm: size,
        }
    }

    /// A 200 × 300 piece with a 10 mm allowance, a notch, a grainline
    /// with arrows, a dart and a name; and a test square.
    fn drawing() -> Drawing {
        let mut shapes = vec![
            line(
                &[
                    (-10.0, -10.0),
                    (210.0, -10.0),
                    (210.0, 310.0),
                    (-10.0, 310.0),
                ],
                true,
                false,
            ),
            line(
                &[(0.0, 0.0), (200.0, 0.0), (200.0, 300.0), (0.0, 300.0)],
                true,
                true,
            ),
            // Notch on the right side, from the cutting line inward.
            line(&[(210.0, 150.0), (204.0, 150.0)], false, false),
            // Grainline with arrowheads at both ends.
            line(&[(100.0, 50.0), (100.0, 250.0)], false, false),
            line(&[(100.0, 250.0), (97.0, 243.0)], false, false),
            line(&[(100.0, 250.0), (103.0, 243.0)], false, false),
            line(&[(100.0, 50.0), (97.0, 57.0)], false, false),
            line(&[(100.0, 50.0), (103.0, 57.0)], false, false),
            // A dart.
            line(&[(40.0, 300.0), (50.0, 200.0), (60.0, 300.0)], false, true),
            // Test square.
            line(
                &[(0.0, 400.0), (100.0, 400.0), (100.0, 500.0), (0.0, 500.0)],
                true,
                false,
            ),
            // A guide crossing everything.
            line(&[(-50.0, 100.0), (500.0, 100.0)], false, true),
        ];
        shapes.push(line(&[(150.0, 20.0), (160.0, 20.0)], false, false));
        Drawing {
            shapes,
            texts: vec![
                text(130.0, 150.0, "Front", 8.0),
                text(130.0, 120.0, "cut 2", 4.0),
                text(50.0, 450.0, "100 mm", 8.0),
            ],
            page_area: None,
        }
    }

    #[test]
    fn a_drawn_piece_is_read_back() {
        let (pieces, ignored) = pieces(&drawing());
        assert_eq!(pieces.len(), 1, "{pieces:?}");
        let (p, _) = &pieces[0];
        assert_eq!(p.name.as_deref(), Some("Front"));
        assert_eq!(p.seam_allowance_mm, 10.0);
        assert_eq!(p.outline.segments.len(), 4);
        assert_eq!(p.notches, vec![DVec2::new(200.0, 150.0)]);
        let (a, b) = p.grain.unwrap();
        assert_eq!(super::super::grain_angle_deg((a, b)), 90.0);
        // The dart and the short dash inside; the guide crosses out.
        assert_eq!(p.internal_lines.len(), 2);
        assert_eq!(ignored, 1);
    }

    #[test]
    fn a_page_frame_is_not_a_piece() {
        let mut d = drawing();
        d.shapes.push(line(
            &[
                (-100.0, -100.0),
                (900.0, -100.0),
                (900.0, 900.0),
                (-100.0, 900.0),
            ],
            true,
            false,
        ));
        d.page_area = Some(1000.0 * 1000.0);
        assert_eq!(pieces(&d).0.len(), 1);
    }
}
