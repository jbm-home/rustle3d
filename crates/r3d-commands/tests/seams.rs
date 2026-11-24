#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

/// Two 100 × 200 rectangles; returns (session, front edges, back edges).
fn two_panels() -> (Session, Vec<String>, Vec<String>) {
    let mut s = Session::new(Project::new("t"));
    let edges = |out: &Value| -> Vec<String> {
        out["created"].as_array().unwrap()[5..9]
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    let a = run(
        &mut s,
        json!({"type": "panel.create_rect", "width": 100, "height": 200}),
    )
    .unwrap();
    let b = run(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [200, 0], "width": 100, "height": 200}),
    )
    .unwrap();
    (s, edges(&a), edges(&b))
}

#[test]
fn seams_list_lengths_and_protect_sewn_edges() {
    let (mut s, a, b) = two_panels();
    // Right side of A to left side of B.
    let out = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": a[1]}, "b": {"edge": b[3], "reversed": true}}),
    )
    .unwrap();
    let seam = out["created"][0].clone();
    let list = run(&mut s, json!({"type": "seam.list"})).unwrap();
    assert_eq!(list[0]["a"]["length_mm"], 200.0);
    assert_eq!(list[0]["mismatch_mm"], 0.0);

    let err = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": a[1]}, "b": {"edge": b[1]}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "sewn");
    let err = run(
        &mut s,
        json!({"type": "edge.split", "edge": a[1], "at_mm": 50}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "sewn");
    let err = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": a[0]}, "b": {"edge": a[0]}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.same_point");

    // Deleting a panel removes its seams.
    let panel_b = s
        .project()
        .edge(b[3].parse().unwrap())
        .unwrap()
        .panel
        .to_string();
    run(
        &mut s,
        json!({"type": "panel.delete", "panel": panel_b, "confirm": true}),
    )
    .unwrap();
    assert!(s.project().seams().is_empty());
    s.undo().unwrap();
    assert_eq!(
        run(&mut s, json!({"type": "seam.list"})).unwrap()[0]["id"],
        seam
    );
}

#[test]
fn colors_and_fabrics_are_validated() {
    let (mut s, _, _) = two_panels();
    let err = run(
        &mut s,
        json!({"type": "garment.set_color", "color": "blue"}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.format");
    let err = run(
        &mut s,
        json!({"type": "garment.set_fabric", "fabric": "leather"}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "malformed");
    run(
        &mut s,
        json!({"type": "garment.set_fabric", "fabric": "silk"}),
    )
    .unwrap();
    let g = s.project().garments().values().next().unwrap();
    assert_eq!(g.fabric, r3d_core::Fabric::Silk);
}

#[test]
fn seam_direction_is_chosen_when_left_out() {
    let (mut s, a, b) = two_panels();
    // Right side of A (going up) to left side of B (going down): reversed.
    let out = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": a[1]}, "b": {"edge": b[3]}}),
    )
    .unwrap();
    let id: r3d_core::SeamId = out["created"][0].as_str().unwrap().parse().unwrap();
    assert!(s.project().seams()[&id].b.reversed);
}

#[test]
fn an_edge_meets_its_mirror_image_point_to_mirrored_point() {
    // The back crotch of a trouser leg, sewn to the same curve on the
    // linked mirror copy: hip to hip, crotch point to crotch point. Both
    // edges run mostly sideways, one left and one right, so walking them
    // the same way on the pattern would sew the hip to the crotch point.
    let mut s = Session::new(Project::new("t"));
    let leg = run(
        &mut s,
        json!({"type": "panel.create_path", "start": [82, 0], "segments": [
            {"kind": "line", "to": [312, 0]},
            {"kind": "line", "to": [375, 780]},
            {"kind": "line", "to": [330, 1000]},
            {"kind": "line", "to": [140, 1020]},
            {"kind": "line", "to": [110, 810]},
            {"kind": "cubic", "c1": [95, 750], "c2": [60, 728], "to": [0, 728]},
            {"kind": "line", "to": [82, 0]}]}),
    )
    .unwrap();
    let panel = leg["created"][0].as_str().unwrap();
    let copy = run(&mut s, json!({"type": "panel.mirror", "panel": panel})).unwrap();
    let copy = copy["created"][0].as_str().unwrap();
    let crotch = |s: &mut Session, panel: &str, from: f64| -> (String, bool) {
        let p = run(s, json!({"type": "panel.get", "panel": panel})).unwrap();
        let e = p["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["curve"]["kind"] == "cubic")
            .unwrap();
        let starts_at_hip = (e["start"]["pos"][1].as_f64().unwrap() - from).abs() < 1e-9;
        (e["id"].as_str().unwrap().to_owned(), starts_at_hip)
    };
    let (a, a_from_hip) = crotch(&mut s, panel, 810.0);
    let (b, b_from_hip) = crotch(&mut s, copy, 810.0);
    assert!(a_from_hip && !b_from_hip);
    let out = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": a}, "b": {"edge": b}}),
    )
    .unwrap();
    let id: r3d_core::SeamId = out["created"][0].as_str().unwrap().parse().unwrap();
    assert!(
        s.project().seams()[&id].b.reversed,
        "hip sewn to the crotch point"
    );
}

#[test]
fn arrange_places_panels_around_in_seam_order() {
    let mut s = Session::new(Project::new("t"));
    common::a_line_skirt(&mut s);
    let place = |s: &Session, name: &str| {
        let (_, panel) = s
            .project()
            .panels()
            .iter()
            .find(|(_, p)| p.name == name)
            .unwrap();
        panel.placement.unwrap()
    };
    let names = ["Front", "Back left", "Back right"];
    run(&mut s, json!({"type": "garment.arrange"})).unwrap();
    // Front, then the back sewn to its right side, then the mirrored back,
    // side by side by width: the front is half the hem, each back a
    // quarter, so the centre backs meet at 180° at the hem.
    assert_eq!(names.map(|n| place(&s, n).angle_deg), [0.0, 135.0, 225.0]);
    assert!((place(&s, "Front").top_cm - 110.0).abs() < 1e-9);
}

#[test]
fn a_range_ending_at_the_length_as_shown_reaches_the_end() {
    // A curve's length is measured, so after edge.set_length 222 it may be
    // 221.9996: `to_mm` 222, the length as shown, was refused.
    let (mut s, a, b) = two_panels();
    run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": a[1], "to_mm": 200.03},
            "b": {"edge": b[3], "reversed": true}}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "seam.list"})).unwrap();
    assert_eq!(list[0]["a"]["length_mm"], 200.0);
    // Clearly past the end is still refused.
    let err = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": a[0], "to_mm": 100.2},
            "b": {"edge": b[0], "reversed": true}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.range");
}
