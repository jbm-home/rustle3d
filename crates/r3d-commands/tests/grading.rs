#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Sizes and grading (M22): a size range, grade rules on points, templates
//! graded to every size, the size draped in 3D, graded exports.

mod common;

use r3d_commands::{Session, Step};
use r3d_core::{Localizable, Project, file};
use r3d_geom::DVec2;
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, r3d_commands::CommandError> {
    s.run_step(&Step::from_value(v).unwrap())
}

fn ok(s: &mut Session, v: Value) -> Value {
    run(s, v).unwrap()
}

/// Women's sizes 36 to 52 based on 40, on the women's M body.
fn ranged() -> Session {
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    ok(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "52", "base": "40"}),
    );
    s
}

fn width(p: &Project, name: &str) -> f64 {
    let (id, _) = p.panels().iter().find(|(_, pn)| pn.name == name).unwrap();
    let b = p.panel_bounding_box(*id).unwrap();
    b.max.x - b.min.x
}

#[test]
fn a_chart_sets_the_sizes_and_their_bodies() {
    let s = ranged();
    let g = s.project().grading().unwrap();
    let names: Vec<&str> = g.sizes.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        ["36", "38", "40", "42", "44", "46", "48", "50", "52"]
    );
    assert_eq!((g.base, g.shown), (2, 2));
    // The bodies keep the avatar's and add the chart's differences.
    let p = s.project();
    assert_eq!(p.size_body(2).unwrap().bust_cm, 90.0);
    assert_eq!(p.size_body(0).unwrap().bust_cm, 82.0);
    assert_eq!(p.size_body(8).unwrap().bust_cm, 120.0);
    let got = s
        .query(&serde_json::from_value(json!({"type": "grading.get"})).unwrap())
        .unwrap();
    assert_eq!(got["base"], "40");
    assert_eq!(got["sizes"][8]["body"]["hips_cm"], 128.0);
}

#[test]
fn a_shirt_is_graded_to_every_size() {
    let mut s = ranged();
    ok(
        &mut s,
        json!({"type": "template.apply", "template": "shirt"}),
    );
    let p = s.project().clone();
    let g = p.grading().unwrap();
    // Every size is a shirt drafted for that body: wider from size to size.
    let backs: Vec<f64> = (0..g.sizes.len())
        .map(|i| width(&p.at_size(i), "Back"))
        .collect();
    assert!(backs.windows(2).all(|w| w[1] > w[0]), "{backs:?}");
    // The base size is the pattern as drawn.
    assert_eq!(width(&p.at_size(g.base), "Back"), width(&p, "Back"));
    // Size 52 is the template drafted directly for its body.
    let mut direct = Session::new(Project::new("Untitled"));
    let body = p.size_body(8).unwrap();
    ok(&mut direct, json!({"type": "avatar.set", "body": body}));
    ok(
        &mut direct,
        json!({"type": "template.apply", "template": "shirt"}),
    );
    // Each piece of size 52 is the one drafted for that body, only placed
    // elsewhere on the board (the sizes nest on each piece's centre).
    let big = p.at_size(8);
    let mut shifts: std::collections::BTreeMap<_, DVec2> = Default::default();
    for (id, point) in direct.project().points() {
        let graded = big.point(*id).unwrap();
        let shift = *shifts.entry(point.panel).or_insert(graded.pos - point.pos);
        assert!(
            (graded.pos - point.pos).distance(shift) < 1e-6,
            "{id}: {:?} vs {:?}",
            graded.pos,
            point.pos
        );
        assert_eq!(shift.y, 0.0);
    }
    // Curves too, mirror copies included.
    for (id, edge) in direct.project().edges() {
        if let (
            r3d_core::EdgeCurve::Cubic { c1, c2 },
            r3d_core::EdgeCurve::Cubic { c1: g1, c2: g2 },
        ) = (&edge.curve, &big.edge(*id).unwrap().curve)
        {
            let shift = shifts[&edge.panel];
            assert!((*g1 - *c1).distance(shift) < 1e-6, "{id}");
            assert!((*g2 - *c2).distance(shift) < 1e-6, "{id}");
        }
    }
    // Seams still match and no piece crosses itself in any size.
    let check = s
        .query(&serde_json::from_value(json!({"type": "grading.check"})).unwrap())
        .unwrap();
    assert_eq!(check["ok"], true, "{check:#}");
    p.check_integrity().unwrap();
}

#[test]
fn a_step_grades_a_point_and_mirror_copies_follow() {
    let mut s = ranged();
    let front = common::a_line_skirt(&mut s);
    // The template graded every point; start again from a plain rule.
    let corner = front.points[1].clone();
    ok(
        &mut s,
        json!({"type": "point.set_grade", "point": corner, "step": [5, -2]}),
    );
    let p = s.project();
    let id = p
        .points()
        .keys()
        .find(|k| json!(k.to_string()) == corner)
        .copied()
        .unwrap();
    let point = p.point(id).unwrap();
    assert_eq!(point.offset(2), DVec2::ZERO);
    assert_eq!(point.offset(3), DVec2::new(5.0, -2.0));
    assert_eq!(point.offset(0), DVec2::new(-10.0, 4.0));
    // The right back is a linked mirror of the left: it cannot be graded
    // itself, and its points follow the source's, mirrored.
    let back = common::piece(&s, "Back left");
    ok(
        &mut s,
        json!({"type": "point.set_grade", "point": back.points[0], "step": [-6, 0]}),
    );
    let copy = common::piece(&s, "Back right");
    let err = run(
        &mut s,
        json!({"type": "point.set_grade", "point": copy.points[0], "step": [1, 0]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "linked_mirror");
    let p = s.project();
    let steps: Vec<f64> = copy
        .points
        .iter()
        .map(|v| {
            let id = p
                .points()
                .keys()
                .find(|k| json!(k.to_string()) == *v)
                .unwrap();
            p.point(*id).unwrap().offset(3).x
        })
        .collect();
    assert!(steps.contains(&6.0), "{steps:?}");
    // A zero step removes the rule.
    ok(
        &mut s,
        json!({"type": "point.set_grade", "point": corner, "step": [0, 0]}),
    );
    assert!(s.project().point(id).unwrap().grade.is_empty());
}

#[test]
fn offsets_fill_the_other_sizes_and_splits_interpolate() {
    let mut s = ranged();
    let front = common::a_line_skirt(&mut s);
    let (a, b) = (front.points[0].clone(), front.points[1].clone());
    ok(
        &mut s,
        json!({"type": "point.set_grade", "point": a, "offsets": {"44": [-8, 0]}}),
    );
    ok(
        &mut s,
        json!({"type": "point.set_grade", "point": b, "offsets": {"44": [8, 0]}}),
    );
    let out = ok(
        &mut s,
        json!({"type": "edge.split", "edge": front.edges[0], "at_mm": 100}),
    );
    let p = s.project();
    let find = |v: &Value| {
        *p.points()
            .keys()
            .find(|k| json!(k.to_string()) == *v)
            .unwrap()
    };
    let pa = p.point(find(&a)).unwrap();
    // 44 is two sizes up: -4 per size, extrapolated to 52 and down to 36.
    assert_eq!(pa.offset(8), DVec2::new(-24.0, 0.0));
    assert_eq!(pa.offset(0), DVec2::new(8.0, 0.0));
    let new = p.point(find(&out["created"][0])).unwrap();
    let hem = p.point(find(&b)).unwrap().pos.distance(pa.pos);
    let t = 100.0 / hem;
    let expected = pa.offset(4).lerp(DVec2::new(8.0, 0.0), t);
    assert!(new.offset(4).distance(expected) < 1e-9);
    // An offset in the base size is refused.
    let err = run(
        &mut s,
        json!({"type": "point.set_grade", "point": a, "offsets": {"40": [1, 0]}}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.base_offset");
}

#[test]
fn a_new_base_redraws_the_pattern_and_keeps_the_grades() {
    let mut s = ranged();
    common::a_line_skirt(&mut s);
    let before = s.project().clone();
    let width_44 = width(&before.at_size(4), "Front");
    ok(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women", "from": "38", "to": "50", "base": "44"}),
    );
    let p = s.project();
    let g = p.grading().unwrap();
    assert_eq!(g.sizes.len(), 7);
    assert_eq!(g.sizes[g.base].name, "44");
    // The board now shows size 44, and the avatar is its body.
    assert!((width(p, "Front") - width_44).abs() < 1e-9);
    assert_eq!(p.avatar().unwrap().body.bust_cm, 98.0);
    // Size 40 is where it was.
    let width_40 = width(&p.at_size(1), "Front");
    assert!((width_40 - width(&before, "Front")).abs() < 1e-6);
    p.check_integrity().unwrap();
    // A size range of one is refused; names must differ.
    let err = run(
        &mut s,
        json!({"type": "grading.set_sizes", "base": "M", "sizes": [{"name": "M"}]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.too_few_sizes");
    let err = run(
        &mut s,
        json!({"type": "grading.set_sizes", "base": "M", "sizes": [{"name": "M"}, {"name": "M"}]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.duplicate_name");
}

#[test]
fn the_shown_size_is_draped_on_its_body() {
    let mut s = ranged();
    common::a_line_skirt(&mut s);
    let base = r3d_commands::drape::prepare(s.project(), None).unwrap();
    ok(&mut s, json!({"type": "grading.show_size", "size": "52"}));
    let p = s.project();
    assert_eq!(p.shown_avatar().unwrap().body.hips_cm, 128.0);
    assert_eq!(p.avatar().unwrap().body.hips_cm, 98.0);
    let big = r3d_commands::drape::prepare(p, None).unwrap();
    let extent = |d: &r3d_commands::drape::Drape| {
        let xs = d.scene.particles.iter().map(|q| q.rest.x);
        xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min)
    };
    assert!(extent(&big) > extent(&base) + 0.05);
    let err = run(&mut s, json!({"type": "grading.show_size", "size": "60"})).unwrap_err();
    assert_eq!(err.message().code, "invalid.unknown_size");
}

#[test]
fn grading_needs_a_range_and_survives_a_save() {
    let mut s = Session::new(Project::new("Untitled"));
    let front = common::a_line_skirt(&mut s);
    let err = run(
        &mut s,
        json!({"type": "point.set_grade", "point": front.points[0], "step": [1, 0]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.no_sizes");
    // Without an avatar, a chart brings its base body.
    ok(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "men", "from": "46", "to": "54", "base": "50"}),
    );
    assert_eq!(s.project().avatar().unwrap().body.bust_cm, 100.0);
    ok(
        &mut s,
        json!({"type": "point.set_grade", "point": front.points[0], "step": [1, 0]}),
    );
    let json = file::to_json(s.project()).unwrap();
    let loaded = file::from_json(&json).unwrap().project;
    assert_eq!(loaded.grading(), s.project().grading());
    let grades = |p: &Project| -> Vec<Vec<DVec2>> {
        p.points().values().map(|pt| pt.grade.clone()).collect()
    };
    assert_eq!(grades(&loaded), grades(s.project()));
    ok(&mut s, json!({"type": "grading.clear", "confirm": true}));
    assert!(s.project().grading().is_none());
    assert!(s.project().points().values().all(|p| p.grade.is_empty()));
}

#[test]
fn the_graded_nest_is_exported() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = r3d_commands::Workspace::create(&tmp.path().join("Skirt.r3d"), "Skirt").unwrap();
    for v in [
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "40"}),
        json!({"type": "template.apply", "template": "straight_skirt"}),
    ] {
        ws.run_step(&Step::from_value(v).unwrap()).unwrap();
    }
    let all = json!(["36", "38", "40", "42", "44"]);
    let export = |ws: &mut r3d_commands::Workspace, v: Value| {
        let out = ws.run_step(&Step::from_value(v).unwrap()).unwrap();
        let path = out["path"].as_str().unwrap().to_owned();
        std::fs::read(tmp.path().join("Skirt.r3d").join(path)).unwrap()
    };
    let svg =
        String::from_utf8(export(&mut ws, json!({"type": "export.svg", "sizes": all}))).unwrap();
    // The base in black, the four others each in its colour, a legend.
    let colours: std::collections::BTreeSet<_> = r3d_io::drawing::SIZE_COLORS[..4]
        .iter()
        .map(|c| format!("#{c:06X}"))
        .collect();
    for c in &colours {
        assert!(svg.contains(c.as_str()), "{c}");
    }
    assert!(svg.contains(">44<"));
    let dxf = String::from_utf8(export(
        &mut ws,
        json!({"type": "export.dxf", "sizes": ["36", "44"]}),
    ))
    .unwrap();
    assert_eq!(dxf.matches("Size: 36").count(), 3);
    assert_eq!(dxf.matches("Size: 44").count(), 3);
    let pdf = export(
        &mut ws,
        json!({"type": "export.pdf", "paper": "single", "sizes": ["44"]}),
    );
    assert!(pdf.starts_with(b"%PDF"));
    let err = ws
        .run_step(&Step::from_value(json!({"type": "export.svg", "sizes": ["60"]})).unwrap())
        .unwrap_err();
    assert!(err.to_string().contains("not a size"), "{err}");
}

#[test]
fn a_template_drafted_before_the_sizes_is_graded_when_they_are_set() {
    // Sizes first, then the shirt; and the shirt first, then the sizes.
    let mut first = ranged();
    ok(
        &mut first,
        json!({"type": "template.apply", "template": "shirt"}),
    );
    let mut later = Session::new(Project::new("Untitled"));
    common::women_m(&mut later);
    ok(
        &mut later,
        json!({"type": "template.apply", "template": "shirt"}),
    );
    ok(
        &mut later,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "52", "base": "40"}),
    );
    let (a, b) = (first.project().at_size(8), later.project().at_size(8));
    // The same pieces in size 52, wherever they sit on the board.
    let mut shifts: std::collections::BTreeMap<_, DVec2> = Default::default();
    for (id, point) in a.points() {
        let other = b.point(*id).unwrap();
        let shift = *shifts.entry(point.panel).or_insert(other.pos - point.pos);
        assert!((other.pos - point.pos).distance(shift) < 1e-6, "{id}");
    }
    for (id, edge) in a.edges() {
        if let (r3d_core::EdgeCurve::Cubic { c1, .. }, r3d_core::EdgeCurve::Cubic { c1: o1, .. }) =
            (&edge.curve, &b.edge(*id).unwrap().curve)
        {
            assert!((*o1 - *c1).distance(shifts[&edge.panel]) < 1e-6, "{id}");
        }
    }
    // A garment drawn by hand stays ungraded.
    let mut plain = Session::new(Project::new("Untitled"));
    ok(
        &mut plain,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 100, "height": 100}),
    );
    ok(
        &mut plain,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "40", "base": "38"}),
    );
    assert!(
        plain
            .project()
            .points()
            .values()
            .all(|p| p.grade.is_empty())
    );
}

#[test]
fn measurements_given_to_a_template_follow_the_sizes() {
    // As the app's start screen does: every size of the form given.
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    ok(
        &mut s,
        json!({"type": "template.apply", "template": "shirt", "sizes":
            {"chest_cm": 90, "length_cm": 72, "sleeve_cm": 49, "ease_cm": 12}}),
    );
    ok(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "52", "base": "40"}),
    );
    let p = s.project();
    let backs: Vec<f64> = (0..9).map(|i| width(&p.at_size(i), "Back")).collect();
    assert!(backs.windows(2).all(|w| w[1] > w[0]), "{backs:?}");
    // Lengths stay as given.
    let height = |q: &Project| {
        let (id, _) = q.panels().iter().find(|(_, pn)| pn.name == "Back").unwrap();
        let b = q.panel_bounding_box(*id).unwrap();
        b.max.y - b.min.y
    };
    assert!((height(&p.at_size(0)) - height(&p.at_size(8))).abs() < 40.0);
}

/// Every size of the built-in charts can be fitted: the avatar each size
/// is draped on measures what the chart says, within a centimetre. A
/// women's 52 (waist 104 cm) came out with a waist of 69 cm, and every
/// size from 48 up was slimmer than asked: trousers graded to it fell.
/// The women's 56 is past what the body can reach (its arms would meet
/// its torso): it comes within 6 cm.
#[test]
fn every_chart_size_is_fitted() {
    let model = r3d_avatar::model::BodyModel::builtin().unwrap();
    for (chart, from, to) in [("women", "32", "56"), ("men", "44", "60")] {
        let mut s = Session::new(Project::new("Untitled"));
        ok(
            &mut s,
            json!({"type": "grading.use_chart", "chart": chart, "from": from, "to": to, "base": from}),
        );
        let range = s
            .run_step(&Step::from_value(json!({"type": "grading.get"})).unwrap())
            .unwrap();
        for size in range["sizes"].as_array().unwrap() {
            let body: r3d_core::Body = serde_json::from_value(size["body"].clone()).unwrap();
            let a = r3d_avatar::fit::fit(&model, &body).achieved;
            let worst = [
                (f64::from(a.bust_cm) - body.bust_cm).abs(),
                (f64::from(a.waist_cm) - body.waist_cm).abs(),
                (f64::from(a.hips_cm) - body.hips_cm).abs(),
            ]
            .into_iter()
            .fold(0.0, f64::max);
            let within = if chart == "women" && size["name"] == "56" {
                6.0
            } else {
                1.0
            };
            assert!(
                worst < within,
                "{chart} {} off by {worst} cm: {a:?}",
                size["name"]
            );
        }
    }
}
