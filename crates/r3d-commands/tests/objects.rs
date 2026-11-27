//! Projects that make an object rather than a garment worn (spec 36): no
//! body, a weightless drape, closed parts stuffed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use glam::Vec3;
use r3d_commands::{CommandError, Progress, Session, Step};
use r3d_core::Project;
use serde_json::{Value, json};

fn run(s: &mut Session, step: Value) -> Result<Value, CommandError> {
    s.run_step(&Step::from_value(step).unwrap())
}

fn code(e: &CommandError) -> String {
    r3d_core::Localizable::message(e).code.to_string()
}

#[test]
fn an_object_has_no_body() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    run(&mut s, json!({"type": "object.set"})).unwrap();
    let p = s.project();
    assert!(p.avatar().is_none(), "the body stayed");
    assert_eq!(p.object().map(|o| o.stuffing), Some(0.5));
    let summary = run(&mut s, json!({"type": "project.summary"})).unwrap();
    assert_eq!(summary["object"]["stuffing"], json!(0.5));
    // A body is refused, a size chart too (it would bring one).
    let body = json!({"type": "avatar.set", "body":
        {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}});
    let e = run(&mut s, body.clone()).unwrap_err();
    assert_eq!(code(&e), "invalid.object");
    let chart = json!({"type": "grading.use_chart", "chart": "women",
        "from": "36", "to": "40", "base": "38"});
    assert_eq!(code(&run(&mut s, chart).unwrap_err()), "invalid.object");
    // The stuffing is changed again, within its range.
    run(&mut s, json!({"type": "object.set", "stuffing": 0.8})).unwrap();
    assert_eq!(s.project().object().map(|o| o.stuffing), Some(0.8));
    let e = run(&mut s, json!({"type": "object.set", "stuffing": 1.5})).unwrap_err();
    assert_eq!(code(&e), "invalid.range");
    // A garment again: a body can be set; undo brings the object back.
    run(&mut s, json!({"type": "object.clear"})).unwrap();
    run(&mut s, body).unwrap();
    assert!(s.project().object().is_none() && s.project().avatar().is_some());
    run(&mut s, json!({"type": "history.undo"})).unwrap();
    run(&mut s, json!({"type": "history.undo"})).unwrap();
    assert_eq!(s.project().object().map(|o| o.stuffing), Some(0.8));
}

/// The cushion template in an object project stuffed at `stuffing`,
/// draped 3 s: how thick its middle is, how far its centre moved, the
/// largest seam gap and the sharpest crease between two triangles away
/// from its seams (degrees).
fn cushion(stuffing: f64) -> (f32, f32, f32, f32) {
    let mut s = Session::new(Project::new("t"));
    run(&mut s, json!({"type": "object.set", "stuffing": stuffing})).unwrap();
    run(
        &mut s,
        json!({"type": "template.apply", "template": "cushion"}),
    )
    .unwrap();
    let out = r3d_commands::drape::simulate(s.project(), 180, None, &Progress::default()).unwrap();
    let scene = &out.drape.scene;
    assert!(scene.weightless && !scene.ground);
    let x = &out.positions;
    assert!(x.iter().all(|p| p.is_finite()));
    let start: Vec3 = scene.particles.iter().map(|p| p.position).sum::<Vec3>() / x.len() as f32;
    let centre: Vec3 = x.iter().sum::<Vec3>() / x.len() as f32;
    // The two faces' particles nearest the middle of the cushion.
    let middle = |front: bool| {
        x.iter()
            .filter(|p| (p.z > centre.z) == front)
            .min_by(|a, b| {
                let d = |p: &Vec3| (p.x - centre.x).powi(2) + (p.y - centre.y).powi(2);
                d(a).total_cmp(&d(b))
            })
            .copied()
            .unwrap()
    };
    let depth = middle(true).z - middle(false).z;
    // Creases: triangles whose corners are all 8 cm or more inside the
    // cushion's outline (its rounded edge turns sharply).
    let half = 0.2 - 0.08;
    let inner = |p: Vec3| (p.x - centre.x).abs() < half && (p.y - centre.y).abs() < half;
    let mut normals: std::collections::HashMap<(u32, u32), Vec3> = Default::default();
    let mut crease = 0.0f32;
    for t in &scene.triangles {
        let v = t.vertices.map(|i| x[i as usize]);
        if !v.iter().all(|p| inner(*p)) {
            continue;
        }
        let n = (v[1] - v[0]).cross(v[2] - v[0]).normalize();
        for k in 0..3 {
            let (a, b) = (t.vertices[k], t.vertices[(k + 1) % 3]);
            if let Some(m) = normals.insert((a.min(b), a.max(b)), n) {
                crease = crease.max(n.dot(m).clamp(-1.0, 1.0).acos().to_degrees());
            }
        }
    }
    (
        depth,
        centre.distance(start),
        out.stats.max_seam_gap,
        crease,
    )
}

#[test]
fn a_stuffed_cushion_is_round_and_smooth() {
    let (flat, flat_moved, flat_gap, flat_crease) = cushion(0.0);
    let (round, moved, gap, crease) = cushion(0.5);
    assert!(round > flat + 0.05, "{round} m thick stuffed, {flat} m not");
    for (m, g, c) in [(flat_moved, flat_gap, flat_crease), (moved, gap, crease)] {
        assert!(m < 0.01, "it floated off {m} m");
        assert!(g < 0.003, "a seam is {g} m open");
        assert!(c < 20.0, "creased by {c}°");
    }
}
