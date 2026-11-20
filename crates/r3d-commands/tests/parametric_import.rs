//! Seamly2D/Valentina patterns (spec 28): a quarter skirt drafted from a
//! measurement file, graded in a size range, drafted again for another
//! body; measurement files giving the body and the sizes. The files in
//! `tests/data/parametric/` were written by hand for these tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::path::{Path, PathBuf};

use r3d_commands::{Step, Workspace};
use r3d_core::{PanelId, Project};
use serde_json::{Value, json};

fn step(v: Value) -> Step {
    Step::from_value(v).unwrap()
}

fn data(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/parametric")
        .join(file)
}

/// A project with the skirt and its measurement file imported.
fn skirt(dir: &Path) -> (Workspace, Value) {
    let mut ws = Workspace::create(&dir.join("p.r3d"), "p").unwrap();
    ws.adopt(&data("body.vit"), "imports").unwrap();
    let rel = ws.adopt(&data("skirt.val"), "imports").unwrap();
    let out = ws
        .run_step(&step(json!({"type": "import.pattern", "path": rel})))
        .unwrap();
    (ws, out)
}

fn front(p: &Project) -> PanelId {
    *p.panels()
        .iter()
        .find(|(_, x)| x.name == "Front")
        .unwrap()
        .0
}

/// Width and height of a piece (mm).
fn size_of(p: &Project, id: PanelId) -> (f64, f64) {
    let b = p.panel_bounding_box(id).unwrap();
    (b.max.x - b.min.x, b.max.y - b.min.y)
}

/// The length of the waist: the piece's top edge.
fn waist(p: &Project, id: PanelId) -> f64 {
    let panel = p.panel(id).unwrap();
    let top = panel
        .contour
        .iter()
        .max_by(|a, b| {
            let y = |e| {
                let e = p.edge(e).unwrap();
                p.point(e.start).unwrap().pos.y + p.point(e.end).unwrap().pos.y
            };
            y(**a).total_cmp(&y(**b))
        })
        .unwrap();
    p.edge_length(*top).unwrap()
}

#[test]
fn a_skirt_is_drafted_from_its_measurement_file() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, out) = skirt(tmp.path());
    assert_eq!(out["panels"].as_array().unwrap().len(), 1, "{out}");
    assert_eq!(out["ignored"], 1, "the facing uses an unsupported curve");
    assert_eq!(out["unsupported"], json!(["spline:parallelCurve"]));
    let p = ws.session().project();
    let id = front(p);
    // Hips 96 / 4 + 1 cm, waist side to knee 104 - 46 cm.
    let (w, h) = size_of(p, id);
    assert!(
        (w - 250.0).abs() < 1e-6 && (h - 580.0).abs() < 1e-6,
        "{w} × {h}"
    );
    assert!((waist(p, id) - 185.0).abs() < 1e-6);
    let panel = p.panel(id).unwrap();
    assert_eq!(panel.seam_allowance_mm, 10.0);
    assert_eq!(p.internal_lines().len(), 1);
    assert_eq!(p.notches().len(), 1);
    let g = p.garments().get(&panel.garment).unwrap();
    assert_eq!(g.name, "skirt");
    let src = g.source.as_ref().unwrap();
    assert_eq!(src.file, "imports/skirt.val");
    assert!(src.measurements.is_some());
    assert_eq!(src.pieces.len(), 1);
}

#[test]
fn the_skirt_is_graded_in_every_size() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut ws, _) = skirt(tmp.path());
    // Women 36–44 drawn in 40: without an avatar, the chart's base body
    // (hips 100 cm) becomes it.
    ws.run_step(&step(json!({"type": "grading.use_chart", "chart": "women",
        "from": "36", "to": "44", "base": "40"})))
        .unwrap();
    let p = ws.session().project();
    let id = front(p);
    let g = p.grading().unwrap();
    for i in 0..g.sizes.len() {
        let hips = p.size_body(i).unwrap().hips_cm;
        let sized = p.at_size(i);
        let (w, h) = size_of(&sized, id);
        // The piece grows by a quarter of the hips from the base size's.
        let want = 250.0 + (hips - 100.0) * 10.0 / 4.0;
        assert!((w - want).abs() < 1e-6, "size {i}: {w} vs {want}");
        assert!((h - 580.0).abs() < 1e-6);
    }
}

#[test]
fn the_skirt_is_drafted_again_for_another_body() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut ws, _) = skirt(tmp.path());
    let id = front(ws.session().project());
    let garment = ws.session().project().panel(id).unwrap().garment;
    ws.run_step(&step(json!({"type": "avatar.set", "body":
        {"masculinity": 0, "height_cm": 168, "bust_cm": 92, "waist_cm": 80, "hips_cm": 104}})))
        .unwrap();
    let before = ws.session().project().clone();
    ws.run_step(&step(
        json!({"type": "garment.redraft", "garment": garment.to_string()}),
    ))
    .unwrap();
    let p = ws.session().project();
    let (w, h) = size_of(p, id);
    assert!(
        (w - 270.0).abs() < 1e-6 && (h - 580.0).abs() < 1e-6,
        "{w} × {h}"
    );
    assert!((waist(p, id) - 210.0).abs() < 1e-6);
    // The hip line follows the side.
    let line = p.internal_lines().values().next().unwrap();
    let end = line.segments.last().unwrap().end();
    let right = p.panel_bounding_box(id).unwrap().max.x;
    assert!((end.x - right).abs() < 1.0, "{} vs {right}", end.x);
    assert!(
        p.garments()[&garment]
            .source
            .as_ref()
            .unwrap()
            .body
            .is_some(),
        "the garment remembers the body"
    );
    ws.run_step(&step(json!({"type": "history.undo"}))).unwrap();
    assert_eq!(ws.session().project(), &before);
}

#[test]
fn a_multisize_file_gives_the_body_and_the_sizes() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&tmp.path().join("p.r3d"), "p").unwrap();
    let rel = ws.adopt(&data("sizes.vst"), "imports").unwrap();
    let out = ws
        .run_step(&step(json!({"type": "import.measurements", "path": rel})))
        .unwrap();
    assert_eq!(out["sizes"], json!(["40", "42", "44", "46", "48"]), "{out}");
    let p = ws.session().project();
    let body = p.avatar().unwrap().body;
    assert_eq!(
        (body.height_cm, body.bust_cm, body.waist_cm, body.hips_cm),
        (170.0, 88.0, 70.0, 96.0)
    );
    let g = p.grading().unwrap();
    assert_eq!(g.sizes[g.base].name, "44");
    assert_eq!(g.sizes[0].bust_cm, -8.0);
    assert_eq!(p.variables()["height_waist_side"], "1040 mm");
}

#[test]
fn a_missing_measurement_file_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&tmp.path().join("p.r3d"), "p").unwrap();
    let rel = ws.adopt(&data("skirt.val"), "imports").unwrap();
    let err = ws
        .run_step(&step(json!({"type": "import.pattern", "path": rel})))
        .unwrap_err();
    assert!(err.to_string().contains("body.vit"), "{err}");
}
