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

/// A cloth triangle. Its normal `(b − a) × (c − a)` points out of its
/// garment (the pattern seen from its right side): contacts between
/// garments ([`crate::layers`]) keep each garment on the outer face of
/// those worn under it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub vertices: [u32; 3],
    /// Index into [`SimScene::fabrics`].
    pub fabric: u16,
    /// The garment it belongs to, numbered from the body out: a garment
    /// is worn over those with lower numbers. Cloth of different
    /// garments touches through thin layer contacts ([`crate::layers`]),
    /// cloth of one garment through particle repulsion
    /// ([`crate::selfcollide`]).
    pub garment: u16,
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

/// An elastic sewn along a line of particles (in order along it): it
/// pulls each stretch of the line to `ratio` of its length on the pattern
/// (docs/physics.md, "Elastic").
#[derive(Debug, Clone, PartialEq)]
pub struct ElasticLine {
    pub particles: Vec<u32>,
    /// Relaxed length over the line's length on the pattern (0–1].
    pub ratio: f32,
    /// Newtons per unit strain.
    pub stiffness: f32,
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
    /// Elastics along lines of particles.
    pub elastics: Vec<ElasticLine>,
    /// Mesh edges that bend differently from the cloth around them (a
    /// quilting's stitched lines): their two particles and the bending
    /// rigidity there (N·m).
    pub creases: Vec<([u32; 2], f32)>,
    /// The body, if any.
    pub collider: Option<StaticMesh>,
    /// How the body's parts move; `None`: it stands still. Needs a
    /// collider built with its parts.
    pub motion: Option<BodyMotion>,
    /// Collide with the ground plane y = 0.
    pub ground: bool,
}
