//! Construction details (spec 21): seams on part of an edge, on runs of
//! edges and on internal lines, with ease; darts and pleats; and a skirt
//! with darts, a pleat and a patch pocket, draped.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use r3d_sim::{CpuSolver, Solver};
use serde_json::{Value, json};

fn run(s: &mut Session, step: Value) -> Result<Value, CommandError> {
    s.run_step(&Step::from_value(step)?)
}

fn ok(s: &mut Session, step: Value) -> Value {
    run(s, step.clone()).unwrap_or_else(|e| panic!("{step}: {e}"))
}

fn id(v: &Value, k: usize) -> String {
    v["created"][k].as_str().unwrap().to_owned()
}

#[test]
fn partial_chained_and_line_seams() {
    let mut s = Session::new(Project::new("t"));
    let body = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 400, "height": 600}),
    );
    let pocket = ok(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [500, 0],
        "width": 150, "height": 160}),
    );
    // The pocket's placement: a U on the body piece.
    let line = ok(
        &mut s,
        json!({"type": "internal_line.add", "panel": id(&body, 0),
        "start": [100, 400], "segments": [
            {"kind": "line", "to": [100, 240]}, {"kind": "line", "to": [250, 240]},
            {"kind": "line", "to": [250, 400]}]}),
    );
    let line = id(&line, 0);
    // The pocket's left side (top to bottom), bottom and right side: a run
    // of edges walked around the pocket (edges 4, 1, 2 from its top left).
    let pe: Vec<String> = (5..9).map(|k| id(&pocket, k)).collect();
    let seam = ok(
        &mut s,
        json!({"type": "seam.add",
        "a": {"edges": [pe[3], pe[0], pe[1]]}, "b": {"line": line}}),
    );
    let list = ok(&mut s, json!({"type": "seam.list"}));
    assert_eq!(list[0]["a"]["length_mm"], 470.0);
    assert_eq!(list[0]["b"]["length_mm"], 470.0);
    assert_eq!(list[0]["mismatch_mm"], 0.0);
    // The run walks the same way as the line: b is not reversed.
    assert_eq!(list[0]["b"]["reversed"], false);
    // Edges that do not follow each other are refused.
    let err = run(
        &mut s,
        json!({"type": "seam.add",
        "a": {"edges": [pe[0], pe[2]]}, "b": {"edge": id(&body, 5)}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.not_joined");
    // Part of an edge: the body's right side from 100 to 300 mm, with ease.
    let be: Vec<String> = (5..9).map(|k| id(&body, k)).collect();
    ok(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": be[1], "from_mm": 100, "to_mm": 300},
        "b": {"edge": pe[2], "from_mm": 0, "to_mm": 140}, "ease_mm": 60}),
    );
    let list = ok(&mut s, json!({"type": "seam.list"}));
    assert_eq!(list[1]["a"]["length_mm"], 200.0);
    assert_eq!(list[1]["mismatch_mm"], 0.0);
    // The same stretch cannot be sewn twice; the rest of the edge can.
    let err = run(
        &mut s,
        json!({"type": "seam.add",
        "a": {"edge": be[1], "from_mm": 250, "to_mm": 400}, "b": {"edge": be[3]}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "sewn");
    ok(
        &mut s,
        json!({"type": "seam.add",
        "a": {"edge": be[1], "from_mm": 300, "to_mm": 600}, "b": {"edge": be[3],
        "from_mm": 0, "to_mm": 300}}),
    );
    s.project().check_integrity().unwrap();
    // Every seam meshes into matching pairs, the line's too.
    ok(
        &mut s,
        json!({"type": "panel.place", "panel": id(&body, 0), "angle_deg": 0, "top_cm": 104}),
    );
    ok(
        &mut s,
        json!({"type": "panel.place", "panel": id(&pocket, 0), "angle_deg": 0, "top_cm": 90}),
    );
    let drape = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    let rest = |i: u32| drape.scene.particles[i as usize].rest;
    // The pocket's run meets the line point for point, by position on the
    // pattern (the pocket sits 400 mm to the right, 0 to 160 vs 240 to 400).
    let pairs: Vec<_> = drape
        .scene
        .seams
        .iter()
        .filter(|[a, b]| (rest(*a).x > 0.45) != (rest(*b).x > 0.45))
        .collect();
    let on_line = pairs
        .iter()
        .filter(|[a, b]| {
            let body = if rest(*a).x < 0.45 {
                rest(*a)
            } else {
                rest(*b)
            };
            (0.099..0.251).contains(&body.x) && (0.239..0.401).contains(&body.y)
        })
        .count();
    // 470 mm at 25 mm: 19 segments, 20 points.
    assert!(on_line >= 19, "{on_line}");
    let _ = seam;
}

#[test]
fn a_dart_is_drawn_folded_and_sewn() {
    let mut s = Session::new(Project::new("t"));
    let p = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 300, "height": 500}),
    );
    let waist = id(&p, 7); // the top edge, walked right to left
    let out = ok(
        &mut s,
        json!({"type": "dart.add", "edge": waist, "at_mm": 100,
        "width_mm": 30, "length_mm": 120}),
    );
    let apex = id(&out, 0);
    let got = ok(&mut s, json!({"type": "panel.get", "panel": id(&p, 0)}));
    assert_eq!(got["point_count"], 7);
    assert_eq!(got["is_simple"], true);
    let at = s.project().point(apex.parse().unwrap()).unwrap().pos;
    // From x = 200 (100 mm along the top from the right), 120 mm down.
    assert!(
        (at.x - 200.0).abs() < 1e-6 && (at.y - 380.0).abs() < 1e-6,
        "{at:?}"
    );
    let seams = ok(&mut s, json!({"type": "seam.list"}));
    assert_eq!(seams[0]["mismatch_mm"], 0.0);
    // Undo takes it all back.
    ok(&mut s, json!({"type": "history.undo"}));
    let got = ok(&mut s, json!({"type": "panel.get", "panel": id(&p, 0)}));
    assert_eq!(got["point_count"], 4);
    assert!(s.project().seams().is_empty());
}

#[test]
fn pleats_fold_an_edge_onto_itself() {
    let mut s = Session::new(Project::new("t"));
    let p = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 400, "height": 500}),
    );
    let waist = id(&p, 7);
    let knife = ok(
        &mut s,
        json!({"type": "pleat.add", "edge": waist, "at_mm": 50,
        "depth_mm": 20}),
    );
    assert_eq!(knife["created"].as_array().unwrap().len(), 1 + 3);
    let boxed = ok(
        &mut s,
        json!({"type": "pleat.add", "edge": waist, "at_mm": 200,
        "depth_mm": 25, "kind": "box", "stitched_mm": 80}),
    );
    // Two fold seams, two stitched lines and their seam, five notches.
    let created = boxed["created"].as_array().unwrap();
    assert_eq!(
        created
            .iter()
            .filter(|c| c.as_str().unwrap().starts_with("seam:"))
            .count(),
        3
    );
    assert_eq!(
        created
            .iter()
            .filter(|c| c.as_str().unwrap().starts_with("line:"))
            .count(),
        2
    );
    assert_eq!(
        created
            .iter()
            .filter(|c| c.as_str().unwrap().starts_with("notch:"))
            .count(),
        5
    );
    // Overlapping the first pleat is refused.
    let err = run(
        &mut s,
        json!({"type": "pleat.add", "edge": waist, "at_mm": 60,
        "depth_mm": 10}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "sewn");
    s.project().check_integrity().unwrap();
    ok(
        &mut s,
        json!({"type": "panel.place", "panel": id(&p, 0), "angle_deg": 0, "top_cm": 104}),
    );
    let drape = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    // The knife pleat's folds meet: 50 mm along the top meets 90 mm.
    let rest = |i: u32| drape.scene.particles[i as usize].rest;
    let x = |mm: f32| (400.0 - mm) / 1000.0;
    assert!(drape.scene.seams.iter().any(|[a, b]| {
        (rest(*a).x - x(50.0)).abs() < 1e-4 && (rest(*b).x - x(90.0)).abs() < 1e-4
    }));
}

#[test]
fn a_skirt_with_darts_a_pleat_and_a_patch_pocket_drapes() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    // Roomier at the waist: the details take some of it in.
    ok(
        &mut s,
        json!({"type": "template.apply", "template": "a_line_skirt",
        "sizes": {"length_cm": 60, "waist_cm": 79}}),
    );
    let front = common::piece(&s, "Front");
    let len = |s: &Session, e: &str| s.project().edge_length(e.parse().unwrap()).unwrap();
    // A dart on either side of the front waist, then a knife pleat in
    // the middle. A dart cuts the edge: the edge keeps its first part, so
    // the far dart goes first, and the pleat on what is between them.
    let fw = front.edges[2].as_str().unwrap().to_owned();
    let l = len(&s, &fw);
    ok(
        &mut s,
        json!({"type": "dart.add", "edge": fw, "at_mm": 0.8 * l, "width_mm": 16,
        "length_mm": 100}),
    );
    let dart = ok(
        &mut s,
        json!({"type": "dart.add", "edge": fw, "at_mm": 0.2 * l,
        "width_mm": 16, "length_mm": 100}),
    );
    let leg2: r3d_core::EdgeId = id(&dart, 2).parse().unwrap();
    let after = s.project().edge(leg2).unwrap().end;
    let (middle, _) = s
        .project()
        .edges()
        .iter()
        .find(|(_, e)| e.start == after)
        .unwrap();
    let middle = middle.to_string();
    let m = len(&s, &middle);
    ok(
        &mut s,
        json!({"type": "pleat.add", "edge": middle, "at_mm": m / 2.0 - 15.0,
        "depth_mm": 15, "stitched_mm": 60}),
    );
    // A patch pocket on the front, on its placement line.
    let fp = ok(&mut s, json!({"type": "panel.get", "panel": front.panel}));
    let (lo, hi) = (&fp["bbox"]["min"], &fp["bbox"]["max"]);
    let cx = (lo[0].as_f64().unwrap() + hi[0].as_f64().unwrap()) / 2.0 + 60.0;
    let top = hi[1].as_f64().unwrap() - 180.0;
    let line = ok(
        &mut s,
        json!({"type": "internal_line.add", "panel": front.panel,
        "start": [cx - 60.0, top], "segments": [
            {"kind": "line", "to": [cx - 60.0, top - 130.0]},
            {"kind": "line", "to": [cx + 60.0, top - 130.0]},
            {"kind": "line", "to": [cx + 60.0, top]}]}),
    );
    let pocket = ok(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Pocket",
        "origin": [cx + 400.0, top - 130.0], "width": 120, "height": 140}),
    );
    let pe: Vec<String> = (5..9).map(|k| id(&pocket, k)).collect();
    // Its sides and bottom (the top folded out of the seam: 10 mm).
    ok(
        &mut s,
        json!({"type": "seam.add",
        "a": {"edges": [pe[3], pe[0], pe[1]], "from_mm": 10, "to_mm": 390},
        "b": {"line": id(&line, 0)}}),
    );
    let place = s
        .project()
        .panel(front.panel.as_str().unwrap().parse().unwrap())
        .unwrap()
        .placement
        .unwrap();
    ok(
        &mut s,
        json!({"type": "panel.place", "panel": id(&pocket, 0),
        "angle_deg": place.angle_deg, "top_cm": place.top_cm - 15.0, "offset_mm": 40}),
    );
    s.project().check_integrity().unwrap();

    let project = s.project();
    let form = Form::of_project(project).unwrap().unwrap();
    let drape = build(project, Some(&form), DEFAULT_SPACING_MM).unwrap();
    let mut solver = CpuSolver::new(drape.scene);
    for _ in 0..180 {
        solver.step(1.0 / 60.0).unwrap();
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let stats = solver.stats();
    let high = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    eprintln!("stats {stats:?}, top {high:.3}");
    assert!(stats.max_seam_gap < 0.01, "seams open: {stats:?}");
    assert!(high > 1.0, "skirt slid down: top at {high}");
}

#[test]
fn holes_are_cut_and_filled_and_mesh_out() {
    let mut s = Session::new(Project::new("t"));
    let p = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 300, "height": 400}),
    );
    let panel = id(&p, 0);
    let hole = ok(
        &mut s,
        json!({"type": "panel.add_hole", "panel": panel,
        "start": [100, 150], "segments": [
            {"kind": "line", "to": [200, 150]}, {"kind": "line", "to": [200, 250]},
            {"kind": "line", "to": [100, 250]}]}),
    );
    let got = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(got["area_mm2"], 120_000.0 - 10_000.0);
    // Crossing the outline is refused.
    let err = run(
        &mut s,
        json!({"type": "panel.add_hole", "panel": panel,
        "start": [250, 10], "segments": [
            {"kind": "line", "to": [350, 10]}, {"kind": "line", "to": [350, 60]}]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.not_across");
    // The cloth leaves it out.
    ok(
        &mut s,
        json!({"type": "panel.place", "panel": panel, "angle_deg": 0, "top_cm": 104}),
    );
    let drape = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    let inside = drape.scene.triangles.iter().any(|t| {
        let c = t
            .vertices
            .iter()
            .map(|i| drape.scene.particles[*i as usize].rest)
            .fold(glam::Vec2::ZERO, |a, b| a + b)
            / 3.0;
        (0.101..0.199).contains(&c.x) && (0.151..0.249).contains(&c.y)
    });
    assert!(!inside);
    // A hole's edge can be sewn (a facing), and filled again.
    let edge = id(&hole, 4);
    ok(&mut s, json!({"type": "panel.remove_hole", "edge": edge}));
    let got = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(got["area_mm2"], 120_000.0);
    s.project().check_integrity().unwrap();
}

#[test]
fn pieces_join_and_cut_each_other() {
    let mut s = Session::new(Project::new("t"));
    let a = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 200, "height": 200}),
    );
    let b = ok(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [150, 50],
        "width": 200, "height": 100}),
    );
    ok(
        &mut s,
        json!({"type": "panel.boolean", "panel": id(&a, 0), "with": id(&b, 0),
        "op": "union"}),
    );
    let got = ok(&mut s, json!({"type": "panel.get", "panel": id(&a, 0)}));
    assert_eq!(got["area_mm2"], 40_000.0 + 15_000.0);
    assert_eq!(got["point_count"], 8);
    assert_eq!(s.project().panels().len(), 1);
    // A circle cut out of it: a hole, its curve fitted back.
    let c = ok(
        &mut s,
        json!({"type": "panel.create_ellipse", "centre": [80, 100],
        "radii": [30, 30]}),
    );
    ok(
        &mut s,
        json!({"type": "panel.boolean", "panel": id(&a, 0), "with": id(&c, 0),
        "op": "difference"}),
    );
    let got = ok(&mut s, json!({"type": "panel.get", "panel": id(&a, 0)}));
    let area = got["area_mm2"].as_f64().unwrap();
    let expected = 55_000.0 - std::f64::consts::PI * 900.0;
    assert!((area - expected).abs() < 20.0, "{area} vs {expected}");
    let panel = s.project().panel(id(&a, 0).parse().unwrap()).unwrap();
    assert_eq!(panel.holes.len(), 1);
    // A piece outside it leaves it whole: nothing to cut, refused.
    let far = ok(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [1000, 0],
        "width": 50, "height": 50}),
    );
    let err = run(
        &mut s,
        json!({"type": "panel.boolean", "panel": id(&a, 0),
        "with": id(&far, 0), "op": "union"}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.apart");
    s.project().check_integrity().unwrap();
}
