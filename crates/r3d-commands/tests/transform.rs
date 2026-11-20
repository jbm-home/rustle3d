//! Turning, scaling and moving a whole piece on the board: everything it
//! owns follows, and its constraints too when they can.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Constraint, Project};
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

fn pos(s: &Session, point: &str) -> [f64; 2] {
    let p = s.project().point(point.parse().unwrap()).unwrap().pos;
    [p.x, p.y]
}

fn close(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

/// A 300 × 400 mm rectangle from the origin: the panel, its four corners
/// (counter-clockwise from the origin) and its four edges (the first one
/// along the bottom).
fn rect(s: &mut Session) -> (String, Vec<String>, Vec<String>) {
    let out = ok(
        s,
        json!({"type": "panel.create_rect", "name": "Back", "origin": [0, 0],
        "width": 300, "height": 400}),
    );
    (
        id(&out, 0),
        (1..5).map(|k| id(&out, k)).collect(),
        (5..9).map(|k| id(&out, k)).collect(),
    )
}

#[test]
fn a_quarter_turn_turns_the_piece_about_its_centre_with_what_it_owns() {
    let mut s = Session::new(Project::new("t"));
    let (panel, points, _) = rect(&mut s);
    ok(
        &mut s,
        json!({"type": "internal_line.add", "panel": panel, "start": [150, 0],
        "segments": [{"kind": "line", "to": [150, 400]}]}),
    );
    ok(
        &mut s,
        json!({"type": "panel.rotate", "panel": panel, "angle_deg": 90}),
    );
    // About (150, 200): (0, 0) goes to (350, 50).
    assert!(
        close(pos(&s, &points[0]), [350.0, 50.0]),
        "{:?}",
        pos(&s, &points[0])
    );
    assert!(close(pos(&s, &points[2]), [-50.0, 350.0]));
    let p = s.project().panel(panel.parse().unwrap()).unwrap();
    assert!((p.grain_angle_deg - 180.0).abs() < 1e-9);
    let line = s.project().internal_lines().values().next().unwrap();
    assert!(close([line.start.x, line.start.y], [350.0, 200.0]));
    s.undo().unwrap();
    assert!(close(pos(&s, &points[0]), [0.0, 0.0]));
    // A quarter turn back from vertical: the grainline at 0, not 360.
    ok(
        &mut s,
        json!({"type": "panel.rotate", "panel": panel, "angle_deg": -90}),
    );
    let p = s.project().panel(panel.parse().unwrap()).unwrap();
    assert!(p.grain_angle_deg.abs() < 1e-9, "{}", p.grain_angle_deg);
}

#[test]
fn a_quarter_turn_swaps_horizontal_and_vertical_constraints() {
    let mut s = Session::new(Project::new("t"));
    let (panel, _, edges) = rect(&mut s);
    ok(
        &mut s,
        json!({"type": "constraint.add",
        "constraint": {"kind": "horizontal", "edge": edges[0]}}),
    );
    ok(
        &mut s,
        json!({"type": "panel.rotate", "panel": panel, "angle_deg": -90}),
    );
    let c = s.project().constraints().values().next().unwrap();
    assert!(matches!(c, Constraint::Vertical { .. }), "{c:?}");
    // Half a turn keeps it; any other angle cannot.
    ok(
        &mut s,
        json!({"type": "panel.rotate", "panel": panel, "angle_deg": 180}),
    );
    let refused = run(
        &mut s,
        json!({"type": "panel.rotate", "panel": panel, "angle_deg": 30}),
    );
    assert!(refused.is_err());
}

#[test]
fn scaling_scales_the_piece_and_its_dimensions() {
    let mut s = Session::new(Project::new("t"));
    let (panel, points, edges) = rect(&mut s);
    ok(
        &mut s,
        json!({"type": "constraint.add", "measure": true,
        "constraint": {"kind": "length", "edge": edges[0], "mm": 1}}),
    );
    ok(
        &mut s,
        json!({"type": "panel.scale", "panel": panel, "factor": [2, 2],
        "centre": [0, 0]}),
    );
    assert!(close(pos(&s, &points[2]), [600.0, 800.0]));
    let c = s.project().constraints().values().next().unwrap();
    assert!((c.value().unwrap() - 600.0).abs() < 1e-6);
    // Across only: a length could no longer hold.
    let refused = run(
        &mut s,
        json!({"type": "panel.scale", "panel": panel, "factor": [1.5, 1]}),
    );
    assert!(refused.is_err());
    let refused = run(
        &mut s,
        json!({"type": "panel.scale", "panel": panel, "factor": [0, 1]}),
    );
    assert!(refused.is_err());
}

#[test]
fn scaling_one_way_stretches_the_piece_that_way() {
    let mut s = Session::new(Project::new("t"));
    let (panel, points, _) = rect(&mut s);
    ok(
        &mut s,
        json!({"type": "panel.scale", "panel": panel, "factor": [1.1, 1]}),
    );
    // About (150, 200): x from -15 to 315, y unchanged.
    assert!(close(pos(&s, &points[0]), [-15.0, 0.0]));
    assert!(close(pos(&s, &points[2]), [315.0, 400.0]));
}

#[test]
fn a_hole_moves_turns_and_scales_with_its_piece() {
    let mut s = Session::new(Project::new("t"));
    let (panel, _, _) = rect(&mut s);
    ok(
        &mut s,
        json!({"type": "panel.add_hole", "panel": panel, "start": [100, 150],
        "segments": [{"kind": "line", "to": [200, 150]},
            {"kind": "line", "to": [200, 250]}, {"kind": "line", "to": [100, 250]}]}),
    );
    let hole_point = |s: &Session| {
        let p = s.project().panel(panel.parse().unwrap()).unwrap();
        let e = s.project().edge(p.holes[0][0]).unwrap();
        let pt = s.project().point(e.start).unwrap().pos;
        [pt.x, pt.y]
    };
    let before = hole_point(&s);
    ok(
        &mut s,
        json!({"type": "panel.translate", "panel": panel, "delta": [1000, 0]}),
    );
    assert!(close(hole_point(&s), [before[0] + 1000.0, before[1]]));
    ok(
        &mut s,
        json!({"type": "panel.rotate", "panel": panel, "angle_deg": 180}),
    );
    // About (1150, 200).
    assert!(close(
        hole_point(&s),
        [2300.0 - before[0] - 1000.0, 400.0 - before[1]]
    ));
}

#[test]
fn a_linked_copy_is_turned_through_its_source() {
    let mut s = Session::new(Project::new("t"));
    let (panel, _, _) = rect(&mut s);
    let copy = id(
        &ok(&mut s, json!({"type": "panel.mirror", "panel": panel})),
        0,
    );
    let refused = run(
        &mut s,
        json!({"type": "panel.rotate", "panel": copy, "angle_deg": 90}),
    );
    assert!(refused.is_err());
    ok(
        &mut s,
        json!({"type": "panel.rotate", "panel": panel, "angle_deg": 90}),
    );
}
