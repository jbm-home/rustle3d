#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Reference scenes of docs/physics.md §8.

use glam::{Vec2, Vec3};
use r3d_sim::collide::StaticMesh;
use r3d_sim::scene::{Particle, Triangle};
use r3d_sim::{FabricPreset, SimScene, Solver};

/// A `w × h` metre sheet of `n × n` vertices; `place` maps the flat (u, v)
/// pattern position to its initial 3D position.
fn sheet(w: f32, h: f32, n: usize, warp: Vec2, place: impl Fn(Vec2) -> Vec3) -> SimScene {
    let mut particles = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let rest = Vec2::new(w * i as f32 / (n - 1) as f32, h * j as f32 / (n - 1) as f32);
            particles.push(Particle {
                position: place(rest),
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
            for v in [[a, b, d], [a, d, c]] {
                triangles.push(Triangle {
                    vertices: v,
                    fabric: 0,
                    warp,
                });
            }
        }
    }
    SimScene {
        particles,
        triangles,
        fabrics: vec![FabricPreset::Cotton.params()],
        ..SimScene::default()
    }
}

fn run(scene: SimScene, seconds: f32) -> Solver {
    let mut s = Solver::new(scene);
    for _ in 0..(seconds * 60.0) as usize {
        s.step(1.0 / 60.0);
    }
    assert!(
        s.positions().iter().all(|p| p.is_finite()),
        "NaN in the simulation"
    );
    s
}

fn min_y(s: &Solver) -> f32 {
    s.positions().iter().map(|p| p.y).fold(f32::MAX, f32::min)
}

#[test]
fn hanging_cloth_sags_and_settles() {
    let n = 21;
    let mut scene = sheet(0.5, 0.5, n, Vec2::Y, |r| Vec3::new(r.x, 1.5, r.y));
    scene.particles[n * (n - 1)].pinned = true;
    scene.particles[n * n - 1].pinned = true;
    // A sheet hung by two corners is a slow pendulum: it takes about ten
    // seconds to come to rest (energy decreases steadily).
    let mut s = run(scene, 12.0);
    assert!(min_y(&s) < 1.5 - 0.3, "did not sag: {}", min_y(&s));
    // Settled: over the next second the shape barely changes.
    let before = s.positions().to_vec();
    for _ in 0..60 {
        s.step(1.0 / 60.0);
    }
    let moved: Vec<f32> = s
        .positions()
        .iter()
        .zip(&before)
        .map(|(a, b)| a.distance(*b))
        .collect();
    let mean = moved.iter().sum::<f32>() / moved.len() as f32;
    let max = moved.iter().copied().fold(0.0, f32::max);
    assert!(
        mean < 0.005 && max < 0.02,
        "still moving: mean {mean} m, max {max} m in 1 s"
    );
    // The pins did not move.
    assert_eq!(s.positions()[n * n - 1].y, 1.5);
}

#[test]
fn cloth_lands_on_the_ground() {
    let mut scene = sheet(0.4, 0.4, 15, Vec2::Y, |r| Vec3::new(r.x, 0.3, r.y));
    scene.ground = true;
    let s = run(scene, 2.0);
    let thickness = FabricPreset::Cotton.params().thickness;
    assert!(
        min_y(&s) >= thickness * 0.99,
        "under the ground: {}",
        min_y(&s)
    );
    assert!(s.stats().max_speed < 0.2, "{:?}", s.stats());
}

/// An icosphere-like UV sphere mesh.
fn sphere(center: Vec3, radius: f32) -> StaticMesh {
    let (rings, segments) = (24, 48);
    let mut positions = Vec::new();
    for r in 0..=rings {
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
    let mut triangles = Vec::new();
    for r in 0..rings {
        for s in 0..segments {
            let a = (r * segments + s) as u32;
            let b = (r * segments + (s + 1) % segments) as u32;
            let (c, d) = (a + segments as u32, b + segments as u32);
            // Outward-facing (counter-clockwise seen from outside).
            triangles.push([a, b, c]);
            triangles.push([b, d, c]);
        }
    }
    StaticMesh::new(positions, triangles, 0.05)
}

#[test]
fn cloth_drapes_over_a_sphere_without_going_through() {
    let center = Vec3::new(0.25, 0.4, 0.25);
    let radius = 0.15;
    let mut scene = sheet(0.5, 0.5, 31, Vec2::Y, |r| Vec3::new(r.x, 0.6, r.y));
    scene.collider = Some(sphere(center, radius));
    scene.ground = true;
    let s = run(scene, 2.5);
    let inside = s
        .positions()
        .iter()
        .map(|p| p.distance(center))
        .fold(f32::MAX, f32::min);
    assert!(inside > radius - 0.004, "penetrates the sphere: {inside}");
    // It draped: the corners fell below the sphere's top.
    assert!(min_y(&s) < center.y);
}

#[test]
fn two_panels_sewn_into_a_tube_close_their_seams() {
    let n = 13;
    let front = sheet(0.3, 0.3, n, Vec2::Y, |r| Vec3::new(r.x, 0.5 + r.y, 0.05));
    let back = sheet(0.3, 0.3, n, Vec2::Y, |r| Vec3::new(r.x, 0.5 + r.y, -0.05));
    let offset = front.particles.len() as u32;
    let mut scene = front;
    scene.particles.extend(back.particles);
    scene
        .triangles
        .extend(back.triangles.into_iter().map(|t| Triangle {
            vertices: t.vertices.map(|v| v + offset),
            ..t
        }));
    for j in 0..n {
        let left = (j * n) as u32;
        let right = (j * n + n - 1) as u32;
        scene.seams.push([left, left + offset]);
        scene.seams.push([right, right + offset]);
    }
    // Hold the front panel by its top edge; the back hangs from the seams.
    for i in 0..n {
        scene.particles[n * (n - 1) + i].pinned = true;
    }
    let s = run(scene, 1.5);
    assert!(
        s.stats().max_seam_gap < 0.002,
        "seams open: {:?}",
        s.stats()
    );
}

#[test]
fn jersey_stretches_more_along_the_weft() {
    let hang = |warp: Vec2| {
        let n = 15;
        let mut scene = sheet(0.2, 0.6, n, warp, |r| Vec3::new(r.x, 1.0 + r.y, 0.0));
        scene.fabrics = vec![FabricPreset::Jersey.params()];
        for i in 0..n {
            scene.particles[n * (n - 1) + i].pinned = true;
        }
        min_y(&run(scene, 2.0))
    };
    // The hem rests at y = 1.0: how far below it is the stretch.
    let along_warp = 1.0 - hang(Vec2::Y); // warp vertical
    let along_weft = 1.0 - hang(Vec2::X); // weft vertical
    assert!(
        along_warp > 0.0 && along_weft > 1.5 * along_warp,
        "weft {along_weft} m vs warp {along_warp} m"
    );
}

#[test]
fn simulation_is_deterministic() {
    let make = || {
        let mut scene = sheet(0.3, 0.3, 11, Vec2::Y, |r| Vec3::new(r.x, 0.4, r.y));
        scene.ground = true;
        scene
    };
    let a = run(make(), 0.5);
    let b = run(make(), 0.5);
    assert_eq!(a.positions(), b.positions());
}
