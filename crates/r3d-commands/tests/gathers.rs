#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! A gathered seam: a skirt twice as wide as its waistband drapes with
//! its seams closed, the fullness spread evenly.

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Session, Step};
use r3d_core::Project;
use r3d_sim::Solver;
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Value {
    s.run_step(&Step::from_value(v).unwrap()).unwrap()
}

/// A rectangle's contour edges: bottom, right, top, left.
fn rect(s: &mut Session, name: &str, origin: [f64; 2], w: f64, h: f64) -> Vec<Value> {
    run(
        s,
        json!({"type": "panel.create_rect", "name": name, "origin": origin, "width": w, "height": h}),
    );
    common::piece(s, name).edges
}

#[test]
fn a_skirt_gathered_into_a_waistband_drapes_closed() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    let band = rect(&mut s, "Band", [0.0, 700.0], 760.0, 60.0);
    let skirt = rect(&mut s, "Skirt", [0.0, 0.0], 1520.0, 500.0);
    run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": band[3]}, "b": {"edge": band[1]}}),
    );
    run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": skirt[3]}, "b": {"edge": skirt[1]}}),
    );
    let waist = run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": band[0]}, "b": {"edge": skirt[2]}}),
    )["created"][0]
        .clone();
    for (piece, top) in [("Band", 112), ("Skirt", 106)] {
        let panel = common::piece(&s, piece).panel;
        run(
            &mut s,
            json!({"type": "panel.place", "panel": panel, "around": "body", "angle_deg": 0, "top_cm": top}),
        );
    }

    // Not gathered: a 760 mm mismatch.
    let seams = run(&mut s, json!({"type": "seam.list"}));
    let listed = |seams: &Value| {
        seams
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == waist)
            .unwrap()
            .clone()
    };
    assert_eq!(listed(&seams)["gathered"], false);
    run(
        &mut s,
        json!({"type": "seam.set_gathered", "seam": waist, "gathered": true}),
    );
    let seam = listed(&run(&mut s, json!({"type": "seam.list"})));
    assert_eq!(seam["gathered"], true);
    assert!((seam["gathering_ratio"].as_f64().unwrap() - 2.0).abs() < 1e-6);

    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let collider = drape.scene.collider.clone().unwrap();
    let mut solver = Solver::new(drape.scene);
    for _ in 0..180 {
        solver.step(1.0 / 60.0);
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let stats = solver.stats();
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    let inside = p
        .iter()
        .filter(|q| collider.sample(**q).is_some_and(|c| c.distance < 0.0))
        .count();
    assert_eq!(inside, 0, "cloth inside the body");
    // It hangs from the waist rather than falling.
    let top = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    assert!(top > 1.0, "the skirt fell: top at {top}");
}
