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
    /// Cutting line of another size of a graded nest: an index into
    /// [`SIZE_COLORS`] (wrapping).
    Size(usize),
}

/// Colours of the sizes of a graded nest (sRGB), told apart in print.
pub const SIZE_COLORS: [u32; 8] = [
    0xD62728, 0x1F77B4, 0x2CA02C, 0x9467BD, 0xFF7F0E, 0x8C564B, 0xE377C2, 0x17BECF,
];

impl Stroke {
    /// The colour of a size's cutting line.
    pub fn size_color(k: usize) -> u32 {
        SIZE_COLORS.get(k % SIZE_COLORS.len()).copied().unwrap_or(0)
    }
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
const GRAIN_SHRINK_STEPS: usize = 12;
/// The graded nest's legend: row height, line sample and gaps (mm).
const LEGEND_ROW_MM: f64 = 9.0;
const LEGEND_LINE_MM: f64 = 20.0;
const LEGEND_GAP_MM: f64 = 10.0;

/// Build the drawing of every panel plus a 100 mm test square placed above
/// the top-left corner, so it is on the first page of a tiled print.
pub fn pattern_drawing(project: &Project) -> Drawing {
    draw(project, project.name(), &[])
}

/// One size of a graded nest: its name, how its cutting lines are drawn,
/// and the lines.
struct Nested {
    name: String,
    stroke: Stroke,
    outlines: Vec<Vec<DVec2>>,
}

/// The pattern in sizes `sizes` (indices into the project's range): one
/// size alone, or several nested. The base size, or else the first listed,
/// is drawn in full (sewing line, notches, internal lines, grainline,
/// name); the others add their cutting lines in colour, named in a legend
/// beside the test square. Without a range: the pattern as it is.
pub fn graded_drawing(project: &Project, sizes: &[usize]) -> Drawing {
    let Some(g) = project.grading() else {
        return pattern_drawing(project);
    };
    let name = |k: usize| g.sizes.get(k).map(|s| s.name.clone()).unwrap_or_default();
    let reference = if sizes.contains(&g.base) {
        g.base
    } else {
        sizes.first().copied().unwrap_or(g.base)
    };
    let main = project.at_size(reference);
    let mut nest = Vec::new();
    if sizes.len() > 1 {
        nest.push(Nested {
            name: name(reference),
            stroke: Stroke::Cut,
            outlines: Vec::new(),
        });
    }
    let others = sizes.iter().filter(|k| **k != reference);
    for (rank, k) in others.enumerate() {
        let sized = project.at_size(*k);
        let outlines = sized
            .panels()
            .iter()
            .filter_map(|(id, panel)| {
                let outline = sized.panel_outline(*id)?;
                Some(if panel.seam_allowance_mm > 0.0 {
                    polygon::offset_outward(&outline, panel.seam_allowance_mm)
                } else {
                    outline
                })
            })
            .collect();
        nest.push(Nested {
            name: name(*k),
            stroke: Stroke::Size(rank),
            outlines,
        });
    }
    let names: Vec<String> = sizes.iter().map(|k| name(*k)).collect();
    let title = format!("{} - {}", project.name(), names.join(" "));
    draw(&main, &title, &nest)
}

fn draw(project: &Project, title: &str, nest: &[Nested]) -> Drawing {
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
        if let Some((a, b)) = grainline(&outline, panel.grain_angle_deg) {
            let center = (a + b) * 0.5;
            let dir = (b - a).normalize_or_zero();
            push_arrow(&mut d, a, b);
            // Beside the grainline, never across it.
            let clearance = 0.5 * approx_text_width(&panel.name, LABEL_MM) + LABEL_MM;
            d.labels.push(Label {
                pos: center + dir.perp() * clearance,
                text: panel.name.clone(),
                size_mm: LABEL_MM,
            });
        }
    }

    for size in nest {
        for outline in &size.outlines {
            d.lines.push(Polyline {
                points: outline.clone(),
                closed: true,
                stroke: size.stroke,
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
        text: title.to_owned(),
        size_mm: LABEL_MM * 0.6,
    });
    // The nest's legend, beside the square: a sample of each size's line.
    for (row, size) in nest.iter().enumerate() {
        let y = corner.y + s - (row as f64 + 0.5) * LEGEND_ROW_MM;
        let start = corner + DVec2::new(s + LEGEND_GAP_MM, 0.0);
        d.lines.push(Polyline {
            points: vec![
                DVec2::new(start.x, y),
                DVec2::new(start.x + LEGEND_LINE_MM, y),
            ],
            closed: false,
            stroke: size.stroke,
        });
        let text_x = start.x + LEGEND_LINE_MM + LEGEND_GAP_MM / 2.0;
        d.labels.push(Label {
            pos: DVec2::new(
                text_x + approx_text_width(&size.name, LABEL_MM * 0.6) / 2.0,
                y,
            ),
            text: size.name.clone(),
            size_mm: LABEL_MM * 0.6,
        });
    }

    if let Some(b) = bounds_of(&d.lines) {
        d.bounds = Aabb {
            min: b.min - DVec2::splat(MARGIN_MM),
            max: b.max + DVec2::splat(MARGIN_MM),
        };
    }
    d
}

/// Where to draw a panel's grainline: through the middle of its bounding
/// box, 60 % of the panel's extent along the grain, shortened until it
/// lies inside the outline (a bag's handle opening, a deep neckline).
pub(crate) fn grainline(outline: &[DVec2], angle_deg: f64) -> Option<(DVec2, DVec2)> {
    let bb = polygon::bounding_box(outline)?;
    let center = (bb.min + bb.max) * 0.5;
    let dir = DVec2::from_angle(angle_deg.to_radians());
    let (lo, hi) = outline
        .iter()
        .map(|p| (*p - center).dot(dir))
        .fold((f64::MAX, f64::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
    let mut half = 0.3 * (hi - lo);
    let n = outline.len();
    let inside = |a: DVec2, b: DVec2| {
        polygon::contains(outline, a)
            && polygon::contains(outline, b)
            && (0..n).all(|i| !polygon::segments_intersect(a, b, outline[i], outline[(i + 1) % n]))
    };
    for _ in 0..GRAIN_SHRINK_STEPS {
        let (a, b) = (center - dir * half, center + dir * half);
        if inside(a, b) {
            return Some((a, b));
        }
        half *= 0.8;
    }
    // The middle is outside the piece (a ring, a crescent): keep the full
    // line rather than none.
    Some((
        center - dir * 0.3 * (hi - lo),
        center + dir * 0.3 * (hi - lo),
    ))
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
