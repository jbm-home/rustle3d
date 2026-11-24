//! Meshing a panel for the simulation: a quality triangulation of the
//! panel's interior that keeps the given boundary points exactly (so the
//! vertices along a seam match on both sides).
//!
//! Constrained Delaunay triangulation with Ruppert/Chew refinement
//! (`spade`).

use glam::DVec2;
use spade::handles::FixedVertexHandle;
use spade::{
    AngleLimit, ConstrainedDelaunayTriangulation, Point2, RefinementParameters, Triangulation,
};

/// A triangulated panel.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelMesh {
    /// Vertex positions (same units as the input, mm for pattern panels).
    pub vertices: Vec<DVec2>,
    /// Counter-clockwise triangles.
    pub triangles: Vec<[u32; 3]>,
    /// For each input boundary point, its vertex index.
    pub boundary: Vec<u32>,
    /// For each hole, for each of its points, its vertex index.
    pub holes: Vec<Vec<u32>>,
    /// For each line, for each of its points, its vertex index.
    pub lines: Vec<Vec<u32>>,
}

/// Errors meshing a panel.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshError {
    /// Fewer than 3 points, non-finite coordinates or repeated points.
    InvalidBoundary,
    /// A line inside the panel crosses its outline or a hole, or leaves it.
    LineOutside,
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MeshError::InvalidBoundary => {
                "the panel outline cannot be meshed (too few, repeated or invalid points)"
            }
            MeshError::LineOutside => "a line sewn inside the panel crosses or leaves its outline",
        })
    }
}

impl std::error::Error for MeshError {}

/// A band of finer triangles along a polyline (an elastic, so the cloth
/// gathers in small folds): a triangular lattice of points `spacing`
/// apart, `rows` rows on each side of the line, added before the
/// refinement, which grades the triangles back to the panel's spacing
/// around it (docs/physics.md, "Elastic").
#[derive(Debug, Clone, PartialEq)]
pub struct FineZone {
    /// Its points, about `spacing` apart.
    pub line: Vec<DVec2>,
    pub spacing: f64,
    pub rows: usize,
}

/// Points already placed, bucketed so a new point too near one is left
/// out (it would make a sliver).
struct Taken {
    cell: f64,
    grid: std::collections::HashMap<(i64, i64), Vec<DVec2>>,
}

impl Taken {
    fn new(cell: f64) -> Self {
        Self {
            cell: cell.max(1e-6),
            grid: std::collections::HashMap::new(),
        }
    }

    fn key(&self, p: DVec2) -> (i64, i64) {
        (
            (p.x / self.cell).floor() as i64,
            (p.y / self.cell).floor() as i64,
        )
    }

    fn put(&mut self, p: DVec2) {
        let key = self.key(p);
        self.grid.entry(key).or_default().push(p);
    }

    /// Whether a point lies within `r` (at most the cell size) of `p`.
    fn near(&self, p: DVec2, r: f64) -> bool {
        let (x, y) = self.key(p);
        (-1..=1).any(|dx| {
            (-1..=1).any(|dy| {
                self.grid
                    .get(&(x + dx, y + dy))
                    .is_some_and(|pts| pts.iter().any(|q| q.distance(p) < r))
            })
        })
    }
}

/// A line's point this close to a point placed already, in units of the
/// spacing, is that point.
const SNAP: f64 = 0.2;

/// The refinement's points nearer a line than this, in units of the
/// spacing, are left out when the lines are added: a triangle between a
/// line's points (a spacing apart) and one of them keeps angles of about
/// 15° at least.
const CLEAR: f64 = 0.3;

/// Insert `lines` as chains of constrained edges, each line point snapped
/// to a point placed already within [`SNAP`] spacings (the outline's, a
/// hole's, another line's or a crossing); lines that cross or touch
/// meet at a shared vertex. For each line, its points' vertices.
fn constrain_lines(
    cdt: &mut Cdt,
    boundary: &[DVec2],
    holes: &[Vec<DVec2>],
    outer: &[FixedVertexHandle],
    hole_handles: &[Vec<FixedVertexHandle>],
    lines: &[Vec<DVec2>],
    spacing: f64,
) -> Result<Vec<Vec<FixedVertexHandle>>, MeshError> {
    let crossings = crossings(lines);
    let mut placed = Placed::new(spacing * SNAP);
    for (p, h) in boundary
        .iter()
        .chain(holes.iter().flatten())
        .zip(outer.iter().chain(hole_handles.iter().flatten()))
    {
        placed.put(*p, *h);
    }
    let loops: Vec<Vec<DVec2>> = std::iter::once(boundary)
        .chain(holes.iter().map(Vec::as_slice))
        .map(|l| l.iter().chain(l.first()).copied().collect())
        .collect();
    let edges = Segments::new(&loops, spacing);
    let inside = |p: DVec2| {
        crate::polygon::contains(boundary, p)
            && !holes.iter().any(|h| crate::polygon::contains(h, p))
    };
    let mut place = |cdt: &mut Cdt, p: DVec2| -> Result<FixedVertexHandle, MeshError> {
        if let Some(h) = placed.near(p) {
            return Ok(h);
        }
        // Just off the outline or a hole (a line ending on a curve the
        // outline follows by chords): moved inside, clear of it.
        let at = edges
            .nearest(p, spacing * SNAP)
            .and_then(|(foot, normal)| {
                [1.0, -1.0]
                    .map(|side| foot + normal * (side * spacing * SNAP))
                    .into_iter()
                    .find(|q| inside(*q))
            })
            .unwrap_or(p);
        if let Some(h) = placed.near(at) {
            placed.put(p, h);
            return Ok(h);
        }
        let h = cdt
            .insert(Point2::new(at.x, at.y))
            .map_err(|_| MeshError::LineOutside)?;
        placed.put(p, h);
        placed.put(at, h);
        Ok(h)
    };
    let mut line_handles = Vec::with_capacity(lines.len());
    for (k, line) in lines.iter().enumerate() {
        let mut handles = Vec::with_capacity(line.len());
        let mut walk: Vec<FixedVertexHandle> = Vec::new();
        for (i, p) in line.iter().enumerate() {
            let h = place(cdt, *p)?;
            walk.push(h);
            handles.push(h);
            for q in crossings.get(&(k, i)).into_iter().flatten() {
                walk.push(place(cdt, *q)?);
            }
        }
        for w in walk.windows(2) {
            if let [a, b] = w {
                if a == b {
                    continue;
                }
                if !cdt.can_add_constraint(*a, *b) {
                    return Err(MeshError::LineOutside);
                }
                cdt.add_constraint(*a, *b);
            }
        }
        line_handles.push(handles);
    }
    Ok(line_handles)
}

/// The lines' segments, bucketed by the cells they pass near.
struct Segments {
    cell: f64,
    grid: std::collections::HashMap<(i64, i64), Vec<(DVec2, DVec2)>>,
}

impl Segments {
    /// Bucketed in cells of `cell`; [`Segments::near`] finds segments
    /// within `cell` of a point.
    fn new(lines: &[Vec<DVec2>], cell: f64) -> Self {
        let cell = cell.max(1e-9);
        let mut grid: std::collections::HashMap<(i64, i64), Vec<(DVec2, DVec2)>> =
            std::collections::HashMap::new();
        for line in lines {
            for w in line.windows(2) {
                if let [a, b] = w {
                    let (lo, hi) = (a.min(*b) - cell, a.max(*b) + cell);
                    let (x0, y0) = ((lo.x / cell).floor() as i64, (lo.y / cell).floor() as i64);
                    let (x1, y1) = ((hi.x / cell).floor() as i64, (hi.y / cell).floor() as i64);
                    for x in x0..=x1 {
                        for y in y0..=y1 {
                            grid.entry((x, y)).or_default().push((*a, *b));
                        }
                    }
                }
            }
        }
        Self { cell, grid }
    }

    /// The point nearest `p` on a segment within `r` (at most the cell),
    /// and that segment's unit normal.
    fn nearest(&self, p: DVec2, r: f64) -> Option<(DVec2, DVec2)> {
        let key = (
            (p.x / self.cell).floor() as i64,
            (p.y / self.cell).floor() as i64,
        );
        self.grid
            .get(&key)?
            .iter()
            .filter_map(|(a, b)| {
                let ab = *b - *a;
                let t = ((p - *a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                let foot = *a + ab * t;
                Some((foot.distance(p), foot, ab.try_normalize()?.perp()))
            })
            .filter(|(d, _, _)| *d < r)
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, foot, normal)| (foot, normal))
    }

    /// Whether a segment passes within `r` (at most the cell) of `p`.
    fn near(&self, p: DVec2, r: f64) -> bool {
        let key = (
            (p.x / self.cell).floor() as i64,
            (p.y / self.cell).floor() as i64,
        );
        self.grid.get(&key).is_some_and(|segs| {
            segs.iter()
                .any(|(a, b)| crate::fit::distance_to_segment(p, *a, *b) < r)
        })
    }
}

/// The vertices placed so far, bucketed by position.
struct Placed {
    radius: f64,
    grid: std::collections::HashMap<(i64, i64), Vec<(DVec2, FixedVertexHandle)>>,
}

impl Placed {
    fn new(radius: f64) -> Self {
        Self {
            radius: radius.max(1e-9),
            grid: std::collections::HashMap::new(),
        }
    }

    fn key(&self, p: DVec2) -> (i64, i64) {
        (
            (p.x / self.radius).floor() as i64,
            (p.y / self.radius).floor() as i64,
        )
    }

    fn put(&mut self, p: DVec2, h: FixedVertexHandle) {
        let key = self.key(p);
        self.grid.entry(key).or_default().push((p, h));
    }

    /// The nearest vertex within the radius of `p`.
    fn near(&self, p: DVec2) -> Option<FixedVertexHandle> {
        let (x, y) = self.key(p);
        (-1..=1)
            .flat_map(|dx| (-1..=1).map(move |dy| (x + dx, y + dy)))
            .filter_map(|key| self.grid.get(&key))
            .flatten()
            .map(|(q, h)| (q.distance(p), *h))
            .filter(|(d, _)| *d < self.radius)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, h)| h)
    }
}

/// The lattice points of `zones` inside the panel: clear of the outline,
/// the holes, the given points and each other by half their spacing.
fn lattice(
    zones: &[FineZone],
    boundary: &[DVec2],
    holes: &[Vec<DVec2>],
    lines: &[Vec<DVec2>],
) -> Vec<DVec2> {
    let finest = zones
        .iter()
        .map(|z| z.spacing)
        .filter(|s| s.is_finite() && *s > 0.0)
        .fold(f64::INFINITY, f64::min);
    if !finest.is_finite() {
        return Vec::new();
    }
    let mut taken = Taken::new(finest);
    for p in boundary
        .iter()
        .chain(holes.iter().flatten())
        .chain(lines.iter().flatten())
    {
        taken.put(*p);
    }
    let mut out = Vec::new();
    for zone in zones {
        let s = zone.spacing;
        if !(s.is_finite() && s >= finest) {
            continue;
        }
        let row = s * 3f64.sqrt() / 2.0;
        let n = zone.line.len();
        for (i, p) in zone.line.iter().enumerate() {
            let prev = i.checked_sub(1).and_then(|j| zone.line.get(j)).unwrap_or(p);
            let next = zone.line.get(i + 1).unwrap_or(p);
            let Some(along) = (*next - *prev).try_normalize() else {
                continue;
            };
            let normal = along.perp();
            for k in 1..=zone.rows {
                // Odd rows sit between the line's points: a triangular
                // lattice, none past the line's last point.
                let shift = if k % 2 == 1 {
                    if i + 1 == n {
                        continue;
                    }
                    s / 2.0
                } else {
                    0.0
                };
                for side in [1.0, -1.0] {
                    let q = *p + along * shift + normal * (side * row * k as f64);
                    let inside = crate::polygon::contains(boundary, q)
                        && crate::polygon::distance_to_loop(boundary, q) > s / 2.0
                        && holes.iter().all(|h| {
                            !crate::polygon::contains(h, q)
                                && crate::polygon::distance_to_loop(h, q) > s / 2.0
                        });
                    if inside && !taken.near(q, s / 2.0) {
                        taken.put(q);
                        out.push(q);
                    }
                }
            }
        }
    }
    out
}

type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;

/// Insert a closed loop of points as constraint edges.
fn constrain_loop(cdt: &mut Cdt, pts: &[DVec2]) -> Result<Vec<FixedVertexHandle>, MeshError> {
    if pts.len() < 3 || pts.iter().any(|p| !p.is_finite()) {
        return Err(MeshError::InvalidBoundary);
    }
    let handles: Vec<FixedVertexHandle> = pts
        .iter()
        .map(|p| cdt.insert(Point2::new(p.x, p.y)))
        .collect::<Result<_, _>>()
        .map_err(|_| MeshError::InvalidBoundary)?;
    let mut unique = handles.clone();
    unique.sort_by_key(|h| h.index());
    unique.dedup();
    if unique.len() != handles.len() {
        return Err(MeshError::InvalidBoundary);
    }
    for i in 0..handles.len() {
        let (a, b) = (handles[i], handles[(i + 1) % handles.len()]);
        if !cdt.can_add_constraint(a, b) {
            return Err(MeshError::InvalidBoundary);
        }
        cdt.add_constraint(a, b);
    }
    Ok(handles)
}

/// Where the lines cross or touch each other: for (line, segment) the
/// points on that segment, in order from its start (its ends excluded).
fn crossings(lines: &[Vec<DVec2>]) -> std::collections::HashMap<(usize, usize), Vec<DVec2>> {
    let segments: Vec<(usize, usize, DVec2, DVec2)> = lines
        .iter()
        .enumerate()
        .flat_map(|(k, l)| {
            l.windows(2).enumerate().filter_map(move |(i, w)| match w {
                [a, b] => Some((k, i, *a, *b)),
                _ => None,
            })
        })
        .collect();
    let mut out: std::collections::HashMap<(usize, usize), Vec<(f64, DVec2)>> =
        std::collections::HashMap::new();
    const EPS: f64 = 1e-9;
    // Points this close (mm) are one.
    const NEAR: f64 = 1e-3;
    for (n, (k1, i1, a, b)) in segments.iter().enumerate() {
        for (k2, i2, c, d) in segments.iter().skip(n + 1) {
            // Consecutive segments of one line share their end already.
            if k1 == k2 && i1.abs_diff(*i2) <= 1 {
                continue;
            }
            let (r, s) = (*b - *a, *d - *c);
            let denom = r.perp_dot(s);
            if denom.abs() < EPS {
                continue;
            }
            let w = *c - *a;
            let t = w.perp_dot(s) / denom;
            let u = w.perp_dot(r) / denom;
            if !((-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u)) {
                continue;
            }
            let p = *a + r * t;
            // A crossing inside a segment becomes a vertex of it; at (or
            // within a micrometre of) a segment's end, it is one already.
            let inside = |x: DVec2, y: DVec2| p.distance(x) > NEAR && p.distance(y) > NEAR;
            if inside(*a, *b) {
                out.entry((*k1, *i1)).or_default().push((t, p));
            }
            if inside(*c, *d) {
                out.entry((*k2, *i2)).or_default().push((u, p));
            }
        }
    }
    out.into_iter()
        .map(|(key, mut pts)| {
            pts.sort_by(|x, y| x.0.total_cmp(&y.0));
            (key, pts.into_iter().map(|(_, p)| p).collect())
        })
        .collect()
}

/// Triangulate the closed polygon `boundary` (counter-clockwise, without a
/// repeated first point) with interior triangles of about `spacing` edge
/// length, leaving out `holes` (closed loops inside it) and following
/// `lines` (open polylines inside it: their points become vertices joined
/// by triangle edges, so a seam can run along them). Every outline and
/// hole point is kept and never split; a line's point closer than a fifth
/// of the spacing to another given point becomes that point.
///
/// Constrained Delaunay triangulation with Ruppert/Chew refinement
/// over the outline and the holes. The refinement cannot see the lines
/// (a constraint that does not enclose anything confuses its
/// inside/outside test, a flood fill by parity), so with lines the panel
/// is triangulated again: the outline, the holes, the lines, then the
/// refinement's points clear of the lines (inserted after it, a point
/// a fraction of a millimetre from a line made slivers that shook the
/// cloth). Along each of `fine`'s lines the triangles are finer.
pub fn mesh_panel(
    boundary: &[DVec2],
    holes: &[Vec<DVec2>],
    lines: &[Vec<DVec2>],
    fine: &[FineZone],
    spacing: f64,
) -> Result<PanelMesh, MeshError> {
    if spacing.is_nan() || spacing <= 0.0 {
        return Err(MeshError::InvalidBoundary);
    }
    if lines.iter().flatten().any(|p| !p.is_finite()) {
        return Err(MeshError::LineOutside);
    }
    let lattice = lattice(fine, boundary, holes, lines);
    let mut cdt: Cdt = ConstrainedDelaunayTriangulation::new();
    let mut outer = constrain_loop(&mut cdt, boundary)?;
    let mut hole_handles: Vec<Vec<FixedVertexHandle>> = holes
        .iter()
        .map(|h| constrain_loop(&mut cdt, h))
        .collect::<Result<_, _>>()?;
    for p in &lattice {
        cdt.insert(Point2::new(p.x, p.y))
            .map_err(|_| MeshError::InvalidBoundary)?;
    }
    let given = cdt.num_vertices();
    // An equilateral triangle of side `spacing` has this area.
    let target_area = 3f64.sqrt() / 4.0 * spacing * spacing;
    cdt.refine(
        RefinementParameters::new()
            .keep_constraint_edges()
            .exclude_outer_faces(true)
            .with_max_allowed_area(target_area * 1.2)
            .with_angle_limit(AngleLimit::from_deg(25.0)),
    );
    let mut line_handles = Vec::with_capacity(lines.len());
    if !lines.is_empty() {
        let added: Vec<DVec2> = cdt
            .vertices()
            .filter(|v| v.fix().index() >= given)
            .map(|v| DVec2::new(v.position().x, v.position().y))
            .collect();
        cdt = ConstrainedDelaunayTriangulation::new();
        outer = constrain_loop(&mut cdt, boundary)?;
        hole_handles = holes
            .iter()
            .map(|h| constrain_loop(&mut cdt, h))
            .collect::<Result<_, _>>()?;
        line_handles = constrain_lines(
            &mut cdt,
            boundary,
            holes,
            &outer,
            &hole_handles,
            lines,
            spacing,
        )?;
        let clear = Segments::new(lines, spacing);
        for p in lattice
            .iter()
            .chain(added.iter().filter(|p| !clear.near(**p, spacing * CLEAR)))
        {
            cdt.insert(Point2::new(p.x, p.y))
                .map_err(|_| MeshError::InvalidBoundary)?;
        }
    }
    // Inside the outline and outside every hole, by each triangle's
    // centroid; flat slivers between points in line along the outline
    // (their centroid on it) are not cloth.
    let holes_pts: Vec<&Vec<DVec2>> = holes.iter().collect();
    let flat = 1e-9 * spacing * spacing;
    let triangles: Vec<[u32; 3]> = cdt
        .inner_faces()
        .filter(|f| {
            let [a, b, c] = f.vertices().map(|v| {
                let p = v.position();
                DVec2::new(p.x, p.y)
            });
            let centroid = (a + b + c) / 3.0;
            crate::polygon::signed_area(&[a, b, c]).abs() > flat
                && crate::polygon::contains(boundary, centroid)
                && !holes_pts
                    .iter()
                    .any(|h| crate::polygon::contains(h, centroid))
        })
        .map(|f| f.vertices().map(|v| v.fix().index() as u32))
        .collect();
    // Only the vertices the triangles use, renumbered in their order (the
    // simulation's results depend on the order of its particles).
    let all: Vec<DVec2> = cdt
        .vertices()
        .map(|v| DVec2::new(v.position().x, v.position().y))
        .collect();
    let mut used = vec![false; all.len()];
    for v in triangles.iter().flatten() {
        if let Some(u) = used.get_mut(*v as usize) {
            *u = true;
        }
    }
    let mut index = vec![u32::MAX; all.len()];
    let mut vertices = Vec::new();
    for (i, p) in all.iter().enumerate() {
        if used.get(i).copied().unwrap_or(false)
            && let Some(slot) = index.get_mut(i)
        {
            *slot = vertices.len() as u32;
            vertices.push(*p);
        }
    }
    let triangles_out: Vec<[u32; 3]> = triangles
        .iter()
        .map(|t| t.map(|v| index.get(v as usize).copied().unwrap_or(0)))
        .collect();
    let remap = |hs: &[FixedVertexHandle], err: MeshError| -> Result<Vec<u32>, MeshError> {
        hs.iter()
            .map(|h| {
                index
                    .get(h.index())
                    .copied()
                    .filter(|i| *i != u32::MAX)
                    .ok_or(err.clone())
            })
            .collect()
    };
    Ok(PanelMesh {
        boundary: remap(&outer, MeshError::InvalidBoundary)?,
        holes: hole_handles
            .iter()
            .map(|h| remap(h, MeshError::InvalidBoundary))
            .collect::<Result<_, _>>()?,
        lines: line_handles
            .iter()
            .map(|h| remap(h, MeshError::LineOutside))
            .collect::<Result<_, _>>()?,
        vertices,
        triangles: triangles_out,
    })
}

/// The triangles covering a polygon with holes, for drawing: its points
/// (the outline's, then each hole's) and triangles over them, no point
/// added. `None` when the outline cannot be triangulated.
pub fn fill(boundary: &[DVec2], holes: &[Vec<DVec2>]) -> Option<(Vec<DVec2>, Vec<[u32; 3]>)> {
    let mut cdt: Cdt = ConstrainedDelaunayTriangulation::new();
    constrain_loop(&mut cdt, boundary).ok()?;
    for h in holes {
        constrain_loop(&mut cdt, h).ok()?;
    }
    let vertices: Vec<DVec2> = cdt
        .vertices()
        .map(|v| DVec2::new(v.position().x, v.position().y))
        .collect();
    let triangles = cdt
        .inner_faces()
        .filter(|f| {
            let [a, b, c] = f.vertices().map(|v| {
                let p = v.position();
                DVec2::new(p.x, p.y)
            });
            let centroid = (a + b + c) / 3.0;
            crate::polygon::signed_area(&[a, b, c]).abs() > 1e-12
                && crate::polygon::contains(boundary, centroid)
                && !holes.iter().any(|h| crate::polygon::contains(h, centroid))
        })
        .map(|f| f.vertices().map(|v| v.fix().index() as u32))
        .collect();
    Some((vertices, triangles))
}

/// Resample a closed polyline so consecutive points are at most `spacing`
/// apart, keeping every original point.
pub fn resample_closed(points: &[DVec2], spacing: f64) -> Vec<DVec2> {
    let n = points.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        let steps = ((a.distance(b) / spacing).ceil() as usize).max(1);
        for k in 0..steps {
            out.push(a.lerp(b, k as f64 / steps as f64));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon::signed_area;

    fn square(size: f64) -> Vec<DVec2> {
        vec![
            DVec2::ZERO,
            DVec2::new(size, 0.0),
            DVec2::new(size, size),
            DVec2::new(0.0, size),
        ]
    }

    #[test]
    fn square_mesh_covers_the_panel_and_keeps_the_boundary() {
        let boundary = resample_closed(&square(100.0), 10.0);
        assert_eq!(boundary.len(), 40);
        let m = mesh_panel(&boundary, &[], &[], &[], 10.0).unwrap();
        let area: f64 = m
            .triangles
            .iter()
            .map(|t| signed_area(&t.map(|i| m.vertices[i as usize])))
            .sum();
        assert!((area - 10_000.0).abs() < 1e-6, "area {area}");
        assert!(
            m.triangles
                .iter()
                .all(|t| { signed_area(&t.map(|i| m.vertices[i as usize])) > 0.0 })
        );
        for (i, v) in m.boundary.iter().enumerate() {
            assert_eq!(m.vertices[*v as usize], boundary[i]);
        }
        // Roughly (100 / 10)² interior cells.
        assert!(
            (80..250).contains(&m.vertices.len()),
            "{}",
            m.vertices.len()
        );
    }

    #[test]
    fn holes_are_left_out_and_lines_followed() {
        let boundary = resample_closed(&square(100.0), 10.0);
        let hole: Vec<DVec2> = vec![
            DVec2::new(30.0, 30.0),
            DVec2::new(30.0, 50.0),
            DVec2::new(50.0, 50.0),
            DVec2::new(50.0, 30.0),
        ];
        let line: Vec<DVec2> = (0..=6)
            .map(|k| DVec2::new(70.0, 20.0 + 10.0 * k as f64))
            .collect();
        let m = mesh_panel(
            &boundary,
            std::slice::from_ref(&hole),
            std::slice::from_ref(&line),
            &[],
            10.0,
        )
        .unwrap();
        let area: f64 = m
            .triangles
            .iter()
            .map(|t| signed_area(&t.map(|i| m.vertices[i as usize])))
            .sum();
        assert!((area - (10_000.0 - 400.0)).abs() < 1e-6, "area {area}");
        for (i, v) in m.holes[0].iter().enumerate() {
            assert_eq!(m.vertices[*v as usize], hole[i]);
        }
        // Consecutive line points are joined by a triangle edge.
        for w in m.lines[0].windows(2) {
            let joined = m
                .triangles
                .iter()
                .any(|t| t.contains(&w[0]) && t.contains(&w[1]));
            assert!(joined);
        }
        // Every vertex is used.
        let used: std::collections::HashSet<u32> = m.triangles.iter().flatten().copied().collect();
        assert_eq!(used.len(), m.vertices.len());
        // A line leaving the panel is refused.
        let out = vec![DVec2::new(50.0, 90.0), DVec2::new(50.0, 130.0)];
        assert_eq!(
            mesh_panel(&boundary, &[], &[out], &[], 10.0),
            Err(MeshError::LineOutside)
        );
    }

    #[test]
    fn a_fine_zone_makes_smaller_triangles_along_its_line() {
        let boundary = resample_closed(&square(200.0), 25.0);
        let line: Vec<DVec2> = (0..=20)
            .map(|k| DVec2::new(100.0, 20.0 + 8.0 * k as f64))
            .collect();
        let zone = FineZone {
            line: line.clone(),
            spacing: 8.0,
            rows: 3,
        };
        let lines = std::slice::from_ref(&line);
        let coarse = mesh_panel(&boundary, &[], lines, &[], 25.0).unwrap();
        let fine = mesh_panel(&boundary, &[], lines, std::slice::from_ref(&zone), 25.0).unwrap();
        // The line's segments are triangle edges, the band's triangles small.
        for w in fine.lines[0].windows(2) {
            let joined = fine
                .triangles
                .iter()
                .any(|t| t.contains(&w[0]) && t.contains(&w[1]));
            assert!(joined);
        }
        let near = |m: &PanelMesh| -> (usize, f64) {
            let areas: Vec<f64> = m
                .triangles
                .iter()
                .map(|t| t.map(|i| m.vertices[i as usize]))
                .filter(|t| ((t[0] + t[1] + t[2]).x / 3.0 - 100.0).abs() < 10.0)
                .map(|t| signed_area(&t))
                .collect();
            let mean = areas.iter().sum::<f64>() / areas.len().max(1) as f64;
            (areas.len(), mean)
        };
        let (n_coarse, a_coarse) = near(&coarse);
        let (n_fine, a_fine) = near(&fine);
        assert!(n_fine > 2 * n_coarse, "{n_fine} vs {n_coarse}");
        assert!(a_fine < a_coarse / 2.0, "{a_fine} vs {a_coarse}");
        let positive = fine
            .triangles
            .iter()
            .all(|t| signed_area(&t.map(|i| fine.vertices[i as usize])) > 0.0);
        assert!(positive);
        // Far from the line, the panel's spacing.
        assert!(fine.vertices.len() < coarse.vertices.len() + 400);
    }

    #[test]
    fn crossing_lines_meet_at_a_vertex() {
        let boundary = resample_closed(&square(100.0), 10.0);
        let across: Vec<DVec2> = (0..=6)
            .map(|k| DVec2::new(20.0 + 10.0 * k as f64, 45.0))
            .collect();
        let down = vec![DVec2::new(55.0, 20.0), DVec2::new(55.0, 80.0)];
        let m = mesh_panel(&boundary, &[], &[across.clone(), down.clone()], &[], 10.0).unwrap();
        // The lines keep their own points, and both pass through the
        // crossing at (55, 45).
        assert_eq!(m.lines[0].len(), across.len());
        assert_eq!(m.lines[1].len(), 2);
        assert!(
            m.vertices
                .iter()
                .any(|v| v.distance(DVec2::new(55.0, 45.0)) < 1e-9)
        );
    }

    /// The smallest angle of the mesh's triangles (degrees).
    fn smallest_angle(m: &PanelMesh) -> f64 {
        m.triangles
            .iter()
            .flat_map(|t| {
                let p = t.map(|i| m.vertices[i as usize]);
                (0..3).map(move |k| {
                    let (a, b, c) = (p[k], p[(k + 1) % 3], p[(k + 2) % 3]);
                    (b - a).angle_to(c - a).abs().to_degrees()
                })
            })
            .fold(180.0, f64::min)
    }

    #[test]
    fn lines_ending_near_the_outline_or_other_points_leave_no_sliver() {
        // Quilting lines end on the true curve, a hair off the outline's
        // chords, sometimes a hair from one of its points, and cross each
        // other next to their own points. Inserted as given, such points
        // made triangles of a fraction of a degree that shook the cloth.
        let boundary = resample_closed(&square(200.0), 25.0);
        let mut lines = Vec::new();
        for k in 1..8 {
            let c = 25.0 * k as f64;
            // Down and up diagonals, ending 0.1 mm inside the outline.
            lines.push(vec![DVec2::new(c, 0.1), DVec2::new(0.1, c)]);
            lines.push(vec![DVec2::new(c, 0.1), DVec2::new(199.9, 200.0 - c)]);
        }
        // One ending 0.003 mm from an outline point.
        lines.push(vec![DVec2::new(100.0, 199.997), DVec2::new(100.0, 120.0)]);
        let lines: Vec<Vec<DVec2>> = lines.iter().map(|l| resample_open(l, 9.0)).collect();
        let m = mesh_panel(&boundary, &[], &lines, &[], 10.0).unwrap();
        let angle = smallest_angle(&m);
        assert!(angle > 5.0, "smallest angle {angle}°");
        // The lines are still followed, every point near where it was.
        for (line, at) in lines.iter().zip(&m.lines) {
            for (p, v) in line.iter().zip(at) {
                assert!(p.distance(m.vertices[*v as usize]) <= 10.0 * SNAP + 1e-9);
            }
            for w in at.windows(2) {
                let joined = w[0] == w[1]
                    || m.triangles
                        .iter()
                        .any(|t| t.contains(&w[0]) && t.contains(&w[1]));
                assert!(joined);
            }
        }
        for (i, v) in m.boundary.iter().enumerate() {
            assert_eq!(m.vertices[*v as usize], boundary[i]);
        }
    }

    /// Points along an open polyline at most `spacing` apart.
    fn resample_open(points: &[DVec2], spacing: f64) -> Vec<DVec2> {
        let mut out = Vec::new();
        for w in points.windows(2) {
            let steps = ((w[0].distance(w[1]) / spacing).ceil() as usize).max(1);
            out.extend((0..steps).map(|k| w[0].lerp(w[1], k as f64 / steps as f64)));
        }
        out.extend(points.last());
        out
    }

    #[test]
    fn invalid_boundaries_are_rejected() {
        assert_eq!(
            mesh_panel(&square(10.0)[..2], &[], &[], &[], 1.0),
            Err(MeshError::InvalidBoundary)
        );
        let mut dup = square(10.0);
        dup.push(DVec2::ZERO);
        assert_eq!(
            mesh_panel(&dup, &[], &[], &[], 1.0),
            Err(MeshError::InvalidBoundary)
        );
        assert_eq!(
            mesh_panel(&square(10.0), &[], &[], &[], 0.0),
            Err(MeshError::InvalidBoundary)
        );
    }
}
