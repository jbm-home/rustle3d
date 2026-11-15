#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

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
fn arrange_places_panels_around_in_seam_order() {
    let mut s = Session::new(Project::new("t"));
    let script =
        r3d_commands::Script::parse(r3d_commands::templates::get("a-line-skirt").unwrap()).unwrap();
    let results = script.run(|step| s.run_step(step)).unwrap();
    let place = |s: &Session, k: usize| {
        let id: r3d_core::PanelId = results[k]["created"][0].as_str().unwrap().parse().unwrap();
        s.project().panel(id).unwrap().placement.unwrap()
    };
    let template = [2, 5, 10].map(|k| place(&s, k).angle_deg);
    run(&mut s, json!({"type": "garment.arrange"})).unwrap();
    // Same arrangement as the template's hand-written one: front, then the
    // back sewn to its right side, then the mirrored back.
    assert_eq!([2, 5, 10].map(|k| place(&s, k).angle_deg), template);
    assert!((place(&s, 2).top_cm - 110.0).abs() < 1e-9);
}
