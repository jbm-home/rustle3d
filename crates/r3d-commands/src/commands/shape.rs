//! Round shapes: ellipses, arc sectors (circle skirts), circular arcs on
//! an edge. Arcs are cubic Béziers of at most 90° each, with handles at
//! `4/3 · tan(θ/4)` of the radius (radial error under 0.03 % of the
//! radius; Riškus, "Approximation of a cubic Bézier curve by circular
//! arcs and vice versa", Information Technology and Control 35(4), 2006).

use std::f64::consts::{FRAC_PI_2, TAU};

use r3d_core::{Edge, EdgeCurve, EdgeId, Notch, Panel, Point, Transaction};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::panel::{Outline, create_panel, default_grain};
use super::{Vec2, editable_edge_panel, not_sewn, v};
use crate::error::{CommandError, Invalid, finite, non_blank, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// The cubic arcs of a circle (centre `c`, radius `r`, scaled by `scale`
/// on x and y) from angle `a0` to `a1` (radians, either way round): the
/// start of each arc with its handles.
fn arcs(c: DVec2, r: f64, scale: DVec2, a0: f64, a1: f64) -> Vec<(DVec2, DVec2, DVec2)> {
    let sweep = a1 - a0;
    let n = ((sweep.abs() / FRAC_PI_2) - 1e-9).ceil().max(1.0) as usize;
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let at = |a: f64| c + DVec2::new(a.cos(), a.sin()) * r * scale;
    let tangent = |a: f64| DVec2::new(-a.sin(), a.cos()) * r * scale;
    (0..n)
        .map(|i| {
            let (s, e) = (a0 + step * i as f64, a0 + step * (i + 1) as f64);
            (at(s), at(s) + tangent(s) * k, at(e) - tangent(e) * k)
        })
        .collect()
}

/// Create an elliptical panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCreateEllipse {
    /// Panel name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Centre on the board, in mm. Defaults to `[0, 0]`.
    #[serde(default)]
    pub centre: Vec2,
    /// Half width and half height (mm, > 0); equal for a circle.
    pub radii: Vec2,
    /// Grainline angle in degrees from +X. Defaults to 90 (vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
}

impl Described for PanelCreateEllipse {
    const SUMMARY: &'static str = "Create an elliptical (or, with equal radii, circular) panel: four curved edges between the points at the right, top, left and bottom. Returns the panel, then its 4 points, then its 4 edges.";
}

impl CommandSpec for PanelCreateEllipse {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("centre", &self.centre)?;
        positive("radii", self.radii[0])?;
        positive("radii", self.radii[1])?;
        let outline: Outline = arcs(v(self.centre), 1.0, v(self.radii), 0.0, TAU)
            .into_iter()
            .map(|(a, c1, c2)| (a, EdgeCurve::Cubic { c1, c2 }))
            .collect();
        create_panel(tx, self.name.as_deref(), outline, self.grain_angle_deg)
    }
}

/// Create an arc sector: a ring cut between two radii.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelCreateSector {
    /// Panel name. Defaults to "Panel N".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Centre of the circles on the board (mm). Defaults to `[0, 0]`; the
    /// piece hangs below it.
    #[serde(default)]
    pub centre: Vec2,
    /// Length of the inner arc (mm, > 0): the waist this piece covers.
    pub inner_length_mm: f64,
    /// Distance from the inner to the outer arc (mm, > 0): the skirt's
    /// length.
    pub length_mm: f64,
    /// The angle the piece covers (degrees, more than 0 and less than
    /// 360): 180 for a half circle, 90 for a quarter. A full circle skirt
    /// is two half circles, or four quarters, sewn together.
    pub sweep_deg: f64,
    /// Grainline angle in degrees from +X. Defaults to 90 (vertical).
    #[serde(default = "default_grain")]
    pub grain_angle_deg: f64,
}

impl Described for PanelCreateSector {
    const SUMMARY: &'static str = "Create an arc sector, the piece of a circle skirt: the inner arc is `inner_length_mm` long (the waist it covers), the outer arc `length_mm` further out (the hem), over `sweep_deg` (e.g. 180 for half a circle skirt). The radius comes from the waist: inner_length / sweep. The piece hangs below `centre`, its waist on top. Returns the panel, then its points (the hem from left to right, then the waist from right to left), then its edges.";
}

impl CommandSpec for PanelCreateSector {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("centre", &self.centre)?;
        positive("inner_length_mm", self.inner_length_mm)?;
        positive("length_mm", self.length_mm)?;
        finite("sweep_deg", &[self.sweep_deg])?;
        if !(self.sweep_deg > 0.0 && self.sweep_deg < 360.0) {
            return Err(CommandError::invalid(
                "sweep_deg",
                Invalid::Range(0.0, 360.0),
            ));
        }
        if let Some(n) = &self.name {
            non_blank("name", n)?;
        }
        let sweep = self.sweep_deg.to_radians();
        let r = self.inner_length_mm / sweep;
        let outer = r + self.length_mm;
        let c = v(self.centre);
        // Hanging down: centred on −90°.
        let (a0, a1) = (-FRAC_PI_2 - sweep / 2.0, -FRAC_PI_2 + sweep / 2.0);
        let mut outline: Outline = Vec::new();
        for (a, c1, c2) in arcs(c, outer, DVec2::ONE, a0, a1) {
            outline.push((a, EdgeCurve::Cubic { c1, c2 }));
        }
        // Up the side, then the waist back, then down the other side.
        let up = c + DVec2::new(a1.cos(), a1.sin()) * outer;
        outline.push((up, EdgeCurve::Line));
        for (a, c1, c2) in arcs(c, r, DVec2::ONE, a1, a0) {
            outline.push((a, EdgeCurve::Cubic { c1, c2 }));
        }
        // The waist's last arc ends where the side down starts.
        let down = c + DVec2::new(a0.cos(), a0.sin()) * r;
        outline.push((down, EdgeCurve::Line));
        create_panel(tx, self.name.as_deref(), outline, self.grain_angle_deg)
    }
}

/// Turn an edge into a circular arc.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeSetArc {
    pub edge: EdgeId,
    /// How far the arc's middle is from the straight line between the
    /// edge's ends (mm, not 0): positive bulges outward (to the right of
    /// the edge, as outlines turn counter-clockwise), negative inward.
    pub bulge_mm: f64,
}

impl Described for EdgeSetArc {
    const SUMMARY: &'static str = "Make an edge a circular arc through its two ends, bulging `bulge_mm` from the straight line (positive outward). An arc over 90° is split into edges of at most 90° (new points between them); then the constraints on the edge are removed and its notches spread along the arc. Returns the new points and edges, if any.";
}

impl CommandSpec for EdgeSetArc {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        finite("bulge_mm", &[self.bulge_mm])?;
        if self.bulge_mm == 0.0 {
            return Err(CommandError::invalid("bulge_mm", Invalid::NotPositive));
        }
        let (edge, panel) = editable_edge_panel(tx, self.edge)?;
        not_sewn(tx, self.edge)?;
        let p = tx.project();
        let pos = |id| p.point(id).map(|p: &Point| p.pos).unwrap_or_default();
        let (a, b) = (pos(edge.start), pos(edge.end));
        let chord = b - a;
        let c = chord.length();
        if c == 0.0 {
            return Err(CommandError::invalid("edge", Invalid::ZeroVector));
        }
        let s = self.bulge_mm;
        // Radius and centre: the centre is on the chord's bisector, on the
        // side away from the bulge (or beyond the chord past a half circle).
        let r = (c * c / 4.0 + s * s) / (2.0 * s.abs());
        let right = DVec2::new(chord.y, -chord.x) / c;
        let out = right * s.signum();
        let centre = (a + b) / 2.0 + out * (s.abs() - r);
        let a0 = (a - centre).to_angle();
        let half = (c / 2.0 / r).clamp(-1.0, 1.0).asin();
        let sweep = if s.abs() > c / 2.0 {
            TAU - 2.0 * half
        } else {
            2.0 * half
        };
        // Walk from a to b round the bulge: a bulge on the right turns
        // about a centre on the left, counter-clockwise.
        let sweep = if s > 0.0 { sweep } else { -sweep };
        let pieces = arcs(centre, r, DVec2::ONE, a0, a0 + sweep);
        let n = pieces.len();
        let total = r * sweep.abs();
        let mut created = Vec::new();
        if n > 1 {
            super::drop_constraints_on(tx, &[self.edge]);
        }
        // Points between the pieces, and the edges after the first.
        let mut points = vec![edge.start];
        for (start, _, _) in pieces.iter().skip(1) {
            let id = tx.new_point_id();
            tx.put_point(id, Point::new(edge.panel, *start));
            points.push(id);
            created.push(id.into());
        }
        points.push(edge.end);
        let mut edges = vec![self.edge];
        for _ in 1..n {
            let id = tx.new_edge_id();
            edges.push(id);
            created.push(id.into());
        }
        for (k, (id, (_, c1, c2))) in edges.iter().zip(&pieces).enumerate() {
            let (Some(start), Some(end)) = (points.get(k), points.get(k + 1)) else {
                continue;
            };
            tx.put_edge(
                *id,
                Edge {
                    start: *start,
                    end: *end,
                    curve: EdgeCurve::Cubic { c1: *c1, c2: *c2 },
                    grade: Vec::new(),
                    ..edge.clone()
                },
            );
        }
        if n > 1 {
            let mut contour = panel.contour.clone();
            let at = contour
                .iter()
                .position(|e| *e == self.edge)
                .map_or(contour.len(), |i| i + 1);
            for (k, id) in edges.iter().skip(1).enumerate() {
                contour.insert((at + k).min(contour.len()), *id);
            }
            tx.put_panel(edge.panel, Panel { contour, ..panel });
            // Notches keep their distance from the start, along the arc.
            let piece = total / n as f64;
            let notches: Vec<_> = tx
                .project()
                .notches()
                .iter()
                .filter(|(_, nt)| nt.edge == self.edge)
                .map(|(id, nt)| (*id, nt.offset_mm))
                .collect();
            for (id, offset) in notches {
                let k = ((offset / piece).floor() as usize).min(n - 1);
                if let Some(e) = edges.get(k) {
                    tx.put_notch(
                        id,
                        Notch {
                            edge: *e,
                            offset_mm: offset - piece * k as f64,
                        },
                    );
                }
            }
        }
        Ok(CommandOutput { created })
    }
}
