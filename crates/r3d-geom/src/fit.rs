//! Curves fitted to digitised points: a polyline (from a DXF file, or a
//! curve flattened by another program) becomes a few straight edges and
//! cubic Bézier curves that follow it within a tolerance, so an imported
//! outline has as few points to edit as a drawn one.
//!
//! Corners are where the polyline turns by more than a given angle at one
//! vertex; between corners, a straight run becomes one line and a curved
//! run is fitted with cubics by least squares, splitting where the error
//! is largest: P. J. Schneider, "An Algorithm for Automatically Fitting
//! Digitized Curves", in *Graphics Gems*, Academic Press, 1990, pp. 612–626.

use glam::DVec2;

use crate::bezier::CubicBez;

/// One fitted piece of a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fitted {
    Line(DVec2, DVec2),
    Cubic(CubicBez),
}

impl Fitted {
    pub fn start(&self) -> DVec2 {
        match self {
            Fitted::Line(a, _) => *a,
            Fitted::Cubic(c) => c.p0,
        }
    }

    pub fn end(&self) -> DVec2 {
        match self {
            Fitted::Line(_, b) => *b,
            Fitted::Cubic(c) => c.p3,
        }
    }
}

/// Points closer than this are the same point (mm).
const SAME_POINT: f64 = 1e-9;
/// Newton reparametrisation is tried when the error is within this factor
/// of the tolerance (Schneider's "iteration error").
const ITERATION_FACTOR: f64 = 4.0;
const NEWTON_ITERATIONS: usize = 20;
/// Reparametrisations tried before splitting, however large the error.
const FREE_ITERATIONS: usize = 4;

/// Fit straight edges and cubics to a polyline. Corners end a piece (see
/// [`is_corner`]); every input point lies within `tolerance` of the
/// result. A closed polyline gives pieces that start at a corner when it
/// has one; its closing segment is implied.
pub fn fit_polyline(points: &[DVec2], closed: bool, tolerance: f64) -> Vec<Fitted> {
    let mut pts: Vec<DVec2> = Vec::with_capacity(points.len());
    for p in points {
        if pts
            .last()
            .is_none_or(|q: &DVec2| q.distance(*p) > SAME_POINT)
        {
            pts.push(*p);
        }
    }
    if closed && pts.len() > 1 && pts[0].distance(pts[pts.len() - 1]) <= SAME_POINT {
        pts.pop();
    }
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    // Turning angle at each vertex (degrees; 0 at the ends of an open line).
    let turn: Vec<f64> = (0..n)
        .map(|i| {
            if !closed && (i == 0 || i == n - 1) {
                return 0.0;
            }
            let a = pts[i] - pts[(i + n - 1) % n];
            let b = pts[(i + 1) % n] - pts[i];
            a.angle_to(b).abs().to_degrees()
        })
        .collect();
    let is_corner = |i: usize| is_corner(turn[(i + n - 1) % n], turn[i], turn[(i + 1) % n]);
    let tol = tolerance.max(1e-6);

    if !closed {
        let mut cuts = vec![0];
        cuts.extend((1..n - 1).filter(|&i| is_corner(i)));
        cuts.push(n - 1);
        let mut out = Vec::new();
        for w in cuts.windows(2) {
            fit_run(&pts[w[0]..=w[1]], None, None, tol, &mut out);
        }
        return out;
    }

    if n < 3 {
        return vec![Fitted::Line(pts[0], pts[1])];
    }
    let corners: Vec<usize> = (0..n).filter(|&i| is_corner(i)).collect();
    let mut out = Vec::new();
    if corners.is_empty() {
        // A smooth loop: two runs, joined smoothly at both ends.
        let half = n / 2;
        let tangent = |i: usize| (pts[(i + 1) % n] - pts[(i + n - 1) % n]).normalize_or_zero();
        let (t0, th) = (tangent(0), tangent(half));
        let first: Vec<DVec2> = pts[..=half].to_vec();
        let mut second: Vec<DVec2> = pts[half..].to_vec();
        second.push(pts[0]);
        fit_run(&first, Some(t0), Some(-th), tol, &mut out);
        fit_run(&second, Some(th), Some(-t0), tol, &mut out);
        return out;
    }
    for (k, &from) in corners.iter().enumerate() {
        let to = corners[(k + 1) % corners.len()];
        let mut run = Vec::new();
        let mut i = from;
        loop {
            run.push(pts[i]);
            if i == to && run.len() > 1 {
                break;
            }
            i = (i + 1) % n;
        }
        fit_run(&run, None, None, tol, &mut out);
    }
    out
}

/// Fit one run between corners: a line if it is straight, cubics
/// otherwise. `t_start` points into the run from its first point,
/// `t_end` into the run from its last point; one-sided when not given.
fn fit_run(
    run: &[DVec2],
    t_start: Option<DVec2>,
    t_end: Option<DVec2>,
    tol: f64,
    out: &mut Vec<Fitted>,
) {
    let (Some(&a), Some(&b)) = (run.first(), run.last()) else {
        return;
    };
    if run.len() == 2 || run.iter().all(|p| distance_to_segment(*p, a, b) <= tol) {
        out.push(Fitted::Line(a, b));
        return;
    }
    // Sample segments much longer than the run's usual spacing, so the
    // fit cannot bulge between far points (a long straight part next to
    // a finely sampled curve).
    let mut lengths: Vec<f64> = run.windows(2).map(|w| w[0].distance(w[1])).collect();
    lengths.sort_by(f64::total_cmp);
    let step = (4.0 * lengths[lengths.len() / 2]).max(tol);
    let mut d = vec![a];
    for w in run.windows(2) {
        let k = (w[0].distance(w[1]) / step).ceil().max(1.0) as usize;
        d.extend((1..=k).map(|j| w[0].lerp(w[1], j as f64 / k as f64)));
    }
    let n = d.len();
    let t1 = t_start
        .unwrap_or_else(|| end_tangent(d[0], d[1], d.get(2).copied()))
        .normalize_or_zero();
    let t2 = t_end
        .unwrap_or_else(|| end_tangent(d[n - 1], d[n - 2], n.checked_sub(3).map(|i| d[i])))
        .normalize_or_zero();
    let mut cubics = Vec::new();
    fit_cubic(&d, t1, t2, tol, &mut cubics);
    for c in cubics {
        let straight = distance_to_segment(c.p1, c.p0, c.p3) <= tol * 0.5
            && distance_to_segment(c.p2, c.p0, c.p3) <= tol * 0.5;
        out.push(if straight {
            Fitted::Line(c.p0, c.p3)
        } else {
            Fitted::Cubic(c)
        });
    }
}

/// Above this turning angle a vertex is always a corner (degrees).
const CORNER_DEG: f64 = 40.0;
/// Below this one it never is: the sampling of a curve.
const BEND_DEG: f64 = 8.0;
/// In between, a vertex is a corner when it turns this many times more
/// than its neighbours: a coarsely sampled curve turns evenly, a corner
/// stands out.
const STANDS_OUT: f64 = 2.5;

/// Whether a vertex turning by `here` degrees, between neighbours turning
/// by `before` and `after`, is a corner.
fn is_corner(before: f64, here: f64, after: f64) -> bool {
    here > CORNER_DEG || (here > BEND_DEG && here > STANDS_OUT * before.max(after))
}

/// Unit tangent at `p0` pointing towards `p1`: from the parabola through
/// `p0`, `p1`, `p2` (parametrised by chord length) when there is a third
/// point, which is far more accurate than the chord on a finely sampled
/// curve.
fn end_tangent(p0: DVec2, p1: DVec2, p2: Option<DVec2>) -> DVec2 {
    let chord = (p1 - p0).normalize_or_zero();
    let Some(p2) = p2 else {
        return chord;
    };
    let (h1, h2) = (p0.distance(p1), p1.distance(p2));
    if h1 <= 0.0 || h2 <= 0.0 {
        return chord;
    }
    let d = p0 * (-(2.0 * h1 + h2) / (h1 * (h1 + h2))) + p1 * ((h1 + h2) / (h1 * h2))
        - p2 * (h1 / (h2 * (h1 + h2)));
    // Never turn by more than a right angle from the chord.
    match d.try_normalize() {
        Some(t) if t.dot(chord) > 0.0 => t,
        _ => chord,
    }
}

/// Schneider's recursive fit of `d` with end tangents `t1` (at the start,
/// pointing forward) and `t2` (at the end, pointing backward).
fn fit_cubic(d: &[DVec2], t1: DVec2, t2: DVec2, tol: f64, out: &mut Vec<CubicBez>) {
    let (Some(&first), Some(&last)) = (d.first(), d.last()) else {
        return;
    };
    if d.len() == 2 {
        let dist = first.distance(last) / 3.0;
        out.push(CubicBez::new(
            first,
            first + t1 * dist,
            last + t2 * dist,
            last,
        ));
        return;
    }
    let mut u = chord_parameters(d);
    let mut bez = generate(d, &u, t1, t2);
    let (mut err, mut split) = max_error(d, &bez, &u);
    let tol2 = tol * tol;
    if err < tol2 {
        out.push(bez);
        return;
    }
    // A few reparametrisations always (the worst point moves to where the
    // fit really fails), more when the fit is nearly good enough.
    for i in 0..NEWTON_ITERATIONS {
        if i >= FREE_ITERATIONS && err >= tol2 * ITERATION_FACTOR * ITERATION_FACTOR {
            break;
        }
        u = reparametrize(d, &u, &bez);
        bez = generate(d, &u, t1, t2);
        (err, split) = max_error(d, &bez, &u);
        if err < tol2 {
            out.push(bez);
            return;
        }
    }
    let centre = (d[split - 1] - d[split + 1])
        .try_normalize()
        .unwrap_or_else(|| (d[split - 1] - d[split]).normalize_or_zero());
    fit_cubic(&d[..=split], t1, centre, tol, out);
    fit_cubic(&d[split..], -centre, t2, tol, out);
}

fn chord_parameters(d: &[DVec2]) -> Vec<f64> {
    let mut u = Vec::with_capacity(d.len());
    let mut acc = 0.0;
    u.push(0.0);
    for w in d.windows(2) {
        acc += w[0].distance(w[1]);
        u.push(acc);
    }
    if acc > 0.0 {
        for x in &mut u {
            *x /= acc;
        }
    }
    u
}

fn bernstein(t: f64) -> [f64; 4] {
    let mt = 1.0 - t;
    [mt * mt * mt, 3.0 * t * mt * mt, 3.0 * t * t * mt, t * t * t]
}

/// Least-squares control points along the given end tangents.
fn generate(d: &[DVec2], u: &[f64], t1: DVec2, t2: DVec2) -> CubicBez {
    let (first, last) = (d[0], d[d.len() - 1]);
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (p, &t) in d.iter().zip(u) {
        let b = bernstein(t);
        let a0 = t1 * b[1];
        let a1 = t2 * b[2];
        c00 += a0.dot(a0);
        c01 += a0.dot(a1);
        c11 += a1.dot(a1);
        let tmp = *p - (first * (b[0] + b[1]) + last * (b[2] + b[3]));
        x0 += a0.dot(tmp);
        x1 += a1.dot(tmp);
    }
    let det = c00 * c11 - c01 * c01;
    let seg = first.distance(last);
    let (mut al, mut ar) = if det.abs() > 1e-12 {
        ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det)
    } else {
        (0.0, 0.0)
    };
    // Wildly wrong (or negative) handles: fall back to a third of the chord.
    let eps = 1e-6 * seg;
    if !(al.is_finite() && ar.is_finite()) || al < eps || ar < eps {
        al = seg / 3.0;
        ar = al;
    }
    CubicBez::new(first, first + t1 * al, last + t2 * ar, last)
}

/// Largest squared distance between a point and the curve at its
/// parameter, and the interior point where it happens.
fn max_error(d: &[DVec2], bez: &CubicBez, u: &[f64]) -> (f64, usize) {
    let mut worst = (0.0, d.len() / 2);
    for i in 1..d.len() - 1 {
        let e = bez.eval(u[i]).distance_squared(d[i]);
        if e >= worst.0 {
            worst = (e, i);
        }
    }
    worst
}

/// One Newton step per point towards the parameter of its nearest curve
/// point.
fn reparametrize(d: &[DVec2], u: &[f64], bez: &CubicBez) -> Vec<f64> {
    let q1 = [
        (bez.p1 - bez.p0) * 3.0,
        (bez.p2 - bez.p1) * 3.0,
        (bez.p3 - bez.p2) * 3.0,
    ];
    let q2 = [(q1[1] - q1[0]) * 2.0, (q1[2] - q1[1]) * 2.0];
    d.iter()
        .zip(u)
        .map(|(p, &t)| {
            let mt = 1.0 - t;
            let q = bez.eval(t);
            let d1 = q1[0] * (mt * mt) + q1[1] * (2.0 * mt * t) + q1[2] * (t * t);
            let d2 = q2[0] * mt + q2[1] * t;
            let num = (q - *p).dot(d1);
            let den = d1.dot(d1) + (q - *p).dot(d2);
            if den.abs() < 1e-12 {
                t
            } else {
                (t - num / den).clamp(0.0, 1.0)
            }
        })
        .collect()
}

/// Distance from `p` to the segment `a b`.
pub fn distance_to_segment(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Distance from `p` to the fitted path, measured on the curves
    /// themselves: flattening is capped in segments, so on a long cubic it
    /// cannot reach the precision the tests' 1e-6 slack needs.
    fn distance_to(path: &[Fitted], p: DVec2) -> f64 {
        path.iter()
            .map(|f| match f {
                Fitted::Line(a, b) => distance_to_segment(p, *a, *b),
                Fitted::Cubic(c) => distance_to_cubic(c, p),
            })
            .fold(f64::MAX, f64::min)
    }

    /// Nearest point by coarse sampling, then golden-section search around
    /// the best sample.
    fn distance_to_cubic(c: &CubicBez, p: DVec2) -> f64 {
        const SAMPLES: usize = 512;
        let dist = |t: f64| c.eval(t).distance(p);
        let best = (0..=SAMPLES)
            .min_by(|&i, &j| {
                dist(i as f64 / SAMPLES as f64).total_cmp(&dist(j as f64 / SAMPLES as f64))
            })
            .unwrap_or(0);
        let step = 1.0 / SAMPLES as f64;
        let (mut lo, mut hi) = (
            (best as f64 * step - step).max(0.0),
            (best as f64 * step + step).min(1.0),
        );
        let g = (5f64.sqrt() - 1.0) / 2.0;
        for _ in 0..100 {
            let (m1, m2) = (hi - g * (hi - lo), lo + g * (hi - lo));
            if dist(m1) < dist(m2) {
                hi = m2;
            } else {
                lo = m1;
            }
        }
        dist(0.5 * (lo + hi))
    }

    fn connected(path: &[Fitted], closed: bool) -> bool {
        let n = path.len();
        let ends = if closed { n } else { n.saturating_sub(1) };
        (0..ends).all(|i| path[i].end().distance(path[(i + 1) % n].start()) < 1e-9)
    }

    #[test]
    fn a_square_stays_four_lines() {
        let sq = [
            DVec2::new(0.0, 0.0),
            DVec2::new(5.0, 0.0),
            DVec2::new(10.0, 0.0),
            DVec2::new(10.0, 10.0),
            DVec2::new(0.0, 10.0),
        ];
        let path = fit_polyline(&sq, true, 0.1);
        assert_eq!(path.len(), 4);
        assert!(path.iter().all(|f| matches!(f, Fitted::Line(..))));
        assert!(connected(&path, true));
    }

    #[test]
    fn a_flattened_arc_becomes_one_or_two_cubics() {
        let arc: Vec<DVec2> = (0..=90)
            .map(|i| DVec2::from_angle(f64::from(i).to_radians()) * 100.0)
            .collect();
        let path = fit_polyline(&arc, false, 0.05);
        assert!((1..=2).contains(&path.len()), "{}", path.len());
        assert!(arc.iter().all(|p| distance_to(&path, *p) <= 0.05 + 1e-9));
        assert_eq!(path[0].start(), arc[0]);
        assert_eq!(path[path.len() - 1].end(), arc[90]);
    }

    #[test]
    fn a_circle_has_no_corner() {
        let circle: Vec<DVec2> = (0..72)
            .map(|i| DVec2::from_angle(f64::from(i * 5).to_radians()) * 50.0)
            .collect();
        let path = fit_polyline(&circle, true, 0.05);
        assert!(connected(&path, true));
        assert!(path.len() <= 6, "{}", path.len());
        assert!(circle.iter().all(|p| distance_to(&path, *p) <= 0.05 + 1e-9));
    }

    #[test]
    fn a_line_into_a_curve_keeps_its_corner() {
        // A straight side then a quarter circle, meeting at a right angle.
        let mut pts = vec![DVec2::new(-100.0, 0.0)];
        pts.extend((0..=45).map(|i| DVec2::from_angle(f64::from(i * 2).to_radians()) * 80.0));
        let path = fit_polyline(&pts, false, 0.1);
        assert!(matches!(path[0], Fitted::Line(..)));
        assert!(path[0].end().distance(DVec2::new(80.0, 0.0)) < 1e-9);
        assert!(pts.iter().all(|p| distance_to(&path, *p) <= 0.1 + 1e-9));
    }

    #[test]
    fn a_shallow_corner_between_curves_is_kept() {
        // Two finely sampled arcs meeting at a 15° kink.
        let mut pts: Vec<DVec2> = (0..=30)
            .map(|i| DVec2::new(f64::from(i) * 5.0, 0.002 * (f64::from(i) * 5.0).powi(2)))
            .collect();
        let kink = pts[30];
        let dir = (pts[30] - pts[29]).normalize();
        let turned = DVec2::from_angle(15f64.to_radians()).rotate(dir);
        pts.extend((1..=30).map(|i| kink + turned * (f64::from(i) * 5.0)));
        let path = fit_polyline(&pts, false, 0.1);
        assert!(path.iter().any(|f| f.end().distance(kink) < 1e-9));
        // A coarse circle (15° per vertex) has no corner.
        let coarse: Vec<DVec2> = (0..24)
            .map(|i| DVec2::from_angle(f64::from(i * 15).to_radians()) * 100.0)
            .collect();
        assert!(fit_polyline(&coarse, true, 2.0).len() <= 4);
    }

    proptest! {
        #[test]
        fn fitted_paths_follow_the_points(
            amp in 1.0f64..80.0,
            freq in 0.2f64..2.0,
            len in 50.0f64..600.0,
            samples in 8usize..200,
            tol in 0.02f64..1.0,
        ) {
            let pts: Vec<DVec2> = (0..=samples)
                .map(|i| {
                    let x = len * i as f64 / samples as f64;
                    DVec2::new(x, amp * (freq * x / 50.0).sin())
                })
                .collect();
            let path = fit_polyline(&pts, false, tol);
            prop_assert!(connected(&path, false));
            prop_assert_eq!(path[0].start(), pts[0]);
            prop_assert_eq!(path[path.len() - 1].end(), pts[samples]);
            for p in &pts {
                prop_assert!(distance_to(&path, *p) <= tol + 1e-6);
            }
        }
    }
}
