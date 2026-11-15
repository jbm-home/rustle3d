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
}

/// Errors meshing a panel.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshError {
    /// Fewer than 3 points, non-finite coordinates or repeated points.
    InvalidBoundary,
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the panel outline cannot be meshed (too few, repeated or invalid points)")
    }
}

impl std::error::Error for MeshError {}

/// Triangulate the closed polygon `boundary` (counter-clockwise, without a
/// repeated first point) with interior triangles of about `spacing` edge
/// length. Boundary points are kept and never split.
pub fn mesh_polygon(boundary: &[DVec2], spacing: f64) -> Result<PanelMesh, MeshError> {
    if boundary.len() < 3
        || boundary.iter().any(|p| !p.is_finite())
        || spacing.is_nan()
        || spacing <= 0.0
    {
        return Err(MeshError::InvalidBoundary);
    }
    let mut cdt: ConstrainedDelaunayTriangulation<Point2<f64>> =
        ConstrainedDelaunayTriangulation::new();
    let handles: Vec<FixedVertexHandle> = boundary
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
    // An equilateral triangle of side `spacing` has this area.
    let target_area = 3f64.sqrt() / 4.0 * spacing * spacing;
    let result = cdt.refine(
        RefinementParameters::new()
            .keep_constraint_edges()
            .exclude_outer_faces(true)
            .with_max_allowed_area(target_area * 1.2)
            .with_angle_limit(AngleLimit::from_deg(25.0)),
    );
    let excluded: std::collections::HashSet<_> = result.excluded_faces.into_iter().collect();
    let vertices = cdt
        .vertices()
        .map(|v| DVec2::new(v.position().x, v.position().y))
        .collect();
    let triangles = cdt
        .inner_faces()
        .filter(|f| !excluded.contains(&f.fix()))
        .map(|f| f.vertices().map(|v| v.fix().index() as u32))
        .collect();
    Ok(PanelMesh {
        vertices,
        triangles,
        boundary: handles.iter().map(|h| h.index() as u32).collect(),
    })
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
        let m = mesh_polygon(&boundary, 10.0).unwrap();
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
    fn invalid_boundaries_are_rejected() {
        assert_eq!(
            mesh_polygon(&square(10.0)[..2], 1.0),
            Err(MeshError::InvalidBoundary)
        );
        let mut dup = square(10.0);
        dup.push(DVec2::ZERO);
        assert_eq!(mesh_polygon(&dup, 1.0), Err(MeshError::InvalidBoundary));
        assert_eq!(
            mesh_polygon(&square(10.0), 0.0),
            Err(MeshError::InvalidBoundary)
        );
    }
}
