//! Parametric patterns (spec 20): dimensions computed from the body's
//! measurements and named variables, redrafted when the body changes and
//! in every size of the range.

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

fn body(waist: f64, hips: f64) -> Value {
    json!({"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": waist, "hips_cm": hips})
}

/// A quarter skirt: the left side (centre front) vertical and 600 long,
/// the waist (top) `waist / 4 + 10 mm` and the hem `hips / 4 + 40 mm`,
/// both horizontal, the hem's left end fixed. Returns the session, the
/// panel and its edges (hem, side, waist, centre front).
fn skirt() -> (Session, String, Vec<String>) {
    let mut s = Session::new(Project::new("t"));
    ok(
        &mut s,
        json!({"type": "avatar.set", "body": body(72.0, 98.0)}),
    );
    let out = ok(
        &mut s,
        json!({"type": "panel.create_polygon", "name": "Front",
        "points": [[0, 0], [260, 0], [200, 600], [0, 600]]}),
    );
    let id = |k: usize| out["created"][k].as_str().unwrap().to_owned();
    let (panel, p0) = (id(0), id(1));
    let edges: Vec<String> = (5..9).map(id).collect();
    for c in [
        json!({"kind": "fixed", "point": p0}),
        json!({"kind": "horizontal", "edge": edges[0]}),
        json!({"kind": "horizontal", "edge": edges[2]}),
        json!({"kind": "vertical", "edge": edges[3]}),
        json!({"kind": "length", "edge": edges[3], "mm": 600}),
        json!({"kind": "length", "edge": edges[2], "mm": 1, "formula": "waist / 4 + 10 mm"}),
        json!({"kind": "length", "edge": edges[0], "mm": 1, "formula": "hips / 4 + ease"}),
    ] {
        if c["formula"] == "hips / 4 + ease" {
            ok(
                &mut s,
                json!({"type": "variable.set", "name": "ease", "formula": "4 cm"}),
            );
        }
        ok(&mut s, json!({"type": "constraint.add", "constraint": c}));
    }
    (s, panel, edges)
}

fn length(s: &mut Session, panel: &str, edge: &str, size: Option<&str>) -> f64 {
    let mut q = json!({"type": "panel.get", "panel": panel});
    if let Some(size) = size {
        q["size"] = json!(size);
    }
    let got = ok(s, q);
    got["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == edge)
        .unwrap()["length_mm"]
        .as_f64()
        .unwrap()
}

#[test]
fn a_skirt_follows_the_body_it_is_made_for() {
    let (mut s, panel, edges) = skirt();
    assert!((length(&mut s, &panel, &edges[2], None) - 190.0).abs() < 1e-3);
    assert!((length(&mut s, &panel, &edges[0], None) - 285.0).abs() < 1e-3);
    // Another body: the pattern redrafts itself.
    ok(
        &mut s,
        json!({"type": "avatar.set", "body": body(80.0, 104.0)}),
    );
    assert!((length(&mut s, &panel, &edges[2], None) - 210.0).abs() < 1e-3);
    assert!((length(&mut s, &panel, &edges[0], None) - 300.0).abs() < 1e-3);
    // More ease: the hem follows the variable.
    ok(
        &mut s,
        json!({"type": "variable.set", "name": "ease", "formula": "6 cm"}),
    );
    assert!((length(&mut s, &panel, &edges[0], None) - 320.0).abs() < 1e-3);
    // One undo step each.
    ok(&mut s, json!({"type": "history.undo"}));
    ok(&mut s, json!({"type": "history.undo"}));
    assert!((length(&mut s, &panel, &edges[2], None) - 190.0).abs() < 1e-3);
    let list = ok(&mut s, json!({"type": "constraint.list", "panel": panel}));
    let formulas: Vec<&Value> = list["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["formula"].is_string())
        .collect();
    assert_eq!(formulas.len(), 2);
    assert!(formulas.iter().all(|c| c["holds"] == true));
    s.project().check_integrity().unwrap();
}

#[test]
fn every_size_is_drafted_with_its_own_measurements() {
    let (mut s, panel, edges) = skirt();
    ok(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women",
        "from": "36", "to": "44", "base": "40"}),
    );
    let grading = ok(&mut s, json!({"type": "grading.get"}));
    let body = s.project().avatar().unwrap().body;
    for size in grading["sizes"].as_array().unwrap() {
        let name = size["name"].as_str().unwrap();
        let waist = body.waist_cm + size["waist_cm"].as_f64().unwrap();
        let got = length(&mut s, &panel, &edges[2], Some(name));
        assert!(
            (got - (waist * 10.0 / 4.0 + 10.0)).abs() < 1e-3,
            "{name}: {got}"
        );
        // The centre front stays 600 long in every size.
        assert!((length(&mut s, &panel, &edges[3], Some(name)) - 600.0).abs() < 1e-3);
    }
    // The waist's corner is graded by the formulas, not by hand.
    let corner = s.project().edge(edges[2].parse().unwrap()).unwrap().start;
    let err = run(
        &mut s,
        json!({"type": "point.set_grade", "point": corner.to_string(),
        "step": [5, 0]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.graded_by_formula");
    s.project().check_integrity().unwrap();
}

#[test]
fn formulas_are_checked() {
    let (mut s, _, edges) = skirt();
    let list = ok(&mut s, json!({"type": "constraint.list"}));
    let waist = list["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["formula"] == "waist / 4 + 10 mm")
        .unwrap()["id"]
        .clone();
    let before = s.project().clone();
    for (formula, code) in [
        ("waist / 4 + 10", "formula.unit"),
        ("collar / 2", "formula.unknown_name"),
        ("waist / 4 +", "formula.syntax"),
        ("waist - hips", "formula.range"),
    ] {
        let err = run(
            &mut s,
            json!({"type": "constraint.set_formula",
            "constraint": waist, "formula": formula}),
        )
        .unwrap_err();
        assert_eq!(err.message().code, code, "{formula}");
        assert_eq!(s.project(), &before);
    }
    let err = run(
        &mut s,
        json!({"type": "variable.set", "name": "a", "formula": "a + 1 mm"}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "formula.cycle");
    // Still in use: refused.
    let err = run(&mut s, json!({"type": "variable.remove", "name": "ease"})).unwrap_err();
    assert_eq!(err.message().code, "formula.unknown_name");
    // Without its formula, the dimension keeps its value.
    ok(
        &mut s,
        json!({"type": "constraint.set_formula", "constraint": waist}),
    );
    ok(
        &mut s,
        json!({"type": "avatar.set", "body": body(90.0, 110.0)}),
    );
    let edge = s.project().edge(edges[2].parse().unwrap()).unwrap().clone();
    let a = s.project().point(edge.start).unwrap().pos;
    let b = s.project().point(edge.end).unwrap().pos;
    assert!((a.distance(b) - 190.0).abs() < 1e-3);
    let vars = ok(&mut s, json!({"type": "variable.list"}));
    assert_eq!(vars["variables"][0]["mm"], 40.0);
    assert_eq!(vars["body"]["waist"], 900.0);
}
