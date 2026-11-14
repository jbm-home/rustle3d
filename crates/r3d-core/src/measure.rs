//! Measurements on the document (lengths in mm, areas in mm²).

use glam::DVec2;
use r3d_geom::{Aabb, CubicBez, polygon};

use crate::id::{EdgeId, PanelId};
use crate::model::{EdgeCurve, Project};

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

    /// Append the flattened edge to `out`, excluding its start point.
    pub fn flatten_into(&self, out: &mut Vec<DVec2>) {
        match self {
            EdgeGeometry::Line(_, b) => out.push(*b),
            EdgeGeometry::Cubic(c) => c.flatten_into(FLATTEN_TOLERANCE_MM, out),
        }
    }
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

    pub fn panel_area(&self, id: PanelId) -> Option<f64> {
        self.panel_outline(id)
            .map(|o| polygon::signed_area(&o).abs())
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
