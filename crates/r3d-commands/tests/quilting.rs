//! Quilting and padding (spec 31): validation, its lines, the quilted
//! cloth in the drape and its puffs.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, build};
use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

#[test]
fn quiltings_are_checked_and_listed() {
    let mut s = Session::new(Project::new("t"));
    let r = run(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 400, "height": 300}),
    )
    .unwrap();
    let panel = r["created"][0].clone();
    let tight = run(
        &mut s,
        json!({"type": "quilting.add", "panel": panel, "spacing_mm": 5}),
    )
    .unwrap_err();
    assert_eq!(tight.message().code, "invalid.range");
    // Channels along the grain (the rectangle's runs up), 50 mm apart.
    let id = run(
        &mut s,
        json!({"type": "quilting.add", "panel": panel, "pattern": "channels"}),
    )
    .unwrap()["created"][0]
        .clone();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    let q = &list["quiltings"][0];
    assert_eq!(q["id"], id);
    assert_eq!(q["area_mm2"], json!(120000.0));
    assert_eq!(q["loft_mm"], json!(8.0));
    // Seven lines across 400 mm (none on the sides: they are the
    // outline), each 0.5 mm short of the top and bottom.
    let stitched = q["stitched_mm"].as_f64().unwrap();
    assert!((stitched - 7.0 * 299.0).abs() < 1.0, "{stitched}");
    // One quilting per area.
    let again = run(&mut s, json!({"type": "quilting.add", "panel": panel})).unwrap_err();
    assert_eq!(again.message().code, "invalid.overlaps");
    run(
        &mut s,
        json!({"type": "quilting.set", "quilting": id, "pattern": "diamonds", "angle_deg": 45}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    assert!(list["quiltings"][0]["stitched_mm"].as_f64().unwrap() > stitched);
}

#[test]
fn quilted_cloth_is_padded_folds_on_its_lines_and_puffs() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": "shirt"}),
    )
    .unwrap();
    let back = common::piece(&s, "Back");
    run(
        &mut s,
        json!({"type": "quilting.add", "panel": back.panel, "spacing_mm": 60,
            "loft_mm": 10, "padding_gsm": 150}),
    )
    .unwrap();
    let drape = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    let base = drape.scene.fabrics[0];
    // The heaviest: the shirt's own details (interfacing, topstitching)
    // add lighter ones.
    let quilted = drape
        .scene
        .fabrics
        .iter()
        .max_by(|a, b| a.density.total_cmp(&b.density))
        .unwrap();
    // Shell and lining with the padding between.
    assert!((quilted.density - (2.0 * base.density + 0.15)).abs() < 1e-6);
    assert!((quilted.thickness - (base.thickness + 0.01)).abs() < 1e-6);
    // The stitched lines fold with the shell and lining alone.
    assert!(!drape.scene.creases.is_empty());
    assert!(
        drape
            .scene
            .creases
            .iter()
            .all(|(_, b)| (b - 2.0 * base.bending).abs() < 1e-9)
    );
    assert!(quilted.bending > 10.0 * base.bending);
    // The puffs: up to the loft between the lines, none on them.
    let back_id: r3d_core::PanelId = back.panel.as_str().unwrap().parse().unwrap();
    let piece = drape.pieces.iter().find(|p| p.panel == back_id).unwrap();
    let puff = &drape.puff[piece.particles.clone()];
    let most = puff.iter().copied().fold(0.0f32, f32::max);
    assert!(most > 0.009 && most <= 0.010 + 1e-6, "{most}");
    let on_lines = drape
        .scene
        .creases
        .iter()
        .map(|([a, _], _)| drape.puff[*a as usize])
        .fold(0.0f32, f32::max);
    assert!(on_lines < 1e-6, "{on_lines}");
    // Nothing puffs outside the quilting.
    let others = drape
        .pieces
        .iter()
        .filter(|p| p.panel != back_id)
        .flat_map(|p| drape.puff[p.particles.clone()].iter())
        .copied()
        .fold(0.0f32, f32::max);
    assert_eq!(others, 0.0);
    // Drawn, the puffs stand out along the cloth's normal.
    let start: Vec<_> = drape.scene.particles.iter().map(|p| p.position).collect();
    let puffed = drape.puffed(&start);
    let moved = start
        .iter()
        .zip(&puffed)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0f32, f32::max);
    assert!((moved - most).abs() < 1e-5);
}
