//! Sketching on the board (spec 30): named dimensions in formulas, labels
//! placed by hand, notches and construction lines in constraints.

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

fn code(r: Result<Value, CommandError>) -> Result<Value, &'static str> {
    r.map_err(|e| e.message().code)
}

fn id(v: &Value, k: usize) -> String {
    v["created"][k].as_str().unwrap().to_owned()
}

fn pos(s: &Session, point: &str) -> [f64; 2] {
    let p = s.project().point(point.parse().unwrap()).unwrap().pos;
    [p.x, p.y]
}

/// A 300 × 400 mm rectangle with its lower left corner at `x`: its points
/// and edges (bottom, right, top, left).
fn rect(s: &mut Session, x: f64) -> (Vec<String>, Vec<String>) {
    let out = ok(
        s,
        json!({"type": "panel.create_polygon", "points": [
            [x, 0], [x + 300.0, 0], [x + 300.0, 400], [x, 400]
        ]}),
    );
    (
        (1..=4).map(|k| id(&out, k)).collect(),
        (5..=8).map(|k| id(&out, k)).collect(),
    )
}

fn add(s: &mut Session, c: Value) -> String {
    id(
        &ok(
            s,
            json!({"type": "constraint.add", "constraint": c, "measure": true}),
        ),
        0,
    )
}

fn value(s: &Session, c: &str) -> f64 {
    s.project()
        .constraint(c.parse().unwrap())
        .unwrap()
        .value()
        .unwrap()
}

#[test]
fn a_named_dimension_drives_another_pieces_formula() {
    let mut s = Session::new(Project::new("t"));
    let (_, bodice) = rect(&mut s, 0.0);
    let (_, sleeve) = rect(&mut s, 1000.0);
    let armhole = add(
        &mut s,
        json!({"kind": "length", "edge": bodice[1], "mm": 1}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.rename", "constraint": armhole, "name": "armhole"}),
    );
    let cap = add(
        &mut s,
        json!({"kind": "length", "edge": sleeve[1], "mm": 1}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.set_formula", "constraint": cap,
            "formula": "armhole + 1.5 cm"}),
    );
    assert!((value(&s, &cap) - 415.0).abs() < 1e-9);
    // The armhole changes: the cap follows, in the same step.
    ok(
        &mut s,
        json!({"type": "constraint.set_value", "constraint": armhole, "value": 420}),
    );
    assert!((value(&s, &cap) - 435.0).abs() < 1e-9);
    let len = s.project().edge_length(sleeve[1].parse().unwrap()).unwrap();
    assert!((len - 435.0).abs() < 1e-4, "{len}");

    // Renamed: the formula follows. Unnamed while used: refused.
    ok(
        &mut s,
        json!({"type": "constraint.rename", "constraint": armhole, "name": "front_armhole"}),
    );
    let f = s
        .project()
        .constraint(cap.parse().unwrap())
        .unwrap()
        .formula()
        .unwrap()
        .to_owned();
    assert_eq!(f, "front_armhole + 1.5 cm");
    assert_eq!(
        code(run(
            &mut s,
            json!({"type": "constraint.rename", "constraint": armhole})
        )),
        Err("formula.unknown_name")
    );
    // Names are unique among dimensions and variables.
    ok(
        &mut s,
        json!({"type": "variable.set", "name": "ease", "formula": "1 cm"}),
    );
    for name in ["ease", "front_armhole", "waist", "2x"] {
        assert!(
            run(
                &mut s,
                json!({"type": "constraint.rename", "constraint": cap, "name": name})
            )
            .is_err(),
            "{name}"
        );
    }
    assert_eq!(
        code(run(
            &mut s,
            json!({"type": "variable.set", "name": "front_armhole", "formula": "1 cm"})
        )),
        Err("invalid.duplicate")
    );
    // Removing a dimension a formula uses is refused.
    assert_eq!(
        code(run(
            &mut s,
            json!({"type": "constraint.remove", "constraint": armhole})
        )),
        Err("formula.unknown_name")
    );
}

#[test]
fn a_label_moves_alone() {
    let mut s = Session::new(Project::new("t"));
    let (pts, edges) = rect(&mut s, 0.0);
    let len = add(&mut s, json!({"kind": "length", "edge": edges[0], "mm": 1}));
    let before: Vec<_> = pts.iter().map(|p| pos(&s, p)).collect();
    for k in 1..=5 {
        ok(
            &mut s,
            json!({"type": "constraint.place_label", "constraint": len,
                "offset": [0, -10.0 * f64::from(k)]}),
        );
    }
    let after: Vec<_> = pts.iter().map(|p| pos(&s, p)).collect();
    assert_eq!(before, after);
    let c = s
        .project()
        .constraint(len.parse().unwrap())
        .unwrap()
        .clone();
    assert_eq!(c.label(), Some([0.0, -50.0]));
    // The drag is one undo step.
    s.undo().unwrap();
    let c = s
        .project()
        .constraint(len.parse().unwrap())
        .unwrap()
        .clone();
    assert_eq!(c.label(), None);
    // Not on a geometric constraint.
    let fixed = add(&mut s, json!({"kind": "fixed", "point": pts[0]}));
    assert_eq!(
        code(run(
            &mut s,
            json!({"type": "constraint.place_label", "constraint": fixed, "offset": [1, 1]})
        )),
        Err("invalid.not_dimension")
    );
}

#[test]
fn a_notch_keeps_its_distance_from_the_end() {
    let mut s = Session::new(Project::new("t"));
    let (pts, edges) = rect(&mut s, 0.0);
    // On the right side (from (300, 0) up to (300, 400)), 80 mm below its
    // top end.
    let notch = id(
        &ok(
            &mut s,
            json!({"type": "notch.add", "edge": edges[1], "offset_mm": 320}),
        ),
        0,
    );
    let c = add(
        &mut s,
        json!({"kind": "notch_distance", "notch": notch, "from": "end", "mm": 1}),
    );
    assert!((value(&s, &c) - 80.0).abs() < 1e-6);
    // The side grows by 100 mm at the top: the notch stays 80 mm from it.
    ok(
        &mut s,
        json!({"type": "point.move", "point": pts[2], "to": [300, 500]}),
    );
    let n = s.project().notch(notch.parse().unwrap()).unwrap().offset_mm;
    assert!((n - 420.0).abs() < 1e-6, "{n}");
    // A typed value moves the notch.
    ok(
        &mut s,
        json!({"type": "constraint.set_value", "constraint": c, "value": 150}),
    );
    let n = s.project().notch(notch.parse().unwrap()).unwrap().offset_mm;
    assert!((n - 350.0).abs() < 1e-6, "{n}");
    let list = ok(&mut s, json!({"type": "constraint.list"}));
    let listed = &list["constraints"].as_array().unwrap()[0];
    assert_eq!(listed["holds"], true);
    assert_eq!(listed["marks"].as_array().unwrap().len(), 2);
    // The notch goes: so does its dimension.
    ok(&mut s, json!({"type": "notch.remove", "notch": notch}));
    assert!(s.project().constraints().is_empty());
}

#[test]
fn points_follow_their_construction_line() {
    let mut s = Session::new(Project::new("t"));
    let (pts, edges) = rect(&mut s, 0.0);
    let panel = s.project().panels().keys().next().unwrap().to_string();
    let line = id(
        &ok(
            &mut s,
            json!({"type": "guide.add", "a": [-100, 250], "b": [500, 250]}),
        ),
        0,
    );
    // The top corners 150 mm above a level line, the bottom left on it.
    let d = add(
        &mut s,
        json!({"kind": "guide_distance", "point": pts[3], "guide": line, "mm": 1}),
    );
    assert!((value(&s, &d) - 150.0).abs() < 1e-6);
    ok(
        &mut s,
        json!({"type": "constraint.add", "constraint":
            {"kind": "horizontal", "edge": edges[2]}}),
    );
    ok(
        &mut s,
        json!({"type": "constraint.add", "constraint":
            {"kind": "on_guide", "point": pts[0], "guide": line}}),
    );
    assert!((pos(&s, &pts[0])[1] - 250.0).abs() < 1e-6);
    // The line moves down 50 mm: the points follow.
    ok(
        &mut s,
        json!({"type": "guide.move", "guide": line, "a": [-100, 200], "b": [500, 200]}),
    );
    assert!((pos(&s, &pts[0])[1] - 200.0).abs() < 1e-6);
    assert!((pos(&s, &pts[3])[1] - 350.0).abs() < 1e-6);
    assert!((pos(&s, &pts[2])[1] - 350.0).abs() < 1e-6);

    // Moving the piece carries the line it alone uses.
    ok(
        &mut s,
        json!({"type": "panel.translate",
            "panel": panel,
            "delta": [0, 1000]}),
    );
    let g = s.project().guide(line.parse().unwrap()).unwrap();
    assert!((g.a.y - 1200.0).abs() < 1e-9 && (g.b.y - 1200.0).abs() < 1e-9);
    assert_eq!(s.project().constraints().len(), 3);
    // The line goes: so do the constraints on it.
    ok(&mut s, json!({"type": "guide.remove", "guide": line}));
    assert_eq!(s.project().constraints().len(), 1);
}
