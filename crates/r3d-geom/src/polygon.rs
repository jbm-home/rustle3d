//! Closed polygons given as a list of vertices (the closing edge is implicit).

use glam::DVec2;

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: DVec2,
    pub max: DVec2,
}

/// Signed area (shoelace). Positive for counter-clockwise polygons.
pub fn signed_area(pts: &[DVec2]) -> f64 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        sum += a.perp_dot(b);
    }
    0.5 * sum
}

/// Perimeter of the closed polygon.
pub fn perimeter(pts: &[DVec2]) -> f64 {
    let n = pts.len();
    if n < 2 {
        return 0.0;
    }
    (0..n).map(|i| pts[i].distance(pts[(i + 1) % n])).sum()
}

/// Bounding box, or `None` for an empty slice.
pub fn bounding_box(pts: &[DVec2]) -> Option<Aabb> {
    let first = *pts.first()?;
    let mut bb = Aabb {
        min: first,
        max: first,
    };
    for p in &pts[1..] {
        bb.min = bb.min.min(*p);
        bb.max = bb.max.max(*p);
    }
    Some(bb)
}

fn orient(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    (b - a).perp_dot(c - a)
}

fn on_segment(a: DVec2, b: DVec2, p: DVec2) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

/// Whether the closed segments `ab` and `cd` intersect (touching counts).
pub fn segments_intersect(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> bool {
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1 == 0.0 && on_segment(c, d, a))
        || (d2 == 0.0 && on_segment(c, d, b))
        || (d3 == 0.0 && on_segment(a, b, c))
        || (d4 == 0.0 && on_segment(a, b, d))
}

/// Whether the closed polygon is simple: no two non-adjacent edges touch and
/// no adjacent edges fold back onto each other. `O(n²)`, fine for pattern
/// panels (hundreds of vertices).
pub fn is_simple(pts: &[DVec2]) -> bool {
    let n = pts.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if a == b {
            return false;
        }
        for j in (i + 1)..n {
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            let (c, d) = (pts[j], pts[(j + 1) % n]);
            if adjacent {
                // Adjacent edges share one vertex; they must not overlap.
                let shared = if j == i + 1 { b } else { a };
                let (p, q) = if j == i + 1 { (a, d) } else { (b, c) };
                if orient(shared, p, q) == 0.0 && (p - shared).dot(q - shared) > 0.0 {
                    return false;
                }
            } else if segments_intersect(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn square() -> Vec<DVec2> {
        vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(100.0, 0.0),
            DVec2::new(100.0, 100.0),
            DVec2::new(0.0, 100.0),
        ]
    }

    #[test]
    fn square_measures() {
        assert_eq!(signed_area(&square()), 10_000.0);
        assert_eq!(perimeter(&square()), 400.0);
        let mut cw = square();
        cw.reverse();
        assert_eq!(signed_area(&cw), -10_000.0);
    }

    #[test]
    fn simple_and_bowtie() {
        assert!(is_simple(&square()));
        let bowtie = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(100.0, 100.0),
            DVec2::new(100.0, 0.0),
            DVec2::new(0.0, 100.0),
        ];
        assert!(!is_simple(&bowtie));
    }

    #[test]
    fn spike_is_not_simple() {
        // Edge goes out and comes straight back along itself.
        let spike = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(100.0, 0.0),
            DVec2::new(50.0, 0.0),
            DVec2::new(0.0, 100.0),
        ];
        assert!(!is_simple(&spike));
    }

    #[test]
    fn bounding_box_of_square() {
        let bb = bounding_box(&square());
        assert_eq!(
            bb,
            Some(Aabb {
                min: DVec2::ZERO,
                max: DVec2::new(100.0, 100.0)
            })
        );
        assert_eq!(bounding_box(&[]), None);
    }

    proptest! {
        #[test]
        fn area_invariant_under_translation(dx in -1e4..1e4f64, dy in -1e4..1e4f64) {
            let moved: Vec<_> = square().iter().map(|p| *p + DVec2::new(dx, dy)).collect();
            prop_assert!((signed_area(&moved) - 10_000.0).abs() < 1e-6);
            prop_assert!(is_simple(&moved));
        }

        #[test]
        fn rectangles_are_simple_with_positive_area(w in 0.1..5000.0f64, h in 0.1..5000.0f64) {
            let r = [DVec2::ZERO, DVec2::new(w, 0.0), DVec2::new(w, h), DVec2::new(0.0, h)];
            prop_assert!(is_simple(&r));
            prop_assert!((signed_area(&r) - w * h).abs() < 1e-6 * w * h);
        }
    }
}
