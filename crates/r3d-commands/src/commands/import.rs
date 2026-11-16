use r3d_core::{AnyId, PanelId, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    InternalLineAdd, NotchAdd, PanelCreatePath, PanelSetSeamAllowance, SegmentSpec, Vec2, v,
};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Gap left between the pieces already on the board and imported ones (mm).
const GAP_MM: f64 = 100.0;
/// A notch further than this from the outline is dropped (mm).
const NOTCH_REACH_MM: f64 = 5.0;

/// An open path (mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LineSpec {
    pub start: Vec2,
    /// At least one segment.
    pub segments: Vec<SegmentSpec>,
}

/// One piece of an imported pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PieceSpec {
    /// Piece name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The sewing line, as in `panel.create_path`: first point (mm)…
    pub start: Vec2,
    /// …and the edges in order; the last should end at `start`.
    pub segments: Vec<SegmentSpec>,
    /// Seam allowance in mm (default 0).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub seam_allowance_mm: f64,
    /// Grainline angle in degrees from +X (default 90, vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
    /// Notches: points on or near the outline (within 5 mm), each put on
    /// the nearest edge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notches: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_lines: Vec<LineSpec>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn default_grain() -> f64 {
    super::DEFAULT_GRAIN_ANGLE_DEG
}

/// Add the pieces of a pattern made elsewhere.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PatternImport {
    /// At least one piece.
    pub pieces: Vec<PieceSpec>,
    /// Keep the given coordinates. By default the pieces are moved, all
    /// together, to the right of the pieces already on the board.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep_position: bool,
}

impl Described for PatternImport {
    const SUMMARY: &'static str = "Add pattern pieces in one step: for each, its sewing line (as in panel.create_path), seam allowance, grainline angle, notches (points near the outline) and internal lines. Placed to the right of the existing pieces unless keep_position. `import.pattern` reads them from a DXF, SVG or PDF file. Returns, for each piece, its panel, points and edges, then its notches and lines.";
}

impl CommandSpec for PatternImport {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if self.pieces.is_empty() {
            return Err(CommandError::invalid("pieces", Invalid::Empty));
        }
        for p in &self.pieces {
            finite("start", &p.start)?;
        }
        let shift = if self.keep_position {
            DVec2::ZERO
        } else {
            beside_existing(tx, &self.pieces)
        };
        let moved = |p: Vec2| -> Vec2 { [p[0] + shift.x, p[1] + shift.y] };
        let moved_segments = |segs: &[SegmentSpec]| -> Vec<SegmentSpec> {
            segs.iter()
                .map(|s| match s {
                    SegmentSpec::Line { to } => SegmentSpec::Line { to: moved(*to) },
                    SegmentSpec::Cubic { c1, c2, to } => SegmentSpec::Cubic {
                        c1: moved(*c1),
                        c2: moved(*c2),
                        to: moved(*to),
                    },
                })
                .collect()
        };

        let mut created: Vec<AnyId> = Vec::new();
        for piece in &self.pieces {
            let out = PanelCreatePath {
                name: piece.name.clone(),
                start: moved(piece.start),
                segments: moved_segments(&piece.segments),
                grain_angle_deg: piece.grain_angle_deg,
            }
            .apply(tx)?;
            let Some(AnyId::Panel(panel)) = out.created.first().copied() else {
                return Err(CommandError::Internal {
                    detail: "panel.create_path returned no panel".into(),
                });
            };
            created.extend(out.created);
            if piece.seam_allowance_mm != 0.0 {
                PanelSetSeamAllowance {
                    panel,
                    mm: piece.seam_allowance_mm,
                }
                .apply(tx)?;
            }
            for n in &piece.notches {
                finite("notches", n)?;
                if let Some(notch) = notch_on(tx, panel, v(moved(*n))) {
                    created.extend(notch.apply(tx)?.created);
                }
            }
            for line in &piece.internal_lines {
                created.extend(
                    InternalLineAdd {
                        panel,
                        start: moved(line.start),
                        segments: moved_segments(&line.segments),
                    }
                    .apply(tx)?
                    .created,
                );
            }
        }
        Ok(CommandOutput { created })
    }
}

/// The shift that puts the imported pieces to the right of the existing
/// ones, bottoms aligned; none on an empty board.
fn beside_existing(tx: &Transaction<'_>, pieces: &[PieceSpec]) -> DVec2 {
    let project = tx.project();
    let right = project
        .panels()
        .keys()
        .filter_map(|id| project.panel_bounding_box(*id))
        .map(|b| b.max.x)
        .fold(f64::NEG_INFINITY, f64::max);
    if !right.is_finite() {
        return DVec2::ZERO;
    }
    let points = pieces.iter().flat_map(|p| {
        std::iter::once(p.start).chain(p.segments.iter().flat_map(|s| match s {
            SegmentSpec::Line { to } => vec![*to],
            SegmentSpec::Cubic { c1, c2, to } => vec![*c1, *c2, *to],
        }))
    });
    let (min_x, min_y) = points
        .filter(|p| p[0].is_finite() && p[1].is_finite())
        .fold((f64::INFINITY, f64::INFINITY), |(x, y), p| {
            (x.min(p[0]), y.min(p[1]))
        });
    let bottom = project
        .panels()
        .keys()
        .filter_map(|id| project.panel_bounding_box(*id))
        .map(|b| b.min.y)
        .fold(f64::INFINITY, f64::min);
    if !(min_x.is_finite() && min_y.is_finite() && bottom.is_finite()) {
        return DVec2::ZERO;
    }
    DVec2::new(right + GAP_MM - min_x, bottom - min_y)
}

/// A notch at the outline point nearest to `p`, if it is close enough.
fn notch_on(tx: &Transaction<'_>, panel: PanelId, p: DVec2) -> Option<NotchAdd> {
    let project = tx.project();
    let edge = project.edge_near(panel, p, NOTCH_REACH_MM)?;
    let line = project.edge_polyline(edge)?;
    // Arc length to the nearest point of the flattened edge.
    let mut walked = 0.0;
    let mut best = (f64::INFINITY, 0.0);
    for w in line.windows(2) {
        let ab = w[1] - w[0];
        let len = ab.length();
        let t = if len > 0.0 {
            ((p - w[0]).dot(ab) / (len * len)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let d = (w[0] + ab * t).distance(p);
        if d < best.0 {
            best = (d, walked + t * len);
        }
        walked += len;
    }
    Some(NotchAdd {
        edge,
        offset_mm: best.1,
    })
}
