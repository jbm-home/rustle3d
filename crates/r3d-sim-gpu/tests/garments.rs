#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Every starter garment drapes on the GPU as on the CPU: the template
//! checks of `r3d-commands` (nothing inside the body, triangles not cut
//! into it, seams and closures closed, the T-shirt at rest), and the same
//! extent as the CPU drape within a couple of centimetres.

#[path = "../../r3d-commands/tests/common/mod.rs"]
mod common;
mod gpu;

use glam::Vec3;
use r3d_commands::actions::AvatarMotion;
use r3d_commands::drape::{Animation, DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Session, Step};
use r3d_core::Project;
use r3d_sim::{CpuSolver, SimScene, Solver};
use r3d_sim_gpu::GpuSolver;
use serde_json::json;

fn garment(template: &str) -> SimScene {
    let mut s = Session::new(Project::new("Untitled"));
    for step in [
        json!({"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
            "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
        json!({"type": "template.apply", "template": template}),
    ] {
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
    }
    let form = Form::of_project(s.project()).unwrap().unwrap();
    build(s.project(), Some(&form), DEFAULT_SPACING_MM)
        .unwrap()
        .scene
}

/// Lowest and highest point after `frames` frames, after the checks.
fn drape(solver: &mut dyn Solver, scene: &SimScene, frames: u32) -> (f32, f32) {
    for _ in 0..frames {
        solver.step(1.0 / 60.0).unwrap();
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let body = scene.collider.as_ref().unwrap();
    let inside = p
        .iter()
        .filter(|q| body.sample(**q).is_some_and(|c| c.distance < 0.0))
        .count();
    assert_eq!(inside, 0, "particles inside the body");
    let sunk = scene
        .triangles
        .iter()
        .map(|t| t.vertices.map(|i| p[i as usize]))
        .flat_map(|[a, b, c]| {
            [
                (a + b + c) / 3.0,
                (a + b) / 2.0,
                (b + c) / 2.0,
                (c + a) / 2.0,
            ]
        })
        .filter_map(|q| body.sample(q).map(|c| -c.distance))
        .fold(0.0f32, f32::max);
    assert!(sunk < 0.001, "triangles cut {sunk} m into the body");
    let stats = solver.stats();
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    assert!(stats.max_closure_gap < 0.005, "unbuttoned: {stats:?}");
    let low = p.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let high = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    (low, high)
}

#[test]
fn starter_garments_drape_on_the_gpu_as_on_the_cpu() {
    let Some(gpu) = gpu::context() else {
        return;
    };
    for template in [
        "t_shirt",
        "shirt",
        "trousers",
        "dress",
        "straight_skirt",
        "a_line_skirt",
        "tote_bag",
    ] {
        let scene = garment(template);
        let on_gpu = drape(
            &mut GpuSolver::new(gpu, scene.clone()).unwrap(),
            &scene,
            180,
        );
        let on_cpu = drape(&mut CpuSolver::new(scene.clone()), &scene, 180);
        eprintln!("{template}: gpu {on_gpu:.3?}, cpu {on_cpu:.3?}");
        // Trousers hang from the waist by friction alone: the two contact
        // solvers let them settle about 3 cm apart (both at rest by 180
        // frames).
        let within = if template == "trousers" { 0.04 } else { 0.02 };
        assert!(
            (on_gpu.0 - on_cpu.0).abs() < within && (on_gpu.1 - on_cpu.1).abs() < within,
            "{template}: gpu {on_gpu:?} vs cpu {on_cpu:?}"
        );
    }
}

#[test]
fn a_t_shirt_drapes_on_an_imported_body_on_the_gpu() {
    let Some(gpu) = gpu::context() else {
        return;
    };
    // The built-in body written in centimetres, Z up, its soles open.
    let body: r3d_core::Body = serde_json::from_value(json!({"masculinity": 0,
        "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}))
    .unwrap();
    let form = Form::of_body(&body, &Default::default()).unwrap();
    let mut obj = String::new();
    for p in &form.positions {
        obj += &format!(
            "v {} {} {}
",
            p.x * 100.0,
            -p.z * 100.0,
            p.y * 100.0
        );
    }
    for t in &form.triangles {
        if t.iter().any(|v| form.positions[*v as usize].y > 0.015) {
            obj += &format!(
                "f {} {} {}
",
                t[0] + 1,
                t[1] + 1,
                t[2] + 1
            );
        }
    }
    let loaded =
        r3d_commands::bodies::register(obj.as_bytes(), r3d_io::model::ModelFormat::Obj).unwrap();
    let mut s = Session::new(Project::new("Untitled"));
    for step in [
        json!({"type": "avatar.import_mesh", "file": "avatars/body.obj", "hash": loaded.hash}),
        json!({"type": "template.apply", "template": "t_shirt"}),
    ] {
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
    }
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let scene = build(s.project(), Some(&form), DEFAULT_SPACING_MM)
        .unwrap()
        .scene;
    let on_gpu = drape(
        &mut GpuSolver::new(gpu, scene.clone()).unwrap(),
        &scene,
        180,
    );
    let on_cpu = drape(&mut CpuSolver::new(scene.clone()), &scene, 180);
    assert!(
        (on_gpu.0 - on_cpu.0).abs() < 0.02 && (on_gpu.1 - on_cpu.1).abs() < 0.02,
        "gpu {on_gpu:?} vs cpu {on_cpu:?}"
    );
}

#[test]
fn a_t_shirt_comes_to_rest_on_the_gpu() {
    let Some(gpu) = gpu::context() else {
        return;
    };
    let mut solver = GpuSolver::new(gpu, garment("t_shirt")).unwrap();
    for _ in 0..240 {
        solver.step(1.0 / 60.0).unwrap();
    }
    let before: Vec<Vec3> = solver.positions().to_vec();
    solver.step(1.0 / 60.0).unwrap();
    let speed = solver
        .positions()
        .iter()
        .zip(&before)
        .map(|(a, b)| a.distance(*b) * 60.0)
        .sum::<f32>()
        / before.len() as f32;
    assert!(speed < 0.01, "still moving at {speed} m/s on average");
}

#[test]
fn a_dress_walks_on_the_gpu() {
    let Some(gpu) = gpu::context() else {
        return;
    };
    let mut s = Session::new(Project::new("Untitled"));
    for step in [
        json!({"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
            "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
        json!({"type": "template.apply", "template": "dress"}),
    ] {
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
    }
    dress_walks(gpu, &s);
}

/// The built-in body imported from an OBJ (no skeleton in the file: ours
/// is placed on it, spec 27) walks in a dress on the GPU.
#[test]
fn a_dress_walks_on_an_imported_body_on_the_gpu() {
    let Some(gpu) = gpu::context() else {
        return;
    };
    let body: r3d_core::Body = serde_json::from_value(json!({"masculinity": 0,
        "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}))
    .unwrap();
    let form = Form::of_body(&body, &Default::default()).unwrap();
    let mut obj = String::new();
    for p in &form.positions {
        obj += &format!(
            "v {} {} {}
",
            p.x, p.y, p.z
        );
    }
    for t in &form.triangles {
        obj += &format!(
            "f {} {} {}
",
            t[0] + 1,
            t[1] + 1,
            t[2] + 1
        );
    }
    let loaded =
        r3d_commands::bodies::register(obj.as_bytes(), r3d_io::model::ModelFormat::Obj).unwrap();
    let mut s = Session::new(Project::new("Untitled"));
    for step in [
        json!({"type": "avatar.import_mesh", "file": "avatars/walker.obj", "hash": loaded.hash}),
        json!({"type": "template.apply", "template": "dress"}),
    ] {
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
    }
    dress_walks(gpu, &s);
}

/// The dress of `s` walks on the GPU: 1.5 s of draping, a second of
/// easing in, a step cycle; it stays finite, outside the moving body,
/// sewn, and up on the shoulders.
fn dress_walks(gpu: &'static r3d_sim_gpu::GpuContext, s: &Session) {
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let mut scene = build(s.project(), Some(&form), DEFAULT_SPACING_MM)
        .unwrap()
        .scene;
    scene.motion = form.body_motion(&Animation::starting(AvatarMotion::Walk.into(), 0.0));
    let mut solver = GpuSolver::new(gpu, scene.clone()).unwrap();
    let frames = 220;
    for _ in 0..frames {
        solver.step(1.0 / 60.0).unwrap();
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let (body, moving) = (
        scene.collider.as_ref().unwrap(),
        scene.motion.as_ref().unwrap(),
    );
    let t = frames as f32 / 60.0;
    let posed = r3d_sim::motion::Posed::new(moving.at(t - 1.0 / 1800.0), moving.at(t));
    let inside = p
        .iter()
        .filter_map(|q| r3d_sim::motion::touch(body, Some(&posed), *q))
        .map(|(c, _)| -c.distance)
        .fold(0.0f32, f32::max);
    assert!(inside < 0.003, "{inside} m inside the walking body");
    assert!(solver.stats().max_seam_gap < 0.01, "{:?}", solver.stats());
    let top = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    assert!(top > 1.2, "the dress slid down: {top}");
}

/// A shirt worn under trousers drapes tucked into them on the GPU as on
/// the CPU (`r3d-commands`' `a_shirt_under_trousers_is_tucked_in`): no
/// shirt shows through the trousers from the hips to under the waistband.
#[test]
fn a_shirt_under_trousers_is_tucked_in_on_the_gpu() {
    let Some(gpu) = gpu::context() else {
        return;
    };
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    for template in ["shirt", "trousers"] {
        s.run_step(
            &Step::from_value(json!({"type": "template.apply", "template": template})).unwrap(),
        )
        .unwrap();
    }
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let scene = build(s.project(), Some(&form), DEFAULT_SPACING_MM)
        .unwrap()
        .scene;
    let mut solver = GpuSolver::new(gpu, scene.clone()).unwrap();
    drape(&mut solver, &scene, 150);
    let x = solver.positions();
    let top = scene
        .triangles
        .iter()
        .filter(|t| t.garment == 1)
        .flat_map(|t| t.vertices)
        .map(|v| x[v as usize])
        .filter(|p| p.x.abs() < 0.1)
        .map(|p| p.y)
        .fold(0.0f32, f32::max);
    let seen = common::seen_through(&scene, x, 0, 1, 0.85, top - 0.03);
    assert!(
        seen.is_empty(),
        "shirt showing through the trousers below {top}: {:?}",
        seen.iter().map(|i| (i, x[*i])).collect::<Vec<_>>()
    );
}
