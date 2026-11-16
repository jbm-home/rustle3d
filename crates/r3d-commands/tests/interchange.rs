//! Pattern interchange: every template, exported as SVG, PDF (one page and
//! A4 tiles) and DXF-AAMA, comes back from each file with the same
//! pieces: names, shapes, seam allowances, notches, internal lines and
//! grainlines. Plus reference files drawn by hand (`tests/data/`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::path::Path;

use r3d_commands::{Step, Workspace};
use r3d_core::{PanelId, Project};
use serde_json::{Value, json};

fn step(v: Value) -> Step {
    Step::from_value(v).unwrap()
}

/// What a piece should keep through a round trip.
#[derive(Debug)]
struct Summary {
    name: String,
    area: f64,
    perimeter: f64,
    seam_allowance: f64,
    notches: usize,
    lines: usize,
    grain: f64,
}

fn summary(p: &Project, id: PanelId) -> Summary {
    let panel = p.panel(id).unwrap();
    Summary {
        name: panel.name.clone(),
        area: p.panel_area(id).unwrap(),
        perimeter: p.panel_perimeter(id).unwrap(),
        seam_allowance: panel.seam_allowance_mm,
        notches: p
            .notches()
            .values()
            .filter(|n| panel.contour.contains(&n.edge))
            .count(),
        lines: p
            .internal_lines()
            .values()
            .filter(|l| l.panel == id)
            .count(),
        grain: panel.grain_angle_deg.rem_euclid(180.0),
    }
}

fn summaries(p: &Project) -> Vec<Summary> {
    let mut all: Vec<Summary> = p.panels().keys().map(|id| summary(p, *id)).collect();
    all.sort_by(|a, b| a.name.cmp(&b.name).then(a.area.total_cmp(&b.area)));
    all
}

/// A template with a seam allowance on every piece, a notch on the first
/// edge of each and an internal line on the first piece.
fn source(dir: &Path, template: &str) -> Workspace {
    let mut ws = Workspace::create(&dir.join("src.r3d"), template).unwrap();
    ws.run_step(&step(
        json!({"type": "template.apply", "template": template}),
    ))
    .unwrap();
    let ids: Vec<PanelId> = ws.session().project().panels().keys().copied().collect();
    for (k, id) in ids.iter().enumerate() {
        let p = ws.session().project();
        if p.panel(*id).unwrap().mirror_of.is_some() {
            continue;
        }
        let edge = p.panel(*id).unwrap().contour[0];
        let len = p.edge_length(edge).unwrap();
        let bb = p.panel_bounding_box(*id).unwrap();
        let c = (bb.min + bb.max) * 0.5;
        let mut steps = vec![
            json!({"type": "panel.set_seam_allowance", "panel": id.to_string(), "mm": 10}),
            json!({"type": "notch.add", "edge": edge.to_string(), "offset_mm": len / 3.0}),
        ];
        if k == 0 {
            steps.push(json!({"type": "internal_line.add", "panel": id.to_string(),
                "start": [c.x - 20.0, c.y - 40.0],
                "segments": [{"kind": "cubic", "c1": [c.x, c.y - 20.0], "c2": [c.x, c.y], "to": [c.x + 20.0, c.y + 30.0]}]}));
        }
        for s in steps {
            ws.run_step(&step(s)).unwrap();
        }
    }
    ws
}

fn compare(label: &str, expected: &[Summary], got: &[Summary]) {
    assert_eq!(expected.len(), got.len(), "{label}: piece count\n{got:#?}");
    for (e, g) in expected.iter().zip(got) {
        let ctx = format!("{label}: {}", e.name);
        assert_eq!(e.name, g.name, "{ctx}");
        assert!(
            (e.area - g.area).abs() <= 0.002 * e.area,
            "{ctx}: area {} vs {}",
            e.area,
            g.area
        );
        assert!(
            (e.perimeter - g.perimeter).abs() <= 0.002 * e.perimeter,
            "{ctx}: perimeter {} vs {}",
            e.perimeter,
            g.perimeter
        );
        assert!(
            (e.seam_allowance - g.seam_allowance).abs() <= 0.15,
            "{ctx}: allowance {}",
            g.seam_allowance
        );
        assert_eq!(e.notches, g.notches, "{ctx}: notches");
        assert_eq!(e.lines, g.lines, "{ctx}: internal lines");
        let d = (e.grain - g.grain).rem_euclid(180.0);
        assert!(
            d.min(180.0 - d) < 0.5,
            "{ctx}: grain {} vs {}",
            e.grain,
            g.grain
        );
    }
}

const TEMPLATES: [&str; 5] = [
    "a_line_skirt",
    "straight_skirt",
    "tote_bag",
    "t_shirt",
    "dress",
];

#[test]
fn every_template_survives_every_format() {
    for template in TEMPLATES {
        let tmp = tempfile::tempdir().unwrap();
        let mut src = source(tmp.path(), template);
        let expected = summaries(src.session().project());
        for (export, file) in [
            (
                json!({"type": "export.svg", "path": "out/p.svg"}),
                "out/p.svg",
            ),
            (
                json!({"type": "export.pdf", "path": "out/one.pdf", "paper": "single"}),
                "out/one.pdf",
            ),
            (
                json!({"type": "export.pdf", "path": "out/a4.pdf", "paper": "a4"}),
                "out/a4.pdf",
            ),
            (
                json!({"type": "export.dxf", "path": "out/p.dxf"}),
                "out/p.dxf",
            ),
        ] {
            src.run_step(&step(export)).unwrap();
            let mut dst = Workspace::create(
                &tmp.path().join(format!("{file}.r3d").replace('/', "_")),
                "dst",
            )
            .unwrap();
            let rel = dst.adopt(&src.dir().join(file)).unwrap();
            let out = dst
                .run_step(&step(json!({"type": "import.pattern", "path": rel})))
                .unwrap();
            let label = format!("{template} {file}");
            assert_eq!(out["scale"], "file", "{label}: {out}");
            compare(&label, &expected, &summaries(dst.session().project()));
            // One undo step.
            dst.run_step(&step(json!({"type": "history.undo"})))
                .unwrap();
            assert!(dst.session().project().panels().is_empty(), "{label}: undo");
        }
    }
}

#[test]
fn imported_pieces_go_beside_existing_ones() {
    let tmp = tempfile::tempdir().unwrap();
    let mut src = source(tmp.path(), "tote_bag");
    src.run_step(&step(json!({"type": "export.dxf", "path": "p.dxf"})))
        .unwrap();
    let rel = src.adopt(&src.dir().join("p.dxf")).unwrap();
    let right = |p: &Project| {
        p.panels()
            .keys()
            .map(|id| p.panel_bounding_box(*id).unwrap().max.x)
            .fold(f64::MIN, f64::max)
    };
    let before = right(src.session().project());
    let n = src.session().project().panels().len();
    let out = src
        .run_step(&step(json!({"type": "import.pattern", "path": rel})))
        .unwrap();
    let p = src.session().project();
    assert_eq!(p.panels().len(), 2 * n);
    for id in out["panels"].as_array().unwrap() {
        let id: PanelId = serde_json::from_value(id.clone()).unwrap();
        assert!(p.panel_bounding_box(id).unwrap().min.x > before);
    }
}

#[test]
fn hand_drawn_reference_files() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&tmp.path().join("ref.r3d"), "ref").unwrap();
    for (file, pieces, scale) in [
        ("skirt-aama-inches.dxf", 2, "file"),
        ("bodice-inkscape.svg", 1, "file"),
    ] {
        let rel = ws.adopt(&data.join(file)).unwrap();
        let out = ws
            .run_step(&step(json!({"type": "import.pattern", "path": rel})))
            .unwrap();
        assert_eq!(
            out["panels"].as_array().unwrap().len(),
            pieces,
            "{file}: {out}"
        );
        assert_eq!(out["scale"], scale, "{file}");
    }
    let p = ws.session().project();
    let by_name = |n: &str| summary(p, *p.panels().iter().find(|(_, x)| x.name == n).unwrap().0);

    // Skirt front: 14 × 24 in trapezoid with a 0.5 in allowance.
    let front = by_name("Skirt Front");
    assert!((front.seam_allowance - 12.7).abs() < 0.15, "{front:?}");
    assert_eq!(front.notches, 2);
    assert_eq!(front.grain, 90.0);
    let back = by_name("Skirt Back");
    assert_eq!(back.lines, 1, "{back:?}");

    // Bodice: a dart, a grainline, a notch, 15 mm allowance, curves.
    let bodice = by_name("Bodice front");
    assert_eq!(bodice.lines, 1, "{bodice:?}");
    assert_eq!(bodice.notches, 1, "{bodice:?}");
    assert!((bodice.seam_allowance - 15.0).abs() < 0.15, "{bodice:?}");
    assert!((bodice.grain - 90.0).abs() < 0.5, "{bodice:?}");
}

#[test]
fn import_errors_are_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&tmp.path().join("e.r3d"), "e").unwrap();
    std::fs::create_dir_all(ws.dir().join("imports")).unwrap();
    std::fs::write(ws.dir().join("imports/a.txt"), "x").unwrap();
    std::fs::write(ws.dir().join("imports/b.svg"), "<svg/>").unwrap();
    std::fs::write(ws.dir().join("imports/c.pdf"), "nope").unwrap();
    use r3d_core::Localizable;
    for (path, code) in [
        ("imports/a.txt", "invalid.format"),
        ("imports/b.svg", "import_no_pieces"),
        ("imports/c.pdf", "import_malformed"),
        ("imports/missing.dxf", "read_failed"),
        ("../x.dxf", "outside_project"),
    ] {
        let err = ws
            .run_step(&step(json!({"type": "import.pattern", "path": path})))
            .unwrap_err();
        assert_eq!(err.message().code, code, "{path}: {err}");
    }
}
