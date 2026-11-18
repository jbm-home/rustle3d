//! Constraints, dimensions, symmetric pieces, construction lines and round
//! shapes (spec 19).

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

fn id(v: &Value, k: usize) -> String {
    v["created"][k].as_str().unwrap().to_owned()
}

fn pos(s: &Session, point: &str) -> [f64; 2] {
    let id = point.parse().unwrap();
    let p = s.project().point(id).unwrap().pos;
    [p.x, p.y]
}

fn close(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
}

/// A half bodice drawn against a vertical axis at x = 0: centre front
/// (on the axis) from the waist up to the neck, then the neckline, the
/// shoulder, the armhole, the side seam and the waist. Returns the
/// panel, its points and edges in drawing order.
fn half_bodice(s: &mut Session) -> (String, Vec<String>, Vec<String>) {
    let out = ok(
        s,
        json!({"type": "panel.create_polygon", "name": "Front", "points": [
            [0, 0], [220, 0], [240, 250], [180, 400], [70, 440], [0, 380]
        ]}),
    );
    let panel = id(&out, 0);
    let points = (1..=6).map(|k| id(&out, k)).collect();
    let edges = (7..=12).map(|k| id(&out, k)).collect();
    (panel, points, edges)
}

#[test]
fn a_half_bodice_with_dimensions_redraws_both_halves() {
    let mut s = Session::new(Project::new("t"));
    let (panel, pts, edges) = half_bodice(&mut s);
    // The waist is horizontal and fixed at the centre front; the side
    // seam's top (the bust point) is 240 mm out.
    ok(
        &mut s,
        json!({"type": "constraint.add",
        "constraint": {"kind": "fixed", "point": pts[0]}}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.add",
        "constraint": {"kind": "horizontal", "edge": edges[0]}}),
    );
    let bust = ok(
        &mut s,
        json!({"type": "constraint.add", "constraint": {
        "kind": "distance", "a": pts[0], "b": pts[2], "mm": 240, "along": "horizontal"}}),
    );
    let bust = id(&bust, 0);
    // The constraints hold: nothing had to move.
    assert!(close(pos(&s, &pts[2]), [240.0, 250.0]));
    // Unfold on the centre front (the closing edge, from the neck down to
    // the waist).
    let unfolded = ok(
        &mut s,
        json!({"type": "panel.unfold", "panel": panel, "edge": edges[5]}),
    );
    assert_eq!(unfolded["created"].as_array().unwrap().len(), 4 + 5);
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(get["point_count"], 10);
    assert_eq!(get["bbox"]["min"][0], -240.0);
    assert_eq!(get["bbox"]["max"][0], 240.0);
    assert_eq!(get["is_simple"], true);
    // Change the bust dimension: the piece and its other half follow.
    ok(
        &mut s,
        json!({"type": "constraint.set_value", "constraint": bust, "value": 260}),
    );
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(get["bbox"]["max"][0], 260.0);
    assert_eq!(get["bbox"]["min"][0], -260.0);
    let area = get["area_mm2"].as_f64().unwrap();
    // The fixed centre front and the waist did not move.
    assert!(close(pos(&s, &pts[0]), [0.0, 0.0]));
    assert!(close(pos(&s, &pts[1]), [220.0, 0.0]) || pos(&s, &pts[1])[1].abs() < 1e-6);
    // One undo step for the whole change.
    ok(&mut s, json!({"type": "history.undo"}));
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(get["bbox"]["max"][0], 240.0);
    ok(&mut s, json!({"type": "history.redo"}));
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(get["area_mm2"].as_f64().unwrap(), area);
    s.project().check_integrity().unwrap();
}

#[test]
fn a_dragged_point_stays_and_the_rest_follows() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 300, "height": 400}),
    );
    let (p0, p1) = (id(&out, 1), id(&out, 2));
    let bottom = id(&out, 5);
    ok(
        &mut s,
        json!({"type": "constraint.add",
        "constraint": {"kind": "horizontal", "edge": bottom}}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.add", "measure": true,
        "constraint": {"kind": "length", "edge": bottom, "mm": 1}}),
    );
    ok(
        &mut s,
        json!({"type": "point.move", "point": p1, "to": [350, 30]}),
    );
    // The moved corner is where it was put; the other end keeps the
    // bottom horizontal and 300 long.
    assert!(close(pos(&s, &p1), [350.0, 30.0]));
    assert!(close(pos(&s, &p0), [50.0, 30.0]));
}

#[test]
fn conflicting_constraints_are_refused_and_nothing_changes() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 300, "height": 400}),
    );
    let (p0, p1, bottom) = (id(&out, 1), id(&out, 2), id(&out, 5));
    for p in [&p0, &p1] {
        ok(
            &mut s,
            json!({"type": "constraint.add",
            "constraint": {"kind": "fixed", "point": p}}),
        );
    }
    let before = s.project().clone();
    let err = run(
        &mut s,
        json!({"type": "constraint.add",
        "constraint": {"kind": "length", "edge": bottom, "mm": 200}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "constraints_unsolvable");
    assert_eq!(s.project(), &before);
    // A duplicate is refused too.
    let err = run(
        &mut s,
        json!({"type": "constraint.add",
        "constraint": {"kind": "fixed", "point": p0}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.duplicate");
}

#[test]
fn the_list_reports_values_freedom_and_redundancy() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 300, "height": 400}),
    );
    let panel = id(&out, 0);
    let (bottom, top) = (id(&out, 5), id(&out, 7));
    ok(
        &mut s,
        json!({"type": "constraint.add", "constraint": {"kind": "horizontal", "edge": bottom}}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.add", "constraint": {"kind": "horizontal", "edge": top}}),
    );
    let par = ok(
        &mut s,
        json!({"type": "constraint.add",
        "constraint": {"kind": "parallel", "a": bottom, "b": top}}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.add", "measure": true,
        "constraint": {"kind": "length", "edge": top, "mm": 1}}),
    );
    let list = ok(&mut s, json!({"type": "constraint.list", "panel": panel}));
    let cs = list["constraints"].as_array().unwrap();
    assert_eq!(cs.len(), 4);
    assert_eq!(cs[3]["measured"], 300.0);
    assert_eq!(cs[3]["mm"], 300.0);
    assert!(cs.iter().all(|c| c["holds"] == true));
    let groups = list["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["redundant"], json!([par["created"][0]]));
    // Four points (8 numbers), three independent equations.
    assert_eq!(groups[0]["freedom"], 5);
}

#[test]
fn a_symmetric_piece_keeps_its_reflection_in_step() {
    let mut s = Session::new(Project::new("t"));
    let (panel, pts, edges) = half_bodice(&mut s);
    ok(
        &mut s,
        json!({"type": "notch.add", "edge": edges[2], "offset_mm": 40}),
    );
    ok(
        &mut s,
        json!({"type": "panel.unfold", "panel": panel, "edge": edges[5]}),
    );
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    // The notch is reflected.
    assert_eq!(get["notches"].as_array().unwrap().len(), 2);
    let derived: Vec<String> = get["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["derived"] == true)
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(derived.len(), 5);
    // The reflected side seam can be sewn (to itself's twin, as a test).
    let side = edges[1].clone();
    let twin_side = derived[3].clone();
    ok(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": side}, "b": {"edge": twin_side}}),
    );
    // A reflected point cannot be moved; a drawn one can, and its twin
    // follows.
    let twin_point = get["edges"][7]["start"]["id"].as_str().unwrap().to_owned();
    let err = run(
        &mut s,
        json!({"type": "point.move", "point": twin_point, "to": [0, 0]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "derived");
    ok(
        &mut s,
        json!({"type": "point.move", "point": pts[3], "to": [190, 410]}),
    );
    let mirrored = pos(&s, &twin_point);
    let drawn = pos(&s, &pts[3]);
    assert!(close(mirrored, [-drawn[0], drawn[1]]), "{mirrored:?}");
    // Splitting a drawn edge adds its twin; the seam on the side stays.
    ok(
        &mut s,
        json!({"type": "edge.split", "edge": edges[3], "at_mm": 50}),
    );
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(get["point_count"], 12);
    assert_eq!(s.project().seams().len(), 1);
    // The axis ends stay on the axis.
    ok(
        &mut s,
        json!({"type": "point.move", "point": pts[0], "to": [15, -5]}),
    );
    assert!(close(pos(&s, &pts[0]), [0.0, -5.0]));
    s.project().check_integrity().unwrap();
    // Unlinked, the halves are ordinary geometry.
    ok(&mut s, json!({"type": "panel.unlink", "panel": panel}));
    ok(
        &mut s,
        json!({"type": "point.move", "point": twin_point, "to": [-100, 300]}),
    );
}

#[test]
fn a_symmetric_piece_moves_with_its_axis_and_pastes_whole() {
    let mut s = Session::new(Project::new("t"));
    let (panel, _, edges) = half_bodice(&mut s);
    ok(
        &mut s,
        json!({"type": "panel.unfold", "panel": panel, "edge": edges[5]}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.add", "measure": true,
        "constraint": {"kind": "length", "edge": edges[0], "mm": 1}}),
    );
    ok(
        &mut s,
        json!({"type": "panel.translate", "panel": panel, "delta": [1000, 0]}),
    );
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(get["bbox"]["min"][0], 760.0);
    assert_eq!(get["bbox"]["max"][0], 1240.0);
    let clip = ok(&mut s, json!({"type": "panel.copy", "panels": [panel]}));
    let pasted = ok(
        &mut s,
        json!({"type": "panel.paste", "clip": clip, "offset": [0, 1000]}),
    );
    let copy = id(&pasted, 0);
    let got = ok(&mut s, json!({"type": "panel.get", "panel": copy}));
    assert!(got["symmetry"].is_object());
    let list = ok(&mut s, json!({"type": "constraint.list", "panel": copy}));
    assert_eq!(list["constraints"].as_array().unwrap().len(), 1);
    s.project().check_integrity().unwrap();
}

#[test]
fn a_smooth_point_stays_smooth() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_polygon",
        "points": [[0, 0], [300, 0], [300, 300], [150, 450], [0, 300]]}),
    );
    let top = id(&out, 4);
    ok(
        &mut s,
        json!({"type": "point.set_smooth", "point": top, "smooth": true}),
    );
    let list = ok(&mut s, json!({"type": "constraint.list"}));
    assert_eq!(list["constraints"][0]["kind"], "tangent");
    // Move a neighbour: the joint stays smooth.
    ok(
        &mut s,
        json!({"type": "point.move", "point": id(&out, 3), "to": [330, 340]}),
    );
    let list = ok(&mut s, json!({"type": "constraint.list"}));
    assert_eq!(list["constraints"][0]["holds"], true);
    // A corner again: the constraint goes.
    ok(
        &mut s,
        json!({"type": "point.set_smooth", "point": top, "smooth": false}),
    );
    let list = ok(&mut s, json!({"type": "constraint.list"}));
    assert!(list["constraints"].as_array().unwrap().is_empty());
}

#[test]
fn a_circle_skirt_sector_has_the_waist_it_was_given() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_sector", "inner_length_mm": 360,
        "length_mm": 600, "sweep_deg": 180}),
    );
    let panel = id(&out, 0);
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    let edges = get["edges"].as_array().unwrap();
    // Hem in two quarter arcs, a side, the waist in two arcs, a side.
    assert_eq!(edges.len(), 6);
    let waist: f64 = edges[3..5]
        .iter()
        .map(|e| e["length_mm"].as_f64().unwrap())
        .sum();
    assert!((waist - 360.0).abs() < 0.2, "{waist}");
    let r = 360.0 / std::f64::consts::PI;
    let hem: f64 = edges[0..2]
        .iter()
        .map(|e| e["length_mm"].as_f64().unwrap())
        .sum();
    assert!(
        (hem - (r + 600.0) * std::f64::consts::PI).abs() < 0.5,
        "{hem}"
    );
    assert_eq!(edges[2]["length_mm"], 600.0);
    assert_eq!(get["is_simple"], true);
    // Hanging below its centre.
    assert!(get["bbox"]["max"][1].as_f64().unwrap() <= 1e-9);
}

#[test]
fn an_arc_bulges_as_asked_and_long_ones_are_split() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_rect", "width": 200, "height": 300}),
    );
    let (panel, bottom) = (id(&out, 0), id(&out, 5));
    let created = ok(
        &mut s,
        json!({"type": "edge.set_arc", "edge": bottom, "bulge_mm": 30}),
    );
    assert!(created["created"].as_array().is_none_or(Vec::is_empty));
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    // Outward: below the bottom edge.
    assert!((get["bbox"]["min"][1].as_f64().unwrap() + 30.0).abs() < 0.05);
    // A half circle over the 300 mm side: two quarter arcs.
    let right = id(&out, 6);
    let created = ok(
        &mut s,
        json!({"type": "edge.set_arc", "edge": right, "bulge_mm": 150}),
    );
    assert_eq!(created["created"].as_array().unwrap().len(), 2);
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert!((get["bbox"]["max"][0].as_f64().unwrap() - 350.0).abs() < 0.1);
    s.project().check_integrity().unwrap();
}

#[test]
fn an_ellipse_is_four_arcs() {
    let mut s = Session::new(Project::new("t"));
    let out = ok(
        &mut s,
        json!({"type": "panel.create_ellipse", "centre": [100, 100], "radii": [80, 40]}),
    );
    let get = ok(&mut s, json!({"type": "panel.get", "panel": id(&out, 0)}));
    assert_eq!(get["bbox"]["min"], json!([20.0, 60.0]));
    assert_eq!(get["bbox"]["max"], json!([180.0, 140.0]));
    let area = get["area_mm2"].as_f64().unwrap();
    assert!((area - std::f64::consts::PI * 80.0 * 40.0).abs() / area < 1e-3);
}

#[test]
fn guides_are_listed_and_kept_out_of_pieces() {
    let mut s = Session::new(Project::new("t"));
    ok(
        &mut s,
        json!({"type": "guide.add", "a": [0, 250], "b": [600, 250]}),
    );
    let list = ok(&mut s, json!({"type": "guide.list"}));
    assert_eq!(list[0]["b"], json!([600.0, 250.0]));
    let summary = ok(&mut s, json!({"type": "project.summary"}));
    assert_eq!(summary["panels"].as_array().unwrap().len(), 0);
}

#[test]
fn a_half_drawn_symmetric_is_unfolded_on_its_closing_edge() {
    let mut s = Session::new(Project::new("t"));
    // Drawn clockwise on purpose: stored the other way round, the axis is
    // still the edge back to the first point.
    let out = ok(
        &mut s,
        json!({"type": "panel.create_polygon", "symmetric": true,
        "points": [[0, 400], [80, 420], [200, 300], [220, 0], [0, 0]]}),
    );
    let panel = id(&out, 0);
    let get = ok(&mut s, json!({"type": "panel.get", "panel": panel}));
    assert_eq!(get["point_count"], 8);
    assert_eq!(get["bbox"]["min"][0], -220.0);
    assert_eq!(get["is_simple"], true);
    assert_eq!(get["symmetry"]["axis_dir"][0], 0.0);
    // A path whose last edge is curved has no straight axis.
    let err = run(
        &mut s,
        json!({"type": "panel.create_path", "symmetric": true,
        "start": [0, 0], "segments": [
            {"kind": "line", "to": [100, 0]},
            {"kind": "cubic", "c1": [100, 100], "c2": [50, 150], "to": [0, 0]}]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.not_straight");
}
