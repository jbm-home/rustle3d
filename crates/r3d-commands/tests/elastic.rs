//! Elastic and shirring (spec 31): validation, listing, what follows the
//! pieces, sizes, and a sleeve's hem gathered in the drape.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::drape::{prepare, simulate};
use r3d_commands::progress::Progress;
use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

/// A 400 × 300 mm rectangle: its panel and edges (bottom, right, top,
/// left).
fn rect(s: &mut Session) -> (Value, Vec<Value>) {
    let r = run(
        s,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 400, "height": 300}),
    )
    .unwrap();
    let created = r["created"].as_array().unwrap();
    (created[0].clone(), created[5..9].to_vec())
}

#[test]
fn elastics_are_checked_listed_and_follow_their_sides() {
    let mut s = Session::new(Project::new("t"));
    let (panel, edges) = rect(&mut s);
    let bad = run(
        &mut s,
        json!({"type": "elastic.add", "side": {"edge": edges[0]}, "ratio": 1.5}),
    )
    .unwrap_err();
    assert_eq!(bad.message().code, "invalid.range");
    let both = run(
        &mut s,
        json!({"type": "elastic.add", "side": {"edge": edges[0]}, "ratio": 0.5, "relaxed_mm": 100}),
    )
    .unwrap_err();
    assert_eq!(both.message().code, "invalid.format");
    // A relaxed length becomes the ratio.
    let id = run(
        &mut s,
        json!({"type": "elastic.add", "side": {"edge": edges[0]}, "relaxed_mm": 240}),
    )
    .unwrap()["created"][0]
        .clone();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    let e = &list["elastics"][0];
    assert_eq!(e["id"], id);
    assert_eq!(e["ratio"], json!(0.6));
    assert_eq!(e["width_mm"], json!(10.0));
    assert_eq!(e["relaxed_mm"], json!(240.0));
    assert_eq!(e["panel"], panel);
    // One elastic per stretch.
    let twice = run(
        &mut s,
        json!({"type": "elastic.add", "side": {"edge": edges[0], "from_mm": 100}}),
    )
    .unwrap_err();
    assert_eq!(twice.message().code, "invalid.has_elastic");
    // The ratio stays when the side changes length.
    run(
        &mut s,
        json!({"type": "edge.set_length", "edge": edges[0], "length_mm": 500}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    assert_eq!(list["elastics"][0]["relaxed_mm"], json!(300.0));
    // Deleting the piece takes its elastic with it, in one undo step.
    run(
        &mut s,
        json!({"type": "panel.delete", "panel": panel, "confirm": true}),
    )
    .unwrap();
    assert!(s.project().details().is_empty());
    s.undo().unwrap();
    assert_eq!(s.project().details().len(), 1);
}

#[test]
fn shirring_adds_rows_of_elastic_across_the_piece() {
    let mut s = Session::new(Project::new("t"));
    let (panel, edges) = rect(&mut s);
    let out = run(
        &mut s,
        json!({"type": "elastic.shirr", "panel": panel, "edge": edges[2], "rows": 4,
            "first_mm": 20, "gap_mm": 12, "ratio": 0.7}),
    )
    .unwrap();
    // A line and its elastic per row.
    assert_eq!(out["created"].as_array().unwrap().len(), 8);
    let p = s.project();
    assert_eq!(p.internal_lines().len(), 4);
    let mut ys: Vec<f64> = p.internal_lines().values().map(|l| l.start.y).collect();
    ys.sort_by(f64::total_cmp);
    // The top edge runs right to left: its inside is below it.
    assert_eq!(ys, vec![244.0, 256.0, 268.0, 280.0]);
    for line in p.internal_lines().values() {
        let r3d_core::PathSegment::Line { to } = line.segments[0] else {
            panic!("a straight row");
        };
        assert!((line.start.distance(to) - 398.0).abs() < 1e-6);
    }
    assert!(p.elastics().all(|(_, e)| (e.ratio - 0.7).abs() < 1e-12));
    // One undo removes them all; removing a row's line removes its
    // elastic.
    s.undo().unwrap();
    assert!(s.project().details().is_empty() && s.project().internal_lines().is_empty());
    s.redo().unwrap();
    let line = *s.project().internal_lines().keys().next().unwrap();
    run(
        &mut s,
        json!({"type": "internal_line.remove", "line": line.to_string()}),
    )
    .unwrap();
    assert_eq!(s.project().details().len(), 3);
}

/// The shirring rows' lines, bottom to top, as (start, end).
fn rows(s: &Session) -> Vec<(r3d_geom::DVec2, r3d_geom::DVec2)> {
    let mut rows: Vec<_> = s
        .project()
        .internal_lines()
        .values()
        .map(|l| {
            let r3d_core::PathSegment::Line { to } = l.segments[0] else {
                panic!("a straight row");
            };
            (l.start, to)
        })
        .collect();
    rows.sort_by(|a, b| a.0.y.total_cmp(&b.0.y));
    rows
}

#[test]
fn shirring_rows_follow_the_outline() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    let r = run(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 400, "height": 300}),
    )
    .unwrap();
    let created = r["created"].as_array().unwrap();
    let (panel, top_right, top_left, top) = (&created[0], &created[3], &created[4], &created[7]);
    run(
        &mut s,
        json!({"type": "elastic.shirr", "panel": panel, "edge": top, "rows": 4,
            "first_mm": 20, "gap_mm": 12}),
    )
    .unwrap();
    let before = rows(&s);
    // The right side slants in: each row ends 1 mm short of it, at its
    // own height.
    run(
        &mut s,
        json!({"type": "point.move", "point": top_right, "to": [300, 300]}),
    )
    .unwrap();
    let after = rows(&s);
    assert_eq!(after.len(), 4);
    for ((a0, a1), (b0, _)) in after.iter().zip(&before) {
        assert_eq!(a0.y, b0.y);
        assert_eq!(a1.y, b0.y);
        // Along the top edge: right to left.
        assert!((a1.x - 1.0).abs() < 1e-6, "{a1}");
        let side = 400.0 - 100.0 * a0.y / 300.0;
        assert!((a0.x - (side - 1.0)).abs() < 1e-6, "{a0} vs {side}");
    }
    // The piece meshes with them.
    run(
        &mut s,
        json!({"type": "panel.place", "panel": panel, "angle_deg": 0, "top_cm": 120}),
    )
    .unwrap();
    prepare(s.project(), None).unwrap();
    // One undo step: the move and the rows.
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!(rows(&s), before);
    s.redo().unwrap();
    assert_eq!(rows(&s), after);
    // The top comes down past three rows: they go, with their elastics.
    run(
        &mut s,
        json!({"type": "point.move", "point": top_left, "to": [0, 250]}),
    )
    .unwrap();
    assert_eq!(rows(&s).len(), 4);
    run(
        &mut s,
        json!({"type": "point.move", "point": top_right, "to": [300, 250]}),
    )
    .unwrap();
    assert_eq!(rows(&s).len(), 1);
    assert_eq!(s.project().details().len(), 1);
    s.project().check_integrity().unwrap();
    s.undo().unwrap();
    assert_eq!(rows(&s).len(), 4);
    assert_eq!(s.project().details().len(), 4);
}

#[test]
fn removing_a_shirring_row_removes_its_line() {
    let mut s = Session::new(Project::new("t"));
    let (panel, edges) = rect(&mut s);
    let out = run(
        &mut s,
        json!({"type": "elastic.shirr", "panel": panel, "edge": edges[2], "rows": 3}),
    )
    .unwrap();
    // Created: line, elastic, line, elastic…
    let created = out["created"].as_array().unwrap();
    run(
        &mut s,
        json!({"type": "elastic.remove", "elastic": created[1]}),
    )
    .unwrap();
    assert_eq!(s.project().internal_lines().len(), 2);
    assert_eq!(s.project().details().len(), 2);
    s.undo().unwrap();
    assert_eq!(s.project().internal_lines().len(), 3);
    // A row topstitched too keeps its line.
    run(
        &mut s,
        json!({"type": "topstitch.add", "side": {"line": created[2]}, "offset_mm": 0}),
    )
    .unwrap();
    run(
        &mut s,
        json!({"type": "elastic.remove", "elastic": created[3]}),
    )
    .unwrap();
    assert_eq!(s.project().internal_lines().len(), 3);
    // An elastic on a line drawn by hand leaves the line.
    let line = run(
        &mut s,
        json!({"type": "internal_line.add", "panel": panel, "start": [50, 50],
            "segments": [{"kind": "line", "to": [350, 50]}]}),
    )
    .unwrap()["created"][0]
        .clone();
    let elastic = run(
        &mut s,
        json!({"type": "elastic.add", "side": {"line": line}}),
    )
    .unwrap()["created"][0]
        .clone();
    run(
        &mut s,
        json!({"type": "elastic.remove", "elastic": elastic}),
    )
    .unwrap();
    assert_eq!(s.project().internal_lines().len(), 4);
}

#[test]
fn a_line_left_outside_its_piece_is_named_by_the_drape() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    let r = run(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 400, "height": 300}),
    )
    .unwrap();
    let created = r["created"].as_array().unwrap();
    let (panel, top_right) = (&created[0], &created[3]);
    let line = run(
        &mut s,
        json!({"type": "internal_line.add", "panel": panel, "start": [20, 250],
            "segments": [{"kind": "line", "to": [380, 250]}]}),
    )
    .unwrap()["created"][0]
        .clone();
    run(
        &mut s,
        json!({"type": "elastic.add", "side": {"line": line}}),
    )
    .unwrap();
    run(
        &mut s,
        json!({"type": "point.move", "point": top_right, "to": [300, 300]}),
    )
    .unwrap();
    run(
        &mut s,
        json!({"type": "panel.place", "panel": panel, "angle_deg": 0, "top_cm": 120}),
    )
    .unwrap();
    let err = prepare(s.project(), None).unwrap_err();
    let m = err.message();
    assert_eq!(m.code, "line_outside");
    assert_eq!(m.args["line"], line.as_str().unwrap());
}

#[test]
fn elastic_lengths_follow_the_sizes() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    common::a_line_skirt(&mut s);
    run(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "40"}),
    )
    .unwrap();
    let front = common::piece(&s, "Front");
    run(
        &mut s,
        json!({"type": "elastic.add", "side": {"edge": front.edges[2]}, "ratio": 0.8}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    let sizes = list["elastics"][0]["sizes"].as_array().unwrap();
    assert_eq!(sizes.len(), 5);
    let lens: Vec<f64> = sizes
        .iter()
        .map(|v| v["relaxed_mm"].as_f64().unwrap())
        .collect();
    assert!(lens[0] < lens[1] && lens[1] < lens[2], "{lens:?}");
}

/// A T-shirt whose left sleeve, `sleeve_cm` long, has an elastic of
/// ratio 0.6 at its hem, draped for 2.5 s: the hem's drawn length, its
/// length on the cloth and the elastic's relaxed length (mm), and the
/// hem's nearest distance to the body (mm).
fn gathered_hem(sleeve_cm: f64) -> (f64, f64, f64, f64) {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": "t_shirt", "sizes": {"sleeve_cm": sleeve_cm}}),
    )
    .unwrap();
    let sleeve = common::piece(&s, "Sleeve left");
    let hem: r3d_core::EdgeId = sleeve.edges[0].as_str().unwrap().parse().unwrap();
    let hem_mm = s.project().edge_length(hem).unwrap();
    run(
        &mut s,
        json!({"type": "elastic.add", "side": {"edge": sleeve.edges[0]}, "ratio": 0.6}),
    )
    .unwrap();
    let out = simulate(s.project(), 150, None, &Progress::default()).unwrap();
    assert!(out.positions.iter().all(|p| p.is_finite()));
    assert!(out.stats.max_seam_gap < 0.01, "{:?}", out.stats);
    let lengths = out.drape.elastic_lengths(&out.positions);
    assert_eq!(lengths.len(), 1);
    let (_, on, rest) = lengths[0];
    let body = &out.form.as_ref().unwrap().positions;
    let touch = out.drape.scene.elastics[0]
        .particles
        .iter()
        .map(|i| {
            let q = out.positions[*i as usize];
            body.iter().map(|b| b.distance(q)).fold(f32::MAX, f32::min)
        })
        .fold(f32::MAX, f32::min);
    let mm = |m: f32| f64::from(m) * 1000.0;
    (hem_mm, mm(on), mm(rest), mm(touch))
}

#[test]
fn a_sleeve_hem_gathers_to_its_elastic() {
    // Past the hand, nothing holds the hem open: it closes to the
    // elastic's length.
    let (hem, on, rest, _) = gathered_hem(64.0);
    assert!((rest - hem * 0.6).abs() < 1.0, "{rest} vs {hem}");
    assert!(
        (on - rest).abs() < rest * 0.05,
        "the hem is {on} mm around, {rest} wanted ({hem} drawn)"
    );
    // On the forearm, thicker than the elastic, it stays stretched on it.
    let (_, on, rest, touch) = gathered_hem(40.0);
    assert!(on > rest * 1.05 && on < hem * 0.8, "{on} mm around");
    assert!(touch < 5.0, "the hem does not hold the arm: {touch} mm off");
}
