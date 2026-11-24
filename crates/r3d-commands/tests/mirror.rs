#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, step: Value) -> Result<Value, CommandError> {
    s.run_step(&Step::from_value(step)?)
}

fn ok(s: &mut Session, step: Value) -> Value {
    run(s, step.clone()).unwrap_or_else(|e| panic!("{step}: {e}"))
}

/// A panel 100 × 200 at the origin with a notch, mirrored to the right.
fn setup() -> (Session, String, String) {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 100, "height": 200}),
    );
    let panel = out["created"][0].as_str().unwrap().to_owned();
    let first_edge = out["created"][5].as_str().unwrap().to_owned();
    ok(
        &mut s,
        json!({"type": "notch.add", "edge": first_edge, "offset_mm": 30}),
    );
    let out = ok(&mut s, json!({"type": "panel.mirror", "panel": panel}));
    let copy = out["created"][0].as_str().unwrap().to_owned();
    (s, panel, copy)
}

fn get(s: &mut Session, panel: &str) -> Value {
    ok(s, json!({"type": "panel.get", "panel": panel}))
}

#[test]
fn copy_is_reflected_counter_clockwise_and_carries_notches() {
    let (mut s, _, copy) = setup();
    let c = get(&mut s, &copy);
    // Default axis: vertical at x = 100 + 25 → copy spans x ∈ [150, 250].
    assert_eq!(c["bbox"]["min"], json!([150.0, 0.0]));
    assert_eq!(c["bbox"]["max"], json!([250.0, 200.0]));
    assert_eq!(c["area_mm2"], 20000.0);
    assert_eq!(c["is_simple"], true);
    assert_eq!(c["grain_angle_deg"], 90.0);
    // Source notch 30 mm along the bottom edge from (0,0) → copy at x = 220.
    assert_eq!(c["notches"][0]["pos"], json!([220.0, 0.0]));
    s.project().check_integrity().unwrap();
}

#[test]
fn copy_follows_source_edits_and_topology_changes() {
    let (mut s, panel, copy) = setup();
    let src = get(&mut s, &panel);
    let corner = src["edges"][1]["start"]["id"].clone(); // (100, 0)
    ok(
        &mut s,
        json!({"type": "point.move", "point": corner, "to": [120, 0]}),
    );
    assert_eq!(get(&mut s, &copy)["bbox"]["min"], json!([130.0, 0.0]));

    let edge = src["edges"][2]["id"].clone();
    ok(
        &mut s,
        json!({"type": "edge.split", "edge": edge, "at_mm": 50}),
    );
    let c = get(&mut s, &copy);
    assert_eq!(c["point_count"], 5);
    assert_eq!(c["is_simple"], true);

    ok(
        &mut s,
        json!({"type": "panel.set_seam_allowance", "panel": panel, "mm": 12}),
    );
    assert_eq!(get(&mut s, &copy)["seam_allowance_mm"], 12.0);
    // Set on the copy, it goes to its piece (a script setting every
    // piece's allowance runs through).
    ok(
        &mut s,
        json!({"type": "panel.set_seam_allowance", "panel": copy, "mm": 8}),
    );
    assert_eq!(get(&mut s, &panel)["seam_allowance_mm"], 8.0);
    assert_eq!(get(&mut s, &copy)["seam_allowance_mm"], 8.0);
    s.project().check_integrity().unwrap();
}

#[test]
fn unchanged_copies_record_no_changes() {
    let (mut s, panel, _) = setup();
    let undo_steps = s.history().undo_len();
    // A rename touches only the source panel record.
    ok(
        &mut s,
        json!({"type": "panel.rename", "panel": panel, "name": "Front"}),
    );
    assert_eq!(s.history().undo_len(), undo_steps + 1);
    s.undo().unwrap();
    s.redo().unwrap();
    s.project().check_integrity().unwrap();
}

#[test]
fn copy_cannot_be_edited_directly_but_can_be_moved() {
    let (mut s, _, copy) = setup();
    let c = get(&mut s, &copy);
    let point = c["edges"][0]["start"]["id"].clone();
    let err = run(
        &mut s,
        json!({"type": "point.move", "point": point, "to": [0, 0]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "linked_mirror");
    let edge = c["edges"][0]["id"].clone();
    let err = run(
        &mut s,
        json!({"type": "notch.add", "edge": edge, "offset_mm": 1}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "linked_mirror");

    ok(
        &mut s,
        json!({"type": "panel.translate", "panel": copy, "delta": [0, 300]}),
    );
    let c = get(&mut s, &copy);
    assert_eq!(c["bbox"]["min"], json!([150.0, 300.0]));
    assert_eq!(c["mirror_of"]["offset"], json!([0.0, 300.0]));
}

#[test]
fn unlink_and_source_deletion_make_copies_independent() {
    let (mut s, panel, copy) = setup();
    ok(
        &mut s,
        json!({"type": "panel.delete", "panel": panel, "confirm": true}),
    );
    let c = get(&mut s, &copy);
    assert!(c.get("mirror_of").is_none());
    assert_eq!(c["area_mm2"], 20000.0);
    let err = run(&mut s, json!({"type": "panel.unlink", "panel": copy})).unwrap_err();
    assert_eq!(err.message().code, "invalid.not_linked");
    s.undo().unwrap();
    assert!(get(&mut s, &copy).get("mirror_of").is_some());
}

#[test]
fn split_and_remove_keep_notch_positions() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 400, "height": 200}),
    );
    let edge = out["created"][5].clone(); // bottom edge (0,0) → (400,0)
    ok(
        &mut s,
        json!({"type": "notch.add", "edge": edge, "offset_mm": 300}),
    );
    let split = ok(
        &mut s,
        json!({"type": "edge.split", "edge": edge, "at_mm": 100}),
    );
    let panel = out["created"][0].as_str().unwrap().to_owned();
    let notch_pos = |s: &mut Session| get(s, &panel)["notches"][0]["pos"].clone();
    assert_eq!(notch_pos(&mut s), json!([300.0, 0.0]));
    ok(
        &mut s,
        json!({"type": "point.remove", "point": split["created"][0]}),
    );
    assert_eq!(notch_pos(&mut s), json!([300.0, 0.0]));
    assert_eq!(get(&mut s, &panel)["point_count"], 4);
}

#[test]
fn split_rejects_positions_outside_the_edge_and_removal_keeps_three_points() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_polygon", "points": [[0, 0], [100, 0], [0, 100]]}),
    );
    let err = run(
        &mut s,
        json!({"type": "edge.split", "edge": out["created"][4], "at_mm": 100}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.outside_edge");
    let err = run(
        &mut s,
        json!({"type": "point.remove", "point": out["created"][1]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.too_few_points");
}

#[test]
fn contour_length_walks_counter_clockwise() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 100, "height": 200}),
    );
    let (a, c) = (out["created"][1].clone(), out["created"][3].clone());
    let r = ok(
        &mut s,
        json!({"type": "measure.contour_length", "from": a, "to": c}),
    );
    assert_eq!(r["length_mm"], 300.0);
    let r = ok(
        &mut s,
        json!({"type": "measure.contour_length", "from": c, "to": a}),
    );
    assert_eq!(r["length_mm"], 300.0);
    assert_eq!(r["edges"].as_array().unwrap().len(), 2);
}
