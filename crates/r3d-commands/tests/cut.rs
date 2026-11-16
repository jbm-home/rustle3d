#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Cutting a panel in two: the pieces, the seam along the cut, what moves
//! with them, and a drape that matches the uncut garment.

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Session, Step};
use r3d_core::{PanelId, Project};
use r3d_sim::Solver;
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Value {
    s.run_step(&Step::from_value(v).unwrap()).unwrap()
}

fn fails(s: &mut Session, v: Value) -> String {
    let before = s.project().clone();
    let err = s.run_step(&Step::from_value(v).unwrap()).unwrap_err();
    assert_eq!(s.project(), &before, "a failed cut changed the project");
    err.to_string()
}

fn area(s: &Session, panel: &Value) -> f64 {
    let id: PanelId = serde_json::from_value(panel.clone()).unwrap();
    s.project().panel_area(id).unwrap()
}

fn seam_lengths(s: &mut Session) -> Vec<(f64, f64)> {
    run(s, json!({"type": "seam.list"}))
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                x["a"]["length_mm"].as_f64().unwrap(),
                x["b"]["length_mm"].as_f64().unwrap(),
            )
        })
        .collect()
}

#[test]
fn a_straight_cut_makes_two_pieces_sewn_together() {
    let mut s = Session::new(Project::new("t"));
    let out = run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Front", "width": 400, "height": 600}),
    );
    let panel = out["created"][0].clone();
    let cut = run(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [150, -50],
            "segments": [{"kind": "line", "to": [150, 650]}]}),
    );
    let created = cut["created"].as_array().unwrap();
    assert_eq!(created.len(), 2, "the new panel and one seam: {cut}");
    let new = created[0].clone();
    // The larger piece keeps the panel.
    assert!((area(&s, &panel) - 250.0 * 600.0).abs() < 1e-6);
    // Without a panel, the one the line crosses (the other is beside it).
    run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Back", "origin": [600, 0], "width": 400, "height": 600}),
    );
    run(
        &mut s,
        json!({"type": "panel.cut", "start": [800, -50], "segments": [{"kind": "line", "to": [800, 650]}]}),
    );
    assert_eq!(s.project().panels().len(), 4);
    // A line across both pieces is ambiguous.
    fails(
        &mut s,
        json!({"type": "panel.cut", "start": [-50, 300], "segments": [{"kind": "line", "to": [1050, 300]}]}),
    );
    assert!((area(&s, &new) - 150.0 * 600.0).abs() < 1e-6);
    assert_eq!(common::piece(&s, "Front 2").panel, new);
    let seams = seam_lengths(&mut s);
    assert_eq!(seams.len(), 2);
    assert!((seams[0].0 - 600.0).abs() < 1e-6 && (seams[0].1 - 600.0).abs() < 1e-6);
    s.project().check_integrity().unwrap();
}

#[test]
fn a_curved_cut_keeps_the_area_and_the_curve() {
    let mut s = Session::new(Project::new("t"));
    let out = run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Front", "width": 400, "height": 600}),
    );
    let panel = out["created"][0].clone();
    let whole = area(&s, &panel);
    // A princess line: from above the top, curving out, to below the hem.
    let cut = run(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [120, 700], "name": "Side",
            "segments": [{"kind": "cubic", "c1": [120, 400], "c2": [220, 300], "to": [220, -100]}]}),
    );
    let new = cut["created"][0].clone();
    let total = area(&s, &panel) + area(&s, &new);
    assert!((total - whole).abs() < 1.0, "{total} vs {whole}");
    // One curved edge each side, sewn with equal lengths.
    let seams = seam_lengths(&mut s);
    assert_eq!(seams.len(), 1);
    assert!((seams[0].0 - seams[0].1).abs() < 1e-6);
    assert!(seams[0].0 > 600.0);
    let side = common::piece(&s, "Side");
    let kinds: Vec<String> = side
        .edges
        .iter()
        .map(|e| {
            let id = serde_json::from_value(e.clone()).unwrap();
            format!("{:?}", s.project().edge(id).unwrap().curve)
        })
        .collect();
    assert!(kinds.iter().any(|k| k.starts_with("Cubic")), "{kinds:?}");
    // Undo puts the whole panel back.
    s.undo().unwrap();
    assert_eq!(s.project().panels().len(), 1);
    assert!((area(&s, &panel) - whole).abs() < 1e-9);
}

#[test]
fn notches_lines_pins_and_seams_follow_their_piece() {
    let mut s = Session::new(Project::new("t"));
    let front = run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Front", "width": 400, "height": 600}),
    );
    let back = run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Back", "origin": [600, 0], "width": 400, "height": 600}),
    );
    let panel = front["created"][0].clone();
    // Edges: bottom, right, top, left. Sew the front's left side to the
    // back's right side; a notch on the front's bottom, near the left.
    let (bottom, left) = (front["created"][5].clone(), front["created"][8].clone());
    run(
        &mut s,
        json!({"type": "seam.add", "a": {"edge": left}, "b": {"edge": back["created"][6]}}),
    );
    run(
        &mut s,
        json!({"type": "notch.add", "edge": bottom, "offset_mm": 50}),
    );
    run(
        &mut s,
        json!({"type": "internal_line.add", "panel": panel, "start": [20, 500],
            "segments": [{"kind": "line", "to": [80, 500]}]}),
    );
    run(
        &mut s,
        json!({"type": "pin.add", "panel": panel, "at": [30, 30], "position": [0.0, 1.0, 0.1]}),
    );
    // A horizontal cut at 200 mm, across the sewn left side.
    let cut = run(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [-50, 200],
            "segments": [{"kind": "line", "to": [450, 200]}]}),
    );
    let new = cut["created"][0].clone();
    let new_id: PanelId = serde_json::from_value(new.clone()).unwrap();
    let kept_id: PanelId = serde_json::from_value(panel.clone()).unwrap();
    let p = s.project();
    // The bottom strip is the smaller piece: it is the new one.
    assert!((p.panel_area(new_id).unwrap() - 400.0 * 200.0).abs() < 1e-6);
    let notch = p.notches().values().next().unwrap();
    assert_eq!(p.edge(notch.edge).unwrap().panel, new_id);
    assert_eq!(p.pins().values().next().unwrap().panel, new_id);
    assert_eq!(p.internal_lines().values().next().unwrap().panel, kept_id);
    // The side seam was split in two (200 + 400), plus the cut's seam.
    let mut seams = seam_lengths(&mut s);
    seams.sort_by(|a, b| a.0.total_cmp(&b.0));
    let lengths: Vec<_> = seams.iter().map(|(a, b)| (a.round(), b.round())).collect();
    assert_eq!(lengths, [(200.0, 200.0), (400.0, 400.0), (400.0, 400.0)]);
    s.project().check_integrity().unwrap();
}

#[test]
fn cuts_that_do_not_cross_twice_are_refused() {
    let mut s = Session::new(Project::new("t"));
    let out = run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Front", "width": 400, "height": 600}),
    );
    let panel = out["created"][0].clone();
    // Ends inside the panel.
    let err = fails(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [150, -50],
            "segments": [{"kind": "line", "to": [150, 300]}]}),
    );
    assert!(err.contains("exactly twice"), "{err}");
    // Misses it.
    fails(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [500, -50],
            "segments": [{"kind": "line", "to": [500, 650]}]}),
    );
    // Crosses four times.
    fails(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [100, -50],
            "segments": [{"kind": "line", "to": [100, 650]}, {"kind": "line", "to": [300, 650]},
                {"kind": "line", "to": [300, -50]}]}),
    );
    // A panel with a linked mirror copy.
    run(&mut s, json!({"type": "panel.mirror", "panel": panel}));
    let err = fails(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [150, -50],
            "segments": [{"kind": "line", "to": [150, 650]}]}),
    );
    assert!(err.contains("mirror"), "{err}");
}

#[test]
fn a_cut_through_a_corner_uses_the_corner() {
    let mut s = Session::new(Project::new("t"));
    let out = run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Front", "width": 400, "height": 400}),
    );
    let panel = out["created"][0].clone();
    let points = s.project().points().len();
    // The diagonal, through two corners: no new outline point, only the
    // corners' copies on the new piece.
    run(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [-50, -50],
            "segments": [{"kind": "line", "to": [450, 450]}]}),
    );
    assert_eq!(s.project().points().len(), points + 2);
    assert_eq!(s.project().panels().len(), 2);
    s.project().check_integrity().unwrap();
}

/// Waist height, hem height and widest reach of the draped skirt (m).
fn drape(s: &Session) -> (f32, f32, f32) {
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let collider = drape.scene.collider.clone().unwrap();
    let mut solver = Solver::new(drape.scene);
    for _ in 0..240 {
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
    let top = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    let hem = p.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let reach = p
        .iter()
        .map(|v| (v.x * v.x + v.z * v.z).sqrt())
        .fold(0.0, f32::max);
    (top, hem, reach)
}

#[test]
fn a_skirt_cut_along_a_princess_line_drapes_like_the_whole() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    let front = common::a_line_skirt(&mut s);
    let (top, hem, reach) = drape(&s);

    let id: PanelId = serde_json::from_value(front.panel.clone()).unwrap();
    let bb = s.project().panel_bounding_box(id).unwrap();
    let x = bb.min.x + (bb.max.x - bb.min.x) * 0.3;
    run(
        &mut s,
        json!({"type": "panel.cut", "panel": front.panel, "start": [x, bb.max.y + 50.0],
            "segments": [{"kind": "cubic", "c1": [x, bb.max.y - 200.0],
                "c2": [x - 40.0, bb.min.y + 200.0], "to": [x - 40.0, bb.min.y - 50.0]}]}),
    );
    let (cut_top, cut_hem, cut_reach) = drape(&s);
    // Same place on the body, same flare. The whole front stretches more
    // under its own weight than the two halves (the solver's known creep,
    // docs/physics.md), so the cut skirt's hem may hang higher, not lower.
    assert!((cut_top - top).abs() < 0.03, "waist {cut_top} vs {top}");
    assert!(
        (cut_reach - reach).abs() < 0.03,
        "reach {cut_reach} vs {reach}"
    );
    assert!(cut_hem > hem - 0.01, "hem {cut_hem} vs {hem}");
}

#[test]
fn a_curved_piece_drawn_clockwise_keeps_its_shape() {
    let mut s = Session::new(Project::new("t"));
    // Clockwise: up the left side, a curved top, down the right side.
    let out = run(
        &mut s,
        json!({"type": "panel.create_path", "start": [0, 0], "segments": [
            {"kind": "line", "to": [0, 300]},
            {"kind": "cubic", "c1": [50, 400], "c2": [250, 400], "to": [300, 300]},
            {"kind": "line", "to": [300, 0]},
            {"kind": "line", "to": [0, 0]}
        ]}),
    );
    let id: PanelId = serde_json::from_value(out["created"][0].clone()).unwrap();
    let p = s.project();
    assert_eq!(p.panel(id).unwrap().contour.len(), 4);
    let area = p.panel_area(id).unwrap();
    // The rectangle plus the arch above it (3/4 of the control height
    // times the mean width, for this symmetric cubic: 300 × 75 × 0.8).
    assert!(
        area > 300.0 * 300.0 + 15_000.0 && area < 300.0 * 300.0 + 20_000.0,
        "{area}"
    );
    let bb = p.panel_bounding_box(id).unwrap();
    assert!((bb.max.y - 375.0).abs() < 0.1, "{bb:?}");
    // Two points only make a piece when an edge is curved.
    let err = s
        .run_step(
            &Step::from_value(json!({"type": "panel.create_path", "start": [0, 0],
                "segments": [{"kind": "line", "to": [100, 0]}]}))
            .unwrap(),
        )
        .unwrap_err();
    assert!(err.to_string().contains("at least 3"), "{err}");
    run(
        &mut s,
        json!({"type": "panel.create_path", "start": [0, 0], "segments": [
            {"kind": "cubic", "c1": [30, 80], "c2": [70, 80], "to": [100, 0]},
            {"kind": "cubic", "c1": [70, -80], "c2": [30, -80], "to": [0, 0]}
        ]}),
    );
}

#[test]
fn the_pieces_of_a_placed_panel_start_where_it_was() {
    let mut s = Session::new(Project::new("t"));
    let out = run(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Front", "width": 400, "height": 600}),
    );
    let panel = out["created"][0].clone();
    run(
        &mut s,
        json!({"type": "panel.place", "panel": panel, "angle_deg": 10, "top_cm": 100}),
    );
    // A slanted cut that leaves the right piece shorter at the top.
    let cut = run(
        &mut s,
        json!({"type": "panel.cut", "panel": panel, "start": [100, -50],
            "segments": [{"kind": "line", "to": [450, 500]}]}),
    );
    let new: PanelId = serde_json::from_value(cut["created"][0].clone()).unwrap();
    let kept: PanelId = serde_json::from_value(panel).unwrap();
    let p = s.project();
    let (pk, pn) = (
        p.panel(kept).unwrap().placement.unwrap(),
        p.panel(new).unwrap().placement.unwrap(),
    );
    assert_eq!((pk.angle_deg, pn.angle_deg), (10.0, 10.0));
    // The kept piece reaches the whole's top; the new one, moved 20 mm
    // away (down and right), keeps its abscissa and height on the body.
    assert_eq!(pk.centre_x_mm, Some(200.0));
    assert_eq!(pk.top_cm, 100.0);
    // The new piece is the triangle under the cut, its top 421.4 mm up
    // the right side: 178.6 mm below the whole's top.
    assert!(
        (pn.top_cm - (100.0 - (600.0 - 421.428_571) / 10.0)).abs() < 1e-3,
        "{pn:?}"
    );
    // It was moved away (right and down); its abscissa moved with it.
    let bb = p.panel_bounding_box(new).unwrap();
    let dx = bb.max.x - 400.0;
    assert!(dx > 5.0 && bb.min.y < -5.0, "{bb:?}");
    assert!(
        (pn.centre_x_mm.unwrap() - (200.0 + dx)).abs() < 1e-9,
        "{pn:?}"
    );
}
