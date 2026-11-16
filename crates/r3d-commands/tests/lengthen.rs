#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

/// Lowest and highest y of every panel (mm).
fn heights(s: &Session) -> Vec<(f64, f64)> {
    let p = s.project();
    p.panels()
        .keys()
        .map(|id| {
            let ys: Vec<f64> = p.panel_outline(*id).unwrap().iter().map(|v| v.y).collect();
            (
                ys.iter().copied().fold(f64::MAX, f64::min),
                ys.iter().copied().fold(f64::MIN, f64::max),
            )
        })
        .collect()
}

#[test]
fn shortening_the_hem_moves_the_whole_hemline() {
    let mut s = Session::new(Project::new("t"));
    let front = common::a_line_skirt(&mut s);
    let front_hem = front.edges[0].clone();
    let before = heights(&s);

    run(
        &mut s,
        json!({"type": "edge.lengthen", "edge": front_hem, "mm": -30}),
    )
    .unwrap();
    let after = heights(&s);
    assert_eq!(before.len(), 3);
    for ((lo0, hi0), (lo1, hi1)) in before.iter().zip(&after) {
        // Every hem (front, back, and the mirrored back) rose by 30 mm;
        // the waist did not move.
        assert!((lo1 - lo0 - 30.0).abs() < 1e-9, "{lo0} -> {lo1}");
        assert!((hi1 - hi0).abs() < 1e-9);
    }
    let seams = run(&mut s, json!({"type": "seam.list"})).unwrap();
    for seam in seams.as_array().unwrap() {
        assert!(seam["mismatch_mm"].as_f64().unwrap() < 0.5, "{seam}");
    }

    s.undo().unwrap();
    assert_eq!(heights(&s), before);

    // Seams cannot be lengthened this way, and a hem cannot cross the waist.
    let side = front.edges[1].clone();
    let err = run(
        &mut s,
        json!({"type": "edge.lengthen", "edge": side, "mm": 10}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "sewn");
    let err = run(
        &mut s,
        json!({"type": "edge.lengthen", "edge": front_hem, "mm": -700}),
    )
    .unwrap_err();
    // The curved side seams cross before the panel turns inside out.
    assert!(
        ["invalid.inverted", "invalid.not_simple"].contains(&err.message().code),
        "{err}"
    );
}
