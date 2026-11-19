//! Reference scenes of docs/physics.md §8, and their checks, for any
//! solver backend: `r3d-sim` runs them on the CPU, `r3d-sim-gpu` includes
//! this module and runs them on the GPU.

use glam::{Vec2, Vec3};
use r3d_geom::Rigid;
use r3d_sim::collide::{Skin, StaticMesh};
use r3d_sim::motion::BodyMotion;
use r3d_sim::scene::{Closure, Particle, Triangle};
use r3d_sim::{FabricPreset, SimScene, Solver};

/// Makes a solver of the backend under test.
pub type Make<'a> = &'a dyn Fn(SimScene) -> Box<dyn Solver>;

/// A `w × h` metre sheet of `n × n` vertices; `place` maps the flat (u, v)
/// pattern position to its initial 3D position.
pub fn sheet(w: f32, h: f32, n: usize, warp: Vec2, place: impl Fn(Vec2) -> Vec3) -> SimScene {
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

pub fn run(make: Make, scene: SimScene, seconds: f32) -> Box<dyn Solver> {
    let mut s = make(scene);
    for _ in 0..(seconds * 60.0) as usize {
        s.step(1.0 / 60.0).unwrap();
    }
    assert!(
        s.positions().iter().all(|p| p.is_finite()),
        "NaN in the simulation"
    );
    s
}

pub fn min_y(s: &dyn Solver) -> f32 {
    s.positions().iter().map(|p| p.y).fold(f32::MAX, f32::min)
}

/// A closed UV sphere mesh (one vertex per pole).
pub fn sphere(center: Vec3, radius: f32) -> StaticMesh {
    let (rings, segments) = (24u32, 48u32);
    let at = |phi: f32, theta: f32| {
        center + radius * Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin())
    };
    let mut positions = vec![center + Vec3::Y * radius];
    for r in 1..rings {
        let phi = std::f32::consts::PI * r as f32 / rings as f32;
        for s in 0..segments {
            positions.push(at(phi, std::f32::consts::TAU * s as f32 / segments as f32));
        }
    }
    let south = positions.len() as u32;
    positions.push(center - Vec3::Y * radius);
    let ring = |r: u32, s: u32| 1 + (r - 1) * segments + s % segments;
    let mut triangles = Vec::new();
    // Counter-clockwise seen from outside.
    for s in 0..segments {
        triangles.push([0, ring(1, s + 1), ring(1, s)]);
        triangles.push([south, ring(rings - 1, s), ring(rings - 1, s + 1)]);
    }
    for r in 1..rings - 1 {
        for s in 0..segments {
            let (a, b) = (ring(r, s), ring(r, s + 1));
            let (c, d) = (ring(r + 1, s), ring(r + 1, s + 1));
            triangles.push([a, b, c]);
            triangles.push([b, d, c]);
        }
    }
    StaticMesh::new(positions, triangles, 0.005)
}

pub fn hanging_cloth_sags_and_settles(make: Make) {
    let n = 21;
    let mut scene = sheet(0.5, 0.5, n, Vec2::Y, |r| Vec3::new(r.x, 1.5, r.y));
    scene.particles[n * (n - 1)].pinned = true;
    scene.particles[n * n - 1].pinned = true;
    // A sheet hung by two corners is a slow pendulum: it takes about ten
    // seconds to come to rest (energy decreases steadily).
    let mut s = run(make, scene, 12.0);
    assert!(min_y(&*s) < 1.5 - 0.3, "did not sag: {}", min_y(&*s));
    // Settled: over the next second the shape barely changes.
    let before = s.positions().to_vec();
    for _ in 0..60 {
        s.step(1.0 / 60.0).unwrap();
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

pub fn cloth_lands_on_the_ground(make: Make) {
    let mut scene = sheet(0.4, 0.4, 15, Vec2::Y, |r| Vec3::new(r.x, 0.3, r.y));
    scene.ground = true;
    let s = run(make, scene, 2.0);
    let thickness = FabricPreset::Cotton.params().thickness;
    assert!(
        min_y(&*s) >= thickness * 0.99,
        "under the ground: {}",
        min_y(&*s)
    );
    assert!(s.stats().max_speed < 0.2, "{:?}", s.stats());
}

pub fn cloth_drapes_over_a_sphere_without_going_through(make: Make) {
    let center = Vec3::new(0.25, 0.4, 0.25);
    let radius = 0.15;
    let mut scene = sheet(0.5, 0.5, 31, Vec2::Y, |r| Vec3::new(r.x, 0.6, r.y));
    scene.collider = Some(sphere(center, radius));
    scene.ground = true;
    let s = run(make, scene, 2.5);
    let inside = s
        .positions()
        .iter()
        .map(|p| p.distance(center))
        .fold(f32::MAX, f32::min);
    assert!(inside > radius - 0.004, "penetrates the sphere: {inside}");
    // It draped: the corners fell below the sphere's top.
    assert!(min_y(&*s) < center.y);
}

/// A closed box from `lo` to `hi` as a body of one bone, that can move.
pub fn moving_box(lo: Vec3, hi: Vec3) -> StaticMesh {
    let positions: Vec<Vec3> = (0..8)
        .map(|i| {
            Vec3::new(
                if i & 1 == 0 { lo.x } else { hi.x },
                if i & 2 == 0 { lo.y } else { hi.y },
                if i & 4 == 0 { lo.z } else { hi.z },
            )
        })
        .collect();
    // Counter-clockwise seen from outside.
    let quads = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    let triangles = quads
        .iter()
        .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
        .collect();
    let skin = Skin {
        bones: [0; 4],
        weights: [1.0, 0.0, 0.0, 0.0],
    };
    StaticMesh::with_skin(positions, triangles, 0.005, vec![skin; 8])
}

/// A sheet lying on a table-like box; then the box moves 20 cm sideways
/// and 5 cm up in a second. The cloth goes along and stays out of it.
pub fn a_moving_body_carries_the_cloth(make: Make) {
    let (lo, hi) = (Vec3::new(0.1, 0.1, 0.1), Vec3::new(0.4, 0.4, 0.4));
    let shift = Vec3::new(0.2, 0.05, 0.0);
    let mut scene = sheet(0.4, 0.4, 25, Vec2::Y, |r| {
        Vec3::new(r.x + 0.05, 0.45, r.y + 0.05)
    });
    scene.collider = Some(moving_box(lo, hi));
    scene.ground = true;
    let ease = |x: f32| x * x * (3.0 - 2.0 * x);
    let offset = |k: usize| shift * ease(k.min(60) as f32 / 60.0);
    scene.motion = Some(BodyMotion {
        start: 1.0,
        fps: 60.0,
        intro: (0..=60)
            .map(|k| {
                vec![Rigid {
                    rotation: glam::Quat::IDENTITY,
                    translation: offset(k),
                }]
            })
            .collect(),
        cycle: Vec::new(),
    });
    let mean_x = |s: &dyn Solver| {
        s.positions().iter().map(|p| p.x).sum::<f32>() / s.positions().len() as f32
    };
    let mut s = run(make, scene, 1.0);
    let before = mean_x(&*s);
    for frame in 1..=90 {
        s.step(1.0 / 60.0).unwrap();
        // Inside the moved box by more than 4 mm.
        let (a, b) = (lo + offset(frame), hi + offset(frame));
        let inside = s.positions().iter().find(|p| {
            p.cmpgt(a + Vec3::splat(0.004)).all() && p.cmplt(b - Vec3::splat(0.004)).all()
        });
        assert!(
            inside.is_none(),
            "frame {frame}: inside the box: {inside:?}"
        );
    }
    assert!(s.positions().iter().all(|p| p.is_finite()));
    let carried = mean_x(&*s) - before;
    assert!(
        (carried - shift.x).abs() < 0.03,
        "the cloth did not go along: {carried} m"
    );
    let top = s.positions().iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(top > hi.y + shift.y, "the cloth fell off: {top}");
}

pub fn two_panels_sewn_into_a_tube_close_their_seams(make: Make) {
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
    let s = run(make, scene, 1.5);
    assert!(
        s.stats().max_seam_gap < 0.002,
        "seams open: {:?}",
        s.stats()
    );
}

/// Two bands hanging side by side, overlapping by 5 cm, 3 cm apart in
/// depth, held by three buttons: the buttons close at a bounded speed and
/// stay closed, the overlapping bands do not push each other away, and
/// nothing blows up. Without the buttons the bands hang apart.
pub fn buttons_hold_two_bands_together(make: Make) {
    let n = 13;
    let build = |buttoned: bool| {
        let left = sheet(0.3, 0.3, n, Vec2::Y, |r| Vec3::new(r.x, 0.5 + r.y, 0.0));
        let right = sheet(0.3, 0.3, n, Vec2::Y, |r| {
            Vec3::new(r.x + 0.25, 0.5 + r.y, 0.03)
        });
        let offset = left.particles.len() as u32;
        let mut scene = left;
        scene.particles.extend(right.particles);
        scene
            .triangles
            .extend(right.triangles.into_iter().map(|t| Triangle {
                vertices: t.vertices.map(|v| v + offset),
                ..t
            }));
        // Each band held by its top edge.
        for i in 0..n {
            scene.particles[n * (n - 1) + i].pinned = true;
            scene.particles[offset as usize + n * (n - 1) + i].pinned = true;
        }
        if buttoned {
            // Column 11 of the left band (x = 0.275) on column 1 of the
            // right one (x = 0.25 + 0.025), rows 1, 5 and 9.
            for j in [1, 5, 9] {
                let a = (j * n + 11) as u32;
                let b = offset + (j * n + 1) as u32;
                scene.closures.push(Closure {
                    pair: [a, b],
                    overlap: 0.06,
                });
            }
        }
        scene
    };
    // One frame: the buttons pull at the closing speed, no faster.
    let mut s = make(build(true));
    s.step(1.0 / 60.0).unwrap();
    let start = s.stats();
    assert!(start.max_closure_gap > 0.02, "{start:?}");
    assert!(start.max_speed < 2.0, "snapped shut: {start:?}");
    let s = run(make, build(true), 1.5);
    let st = s.stats();
    assert!(st.max_closure_gap < 0.002, "buttons open: {st:?}");
    // The free bottom corners: the bands hang together, not pushed apart.
    let (l, r) = (s.positions()[11], s.positions()[n * n + 1]);
    assert!(l.distance(r) < 0.04, "{l} {r}");
    let free = run(make, build(false), 1.5);
    let (l, r) = (free.positions()[11], free.positions()[n * n + 1]);
    assert!((l.z - r.z).abs() > 0.02, "unbuttoned bands met: {l} {r}");
}

pub fn jersey_stretches_more_along_the_weft(make: Make) {
    let hang = |warp: Vec2| {
        let n = 15;
        let mut scene = sheet(0.2, 0.6, n, warp, |r| Vec3::new(r.x, 1.0 + r.y, 0.0));
        scene.fabrics = vec![FabricPreset::Jersey.params()];
        for i in 0..n {
            scene.particles[n * (n - 1) + i].pinned = true;
        }
        min_y(&*run(make, scene, 2.0))
    };
    // The hem rests at y = 1.0: how far below it is the stretch.
    let along_warp = 1.0 - hang(Vec2::Y); // warp vertical
    let along_weft = 1.0 - hang(Vec2::X); // weft vertical
    assert!(
        along_warp > 0.0 && along_weft > 1.5 * along_warp,
        "weft {along_weft} m vs warp {along_warp} m"
    );
}

pub fn stacked_sheets_do_not_pass_through_each_other(make: Make) {
    let n = 15;
    let lower = sheet(0.3, 0.3, n, Vec2::Y, |r| Vec3::new(r.x, 0.1, r.y));
    let upper = sheet(0.3, 0.3, n, Vec2::Y, |r| {
        Vec3::new(r.x + 0.02, 0.15, r.y + 0.02)
    });
    let offset = lower.particles.len() as u32;
    let mut scene = lower;
    scene.particles.extend(upper.particles);
    scene
        .triangles
        .extend(upper.triangles.into_iter().map(|t| Triangle {
            vertices: t.vertices.map(|v| v + offset),
            ..t
        }));
    scene.ground = true;
    let s = run(make, scene, 2.0);
    let (low, high) = s.positions().split_at(offset as usize);
    let top_of_lower = low.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    let bottom_of_upper = high.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    // Without self-collision both end flat on the ground at the same
    // height. With it the upper sheet stays above: its particles (17 mm
    // apart) nest in the gaps of the lower grid, about 8 mm higher.
    assert!(
        bottom_of_upper > top_of_lower + 0.005,
        "upper {bottom_of_upper} vs lower {top_of_lower}"
    );
}

pub fn grabbed_cloth_follows_the_hand_and_falls_when_released(make: Make) {
    let n = 15;
    let mut scene = sheet(0.3, 0.3, n, Vec2::Y, |r| Vec3::new(r.x, 0.0, r.y));
    scene.ground = true;
    let mut s = run(make, scene, 0.5);
    let corner = n * n - 1;
    let start = s.positions()[corner];
    s.grab(corner);
    let hand = start + Vec3::new(0.0, 0.3, 0.0);
    s.move_grab(hand);
    for _ in 0..60 {
        s.step(1.0 / 60.0).unwrap();
    }
    let held = s.positions()[corner];
    assert!(
        held.distance(hand) < 0.03,
        "corner at {held}, hand at {hand}"
    );
    s.release();
    for _ in 0..60 {
        s.step(1.0 / 60.0).unwrap();
    }
    // Down from the hand at 30 cm; it may land on folds of its own cloth,
    // each layer two particle radii (17 mm here) thick.
    let y = s.positions()[corner].y;
    assert!(y < 0.1, "did not fall back: at {y} m");
}
