#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{EdgeId, Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

fn length(s: &Session, edge: &Value) -> f64 {
    let id: EdgeId = serde_json::from_value(edge.clone()).unwrap();
    s.project().edge_length(id).unwrap()
}

/// The largest length difference between the two sides of a seam (mm).
fn worst_mismatch(s: &mut Session) -> f64 {
    let seams = run(s, json!({"type": "seam.list"})).unwrap();
    seams
        .as_array()
        .unwrap()
        .iter()
        .map(|seam| seam["mismatch_mm"].as_f64().unwrap())
        .fold(0.0, f64::max)
}

#[test]
fn a_side_seam_gets_its_length_and_every_seam_stays_matched() {
    let mut s = Session::new(Project::new("t"));
    let front = common::a_line_skirt(&mut s);
    let side = front.edges[1].clone();
    let before = length(&s, &side);
    let mismatch = worst_mismatch(&mut s);

    run(
        &mut s,
        json!({"type": "edge.set_length", "edge": side, "length_mm": before + 50.0}),
    )
    .unwrap();
    assert!((length(&s, &side) - (before + 50.0)).abs() < 1e-6);
    // The back's side, the other side seam and the mirrored back follow.
    assert!(worst_mismatch(&mut s) < mismatch + 0.5);

    s.undo().unwrap();
    assert!((length(&s, &side) - before).abs() < 1e-9);
}

#[test]
fn a_hem_widens_about_its_middle() {
    let mut s = Session::new(Project::new("t"));
    let front = common::a_line_skirt(&mut s);
    let hem = front.edges[0].clone();
    let before = length(&s, &hem);

    run(
        &mut s,
        json!({"type": "edge.set_length", "edge": hem, "length_mm": before + 40.0}),
    )
    .unwrap();
    assert!((length(&s, &hem) - (before + 40.0)).abs() < 1e-6);

    for bad in [0.0, -10.0] {
        let err = run(
            &mut s,
            json!({"type": "edge.set_length", "edge": hem, "length_mm": bad}),
        )
        .unwrap_err();
        assert_eq!(err.message().code, "invalid.not_positive");
    }
}

#[test]
fn dragging_an_edge_is_one_undo_step() {
    let mut s = Session::new(Project::new("t"));
    let front = common::a_line_skirt(&mut s);
    let hem = front.edges[0].clone();
    let start = s.project().clone();
    // A drag runs one move per pointer event: they merge.
    for _ in 0..5 {
        run(
            &mut s,
            json!({"type": "edge.move", "edge": hem, "by": [0, -4]}),
        )
        .unwrap();
    }
    let id: EdgeId = serde_json::from_value(hem.clone()).unwrap();
    let edge = s.project().edge(id).unwrap().clone();
    let moved = s.project().point(edge.start).unwrap().pos;
    let was = start.point(edge.start).unwrap().pos;
    assert!((moved - was - r3d_geom::DVec2::new(0.0, -20.0)).length() < 1e-9);
    s.undo().unwrap();
    assert_eq!(s.project(), &start);
}
