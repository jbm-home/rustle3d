//! What the solver simulates: plain data, built by the caller from the
//! document (the "sim bridge"), so the solver knows nothing of patterns.

use glam::{Vec2, Vec3};

use crate::collide::StaticMesh;
use crate::fabric::FabricParams;

/// A cloth vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    /// Initial position (m).
    pub position: Vec3,
    /// Rest position on the flat pattern (m): the shape the cloth wants.
    pub rest: Vec2,
    /// Held in place (infinite mass).
    pub pinned: bool,
}

/// A cloth triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub vertices: [u32; 3],
    /// Index into [`SimScene::fabrics`].
    pub fabric: u16,
    /// Warp (grainline) direction on the pattern, unit length.
    pub warp: Vec2,
}

/// A complete scene.
#[derive(Debug, Clone, Default)]
pub struct SimScene {
    pub particles: Vec<Particle>,
    pub triangles: Vec<Triangle>,
    pub fabrics: Vec<FabricParams>,
    /// Vertex pairs sewn together.
    pub seams: Vec<[u32; 2]>,
    /// The body, if any.
    pub collider: Option<StaticMesh>,
    /// Collide with the ground plane y = 0.
    pub ground: bool,
}
