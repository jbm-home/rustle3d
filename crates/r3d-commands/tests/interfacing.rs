//! Interfacing (spec 31): areas, validation, what follows the piece, and
//! the composite cloth in the drape. That interfaced cloth stands where
//! bare cloth bends over is checked by the solvers' reference scenes
//! (`an_interfaced_band_stands_where_a_bare_one_falls`).
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
fn interfacings_are_checked_listed_and_follow_their_piece() {
    let mut s = Session::new(Project::new("t"));
    let r = run(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 400, "height": 300}),
    )
    .unwrap();
    let panel = r["created"][0].clone();
    let edges = r["created"].as_array().unwrap()[5..9].to_vec();
    // Edges must follow each other around the piece.
    let apart = run(
        &mut s,
        json!({"type": "interfacing.add", "panel": panel,
            "area": {"kind": "band", "edges": [edges[0], edges[2]], "width_mm": 40}}),
    )
    .unwrap_err();
    assert_eq!(apart.message().code, "invalid.not_joined");
    let band = run(
        &mut s,
        json!({"type": "interfacing.add", "panel": panel, "weight": "heavy",
            "area": {"kind": "band", "edges": [edges[0]], "width_mm": 40}}),
    )
    .unwrap()["created"][0]
        .clone();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    let i = &list["interfacings"][0];
    assert_eq!(i["id"], band);
    assert_eq!(i["area_mm2"], json!(16000.0));
    assert_eq!(i["weight"], json!("heavy"));
    assert_eq!(i["fabric"], json!("woven"));
    // Another one over it is refused; beside it, fine.
    let over = run(&mut s, json!({"type": "interfacing.add", "panel": panel})).unwrap_err();
    assert_eq!(over.message().code, "invalid.overlaps");
    run(
        &mut s,
        json!({"type": "interfacing.add", "panel": panel,
            "area": {"kind": "band", "edges": [edges[2]], "width_mm": 40}}),
    )
    .unwrap();
    // Inside a closed line; an open one is refused.
    let open = run(
        &mut s,
        json!({"type": "internal_line.add", "panel": panel, "start": [100, 100],
            "segments": [{"kind": "line", "to": [200, 100]}, {"kind": "line", "to": [200, 200]}]}),
    )
    .unwrap()["created"][0]
        .clone();
    let refused = run(
        &mut s,
        json!({"type": "interfacing.add", "panel": panel,
            "area": {"kind": "inside", "line": open}}),
    )
    .unwrap_err();
    assert_eq!(refused.message().code, "invalid.not_closed");
    let closed = run(
        &mut s,
        json!({"type": "internal_line.add", "panel": panel, "start": [100, 100],
            "segments": [{"kind": "line", "to": [200, 100]}, {"kind": "line", "to": [200, 200]},
                {"kind": "line", "to": [100, 200]}, {"kind": "line", "to": [100, 100]}]}),
    )
    .unwrap()["created"][0]
        .clone();
    run(
        &mut s,
        json!({"type": "interfacing.add", "panel": panel, "fabric": "knit",
            "area": {"kind": "inside", "line": closed}}),
    )
    .unwrap();
    assert_eq!(s.project().details().len(), 3);
    // Its line gone, the interfacing goes with it, in the same step.
    run(
        &mut s,
        json!({"type": "internal_line.remove", "line": closed}),
    )
    .unwrap();
    assert_eq!(s.project().details().len(), 2);
    s.undo().unwrap();
    assert_eq!(s.project().details().len(), 3);
    run(
        &mut s,
        json!({"type": "interfacing.set", "interfacing": band, "width_mm": 60}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    assert_eq!(list["interfacings"][0]["area_mm2"], json!(24000.0));
}

#[test]
fn interfaced_cloth_is_heavier_and_stiffer_in_the_drape() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": "shirt"}),
    )
    .unwrap();
    let before = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    // The shirt's collar comes interfaced (medium): make it heavy.
    let collar = common::piece(&s, "Collar");
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    let on_collar = list["interfacings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["panel"] == collar.panel)
        .unwrap()["id"]
        .clone();
    run(
        &mut s,
        json!({"type": "interfacing.set", "interfacing": on_collar, "weight": "heavy"}),
    )
    .unwrap();
    // A band inside the back's hem: its edge is a line of the mesh.
    let back = common::piece(&s, "Back");
    run(
        &mut s,
        json!({"type": "interfacing.add", "panel": back.panel, "weight": "light",
            "area": {"kind": "band", "edges": [back.edges[0]], "width_mm": 50}}),
    )
    .unwrap();
    let drape = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    assert_eq!(drape.scene.fabrics.len(), before.scene.fabrics.len() + 2);
    let base = drape.scene.fabrics[0];
    let heavy = drape
        .scene
        .fabrics
        .iter()
        .max_by(|a, b| a.bending.total_cmp(&b.bending))
        .unwrap();
    assert!(heavy.bending > 10.0 * base.bending, "{heavy:?} vs {base:?}");
    assert!(heavy.density > base.density && heavy.thickness > base.thickness);
    // Every collar triangle is interfaced.
    let collar_id: r3d_core::PanelId = collar.panel.as_str().unwrap().parse().unwrap();
    let piece = drape.pieces.iter().find(|p| p.panel == collar_id).unwrap();
    let stiff = drape.scene.fabrics.iter().position(|f| f == heavy).unwrap();
    let mut collar_tris = drape
        .scene
        .triangles
        .iter()
        .filter(|t| piece.particles.contains(&(t.vertices[0] as usize)));
    assert!(collar_tris.clone().count() > 0);
    assert!(collar_tris.all(|t| usize::from(t.fabric) == stiff));
    // The back's band ends on a row of points 50 mm above its hem.
    let back_id: r3d_core::PanelId = back.panel.as_str().unwrap().parse().unwrap();
    let piece = drape.pieces.iter().find(|p| p.panel == back_id).unwrap();
    let hem: r3d_core::EdgeId = back.edges[0].as_str().unwrap().parse().unwrap();
    let p = s.project();
    let hem_y = p.point(p.edge(hem).unwrap().start).unwrap().pos.y;
    let band_y = ((hem_y + 50.0) / 1000.0) as f32;
    let on_line = drape.scene.particles[piece.particles.clone()]
        .iter()
        .filter(|p| (p.rest.y - band_y).abs() < 1e-4)
        .count();
    assert!(on_line > 10, "{on_line} points on the band's edge");
}
