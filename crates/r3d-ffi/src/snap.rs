//! Snapping on the board: a pointer near something worth landing on (a
//! corner, an edge's middle, an edge, a construction line, a symmetry
//! axis) or level with a point it is drawn from lands exactly there, and
//! the board says why (`kind`, shown as a hint).

use r3d_core::{PointId, Project};
use r3d_geom::DVec2;

/// Where a pointer lands, and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Landing {
    pub at: DVec2,
    /// `point`, `midpoint`, `edge`, `guide`, `axis`, `horizontal`,
    /// `vertical`, or `free`.
    pub kind: &'static str,
    /// The point it is level with (`horizontal`, `vertical`).
    pub from: Option<DVec2>,
}

/// The nearest point to `p` on the line through `a` and `b`.
fn on_line(p: DVec2, a: DVec2, b: DVec2) -> DVec2 {
    let d = b - a;
    let len2 = d.length_squared();
    if len2 == 0.0 {
        return a;
    }
    a + d * ((p - a).dot(d) / len2)
}

/// How far `p` is from the line through `a` and `b`.
pub(crate) fn distance_to_line(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    on_line(p, a, b).distance(p)
}

/// The nearest point to `p` on a polyline.
fn on_polyline(p: DVec2, pts: &[DVec2]) -> Option<DVec2> {
    pts.windows(2)
        .filter_map(|w| match w {
            [a, b] => {
                let d = *b - *a;
                let len2 = d.length_squared();
                let t = if len2 > 0.0 {
                    ((p - *a).dot(d) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                Some(*a + d * t)
            }
            _ => None,
        })
        .min_by(|x, y| x.distance(p).total_cmp(&y.distance(p)))
}

/// Where `p` lands within `max_mm`: on a corner (but `skip`), an edge's
/// middle, level with one of `align` (horizontally or vertically), on an
/// edge, a construction line or a symmetry axis; otherwise where it is.
pub(crate) fn land(
    project: &Project,
    p: DVec2,
    max_mm: f64,
    align: &[DVec2],
    skip: Option<PointId>,
) -> Landing {
    let free = Landing {
        at: p,
        kind: "free",
        from: None,
    };
    let near = |q: DVec2| q.distance(p) <= max_mm;
    // 1. Corners, then the middles of edges.
    let corner = project
        .points()
        .iter()
        .filter(|(id, _)| Some(**id) != skip)
        .map(|(_, pt)| pt.pos)
        .filter(|q| near(*q))
        .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)));
    if let Some(at) = corner {
        return Landing {
            at,
            kind: "point",
            from: None,
        };
    }
    let polylines: Vec<Vec<DVec2>> = project
        .edges()
        .iter()
        .filter(|(_, e)| skip.is_none_or(|s| e.start != s && e.end != s))
        .filter_map(|(id, _)| project.edge_polyline(*id))
        .collect();
    let middle = polylines
        .iter()
        .filter_map(|pts| {
            let geom_mid = halfway(pts)?;
            near(geom_mid).then_some(geom_mid)
        })
        .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)));
    if let Some(at) = middle {
        return Landing {
            at,
            kind: "midpoint",
            from: None,
        };
    }
    // 2. Level with a point drawn from.
    let mut level: Option<Landing> = None;
    for a in align {
        for (kind, at) in [
            ("horizontal", DVec2::new(p.x, a.y)),
            ("vertical", DVec2::new(a.x, p.y)),
        ] {
            if near(at) && level.is_none_or(|l| at.distance(p) < l.at.distance(p)) {
                level = Some(Landing {
                    at,
                    kind,
                    from: Some(*a),
                });
            }
        }
    }
    // 3. Lines: edges, construction lines, axes.
    let mut lines: Vec<(&'static str, DVec2)> = Vec::new();
    for pts in &polylines {
        if let Some(q) = on_polyline(p, pts) {
            lines.push(("edge", q));
        }
    }
    for g in project.guides().values() {
        lines.push(("guide", on_line(p, g.a, g.b)));
    }
    for panel in project.panels().values() {
        if let Some(sym) = &panel.symmetry {
            lines.push(("axis", on_line(p, sym.origin, sym.origin + sym.dir)));
        }
    }
    let line = lines
        .into_iter()
        .filter(|(_, q)| near(*q))
        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)));
    match (level, line) {
        // Level with a point and on a line: where they cross, if near.
        (Some(l), Some((kind, q))) => {
            let crossing = match l.kind {
                "horizontal" => cross_level(p, q, l.at.y, true),
                _ => cross_level(p, q, l.at.x, false),
            };
            match crossing.filter(|c| near(*c)) {
                Some(at) => Landing {
                    at,
                    kind,
                    from: l.from,
                },
                None => l,
            }
        }
        (Some(l), None) => l,
        (None, Some((kind, at))) => Landing {
            at,
            kind,
            from: None,
        },
        (None, None) => free,
    }
}

/// Where the line through `p` and its foot `q` on a line (that line is
/// perpendicular to `q − p`) meets `y = level` (or `x = level`).
fn cross_level(p: DVec2, q: DVec2, level: f64, horizontal: bool) -> Option<DVec2> {
    // The line itself: through q, perpendicular to (q − p).
    let n = q - p;
    let dir = if n.length_squared() > 1e-18 {
        DVec2::new(-n.y, n.x)
    } else {
        return Some(if horizontal {
            DVec2::new(p.x, level)
        } else {
            DVec2::new(level, p.y)
        });
    };
    if horizontal {
        (dir.y.abs() > 1e-12).then(|| q + dir * ((level - q.y) / dir.y))
    } else {
        (dir.x.abs() > 1e-12).then(|| q + dir * ((level - q.x) / dir.x))
    }
}

/// The point halfway along a polyline.
fn halfway(pts: &[DVec2]) -> Option<DVec2> {
    let total: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut walked = 0.0;
    for w in pts.windows(2) {
        let len = w[0].distance(w[1]);
        if walked + len >= total / 2.0 && len > 0.0 {
            return Some(w[0].lerp(w[1], (total / 2.0 - walked) / len));
        }
        walked += len;
    }
    pts.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3d_commands::{Session, Step};
    use serde_json::json;

    fn square() -> Session {
        let mut s = Session::new(Project::new("t"));
        let step = json!({"type": "panel.create_rect", "width": 100, "height": 100});
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
        let step = json!({"type": "guide.add", "a": [0, 300], "b": [10, 300]});
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
        s
    }

    #[test]
    fn lands_on_corners_middles_lines_and_levels() {
        let s = square();
        let p = s.project();
        let l = land(p, DVec2::new(98.0, 3.0), 5.0, &[], None);
        assert_eq!((l.kind, l.at), ("point", DVec2::new(100.0, 0.0)));
        let l = land(p, DVec2::new(52.0, 2.0), 5.0, &[], None);
        assert_eq!((l.kind, l.at), ("midpoint", DVec2::new(50.0, 0.0)));
        let l = land(p, DVec2::new(30.0, 3.0), 5.0, &[], None);
        assert_eq!((l.kind, l.at), ("edge", DVec2::new(30.0, 0.0)));
        let l = land(p, DVec2::new(400.0, 302.0), 5.0, &[], None);
        assert_eq!((l.kind, l.at), ("guide", DVec2::new(400.0, 300.0)));
        let l = land(
            p,
            DVec2::new(400.0, 203.0),
            5.0,
            &[DVec2::new(250.0, 200.0)],
            None,
        );
        assert_eq!((l.kind, l.at), ("horizontal", DVec2::new(400.0, 200.0)));
        let l = land(p, DVec2::new(400.0, 250.0), 5.0, &[], None);
        assert_eq!(l.kind, "free");
        // Level with a point and on the guide: where they cross.
        let l = land(
            p,
            DVec2::new(252.0, 298.0),
            5.0,
            &[DVec2::new(250.0, 0.0)],
            None,
        );
        assert_eq!((l.kind, l.at), ("guide", DVec2::new(250.0, 300.0)));
    }
}
