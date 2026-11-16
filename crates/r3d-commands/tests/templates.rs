#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Every starter garment, at its default sizes on a standard body, is sewn
//! with matched seams and drapes onto the body (docs/specs/11).

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Session, Step};
use r3d_core::{Around, Project};
use r3d_sim::Solver;
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Value {
    s.run_step(&Step::from_value(v).unwrap()).unwrap()
}

/// Lowest and highest cloth point (m) after `frames` frames.
fn drape(s: &Session, frames: u32) -> (f32, f32) {
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let mut solver = Solver::new(drape.scene);
    for _ in 0..frames {
        solver.step(1.0 / 60.0);
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let stats = solver.stats();
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    let low = p.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let high = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    (low, high)
}

fn draft(template: &str) -> Session {
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": template}),
    );
    let seams = run(&mut s, json!({"type": "seam.list"}));
    for seam in seams.as_array().unwrap() {
        assert!(seam["mismatch_mm"].as_f64().unwrap() < 1.0, "{seam}");
    }
    s
}

#[test]
fn the_list_fills_sizes_from_the_avatar() {
    let mut s = Session::new(Project::new("t"));
    let list = run(&mut s, json!({"type": "template.list"}));
    let shirt = &list[0];
    assert_eq!(shirt["template"], "t_shirt");
    // Without an avatar: the standard size.
    assert_eq!(shirt["sizes"]["chest_cm"], 90.0);
    assert!(shirt["sizes"].get("waist_cm").is_none());
    run(
        &mut s,
        json!({"type": "avatar.set", "body": {"masculinity": 1, "height_cm": 182,
            "bust_cm": 104, "waist_cm": 90, "hips_cm": 102}}),
    );
    let list = run(&mut s, json!({"type": "template.list"}));
    assert_eq!(list[0]["sizes"]["chest_cm"], 104.0);
    assert_eq!(list[1]["sizes"]["waist_cm"], 90.0);
}

#[test]
fn a_t_shirt_hangs_from_the_shoulders() {
    let s = draft("t_shirt");
    let p = s.project();
    assert_eq!(p.name(), "T-shirt");
    assert_eq!(p.panels().len(), 4);
    assert_eq!(p.seams().len(), 10);
    let on_arms = p
        .panels()
        .values()
        .filter(|pn| pn.placement.is_some_and(|pl| pl.around != Around::Body))
        .count();
    assert_eq!(on_arms, 2);
    let (low, high) = drape(&s, 180);
    eprintln!("t-shirt: {low:.3}..{high:.3}");
    // A 168 cm body: the neckline stays near the base of the neck (1.39 m
    // to the shoulder tops), the hem above the knees.
    assert!(high > 1.35 && high < 1.50, "top at {high}");
    assert!(low > 0.7, "hem at {low}");
}

#[test]
fn skirts_hang_from_the_waist() {
    for template in ["straight_skirt", "a_line_skirt"] {
        let s = draft(template);
        assert_eq!(s.project().panels().len(), 3);
        let (low, high) = drape(&s, 120);
        eprintln!("{template}: {low:.3}..{high:.3}");
        assert!(high > 1.03, "{template} slid down: top at {high}");
        assert!(low > 0.3, "{template} hem at {low}");
    }
}
