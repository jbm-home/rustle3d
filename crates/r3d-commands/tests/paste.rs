//! Semantic copy and paste: pieces come with their seams, notches,
//! internal lines, pins, grade rules and mirror links.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, step: Value) -> Result<Value, CommandError> {
    s.run_step(&Step::from_value(step)?)
}

/// A graded project with the T-shirt: front, back and sleeves, sewn.
fn t_shirt() -> Session {
    let mut s = Session::new(Project::new("t"));
    run(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "40"}),
    )
    .unwrap();
    run(
        &mut s,
        json!({"type": "template.apply", "template": "t_shirt"}),
    )
    .unwrap();
    s
}

fn ids(v: &Value, prefix: &str) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x.as_str())
        .filter(|x| x.starts_with(prefix))
        .map(str::to_owned)
        .collect()
}

#[test]
fn two_sewn_pieces_paste_with_their_seam() {
    let mut s = t_shirt();
    let seams = run(&mut s, json!({"type": "seam.list"})).unwrap();
    // Two pieces sewn together: the first seam's.
    let seam = &seams[0];
    let edge_panel = |s: &mut Session, edge: &Value| {
        let panels = run(s, json!({"type": "panel.list"})).unwrap();
        panels
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap().to_owned())
            .find(|id| {
                let got = run(s, json!({"type": "panel.get", "panel": id})).unwrap();
                got["edges"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| &e["id"] == edge)
            })
            .unwrap()
    };
    let a = edge_panel(&mut s, &seam["a"]["edge"]);
    let b = edge_panel(&mut s, &seam["b"]["edge"]);
    assert_ne!(a, b);
    let clip = run(&mut s, json!({"type": "panel.copy", "panels": [a, b]})).unwrap();
    let before = s.project().clone();

    let out = run(&mut s, json!({"type": "panel.paste", "clip": clip})).unwrap();
    let created = &out["created"];
    let new_panels = ids(created, "panel:");
    assert_eq!(new_panels.len(), 2);
    // Every seam between the two came along; none to the other pieces.
    let between = before
        .seams()
        .values()
        .filter(|sm| {
            [sm.a.edge, sm.b.edge].iter().all(|e| {
                let p = before.edge(*e).unwrap().panel.to_string();
                p == a || p == b
            })
        })
        .count();
    assert!(between >= 1);
    assert_eq!(ids(created, "seam:").len(), between);
    let p = s.project();
    assert_eq!(p.seams().len(), before.seams().len() + between);
    // Grade rules came along (same sizes), so every size is drawn.
    let new: r3d_core::PanelId = new_panels[0].parse().unwrap();
    assert!(
        p.points()
            .values()
            .any(|pt| pt.panel == new && !pt.grade.is_empty())
    );
    // Pasted beside the originals: they would start in the same place
    // around the body, so they are left unplaced.
    assert!(p.panel(new).unwrap().placement.is_none());
    // To the right of every other piece.
    let right = before
        .panels()
        .keys()
        .filter_map(|id| before.panel_bounding_box(*id))
        .map(|b| b.max.x)
        .fold(f64::MIN, f64::max);
    assert!(p.panel_bounding_box(new).unwrap().min.x > right);
    p.check_integrity().unwrap();
}

#[test]
fn pieces_paste_into_another_project() {
    let mut src = t_shirt();
    let panels: Vec<String> = src
        .project()
        .panels()
        .keys()
        .map(ToString::to_string)
        .collect();
    let clip = run(&mut src, json!({"type": "panel.copy", "panels": panels})).unwrap();
    // Through text, as on the clipboard.
    let clip: Value = serde_json::from_str(&clip.to_string()).unwrap();

    let mut dst = Session::new(Project::new("other"));
    run(
        &mut dst,
        json!({"type": "panel.paste", "clip": clip, "offset": [0, 0]}),
    )
    .unwrap();
    let (a, b) = (src.project(), dst.project());
    assert_eq!(b.panels().len(), a.panels().len());
    assert_eq!(b.seams().len(), a.seams().len());
    assert_eq!(b.notches().len(), a.notches().len());
    // Placed around the body as in the source: ready to drape.
    assert!(b.panels().values().all(|p| p.placement.is_some()));
    // No size range here: the grade rules are dropped.
    assert!(b.points().values().all(|p| p.grade.is_empty()));
    // The same outlines, at the same place.
    let area = |p: &Project| -> f64 {
        p.panels()
            .keys()
            .filter_map(|id| p.panel_bounding_box(*id))
            .map(|b| (b.max - b.min).element_product())
            .sum()
    };
    assert!((area(a) - area(b)).abs() < 1e-6);
}

#[test]
fn a_bad_clip_changes_nothing() {
    let mut s = t_shirt();
    let before = s.project().clone();
    for (clip, code) in [
        (json!({"nope": 1}), "malformed"),
        (json!([1, 2]), "malformed"),
    ] {
        let err = run(&mut s, json!({"type": "panel.paste", "clip": clip})).unwrap_err();
        assert_eq!(err.message().code, code);
    }
    let panel = s.project().panels().keys().next().unwrap().to_string();
    let mut clip = run(&mut s, json!({"type": "panel.copy", "panels": [panel]})).unwrap();
    clip["version"] = json!(1);
    let err = run(&mut s, json!({"type": "panel.paste", "clip": clip})).unwrap_err();
    assert_eq!(err.message().code, "malformed");
    assert_eq!(s.project(), &before);
    let err = run(&mut s, json!({"type": "panel.copy", "panels": []})).unwrap_err();
    assert_eq!(err.message().code, "invalid.empty");
}

#[test]
fn a_mirror_link_pastes_when_its_source_comes_too() {
    let mut s = Session::new(Project::new("t"));
    let rect = run(
        &mut s,
        json!({"type": "panel.create_rect", "width": 100, "height": 200}),
    )
    .unwrap();
    let source = rect["created"][0].as_str().unwrap().to_owned();
    let copy = run(&mut s, json!({"type": "panel.mirror", "panel": source})).unwrap();
    let copy = copy["created"][0].as_str().unwrap().to_owned();

    let both = run(
        &mut s,
        json!({"type": "panel.copy", "panels": [source, copy]}),
    )
    .unwrap();
    run(&mut s, json!({"type": "panel.paste", "clip": both})).unwrap();
    let linked = |p: &Project| {
        p.panels()
            .values()
            .filter(|p| p.mirror_of.is_some())
            .count()
    };
    assert_eq!(linked(s.project()), 2);

    // The copy alone: it is no longer linked (its source stays behind).
    let alone = run(&mut s, json!({"type": "panel.copy", "panels": [copy]})).unwrap();
    run(&mut s, json!({"type": "panel.paste", "clip": alone})).unwrap();
    assert_eq!(linked(s.project()), 2);
    s.project().check_integrity().unwrap();
}
