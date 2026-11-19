//! Where a seam side runs: a path along contour edges or an internal line,
//! cut to its range (`docs/specs/02-sewing.md`).

use glam::DVec2;

use crate::id::{EdgeId, InternalLineId, PanelId};
use crate::measure::FLATTEN_TOLERANCE_MM;
use crate::model::{PathSegment, Project, SeamSide};

/// Something a seam can run along.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Track {
    Edge(EdgeId),
    Line(InternalLineId),
}

/// A stretch of a track: from `from` to `to` mm along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub track: Track,
    pub from: f64,
    pub to: f64,
}

impl Span {
    pub fn length(&self) -> f64 {
        (self.to - self.from).max(0.0)
    }
}

impl Project {
    /// An internal line as a polyline from its start (mm).
    pub fn line_polyline(&self, id: InternalLineId) -> Option<Vec<DVec2>> {
        let line = self.internal_line(id)?;
        let mut out = vec![line.start];
        let mut at = line.start;
        for seg in &line.segments {
            match seg {
                PathSegment::Line { to } => out.push(*to),
                PathSegment::Cubic { c1, c2, to } => {
                    r3d_geom::CubicBez::new(at, *c1, *c2, *to)
                        .flatten_into(FLATTEN_TOLERANCE_MM, &mut out);
                }
            }
            at = seg.end();
        }
        Some(out)
    }

    /// The length of a track (mm).
    pub fn track_length(&self, track: Track) -> Option<f64> {
        match track {
            Track::Edge(e) => self.edge_length(e),
            Track::Line(l) => Some(polyline_length(&self.line_polyline(l)?)),
        }
    }

    /// The point `s` mm along a track (clamped), and the unit direction
    /// there.
    pub fn track_point(&self, track: Track, s: f64) -> Option<(DVec2, DVec2)> {
        match track {
            Track::Edge(e) => Some(self.edge_geometry(e)?.point_at_length(s)),
            Track::Line(l) => Some(point_along(&self.line_polyline(l)?, s)),
        }
    }

    /// The piece a track belongs to.
    pub fn track_panel(&self, track: Track) -> Option<PanelId> {
        match track {
            Track::Edge(e) => self.edge(e).map(|e| e.panel),
            Track::Line(l) => self.internal_line(l).map(|l| l.panel),
        }
    }

    /// The stretches a seam side runs along, in the order of its path
    /// (before `reversed` is applied), cut to its range. `None` when it
    /// names something missing.
    pub fn side_spans(&self, side: &SeamSide) -> Option<Vec<Span>> {
        let tracks: Vec<Track> = match side.line {
            Some(l) => vec![Track::Line(l)],
            None => side.edges.iter().map(|e| Track::Edge(*e)).collect(),
        };
        let lengths: Vec<f64> = tracks
            .iter()
            .map(|t| self.track_length(*t))
            .collect::<Option<_>>()?;
        let total: f64 = lengths.iter().sum();
        let from = side.from_mm.unwrap_or(0.0).clamp(0.0, total);
        let to = side.to_mm.unwrap_or(total).clamp(from, total);
        let mut out = Vec::new();
        let mut start = 0.0;
        for (track, len) in tracks.into_iter().zip(lengths) {
            let (a, b) = ((from - start).max(0.0), (to - start).min(len));
            if b > a || (out.is_empty() && b >= a && to == from) {
                out.push(Span {
                    track,
                    from: a,
                    to: b,
                });
            }
            start += len;
        }
        Some(out)
    }

    /// The length of a seam side (mm).
    pub fn side_length(&self, side: &SeamSide) -> Option<f64> {
        Some(self.side_spans(side)?.iter().map(Span::length).sum())
    }

    /// The piece a seam side is on.
    pub fn side_panel(&self, side: &SeamSide) -> Option<PanelId> {
        match side.line {
            Some(l) => self.track_panel(Track::Line(l)),
            None => self.track_panel(Track::Edge(*side.edges.first()?)),
        }
    }

    /// A seam side as a polyline, in the direction it is walked (mm).
    pub fn side_polyline(&self, side: &SeamSide) -> Option<Vec<DVec2>> {
        let mut out: Vec<DVec2> = Vec::new();
        for span in self.side_spans(side)? {
            let full = match span.track {
                Track::Edge(e) => self.edge_polyline(e)?,
                Track::Line(l) => self.line_polyline(l)?,
            };
            for p in sub_polyline(&full, span.from, span.to) {
                if out.last().is_none_or(|q| q.distance(p) > 1e-9) {
                    out.push(p);
                }
            }
        }
        if side.reversed {
            out.reverse();
        }
        Some(out)
    }
}

/// The length of a polyline.
pub fn polyline_length(pts: &[DVec2]) -> f64 {
    pts.windows(2)
        .map(|w| match w {
            [a, b] => a.distance(*b),
            _ => 0.0,
        })
        .sum()
}

/// The point `s` along a polyline (clamped) and the unit direction there.
pub fn point_along(pts: &[DVec2], s: f64) -> (DVec2, DVec2) {
    let mut walked = 0.0;
    let mut last = (pts.first().copied().unwrap_or_default(), DVec2::X);
    for w in pts.windows(2) {
        let [a, b] = [w[0], w[1]];
        let len = a.distance(b);
        let dir = (b - a).try_normalize().unwrap_or(DVec2::X);
        if len > 0.0 && walked + len >= s {
            return (a.lerp(b, ((s - walked) / len).clamp(0.0, 1.0)), dir);
        }
        walked += len;
        last = (b, dir);
    }
    last
}

/// The part of a polyline from `from` to `to` mm along it.
pub fn sub_polyline(pts: &[DVec2], from: f64, to: f64) -> Vec<DVec2> {
    let mut out = vec![point_along(pts, from).0];
    let mut walked = 0.0;
    for w in pts.windows(2) {
        let len = w[0].distance(w[1]);
        walked += len;
        if walked > from && walked < to {
            out.push(w[1]);
        }
    }
    out.push(point_along(pts, to).0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::Transaction;
    use crate::model::{Edge, EdgeCurve, InternalLine, Panel, Point};

    #[test]
    fn a_side_runs_along_a_chain_of_edges_or_a_line() {
        let mut p = Project::new("t");
        let mut tx = Transaction::begin(&mut p);
        let panel = tx.new_panel_id();
        let corners = [[0.0, 0.0], [100.0, 0.0], [100.0, 50.0], [0.0, 50.0]];
        let pts = corners.map(|c| {
            let id = tx.new_point_id();
            tx.put_point(id, Point::new(panel, DVec2::from(c)));
            id
        });
        let edges = [0, 1, 2, 3].map(|i| {
            let id = tx.new_edge_id();
            tx.put_edge(
                id,
                Edge::new(panel, pts[i], pts[(i + 1) % 4], EdgeCurve::Line),
            );
            id
        });
        let garment = tx
            .project()
            .default_garment()
            .unwrap_or(crate::GarmentId::from_raw(1));
        tx.put_panel(
            panel,
            Panel {
                name: "P".into(),
                garment,
                contour: edges.to_vec(),
                grain_angle_deg: 90.0,
                seam_allowance_mm: 0.0,
                mirror_of: None,
                placement: None,
                symmetry: None,
                holes: Vec::new(),
                print: Default::default(),
            },
        );
        let line = tx.new_internal_line_id();
        tx.put_internal_line(
            line,
            InternalLine {
                panel,
                start: DVec2::new(10.0, 10.0),
                segments: vec![
                    PathSegment::Line {
                        to: DVec2::new(10.0, 40.0),
                    },
                    PathSegment::Line {
                        to: DVec2::new(90.0, 40.0),
                    },
                ],
            },
        );
        tx.commit();
        // The bottom from 60 mm, round the corner, up 20 mm of the right.
        let side = SeamSide {
            edges: vec![edges[0], edges[1]],
            line: None,
            from_mm: Some(60.0),
            to_mm: Some(120.0),
            reversed: false,
        };
        let spans = p.side_spans(&side).unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!((spans[0].from, spans[0].to), (60.0, 100.0));
        assert_eq!((spans[1].from, spans[1].to), (0.0, 20.0));
        assert_eq!(p.side_length(&side), Some(60.0));
        let poly = p.side_polyline(&side).unwrap();
        assert_eq!(poly.first(), Some(&DVec2::new(60.0, 0.0)));
        assert_eq!(poly.last(), Some(&DVec2::new(100.0, 20.0)));
        let on_line = SeamSide {
            edges: Vec::new(),
            line: Some(line),
            from_mm: None,
            to_mm: None,
            reversed: true,
        };
        assert_eq!(p.side_length(&on_line), Some(110.0));
        let poly = p.side_polyline(&on_line).unwrap();
        assert_eq!(poly.first(), Some(&DVec2::new(90.0, 40.0)));
        assert_eq!(p.side_panel(&on_line), Some(panel));
    }
}
