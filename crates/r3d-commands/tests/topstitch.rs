//! Topstitching (spec 31): validation, rows, the stitches on the draped
//! cloth and the stiffer cloth under them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, LinesOnCloth, build};
use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

#[test]
fn topstitches_are_checked_and_listed() {
    let mut s = Session::new(Project::new("t"));
    let r = run(
        &mut s,
        json!({"type": "panel.create_rect", "origin": [0, 0], "width": 400, "height": 300}),
    )
    .unwrap();
    let bottom = r["created"][5].clone();
    for (bad, code) in [
        (json!({"offset_mm": 0.5}), "invalid.range"),
        (json!({"rows": 4}), "invalid.range"),
        (json!({"stitch_mm": 12}), "invalid.range"),
        (json!({"color": "white"}), "invalid.format"),
    ] {
        let mut step = json!({"type": "topstitch.add", "side": {"edge": bottom}});
        for (k, v) in bad.as_object().unwrap() {
            step[k] = v.clone();
        }
        let err = run(&mut s, step).unwrap_err();
        assert_eq!(err.message().code, code, "{bad}");
    }
    let id = run(
        &mut s,
        json!({"type": "topstitch.add", "side": {"edge": bottom}, "rows": 2,
            "offset_mm": 6, "gap_mm": 6}),
    )
    .unwrap()["created"][0]
        .clone();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    let t = &list["topstitches"][0];
    assert_eq!(t["id"], id);
    // Two rows inside the piece, 6 and 12 mm up, 0.5 mm short of the
    // sides at each end.
    assert_eq!(t["rows_mm"], json!([399.0, 399.0]));
    assert_eq!(t["thread"], json!("normal"));
    assert_eq!(t["own_color"], json!(false));
    // The default thread: the garment's colour, darker.
    let garment = s.project().garments().values().next().unwrap().rgb();
    let shade = u32::from_str_radix(&t["color"].as_str().unwrap()[1..], 16).unwrap();
    assert!(shade & 0xFF <= garment & 0xFF && shade != garment);
    run(
        &mut s,
        json!({"type": "topstitch.set", "topstitch": id, "color": "#FFFFFF", "thread": "heavy"}),
    )
    .unwrap();
    let list = run(&mut s, json!({"type": "detail.list"})).unwrap();
    assert_eq!(list["topstitches"][0]["color"], json!("#FFFFFF"));
    // An elastic is not a topstitch.
    let elastic = run(
        &mut s,
        json!({"type": "elastic.add", "side": {"edge": r["created"][7]}}),
    )
    .unwrap()["created"][0]
        .clone();
    let wrong = run(
        &mut s,
        json!({"type": "topstitch.remove", "topstitch": elastic}),
    )
    .unwrap_err();
    assert_eq!(wrong.message().code, "invalid.other_kind");
}

#[test]
fn stitches_lie_on_the_cloth_and_stiffen_it() {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": "shirt"}),
    )
    .unwrap();
    let before = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    let back = common::piece(&s, "Back");
    // Two rows along the back's hem, in white (the template topstitches
    // its yoke already).
    run(
        &mut s,
        json!({"type": "topstitch.add", "side": {"edge": back.edges[0]}, "rows": 2,
            "offset_mm": 1.5, "gap_mm": 5, "color": "#FFFFFF", "thread": "heavy"}),
    )
    .unwrap();
    let drape = build(s.project(), None, DEFAULT_SPACING_MM).unwrap();
    // The cloth under them takes the stiffer topstitched fabric.
    let base = drape.scene.fabrics[0];
    let stiff = |t: &&r3d_sim::scene::Triangle| {
        drape.scene.fabrics[usize::from(t.fabric)].bending > base.bending
    };
    let count = |d: &r3d_commands::drape::Drape| d.scene.triangles.iter().filter(stiff).count();
    let more = count(&drape) as i64
        - before
            .scene
            .triangles
            .iter()
            .filter(|t| before.scene.fabrics[usize::from(t.fabric)].bending > base.bending)
            .count() as i64;
    assert!(
        more > 0 && more < drape.scene.triangles.len() as i64 / 20,
        "{more}"
    );
    // On the cloth where it starts: one strip per stitch, every 3 mm
    // along both rows.
    let start: Vec<_> = drape.scene.particles.iter().map(|p| p.position).collect();
    let on = LinesOnCloth::new(&drape, &start);
    let stitches = on.stitches(s.project());
    // The yoke's and ours.
    assert_eq!(stitches.len(), 2);
    let (dashes, _, width) = stitches
        .iter()
        .find(|(_, rgb, _)| *rgb == 0xFFFFFF)
        .unwrap();
    assert!((width - 0.0008).abs() < 1e-6);
    let hem = s
        .project()
        .edge_length(back.edges[0].as_str().unwrap().parse().unwrap())
        .unwrap();
    let want = 2.0 * hem / 3.0;
    assert!(
        (dashes.len() as f64 - want).abs() < 0.1 * want,
        "{} stitches for {hem} mm",
        dashes.len()
    );
    for [(a, _), (b, _)] in dashes {
        let len = a.distance(*b);
        assert!(len > 0.0015 && len < 0.004, "a stitch {len} m long");
    }
}
