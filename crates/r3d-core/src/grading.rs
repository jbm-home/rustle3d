//! Graded sizes: the pattern in each size of its range
//! (`docs/specs/18-grading.md`).
//!
//! The document holds the pattern in its base size. Each point's grade
//! says where it is in every other size (an offset per size, as in a
//! grade rule table); the rest of a piece follows its points: a curve's
//! control points keep their direction from their end point and scale
//! with the edge's chord, so corners keep their angles, unless the edge
//! grades them too (templates do, so their curves are exact in every
//! size); internal lines and pins move with the points around them.

use std::borrow::Cow;

use glam::DVec2;

use crate::id::{EdgeId, PanelId};
use crate::model::{Body, EdgeCurve, Grading, PathSegment, Point, Project};

/// Below this chord (mm), a curve's handles are not scaled.
const MIN_CHORD_MM: f64 = 1e-6;
/// Smallest body measurement a size can have (cm).
const MIN_BODY_CM: f64 = 1.0;

impl Grading {
    /// The index of the size called `name`.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.sizes.iter().position(|s| s.name == name)
    }
}

impl Point {
    /// Offset of the point in size `i` from its base position (mm).
    pub fn offset(&self, i: usize) -> DVec2 {
        self.grade.get(i).copied().unwrap_or(DVec2::ZERO)
    }
}

/// How far each point of a piece moves; inside the piece, by inverse
/// squared distance to its points.
struct Field(Vec<(DVec2, DVec2)>);

impl Field {
    fn at(&self, p: DVec2) -> DVec2 {
        let mut sum = DVec2::ZERO;
        let mut weights = 0.0;
        for (at, offset) in &self.0 {
            let w = 1.0 / (at.distance_squared(p) + 1.0);
            sum += *offset * w;
            weights += w;
        }
        if weights > 0.0 { sum / weights } else { sum }
    }
}

impl Project {
    /// The body of size `i`: the avatar's, changed by the size's
    /// differences. `None` without an avatar or a size `i`.
    pub fn size_body(&self, i: usize) -> Option<Body> {
        let body = self.avatar.as_ref()?.body;
        let size = self.grading.as_ref()?.sizes.get(i)?;
        let at_least = |v: f64| v.max(MIN_BODY_CM);
        Some(Body {
            masculinity: body.masculinity,
            height_cm: at_least(body.height_cm + size.height_cm),
            bust_cm: at_least(body.bust_cm + size.bust_cm),
            waist_cm: at_least(body.waist_cm + size.waist_cm),
            hips_cm: at_least(body.hips_cm + size.hips_cm),
        })
    }

    /// The grade of a new point at `pos` inside or on `panel`: moved like
    /// the piece's points around it. Empty when none of them is graded.
    pub fn grade_at(&self, panel: PanelId, pos: DVec2) -> Vec<DVec2> {
        let Some(n) = self.grading.as_ref().map(|g| g.sizes.len()) else {
            return Vec::new();
        };
        let points: Vec<&Point> = self.points.values().filter(|p| p.panel == panel).collect();
        if points.iter().all(|p| p.grade.is_empty()) {
            return Vec::new();
        }
        (0..n)
            .map(|i| Field(points.iter().map(|p| (p.pos, p.offset(i))).collect()).at(pos))
            .collect()
    }

    /// The grade of a new point `fraction` (0–1) of the way along `edge`:
    /// between its end points' grades.
    pub fn grade_along(&self, edge: EdgeId, fraction: f64) -> Vec<DVec2> {
        let Some(n) = self.grading.as_ref().map(|g| g.sizes.len()) else {
            return Vec::new();
        };
        let ends = self
            .edges
            .get(&edge)
            .and_then(|e| Some((self.points.get(&e.start)?, self.points.get(&e.end)?)));
        let Some((a, b)) = ends else {
            return Vec::new();
        };
        if a.grade.is_empty() && b.grade.is_empty() {
            return Vec::new();
        }
        (0..n)
            .map(|i| a.offset(i).lerp(b.offset(i), fraction))
            .collect()
    }

    /// A point of `panel` in the shown size (a click on the draped
    /// garment) brought back to the base size, the pattern being edited:
    /// moved back like the piece's points around it.
    pub fn from_shown(&self, panel: PanelId, pos: DVec2) -> DVec2 {
        let Some(g) = self.grading.as_ref().filter(|g| g.shown != g.base) else {
            return pos;
        };
        let field = Field(
            self.points
                .values()
                .filter(|p| p.panel == panel)
                .map(|p| (p.pos + p.offset(g.shown), p.offset(g.shown)))
                .collect(),
        );
        pos - field.at(pos)
    }

    /// The body the garment is draped on: the avatar's in the shown size.
    pub fn shown_body(&self) -> Option<Body> {
        match &self.grading {
            Some(g) if g.shown != g.base => self.size_body(g.shown),
            _ => self.avatar.as_ref().map(|a| a.body),
        }
    }

    /// The project as draped and shown in 3D: in its shown size.
    pub fn as_shown(&self) -> Cow<'_, Project> {
        match &self.grading {
            Some(g) if g.shown != g.base => Cow::Owned(self.at_size(g.shown)),
            _ => Cow::Borrowed(self),
        }
    }

    /// The project in size `i` of its range, as if it had been drawn in
    /// it: every point at its graded position, curves, internal lines and
    /// pins following them, the avatar in that size's body, and the range
    /// based on `i`. Without a range or a size `i`: a copy.
    pub fn at_size(&self, i: usize) -> Project {
        let mut out = self.clone();
        let Some(grading) = self.grading.as_ref().filter(|g| i < g.sizes.len()) else {
            return out;
        };
        if i == grading.base {
            return out;
        }
        for point in out.points.values_mut() {
            let shift = point.offset(i);
            point.pos += shift;
            for g in &mut point.grade {
                *g -= shift;
            }
        }
        for edge in out.edges.values_mut() {
            let EdgeCurve::Cubic { c1, c2 } = &mut edge.curve else {
                continue;
            };
            if let Some([g1, g2]) = edge.grade.get(i).copied() {
                // Graded handles.
                *c1 += g1;
                *c2 += g2;
                for g in &mut edge.grade {
                    g[0] -= g1;
                    g[1] -= g2;
                }
                continue;
            }
            let (Some(a), Some(b)) = (self.points.get(&edge.start), self.points.get(&edge.end))
            else {
                continue;
            };
            let (da, db) = (a.offset(i), b.offset(i));
            let chord = a.pos.distance(b.pos);
            let scale = if chord > MIN_CHORD_MM {
                (b.pos + db).distance(a.pos + da) / chord
            } else {
                1.0
            };
            *c1 = a.pos + da + (*c1 - a.pos) * scale;
            *c2 = b.pos + db + (*c2 - b.pos) * scale;
        }
        let field = |panel| {
            Field(
                self.points
                    .values()
                    .filter(|p| p.panel == panel)
                    .map(|p| (p.pos, p.offset(i)))
                    .collect(),
            )
        };
        for line in out.internal_lines.values_mut() {
            let f = field(line.panel);
            line.start += f.at(line.start);
            for seg in &mut line.segments {
                match seg {
                    PathSegment::Line { to } => *to += f.at(*to),
                    PathSegment::Cubic { c1, c2, to } => {
                        *c1 += f.at(*c1);
                        *c2 += f.at(*c2);
                        *to += f.at(*to);
                    }
                }
            }
        }
        for pin in out.pins.values_mut() {
            pin.at += field(pin.panel).at(pin.at);
        }
        if let Some(avatar) = &mut out.avatar
            && let Some(body) = self.size_body(i)
        {
            avatar.body = body;
            avatar.preset = None;
        }
        if let Some(g) = &mut out.grading {
            let base = g.sizes.get(i).cloned();
            if let Some(base) = base {
                for s in &mut g.sizes {
                    s.height_cm -= base.height_cm;
                    s.bust_cm -= base.bust_cm;
                    s.waist_cm -= base.waist_cm;
                    s.hips_cm -= base.hips_cm;
                }
            }
            g.base = i;
        }
        out
    }
}
