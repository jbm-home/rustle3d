#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

#[test]
fn pins_are_validated_listed_and_follow_their_panel() {
    let mut s = Session::new(Project::new("t"));
    let rect = run(
        &mut s,
        json!({"type": "panel.create_rect", "width": 100, "height": 200}),
    )
    .unwrap();
    let panel = rect["created"][0].clone();
    let err = run(
        &mut s,
        json!({"type": "pin.add", "panel": "panel:99", "at": [0, 0], "position": [0, 1, 0]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "not_found");
    let out = run(
        &mut s,
        json!({"type": "pin.add", "panel": panel, "at": [50, 190], "position": [0.0, 1.5, 0.2]}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "pin.list"})).unwrap();
    assert_eq!(list[0]["id"], out["created"][0]);
    assert_eq!(list[0]["position"], json!([0.0, 1.5, 0.2]));

    run(
        &mut s,
        json!({"type": "panel.delete", "panel": panel, "confirm": true}),
    )
    .unwrap();
    assert!(s.project().pins().is_empty());
    s.undo().unwrap();
    assert_eq!(s.project().pins().len(), 1);
}
