//! The geometric constraint solver over a project: constraints and
//! dimensions become equations on the points (and, where a smooth joint
//! needs them, on the curve handles), solved by `r3d_geom::solve`
//! (damped minimum-norm Gauss–Newton). See `docs/specs/19-constraints.md`.
//!
//! Variables are the positions of the points the constraints name, and the
//! handles of curves meeting at a smooth (`tangent`) point, kept as offsets
//! from their end points so that a moving point carries its handles along,
//! as `point.move` does. Fixed points are constants. Constraints that share
//! no point are solved separately (connected components), so a change on
//! one piece never moves another.
//!
//! The solver moves the geometry as little as possible. The points and
//! handles the last edit moved ("touched") are held first; only if the
//! constraints cannot hold with them are they released, and then they are
//! still stiffer than the rest.

use std::collections::{BTreeMap, BTreeSet};

use glam::DVec2;
use r3d_geom::CubicBez;
use r3d_geom::solve::{Options, System, solve};

use crate::id::{ConstraintId, EdgeId, PointId};
use crate::model::{Along, Constraint, EdgeCurve, Project};

/// Length that turns angular residuals (radians) into millimetres, so that
/// both weigh alike in the solver.
const ANGLE_SCALE_MM: f64 = 100.0;
/// A constraint holds when its residual is within this (mm).
pub const TOLERANCE_MM: f64 = 1e-6;
/// The solver aims a little tighter, so a solved drawing passes the check.
const SOLVE_TOLERANCE: f64 = 1e-7;
/// Stiffness of the touched variables when they have to give way.
const TOUCHED_WEIGHT: f64 = 1e4;

/// What the last edit moved: these are held where they are when possible.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Touched {
    pub points: BTreeSet<PointId>,
    pub edges: BTreeSet<EdgeId>,
}

/// A change the solver makes.
#[derive(Clone, Debug, PartialEq)]
pub enum Update {
    Point(PointId, DVec2),
    Curve(EdgeId, EdgeCurve),
}

/// Constraints that cannot all hold.
#[derive(Clone, Debug, PartialEq)]
pub struct Unsolvable {
    /// The constraints left unsatisfied at the closest point found.
    pub constraints: Vec<ConstraintId>,
}

/// How constrained a group of constraints sharing points is.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupStatus {
    pub constraints: Vec<ConstraintId>,
    /// Ways the constrained geometry can still move (including moving or
    /// turning it as a whole, unless points are fixed).
    pub freedom: usize,
    /// Constraints that repeat what others already say (over-constrained).
    pub redundant: Vec<ConstraintId>,
    /// All of them hold.
    pub satisfied: bool,
}

/// One equation of the system.
#[derive(Clone, Debug)]
enum Eq {
    CoincidentX(PointId, PointId),
    CoincidentY(PointId, PointId),
    Horizontal(EdgeId),
    Vertical(EdgeId),
    Parallel(EdgeId, EdgeId),
    Perpendicular(EdgeId, EdgeId),
    /// The edge arriving at a smooth point and the edge leaving it.
    Tangent(EdgeId, EdgeId),
    Equal(EdgeId, EdgeId),
    OnEdge(PointId, EdgeId),
    /// `sign` orients a horizontal or vertical distance as it was found.
    Distance(PointId, PointId, f64, Along, f64),
    Length(EdgeId, f64),
    /// Radians; the shared point, if any.
    Angle(EdgeId, EdgeId, f64, Option<PointId>),
    /// A symmetric piece's half ends on its axis (origin, unit direction).
    OnAxis(PointId, DVec2, DVec2),
}

/// The geometry the equations read, with the variables plugged in.
struct Sketch<'a> {
    project: &'a Project,
    /// Index of each variable point's x (y follows).
    points: BTreeMap<PointId, usize>,
    /// Index of each variable curve's handle offsets (4 numbers).
    handles: BTreeMap<EdgeId, usize>,
    eqs: Vec<(Option<ConstraintId>, Eq, Vec<usize>)>,
    n: usize,
}

/// A cubic's handles as offsets from its ends.
fn offsets(project: &Project, e: EdgeId) -> Option<(DVec2, DVec2)> {
    let edge = project.edge(e)?;
    match edge.curve {
        EdgeCurve::Cubic { c1, c2 } => Some((
            c1 - project.point(edge.start)?.pos,
            c2 - project.point(edge.end)?.pos,
        )),
        EdgeCurve::Line => None,
    }
}

/// An edge's shape at `x`: its ends, and its handles if curved.
#[derive(Clone, Copy)]
struct Shape {
    a: DVec2,
    b: DVec2,
    handles: Option<(DVec2, DVec2)>,
}

impl Shape {
    fn bez(&self) -> CubicBez {
        match self.handles {
            Some((c1, c2)) => CubicBez::new(self.a, c1, c2, self.b),
            None => CubicBez::line(self.a, self.b),
        }
    }

    /// Unit direction leaving the start, and arriving at the end.
    fn end_dirs(&self) -> (DVec2, DVec2) {
        let chord = self.b - self.a;
        let (out, arrive) = match self.handles {
            Some((c1, c2)) => {
                let out = [c1 - self.a, c2 - self.a, chord]
                    .into_iter()
                    .find(|d| d.length_squared() > 1e-18)
                    .unwrap_or(chord);
                let arrive = [self.b - c2, self.b - c1, chord]
                    .into_iter()
                    .find(|d| d.length_squared() > 1e-18)
                    .unwrap_or(chord);
                (out, arrive)
            }
            None => (chord, chord),
        };
        (unit(out), unit(arrive))
    }

    fn length(&self) -> f64 {
        match self.handles {
            Some(_) => gauss_length(&self.bez()),
            None => self.a.distance(self.b),
        }
    }
}

fn unit(v: DVec2) -> DVec2 {
    v.try_normalize().unwrap_or(DVec2::X)
}

/// Arc length of a cubic by 3 × 8-point Gauss–Legendre quadrature: a
/// smooth function of the control points (an adaptive length is not, and
/// its noise would spoil the solver's derivatives).
fn gauss_length(c: &CubicBez) -> f64 {
    const NODES: [(f64, f64); 8] = [
        (-0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
        (-0.796_666_477_413_626_7, 0.222_381_034_453_374_47),
        (-0.525_532_409_916_329, 0.313_706_645_877_887_3),
        (-0.183_434_642_495_649_8, 0.362_683_783_378_362),
        (0.183_434_642_495_649_8, 0.362_683_783_378_362),
        (0.525_532_409_916_329, 0.313_706_645_877_887_3),
        (0.796_666_477_413_626_7, 0.222_381_034_453_374_47),
        (0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
    ];
    let speed = |t: f64| {
        let mt = 1.0 - t;
        ((c.p1 - c.p0) * (3.0 * mt * mt)
            + (c.p2 - c.p1) * (6.0 * mt * t)
            + (c.p3 - c.p2) * (3.0 * t * t))
            .length()
    };
    let parts = 3;
    (0..parts)
        .map(|k| {
            let (lo, hi) = (k as f64 / parts as f64, (k + 1) as f64 / parts as f64);
            let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
            NODES
                .iter()
                .map(|(x, w)| w * speed(mid + half * x))
                .sum::<f64>()
                * half
        })
        .sum()
}

/// Signed distance from `p` to a curve (positive on its left).
fn signed_distance(c: &CubicBez, p: DVec2) -> f64 {
    let mut best = (f64::INFINITY, 0.0);
    let samples = 32;
    for k in 0..=samples {
        let t = k as f64 / samples as f64;
        let d = c.eval(t).distance_squared(p);
        if d < best.0 {
            best = (d, t);
        }
    }
    let mut t = best.1;
    // Newton on (B(t) − p)·B'(t) = 0, kept inside the curve.
    for _ in 0..8 {
        let mt = 1.0 - t;
        let d1 = (c.p1 - c.p0) * (3.0 * mt * mt)
            + (c.p2 - c.p1) * (6.0 * mt * t)
            + (c.p3 - c.p2) * (3.0 * t * t);
        let d2 = (c.p2 - c.p1 * 2.0 + c.p0) * (6.0 * mt) + (c.p3 - c.p2 * 2.0 + c.p1) * (6.0 * t);
        let r = c.eval(t) - p;
        let den = d1.dot(d1) + r.dot(d2);
        if den.abs() < 1e-12 {
            break;
        }
        t = (t - r.dot(d1) / den).clamp(0.0, 1.0);
    }
    let tangent = c.tangent(t);
    tangent.perp_dot(p - c.eval(t))
}

impl<'a> Sketch<'a> {
    fn pos(&self, p: PointId, x: &[f64]) -> DVec2 {
        match self.points.get(&p) {
            Some(i) => DVec2::new(
                x.get(*i).copied().unwrap_or_default(),
                x.get(i + 1).copied().unwrap_or_default(),
            ),
            None => self.project.point(p).map(|p| p.pos).unwrap_or_default(),
        }
    }

    fn shape(&self, e: EdgeId, x: &[f64]) -> Shape {
        let Some(edge) = self.project.edge(e) else {
            return Shape {
                a: DVec2::ZERO,
                b: DVec2::ZERO,
                handles: None,
            };
        };
        let (a, b) = (self.pos(edge.start, x), self.pos(edge.end, x));
        let handles = match self.handles.get(&e) {
            Some(i) => {
                let get = |k: usize| x.get(i + k).copied().unwrap_or_default();
                Some((
                    a + DVec2::new(get(0), get(1)),
                    b + DVec2::new(get(2), get(3)),
                ))
            }
            None => offsets(self.project, e).map(|(o1, o2)| (a + o1, b + o2)),
        };
        Shape { a, b, handles }
    }

    /// Unit direction of edge `e` leaving point `p` (one of its ends).
    fn leaving(&self, e: EdgeId, p: PointId, x: &[f64]) -> DVec2 {
        let (out, arrive) = self.shape(e, x).end_dirs();
        match self.project.edge(e) {
            Some(edge) if edge.start == p => out,
            _ => -arrive,
        }
    }

    fn eval(&self, eq: &Eq, x: &[f64]) -> f64 {
        match eq {
            Eq::CoincidentX(a, b) => self.pos(*b, x).x - self.pos(*a, x).x,
            Eq::CoincidentY(a, b) => self.pos(*b, x).y - self.pos(*a, x).y,
            Eq::Horizontal(e) => {
                let s = self.shape(*e, x);
                s.b.y - s.a.y
            }
            Eq::Vertical(e) => {
                let s = self.shape(*e, x);
                s.b.x - s.a.x
            }
            Eq::Parallel(a, b) | Eq::Perpendicular(a, b) => {
                let (sa, sb) = (self.shape(*a, x), self.shape(*b, x));
                let (ua, ub) = (unit(sa.b - sa.a), unit(sb.b - sb.a));
                if matches!(eq, Eq::Parallel(..)) {
                    ANGLE_SCALE_MM * ua.perp_dot(ub)
                } else {
                    ANGLE_SCALE_MM * ua.dot(ub)
                }
            }
            Eq::Tangent(arriving, leaving) => {
                let u_in = self.shape(*arriving, x).end_dirs().1;
                let u_out = self.shape(*leaving, x).end_dirs().0;
                ANGLE_SCALE_MM * u_in.perp_dot(u_out).atan2(u_in.dot(u_out))
            }
            Eq::Equal(a, b) => self.shape(*a, x).length() - self.shape(*b, x).length(),
            Eq::OnEdge(p, e) => {
                let s = self.shape(*e, x);
                let p = self.pos(*p, x);
                match s.handles {
                    Some(_) => signed_distance(&s.bez(), p),
                    None => unit(s.b - s.a).perp_dot(p - s.a),
                }
            }
            Eq::Distance(a, b, mm, along, sign) => {
                let d = self.pos(*b, x) - self.pos(*a, x);
                match along {
                    Along::Direct => d.length() - mm,
                    Along::Horizontal => d.x * sign - mm,
                    Along::Vertical => d.y * sign - mm,
                }
            }
            Eq::Length(e, mm) => self.shape(*e, x).length() - mm,
            Eq::Angle(a, b, rad, shared) => {
                let (ua, ub) = match shared {
                    Some(p) => (self.leaving(*a, *p, x), self.leaving(*b, *p, x)),
                    None => {
                        let (sa, sb) = (self.shape(*a, x), self.shape(*b, x));
                        (unit(sa.b - sa.a), unit(sb.b - sb.a))
                    }
                };
                ANGLE_SCALE_MM * (ua.perp_dot(ub).abs().atan2(ua.dot(ub)) - rad)
            }
            Eq::OnAxis(p, origin, dir) => dir.perp_dot(self.pos(*p, x) - *origin),
        }
    }

    fn x0(&self) -> Vec<f64> {
        let mut x = vec![0.0; self.n];
        for (p, i) in &self.points {
            if let Some(pt) = self.project.point(*p) {
                if let Some(v) = x.get_mut(*i) {
                    *v = pt.pos.x;
                }
                if let Some(v) = x.get_mut(i + 1) {
                    *v = pt.pos.y;
                }
            }
        }
        for (e, i) in &self.handles {
            if let Some((o1, o2)) = offsets(self.project, *e) {
                for (k, v) in [o1.x, o1.y, o2.x, o2.y].into_iter().enumerate() {
                    if let Some(slot) = x.get_mut(i + k) {
                        *slot = v;
                    }
                }
            }
        }
        x
    }

    /// The changes that put the solution `x` into the project.
    fn updates(&self, x: &[f64]) -> Vec<Update> {
        let mut out = Vec::new();
        let mut moved = BTreeSet::new();
        for p in self.points.keys() {
            let to = self.pos(*p, x);
            if self.project.point(*p).is_some_and(|pt| pt.pos != to) {
                out.push(Update::Point(*p, to));
                moved.insert(*p);
            }
        }
        for (id, edge) in self.project.edges() {
            let EdgeCurve::Cubic { c1, c2 } = edge.curve else {
                continue;
            };
            if !(moved.contains(&edge.start)
                || moved.contains(&edge.end)
                || self.handles.contains_key(id))
            {
                continue;
            }
            if let Some((h1, h2)) = self.shape(*id, x).handles
                && (h1 != c1 || h2 != c2)
            {
                out.push(Update::Curve(*id, EdgeCurve::Cubic { c1: h1, c2: h2 }));
            }
        }
        out
    }
}

impl System for Sketch<'_> {
    fn vars(&self) -> usize {
        self.n
    }
    fn equations(&self) -> usize {
        self.eqs.len()
    }
    fn residual(&self, i: usize, x: &[f64]) -> f64 {
        self.eqs.get(i).map_or(0.0, |(_, eq, _)| self.eval(eq, x))
    }
    fn depends(&self, i: usize) -> &[usize] {
        self.eqs.get(i).map_or(&[], |(_, _, d)| d)
    }
}

/// The contour edges arriving at and leaving `p`.
fn joint(project: &Project, p: PointId) -> Option<(EdgeId, EdgeId)> {
    let panel = project.panel(project.point(p)?.panel)?;
    let arriving = panel
        .contour
        .iter()
        .find(|e| project.edge(**e).is_some_and(|e| e.end == p))?;
    let leaving = panel
        .contour
        .iter()
        .find(|e| project.edge(**e).is_some_and(|e| e.start == p))?;
    Some((*arriving, *leaving))
}

/// The point two edges share, if any.
fn shared_point(project: &Project, a: EdgeId, b: EdgeId) -> Option<PointId> {
    let (ea, eb) = (project.edge(a)?, project.edge(b)?);
    [ea.start, ea.end]
        .into_iter()
        .find(|p| *p == eb.start || *p == eb.end)
}

/// The points a constraint moves (an edge's: its two ends) and the curves
/// whose handles it turns.
fn reach(project: &Project, c: &Constraint) -> (Vec<PointId>, Vec<EdgeId>) {
    let mut points = c.points();
    for e in c.edges() {
        if let Some(edge) = project.edge(e) {
            points.extend([edge.start, edge.end]);
        }
    }
    let mut curves = Vec::new();
    if let Constraint::Tangent { point } = c
        && let Some((a, b)) = joint(project, *point)
    {
        for e in [a, b] {
            if let Some(edge) = project.edge(e) {
                points.extend([edge.start, edge.end]);
                if matches!(edge.curve, EdgeCurve::Cubic { .. }) {
                    curves.push(e);
                }
            }
        }
    }
    (points, curves)
}

/// The equations of a constraint, measured as the geometry is now for the
/// orientation of horizontal and vertical distances.
fn equations(project: &Project, c: &Constraint) -> Vec<Eq> {
    match c {
        Constraint::Fixed { .. } => Vec::new(),
        Constraint::Coincident { a, b } => vec![Eq::CoincidentX(*a, *b), Eq::CoincidentY(*a, *b)],
        Constraint::Horizontal { edge } => vec![Eq::Horizontal(*edge)],
        Constraint::Vertical { edge } => vec![Eq::Vertical(*edge)],
        Constraint::Parallel { a, b } => vec![Eq::Parallel(*a, *b)],
        Constraint::Perpendicular { a, b } => vec![Eq::Perpendicular(*a, *b)],
        Constraint::Tangent { point } => joint(project, *point)
            .map(|(a, b)| Eq::Tangent(a, b))
            .into_iter()
            .collect(),
        Constraint::EqualLength { a, b } => vec![Eq::Equal(*a, *b)],
        Constraint::OnEdge { point, edge } => vec![Eq::OnEdge(*point, *edge)],
        Constraint::Distance { a, b, mm, along } => {
            let d = match (project.point(*a), project.point(*b)) {
                (Some(pa), Some(pb)) => pb.pos - pa.pos,
                _ => DVec2::ZERO,
            };
            let sign = match along {
                Along::Horizontal if d.x < 0.0 => -1.0,
                Along::Vertical if d.y < 0.0 => -1.0,
                _ => 1.0,
            };
            vec![Eq::Distance(*a, *b, *mm, *along, sign)]
        }
        Constraint::Length { edge, mm } => vec![Eq::Length(*edge, *mm)],
        Constraint::Angle { a, b, deg } => vec![Eq::Angle(
            *a,
            *b,
            deg.to_radians(),
            shared_point(project, *a, *b),
        )],
    }
}

/// The two ends of a symmetric piece's drawn half, which stay on its axis,
/// with the axis (origin, unit direction).
fn axis_ends(project: &Project) -> BTreeMap<PointId, (DVec2, DVec2)> {
    let mut out = BTreeMap::new();
    for panel in project.panels().values() {
        let Some(sym) = &panel.symmetry else {
            continue;
        };
        let dir = unit(sym.dir);
        out.insert(sym.start, (sym.origin, dir));
        out.insert(sym.end, (sym.origin, dir));
    }
    out
}

/// The project's constraints in groups that share points, each with the
/// points and curves it moves.
fn groups(project: &Project) -> Vec<(Vec<ConstraintId>, BTreeSet<PointId>, BTreeSet<EdgeId>)> {
    let mut groups: Vec<(Vec<ConstraintId>, BTreeSet<PointId>, BTreeSet<EdgeId>)> = Vec::new();
    for (id, c) in project.constraints() {
        let (points, curves) = reach(project, c);
        let mut merged = (
            vec![*id],
            BTreeSet::from_iter(points),
            BTreeSet::from_iter(curves),
        );
        let mut k = 0;
        while k < groups.len() {
            let touches = groups.get(k).is_some_and(|g| !g.1.is_disjoint(&merged.1));
            if touches {
                let g = groups.swap_remove(k);
                merged.0.extend(g.0);
                merged.1.extend(g.1);
                merged.2.extend(g.2);
            } else {
                k += 1;
            }
        }
        merged.0.sort();
        groups.push(merged);
    }
    groups.sort_by_key(|g| g.0.first().copied());
    groups
}

/// The system of one group, with `held` points as constants too.
fn sketch<'a>(
    project: &'a Project,
    ids: &[ConstraintId],
    points: &BTreeSet<PointId>,
    curves: &BTreeSet<EdgeId>,
    held: &BTreeSet<PointId>,
    held_curves: &BTreeSet<EdgeId>,
) -> Sketch<'a> {
    let fixed: BTreeSet<PointId> = ids
        .iter()
        .filter_map(|id| match project.constraint(*id) {
            Some(Constraint::Fixed { point }) => Some(*point),
            _ => None,
        })
        .collect();
    let mut n = 0;
    let mut vars = BTreeMap::new();
    for p in points {
        if !fixed.contains(p) && !held.contains(p) {
            vars.insert(*p, n);
            n += 2;
        }
    }
    let mut handles = BTreeMap::new();
    for e in curves {
        if !held_curves.contains(e) {
            handles.insert(*e, n);
            n += 4;
        }
    }
    let mut s = Sketch {
        project,
        points: vars,
        handles,
        eqs: Vec::new(),
        n,
    };
    let depends = |s: &Sketch<'_>, pts: &[PointId], es: &[EdgeId]| {
        let mut d = Vec::new();
        for p in pts {
            if let Some(i) = s.points.get(p) {
                d.extend([*i, i + 1]);
            }
        }
        for e in es {
            if let Some(i) = s.handles.get(e) {
                d.extend(*i..i + 4);
            }
        }
        d
    };
    for id in ids {
        let Some(c) = project.constraint(*id) else {
            continue;
        };
        let (pts, es) = reach(project, c);
        for eq in equations(project, c) {
            let d = depends(&s, &pts, &es);
            s.eqs.push((Some(*id), eq, d));
        }
    }
    for (p, (origin, dir)) in axis_ends(project) {
        if s.points.contains_key(&p) {
            let d = depends(&s, &[p], &[]);
            s.eqs.push((None, Eq::OnAxis(p, origin, dir), d));
        }
    }
    s
}

fn opts() -> Options {
    Options {
        tolerance: SOLVE_TOLERANCE,
        max_iterations: 100,
    }
}

/// Whether a constraint holds as the geometry is now.
pub fn holds(project: &Project, id: ConstraintId) -> bool {
    let Some(c) = project.constraint(id) else {
        return true;
    };
    let (points, curves) = reach(project, c);
    let s = sketch(
        project,
        &[id],
        &points.into_iter().collect(),
        &curves.into_iter().collect(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    let x = s.x0();
    (0..s.equations()).all(|i| s.residual(i, &x).abs() <= TOLERANCE_MM)
}

/// Move the geometry so that every constraint holds, moving `touched` as
/// little as possible. Returns the changes to make (none when everything
/// already holds).
pub fn solve_project(project: &Project, touched: &Touched) -> Result<Vec<Update>, Unsolvable> {
    let mut out = Vec::new();
    for (ids, points, curves) in groups(project) {
        let free = sketch(
            project,
            &ids,
            &points,
            &curves,
            &BTreeSet::new(),
            &BTreeSet::new(),
        );
        let x0 = free.x0();
        if (0..free.equations()).all(|i| free.residual(i, &x0).abs() <= TOLERANCE_MM) {
            continue;
        }
        // First hold what the edit moved.
        let held: BTreeSet<PointId> = points.intersection(&touched.points).copied().collect();
        let held_curves: BTreeSet<EdgeId> = curves.intersection(&touched.edges).copied().collect();
        if !held.is_empty() || !held_curves.is_empty() {
            let s = sketch(project, &ids, &points, &curves, &held, &held_curves);
            let sol = solve(&s, &s.x0(), &[], &opts());
            if sol.converged {
                out.extend(s.updates(&sol.x));
                continue;
            }
        }
        // Then let it give way, stiffly.
        let mut weights = vec![1.0; free.n];
        for (p, i) in &free.points {
            if touched.points.contains(p) {
                for k in [*i, i + 1] {
                    if let Some(w) = weights.get_mut(k) {
                        *w = TOUCHED_WEIGHT;
                    }
                }
            }
        }
        for (e, i) in &free.handles {
            if touched.edges.contains(e) {
                for k in *i..i + 4 {
                    if let Some(w) = weights.get_mut(k) {
                        *w = TOUCHED_WEIGHT;
                    }
                }
            }
        }
        let sol = solve(&free, &x0, &weights, &opts());
        if !sol.converged {
            let mut constraints: Vec<ConstraintId> = (0..free.equations())
                .filter(|i| free.residual(*i, &sol.x).abs() > TOLERANCE_MM)
                .filter_map(|i| free.eqs.get(i).and_then(|(id, _, _)| *id))
                .collect();
            constraints.dedup();
            return Err(Unsolvable { constraints });
        }
        out.extend(free.updates(&sol.x));
    }
    Ok(out)
}

/// How constrained each group of constraints is, as the geometry is now.
pub fn status(project: &Project) -> Vec<GroupStatus> {
    groups(project)
        .into_iter()
        .map(|(ids, points, curves)| {
            let s = sketch(
                project,
                &ids,
                &points,
                &curves,
                &BTreeSet::new(),
                &BTreeSet::new(),
            );
            let x = s.x0();
            let sol = solve(
                &s,
                &x,
                &[],
                &Options {
                    tolerance: TOLERANCE_MM,
                    max_iterations: 0,
                },
            );
            let mut redundant: Vec<ConstraintId> = sol
                .redundant
                .iter()
                .filter_map(|i| s.eqs.get(*i).and_then(|(id, _, _)| *id))
                .collect();
            redundant.dedup();
            GroupStatus {
                constraints: ids,
                freedom: sol.freedom(),
                redundant,
                satisfied: sol.residual <= TOLERANCE_MM,
            }
        })
        .collect()
}

/// A dimension's current value as the geometry is now (mm or degrees);
/// `None` for other constraints or missing geometry.
pub fn measure(project: &Project, c: &Constraint) -> Option<f64> {
    let s = Sketch {
        project,
        points: BTreeMap::new(),
        handles: BTreeMap::new(),
        eqs: Vec::new(),
        n: 0,
    };
    let zero = c.with_value(0.0)?;
    let eq = equations(project, &zero).into_iter().next()?;
    let v = s.eval(&eq, &[]);
    Some(match c {
        Constraint::Angle { .. } => (v / ANGLE_SCALE_MM).to_degrees(),
        _ => v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::Transaction;
    use crate::model::{Edge, Panel, Point};

    /// A quad panel; returns its points and edges (bottom, right, top,
    /// left).
    fn quad(p: &mut Project, corners: [[f64; 2]; 4]) -> ([PointId; 4], [EdgeId; 4]) {
        let mut tx = Transaction::begin(p);
        let panel = tx.new_panel_id();
        let pts = corners.map(|c| {
            let id = tx.new_point_id();
            tx.put_point(id, Point::new(panel, DVec2::from(c)));
            id
        });
        let edges = [0, 1, 2, 3].map(|i| {
            let id = tx.new_edge_id();
            tx.put_edge(
                id,
                Edge::new(panel, pts[i], pts[(i + 1) % 4], EdgeCurve::Line),
            );
            id
        });
        let garment = tx.project().default_garment().unwrap();
        tx.put_panel(
            panel,
            Panel {
                name: "Q".into(),
                garment,
                contour: edges.to_vec(),
                grain_angle_deg: 90.0,
                seam_allowance_mm: 10.0,
                mirror_of: None,
                placement: None,
                symmetry: None,
            },
        );
        tx.commit();
        (pts, edges)
    }

    fn add(p: &mut Project, c: Constraint) -> ConstraintId {
        let mut tx = Transaction::begin(p);
        let id = tx.new_constraint_id();
        tx.put_constraint(id, c);
        tx.commit();
        id
    }

    fn apply(p: &mut Project, touched: &Touched) {
        let updates = solve_project(p, touched).unwrap();
        let mut tx = Transaction::begin(p);
        for u in updates {
            match u {
                Update::Point(id, to) => {
                    let pt = tx.project().point(id).unwrap().clone();
                    tx.put_point(id, Point { pos: to, ..pt });
                }
                Update::Curve(id, curve) => {
                    let e = tx.project().edge(id).unwrap().clone();
                    tx.put_edge(id, Edge { curve, ..e });
                }
            }
        }
        tx.commit();
    }

    fn pos(p: &Project, id: PointId) -> DVec2 {
        p.point(id).unwrap().pos
    }

    #[test]
    fn a_tilted_edge_becomes_horizontal_and_a_length_holds() {
        let mut p = Project::new("t");
        let (pts, edges) = quad(
            &mut p,
            [[0.0, 0.0], [100.0, 6.0], [100.0, 80.0], [0.0, 80.0]],
        );
        add(&mut p, Constraint::Fixed { point: pts[0] });
        add(&mut p, Constraint::Horizontal { edge: edges[0] });
        let len = add(
            &mut p,
            Constraint::Length {
                edge: edges[0],
                mm: 120.0,
            },
        );
        apply(&mut p, &Touched::default());
        assert_eq!(pos(&p, pts[0]), DVec2::ZERO);
        assert!((pos(&p, pts[1]) - DVec2::new(120.0, 0.0)).length() < 1e-5);
        assert!(holds(&p, len));
        // Points no constraint names stay where they were.
        assert_eq!(pos(&p, pts[2]), DVec2::new(100.0, 80.0));
        let st = status(&p);
        assert_eq!(st.len(), 1);
        assert!(st[0].satisfied && st[0].redundant.is_empty());
        // Two points, one fixed, two equations: nothing left free.
        assert_eq!(st[0].freedom, 0);
    }

    #[test]
    fn a_touched_point_is_held_and_the_rest_follows() {
        let mut p = Project::new("t");
        let (pts, edges) = quad(
            &mut p,
            [[0.0, 0.0], [100.0, 0.0], [100.0, 80.0], [0.0, 80.0]],
        );
        add(&mut p, Constraint::Horizontal { edge: edges[0] });
        // Drag the bottom right corner up by 10: the bottom left follows.
        {
            let mut tx = Transaction::begin(&mut p);
            let pt = tx.project().point(pts[1]).unwrap().clone();
            tx.put_point(
                pts[1],
                Point {
                    pos: DVec2::new(100.0, 10.0),
                    ..pt
                },
            );
            tx.commit();
        }
        let touched = Touched {
            points: BTreeSet::from([pts[1]]),
            edges: BTreeSet::new(),
        };
        apply(&mut p, &touched);
        assert_eq!(pos(&p, pts[1]), DVec2::new(100.0, 10.0));
        assert!((pos(&p, pts[0]) - DVec2::new(0.0, 10.0)).length() < 1e-6);
    }

    #[test]
    fn conflicts_are_reported_with_their_constraints() {
        let mut p = Project::new("t");
        let (pts, edges) = quad(
            &mut p,
            [[0.0, 0.0], [100.0, 0.0], [100.0, 80.0], [0.0, 80.0]],
        );
        add(&mut p, Constraint::Fixed { point: pts[0] });
        add(&mut p, Constraint::Fixed { point: pts[1] });
        let len = add(
            &mut p,
            Constraint::Length {
                edge: edges[0],
                mm: 50.0,
            },
        );
        let err = solve_project(&p, &Touched::default()).unwrap_err();
        assert_eq!(err.constraints, vec![len]);
    }

    #[test]
    fn redundant_constraints_are_found() {
        let mut p = Project::new("t");
        let (_, edges) = quad(
            &mut p,
            [[0.0, 0.0], [100.0, 0.0], [100.0, 80.0], [0.0, 80.0]],
        );
        add(&mut p, Constraint::Horizontal { edge: edges[0] });
        add(&mut p, Constraint::Horizontal { edge: edges[2] });
        let par = add(
            &mut p,
            Constraint::Parallel {
                a: edges[0],
                b: edges[2],
            },
        );
        let st = status(&p);
        assert_eq!(st.len(), 1);
        assert_eq!(st[0].redundant, vec![par]);
    }

    #[test]
    fn a_smooth_joint_turns_the_handles() {
        let mut p = Project::new("t");
        let (pts, edges) = quad(
            &mut p,
            [[0.0, 0.0], [100.0, 0.0], [100.0, 80.0], [0.0, 80.0]],
        );
        // Curve the bottom and right edges, meeting at a corner.
        {
            let mut tx = Transaction::begin(&mut p);
            for (e, c1, c2) in [
                (edges[0], [30.0, -10.0], [80.0, -10.0]),
                (edges[1], [110.0, 20.0], [110.0, 60.0]),
            ] {
                let edge = tx.project().edge(e).unwrap().clone();
                tx.put_edge(
                    e,
                    Edge {
                        curve: EdgeCurve::Cubic {
                            c1: DVec2::from(c1),
                            c2: DVec2::from(c2),
                        },
                        ..edge
                    },
                );
            }
            tx.commit();
        }
        let t = add(&mut p, Constraint::Tangent { point: pts[1] });
        apply(&mut p, &Touched::default());
        assert!(holds(&p, t));
        // The corner did not have to move: turning the handles was enough
        // and moved less.
        assert!((pos(&p, pts[1]) - DVec2::new(100.0, 0.0)).length() < 1.0);
    }

    #[test]
    fn measure_reads_dimensions() {
        let mut p = Project::new("t");
        let (pts, edges) = quad(&mut p, [[0.0, 0.0], [30.0, 0.0], [30.0, 40.0], [0.0, 40.0]]);
        let d = Constraint::Distance {
            a: pts[0],
            b: pts[2],
            mm: 1.0,
            along: Along::Direct,
        };
        assert!((measure(&p, &d).unwrap() - 50.0).abs() < 1e-9);
        let a = Constraint::Angle {
            a: edges[0],
            b: edges[1],
            deg: 1.0,
        };
        assert!((measure(&p, &a).unwrap() - 90.0).abs() < 1e-9);
        let h = Constraint::Distance {
            a: pts[1],
            b: pts[0],
            mm: 1.0,
            along: Along::Horizontal,
        };
        assert!((measure(&p, &h).unwrap() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn gauss_length_matches_the_adaptive_one() {
        let c = CubicBez::new(
            DVec2::ZERO,
            DVec2::new(30.0, 60.0),
            DVec2::new(90.0, -40.0),
            DVec2::new(120.0, 10.0),
        );
        assert!((gauss_length(&c) - c.arc_length(1e-7)).abs() < 1e-4);
    }
}
