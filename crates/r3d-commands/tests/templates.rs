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
    let collider = drape.scene.collider.clone().unwrap();
    let triangles: Vec<[u32; 3]> = drape.scene.triangles.iter().map(|t| t.vertices).collect();
    let mut solver = Solver::new(drape.scene);
    for _ in 0..frames {
        solver.step(1.0 / 60.0);
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    // No cloth inside the body.
    let inside = p
        .iter()
        .filter(|q| collider.sample(**q).is_some_and(|c| c.distance < 0.0))
        .count();
    assert_eq!(inside, 0, "particles inside the body");
    // Nor between particles: triangles are flat, and across a convex part
    // of the body (the bust) their middle could sink in.
    let sunk = triangles
        .iter()
        .map(|t| t.map(|i| p[i as usize]))
        .flat_map(|[a, b, c]| {
            [
                (a + b + c) / 3.0,
                (a + b) / 2.0,
                (b + c) / 2.0,
                (c + a) / 2.0,
            ]
        })
        .filter_map(|q| collider.sample(q).map(|c| -c.distance))
        .fold(0.0f32, f32::max);
    // Within 1 mm: unseen, where 2 mm showed the body through a dress.
    assert!(sunk < 0.001, "triangles cut {sunk} m into the body");
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
    assert_eq!(list[1]["template"], "dress");
    assert_eq!(list[2]["sizes"]["waist_cm"], 90.0);
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
fn a_t_shirt_comes_to_rest() {
    // Edges held out of the body used to let go of the particles they had
    // pushed off the skin; they fell back and were pushed again every
    // substep, and the shirt kept fluttering (12 cm/s on average, for
    // ever). Settled cloth moves about 1 mm/s.
    let s = draft("t_shirt");
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let mut solver = Solver::new(drape.scene);
    for _ in 0..240 {
        solver.step(1.0 / 60.0);
    }
    let before = solver.positions().to_vec();
    solver.step(1.0 / 60.0);
    let speed = solver
        .positions()
        .iter()
        .zip(&before)
        .map(|(a, b)| a.distance(*b) * 60.0)
        .sum::<f32>()
        / before.len() as f32;
    assert!(speed < 0.01, "still moving at {speed} m/s on average");
}

#[test]
fn long_sleeves_follow_the_forearms() {
    // Past the elbow a sleeve starts around the forearm, not along the
    // upper arm's axis carried on (which left it off the arm, crumpled,
    // its seams open).
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": "t_shirt",
            "sizes": {"sleeve_cm": 50}}),
    );
    let (low, high) = drape(&s, 300);
    eprintln!("long sleeves: {low:.3}..{high:.3}");
    assert!(high > 1.35 && high < 1.50, "top at {high}");
}

#[test]
fn a_dress_hangs_from_the_shoulders() {
    let s = draft("dress");
    assert_eq!(s.project().panels().len(), 2);
    assert_eq!(s.project().seams().len(), 6);
    let (low, high) = drape(&s, 180);
    eprintln!("dress: {low:.3}..{high:.3}");
    assert!(high > 1.38 && high < 1.50, "top at {high}");
    assert!(low > 0.45 && low < 0.7, "hem at {low}");
}

#[test]
fn a_tote_bag_hangs_from_its_handles() {
    let s = draft("tote_bag");
    let p = s.project();
    assert_eq!(p.panels().len(), 2);
    assert_eq!(p.seams().len(), 5);
    assert_eq!(p.pins().len(), 2);
    let (low, high) = drape(&s, 180);
    eprintln!("tote: {low:.3}..{high:.3}");
    // Held at 1.25 m by the pins; 28 cm handles above a 40 cm bag.
    assert!((high - 1.25).abs() < 0.02, "top at {high}");
    assert!(low > 0.5 && low < 0.65, "bottom at {low}");
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
