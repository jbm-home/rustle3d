//! Buttons and zippers in the document (spec 26): validation, listing,
//! what follows their pieces, sizes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

/// Two overlapping fronts, 200 × 400 mm, the right one 30 mm left of
/// the left one's edge: ids of the panels and their edges.
fn fronts(s: &mut Session) -> (Value, Value, Vec<Value>, Vec<Value>) {
    let left = run(
        s,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 200, "height": 400}),
    )
    .unwrap();
    let right = run(
        s,
        json!({"type": "panel.create_rect", "origin": [300, 0], "width": 200, "height": 400}),
    )
    .unwrap();
    let edges = |r: &Value| r["created"].as_array().unwrap()[5..9].to_vec();
    (
        left["created"][0].clone(),
        right["created"][0].clone(),
        edges(&left),
        edges(&right),
    )
}

#[test]
fn buttons_are_checked_listed_and_follow_their_pieces() {
    let mut s = Session::new(Project::new("t"));
    let (left, right, _, _) = fronts(&mut s);
    let outside = run(
        &mut s,
        json!({"type": "button.add", "panel": left, "at": [250, 100],
            "hole_panel": right, "hole_at": [310, 100]}),
    )
    .unwrap_err();
    assert_eq!(outside.message().code, "invalid.outside_piece");
    let big = run(
        &mut s,
        json!({"type": "button.add", "panel": left, "at": [190, 100], "diameter_mm": 200,
            "hole_panel": right, "hole_at": [310, 100]}),
    )
    .unwrap_err();
    assert_eq!(big.message().code, "invalid.range");
    let out = run(
        &mut s,
        json!({"type": "button.add", "panel": left, "at": [185, 100],
            "hole_panel": right, "hole_at": [315, 100]}),
    )
    .unwrap();
    let id = out["created"][0].clone();
    let list = run(&mut s, json!({"type": "trim.list"})).unwrap();
    let b = &list["buttons"][0];
    assert_eq!(b["id"], id);
    assert_eq!(b["diameter_mm"], json!(11.0));
    assert_eq!(b["closed"], json!(true));
    // The slit runs inward, towards the right front's middle: 11 + 3 mm.
    assert_eq!(b["slit"], json!([[315.0, 100.0], [329.0, 100.0]]));

    // Moving a piece moves what is on it.
    run(
        &mut s,
        json!({"type": "panel.translate", "panel": right, "delta": [100, 0]}),
    )
    .unwrap();
    let b = s.project().buttons().values().next().unwrap().clone();
    assert_eq!(b.hole.at.x, 415.0);
    assert_eq!(b.at.x, 185.0);

    run(
        &mut s,
        json!({"type": "button.set_closed", "buttons": [id], "closed": false}),
    )
    .unwrap();
    assert!(!s.project().buttons().values().next().unwrap().closed);

    // A piece deleted takes its buttons with it, in the same undo step.
    run(
        &mut s,
        json!({"type": "panel.delete", "panel": right, "confirm": true}),
    )
    .unwrap();
    assert!(s.project().buttons().is_empty());
    s.undo().unwrap();
    assert_eq!(s.project().buttons().len(), 1);
}

#[test]
fn zippers_join_free_stretches_and_open_from_the_top() {
    let mut s = Session::new(Project::new("t"));
    let (_, _, le, re) = fronts(&mut s);
    // Edges: 0 bottom, 1 right side, 2 top, 3 left side (going down).
    let zip = run(
        &mut s,
        json!({"type": "zipper.add",
            "a": {"edge": le[1], "to_mm": 180},
            "b": {"edge": re[3], "from_mm": 220, "reversed": true}}),
    )
    .unwrap();
    let id = zip["created"][0].clone();
    // The zipped stretch cannot be sewn.
    let err = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": le[1]}, "b": {"edge": re[3]}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "sewn");
    // The rest of the edges can.
    run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": le[1], "from_mm": 180},
            "b": {"edge": re[3], "to_mm": 220, "reversed": true}}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "trim.list"})).unwrap();
    assert_eq!(list["zippers"][0]["length_mm"], json!(180.0));
    assert_eq!(list["zippers"][0]["closed"], json!(1.0));
    run(
        &mut s,
        json!({"type": "zipper.set_open", "zipper": id, "open_mm": 45}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "trim.list"})).unwrap();
    assert_eq!(list["zippers"][0]["closed"], json!(0.75));
    let err = run(
        &mut s,
        json!({"type": "zipper.set_open", "zipper": id, "open_mm": -1}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.negative");
}

#[test]
fn buttons_follow_the_grading() {
    let mut s = Session::new(Project::new("t"));
    let body =
        json!({"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98});
    run(&mut s, json!({"type": "avatar.set", "body": body})).unwrap();
    let (left, right, _, _) = fronts(&mut s);
    run(
        &mut s,
        json!({"type": "button.add", "panel": left, "at": [185, 200],
            "hole_panel": right, "hole_at": [315, 200]}),
    )
    .unwrap();
    run(
        &mut s,
        json!({"type": "grading.set_sizes", "sizes": [
            {"name": "S", "height_cm": 0, "bust_cm": -4, "waist_cm": -4, "hips_cm": -4},
            {"name": "M", "height_cm": 0, "bust_cm": 0, "waist_cm": 0, "hips_cm": 0}],
            "base": "M"}),
    )
    .unwrap();
    // Every point of the left front 10 mm further right in size S.
    let p = s.project().clone();
    let left_id = p.panels().keys().next().copied().unwrap();
    for (id, pt) in p.points() {
        if pt.panel == left_id {
            run(
                &mut s,
                json!({"type": "point.set_grade", "point": id.to_string(),
                    "offsets": {"S": [10, 0]}}),
            )
            .unwrap();
        }
    }
    let small = s.project().at_size(0);
    let b = small.buttons().values().next().unwrap();
    assert!((b.at.x - 195.0).abs() < 1e-9, "{}", b.at.x);
    assert!((b.hole.at.x - 315.0).abs() < 1e-9);
}
