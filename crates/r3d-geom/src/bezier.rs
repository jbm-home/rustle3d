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
    fn flatten_ends_at_p3() {
        let mut out = Vec::new();
        straight().flatten_into(0.01, &mut out);
        assert_eq!(out.last().copied(), Some(DVec2::new(30.0, 0.0)));
    }
}
