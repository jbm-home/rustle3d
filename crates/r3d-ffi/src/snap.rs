//! Snapping on the board: a pointer near something worth landing on (a
//! corner, an edge's middle, where two lines cross, an edge, a
//! construction line, a symmetry axis) or level with a point it is drawn from lands exactly there, and
//! the board says why (`kind`, shown as a hint).

use r3d_core::{PointId, Project};
use r3d_geom::DVec2;

/// Where a pointer lands, and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Landing {
    pub at: DVec2,
    /// `point`, `midpoint`, `crossing`, `edge`, `guide`, `axis`,
    /// `horizontal`, `vertical`, or `free`.
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

/// Where the segments `a0`–`a1` and `b0`–`b1` cross (`a_line`, `b_line`:
/// that one is a whole line, not a segment).
fn crossing(
    (a0, a1, a_line): (DVec2, DVec2, bool),
    (b0, b1, b_line): (DVec2, DVec2, bool),
) -> Option<DVec2> {
    let (da, db) = (a1 - a0, b1 - b0);
    let den = da.perp_dot(db);
    if den.abs() < 1e-12 * da.length() * db.length() {
        return None;
    }
    let t = (b0 - a0).perp_dot(db) / den;
    let u = (b0 - a0).perp_dot(da) / den;
    let inside = |k: f64, line: bool| line || (-1e-9..=1.0 + 1e-9).contains(&k);
    (inside(t, a_line) && inside(u, b_line)).then(|| a0 + da * t)
}

/// Where two of the lines near `p` cross, if within `max_mm`: two
/// construction lines, a line and an edge, two edges (of different pieces
/// or apart on one; an edge's own corners are corners), symmetry axes.
fn crossing_near(
    p: DVec2,
    max_mm: f64,
    polylines: &[Vec<DVec2>],
    lines: &[(DVec2, DVec2)],
) -> Option<DVec2> {
    // Each candidate: a segment or a whole line, and what it belongs to.
    let mut parts: Vec<((DVec2, DVec2, bool), usize)> = Vec::new();
    for (k, pts) in polylines.iter().enumerate() {
        for w in pts.windows(2) {
            if let [a, b] = w
                && on_polyline(p, &[*a, *b]).is_some_and(|q| q.distance(p) <= 2.0 * max_mm)
            {
                parts.push(((*a, *b, false), k));
            }
        }
    }
    for (k, (a, b)) in lines.iter().enumerate() {
        if distance_to_line(p, *a, *b) <= 2.0 * max_mm {
            parts.push(((*a, *b, true), polylines.len() + k));
        }
    }
    let mut best: Option<DVec2> = None;
    for (i, (a, owner_a)) in parts.iter().enumerate() {
        for (b, owner_b) in parts.iter().skip(i + 1) {
            if owner_a == owner_b {
                continue;
            }
            if let Some(c) = crossing(*a, *b)
                && c.distance(p) <= max_mm
                && best.is_none_or(|x| c.distance(p) < x.distance(p))
            {
                best = Some(c);
            }
        }
    }
    best
}

/// Where `p` lands within `max_mm`: on a corner (but `skip`), an edge's
/// middle, where two lines cross, level with one of `align` (horizontally or vertically), on an
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
    let whole_lines: Vec<(DVec2, DVec2)> = project
        .guides()
        .values()
        .map(|g| (g.a, g.b))
        .chain(
            project
                .panels()
                .values()
                .filter_map(|panel| panel.symmetry.as_ref())
                .map(|sym| (sym.origin, sym.origin + sym.dir)),
        )
        .collect();
    if let Some(at) = crossing_near(p, max_mm, &polylines, &whole_lines) {
        return Landing {
            at,
            kind: "crossing",
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

    #[test]
    fn lands_where_two_lines_cross() {
        let mut s = square();
        let step = json!({"type": "guide.add", "a": [400, 0], "b": [410, 500]});
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
        let p = s.project();
        // Two construction lines: (400, 0)–(410, 500) meets y = 300 at
        // x = 406.
        let l = land(p, DVec2::new(404.0, 302.0), 5.0, &[], None);
        assert_eq!(l.kind, "crossing");
        assert!(l.at.distance(DVec2::new(406.0, 300.0)) < 1e-9, "{:?}", l.at);
        // A line across an edge: x = 400 + y / 50 meets the square's
        // top (y = 100)? No: x = 402 is outside it. The level line
        // y = 300 meets nothing of the square either; its side x = 100
        // extended is not a line. Nothing to cross there.
        let l = land(p, DVec2::new(100.0, 297.0), 5.0, &[], None);
        assert_eq!(l.kind, "guide");
        // An edge's own corner is a corner, not a crossing.
        let l = land(p, DVec2::new(97.0, 97.0), 5.0, &[], None);
        assert_eq!(l.kind, "point");
    }
}
