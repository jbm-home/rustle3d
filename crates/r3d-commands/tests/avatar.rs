#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::json;

#[test]
fn avatar_is_validated_fitted_and_undoable() {
    let mut s = Session::new(Project::new("t"));
    let run = |s: &mut Session, v| s.run_step(&Step::from_value(v).unwrap());
    assert_eq!(
        run(&mut s, json!({"type": "avatar.get"})).unwrap(),
        json!(null)
    );

    let bad = json!({"type": "avatar.set", "body": {"masculinity": 0, "height_cm": 300, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}});
    let err = run(&mut s, bad).unwrap_err();
    assert_eq!(err.message().code, "invalid.range");
    assert_eq!(
        err.message().args.get("field").map(String::as_str),
        Some("height_cm")
    );

    let presets = run(&mut s, json!({"type": "avatar.presets"})).unwrap();
    let m = presets
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Women M")
        .unwrap()
        .clone();
    run(
        &mut s,
        json!({"type": "avatar.set", "body": m["body"], "preset": "Women M"}),
    )
    .unwrap();
    let got = run(&mut s, json!({"type": "avatar.get"})).unwrap();
    assert_eq!(got["preset"], "Women M");
    let achieved = got["achieved"]["waist_cm"].as_f64().unwrap();
    assert!((achieved - m["body"]["waist_cm"].as_f64().unwrap()).abs() < 1.5);
    // A 168 cm body: the waist, then the hips (the widest tape, low on the
    // seat), then the crotch, about where general anthropometry has them
    // (0.63, 0.50 and 0.46 of the stature).
    let at = |k: &str| got["heights_cm"][k].as_f64().unwrap();
    let (waist, hips, crotch) = (at("waist"), at("hips"), at("crotch"));
    assert!(waist > 100.0 && waist < 112.0, "waist at {waist}");
    assert!(hips > 78.0 && hips < waist - 15.0, "hips at {hips}");
    assert!(crotch > 72.0 && crotch < hips, "crotch at {crotch}");

    s.undo().unwrap();
    assert!(s.project().avatar().is_none());
}
