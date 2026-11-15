#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Script, Session, Step, templates};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

fn skirt() -> (Session, Vec<Value>) {
    let mut s = Session::new(Project::new("t"));
    let script = Script::parse(templates::get("a-line-skirt").unwrap()).unwrap();
    let results = script.run(|step| s.run_step(step)).unwrap();
    (s, results)
}

fn max_mismatch(s: &mut Session) -> f64 {
    let seams = run(s, json!({"type": "seam.list"})).unwrap();
    seams
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["mismatch_mm"].as_f64().unwrap())
        .fold(0.0, f64::max)
}

/// (x, y) of a point, mm.
fn pos(s: &Session, id: &Value) -> (f64, f64) {
    let p = s
        .project()
        .point(id.as_str().unwrap().parse().unwrap())
        .unwrap()
        .pos;
    (p.x, p.y)
}

#[test]
fn taking_in_a_side_seam_keeps_it_matched() {
    let (mut s, results) = skirt();
    let side_seam = results[14]["created"][0].clone(); // front right side + back left
    let front = &results[2]["created"];
    let (hem, waist) = (front[2].clone(), front[3].clone()); // (780,0), (575,600)
    let (hem0, waist0) = (pos(&s, &hem), pos(&s, &waist));

    run(
        &mut s,
        json!({"type": "seam.take_in", "seam": side_seam, "mm": 20}),
    )
    .unwrap();
    assert!(max_mismatch(&mut s) < 0.5);
    // The front's right side moved inward (left) at both ends.
    assert!(pos(&s, &hem).0 < hem0.0 - 5.0 && pos(&s, &waist).0 < waist0.0 - 5.0);

    s.undo().unwrap();
    assert_eq!(pos(&s, &hem), hem0);

    // Only at the waist: the hem stays. The back is a linked mirror, so the
    // left side seam is taken in too (its waist point moves right), and
    // every seam stays matched even though the change is tapered.
    let left_waist = front[4].clone(); // (205, 600)
    let left0 = pos(&s, &left_waist);
    run(
        &mut s,
        json!({"type": "seam.take_in", "seam": side_seam, "mm": 60, "at": waist}),
    )
    .unwrap();
    assert_eq!(pos(&s, &hem), hem0);
    assert!(pos(&s, &waist).0 < waist0.0 - 20.0);
    assert!(pos(&s, &left_waist).0 > left0.0 + 20.0);
    assert!(max_mismatch(&mut s) < 0.5, "{}", max_mismatch(&mut s));

    let err = run(
        &mut s,
        json!({"type": "seam.take_in", "seam": side_seam, "mm": 5, "at": front[1]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.not_seam_end");
}
