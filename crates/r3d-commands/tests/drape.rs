#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! End to end: a sewn skirt on the avatar drapes and stays on the hips.

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
mod common;

use r3d_commands::{Session, Step};
use r3d_core::Project;
use r3d_sim::{CpuSolver, Solver};

#[test]
fn skirt_template_closes_and_stays_on_the_body() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    common::a_line_skirt(&mut s);
    let seams = s
        .run_step(&Step::from_value(serde_json::json!({"type": "seam.list"})).unwrap())
        .unwrap();
    let seams = seams.as_array().unwrap();
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

    let mut solver = CpuSolver::new(drape.scene);
    let start = std::time::Instant::now();
    for _ in 0..120 {
        solver.step(1.0 / 60.0).unwrap();
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
    common::women_m(&mut s);
    let front = common::a_line_skirt(&mut s);
    // Hold the middle of the front hem out in front of the knees.
    let hand = [0.05, 0.6, 0.35];
    let id: r3d_core::PanelId = front.panel.as_str().unwrap().parse().unwrap();
    let b = s.project().panel_bounding_box(id).unwrap();
    let pin = serde_json::json!({"type": "pin.add", "panel": front.panel,
        "at": [(b.min.x + b.max.x) / 2.0, 0], "position": hand});
    s.run_step(&Step::from_value(pin).unwrap()).unwrap();

    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let held = drape.pins[0].1.clone();
    assert!(held.len() > 3, "{held:?}");
    let mut solver = CpuSolver::new(drape.scene);
    for _ in 0..60 {
        solver.step(1.0 / 60.0).unwrap();
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
    common::women_m(&mut s);
    let front = common::a_line_skirt(&mut s);
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let old = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let mut solver = CpuSolver::new(old.scene.clone());
    for _ in 0..90 {
        solver.step(1.0 / 60.0).unwrap();
    }
    let draped = solver.positions().to_vec();

    // Lengthen the front by 50 mm: move its two hem corners down.
    for point in &front.points[..2] {
        let id: r3d_core::PointId = point.as_str().unwrap().parse().unwrap();
        let x = s.project().point(id).unwrap().pos.x;
        let step = serde_json::json!({"type": "point.move", "point": point, "to": [x, -50.0]});
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
    // The added fabric (rest y < 0) starts next to the drape (no jump)...
    let added: Vec<usize> = front_new
        .particles
        .clone()
        .filter(|i| rest(&new, *i).y < -0.049)
        .collect();
    assert!(!added.is_empty());
    for i in &added {
        let gap = front_old
            .particles
            .clone()
            .map(|j| draped[j].distance(pos(&new, *i)))
            .fold(f32::MAX, f32::min);
        assert!(gap < 0.08, "jump of {gap} m");
    }
    // ...and once it has hung for a second, the longer skirt is lower than
    // the same drape without the change.
    let lowest = |s: &CpuSolver, piece: &r3d_commands::drape::Piece| {
        piece
            .particles
            .clone()
            .map(|i| s.positions()[i].y)
            .fold(f32::MAX, f32::min)
    };
    let mut longer = CpuSolver::new(new.scene.clone());
    longer.continue_at(solver.time());
    for _ in 0..60 {
        solver.step(1.0 / 60.0).unwrap();
        longer.step(1.0 / 60.0).unwrap();
    }
    let (before, after) = (lowest(&solver, front_old), lowest(&longer, front_new));
    assert!(after < before - 0.02, "{before} -> {after}");
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

    let out = r3d_commands::drape::simulate(s.project(), 120, &r3d_commands::Progress::default())
        .unwrap();
    let top = out.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(out.stats.max_seam_gap < 0.005, "{:?}", out.stats);
    assert!(top > 0.95, "slid down: {top}");
}

#[test]
fn style_lines_and_edges_follow_the_cloth() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    let front = common::a_line_skirt(&mut s);
    let line = serde_json::json!({"type": "internal_line.add", "panel": front.panel,
        "start": [100, 300], "segments": [{"kind": "line", "to": [600, 300]}]});
    s.run_step(&Step::from_value(line).unwrap()).unwrap();
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let positions: Vec<glam::Vec3> = drape.scene.particles.iter().map(|p| p.position).collect();
    let on = r3d_commands::drape::LinesOnCloth::new(&drape, &positions);
    let lines = on.internal_lines(s.project());
    assert_eq!(lines.len(), 1);
    // Sampled every 10 mm along the 500 mm line, all on the front piece, at
    // the same height (it is horizontal on the pattern and the start pose
    // keeps heights), each with a unit normal.
    let line = &lines[0];
    assert!(line.len() > 40, "{}", line.len());
    let y0 = line[0].0.y;
    assert!(
        line.iter()
            .all(|(p, n)| (p.y - y0).abs() < 1e-3 && (n.length() - 1.0).abs() < 1e-4)
    );
    let hem: r3d_core::EdgeId = front.edges[0].as_str().unwrap().parse().unwrap();
    let edge = on.edge(hem).unwrap();
    assert!(edge.len() > 10);
}
