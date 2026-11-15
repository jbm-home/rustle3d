#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! End to end: a sewn skirt on the avatar drapes and stays on the hips.

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Script, Session, Step, templates};
use r3d_core::Project;
use r3d_sim::Solver;

#[test]
fn skirt_template_closes_and_stays_on_the_body() {
    let mut s = Session::new(Project::new("t"));
    let body = serde_json::json!({"type": "avatar.set", "preset": "Women M",
        "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}});
    s.run_step(&Step::from_value(body).unwrap()).unwrap();
    let script = Script::parse(templates::get("a-line-skirt").unwrap()).unwrap();
    let results = script.run(|step| s.run_step(step)).unwrap();
    let seams = results.last().unwrap().as_array().unwrap();
    assert_eq!(seams.len(), 3);
    assert!(seams.iter().all(|s| s["mismatch_mm"] == 0.0), "{seams:?}");

    let project = s.project();
    let form = Form::of_project(project).unwrap().unwrap();
    let drape = build(project, Some(&form), DEFAULT_SPACING_MM).unwrap();
    assert!(!drape.scene.seams.is_empty());
    // All pieces share the same waistline and hem: sewn points pair at the
    // same pattern height (a reversed seam would twist the skirt).
    let rest = |i: u32| drape.scene.particles[i as usize].rest.y;
    let twist = drape
        .scene
        .seams
        .iter()
        .map(|[a, b]| (rest(*a) - rest(*b)).abs())
        .fold(0.0, f32::max);
    assert!(twist < 0.002, "twisted seam: {twist} m");
    let n = drape.scene.particles.len();
    eprintln!(
        "{n} particles, {} triangles, {} seam pairs",
        drape.scene.triangles.len(),
        drape.scene.seams.len()
    );

    let mut solver = Solver::new(drape.scene);
    let start = std::time::Instant::now();
    for _ in 0..120 {
        solver.step(1.0 / 60.0);
    }
    let per_frame = start.elapsed() / 120;
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let stats = solver.stats();
    let low = p.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let high = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    eprintln!("{per_frame:?}/frame, stats {stats:?}, y {low:.3}..{high:.3}");
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    // Placed at the waist (1.10 m) and fitted to it (74 cm for a 73 cm
    // waist), it stays there instead of sliding down to the hips.
    assert!(high > 1.05, "skirt slid down: top at {high}");
    assert!(low > 0.2, "skirt collapsed: hem at {low}");
}

#[test]
fn a_pin_holds_the_cloth_where_it_says() {
    let mut s = Session::new(Project::new("t"));
    let body = serde_json::json!({"type": "avatar.set", "preset": "Women M",
        "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}});
    s.run_step(&Step::from_value(body).unwrap()).unwrap();
    let script = Script::parse(templates::get("a-line-skirt").unwrap()).unwrap();
    let results = script.run(|step| s.run_step(step)).unwrap();
    let front = results[2]["created"][0].clone();
    // Hold the middle of the front hem out in front of the knees.
    let hand = [0.05, 0.6, 0.35];
    let pin =
        serde_json::json!({"type": "pin.add", "panel": front, "at": [390, 0], "position": hand});
    s.run_step(&Step::from_value(pin).unwrap()).unwrap();

    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let held = drape.pins[0].1.clone();
    assert!(held.len() > 3, "{held:?}");
    let mut solver = Solver::new(drape.scene);
    for _ in 0..60 {
        solver.step(1.0 / 60.0);
    }
    let at = solver.positions()[held[0]];
    assert!(
        at.distance(glam::Vec3::from_array(hand.map(|v| v as f32))) < 1e-4,
        "{at}"
    );
    assert!(solver.stats().max_seam_gap < 0.005);
}

#[test]
fn edits_carry_the_drape_over_without_a_jump() {
    let mut s = Session::new(Project::new("t"));
    let body = serde_json::json!({"type": "avatar.set",
        "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}});
    s.run_step(&Step::from_value(body).unwrap()).unwrap();
    let script = Script::parse(templates::get("a-line-skirt").unwrap()).unwrap();
    let results = script.run(|step| s.run_step(step)).unwrap();
    let front = &results[2]["created"];
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let old = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let mut solver = Solver::new(old.scene.clone());
    for _ in 0..90 {
        solver.step(1.0 / 60.0);
    }
    let draped = solver.positions().to_vec();

    // Lengthen the front by 50 mm: move its two hem corners down.
    for (k, x) in [(1, 0.0), (2, 780.0)] {
        let step = serde_json::json!({"type": "point.move", "point": front[k], "to": [x, -50.0]});
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
    }
    let mut new = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    r3d_commands::drape::carry_over(&old, &draped, &mut new);

    let front_old = &old.pieces[0];
    let front_new = &new.pieces[0];
    let pos = |d: &r3d_commands::drape::Drape, i: usize| d.scene.particles[i].position;
    let rest = |d: &r3d_commands::drape::Drape, i: usize| d.scene.particles[i].rest;
    // A waist particle (rest y = 0.6 m) is where the drape left it.
    let waist_new = front_new
        .particles
        .clone()
        .find(|i| rest(&new, *i).y > 0.599)
        .unwrap();
    let waist_old = front_old
        .particles
        .clone()
        .min_by(|a, b| {
            rest(&old, *a)
                .distance(rest(&new, waist_new))
                .total_cmp(&rest(&old, *b).distance(rest(&new, waist_new)))
        })
        .unwrap();
    assert!(pos(&new, waist_new).distance(draped[waist_old]) < 0.005);
    // The new hem (rest y < 0) continues the drape below the old hem.
    let old_hem = front_old
        .particles
        .clone()
        .map(|i| draped[i].y)
        .fold(f32::MAX, f32::min);
    let new_hem = front_new
        .particles
        .clone()
        .map(|i| pos(&new, i).y)
        .fold(f32::MAX, f32::min);
    assert!(
        new_hem < old_hem - 0.03 && new_hem > old_hem - 0.08,
        "{old_hem} -> {new_hem}"
    );
}

#[test]
fn a_skirt_made_from_scratch_sews_arranges_and_drapes() {
    let mut s = Session::new(Project::new("t"));
    let run =
        |s: &mut Session, v: serde_json::Value| s.run_step(&Step::from_value(v).unwrap()).unwrap();
    run(
        &mut s,
        serde_json::json!({"type": "avatar.set",
        "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
    );
    // Two trapezoids, no placement, no seam direction given.
    let a = run(
        &mut s,
        serde_json::json!({"type": "panel.create_polygon", "name": "Front",
        "points": [[0, 0], [600, 0], [500, 600], [100, 600]]}),
    );
    let b = run(
        &mut s,
        serde_json::json!({"type": "panel.create_polygon", "name": "Back",
        "points": [[700, 0], [1300, 0], [1200, 600], [800, 600]]}),
    );
    let (a, b) = (&a["created"], &b["created"]);
    run(
        &mut s,
        serde_json::json!({"type": "seam.add", "a": {"edge": a[6]}, "b": {"edge": b[8]}}),
    );
    run(
        &mut s,
        serde_json::json!({"type": "seam.add", "a": {"edge": b[6]}, "b": {"edge": a[8]}}),
    );
    run(&mut s, serde_json::json!({"type": "garment.arrange"}));

    let out = r3d_commands::drape::simulate(s.project(), 120).unwrap();
    let top = out.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(out.stats.max_seam_gap < 0.005, "{:?}", out.stats);
    assert!(top > 0.95, "slid down: {top}");
}
