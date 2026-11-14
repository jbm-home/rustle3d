//! A format-independent drawing of the pattern, in board millimetres (y up).

use r3d_core::Project;
use r3d_geom::{Aabb, DVec2, polygon};

/// What a line represents; renderers map it to a style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stroke {
    /// Cutting line (outermost).
    Cut,
    /// Sewing line, when a seam allowance is drawn outside it.
    Sew,
    /// Internal line (fold, placement…).
    Internal,
    /// Notch tick.
    Notch,
    /// Grainline arrow.
    Grain,
    /// Calibration marks (test square).
    Mark,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Polyline {
    pub points: Vec<DVec2>,
    pub closed: bool,
    pub stroke: Stroke,
}

/// Text centred on `pos`.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub pos: DVec2,
    pub text: String,
    pub size_mm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub lines: Vec<Polyline>,
    pub labels: Vec<Label>,
    /// Everything, plus a margin.
    pub bounds: Aabb,
}

/// Approximate width of `text` in a Helvetica-like font: ~0.5 em per
/// glyph. Good enough to centre labels and keep them off nearby lines.
pub fn approx_text_width(text: &str, size_mm: f64) -> f64 {
    0.5 * size_mm * text.chars().count() as f64
}

/// Side of the calibration square printed with every pattern (mm).
pub const TEST_SQUARE_MM: f64 = 100.0;
const MARGIN_MM: f64 = 10.0;
const NOTCH_MM: f64 = 6.0;
const ARROW_MM: f64 = 8.0;
const LABEL_MM: f64 = 8.0;

/// Build the drawing of every panel plus a 100 mm test square placed above
/// the top-left corner, so it is on the first page of a tiled print.
pub fn pattern_drawing(project: &Project) -> Drawing {
    let mut d = Drawing {
        lines: Vec::new(),
        labels: Vec::new(),
        bounds: Aabb {
            min: DVec2::ZERO,
            max: DVec2::ZERO,
        },
    };
    for (id, panel) in project.panels() {
        let Some(outline) = project.panel_outline(*id) else {
            continue;
        };
        let sa = panel.seam_allowance_mm;
        if sa > 0.0 {
            d.lines.push(Polyline {
                points: polygon::offset_outward(&outline, sa),
                closed: true,
                stroke: Stroke::Cut,
            });
            d.lines.push(Polyline {
                points: outline.clone(),
                closed: true,
                stroke: Stroke::Sew,
            });
        } else {
            d.lines.push(Polyline {
                points: outline.clone(),
                closed: true,
                stroke: Stroke::Cut,
            });
        }
        // Notches: a tick cut inward from the cutting line.
        for (nid, notch) in project.notches() {
            if !panel.contour.contains(&notch.edge) {
                continue;
            }
            if let Some(at) = project.notch_placement(*nid) {
                let outward = DVec2::new(at.tangent.y, -at.tangent.x);
                let start = at.pos + outward * sa;
                d.lines.push(Polyline {
                    points: vec![start, start - outward * NOTCH_MM],
                    closed: false,
                    stroke: Stroke::Notch,
                });
            }
        }
        for lid in project
            .internal_lines()
            .iter()
            .filter(|(_, l)| l.panel == *id)
            .map(|(lid, _)| *lid)
        {
            if let Some(points) = project.internal_line_polyline(lid) {
                d.lines.push(Polyline {
                    points,
                    closed: false,
                    stroke: Stroke::Internal,
                });
            }
        }
        if let Some(bb) = polygon::bounding_box(&outline) {
            let center = (bb.min + bb.max) * 0.5;
            let dir = DVec2::from_angle(panel.grain_angle_deg.to_radians());
            let (lo, hi) = outline
                .iter()
                .map(|p| (*p - center).dot(dir))
                .fold((f64::MAX, f64::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
            let half = 0.3 * (hi - lo);
            push_arrow(&mut d, center - dir * half, center + dir * half);
            // Beside the grainline, never across it.
            let clearance = 0.5 * approx_text_width(&panel.name, LABEL_MM) + LABEL_MM;
            d.labels.push(Label {
                pos: center + dir.perp() * clearance,
                text: panel.name.clone(),
                size_mm: LABEL_MM,
            });
        }
    }

    let content = bounds_of(&d.lines);
    let corner = match content {
        Some(b) => DVec2::new(b.min.x, b.max.y + 2.0 * MARGIN_MM),
        None => DVec2::ZERO,
    };
    let s = TEST_SQUARE_MM;
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
        pos: corner + DVec2::new(s / 2.0, s / 2.0),
        text: format!("{s:.0} mm"),
        size_mm: LABEL_MM,
    });
    d.labels.push(Label {
        pos: corner + DVec2::new(s / 2.0, s / 2.0 - 1.5 * LABEL_MM),
        text: project.name().to_owned(),
        size_mm: LABEL_MM * 0.6,
    });

    if let Some(b) = bounds_of(&d.lines) {
        d.bounds = Aabb {
            min: b.min - DVec2::splat(MARGIN_MM),
            max: b.max + DVec2::splat(MARGIN_MM),
        };
    }
    d
}

fn push_arrow(d: &mut Drawing, a: DVec2, b: DVec2) {
    d.lines.push(Polyline {
        points: vec![a, b],
        closed: false,
        stroke: Stroke::Grain,
    });
    for (tip, back) in [(b, a), (a, b)] {
        let dir = (back - tip).normalize_or_zero();
        for angle in [0.4_f64, -0.4] {
            let wing = DVec2::from_angle(angle).rotate(dir) * ARROW_MM;
            d.lines.push(Polyline {
                points: vec![tip, tip + wing],
                closed: false,
                stroke: Stroke::Grain,
            });
        }
    }
}

fn bounds_of(lines: &[Polyline]) -> Option<Aabb> {
    let all: Vec<DVec2> = lines
        .iter()
        .flat_map(|l| l.points.iter().copied())
        .collect();
    polygon::bounding_box(&all)
}
