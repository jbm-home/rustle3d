//! Cubic Bézier segments.

use glam::DVec2;

/// Maximum number of segments used when flattening one cubic.
const MAX_FLATTEN_SEGMENTS: usize = 1024;

/// A cubic Bézier segment `p0 → p3` with control points `p1`, `p2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBez {
    pub p0: DVec2,
    pub p1: DVec2,
    pub p2: DVec2,
    pub p3: DVec2,
}

impl CubicBez {
    pub fn new(p0: DVec2, p1: DVec2, p2: DVec2, p3: DVec2) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// The straight segment `a → b` as a cubic, with the same uniform
    /// parametrisation as the line (`eval(t) = a + t (b − a)`).
    pub fn line(a: DVec2, b: DVec2) -> Self {
        Self::new(a, a.lerp(b, 1.0 / 3.0), a.lerp(b, 2.0 / 3.0), b)
    }

    /// The same curve walked from `p3` to `p0`.
    pub fn reversed(&self) -> Self {
        Self::new(self.p3, self.p2, self.p1, self.p0)
    }

    /// The part of the curve between parameters `lo ≤ hi` (in `[0, 1]`).
    pub fn sub(&self, lo: f64, hi: f64) -> Self {
        let right = if lo > 0.0 { self.split(lo).1 } else { *self };
        if hi >= 1.0 {
            return right;
        }
        let s = if lo < 1.0 {
            (hi - lo) / (1.0 - lo)
        } else {
            0.0
        };
        right.split(s.clamp(0.0, 1.0)).0
    }

    /// Where this curve crosses `other`: `(t, u)` parameter pairs on each,
    /// accurate to about `tolerance` (mm) in position. Both curves are
    /// flattened and their segments intersected; touching ends count, so
    /// a crossing at a shared flattening vertex may be reported twice
    /// (callers merge crossings closer than their own tolerance). Overlapping
    /// collinear pieces are not reported.
    pub fn crossings(&self, other: &CubicBez, tolerance: f64) -> Vec<(f64, f64)> {
        let flat = |c: &CubicBez| {
            let n = c.flatten_segment_count(tolerance);
            (0..=n)
                .map(|i| {
                    let t = i as f64 / n as f64;
                    (t, c.eval(t))
                })
                .collect::<Vec<_>>()
        };
        let (a, b) = (flat(self), flat(other));
        let mut out = Vec::new();
        for wa in a.windows(2) {
            let [(ta0, p), (ta1, p2)] = [wa[0], wa[1]];
            for wb in b.windows(2) {
                let [(tb0, q), (tb1, q2)] = [wb[0], wb[1]];
                let (r, s) = (p2 - p, q2 - q);
                let denom = r.perp_dot(s);
                if denom.abs() <= f64::EPSILON * r.length() * s.length() {
                    continue;
                }
                let x = (q - p).perp_dot(s) / denom;
                let y = (q - p).perp_dot(r) / denom;
                if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
                    out.push((ta0 + x * (ta1 - ta0), tb0 + y * (tb1 - tb0)));
                }
            }
        }
        out
    }

    /// Point at parameter `t ∈ [0, 1]`.
    pub fn eval(&self, t: f64) -> DVec2 {
        let mt = 1.0 - t;
        self.p0 * (mt * mt * mt)
            + self.p1 * (3.0 * mt * mt * t)
            + self.p2 * (3.0 * mt * t * t)
            + self.p3 * (t * t * t)
    }

    /// Split at `t` with de Casteljau's algorithm.
    pub fn split(&self, t: f64) -> (CubicBez, CubicBez) {
        let a = self.p0.lerp(self.p1, t);
        let b = self.p1.lerp(self.p2, t);
        let c = self.p2.lerp(self.p3, t);
        let ab = a.lerp(b, t);
        let bc = b.lerp(c, t);
        let m = ab.lerp(bc, t);
        (
            CubicBez::new(self.p0, a, ab, m),
            CubicBez::new(m, bc, c, self.p3),
        )
    }

    /// Number of uniform segments so that the polyline stays within
    /// `tolerance` of the curve.
    ///
    /// Uses the classic bound `max |B''| / (8 n²)` with
    /// `|B''| ≤ 6 max(|p0 − 2p1 + p2|, |p1 − 2p2 + p3|)`.
    pub fn flatten_segment_count(&self, tolerance: f64) -> usize {
        let dd = (self.p0 - 2.0 * self.p1 + self.p2)
            .length()
            .max((self.p1 - 2.0 * self.p2 + self.p3).length());
        if tolerance.is_nan() || tolerance <= 0.0 || !dd.is_finite() {
            return 1;
        }
        let n = (0.75 * dd / tolerance).sqrt().ceil();
        if n.is_finite() && n >= 1.0 {
            (n as usize).min(MAX_FLATTEN_SEGMENTS)
        } else {
            1
        }
    }

    /// Append the flattened curve to `out`, **excluding** `p0` (so that
    /// consecutive segments can be chained without duplicates).
    pub fn flatten_into(&self, tolerance: f64, out: &mut Vec<DVec2>) {
        let n = self.flatten_segment_count(tolerance);
        for i in 1..=n {
            out.push(self.eval(i as f64 / n as f64));
        }
    }

    /// Arc length, accurate to roughly `tolerance` (mm).
    pub fn arc_length(&self, tolerance: f64) -> f64 {
        arc_length_rec(self, tolerance.max(1e-9), 0)
    }

    /// Parameter `t` whose arc length from `p0` is `length` (clamped to the
    /// curve), found by bisection to within `tolerance` (mm).
    pub fn param_at_length(&self, length: f64, tolerance: f64) -> f64 {
        let total = self.arc_length(tolerance);
        if length.is_nan() || length <= 0.0 {
            return 0.0;
        }
        if length >= total {
            return 1.0;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            let l = self.split(mid).0.arc_length(tolerance);
            if (l - length).abs() <= tolerance {
                return mid;
            }
            if l < length {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    /// Unit tangent at `t` (falls back to the chord for degenerate ends).
    pub fn tangent(&self, t: f64) -> DVec2 {
        let mt = 1.0 - t;
        let d = (self.p1 - self.p0) * (3.0 * mt * mt)
            + (self.p2 - self.p1) * (6.0 * mt * t)
            + (self.p3 - self.p2) * (3.0 * t * t);
        d.try_normalize()
            .or_else(|| (self.p3 - self.p0).try_normalize())
            .unwrap_or(DVec2::X)
    }
}

fn arc_length_rec(c: &CubicBez, tol: f64, depth: u32) -> f64 {
    let chord = c.p0.distance(c.p3);
    let poly = c.p0.distance(c.p1) + c.p1.distance(c.p2) + c.p2.distance(c.p3);
    // The arc length lies between chord and control-polygon length.
    if poly - chord <= tol || depth >= 24 {
        // Gravesen's estimate, much better than either bound alone.
        return (2.0 * chord + poly) / 3.0;
    }
    let (a, b) = c.split(0.5);
    arc_length_rec(&a, tol * 0.5, depth + 1) + arc_length_rec(&b, tol * 0.5, depth + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight() -> CubicBez {
        CubicBez::new(
            DVec2::new(0.0, 0.0),
            DVec2::new(10.0, 0.0),
            DVec2::new(20.0, 0.0),
            DVec2::new(30.0, 0.0),
        )
    }

    #[test]
    fn crossings_of_a_line_and_an_arch() {
        let arch = CubicBez::new(
            DVec2::new(0.0, 0.0),
            DVec2::new(0.0, 100.0),
            DVec2::new(100.0, 100.0),
            DVec2::new(100.0, 0.0),
        );
        let line = CubicBez::line(DVec2::new(-10.0, 50.0), DVec2::new(110.0, 50.0));
        let mut found = line.crossings(&arch, 1e-3);
        found.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
        assert_eq!(found.len(), 2);
        for (t, u) in found {
            assert!(line.eval(t).distance(arch.eval(u)) < 1e-2);
            assert!((arch.eval(u).y - 50.0).abs() < 1e-2);
        }
    }

    #[test]
    fn sub_curve_matches_the_curve() {
        let c = CubicBez::new(
            DVec2::new(0.0, 0.0),
            DVec2::new(5.0, 40.0),
            DVec2::new(35.0, -20.0),
            DVec2::new(40.0, 10.0),
        );
        let s = c.sub(0.2, 0.7);
        assert!(s.p0.distance(c.eval(0.2)) < 1e-12);
        assert!(s.p3.distance(c.eval(0.7)) < 1e-12);
        assert!(s.eval(0.5).distance(c.eval(0.45)) < 1e-9);
        assert!(
            CubicBez::line(DVec2::ZERO, DVec2::X)
                .eval(0.25)
                .distance(DVec2::new(0.25, 0.0))
                < 1e-12
        );
    }

    #[test]
    fn straight_cubic_has_chord_length() {
        assert!((straight().arc_length(1e-6) - 30.0).abs() < 1e-6);
    }

    #[test]
    fn quarter_circle_length() {
        // Standard cubic approximation of a quarter circle of radius 100.
        let k = 0.552_284_749_830_793_4 * 100.0;
        let c = CubicBez::new(
            DVec2::new(100.0, 0.0),
            DVec2::new(100.0, k),
            DVec2::new(k, 100.0),
            DVec2::new(0.0, 100.0),
        );
        let expected = std::f64::consts::FRAC_PI_2 * 100.0;
        // The cubic itself deviates from the circle by ~0.03%.
        assert!((c.arc_length(1e-6) - expected).abs() < 0.05);
    }

    #[test]
    fn split_is_continuous() {
        let c = CubicBez::new(
            DVec2::new(0.0, 0.0),
            DVec2::new(5.0, 40.0),
            DVec2::new(35.0, -20.0),
            DVec2::new(40.0, 10.0),
        );
        let (a, b) = c.split(0.3);
        assert!(a.p3.distance(c.eval(0.3)) < 1e-12);
        assert!(b.eval(0.5).distance(c.eval(0.65)) < 1e-9);
    }

    #[test]
    fn param_at_length_inverts_arc_length() {
        let c = CubicBez::new(
            DVec2::new(0.0, 0.0),
            DVec2::new(0.0, 100.0),
            DVec2::new(200.0, 100.0),
            DVec2::new(200.0, 0.0),
        );
        let total = c.arc_length(1e-6);
        for frac in [0.1, 0.25, 0.5, 0.9] {
            let t = c.param_at_length(frac * total, 1e-6);
            let l = c.split(t).0.arc_length(1e-6);
            assert!((l - frac * total).abs() < 1e-4, "{frac}: {l}");
        }
        assert_eq!(c.param_at_length(-5.0, 1e-6), 0.0);
        assert_eq!(c.param_at_length(total + 5.0, 1e-6), 1.0);
    }

    #[test]
    fn tangent_of_straight_cubic_is_x() {
        assert!((straight().tangent(0.5) - DVec2::X).length() < 1e-12);
    }

    #[test]
    fn flatten_ends_at_p3() {
        let mut out = Vec::new();
        straight().flatten_into(0.01, &mut out);
        assert_eq!(out.last().copied(), Some(DVec2::new(30.0, 0.0)));
    }
}
