//! The pieces of a Seamly2D/Valentina pattern: a closed list of nodes
//! (points and curves, through the `<modeling>` copies of the drafting
//! objects). A curve between two points lying on it runs from one to the
//! other; between two curves, from where they cross. Notches stand on
//! points; internal paths are open lists of nodes; a fold line unfolds
//! the piece.

use std::collections::HashMap;

use r3d_core::PathSegment;
use r3d_geom::DVec2;
use roxmltree::Node;

use super::curve::{Curve, Loc, dir};
use super::eval::{Draft, ON_CURVE_MM, Obj};
use crate::import::{ImportedPiece, Path};

/// One node of a path, resolved.
enum Item {
    Point { pos: DVec2, notch: bool, id: u32 },
    Curve(Curve),
}

/// Two points closer than this are one (mm).
const SAME_MM: f64 = 1e-3;

pub struct Pieces<'d, 'a> {
    pub draft: &'d Draft<'a>,
    /// Modeling id → drafting object id.
    pub modeling: HashMap<u32, u32>,
    /// Modeling paths (internal lines) by id.
    pub paths: HashMap<u32, Node<'d, 'd>>,
}

fn id(n: Node, attr: &str) -> Option<u32> {
    n.attribute(attr)?.trim().parse().ok().filter(|v| *v != 0)
}

impl Pieces<'_, '_> {
    fn object(&self, modeling: u32) -> Option<&Obj> {
        let calc = self.modeling.get(&modeling).copied().unwrap_or(modeling);
        self.draft.objs.get(&calc)
    }

    /// The items of a `<nodes>` list; `None` when one cannot be found.
    fn items(&self, nodes: Node) -> Option<Vec<Item>> {
        nodes
            .children()
            .filter(|c| c.has_tag_name("node") && c.attribute("excluded") != Some("true"))
            .map(|c| {
                let m = id(c, "idObject")?;
                match (c.attribute("type").unwrap_or("NodePoint"), self.object(m)?) {
                    ("NodePoint", Obj::Point { pos, .. }) => Some(Item::Point {
                        pos: *pos,
                        notch: c.attribute("passmark") == Some("true")
                            || c.attribute("notch") == Some("true"),
                        id: m,
                    }),
                    (_, Obj::Curve { curve, .. }) => {
                        let rev = matches!(c.attribute("reverse"), Some("1") | Some("true"));
                        Some(Item::Curve(if rev {
                            curve.reversed()
                        } else {
                            curve.clone()
                        }))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Where a curve item starts (`from_prev`) or ends, given its
    /// neighbour: at a point lying on it, or where a neighbouring curve
    /// crosses it.
    fn bound(c: &Curve, neighbour: Option<&Item>) -> Option<Loc> {
        match neighbour? {
            Item::Point { pos, .. } => c.locate(*pos, ON_CURVE_MM),
            Item::Curve(other) => {
                let ends = [c.start(), c.end()];
                c.crossings(other)
                    .into_iter()
                    .find(|(_, _, p)| ends.iter().flatten().all(|e| e.distance(*p) > SAME_MM))
                    .map(|(l, _, _)| l)
            }
        }
    }

    /// A list of items as a path; closed: the ends wrap around.
    fn path(items: &[Item], closed: bool) -> (Option<Path>, Vec<(u32, DVec2)>) {
        let n = items.len();
        let mut path: Option<Path> = None;
        let mut notches = Vec::new();
        let push_to = |path: &mut Option<Path>, p: DVec2| match path {
            None => {
                *path = Some(Path {
                    start: p,
                    segments: Vec::new(),
                })
            }
            Some(path) => {
                if path.end().distance(p) > SAME_MM {
                    path.segments.push(PathSegment::Line { to: p });
                }
            }
        };
        for (i, item) in items.iter().enumerate() {
            match item {
                Item::Point { pos, notch, id } => {
                    push_to(&mut path, *pos);
                    if *notch {
                        notches.push((*id, *pos));
                    }
                }
                Item::Curve(c) => {
                    let prev = if i > 0 {
                        items.get(i - 1)
                    } else if closed {
                        items.get(n.saturating_sub(1))
                    } else {
                        None
                    };
                    let next = if i + 1 < n {
                        items.get(i + 1)
                    } else if closed {
                        items.first()
                    } else {
                        None
                    };
                    let last = Loc {
                        seg: c.segs.len().saturating_sub(1),
                        t: 1.0,
                    };
                    let a = Self::bound(c, prev).unwrap_or(Loc { seg: 0, t: 0.0 });
                    let b = Self::bound(c, next).unwrap_or(last);
                    let part = if a < b { c.between(a, b) } else { c.clone() };
                    let Some(start) = part.start() else {
                        continue;
                    };
                    push_to(&mut path, start);
                    if let Some(path) = path.as_mut() {
                        for s in &part.segs {
                            if s.p0.distance(s.p3) <= SAME_MM && s.p1.distance(s.p0) <= SAME_MM {
                                continue;
                            }
                            path.segments.push(PathSegment::Cubic {
                                c1: s.p1,
                                c2: s.p2,
                                to: s.p3,
                            });
                        }
                    }
                }
            }
        }
        (path, notches)
    }

    /// The piece `<detail>` / `<piece>`; `None` when it uses something
    /// that could not be built, or has no area.
    pub fn piece(&self, d: Node) -> Option<ImportedPiece> {
        // Files from before 2016 list the nodes in the piece itself.
        let nodes = d.children().find(|c| c.has_tag_name("nodes")).unwrap_or(d);
        let items = self.items(nodes)?;
        let (path, notches) = Self::path(&items, true);
        let mut outline = crate::import::closed(path?);
        if outline.segments.len() < 2 {
            return None;
        }
        // A fold line: the piece is the half drawn and its mirror image.
        if let Some(m) = d.children().find(|c| c.has_tag_name("mirrorLine"))
            && d.attribute("showFullPiece") != Some("false")
            && let (Some(p1), Some(p2)) = (self.node_point(m, "p1"), self.node_point(m, "p2"))
        {
            outline = unfold(&outline, p1, p2).unwrap_or(outline);
        }
        let draft = self.draft;
        let seam_allowance_mm = if (d.attribute("seamAllowance") == Some("true")
            || d.attribute("supplement") == Some("1"))
            && d.attribute("seamAllowanceBuiltIn") != Some("true")
        {
            draft
                .formula(d.attribute("width").unwrap_or("0"), None)
                .map_or(0.0, |w| (w * draft.k).max(0.0))
        } else {
            0.0
        };
        let internal_lines = d
            .children()
            .filter(|c| c.has_tag_name("iPaths"))
            .flat_map(|c| c.children().filter(|r| r.has_tag_name("record")))
            .filter_map(|r| self.paths.get(&id(r, "path")?))
            .filter(|p| {
                p.attribute("visible")
                    .is_none_or(|v| draft.formula(v, None).is_ok_and(|v| v != 0.0))
            })
            .filter_map(|p| {
                let items = self.items(p.children().find(|c| c.has_tag_name("nodes"))?)?;
                Self::path(&items, false)
                    .0
                    .filter(|l| !l.segments.is_empty())
            })
            .collect();
        Some(ImportedPiece {
            name: d.attribute("name").map(str::to_owned),
            outline,
            seam_allowance_mm,
            grain: self.grain(d),
            notches: notches.into_iter().map(|(_, p)| p).collect(),
            internal_lines,
        })
    }

    fn node_point(&self, n: Node, attr: &str) -> Option<DVec2> {
        match self.object(id(n, attr)?)? {
            Obj::Point { pos, .. } => Some(*pos),
            Obj::Curve { .. } => None,
        }
    }

    /// The grainline: between its two pins, else its angle (degrees,
    /// formula; 90 by default).
    fn grain(&self, d: Node) -> Option<(DVec2, DVec2)> {
        let g = d.children().find(|c| c.has_tag_name("grainline"))?;
        let on = g
            .attribute("enabled")
            .or_else(|| g.attribute("visible"))
            .is_some_and(|v| v == "true");
        if !on {
            return None;
        }
        let pin = |a: &str, b: &str| self.node_point(g, a).or_else(|| self.node_point(g, b));
        if let (Some(bottom), Some(top)) =
            (pin("bottomPin", "bottomAnchor"), pin("topPin", "topAnchor"))
            && bottom.distance(top) > SAME_MM
        {
            return Some((bottom, top));
        }
        let angle = self
            .draft
            .formula(g.attribute("rotation").unwrap_or("90"), None)
            .ok()?;
        Some((DVec2::ZERO, dir(angle)))
    }
}

/// The piece with its half between the fold points `p1` and `p2`
/// mirrored across them, in place of the straight fold edge.
fn unfold(outline: &Path, p1: DVec2, p2: DVec2) -> Option<Path> {
    // Vertices and the segment arriving at each.
    let mut verts = vec![outline.start];
    verts.extend(outline.segments.iter().map(PathSegment::end));
    verts.pop();
    let n = verts.len();
    let i1 = verts.iter().position(|v| v.distance(p1) <= SAME_MM)?;
    let i2 = verts.iter().position(|v| v.distance(p2) <= SAME_MM)?;
    // Segment k runs from vertex k to vertex k + 1.
    let run = |from: usize, to: usize| -> Vec<(DVec2, PathSegment)> {
        let mut out = Vec::new();
        let mut k = from;
        while k != to {
            if let (Some(v), Some(s)) = (verts.get(k), outline.segments.get(k)) {
                out.push((*v, s.clone()));
            }
            k = (k + 1) % n;
        }
        out
    };
    let (a, b) = (run(i1, i2), run(i2, i1));
    // The fold is the side that is one straight edge.
    let straight = |r: &[(DVec2, PathSegment)]| matches!(r, [(_, PathSegment::Line { .. })]);
    let half = if straight(&b) {
        a
    } else if straight(&a) {
        b
    } else {
        return None;
    };
    let (start, end) = (half.first()?.0, half.last()?.1.end());
    let axis = end - start;
    let u = axis.try_normalize()?;
    let m = |p: DVec2| {
        let w = p - start;
        start + u * (2.0 * w.dot(u)) - w
    };
    let mut segments: Vec<PathSegment> = half.iter().map(|(_, s)| s.clone()).collect();
    // The mirror image, walked back from `end` to `start`.
    for (from, s) in half.iter().rev() {
        segments.push(match s {
            PathSegment::Line { .. } => PathSegment::Line { to: m(*from) },
            PathSegment::Cubic { c1, c2, .. } => PathSegment::Cubic {
                c1: m(*c2),
                c2: m(*c1),
                to: m(*from),
            },
        });
    }
    Some(Path { start, segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfolding_a_half() {
        // Half a rectangle against the fold x = 0, from (0,0) to (0,100).
        let half = Path::from_polyline(&[
            DVec2::new(0.0, 0.0),
            DVec2::new(50.0, 0.0),
            DVec2::new(50.0, 100.0),
            DVec2::new(0.0, 100.0),
            DVec2::new(0.0, 0.0),
        ])
        .unwrap();
        let full = unfold(&half, DVec2::ZERO, DVec2::new(0.0, 100.0)).unwrap();
        let pts = full.polyline();
        let area = r3d_geom::polygon::signed_area(&pts).abs();
        assert!((area - 10_000.0).abs() < 1e-6, "{area}");
        assert_eq!(full.end(), full.start);
    }
}
