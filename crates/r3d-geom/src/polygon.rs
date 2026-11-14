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

/// Triangulate a simple **counter-clockwise** polygon by ear clipping, for
/// display (filled panels). Returns index triples into `pts`. `O(n²)`, fine
/// for pattern outlines. Collinear vertices are handled; a polygon that is
/// not simple may come back partially triangulated rather than looping.
pub fn triangulate(pts: &[DVec2]) -> Vec<[usize; 3]> {
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    let mut out = Vec::with_capacity(pts.len().saturating_sub(2));
    let mut guard = 0;
    while idx.len() > 3 && guard < pts.len() * pts.len() {
        guard += 1;
        let n = idx.len();
        let ear = (0..n).find(|&i| {
            let (a, b, c) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
            let (pa, pb, pc) = (pts[a], pts[b], pts[c]);
            if orient(pa, pb, pc) <= 0.0 {
                return false; // reflex or collinear corner
            }
            idx.iter()
                .all(|&j| j == a || j == b || j == c || !point_in_triangle(pts[j], pa, pb, pc))
        });
        match ear {
            Some(i) => {
                out.push([idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]]);
                idx.remove(i);
            }
            None => {
                // Only collinear corners left: drop one and continue.
                let flat = (0..n).find(|&i| {
                    let (a, b, c) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
                    orient(pts[a], pts[b], pts[c]) == 0.0
                });
                match flat {
                    Some(i) => {
                        idx.remove(i);
                    }
                    None => break,
                }
            }
        }
    }
    if idx.len() == 3 && orient(pts[idx[0]], pts[idx[1]], pts[idx[2]]) > 0.0 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
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

fn point_in_triangle(p: DVec2, a: DVec2, b: DVec2, c: DVec2) -> bool {
    orient(a, b, p) >= 0.0 && orient(b, c, p) >= 0.0 && orient(c, a, p) >= 0.0
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

    fn triangulated_area(pts: &[DVec2]) -> f64 {
        triangulate(pts)
            .iter()
            .map(|t| signed_area(&[pts[t[0]], pts[t[1]], pts[t[2]]]))
            .sum()
    }

    #[test]
    fn contains_points() {
        assert!(contains(&square(), DVec2::new(50.0, 50.0)));
        assert!(!contains(&square(), DVec2::new(150.0, 50.0)));
        assert!(!contains(&square(), DVec2::new(-1.0, 50.0)));
    }

    #[test]
    fn triangulation_covers_convex_and_concave_shapes() {
        assert_eq!(triangulate(&square()).len(), 2);
        assert!((triangulated_area(&square()) - 10_000.0).abs() < 1e-9);
        // An L shape (concave) with a collinear vertex on its long side.
        let l = [
            DVec2::new(0.0, 0.0),
            DVec2::new(50.0, 0.0),
            DVec2::new(100.0, 0.0),
            DVec2::new(100.0, 50.0),
            DVec2::new(50.0, 50.0),
            DVec2::new(50.0, 100.0),
            DVec2::new(0.0, 100.0),
        ];
        assert!((triangulated_area(&l) - signed_area(&l)).abs() < 1e-9);
        for t in triangulate(&l) {
            assert!(signed_area(&[l[t[0]], l[t[1]], l[t[2]]]) > 0.0);
        }
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
        fn triangulation_preserves_area_of_regular_polygons(n in 3usize..64, r in 1.0..1000.0f64) {
            let pts: Vec<_> = (0..n)
                .map(|i| DVec2::from_angle(i as f64 / n as f64 * std::f64::consts::TAU) * r)
                .collect();
            prop_assert_eq!(triangulate(&pts).len(), n - 2);
            prop_assert!((triangulated_area(&pts) - signed_area(&pts)).abs() < 1e-6 * r * r);
        }

        #[test]
        fn rectangles_are_simple_with_positive_area(w in 0.1..5000.0f64, h in 0.1..5000.0f64) {
            let r = [DVec2::ZERO, DVec2::new(w, 0.0), DVec2::new(w, h), DVec2::new(0.0, h)];
            prop_assert!(is_simple(&r));
            prop_assert!((signed_area(&r) - w * h).abs() < 1e-6 * w * h);
        }
    }
}
