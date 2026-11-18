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

/// Convex hull (counter-clockwise, no collinear points), Andrew's monotone
/// chain. A tape measure around a body section follows this hull.
pub fn convex_hull(pts: &[DVec2]) -> Vec<DVec2> {
    let mut p: Vec<DVec2> = pts.iter().copied().filter(|v| v.is_finite()).collect();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let half = |points: &mut dyn Iterator<Item = DVec2>| {
        let mut chain: Vec<DVec2> = Vec::new();
        for q in points {
            while chain.len() >= 2
                && orient(chain[chain.len() - 2], chain[chain.len() - 1], q) <= 0.0
            {
                chain.pop();
            }
            chain.push(q);
        }
        chain.pop(); // shared with the other half
        chain
    };
    let mut hull = half(&mut p.iter().copied());
    hull.extend(half(&mut p.iter().rev().copied()));
    hull
}

/// Whether `p` lies inside the closed polygon (even-odd rule; points on
/// the boundary may go either way).
pub fn contains(pts: &[DVec2], p: DVec2) -> bool {
    let n = pts.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// Corners sharper than this miter ratio are beveled.
const MITER_LIMIT: f64 = 4.0;

/// Offset a **counter-clockwise** simple polygon outward by `distance` (mm),
/// with miter joins (beveled when sharper than [`MITER_LIMIT`]). Used for
/// seam allowances. Consecutive duplicate vertices are ignored.
///
/// Limitation: very tight concave corners relative to `distance` can make
/// the result locally self-overlapping. That is acceptable for drawing seam
/// allowances on garment panels, where radii are much larger than the
/// allowance.
pub fn offset_outward(pts: &[DVec2], distance: f64) -> Vec<DVec2> {
    let mut ring: Vec<DVec2> = Vec::with_capacity(pts.len());
    for p in pts {
        if ring.last() != Some(p) {
            ring.push(*p);
        }
    }
    while ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    let n = ring.len();
    if n < 3 || distance == 0.0 {
        return ring;
    }
    // Outward normal of a CCW edge is its right-hand normal.
    let normal = |a: DVec2, b: DVec2| {
        let d = (b - a).normalize_or_zero();
        DVec2::new(d.y, -d.x)
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prev = ring[(i + n - 1) % n];
        let cur = ring[i];
        let next = ring[(i + 1) % n];
        let n1 = normal(prev, cur);
        let n2 = normal(cur, next);
        let bisector = n1 + n2;
        let cos_half = bisector.length() * 0.5; // cos of half the turn angle
        if cos_half > 1.0 / MITER_LIMIT {
            out.push(cur + bisector.normalize_or_zero() * (distance / cos_half));
        } else {
            out.push(cur + n1 * distance);
            out.push(cur + n2 * distance);
        }
    }
    out
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

    #[test]
    fn convex_hull_of_square_with_inner_points() {
        let mut pts = square();
        pts.push(DVec2::new(50.0, 50.0));
        pts.push(DVec2::new(50.0, 0.0)); // collinear on an edge
        let hull = convex_hull(&pts);
        assert_eq!(hull.len(), 4);
        assert!((perimeter(&hull) - 400.0).abs() < 1e-9);
        assert!(signed_area(&hull) > 0.0);
    }

    #[test]
    fn contains_points() {
        assert!(contains(&square(), DVec2::new(50.0, 50.0)));
        assert!(!contains(&square(), DVec2::new(150.0, 50.0)));
        assert!(!contains(&square(), DVec2::new(-1.0, 50.0)));
    }

    #[test]
    fn offset_square_grows_by_distance() {
        let out = offset_outward(&square(), 10.0);
        assert_eq!(out.len(), 4);
        assert!((signed_area(&out) - 120.0 * 120.0).abs() < 1e-9);
        assert!(out[0].distance(DVec2::new(-10.0, -10.0)) < 1e-9);
    }

    #[test]
    fn offset_bevels_sharp_spikes() {
        let spike = [DVec2::ZERO, DVec2::new(100.0, 0.0), DVec2::new(0.0, 5.0)];
        let out = offset_outward(&spike, 10.0);
        assert!(out.len() > 3);
        assert!(out.iter().all(|p| p.length() < 150.0));
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
