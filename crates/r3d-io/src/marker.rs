//! The drawing of a marker: pieces laid out on a strip of fabric, at 1:1
//! (`docs/specs/23-production.md`). The layout itself is computed by the
//! command layer; this only draws it, for the SVG and PDF writers.

use r3d_geom::{Aabb, DVec2, polygon};

use crate::drawing::{
    Drawing, Label, Polyline, Stroke, TEST_SQUARE_MM, approx_text_width, grainline, push_arrow,
};

/// One piece where it lies on the fabric (marker mm: x along the length,
/// y across the width, 0 at the bottom selvedge).
#[derive(Clone, Debug, PartialEq)]
pub struct SheetPiece {
    /// Cutting line, closed.
    pub cut: Vec<DVec2>,
    /// Sewing line, when a seam allowance is drawn outside it.
    pub sew: Option<Vec<DVec2>>,
    /// Holes, cut on their line.
    pub holes: Vec<Vec<DVec2>>,
    /// Direction of the piece's grainline on the fabric (degrees).
    pub grain_angle_deg: f64,
    /// Name and size, written on the piece.
    pub label: String,
    /// Which of the marker's sizes it is (0: the first, drawn in black;
    /// the others in the graded nest's colours).
    pub size_rank: usize,
}

/// A marker to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub width_mm: f64,
    pub length_mm: f64,
    pub pieces: Vec<SheetPiece>,
    /// Written under the fabric: the project, the sizes, the figures.
    pub caption: Vec<String>,
}

const MARGIN_MM: f64 = 10.0;
const LABEL_MM: f64 = 8.0;
const CAPTION_MM: f64 = 6.0;
/// Fabric drawn past the marker's end, so its last piece is not on the
/// edge of the drawing.
const TAIL_MM: f64 = 20.0;

/// The marker's drawing: the fabric's two selvedges and its ends, each
/// piece's lines, grainline and label, a caption and the 100 mm test
/// square below the fabric.
pub fn marker_drawing(sheet: &Sheet) -> Drawing {
    let mut d = Drawing {
        lines: Vec::new(),
        labels: Vec::new(),
        bounds: Aabb {
            min: DVec2::ZERO,
            max: DVec2::ZERO,
        },
    };
    let (w, l) = (sheet.width_mm, sheet.length_mm);
    // The fabric: selvedges along the length, the marker's two ends.
    d.lines.push(Polyline {
        points: vec![
            DVec2::ZERO,
            DVec2::new(l, 0.0),
            DVec2::new(l, w),
            DVec2::new(0.0, w),
        ],
        closed: true,
        stroke: Stroke::Mark,
    });
    d.lines.push(Polyline {
        points: vec![DVec2::new(l, 0.0), DVec2::new(l + TAIL_MM, 0.0)],
        closed: false,
        stroke: Stroke::Mark,
    });
    d.lines.push(Polyline {
        points: vec![DVec2::new(l, w), DVec2::new(l + TAIL_MM, w)],
        closed: false,
        stroke: Stroke::Mark,
    });
    for piece in &sheet.pieces {
        let stroke = match piece.size_rank {
            0 => Stroke::Cut,
            k => Stroke::Size(k - 1),
        };
        d.lines.push(Polyline {
            points: piece.cut.clone(),
            closed: true,
            stroke,
        });
        if let Some(sew) = &piece.sew {
            d.lines.push(Polyline {
                points: sew.clone(),
                closed: true,
                stroke: Stroke::Sew,
            });
        }
        for hole in &piece.holes {
            d.lines.push(Polyline {
                points: hole.clone(),
                closed: true,
                stroke,
            });
        }
        let inner = piece.sew.as_ref().unwrap_or(&piece.cut);
        if let Some((a, b)) = grainline(inner, piece.grain_angle_deg) {
            push_arrow(&mut d, a, b);
            let dir = (b - a).normalize_or_zero();
            // Beside the grainline, never across it; small pieces get a
            // smaller label.
            let span =
                polygon::bounding_box(inner).map_or(0.0, |bb| (bb.max - bb.min).min_element());
            let size = LABEL_MM.min(span / 4.0).max(2.0);
            let clearance = 0.5 * size + size * 0.6;
            d.labels.push(Label {
                pos: (a + b) * 0.5 + dir.perp() * clearance,
                text: piece.label.clone(),
                size_mm: size,
            });
        }
    }
    // Below the fabric: the caption, then the test square.
    let mut y = -MARGIN_MM - CAPTION_MM;
    for line in &sheet.caption {
        d.labels.push(Label {
            pos: DVec2::new(approx_text_width(line, CAPTION_MM) / 2.0, y),
            text: line.clone(),
            size_mm: CAPTION_MM,
        });
        y -= 1.6 * CAPTION_MM;
    }
    let s = TEST_SQUARE_MM;
    let corner = DVec2::new(0.0, y - s);
    d.lines.push(Polyline {
        points: vec![
            corner,
            corner + DVec2::new(s, 0.0),
            corner + DVec2::new(s, s),
            corner + DVec2::new(0.0, s),
        ],
        closed: true,
        stroke: Stroke::Mark,
    });
    d.labels.push(Label {
        pos: corner + DVec2::splat(s / 2.0),
        text: format!("{s:.0} mm"),
        size_mm: LABEL_MM,
    });
    let right = (l + TAIL_MM).max(
        sheet
            .caption
            .iter()
            .map(|c| approx_text_width(c, CAPTION_MM))
            .fold(s, f64::max),
    );
    d.bounds = Aabb {
        min: DVec2::new(-MARGIN_MM, corner.y - MARGIN_MM),
        max: DVec2::new(right + MARGIN_MM, w + MARGIN_MM),
    };
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_the_fabric_pieces_and_square() {
        let square = vec![
            DVec2::new(10.0, 10.0),
            DVec2::new(110.0, 10.0),
            DVec2::new(110.0, 60.0),
            DVec2::new(10.0, 60.0),
        ];
        let sheet = Sheet {
            width_mm: 1500.0,
            length_mm: 120.0,
            pieces: vec![SheetPiece {
                cut: square.clone(),
                sew: None,
                holes: Vec::new(),
                grain_angle_deg: 0.0,
                label: "Front 40".into(),
                size_rank: 1,
            }],
            caption: vec!["1500 x 120 mm".into()],
        };
        let d = marker_drawing(&sheet);
        assert!(d.lines.iter().any(|l| l.stroke == Stroke::Size(0)));
        assert!(d.lines.iter().any(|l| l.stroke == Stroke::Grain));
        assert!(d.labels.iter().any(|l| l.text == "Front 40"));
        assert!(d.labels.iter().any(|l| l.text == "100 mm"));
        assert!(d.bounds.max.y >= 1500.0 && d.bounds.min.y < -100.0);
    }
}
