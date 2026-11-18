//! Measurements on the document (lengths in mm, areas in mm²).

use glam::DVec2;
use r3d_geom::{Aabb, CubicBez, polygon};

use crate::id::{EdgeId, InternalLineId, NotchId, PanelId};
use crate::model::{EdgeCurve, PathSegment, Project};

/// Max distance between a curve and its flattened polyline, in mm.
pub const FLATTEN_TOLERANCE_MM: f64 = 0.01;
/// Arc-length accuracy, in mm.
pub const LENGTH_TOLERANCE_MM: f64 = 1e-4;

/// Resolved geometry of one edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EdgeGeometry {
    Line(DVec2, DVec2),
    Cubic(CubicBez),
}

impl EdgeGeometry {
    pub fn length(&self) -> f64 {
        match self {
            EdgeGeometry::Line(a, b) => a.distance(*b),
            EdgeGeometry::Cubic(c) => c.arc_length(LENGTH_TOLERANCE_MM),
        }
    }

    /// Point and unit tangent at `length` mm from the start (clamped).
    pub fn point_at_length(&self, length: f64) -> (DVec2, DVec2) {
        match self {
            EdgeGeometry::Line(a, b) => {
                let d = *b - *a;
                let len = d.length();
                let t = if len > 0.0 {
                    (length / len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (a.lerp(*b, t), d.try_normalize().unwrap_or(DVec2::X))
            }
            EdgeGeometry::Cubic(c) => {
                let t = c.param_at_length(length, LENGTH_TOLERANCE_MM);
                (c.eval(t), c.tangent(t))
            }
        }
    }

    /// Append the flattened edge to `out`, excluding its start point.
    pub fn flatten_into(&self, out: &mut Vec<DVec2>) {
        match self {
            EdgeGeometry::Line(_, b) => out.push(*b),
            EdgeGeometry::Cubic(c) => c.flatten_into(FLATTEN_TOLERANCE_MM, out),
        }
    }
}

/// Resolved position of a notch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotchPlacement {
    pub pos: DVec2,
    /// Unit tangent of the edge at the notch (contour direction).
    pub tangent: DVec2,
    /// The stored offset exceeds the edge length; the notch sits at the end.
    pub clamped: bool,
}

impl Project {
    /// Geometry of an edge with resolved point positions.
    pub fn edge_geometry(&self, id: EdgeId) -> Option<EdgeGeometry> {
        let edge = self.edge(id)?;
        let a = self.point(edge.start)?.pos;
        let b = self.point(edge.end)?.pos;
        Some(match &edge.curve {
            EdgeCurve::Line => EdgeGeometry::Line(a, b),
            EdgeCurve::Cubic { c1, c2 } => EdgeGeometry::Cubic(CubicBez::new(a, *c1, *c2, b)),
        })
    }

    pub fn edge_length(&self, id: EdgeId) -> Option<f64> {
        self.edge_geometry(id).map(|g| g.length())
    }

    /// Closed polyline approximating the panel contour (closing edge implicit).
    /// An edge as a polyline from its start to its end (mm).
    pub fn edge_polyline(&self, id: EdgeId) -> Option<Vec<DVec2>> {
        let edge = self.edge(id)?;
        let mut out = vec![self.point(edge.start)?.pos];
        self.edge_geometry(id)?.flatten_into(&mut out);
        Some(out)
    }

    /// The contour edge of `panel` nearest to the pattern point `p`, if it
    /// is within `max_mm`.
    pub fn edge_near(&self, panel: PanelId, p: DVec2, max_mm: f64) -> Option<EdgeId> {
        self.panel(panel)?
            .contour
            .iter()
            .filter_map(|e| {
                let line = self.edge_polyline(*e)?;
                let d = line
                    .windows(2)
                    .map(|w| distance_to_segment(p, w[0], w[1]))
                    .fold(f64::MAX, f64::min);
                (d <= max_mm).then_some((d, *e))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, e)| e)
    }

    pub fn panel_outline(&self, id: PanelId) -> Option<Vec<DVec2>> {
        let panel = self.panel(id)?;
        let mut out = Vec::new();
        for e in &panel.contour {
            self.edge_geometry(*e)?.flatten_into(&mut out);
        }
        // Each edge pushed its end point; rotate so the loop starts at the
        // first edge's start point (= the last pushed point).
        if let Some(last) = out.pop() {
            out.insert(0, last);
        }
        Some(out)
    }

    /// Where a notch sits: position, unit tangent of its edge there, and
    /// whether its offset was clamped because the edge is shorter.
    pub fn notch_placement(&self, id: NotchId) -> Option<NotchPlacement> {
        let notch = self.notch(id)?;
        let geom = self.edge_geometry(notch.edge)?;
        let length = geom.length();
        let (pos, tangent) = geom.point_at_length(notch.offset_mm);
        Some(NotchPlacement {
            pos,
            tangent,
            clamped: notch.offset_mm > length,
        })
    }

    /// Flattened polyline of an internal line.
    pub fn internal_line_polyline(&self, id: InternalLineId) -> Option<Vec<DVec2>> {
        let line = self.internal_line(id)?;
        let mut out = vec![line.start];
        let mut from = line.start;
        for seg in &line.segments {
            match seg {
                PathSegment::Line { to } => out.push(*to),
                PathSegment::Cubic { c1, c2, to } => {
                    CubicBez::new(from, *c1, *c2, *to).flatten_into(FLATTEN_TOLERANCE_MM, &mut out)
                }
            }
            from = seg.end();
        }
        Some(out)
    }

    /// A piece's holes, each a closed polyline (clockwise).
    pub fn hole_outlines(&self, id: PanelId) -> Vec<Vec<DVec2>> {
        let Some(panel) = self.panel(id) else {
            return Vec::new();
        };
        panel
            .holes
            .iter()
            .filter_map(|h| {
                let mut ring = Vec::new();
                for e in h {
                    let line = self.edge_polyline(*e)?;
                    ring.extend(line.iter().take(line.len().saturating_sub(1)));
                }
                Some(ring)
            })
            .collect()
    }

    /// The area of a piece's cloth: its outline's, less its holes'.
    pub fn panel_area(&self, id: PanelId) -> Option<f64> {
        let outer = polygon::signed_area(&self.panel_outline(id)?).abs();
        let holes: f64 = self
            .hole_outlines(id)
            .iter()
            .map(|h| polygon::signed_area(h).abs())
            .sum();
        Some(outer - holes)
    }

    pub fn panel_perimeter(&self, id: PanelId) -> Option<f64> {
        let panel = self.panel(id)?;
        panel.contour.iter().map(|e| self.edge_length(*e)).sum()
    }

    pub fn panel_bounding_box(&self, id: PanelId) -> Option<Aabb> {
        polygon::bounding_box(&self.panel_outline(id)?)
    }

    /// Whether the panel contour does not cross itself.
    pub fn panel_is_simple(&self, id: PanelId) -> Option<bool> {
        self.panel_outline(id).map(|o| polygon::is_simple(&o))
    }
}

fn distance_to_segment(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}
