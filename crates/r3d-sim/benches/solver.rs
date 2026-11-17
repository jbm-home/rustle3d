//! Solver cost per frame (docs/physics.md §7): a sheet draped over a
//! sphere at the particle counts of the targets (skirt ≈ 4k, T-shirt ≈ 10k).
//! Run with `cargo bench -p r3d-sim`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // bench code

use criterion::{Criterion, criterion_group, criterion_main};
use glam::{Vec2, Vec3};
use r3d_sim::collide::StaticMesh;
use r3d_sim::scene::{Particle, Triangle};
use r3d_sim::{CpuSolver, FabricPreset, SimScene, Solver};

/// A closed UV sphere.
fn sphere(center: Vec3, radius: f32) -> StaticMesh {
    let (rings, segments) = (32u32, 64u32);
    let mut positions = vec![center + Vec3::Y * radius];
    for r in 1..rings {
        let phi = std::f32::consts::PI * r as f32 / rings as f32;
        for s in 0..segments {
            let theta = std::f32::consts::TAU * s as f32 / segments as f32;
            positions.push(
                center
                    + radius
                        * Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()),
            );
        }
    }
    let south = positions.len() as u32;
    positions.push(center - Vec3::Y * radius);
    let ring = |r: u32, s: u32| 1 + (r - 1) * segments + s % segments;
    let mut triangles = Vec::new();
    for s in 0..segments {
        triangles.push([0, ring(1, s + 1), ring(1, s)]);
        triangles.push([south, ring(rings - 1, s), ring(rings - 1, s + 1)]);
    }
    for r in 1..rings - 1 {
        for s in 0..segments {
            let (a, b, c, d) = (
                ring(r, s),
                ring(r, s + 1),
                ring(r + 1, s),
                ring(r + 1, s + 1),
            );
            triangles.push([a, b, c]);
            triangles.push([b, d, c]);
        }
    }
    StaticMesh::new(positions, triangles, 0.005)
}

/// An `n × n` sheet of 1 m above a sphere, already falling for 1 s.
fn draping(n: usize) -> CpuSolver {
    let mut particles = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            let rest = Vec2::new(i as f32, j as f32) / (n - 1) as f32;
            particles.push(Particle {
                position: Vec3::new(rest.x - 0.5, 1.0, rest.y - 0.5),
                rest,
                pinned: false,
            });
        }
    }
    let mut triangles = Vec::new();
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = (j * n + i) as u32;
            let (b, c, d) = (a + 1, a + n as u32, a + n as u32 + 1);
            for vertices in [[a, b, d], [a, d, c]] {
                triangles.push(Triangle {
                    vertices,
                    fabric: 0,
                    warp: Vec2::Y,
                });
            }
        }
    }
    let mut solver = CpuSolver::new(SimScene {
        particles,
        triangles,
        fabrics: vec![FabricPreset::Cotton.params()],
        collider: Some(sphere(Vec3::new(0.0, 0.6, 0.0), 0.3)),
        ground: true,
        ..SimScene::default()
    });
    for _ in 0..60 {
        solver.step(1.0 / 60.0).unwrap();
    }
    solver
}

fn frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame");
    group.sample_size(20);
    for (name, n) in [("4k particles", 64), ("10k particles", 100)] {
        let mut solver = draping(n);
        group.bench_function(name, |b| b.iter(|| solver.step(1.0 / 60.0).unwrap()));
    }
    group.finish();
}

criterion_group!(benches, frame);
criterion_main!(benches);
