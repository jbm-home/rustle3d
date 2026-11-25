//! A saved pattern added to another project (docs/specs/32): its
//! garments come with what belongs to them, drafted again for the
//! project's body where they were drafted from one.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{GarmentId, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, step: Value) -> Result<Value, CommandError> {
    s.run_step(&Step::from_value(step)?)
}

fn ok(s: &mut Session, step: Value) -> Value {
    run(s, step).unwrap()
}

fn woman() -> Value {
    json!({"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98})
}

fn man() -> Value {
    json!({"masculinity": 1, "height_cm": 182, "bust_cm": 104, "waist_cm": 90, "hips_cm": 102})
}

fn with_body(body: Value) -> Session {
    let mut s = Session::new(Project::new("t"));
    ok(&mut s, json!({"type": "avatar.set", "body": body}));
    s
}

/// The project as a saved pattern holds it.
fn saved(s: &Session) -> Value {
    serde_json::from_str(&r3d_core::file::to_json(s.project()).unwrap()).unwrap()
}

/// Width and height of each piece of `garment`, in drafting order (mm).
fn sizes(p: &Project, garment: GarmentId) -> Vec<(f64, f64)> {
    p.panels()
        .iter()
        .filter(|(_, pn)| pn.garment == garment)
        .map(|(id, _)| {
            let b = p.panel_bounding_box(*id).unwrap();
            (b.max.x - b.min.x, b.max.y - b.min.y)
        })
        .collect()
}

fn newest(p: &Project) -> GarmentId {
    p.newest_garment().unwrap()
}

#[test]
fn a_template_garment_is_drafted_again_for_the_new_body() {
    let mut a = with_body(woman());
    ok(
        &mut a,
        json!({"type": "template.apply", "template": "t_shirt"}),
    );
    let pattern = saved(&a);

    // A man's project that has trousers already.
    let mut b = with_body(man());
    ok(
        &mut b,
        json!({"type": "template.apply", "template": "trousers"}),
    );
    let before = b.project().clone();
    let out = ok(&mut b, json!({"type": "pattern.add", "pattern": pattern}));
    let created = out["created"].as_array().unwrap();
    assert!(created[0].as_str().unwrap().starts_with("garment:"));
    let p = b.project();
    assert_eq!(p.garments().len(), 2);
    let tee = newest(p);
    let trousers = before.newest_garment().unwrap();
    assert!(p.garments()[&tee].layer > p.garments()[&trousers].layer);
    assert_eq!(p.garments()[&tee].name, "T-shirt");
    // Sewn as before: as many seams as the T-shirt had, plus the trousers'.
    assert_eq!(
        p.seams().len(),
        a.project().seams().len() + before.seams().len()
    );

    // The pieces are the T-shirt drafted for the man.
    let mut c = with_body(man());
    ok(
        &mut c,
        json!({"type": "template.apply", "template": "t_shirt"}),
    );
    let want = sizes(c.project(), newest(c.project()));
    let got = sizes(p, tee);
    assert_eq!(got.len(), want.len());
    for ((gw, gh), (ww, wh)) in got.iter().zip(&want) {
        assert!(
            (gw - ww).abs() < 0.5 && (gh - wh).abs() < 0.5,
            "{got:?} vs {want:?}"
        );
    }
    let woman_sizes = sizes(a.project(), newest(a.project()));
    assert!(got[0].0 > woman_sizes[0].0 + 10.0);

    // One undo step.
    b.undo().unwrap();
    assert_eq!(b.project(), &before);
}

#[test]
fn edits_given_sizes_and_details_come_along() {
    let mut a = with_body(woman());
    ok(
        &mut a,
        json!({"type": "template.apply", "template": "straight_skirt",
            "sizes": {"waist_cm": 70, "length_cm": 60}}),
    );
    let front = *a.project().panels().keys().next().unwrap();
    let hem = a.project().panel(front).unwrap().contour[0];
    ok(
        &mut a,
        json!({"type": "notch.add", "edge": hem, "offset_mm": 50}),
    );
    ok(
        &mut a,
        json!({"type": "elastic.add", "side": {"edge": hem}, "ratio": 0.8}),
    );
    ok(
        &mut a,
        json!({"type": "variable.set", "name": "ease", "formula": "2 cm"}),
    );
    let pattern = saved(&a);
    let notches = a.project().notches().len();

    let mut b = with_body(man());
    ok(
        &mut b,
        json!({"type": "variable.set", "name": "ease", "formula": "5 cm"}),
    );
    ok(&mut b, json!({"type": "pattern.add", "pattern": pattern}));
    let p = b.project();
    // On an empty board the project's garment becomes the skirt.
    assert_eq!(p.garments().len(), 1);
    let g = &p.garments()[&newest(p)];
    assert_eq!(g.layer, 0);
    let t = g.template.as_ref().unwrap();
    // The waist given follows the body (+17 cm); the length stays.
    assert_eq!(t.sizes["waist_cm"], 87.0);
    assert_eq!(t.sizes["length_cm"], 60.0);
    assert_eq!(p.notches().len(), notches);
    assert_eq!(p.details().len(), 1);
    // The project's variable keeps its formula.
    assert_eq!(p.variables()["ease"], "5 cm");
}

#[test]
fn a_drawn_piece_comes_as_drawn_with_its_named_dimension() {
    let mut a = with_body(woman());
    let out = ok(
        &mut a,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 300, "height": 200}),
    );
    let panel = out["created"][0].as_str().unwrap().to_owned();
    let edge = out["created"][5].as_str().unwrap().to_owned();
    ok(
        &mut a,
        json!({"type": "constraint.add", "measure": true,
            "constraint": {"kind": "length", "edge": edge, "mm": 300, "name": "width"}}),
    );
    ok(
        &mut a,
        json!({"type": "button.add", "panel": panel, "at": [100, 100],
            "hole_panel": panel, "hole_at": [200, 100]}),
    );
    let pattern = saved(&a);

    let mut b = with_body(man());
    ok(&mut b, json!({"type": "pattern.add", "pattern": pattern}));
    let p = b.project();
    let id = *p.panels().keys().next().unwrap();
    let bb = p.panel_bounding_box(id).unwrap();
    assert!((bb.max.x - bb.min.x - 300.0).abs() < 1e-6);
    assert_eq!(p.buttons().len(), 1);
    let names: Vec<_> = p.constraints().values().filter_map(|c| c.name()).collect();
    assert_eq!(names, ["width"]);
}

#[test]
fn a_garment_added_to_a_size_range_is_graded() {
    let mut a = with_body(woman());
    ok(
        &mut a,
        json!({"type": "template.apply", "template": "a_line_skirt"}),
    );
    let pattern = saved(&a);

    let mut b = with_body(woman());
    ok(
        &mut b,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "40"}),
    );
    ok(&mut b, json!({"type": "pattern.add", "pattern": pattern}));
    let p = b.project();
    assert!(p.points().values().any(|pt| !pt.grade.is_empty()));
}

#[test]
fn an_older_file_is_migrated_and_a_bad_one_refused() {
    let v10: Value =
        serde_json::from_str(include_str!("../../r3d-core/tests/golden/v10/sample.json")).unwrap();
    let mut b = with_body(man());
    let out = ok(&mut b, json!({"type": "pattern.add", "pattern": v10}));
    assert!(out["created"].as_array().unwrap().len() > 1);

    let err = run(
        &mut b,
        json!({"type": "pattern.add", "pattern": {"format": "other"}}),
    )
    .unwrap_err();
    assert_eq!(
        r3d_core::Localizable::message(&err).code,
        "file.not_a_project"
    );
    let empty = saved(&Session::new(Project::new("e")));
    let err = run(&mut b, json!({"type": "pattern.add", "pattern": empty})).unwrap_err();
    assert_eq!(r3d_core::Localizable::message(&err).code, "invalid.empty");
}
