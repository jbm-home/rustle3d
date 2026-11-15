#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! End to end: a sewn skirt on the avatar drapes and stays on the hips.

use r3d_avatar::fit::{fit, fitted_positions};
use r3d_commands::drape::{BodyMesh, DEFAULT_SPACING_MM, build};
use r3d_commands::queries::body_model;
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
    let body = project.avatar().unwrap().body;
    let model = body_model().unwrap();
    let positions = fitted_positions(model, &fit(model, &body));
    let drape = build(
        project,
        Some(BodyMesh {
            positions: &positions,
            triangles: model.triangles(),
        }),
        DEFAULT_SPACING_MM,
    )
    .unwrap();
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

    let model = body_model().unwrap();
    let positions = fitted_positions(model, &fit(model, &s.project().avatar().unwrap().body));
    let drape = build(
        s.project(),
        Some(BodyMesh {
            positions: &positions,
            triangles: model.triangles(),
        }),
        DEFAULT_SPACING_MM,
    )
    .unwrap();
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
