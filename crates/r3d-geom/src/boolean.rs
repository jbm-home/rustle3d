//! Boolean operations on regions bounded by polygons (with holes): union
//! and difference, for joining two pattern pieces or cutting one with
//! another.
//!
//! Both regions' loops go into one constrained Delaunay triangulation
//! (`spade`), split where they cross; each triangle is then inside or
//! outside the result (by its centroid, in each operand), and the result's
//! boundary is the set of triangle edges between inside and outside,
//! chained into loops: counter-clockwise outlines, clockwise holes. Robust
//! where loops cross or share stretches, as the triangulation resolves the
//! intersections exactly once.

use std::collections::BTreeMap;

use glam::DVec2;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::polygon::{contains, signed_area};

/// A region: an outline (counter-clockwise) and holes.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub outline: Vec<DVec2>,
    pub holes: Vec<Vec<DVec2>>,
}

impl Region {
    fn contains(&self, p: DVec2) -> bool {
        contains(&self.outline, p) && !self.holes.iter().any(|h| contains(h, p))
    }
}

/// What to keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// Inside either.
    Union,
    /// Inside the first, outside the second.
    Difference,
}

/// The loops bounding `op` of `a` and `b`: counter-clockwise outlines and
/// clockwise holes, in no particular order. Empty when nothing is left;
/// `None` when a loop cannot be triangulated.
pub fn boolean(a: &Region, b: &Region, op: Op) -> Option<Vec<Vec<DVec2>>> {
    let mut cdt: ConstrainedDelaunayTriangulation<Point2<f64>> =
        ConstrainedDelaunayTriangulation::new();
    for region in [a, b] {
        for ring in std::iter::once(&region.outline).chain(&region.holes) {
            if ring.len() < 3 || ring.iter().any(|p| !p.is_finite()) {
                return None;
            }
            let handles: Vec<_> = ring
                .iter()
                .map(|p| cdt.insert(Point2::new(p.x, p.y)).ok())
                .collect::<Option<_>>()?;
            for (i, h) in handles.iter().enumerate() {
                let to = *handles.get((i + 1) % handles.len())?;
                if *h != to {
                    // Where it crosses another loop, both are split there.
                    cdt.add_constraint_and_split(*h, to, |p| p);
                }
            }
        }
    }
    let keep = |p: DVec2| match op {
        Op::Union => a.contains(p) || b.contains(p),
        Op::Difference => a.contains(p) && !b.contains(p),
    };
    // Which faces are kept, by their fixed handle.
    let pos = |p: Point2<f64>| DVec2::new(p.x, p.y);
    let mut kept = std::collections::HashSet::new();
    for f in cdt.inner_faces() {
        let [x, y, z] = f.vertices().map(|v| pos(v.position()));
        if signed_area(&[x, y, z]).abs() > 1e-12 && keep((x + y + z) / 3.0) {
            kept.insert(f.fix());
        }
    }
    // Boundary edges, oriented with the kept face on their left: from →
    // to, by vertex index.
    let mut next: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut points: BTreeMap<usize, DVec2> = BTreeMap::new();
    for e in cdt.directed_edges() {
        let inside = e.face().as_inner().is_some_and(|f| kept.contains(&f.fix()));
        let outside = !e
            .rev()
            .face()
            .as_inner()
            .is_some_and(|f| kept.contains(&f.fix()));
        if inside && outside {
            let (from, to) = (e.from(), e.to());
            points.insert(from.fix().index(), pos(from.position()));
            points.insert(to.fix().index(), pos(to.position()));
            next.entry(from.fix().index())
                .or_default()
                .push(to.fix().index());
        }
    }
    // Chain them into loops.
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.iter().find(|(_, v)| !v.is_empty()) {
        let mut ring = Vec::new();
        let mut at = start;
        loop {
            ring.push(*points.get(&at)?);
            let to = next.get_mut(&at)?.pop()?;
            at = to;
            if at == start {
                break;
            }
            if ring.len() > points.len() {
                return None;
            }
        }
        loops.push(simplify(ring));
    }
    Some(loops.into_iter().filter(|l| l.len() >= 3).collect())
}

/// Drop points in line with their neighbours.
fn simplify(ring: Vec<DVec2>) -> Vec<DVec2> {
    let n = ring.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (Some(p), Some(a), Some(b)) = (
            ring.get(i),
            ring.get((i + n - 1) % n),
            ring.get((i + 1) % n),
        ) else {
            continue;
        };
        let (u, v) = (*p - *a, *b - *p);
        let straight = u.perp_dot(v).abs() <= 1e-9 * u.length() * v.length() && u.dot(v) > 0.0;
        if !straight {
            out.push(*p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64) -> Vec<DVec2> {
        vec![
            DVec2::new(x, y),
            DVec2::new(x + s, y),
            DVec2::new(x + s, y + s),
            DVec2::new(x, y + s),
        ]
    }

    fn region(outline: Vec<DVec2>) -> Region {
        Region {
            outline,
            holes: Vec::new(),
        }
    }

    #[test]
    fn union_of_overlapping_squares_is_one_outline() {
        let a = region(square(0.0, 0.0, 100.0));
        let b = region(square(50.0, 50.0, 100.0));
        let loops = boolean(&a, &b, Op::Union).unwrap();
        assert_eq!(loops.len(), 1);
        assert!((signed_area(&loops[0]) - (20_000.0 - 2_500.0)).abs() < 1e-6);
        assert_eq!(loops[0].len(), 8);
    }

    #[test]
    fn difference_cuts_a_notch_or_a_hole() {
        let a = region(square(0.0, 0.0, 100.0));
        let notch = region(square(80.0, 40.0, 40.0));
        let loops = boolean(&a, &notch, Op::Difference).unwrap();
        assert_eq!(loops.len(), 1);
        assert!((signed_area(&loops[0]) - (10_000.0 - 800.0)).abs() < 1e-6);
        let hole = region(square(30.0, 30.0, 20.0));
        let loops = boolean(&a, &hole, Op::Difference).unwrap();
        assert_eq!(loops.len(), 2);
        let areas: Vec<f64> = loops.iter().map(|l| signed_area(l)).collect();
        assert!(
            areas.contains(&10_000.0) && areas.contains(&-400.0),
            "{areas:?}"
        );
        // Nothing left.
        let all = region(square(-10.0, -10.0, 200.0));
        assert!(boolean(&a, &all, Op::Difference).unwrap().is_empty());
    }
}
