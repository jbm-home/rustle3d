//! What the solver simulates: plain data, built by the caller from the
//! document (the "sim bridge"), so the solver knows nothing of patterns.

use glam::{Vec2, Vec3};

use crate::collide::StaticMesh;
use crate::fabric::FabricParams;
use crate::motion::BodyMotion;

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

/// Two points held together by a closure: a button through its
/// buttonhole, or a closed stretch of zipper. A stiff tie that closes at
/// a bounded speed ([`crate::solver::CLOSING_SPEED`]), so a garment can be
/// fastened at any time, not only while its seams close.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Closure {
    pub pair: [u32; 2],
    /// The cloth within this distance of each end, on its pattern (m),
    /// lies on the cloth around the other end on purpose (a front band
    /// buttoned over the other): the two patches do not repel each other.
    /// 0: only the usual neighbourhood of a seam.
    pub overlap: f32,
}

/// A complete scene.
#[derive(Debug, Clone, Default)]
pub struct SimScene {
    pub particles: Vec<Particle>,
    pub triangles: Vec<Triangle>,
    pub fabrics: Vec<FabricParams>,
    /// Vertex pairs sewn together.
    pub seams: Vec<[u32; 2]>,
    /// Vertex pairs held by a closure (a button, a closed zipper).
    pub closures: Vec<Closure>,
    /// The body, if any.
    pub collider: Option<StaticMesh>,
    /// How the body's parts move; `None`: it stands still. Needs a
    /// collider built with its parts.
    pub motion: Option<BodyMotion>,
    /// Collide with the ground plane y = 0.
    pub ground: bool,
}
