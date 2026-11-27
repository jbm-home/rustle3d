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
                    garment: 0,
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
    // Down from the hand at 30 cm. Where the corner ends is chaotic: it
    // may rest on a fold standing 12 cm high, and a 5 mm change of the hand
    // flips that. The cloth's mean height is not: a pile of a few layers
    // (25 to 55 mm), against more than 100 mm while held.
    let y = s.positions()[corner].y;
    assert!(y < 0.15, "corner did not fall back: at {y} m");
    let mean = s.positions().iter().map(|p| p.y).sum::<f32>() / s.positions().len() as f32;
    assert!(mean < 0.07, "cloth did not fall back: mean height {mean} m");
}

/// The length of the polyline through `ids` at the solver's positions.
fn path_length(s: &dyn Solver, ids: &[usize]) -> f32 {
    ids.windows(2)
        .map(|w| s.positions()[w[0]].distance(s.positions()[w[1]]))
        .sum()
}

pub fn an_elastic_gathers_the_edge_it_is_sewn_on(make: Make) {
    // A 40 × 30 cm sheet hung by its top edge, an elastic along its
    // bottom edge pulling it to 60 % of its length (a sleeve's hem).
    let n = 21;
    let mut scene = sheet(0.4, 0.3, n, Vec2::Y, |r| Vec3::new(r.x, 1.0 + r.y, 0.0));
    for i in 0..n {
        scene.particles[(n - 1) * n + i].pinned = true;
    }
    let edge: Vec<usize> = (0..n).collect();
    scene.elastics.push(r3d_sim::ElasticLine {
        particles: edge.iter().map(|i| *i as u32).collect(),
        ratio: 0.6,
        stiffness: 8.0,
    });
    let s = run(make, scene, 3.0);
    let gathered = path_length(&*s, &edge);
    // Gravity pulls the hem open a little against the elastic.
    assert!(
        (gathered - 0.24).abs() < 0.24 * 0.08,
        "the elastic's edge is {gathered} m long, 0.24 wanted"
    );
    // The cloth beside it gathers: the next rows keep most of their
    // length on the cloth but span much less.
    let row: Vec<usize> = (2 * n..3 * n).collect();
    let along = path_length(&*s, &row);
    let span = s.positions()[row[0]].distance(s.positions()[row[n - 1]]);
    assert!(
        along > 1.2 * span,
        "no gathers: {along} m along, {span} m across"
    );
}

/// A closed vertical cylinder mesh of `radius` from `y0` up to `y1`, on
/// the Y axis.
pub fn cylinder(radius: f32, y0: f32, y1: f32) -> StaticMesh {
    let segments = 48u32;
    let mut positions = vec![Vec3::new(0.0, y1, 0.0), Vec3::new(0.0, y0, 0.0)];
    for y in [y1, y0] {
        for s in 0..segments {
            let a = std::f32::consts::TAU * s as f32 / segments as f32;
            positions.push(Vec3::new(radius * a.cos(), y, radius * a.sin()));
        }
    }
    let (top, bottom) = (
        |s: u32| 2 + s % segments,
        |s: u32| 2 + segments + s % segments,
    );
    let mut triangles = Vec::new();
    // Counter-clockwise seen from outside.
    for s in 0..segments {
        triangles.push([0, top(s + 1), top(s)]);
        triangles.push([1, bottom(s), bottom(s + 1)]);
        triangles.push([top(s), top(s + 1), bottom(s)]);
        triangles.push([top(s + 1), bottom(s + 1), bottom(s)]);
    }
    StaticMesh::new(positions, triangles, 0.005)
}

/// How far the top of a jersey tube 30 cm long, 14 cm in radius, hung
/// around a 12 cm cylinder (a body's waist), slides from the end of the
/// sewing ramp (when an elastic has reached its relaxed length) to 2 s;
/// with `elastic`, an elastic at 70 % along its top edge. Also its seam's
/// largest gap at the end.
fn tube_slides(make: Make, elastic: bool) -> (f32, f32) {
    let (n, radius, top) = (24, 0.14, 1.0);
    let around = std::f32::consts::TAU * radius;
    let mut scene = sheet(around, 0.3, n, Vec2::Y, |r| {
        let a = std::f32::consts::TAU * r.x / around;
        Vec3::new(radius * a.cos(), top - 0.3 + r.y, -radius * a.sin())
    });
    scene.fabrics = vec![FabricPreset::Jersey.params()];
    // Closed along its first and last columns.
    scene.seams = (0..n)
        .map(|j| [(j * n) as u32, (j * n + n - 1) as u32])
        .collect();
    let rim: Vec<usize> = ((n - 1) * n..n * n).collect();
    if elastic {
        scene.elastics.push(r3d_sim::ElasticLine {
            particles: rim.iter().map(|i| *i as u32).collect(),
            ratio: 0.7,
            stiffness: 40.0,
        });
    }
    scene.collider = Some(cylinder(0.12, 0.3, 1.2));
    let height =
        |s: &dyn Solver| rim.iter().map(|i| s.positions()[*i].y).sum::<f32>() / rim.len() as f32;
    let mut s = make(scene);
    let frames = |seconds: f32| (seconds * 60.0).round() as usize;
    for _ in 0..frames(r3d_sim::solver::SEWING_TIME) {
        s.step(1.0 / 60.0).unwrap();
    }
    let sewn = height(&*s);
    for _ in frames(r3d_sim::solver::SEWING_TIME)..frames(2.0) {
        s.step(1.0 / 60.0).unwrap();
    }
    assert!(
        s.positions().iter().all(|p| p.is_finite()),
        "NaN in the simulation"
    );
    (sewn - height(&*s), s.stats().max_seam_gap)
}

pub fn an_elastic_holds_the_cloth_up_on_the_body(make: Make) {
    // In XPBD with one iteration a substep, a stiff constraint the body
    // keeps from closing pushes the cloth into it no further than its
    // error times the curvature, a hundredth of its tension's pressure:
    // friction from that push alone let elastic waists slide off. Its
    // tension counts in the friction (docs/physics.md, "Elastic").
    let (bare, _) = tube_slides(make, false);
    assert!(bare > 0.1, "a loose tube should slide down: {bare} m");
    let (held, gap) = tube_slides(make, true);
    assert!(held < 0.01, "the elastic let the tube slide {held} m");
    // The elastic pulls the seam's two ends apart along the rim and its
    // pressure made friction hold each where it was: the seam stayed 36
    // mm open (docs/physics.md §5, "Friction on a seam").
    assert!(gap < 0.002, "the seam beside the elastic is {gap} m open");
}

/// How high a 30 × 8 cm band of `fabric`, clamped at its foot and leaning
/// 5° at the start, stands after 3 s: its top's height over its width.
fn band_stands(make: Make, fabric: r3d_sim::FabricParams) -> f32 {
    let n = 16;
    let lean = 5f32.to_radians();
    let mut scene = sheet(0.3, 0.08, n, Vec2::Y, |r| {
        Vec3::new(r.x, 1.0 + r.y * lean.cos(), r.y * lean.sin())
    });
    scene.fabrics = vec![fabric];
    // Two rows held: a single one would be a hinge the band turns about
    // (an edge of one triangle bends nothing).
    for p in scene.particles.iter_mut().take(2 * n) {
        p.pinned = true;
    }
    let s = run(make, scene, 3.0);
    let top: f32 = (n * (n - 1)..n * n)
        .map(|i| s.positions()[i].y)
        .sum::<f32>()
        / n as f32;
    (top - 1.0) / 0.08
}

pub fn an_interfaced_band_stands_where_a_bare_one_falls(make: Make) {
    let cotton = FabricPreset::Cotton.params();
    let heavy = r3d_sim::fabric::interfacing(
        r3d_sim::InterfacingMake::NonWoven,
        r3d_sim::InterfacingWeight::Heavy,
    );
    let bare = band_stands(make, cotton);
    let fused = band_stands(make, cotton.fused(heavy));
    // Cotton's bending length is about 2 cm: 8 cm of it bends over
    // (0.52 measured); fused, about 6.5 cm, it stands (0.95 on the CPU,
    // 0.86 on the GPU, whose sweep in colour order converges the stiff
    // hinges a little less).
    assert!(bare < 0.7, "the bare band stands {bare}");
    assert!(fused > 0.8, "the interfaced band falls: {fused}");
}

/// A cushion: two `side` metre squares of `n × n` vertices facing out,
/// `gap` apart, sewn all around (or along three sides only with `open`),
/// weightless, stuffed at `pressure` (Pa). The front's vertices come
/// first, then the back's.
fn cushion(side: f32, n: usize, pressure: f32, open: bool) -> SimScene {
    let gap = 0.03;
    let front = sheet(side, side, n, Vec2::Y, |r| {
        Vec3::new(r.x, 1.0 + r.y, gap / 2.0)
    });
    // Mirrored, so it faces −z: its outside is out of the cushion too.
    let back = sheet(side, side, n, Vec2::Y, |r| {
        Vec3::new(side - r.x, 1.0 + r.y, -gap / 2.0)
    });
    let offset = front.particles.len() as u32;
    let mut scene = front;
    scene.particles.extend(back.particles);
    scene
        .triangles
        .extend(back.triangles.into_iter().map(|t| Triangle {
            vertices: t.vertices.map(|v| v + offset),
            ..t
        }));
    let at = |i: usize, j: usize| (j * n + i) as u32;
    let mirror = |i: usize, j: usize| offset + at(n - 1 - i, j);
    let mut edge: Vec<(usize, usize)> = Vec::new();
    for k in 0..n {
        edge.push((0, k));
        edge.push((n - 1, k));
        edge.push((k, 0));
        if !open {
            edge.push((k, n - 1));
        }
    }
    edge.sort_unstable();
    edge.dedup();
    scene.seams = edge
        .iter()
        .map(|(i, j)| [at(*i, *j), mirror(*i, *j)])
        .collect();
    scene.weightless = true;
    scene.pressure = pressure;
    scene
}

fn centroid(s: &dyn Solver) -> Vec3 {
    s.positions().iter().sum::<Vec3>() / s.positions().len() as f32
}

/// The largest angle between the normals of two triangles sharing an
/// edge, over the triangles of the first `particles` particles whose
/// corners all lie at least `margin` vertices in from the sheet's edges
/// (radians): how creased the cloth is away from its seams.
fn largest_crease(s: &dyn Solver, scene: &SimScene, n: usize, margin: usize) -> f32 {
    let x = s.positions();
    let inner = |v: u32| {
        let (i, j) = (v as usize % (n * n) % n, v as usize % (n * n) / n);
        i >= margin && j >= margin && i < n - margin && j < n - margin
    };
    let mut by_edge: std::collections::HashMap<(u32, u32), Vec3> = Default::default();
    let mut worst = 0.0f32;
    for t in &scene.triangles {
        let v = t.vertices;
        if !v.iter().all(|i| inner(*i)) {
            continue;
        }
        let normal = (x[v[1] as usize] - x[v[0] as usize])
            .cross(x[v[2] as usize] - x[v[0] as usize])
            .normalize();
        for k in 0..3 {
            let (a, b) = (v[k], v[(k + 1) % 3]);
            let key = (a.min(b), a.max(b));
            if let Some(other) = by_edge.insert(key, normal) {
                worst = worst.max(normal.dot(other).clamp(-1.0, 1.0).acos());
            }
        }
    }
    worst
}

/// How thick a cushion (see [`cushion`]) is in its middle after 3 s,
/// how far it drifted, and the solver.
fn swell(make: Make, scene: SimScene, n: usize) -> (f32, f32, Box<dyn Solver>) {
    let start =
        scene.particles.iter().map(|p| p.position).sum::<Vec3>() / scene.particles.len() as f32;
    let s = run(make, scene, 3.0);
    let middle = (n / 2) * n + n / 2;
    let depth = s.positions()[middle].z - s.positions()[n * n + middle].z;
    (depth, centroid(&*s).distance(start), s)
}

/// Weightless, a cushion keeps the shape its seams give it: its faces
/// smooth, its seams closed, where it was laid, at rest. Stuffed, it
/// swells into a pillow, as smooth.
pub fn a_stuffed_cushion_swells_smooth_and_stays_put(make: Make) {
    let (side, n) = (0.3, 16);
    let mut depths = Vec::new();
    for pressure in [0.0f32, 200.0] {
        let scene = cushion(side, n, pressure, false);
        let (depth, moved, mut s) = swell(make, scene.clone(), n);
        depths.push(depth);
        let st = s.stats();
        assert!(st.max_seam_gap < 0.003, "seams open: {st:?}");
        assert!(moved < 0.01, "drifted {moved} m at {pressure} Pa");
        // Three rows in from the seams: a pillow's edge turns sharply
        // there; a crease is 30° and more between two triangles, its
        // round about 15° on 2 cm triangles.
        let crease = largest_crease(&*s, &scene, n, 3).to_degrees();
        assert!(crease < 20.0, "creased by {crease}° at {pressure} Pa");
        let before = s.positions().to_vec();
        s.step(1.0 / 60.0).unwrap();
        let fastest = s
            .positions()
            .iter()
            .zip(&before)
            .map(|(a, b)| a.distance(*b) * 60.0)
            .fold(0.0, f32::max);
        assert!(
            fastest < 0.05,
            "still moving at {fastest} m/s ({pressure} Pa)"
        );
    }
    let [bare, stuffed] = depths[..] else {
        unreachable!()
    };
    assert!(
        stuffed > bare + 0.03,
        "not stuffed: {stuffed} m thick, {bare} m without stuffing"
    );
}

/// An open bag is not stuffed (it would fly off its mouth), even with a
/// pressure; weightless, it stays where it was laid.
pub fn an_open_bag_is_not_stuffed(make: Make) {
    let n = 16;
    let (bare, _, _) = swell(make, cushion(0.3, n, 0.0, true), n);
    let (pushed, moved, _) = swell(make, cushion(0.3, n, 200.0, true), n);
    assert!(
        (pushed - bare).abs() < 1e-4,
        "the open bag swelled: {pushed} m, {bare} m without"
    );
    assert!(moved < 0.01, "drifted {moved} m");
}
