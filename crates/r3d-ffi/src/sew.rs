//! The Sew tool on the board: the outline edge or internal line under the
//! pointer, and where along it a seam side would start or end, snapped to
//! the track's ends, its notches and the ends of the seams already on it
//! (`docs/specs/02-sewing.md`, `docs/specs/21-construction-details.md`).

use r3d_core::seam::{Track, polyline_length};
use r3d_core::{EdgeId, PanelId, Project};
use r3d_geom::DVec2;

/// A place on a track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Spot {
    pub track: Track,
    pub panel: PanelId,
    /// How far along the track (mm from its start).
    pub offset: f64,
    /// The track's length (mm).
    pub length: f64,
    /// Where that is on the board (mm).
    pub at: DVec2,
}

/// A track as a polyline from its start (mm).
fn polyline(project: &Project, track: Track) -> Option<Vec<DVec2>> {
    match track {
        Track::Edge(e) => project.edge_polyline(e),
        Track::Line(l) => project.line_polyline(l),
    }
}

/// The point of `pts` nearest to `p`: how far it is, and how far along.
fn nearest(pts: &[DVec2], p: DVec2) -> Option<(f64, f64)> {
    let mut walked = 0.0;
    let mut best: Option<(f64, f64)> = None;
    for w in pts.windows(2) {
        let [a, b] = [w[0], w[1]];
        let ab = b - a;
        let len = ab.length();
        let t = if len > 0.0 {
            ((p - a).dot(ab) / (len * len)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let d = p.distance(a + ab * t);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, walked + len * t));
        }
        walked += len;
    }
    best
}

/// Where a seam side may stop on `track`: its ends, its notches and the
/// ends of the seam stretches already on it.
fn stops(project: &Project, track: Track, length: f64) -> Vec<f64> {
    let mut out = vec![0.0, length];
    if let Track::Edge(e) = track {
        out.extend(
            project
                .notches()
                .values()
                .filter(|n| n.edge == e)
                .map(|n| n.offset_mm.min(length)),
        );
    }
    for seam in project.seams().values() {
        for side in [&seam.a, &seam.b] {
            for span in project.side_spans(side).unwrap_or_default() {
                if span.track == track {
                    out.extend([span.from, span.to]);
                }
            }
        }
    }
    out
}

/// The spot on `track` nearest to `p`, its offset snapped to a stop
/// within `snap_mm`.
pub(crate) fn spot_on(project: &Project, track: Track, p: DVec2, snap_mm: f64) -> Option<Spot> {
    let pts = polyline(project, track)?;
    let (_, raw) = nearest(&pts, p)?;
    let length = polyline_length(&pts);
    let offset = stops(project, track, length)
        .into_iter()
        .map(|s| (s, (s - raw).abs()))
        .filter(|(_, d)| *d <= snap_mm)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(raw, |(s, _)| s);
    Some(Spot {
        track,
        panel: project.track_panel(track)?,
        offset,
        length,
        at: r3d_core::seam::point_along(&pts, offset).0,
    })
}

/// The outline edge or internal line nearest to `p` within `max_mm`, and
/// the spot on it.
pub(crate) fn spot_near(project: &Project, p: DVec2, max_mm: f64) -> Option<Spot> {
    let edges = project.panels().values().flat_map(|panel| {
        panel
            .contour
            .iter()
            .chain(panel.holes.iter().flatten())
            .map(|e| Track::Edge(*e))
    });
    let lines = project.internal_lines().keys().map(|l| Track::Line(*l));
    let track = edges
        .chain(lines)
        .filter_map(|t| Some((nearest(&polyline(project, t)?, p)?.0, t)))
        .filter(|(d, _)| *d <= max_mm)
        .min_by(|a, b| a.0.total_cmp(&b.0))?
        .1;
    spot_on(project, track, p, max_mm)
}

/// Edges of one piece put in their order around it, so that a run picked
/// in any order is sewn as one side. Edges of other pieces, or a set that
/// is not one run, are left in the order given (the seam then says why).
pub(crate) fn chain_order(project: &Project, edges: &[EdgeId]) -> Vec<EdgeId> {
    let contour = edges
        .first()
        .and_then(|e| project.edge(*e))
        .and_then(|e| project.panel(e.panel))
        .map(|p| p.contour.clone())
        .unwrap_or_default();
    let n = contour.len();
    let picked = |i: usize| contour.get(i % n.max(1)).is_some_and(|e| edges.contains(e));
    // The run starts at a picked edge whose predecessor is not picked.
    let start = (0..n).find(|i| picked(*i) && !picked(i + n - 1));
    let ordered: Vec<EdgeId> = match start {
        Some(s) => (s..s + n)
            .take_while(|i| picked(*i))
            .filter_map(|i| contour.get(i % n).copied())
            .collect(),
        None => Vec::new(),
    };
    if ordered.len() == edges.len() {
        ordered
    } else {
        edges.to_vec()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use r3d_commands::{Session, Step};
    use serde_json::json;

    fn run(s: &mut Session, step: serde_json::Value) {
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
    }

    fn rect() -> (Session, Vec<EdgeId>) {
        let mut s = Session::new(Project::new("t"));
        run(
            &mut s,
            json!({"type": "panel.create_rect", "width": 100, "height": 200}),
        );
        let edges = s
            .project()
            .panels()
            .values()
            .next()
            .unwrap()
            .contour
            .clone();
        (s, edges)
    }

    #[test]
    fn spot_snaps_to_ends_and_notches() {
        let (mut s, edges) = rect();
        let e = edges[0];
        let a = s.project().edge_polyline(e).unwrap();
        let mid = a[0].lerp(a[1], 0.5);
        let len = a[0].distance(a[1]);
        let free = spot_near(s.project(), mid + DVec2::new(0.3, 0.3), 2.0).unwrap();
        assert_eq!(free.track, Track::Edge(e));
        assert!((free.offset - len / 2.0).abs() < 1.0);
        let end = spot_on(s.project(), Track::Edge(e), a[0].lerp(a[1], 0.02), 5.0).unwrap();
        assert_eq!(end.offset, 0.0);
        let at = len / 2.0 - 8.0;
        run(
            &mut s,
            json!({"type": "notch.add", "edge": e, "offset_mm": at}),
        );
        let snapped = spot_on(s.project(), Track::Edge(e), mid, 12.0).unwrap();
        assert!((snapped.offset - at).abs() < 1e-9);
        let centre = a[0].lerp(a[1], 0.5) + (a[1] - a[0]).perp() * 0.3;
        assert!(spot_near(s.project(), centre, 2.0).is_none());
    }

    #[test]
    fn chains_are_put_in_contour_order() {
        let (s, edges) = rect();
        let p = s.project();
        // Across the contour's start: the last edge, then the first.
        assert_eq!(
            chain_order(p, &[edges[0], edges[3]]),
            vec![edges[3], edges[0]]
        );
        assert_eq!(
            chain_order(p, &[edges[2], edges[1]]),
            vec![edges[1], edges[2]]
        );
        // Not one run: as given.
        assert_eq!(
            chain_order(p, &[edges[2], edges[0]]),
            vec![edges[2], edges[0]]
        );
    }
}
