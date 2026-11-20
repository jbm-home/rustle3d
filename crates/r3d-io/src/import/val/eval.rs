//! Running the drafting steps of a Seamly2D/Valentina pattern: each
//! `<calculation>` element builds points and curves from earlier ones
//! with formulas (spec 28 lists the tools and their geometry). Geometry is
//! kept in millimetres with y up, so the files' angles (counter-clockwise
//! on screen, 0° to the right) are ordinary angles; formulas see lengths
//! in the pattern's unit.

use std::collections::{BTreeSet, HashMap, HashSet};

use r3d_geom::{CubicBez, DVec2};
use roxmltree::Node;

use super::curve::{Curve, Loc, angle_of, arc, ccw_sweep, dir, ellipse_arc};
use super::measure::Values;
use super::{ValError, formula};

/// What a curve is, for the names formulas give it.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// Spline or spline path between two named points.
    Spline,
    Path,
    Arc {
        center: DVec2,
        radius: f64,
    },
    ElArc,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    Point { pos: DVec2, name: String },
    Curve { curve: Curve, kind: Kind },
}

/// Tolerance for a point lying on a curve (mm).
pub const ON_CURVE_MM: f64 = 0.2;

/// Internal outcome of a step: an error, or a step skipped because it
/// uses an object that could not be built.
enum Fail {
    Error(ValError),
    Skip,
}

impl From<ValError> for Fail {
    fn from(e: ValError) -> Self {
        Fail::Error(e)
    }
}

type Step<T> = Result<T, Fail>;

pub struct Draft<'a> {
    /// Millimetres per pattern unit.
    pub k: f64,
    values: &'a Values,
    vars: HashMap<String, f64>,
    names: HashMap<String, DVec2>,
    pub objs: HashMap<u32, Obj>,
    absent: HashSet<u32>,
    /// Kinds of steps not supported, as written in the file.
    pub unsupported: BTreeSet<String>,
}

fn construction(node: Node, detail: impl Into<String>) -> Fail {
    Fail::Error(ValError::Construction {
        tool: node
            .attribute("type")
            .unwrap_or(node.tag_name().name())
            .to_owned(),
        id: node
            .attribute("id")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        detail: detail.into(),
    })
}

/// Where two infinite lines `a1 a2` and `b1 b2` meet.
pub fn line_line(a1: DVec2, a2: DVec2, b1: DVec2, b2: DVec2) -> Option<DVec2> {
    let (r, s) = (a2 - a1, b2 - b1);
    let d = r.perp_dot(s);
    if d.abs() <= 1e-12 * r.length() * s.length() || r.length() < 1e-12 {
        return None;
    }
    Some(a1 + r * ((b1 - a1).perp_dot(s) / d))
}

/// Where the infinite line through `a` along `u` (unit) meets a circle:
/// the parameters along `u`.
fn line_circle(a: DVec2, u: DVec2, c: DVec2, r: f64) -> Vec<f64> {
    let w = a - c;
    let b = w.dot(u);
    let disc = b * b - (w.length_squared() - r * r);
    if disc < -1e-9 {
        Vec::new()
    } else if disc.abs() <= 1e-9 {
        vec![-b]
    } else {
        let s = disc.sqrt();
        vec![-b - s, -b + s]
    }
}

/// Where two circles meet.
fn circle_circle(c1: DVec2, r1: f64, c2: DVec2, r2: f64) -> Vec<DVec2> {
    let d = c1.distance(c2);
    if d < 1e-9 || d > r1 + r2 + 1e-9 || d < (r1 - r2).abs() - 1e-9 {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h2 = r1 * r1 - a * a;
    let u = (c2 - c1) / d;
    let m = c1 + u * a;
    if h2 <= 1e-9 {
        return vec![m];
    }
    let h = h2.sqrt();
    vec![m + u.perp() * h, m - u.perp() * h]
}

/// Of two points, the one on the right of the line `from → to`
/// (`cross = 1`) or on its left (`2`); a lone point as it is.
fn by_side(points: &[DVec2], from: DVec2, to: DVec2, cross: &str) -> Option<DVec2> {
    let right = |p: &DVec2| (to - from).perp_dot(*p - from) < 0.0;
    match points {
        [p] => Some(*p),
        [_, _] => points
            .iter()
            .find(|p| if cross == "2" { !right(p) } else { right(p) })
            .copied(),
        _ => None,
    }
}

fn rotate_about(p: DVec2, o: DVec2, deg: f64) -> DVec2 {
    o + (p - o).rotate(dir(deg))
}

fn mirror(p: DVec2, a: DVec2, b: DVec2) -> DVec2 {
    let u = (b - a).normalize_or_zero();
    let w = p - a;
    a + u * (2.0 * w.dot(u)) - w
}

impl<'a> Draft<'a> {
    pub fn new(k: f64, values: &'a Values) -> Self {
        Draft {
            k,
            values,
            vars: HashMap::new(),
            names: HashMap::new(),
            objs: HashMap::new(),
            absent: HashSet::new(),
            unsupported: BTreeSet::new(),
        }
    }

    /// The value of a name in a formula (lengths in pattern units).
    fn lookup(&self, name: &str, current: Option<f64>) -> Option<f64> {
        if name == "CurrentLength" || name == "CurrentSeamAllowance" {
            return current;
        }
        if let Some(v) = self.vars.get(name) {
            return Some(*v);
        }
        if let Some(v) = self.values.get(name) {
            return Some(if v.length { v.value / self.k } else { v.value });
        }
        let (rest, angle) = match name.strip_prefix("AngleLine_") {
            Some(r) => (r, true),
            None => (name.strip_prefix("Line_")?, false),
        };
        // Point names may hold `_`: try each split.
        rest.match_indices('_').find_map(|(i, _)| {
            let a = self.names.get(rest.get(..i)?)?;
            let b = self.names.get(rest.get(i + 1..)?)?;
            Some(if angle {
                angle_of(*b - *a)
            } else {
                a.distance(*b) / self.k
            })
        })
    }

    /// Compute a formula.
    pub fn formula(&self, src: &str, current: Option<f64>) -> Result<f64, ValError> {
        formula::eval(src, &|n| self.lookup(n, current))
    }

    /// An attribute's formula (`default` when absent).
    fn f(&self, n: Node, attr: &str, default: &str, current: Option<f64>) -> Step<f64> {
        Ok(self.formula(n.attribute(attr).unwrap_or(default), current)?)
    }

    /// An attribute's formula as a length (mm).
    fn len(&self, n: Node, attr: &str, default: &str, current: Option<f64>) -> Step<f64> {
        Ok(self.f(n, attr, default, current)? * self.k)
    }

    fn num(n: Node, attr: &str, default: f64) -> f64 {
        n.attribute(attr)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default)
    }

    fn id(n: Node, attr: &str) -> Option<u32> {
        n.attribute(attr)?.trim().parse().ok().filter(|v| *v != 0)
    }

    /// The object `id` refers to.
    fn obj(&self, id: Option<u32>) -> Step<&Obj> {
        let Some(id) = id else {
            return Err(Fail::Error(ValError::MissingObject { id: 0 }));
        };
        if self.absent.contains(&id) {
            return Err(Fail::Skip);
        }
        self.objs
            .get(&id)
            .ok_or(Fail::Error(ValError::MissingObject { id }))
    }

    fn point(&self, n: Node, attr: &str) -> Step<DVec2> {
        match self.obj(Self::id(n, attr))? {
            Obj::Point { pos, .. } => Ok(*pos),
            Obj::Curve { .. } => Err(construction(n, format!("`{attr}` is not a point"))),
        }
    }

    fn point_name(&self, n: Node, attr: &str) -> String {
        match self.obj(Self::id(n, attr)) {
            Ok(Obj::Point { name, .. }) => name.clone(),
            _ => String::new(),
        }
    }

    fn curve(&self, n: Node, attr: &str) -> Step<(Curve, Kind)> {
        match self.obj(Self::id(n, attr))? {
            Obj::Curve { curve, kind } => Ok((curve.clone(), kind.clone())),
            Obj::Point { .. } => Err(construction(n, format!("`{attr}` is not a curve"))),
        }
    }

    pub fn add_point(&mut self, id: u32, name: &str, pos: DVec2) {
        if !name.is_empty() {
            self.names.insert(name.to_owned(), pos);
        }
        self.objs.insert(
            id,
            Obj::Point {
                pos,
                name: name.to_owned(),
            },
        );
    }

    fn add_curve(&mut self, id: Option<u32>, curve: Curve, kind: Kind) {
        if let Some(id) = id {
            self.objs.insert(id, Obj::Curve { curve, kind });
        }
    }

    fn set(&mut self, name: String, v: f64) {
        self.vars.insert(name, v);
    }

    /// Names for a spline or spline path (`Spl_A_B`, `SplPath_A_B`): its
    /// length, the angles and lengths of its end handles, and for a path
    /// each segment's.
    fn name_spline(&mut self, prefix: &str, base: &str, c: &Curve) {
        let k = self.k;
        let (Some(first), Some(last)) = (c.segs.first(), c.segs.last()) else {
            return;
        };
        self.set(format!("{prefix}{base}"), c.length() / k);
        self.set(
            format!("Angle1{prefix}{base}"),
            angle_of(first.p1 - first.p0),
        );
        self.set(format!("Angle2{prefix}{base}"), angle_of(last.p2 - last.p3));
        self.set(
            format!("C1Length{prefix}{base}"),
            first.p1.distance(first.p0) / k,
        );
        self.set(
            format!("C2Length{prefix}{base}"),
            last.p2.distance(last.p3) / k,
        );
        if prefix == "SplPath_" {
            for (i, s) in c.segs.iter().enumerate() {
                let seg = format!("{base}_Seg_{}", i + 1);
                self.set(format!("{prefix}{seg}"), s.arc_length(0.01) / k);
                self.set(format!("Angle1{prefix}{seg}"), angle_of(s.p1 - s.p0));
                self.set(format!("Angle2{prefix}{seg}"), angle_of(s.p2 - s.p3));
                self.set(format!("C1Length{prefix}{seg}"), s.p1.distance(s.p0) / k);
                self.set(format!("C2Length{prefix}{seg}"), s.p2.distance(s.p3) / k);
            }
        }
    }

    /// Run the steps of one `<calculation>`.
    pub fn calculation(&mut self, calc: Node) -> Result<(), ValError> {
        for n in calc.children().filter(Node::is_element) {
            let tag = n.tag_name().name();
            let kind = n.attribute("type").unwrap_or("");
            let result = match tag {
                "point" => self.point_tool(n, kind),
                "line" => Ok(()),
                "spline" => self.spline_tool(n, kind),
                "arc" => self.arc_tool(n, kind),
                "elArc" => self.el_arc_tool(n, kind),
                "operation" => self.operation(n, kind),
                _ => self.unsupported(n),
            };
            match result {
                Ok(()) => {}
                Err(Fail::Error(e)) => return Err(e),
                Err(Fail::Skip) => self.mark_absent(n),
            }
        }
        Ok(())
    }

    /// A step that could not be built: the objects it would have made are
    /// absent, and so are the steps that use them.
    fn mark_absent(&mut self, n: Node) {
        for attr in [
            "id",
            "point1",
            "point2",
            "segment1Id",
            "segment2Id",
            "segment3Id",
            "segment4Id",
        ] {
            if let Some(id) = Self::id(n, attr) {
                self.absent.insert(id);
            }
        }
        for d in n.descendants().filter(|d| {
            d.has_tag_name("item") && d.parent().is_some_and(|p| p.has_tag_name("destination"))
        }) {
            if let Some(id) = Self::id(d, "idObject") {
                self.absent.insert(id);
            }
        }
    }

    fn unsupported(&mut self, n: Node) -> Step<()> {
        self.unsupported.insert(format!(
            "{}:{}",
            n.tag_name().name(),
            n.attribute("type").unwrap_or("")
        ));
        Err(Fail::Skip)
    }

    fn point_tool(&mut self, n: Node, kind: &str) -> Step<()> {
        let id = Self::id(n, "id").ok_or(Fail::Error(ValError::MissingObject { id: 0 }))?;
        let name = n.attribute("name").unwrap_or("").to_owned();
        let pos = match kind {
            "single" => DVec2::new(
                Self::num(n, "x", 10.0) * self.k,
                -Self::num(n, "y", 10.0) * self.k,
            ),
            "endLine" => {
                let b = self.point(n, "basePoint")?;
                b + dir(self.f(n, "angle", "0", None)?) * self.len(n, "length", "100", None)?
            }
            "alongLine" => {
                let a = self.point(n, "firstPoint")?;
                let c = self.point(n, "secondPoint")?;
                let u = (c - a)
                    .try_normalize()
                    .ok_or_else(|| construction(n, "the two points are the same"))?;
                a + u * self.len(n, "length", "0", Some(a.distance(c) / self.k))?
            }
            "normal" => {
                let a = self.point(n, "firstPoint")?;
                let c = self.point(n, "secondPoint")?;
                let d = angle_of(c - a) + 90.0 + Self::num(n, "angle", 0.0);
                a + dir(d) * self.len(n, "length", "0", None)?
            }
            "bisector" => {
                let a = self.point(n, "firstPoint")?;
                let b = self.point(n, "secondPoint")?;
                let c = self.point(n, "thirdPoint")?;
                let alpha = angle_of(a - b);
                let theta = (angle_of(c - b) - alpha).rem_euclid(360.0);
                let d = if theta <= 180.0 {
                    alpha + theta / 2.0
                } else {
                    alpha - (360.0 - theta) / 2.0
                };
                b + dir(d) * self.len(n, "length", "0", None)?
            }
            "shoulder" => {
                let a = self.point(n, "p1Line")?;
                let b = self.point(n, "p2Line")?;
                let s = self.point(n, "pShoulder")?;
                let r = self.len(n, "length", "0", None)?;
                let u = (b - a)
                    .try_normalize()
                    .ok_or_else(|| construction(n, "the line has no length"))?;
                if r <= 0.0 {
                    b
                } else {
                    line_circle(a, u, s, r)
                        .into_iter()
                        .filter(|t| *t >= 0.0)
                        .fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.max(t))))
                        .map_or(b, |t| a + u * t)
                }
            }
            "lineIntersect" => line_line(
                self.point(n, "p1Line1")?,
                self.point(n, "p2Line1")?,
                self.point(n, "p1Line2")?,
                self.point(n, "p2Line2")?,
            )
            .ok_or_else(|| construction(n, "the lines are parallel"))?,
            "pointOfContact" => {
                let o = self.point(n, "center")?;
                let r = self.len(n, "radius", "0", None)?.abs();
                let a = self.point(n, "firstPoint")?;
                let b = self.point(n, "secondPoint")?;
                let u = (b - a)
                    .try_normalize()
                    .ok_or_else(|| construction(n, "the line has no length"))?;
                let ts = line_circle(a, u, o, r);
                let ab = a.distance(b);
                let on: Vec<f64> = ts
                    .iter()
                    .copied()
                    .filter(|t| (-1e-9..=ab + 1e-9).contains(t))
                    .collect();
                let t = match (ts.as_slice(), on.as_slice()) {
                    ([t], _) | (_, [t]) => *t,
                    ([t0, t1], _) => {
                        if t0.abs() <= t1.abs() {
                            *t0
                        } else {
                            *t1
                        }
                    }
                    _ => return Err(construction(n, "the line misses the circle")),
                };
                a + u * t
            }
            "height" => {
                let b = self.point(n, "basePoint")?;
                let a = self.point(n, "p1Line")?;
                let c = self.point(n, "p2Line")?;
                let u = (c - a)
                    .try_normalize()
                    .ok_or_else(|| construction(n, "the line has no length"))?;
                a + u * (b - a).dot(u)
            }
            "triangle" => {
                let a1 = self.point(n, "axisP1")?;
                let a2 = self.point(n, "axisP2")?;
                let f = self.point(n, "firstPoint")?;
                let s = self.point(n, "secondPoint")?;
                let u = (a2 - a1)
                    .try_normalize()
                    .ok_or_else(|| construction(n, "the axis has no length"))?;
                let q = line_line(a1, a2, f, s)
                    .ok_or_else(|| construction(n, "the axis is parallel to the points"))?;
                let sq = (q - a1).dot(u);
                line_circle(a1, u, (f + s) / 2.0, f.distance(s) / 2.0)
                    .into_iter()
                    .filter(|t| *t >= sq - 1e-9)
                    .fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.min(t))))
                    .map(|t| a1 + u * t)
                    .ok_or_else(|| construction(n, "no right angle on the axis"))?
            }
            "pointOfIntersection" | "intersectXY" => {
                let a = self.point(n, "firstPoint")?;
                let b = self.point(n, "secondPoint")?;
                DVec2::new(a.x, b.y)
            }
            "lineIntersectAxis" => {
                let b = self.point(n, "basePoint")?;
                let d = dir(self.f(n, "angle", "0", None)?);
                line_line(b, b + d, self.point(n, "p1Line")?, self.point(n, "p2Line")?)
                    .ok_or_else(|| construction(n, "the axis is parallel to the line"))?
            }
            "curveIntersectAxis" => {
                let b = self.point(n, "basePoint")?;
                let d = dir(self.f(n, "angle", "0", None)?);
                let (c, kind) = self.curve(n, "curve")?;
                let axis = Curve {
                    segs: vec![CubicBez::line(b - d * 1e6, b + d * 1e6)],
                };
                let hits: Vec<(Loc, DVec2)> = c
                    .crossings(&axis)
                    .into_iter()
                    .map(|(l, _, p)| (l, p))
                    .filter(|(_, p)| p.distance(b) > 1e-6)
                    .collect();
                let pick = |forward: bool| {
                    hits.iter()
                        .filter(|(_, p)| ((*p - b).dot(d) > 0.0) == forward)
                        .min_by(|x, y| x.1.distance(b).total_cmp(&y.1.distance(b)))
                        .copied()
                };
                let (at, p) = match pick(true).or_else(|| pick(false)) {
                    Some(hit) => hit,
                    None => match c.locate(b, ON_CURVE_MM) {
                        Some(l) => (l, b),
                        None => return Err(construction(n, "the axis misses the curve")),
                    },
                };
                self.split_curve(n, &c, &kind, at, &name, ("segment1Id", "segment2Id"));
                p
            }
            "cutArc" | "cutSpline" | "cutSplinePath" => {
                let attr = match kind {
                    "cutArc" => "arc",
                    "cutSpline" => "spline",
                    _ => "splinePath",
                };
                let (c, ckind) = self.curve(n, attr)?;
                let total = c.length();
                let mut l = self.len(n, "length", "0", Some(total / self.k))?;
                if n.attribute("direction") == Some("backward") {
                    l = total - l;
                } else if l < 0.0 {
                    l += total;
                }
                let at = c.at_length(l.clamp(0.0, total));
                let p = c
                    .point(at)
                    .ok_or_else(|| construction(n, "the curve is empty"))?;
                self.split_curve(n, &c, &ckind, at, &name, ("segment1Id", "segment2Id"));
                p
            }
            "pointOfIntersectionArcs" => {
                let (c1, k1) = self.curve(n, "firstArc")?;
                let (c2, k2) = self.curve(n, "secondArc")?;
                let (
                    Kind::Arc {
                        center: o1,
                        radius: r1,
                    },
                    Kind::Arc {
                        center: o2,
                        radius: r2,
                    },
                ) = (&k1, &k2)
                else {
                    return Err(construction(n, "not two arcs"));
                };
                let on: Vec<DVec2> = circle_circle(*o1, *r1, *o2, *r2)
                    .into_iter()
                    .filter(|p| {
                        c1.locate(*p, ON_CURVE_MM).is_some() && c2.locate(*p, ON_CURVE_MM).is_some()
                    })
                    .collect();
                let p = by_side(&on, *o1, *o2, n.attribute("crossPoint").unwrap_or("1"))
                    .ok_or_else(|| construction(n, "the arcs do not meet"))?;
                if let Some(at) = c1.locate(p, ON_CURVE_MM) {
                    self.split_curve(n, &c1, &k1, at, &name, ("segment1Id", "segment2Id"));
                }
                if let Some(at) = c2.locate(p, ON_CURVE_MM) {
                    self.split_curve(n, &c2, &k2, at, &name, ("segment3Id", "segment4Id"));
                }
                p
            }
            "pointOfIntersectionCircles" => {
                let o1 = self.point(n, "c1Center")?;
                let o2 = self.point(n, "c2Center")?;
                let r1 = self.len(n, "c1Radius", "0", None)?.abs();
                let r2 = self.len(n, "c2Radius", "0", None)?.abs();
                by_side(
                    &circle_circle(o1, r1, o2, r2),
                    o1,
                    o2,
                    n.attribute("crossPoint").unwrap_or("1"),
                )
                .ok_or_else(|| construction(n, "the circles do not meet"))?
            }
            "pointFromCircleAndTangent" | "pointFromArcAndTangent" => {
                let t = self.point(n, "tangent")?;
                let (o, r, on_arc) = if kind == "pointFromArcAndTangent" {
                    let (c, k) = self.curve(n, "arc")?;
                    let Kind::Arc { center, radius } = k else {
                        return Err(construction(n, "not an arc"));
                    };
                    (center, radius, Some(c))
                } else {
                    (
                        self.point(n, "cCenter")?,
                        self.len(n, "cRadius", "0", None)?.abs(),
                        None,
                    )
                };
                let d = t.distance(o);
                if d < r - 1e-6 {
                    return Err(construction(n, "the point is inside the circle"));
                }
                let mut pts = if (d - r).abs() <= 1e-6 {
                    vec![t]
                } else {
                    circle_circle(t, (d * d - r * r).sqrt(), o, r)
                };
                if let Some(c) = on_arc {
                    pts.retain(|p| c.locate(*p, ON_CURVE_MM).is_some());
                }
                by_side(&pts, t, o, n.attribute("crossPoint").unwrap_or("1"))
                    .ok_or_else(|| construction(n, "no tangent point"))?
            }
            "pointOfIntersectionCurves" => {
                let (c1, k1) = self.curve(n, "curve1")?;
                let (c2, k2) = self.curve(n, "curve2")?;
                let hits = c1.crossings(&c2);
                let v = n.attribute("vCrossPoint").unwrap_or("1");
                let h = n.attribute("hCrossPoint").unwrap_or("1");
                let best = hits.iter().min_by(|a, b| {
                    let (pa, pb) = (a.2, b.2);
                    let vy = if v == "2" {
                        pa.y.total_cmp(&pb.y)
                    } else {
                        pb.y.total_cmp(&pa.y)
                    };
                    let close = (pa.y - pb.y).abs() < 1e-6;
                    if !close {
                        return vy;
                    }
                    if h == "2" {
                        pb.x.total_cmp(&pa.x)
                    } else {
                        pa.x.total_cmp(&pb.x)
                    }
                });
                let (l1, l2, p) = *best.ok_or_else(|| construction(n, "the curves do not meet"))?;
                self.split_curve(n, &c1, &k1, l1, &name, ("segment1Id", "segment2Id"));
                self.split_curve(n, &c2, &k2, l2, &name, ("segment3Id", "segment4Id"));
                p
            }
            "trueDarts" => return self.true_darts(n),
            _ => return self.unsupported(n),
        };
        self.add_point(id, &name, pos);
        Ok(())
    }

    /// The two parts of a curve cut at `at` by point `cut`: kept under the
    /// ids the file gives them (else, for an arc, the ids after the
    /// tool's), with their names for formulas.
    fn split_curve(
        &mut self,
        n: Node,
        c: &Curve,
        kind: &Kind,
        at: Loc,
        cut: &str,
        ids: (&str, &str),
    ) {
        let (a, b) = c.split(at);
        let id = Self::id(n, "id");
        let implicit = |k: u32| match kind {
            Kind::Arc { .. } if ids.0 == "segment1Id" => id.map(|i| i + k),
            Kind::Arc { .. } => id.map(|i| i + k + 2),
            _ => None,
        };
        let id1 = Self::id(n, ids.0).or_else(|| implicit(1));
        let id2 = Self::id(n, ids.1).or_else(|| implicit(2));
        let (start, end) = (self.name_at(c.start()), self.name_at(c.end()));
        match kind {
            Kind::Spline => {
                self.name_spline("Spl_", &format!("{start}_{cut}"), &a);
                self.name_spline("Spl_", &format!("{cut}_{end}"), &b);
            }
            Kind::Path => {
                self.name_spline("SplPath_", &format!("{start}_{cut}"), &a);
                self.name_spline("SplPath_", &format!("{cut}_{end}"), &b);
            }
            Kind::Arc { .. } | Kind::ElArc => {}
        }
        self.add_curve(id1, a, kind.clone());
        self.add_curve(id2, b, kind.clone());
    }

    /// The name of the point at `p`, if one is there.
    fn name_at(&self, p: Option<DVec2>) -> String {
        p.and_then(|p| {
            self.names
                .iter()
                .find(|(_, q)| q.distance(p) < 1e-6)
                .map(|(n, _)| n.clone())
        })
        .unwrap_or_default()
    }

    fn true_darts(&mut self, n: Node) -> Step<()> {
        let b1 = self.point(n, "baseLineP1")?;
        let b2 = self.point(n, "baseLineP2")?;
        let d1 = self.point(n, "dartP1")?;
        let d2 = self.point(n, "dartP2")?;
        let d3 = self.point(n, "dartP3")?;
        let delta = angle_of(d1 - d2) - angle_of(d3 - d2);
        let b2r = rotate_about(b2, d2, delta);
        let p1 = line_line(b1, b2r, d2, d1)
            .ok_or_else(|| construction(n, "the dart's leg is parallel to the base line"))?;
        let p2 = rotate_about(p1, d2, -delta);
        let (Some(i1), Some(i2)) = (Self::id(n, "point1"), Self::id(n, "point2")) else {
            return Err(construction(n, "no ids for the two points"));
        };
        self.add_point(i1, n.attribute("name1").unwrap_or(""), p1);
        self.add_point(i2, n.attribute("name2").unwrap_or(""), p2);
        Ok(())
    }

    /// A control handle: `length` (formula, pattern units) along `angle`
    /// (formula, degrees) from `p`; a zero length is made 0.01 mm.
    fn handle(&self, n: Node, p: DVec2, angle: &str, length: &str) -> Step<DVec2> {
        let mut l = self.len(n, length, "0", None)?;
        if l == 0.0 {
            l = 0.01;
        }
        Ok(p + dir(self.f(n, angle, "0", None)?) * l)
    }

    /// The handle lengths of the old splines: `kCurve · |P0P3| / √2 ·
    /// 4/3 · tan(22.5°)`, times each end's asymmetry.
    fn old_handles(
        p0: DVec2,
        p3: DVec2,
        a1: f64,
        a2: f64,
        kasm: (f64, f64),
        kcurve: f64,
    ) -> CubicBez {
        let mut chord = p0.distance(p3);
        if chord < 1e-9 {
            chord = 2.0 * 0.1555;
        }
        let l = kcurve * chord / 2f64.sqrt() * 4.0 / 3.0 * 22.5f64.to_radians().tan();
        CubicBez::new(p0, p0 + dir(a1) * l * kasm.0, p3 + dir(a2) * l * kasm.1, p3)
    }

    fn spline_tool(&mut self, n: Node, kind: &str) -> Step<()> {
        let id = Self::id(n, "id");
        let pts = n
            .children()
            .filter(|c| c.has_tag_name("pathPoint"))
            .collect::<Vec<_>>();
        let (curve, path, first, last) = match kind {
            "simpleInteractive" => {
                let p0 = self.point(n, "point1")?;
                let p3 = self.point(n, "point4")?;
                let c1 = self.handle(n, p0, "angle1", "length1")?;
                let c2 = self.handle(n, p3, "angle2", "length2")?;
                (
                    Curve {
                        segs: vec![CubicBez::new(p0, c1, c2, p3)],
                    },
                    false,
                    self.point_name(n, "point1"),
                    self.point_name(n, "point4"),
                )
            }
            "simple" => {
                let p0 = self.point(n, "point1")?;
                let p3 = self.point(n, "point4")?;
                let seg = Self::old_handles(
                    p0,
                    p3,
                    Self::num(n, "angle1", 270.0),
                    Self::num(n, "angle2", 90.0),
                    (Self::num(n, "kAsm1", 1.0), Self::num(n, "kAsm2", 1.0)),
                    Self::num(n, "kCurve", 1.0),
                );
                (
                    Curve { segs: vec![seg] },
                    false,
                    self.point_name(n, "point1"),
                    self.point_name(n, "point4"),
                )
            }
            "cubicBezier" => (
                Curve {
                    segs: vec![CubicBez::new(
                        self.point(n, "point1")?,
                        self.point(n, "point2")?,
                        self.point(n, "point3")?,
                        self.point(n, "point4")?,
                    )],
                },
                false,
                self.point_name(n, "point1"),
                self.point_name(n, "point4"),
            ),
            "pathInteractive" => {
                let list = &pts;
                let mut segs = Vec::new();
                for w in list.windows(2) {
                    let (Some(a), Some(b)) = (w.first(), w.get(1)) else {
                        continue;
                    };
                    let p0 = self.point(*a, "pSpline")?;
                    let p3 = self.point(*b, "pSpline")?;
                    let c1 = self.handle(*a, p0, "angle2", "length2")?;
                    let c2 = self.handle(*b, p3, "angle1", "length1")?;
                    segs.push(CubicBez::new(p0, c1, c2, p3));
                }
                let names = (
                    list.first().map(|p| self.point_name(*p, "pSpline")),
                    list.last().map(|p| self.point_name(*p, "pSpline")),
                );
                (
                    Curve { segs },
                    true,
                    names.0.unwrap_or_default(),
                    names.1.unwrap_or_default(),
                )
            }
            "path" => {
                let list = &pts;
                let kcurve = Self::num(n, "kCurve", 1.0);
                let mut segs = Vec::new();
                for w in list.windows(2) {
                    let (Some(a), Some(b)) = (w.first(), w.get(1)) else {
                        continue;
                    };
                    segs.push(Self::old_handles(
                        self.point(*a, "pSpline")?,
                        self.point(*b, "pSpline")?,
                        Self::num(*a, "angle", 0.0),
                        Self::num(*b, "angle", 0.0) + 180.0,
                        (Self::num(*a, "kAsm2", 1.0), Self::num(*b, "kAsm1", 1.0)),
                        kcurve,
                    ));
                }
                let names = (
                    list.first().map(|p| self.point_name(*p, "pSpline")),
                    list.last().map(|p| self.point_name(*p, "pSpline")),
                );
                (
                    Curve { segs },
                    true,
                    names.0.unwrap_or_default(),
                    names.1.unwrap_or_default(),
                )
            }
            "cubicBezierPath" => {
                let list = &pts;
                let p: Vec<DVec2> = list
                    .iter()
                    .map(|q| self.point(*q, "pSpline"))
                    .collect::<Step<_>>()?;
                let mut segs: Vec<CubicBez> = Vec::new();
                for k in 0..p.len().saturating_sub(1) / 3 {
                    let (Some(p0), Some(c1), Some(c2), Some(p3)) = (
                        p.get(3 * k),
                        p.get(3 * k + 1),
                        p.get(3 * k + 2),
                        p.get(3 * k + 3),
                    ) else {
                        break;
                    };
                    let mut c1 = *c1;
                    // Smooth joints: the first handle goes on with the
                    // previous segment's last one.
                    if let Some(prev) = segs.last()
                        && let Some(d) = (*p0 - prev.p2).try_normalize()
                    {
                        c1 = *p0 + d * c1.distance(*p0);
                    }
                    segs.push(CubicBez::new(*p0, c1, *c2, *p3));
                }
                let names = (
                    list.first().map(|q| self.point_name(*q, "pSpline")),
                    list.last().map(|q| self.point_name(*q, "pSpline")),
                );
                (
                    Curve { segs },
                    true,
                    names.0.unwrap_or_default(),
                    names.1.unwrap_or_default(),
                )
            }
            _ => return self.unsupported(n),
        };
        if curve.segs.is_empty() {
            return Err(construction(n, "the curve has no segment"));
        }
        let (prefix, kind) = if path {
            ("SplPath_", Kind::Path)
        } else {
            ("Spl_", Kind::Spline)
        };
        let mut base = format!("{first}_{last}");
        if let Some(d) = n
            .attribute("duplicate")
            .filter(|d| !d.is_empty() && *d != "0")
        {
            base = format!("{base}_{d}");
        }
        self.name_spline(prefix, &base, &curve);
        if let Some(alias) = n.attribute("alias").filter(|a| !a.is_empty()) {
            self.name_spline(prefix, alias, &curve);
        }
        self.add_curve(id, curve, kind);
        Ok(())
    }

    fn arc_tool(&mut self, n: Node, kind: &str) -> Step<()> {
        let id = Self::id(n, "id");
        let o = self.point(n, "center")?;
        let r = self.len(n, "radius", "10", None)?;
        let f1 = self.f(n, "angle1", "180", None)?;
        let (sweep, f2) = match kind {
            "simple" => {
                let f2 = self.f(n, "angle2", "270", None)?;
                let s = ccw_sweep(f1, f2);
                (if r < 0.0 { -(360.0 - s) } else { s }, f2)
            }
            "arcWithLength" => {
                if r == 0.0 {
                    return Err(construction(n, "the radius is zero"));
                }
                let l = self.len(n, "length", "0", None)?;
                let s = (l / r).to_degrees().clamp(-360.0, 360.0);
                (s, f1 + s)
            }
            _ => return self.unsupported(n),
        };
        if r == 0.0 {
            return Err(construction(n, "the radius is zero"));
        }
        let curve = arc(o, r.abs(), f1, sweep);
        if let Some(i) = id {
            let c = self.point_name(n, "center");
            let base = format!("{c}_{i}");
            self.set(
                format!("Arc_{base}"),
                r.abs() * sweep.abs().to_radians() / self.k,
            );
            self.set(format!("Angle1Arc_{base}"), f1);
            self.set(format!("Angle2Arc_{base}"), f2);
            self.set(format!("RadiusArc_{base}"), r.abs() / self.k);
        }
        self.add_curve(
            id,
            curve,
            Kind::Arc {
                center: o,
                radius: r.abs(),
            },
        );
        Ok(())
    }

    fn el_arc_tool(&mut self, n: Node, kind: &str) -> Step<()> {
        if kind != "simple" {
            return self.unsupported(n);
        }
        let id = Self::id(n, "id");
        let o = self.point(n, "center")?;
        let a = self.len(n, "radius1", "10", None)?;
        let b = self.len(n, "radius2", "10", None)?;
        let f1 = self.f(n, "angle1", "180", None)?;
        let f2 = self.f(n, "angle2", "270", None)?;
        let rot = self.f(n, "rotationAngle", "0", None)?;
        if a == 0.0 || b == 0.0 {
            return Err(construction(n, "a radius is zero"));
        }
        // Polar angles to the ellipse's own parameter.
        let param = |f: f64| {
            let f = f.to_radians();
            (a.abs() * f.sin()).atan2(b.abs() * f.cos())
        };
        let (t1, t2) = (param(f1), param(f2));
        let s = ccw_sweep(t1.to_degrees(), t2.to_degrees()).to_radians();
        let sweep = if a < 0.0 || b < 0.0 {
            -(std::f64::consts::TAU - s)
        } else {
            s
        };
        let curve = ellipse_arc(o, a.abs(), b.abs(), rot, t1, sweep);
        if let Some(i) = id {
            let c = self.point_name(n, "center");
            let base = format!("{c}_{i}");
            self.set(format!("ElArc_{base}"), curve.length() / self.k);
            self.set(format!("Radius1ElArc_{base}"), a.abs() / self.k);
            self.set(format!("Radius2ElArc_{base}"), b.abs() / self.k);
            self.set(format!("Angle1ElArc_{base}"), f1);
            self.set(format!("Angle2ElArc_{base}"), f2);
            self.set(format!("RotationElArc_{base}"), rot);
        }
        self.add_curve(id, curve, Kind::ElArc);
        Ok(())
    }

    /// Rotation, mirror and move of a set of objects: each source gives
    /// the destination in the same place in the list.
    fn operation(&mut self, n: Node, kind: &str) -> Step<()> {
        let items = |tag: &str| {
            n.children()
                .find(|c| c.has_tag_name(tag))
                .map(|s| {
                    s.children()
                        .filter(|c| c.has_tag_name("item"))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        let (sources, dests) = (items("source"), items("destination"));
        let objs: Vec<Obj> = sources
            .iter()
            .map(|s| self.obj(Self::id(*s, "idObject")).cloned())
            .collect::<Step<_>>()?;
        let map: Box<dyn Fn(DVec2) -> DVec2> = match kind {
            "rotation" => {
                let o = self.point(n, "center")?;
                let a = self.f(n, "angle", "0", None)?;
                Box::new(move |p| rotate_about(p, o, a))
            }
            "flippingByLine" => {
                let a = self.point(n, "p1Line")?;
                let b = self.point(n, "p2Line")?;
                if a.distance(b) < 1e-9 {
                    return Err(construction(n, "the line has no length"));
                }
                Box::new(move |p| mirror(p, a, b))
            }
            "flippingByAxis" => {
                let o = self.point(n, "center")?;
                if n.attribute("axisType") == Some("2") {
                    Box::new(move |p| DVec2::new(p.x, 2.0 * o.y - p.y))
                } else {
                    Box::new(move |p| DVec2::new(2.0 * o.x - p.x, p.y))
                }
            }
            "moving" => {
                let shift =
                    dir(self.f(n, "angle", "0", None)?) * self.len(n, "length", "0", None)?;
                let rot = self.f(n, "rotationAngle", "0", None)?;
                let center = match Self::id(n, "center") {
                    Some(_) => self.point(n, "center")?,
                    None => {
                        let all: Vec<DVec2> = objs
                            .iter()
                            .flat_map(|o| match o {
                                Obj::Point { pos, .. } => vec![*pos],
                                Obj::Curve { curve, .. } => curve.polyline(),
                            })
                            .collect();
                        let bb = r3d_geom::polygon::bounding_box(&all)
                            .ok_or_else(|| construction(n, "nothing to move"))?;
                        (bb.min + bb.max) / 2.0 + shift
                    }
                };
                Box::new(move |p| rotate_about(p + shift, center, rot))
            }
            _ => return self.unsupported(n),
        };
        let suffix = n.attribute("suffix").unwrap_or("");
        for (src, (obj, dest)) in sources.iter().zip(objs.iter().zip(&dests)) {
            let Some(did) = Self::id(*dest, "idObject") else {
                continue;
            };
            match obj {
                Obj::Point { pos, name } => {
                    let new_name = src
                        .attribute("name")
                        .filter(|s| !s.is_empty())
                        .map_or_else(|| format!("{name}{suffix}"), str::to_owned);
                    self.add_point(did, &new_name, map(*pos));
                }
                Obj::Curve { curve, kind } => {
                    let kind = match kind {
                        Kind::Arc { center, radius } => Kind::Arc {
                            center: map(*center),
                            radius: *radius,
                        },
                        k => k.clone(),
                    };
                    self.add_curve(Some(did), curve.mapped(&map), kind);
                }
            }
        }
        Ok(())
    }

    /// Variables of the pattern (`#name`), in file order.
    pub fn variable(&mut self, name: &str, src: &str) -> Result<(), ValError> {
        let v = self.formula(src, None)?;
        self.vars.insert(name.to_owned(), v);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circles_and_sides() {
        let pts = circle_circle(DVec2::ZERO, 5.0, DVec2::new(8.0, 0.0), 5.0);
        assert_eq!(pts.len(), 2);
        let right = by_side(&pts, DVec2::ZERO, DVec2::new(8.0, 0.0), "1").unwrap();
        assert!(right.y < 0.0);
        let left = by_side(&pts, DVec2::ZERO, DVec2::new(8.0, 0.0), "2").unwrap();
        assert!(left.y > 0.0);
    }

    #[test]
    fn mirror_across_a_line() {
        let p = mirror(DVec2::new(1.0, 2.0), DVec2::ZERO, DVec2::new(1.0, 0.0));
        assert!(p.distance(DVec2::new(1.0, -2.0)) < 1e-12);
    }
}
