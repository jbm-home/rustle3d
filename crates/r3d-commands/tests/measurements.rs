//! Body measurements besides the four (spec 29): estimated until typed,
//! named in formulas, graded in sizes, read by templates.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Measure, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, step: Value) -> Result<Value, CommandError> {
    s.run_step(&Step::from_value(step)?)
}

fn ok(s: &mut Session, step: Value) -> Value {
    run(s, step.clone()).unwrap_or_else(|e| panic!("{step}: {e}"))
}

fn women_m() -> Session {
    let mut s = Session::new(Project::new("t"));
    ok(
        &mut s,
        json!({"type": "avatar.set", "preset": "Women M", "body":
            {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
    );
    s
}

fn shirt_sleeve(s: &mut Session) -> f64 {
    let list = ok(s, json!({"type": "template.list"}));
    let shirt = list
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["template"] == "shirt")
        .unwrap();
    shirt["sizes"]["sleeve_cm"].as_f64().unwrap()
}

#[test]
fn typed_measurements_replace_their_estimates_until_reset() {
    let mut s = women_m();
    let body = s.project().avatar().unwrap().body;
    let estimate = Measure::ArmLength.estimate(&body);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(near(shirt_sleeve(&mut s), estimate - 2.0));

    ok(
        &mut s,
        json!({"type": "avatar.set_measures", "measures": {"arm_length": 60, "neck": 37}}),
    );
    let avatar = s.project().avatar().unwrap();
    assert!(!avatar.measure(Measure::ArmLength).estimated);
    assert_eq!(avatar.measure(Measure::Neck).cm, 37.0);
    assert!(avatar.measure(Measure::Knee).estimated);
    assert!(near(shirt_sleeve(&mut s), 58.0));

    // Changing the four keeps them.
    ok(
        &mut s,
        json!({"type": "avatar.set", "body":
            {"masculinity": 0, "height_cm": 170, "bust_cm": 92, "waist_cm": 75, "hips_cm": 100}}),
    );
    assert!(near(shirt_sleeve(&mut s), 58.0));

    ok(
        &mut s,
        json!({"type": "avatar.set_measures", "measures": {"arm_length": null}}),
    );
    let body = s.project().avatar().unwrap().body;
    assert!(near(
        shirt_sleeve(&mut s),
        Measure::ArmLength.estimate(&body) - 2.0
    ));
    assert_eq!(s.project().avatar().unwrap().measures.len(), 1);
}

#[test]
fn wrong_measurements_are_refused() {
    let code = |r: Result<Value, CommandError>| r.map_err(|e| e.message().code);
    let mut none = Session::new(Project::new("t"));
    assert_eq!(
        code(run(
            &mut none,
            json!({"type": "avatar.set_measures", "measures": {"neck": 37}})
        )),
        Err("invalid.no_body")
    );
    let mut s = women_m();
    assert_eq!(
        code(run(
            &mut s,
            json!({"type": "avatar.set_measures", "measures": {"neck": 12}})
        )),
        Err("invalid.range")
    );
    assert_eq!(
        code(run(
            &mut s,
            json!({"type": "avatar.set_measures", "measures": {}})
        )),
        Err("invalid.empty")
    );
    assert!(
        run(
            &mut s,
            json!({"type": "avatar.set_measures", "measures": {"collar": 40}})
        )
        .is_err()
    );
    assert!(s.project().avatar().unwrap().measures.is_empty());
}

#[test]
fn formulas_name_them_and_sizes_grade_them() {
    let mut s = women_m();
    ok(
        &mut s,
        json!({"type": "avatar.set_measures", "measures": {"neck": 38}}),
    );
    ok(
        &mut s,
        json!({"type": "variable.set", "name": "collar", "formula": "neck + 2 cm"}),
    );
    let vars = ok(&mut s, json!({"type": "variable.list"}));
    assert_eq!(vars["variables"][0]["mm"], 400.0);
    assert_eq!(vars["body"]["neck"], 380.0);
    assert!(vars["body"]["inside_leg"].as_f64().unwrap() > 700.0);
    // A name of a measurement cannot name a variable.
    assert!(
        run(
            &mut s,
            json!({"type": "variable.set", "name": "wrist", "formula": "16 cm"})
        )
        .is_err()
    );

    ok(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "38"}),
    );
    let p = s.project();
    let base = p.avatar().unwrap();
    let n = p.grading().unwrap().sizes.len();
    for i in 0..n {
        let sized = p.at_size(i);
        let a = sized.avatar().unwrap();
        // The typed neck keeps its distance to its estimate.
        let off = a.measure(Measure::Neck).cm - Measure::Neck.estimate(&a.body);
        let base_off = 38.0 - Measure::Neck.estimate(&base.body);
        assert!((off - base_off).abs() < 1e-9, "size {i}");
    }
}

#[test]
fn avatar_get_lists_every_measurement() {
    let mut s = women_m();
    let got = ok(&mut s, json!({"type": "avatar.get"}));
    let list = got["measures"].as_array().unwrap();
    assert_eq!(list.len(), Measure::ALL.len());
    for m in list {
        assert_eq!(m["estimated"], true);
        let (cm, reached) = (m["cm"].as_f64().unwrap(), m["reached_cm"].as_f64().unwrap());
        assert!((cm - reached).abs() <= 2.5, "{m}");
    }
    let neck = list.iter().find(|m| m["name"] == "neck").unwrap();
    assert_eq!(neck["group"], "torso");
    assert_eq!(neck["follows"], true);
    let knee = list.iter().find(|m| m["name"] == "knee").unwrap();
    assert_eq!(knee["follows"], false);
}
