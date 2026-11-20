//! Curves of a Seamly2D/Valentina pattern as chains of cubic Béziers (mm,
//! y up): splines and spline paths as they are, arcs and elliptical arcs
//! approximated in pieces of at most 90° (handles `4/3 · tan(θ/4)`, as
//! for any circular arc; an ellipse is the affine image of a circle, so
//! its pieces are the images of the circle's).

use r3d_geom::{CubicBez, DVec2};

/// Length tolerance (mm).
const TOL: f64 = 0.01;

/// A place on a curve: segment index and parameter in it.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Loc {
    pub seg: usize,
    pub t: f64,
}

/// A chain of cubics, each starting where the previous one ends.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Curve {
    pub segs: Vec<CubicBez>,
}

impl Curve {
    pub fn start(&self) -> Option<DVec2> {
        self.segs.first().map(|s| s.p0)
    }

    pub fn end(&self) -> Option<DVec2> {
        self.segs.last().map(|s| s.p3)
    }

    pub fn length(&self) -> f64 {
        self.segs.iter().map(|s| s.arc_length(TOL)).sum()
    }

    pub fn reversed(&self) -> Curve {
        Curve {
            segs: self.segs.iter().rev().map(CubicBez::reversed).collect(),
        }
    }

    /// The curve with every defining point moved by `f` (an affine map).
    pub fn mapped(&self, f: impl Fn(DVec2) -> DVec2) -> Curve {
        Curve {
            segs: self
                .segs
                .iter()
                .map(|s| CubicBez::new(f(s.p0), f(s.p1), f(s.p2), f(s.p3)))
                .collect(),
        }
    }

    pub fn polyline(&self) -> Vec<DVec2> {
        let mut out: Vec<DVec2> = self.start().into_iter().collect();
        for s in &self.segs {
            s.flatten_into(0.05, &mut out);
        }
        out
    }

    pub fn point(&self, at: Loc) -> Option<DVec2> {
        Some(self.segs.get(at.seg)?.eval(at.t))
    }

    /// The place at `length` from the start, clamped to the curve.
    pub fn at_length(&self, length: f64) -> Loc {
        let mut left = length.max(0.0);
        let last = self.segs.len().saturating_sub(1);
        for (i, s) in self.segs.iter().enumerate() {
            let l = s.arc_length(TOL);
            if left <= l || i == last {
                return Loc {
                    seg: i,
                    t: s.param_at_length(left, TOL * 0.1),
                };
            }
            left -= l;
        }
        Loc { seg: 0, t: 0.0 }
    }

    /// The place on the curve nearest to `p`, if it is within `tol`.
    pub fn locate(&self, p: DVec2, tol: f64) -> Option<Loc> {
        const N: usize = 48;
        let mut best: Option<(f64, Loc)> = None;
        for (i, s) in self.segs.iter().enumerate() {
            let mut t0 = 0.0;
            let mut d0 = f64::INFINITY;
            for k in 0..=N {
                let t = k as f64 / N as f64;
                let d = s.eval(t).distance(p);
                if d < d0 {
                    (d0, t0) = (d, t);
                }
            }
            // Refine by ternary search around the best sample.
            let (mut lo, mut hi) = (
                (t0 - 1.0 / N as f64).max(0.0),
                (t0 + 1.0 / N as f64).min(1.0),
            );
            for _ in 0..60 {
                let m1 = lo + (hi - lo) / 3.0;
                let m2 = hi - (hi - lo) / 3.0;
                if s.eval(m1).distance(p) < s.eval(m2).distance(p) {
                    hi = m2;
                } else {
                    lo = m1;
                }
            }
            let t = 0.5 * (lo + hi);
            let d = s.eval(t).distance(p);
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, Loc { seg: i, t }));
            }
        }
        best.filter(|(d, _)| *d <= tol).map(|(_, l)| l)
    }

    /// The two parts on either side of `at`.
    pub fn split(&self, at: Loc) -> (Curve, Curve) {
        let mut left = Vec::new();
        let mut right = Vec::new();
        for (i, s) in self.segs.iter().enumerate() {
            if i < at.seg {
                left.push(*s);
            } else if i > at.seg {
                right.push(*s);
            } else {
                let (a, b) = s.split(at.t.clamp(0.0, 1.0));
                if at.t > 1e-12 {
                    left.push(a);
                }
                if at.t < 1.0 - 1e-12 {
                    right.push(b);
                }
            }
        }
        (Curve { segs: left }, Curve { segs: right })
    }

    /// The part between `a` and `b` (`a` before `b`).
    pub fn between(&self, a: Loc, b: Loc) -> Curve {
        let (_, tail) = self.split(a);
        // `b` in the tail's terms.
        let b = if b.seg == a.seg {
            let t = if a.t < 1.0 {
                (b.t - a.t) / (1.0 - a.t)
            } else {
                0.0
            };
            Loc {
                seg: 0,
                t: t.clamp(0.0, 1.0),
            }
        } else {
            let skipped = if a.t < 1.0 - 1e-12 { a.seg } else { a.seg + 1 };
            Loc {
                seg: b.seg.saturating_sub(skipped),
                t: b.t,
            }
        };
        tail.split(b).0
    }

    /// Where the two curves cross (places on each and the point), ends
    /// included, crossings closer than 0.01 mm merged.
    pub fn crossings(&self, other: &Curve) -> Vec<(Loc, Loc, DVec2)> {
        let mut out: Vec<(Loc, Loc, DVec2)> = Vec::new();
        for (i, a) in self.segs.iter().enumerate() {
            for (j, b) in other.segs.iter().enumerate() {
                for (t, u) in a.crossings(b, 0.001) {
                    let p = a.eval(t);
                    if out.iter().all(|(_, _, q)| q.distance(p) > 0.01) {
                        out.push((Loc { seg: i, t }, Loc { seg: j, t: u }, p));
                    }
                }
            }
        }
        out
    }
}

/// Unit vector at `deg` degrees (y up, counter-clockwise from +x).
pub fn dir(deg: f64) -> DVec2 {
    let r = deg.to_radians();
    DVec2::new(r.cos(), r.sin())
}

/// Angle of `v` in degrees, in `[0, 360)`.
pub fn angle_of(v: DVec2) -> f64 {
    let a = v.y.atan2(v.x).to_degrees().rem_euclid(360.0);
    if a >= 360.0 { 0.0 } else { a }
}

/// Counter-clockwise sweep from `a1` to `a2` (degrees, in `(0, 360]`).
pub fn ccw_sweep(a1: f64, a2: f64) -> f64 {
    let s = (a2 - a1).rem_euclid(360.0);
    if s < 1e-9 { 360.0 } else { s }
}

/// A circular arc from angle `from` turning by `sweep` degrees
/// (negative: clockwise).
pub fn arc(center: DVec2, radius: f64, from: f64, sweep: f64) -> Curve {
    ellipse_arc(
        center,
        radius,
        radius,
        0.0,
        from.to_radians(),
        sweep.to_radians(),
    )
}

/// An elliptical arc, radii `a` (local x) and `b` (local y), turned by
/// `rotation` degrees, from parametric angle `t0` (radians) by `sweep`
/// radians.
pub fn ellipse_arc(center: DVec2, a: f64, b: f64, rotation: f64, t0: f64, sweep: f64) -> Curve {
    let pieces = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = sweep / pieces as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let rot = dir(rotation);
    let map = |v: DVec2| center + DVec2::new(a * v.x, b * v.y).rotate(rot);
    let segs = (0..pieces)
        .map(|i| {
            let t1 = t0 + step * i as f64;
            let t2 = t1 + step;
            let (p, q) = (DVec2::from_angle(t1), DVec2::from_angle(t2));
            CubicBez::new(map(p), map(p + p.perp() * k), map(q - q.perp() * k), map(q))
        })
        .collect();
    Curve { segs }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_circle_length() {
        let c = arc(DVec2::ZERO, 100.0, 0.0, 90.0);
        assert!((c.length() - 50.0 * std::f64::consts::PI).abs() < 0.05);
        assert!(c.end().unwrap().distance(DVec2::new(0.0, 100.0)) < 1e-9);
        let cw = arc(DVec2::ZERO, 100.0, 0.0, -90.0);
        assert!(cw.end().unwrap().distance(DVec2::new(0.0, -100.0)) < 1e-9);
    }

    #[test]
    fn split_and_between() {
        let c = Curve {
            segs: vec![
                CubicBez::line(DVec2::ZERO, DVec2::new(100.0, 0.0)),
                CubicBez::line(DVec2::new(100.0, 0.0), DVec2::new(100.0, 100.0)),
            ],
        };
        let at = c.at_length(150.0);
        assert!(c.point(at).unwrap().distance(DVec2::new(100.0, 50.0)) < 0.01);
        let a = c.locate(DVec2::new(50.0, 0.3), 1.0).unwrap();
        let mid = c.between(a, at);
        assert!((mid.length() - 100.0).abs() < 0.02);
        assert!(c.locate(DVec2::new(50.0, 5.0), 1.0).is_none());
    }

    #[test]
    fn crossing_curves() {
        let a = Curve {
            segs: vec![CubicBez::line(
                DVec2::new(-10.0, 1.0),
                DVec2::new(10.0, 1.0),
            )],
        };
        let b = arc(DVec2::ZERO, 5.0, 0.0, 180.0);
        let x = a.crossings(&b);
        assert_eq!(x.len(), 2);
    }
}
