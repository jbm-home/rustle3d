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

/// Triangulate the closed polygon `boundary` (counter-clockwise, without a
/// repeated first point) with interior triangles of about `spacing` edge
/// length, leaving out `holes` (closed loops inside it) and following
/// `lines` (open polylines inside it: their points become vertices joined
/// by triangle edges, so a seam can run along them). Every given point is
/// kept and never split.
///
/// Constrained Delaunay triangulation with Ruppert/Chew refinement over
/// the outline and the holes; the lines are inserted after the refinement
/// (a constraint that does not enclose anything would confuse the
/// refinement's inside/outside test).
pub fn mesh_panel(
    boundary: &[DVec2],
    holes: &[Vec<DVec2>],
    lines: &[Vec<DVec2>],
    spacing: f64,
) -> Result<PanelMesh, MeshError> {
    if spacing.is_nan() || spacing <= 0.0 {
        return Err(MeshError::InvalidBoundary);
    }
    let mut cdt: Cdt = ConstrainedDelaunayTriangulation::new();
    let outer = constrain_loop(&mut cdt, boundary)?;
    let hole_handles: Vec<Vec<FixedVertexHandle>> = holes
        .iter()
        .map(|h| constrain_loop(&mut cdt, h))
        .collect::<Result<_, _>>()?;
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
    for line in lines {
        if line.iter().any(|p| !p.is_finite()) {
            return Err(MeshError::LineOutside);
        }
        let handles: Vec<FixedVertexHandle> = line
            .iter()
            .map(|p| cdt.insert(Point2::new(p.x, p.y)))
            .collect::<Result<_, _>>()
            .map_err(|_| MeshError::LineOutside)?;
        for w in handles.windows(2) {
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
        let m = mesh_panel(&boundary, &[], &[], 10.0).unwrap();
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
            mesh_panel(&boundary, &[], &[out], 10.0),
            Err(MeshError::LineOutside)
        );
    }

    #[test]
    fn invalid_boundaries_are_rejected() {
        assert_eq!(
            mesh_panel(&square(10.0)[..2], &[], &[], 1.0),
            Err(MeshError::InvalidBoundary)
        );
        let mut dup = square(10.0);
        dup.push(DVec2::ZERO);
        assert_eq!(
            mesh_panel(&dup, &[], &[], 1.0),
            Err(MeshError::InvalidBoundary)
        );
        assert_eq!(
            mesh_panel(&square(10.0), &[], &[], 0.0),
            Err(MeshError::InvalidBoundary)
        );
    }
}
