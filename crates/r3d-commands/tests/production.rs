#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Production (M30, spec 23): fabric details, indicators per piece, the
//! marker of a graded shirt, its export.

mod common;

use r3d_commands::drape::fabric_preset;
use r3d_commands::{Session, Step, Workspace};
use r3d_core::{Fabric, Localizable, Project};
use r3d_geom::{DVec2, polygon};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, r3d_commands::CommandError> {
    s.run_step(&Step::from_value(v).unwrap())
}

fn ok(s: &mut Session, v: Value) -> Value {
    run(s, v).unwrap()
}

#[test]
fn usual_weights_are_the_drapes() {
    for f in [
        Fabric::Cotton,
        Fabric::Denim,
        Fabric::Jersey,
        Fabric::Silk,
        Fabric::Wool,
    ] {
        let density = fabric_preset(f).params().density;
        assert!(
            ((f.weight_gsm() / 1000.0) as f32 - density).abs() < 1e-7,
            "{f:?}"
        );
    }
}

#[test]
fn fabric_details_are_checked_and_undone() {
    let mut s = Session::new(Project::new("Untitled"));
    let before = s.project().clone();
    ok(
        &mut s,
        json!({"type": "garment.set_fabric_details", "weight_gsm": 150, "width_mm": 1400, "price_per_m": 9.9}),
    );
    let g = s.project().garments().values().next().unwrap();
    assert_eq!(g.weight_gsm(), 150.0);
    assert_eq!(g.fabric_details.width_mm, Some(1400.0));
    // Omitted: back to the defaults.
    ok(&mut s, json!({"type": "garment.set_fabric_details"}));
    let g = s.project().garments().values().next().unwrap();
    assert_eq!(g.weight_gsm(), Fabric::Cotton.weight_gsm());
    assert_eq!(g.fabric_details.price_per_m, None);
    // Typing merges into one undo step.
    s.undo().unwrap();
    assert_eq!(s.project(), &before);
    for bad in [
        json!({"type": "garment.set_fabric_details", "weight_gsm": 5}),
        json!({"type": "garment.set_fabric_details", "width_mm": -1}),
        json!({"type": "garment.set_fabric_details", "price_per_m": -2}),
    ] {
        let e = run(&mut s, bad).unwrap_err();
        assert!(e.message().code.starts_with("invalid."), "{e}");
    }
}

#[test]
fn indicators_of_a_rectangle() {
    let mut s = Session::new(Project::new("Untitled"));
    let made = ok(
        &mut s,
        json!({"type": "panel.create_rect", "name": "Pocket", "origin": [0, 0], "width": 100, "height": 200}),
    );
    let panel = made["created"][0].as_str().unwrap().to_owned();
    ok(
        &mut s,
        json!({"type": "panel.set_seam_allowance", "panel": panel, "mm": 10}),
    );
    ok(
        &mut s,
        json!({"type": "garment.set_fabric_details", "weight_gsm": 200}),
    );
    let out = ok(&mut s, json!({"type": "production.indicators"}));
    let g = &out["garments"][0];
    assert_eq!(g["weight_gsm"], 200.0);
    let piece = &g["pieces"][0];
    assert_eq!(piece["area_mm2"], 20000.0);
    assert_eq!(piece["cut_area_mm2"], 26400.0);
    assert_eq!(piece["perimeter_mm"], 600.0);
    assert_eq!(piece["mass_g"], 5.28);
    assert_eq!(g["totals"]["mass_g"], 5.28);
    assert_eq!(g["totals"]["pieces"], 1);
}

/// A shirt graded 36 to 52, with a 10 mm seam allowance and a price.
fn graded_shirt() -> Session {
    let mut s = Session::new(Project::new("Shirt"));
    common::women_m(&mut s);
    ok(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "52", "base": "40"}),
    );
    ok(
        &mut s,
        json!({"type": "template.apply", "template": "shirt"}),
    );
    let panels: Vec<String> = s.project().panels().keys().map(|p| p.to_string()).collect();
    for p in panels {
        // Mirror copies follow their source.
        let _ = run(
            &mut s,
            json!({"type": "panel.set_seam_allowance", "panel": p, "mm": 10}),
        );
    }
    ok(
        &mut s,
        json!({"type": "garment.set_fabric_details", "width_mm": 1500, "price_per_m": 12.5}),
    );
    s
}

fn laid(piece: &Value) -> Vec<DVec2> {
    piece["cut"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| DVec2::new(p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
        .collect()
}

#[test]
fn a_graded_shirt_on_150_cm() {
    let mut s = graded_shirt();
    let q = json!({"type": "marker.compute", "outlines": true,
        "marker": {"sizes": ["38", "40", "42"]}});
    let m = ok(&mut s, q.clone());
    let pieces = m["pieces"].as_array().unwrap();
    let per_size = s.project().panels().len();
    assert_eq!(pieces.len(), 3 * per_size);
    let eff = m["efficiency_pct"].as_f64().unwrap();
    assert!(eff > 60.0, "efficiency {eff} %");
    let (len, yard) = (
        m["length_mm"].as_f64().unwrap(),
        m["yardage_m"].as_f64().unwrap(),
    );
    assert!(yard * 1000.0 >= len && yard * 1000.0 < len + 10.0);
    assert!((m["cost"].as_f64().unwrap() - (yard * 12.5 * 100.0).round() / 100.0).abs() < 1e-9);
    // Every piece on the fabric, its grain along the length.
    let outlines: Vec<Vec<DVec2>> = pieces.iter().map(laid).collect();
    for (p, o) in pieces.iter().zip(&outlines) {
        let bb = polygon::bounding_box(o).unwrap();
        assert!(bb.min.y >= -0.01 && bb.max.y <= 1500.01 && bb.min.x >= -0.01);
        assert!(bb.max.x <= len + 0.01);
        let grain = p["grain_angle_deg"].as_f64().unwrap();
        assert!(
            grain.abs() < 1e-6 || (grain - 180.0).abs() < 1e-6,
            "{grain}"
        );
    }
    // No two pieces overlap (sampled every 4 mm inside both outlines).
    for i in 0..outlines.len() {
        for j in i + 1..outlines.len() {
            let (a, b) = (&outlines[i], &outlines[j]);
            let (ba, bb) = (
                polygon::bounding_box(a).unwrap(),
                polygon::bounding_box(b).unwrap(),
            );
            let (lo, hi) = (ba.min.max(bb.min), ba.max.min(bb.max));
            let mut x = lo.x + 2.0;
            while x < hi.x {
                let mut y = lo.y + 2.0;
                while y < hi.y {
                    let p = DVec2::new(x, y);
                    assert!(
                        !(polygon::contains(a, p) && polygon::contains(b, p)),
                        "pieces {i} and {j} overlap at {p}"
                    );
                    y += 4.0;
                }
                x += 4.0;
            }
        }
    }
    // Deterministic.
    assert_eq!(ok(&mut s, q), m);
}

#[test]
fn quantities_and_a_narrow_fabric() {
    let mut s = graded_shirt();
    let back = s
        .project()
        .panels()
        .iter()
        .find(|(_, p)| p.name == "Back")
        .map(|(id, _)| id.to_string())
        .unwrap();
    let m = ok(
        &mut s,
        json!({"type": "marker.compute", "marker": {"quantities": [
            {"panel": back, "count": 2, "pairs": true}]}}),
    );
    let backs: Vec<&Value> = m["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["panel"] == back.as_str())
        .collect();
    assert_eq!(backs.len(), 2);
    assert_ne!(backs[0]["mirrored"], backs[1]["mirrored"]);
    let e = run(
        &mut s,
        json!({"type": "marker.compute", "marker": {"width_mm": 300}}),
    )
    .unwrap_err();
    assert_eq!(e.message().code, "marker_too_narrow");
}

#[test]
fn the_marker_exports_as_svg_and_pdf() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Shirt.r3d");
    r3d_core::project_dir::create(&dir, graded_shirt().project()).unwrap();
    let mut ws = Workspace::open(&dir).unwrap();
    let out = ws
        .run_step(
            &Step::from_value(json!({"type": "export.marker", "marker": {"sizes": ["38", "40"]}}))
                .unwrap(),
        )
        .unwrap();
    assert_eq!(out["path"], "exports/Shirt-marker.svg");
    let svg = std::fs::read_to_string(dir.join("exports/Shirt-marker.svg")).unwrap();
    assert!(svg.contains("Back 38") && svg.contains("100 mm"));
    let out = ws
        .run_step(
            &Step::from_value(json!({"type": "export.marker", "format": "pdf",
                "marker": {"sizes": ["40"]}, "path": "exports/m.pdf"}))
            .unwrap(),
        )
        .unwrap();
    assert!(out["yardage_m"].as_f64().unwrap() > 0.5);
    let pdf = std::fs::read(dir.join("exports/m.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn notes_are_kept_trimmed_and_undone() {
    let mut s = Session::new(Project::new("Untitled"));
    ok(
        &mut s,
        json!({"type": "garment.set_notes", "notes": "Press seams open.\n\n"}),
    );
    let g = s.project().garments().values().next().unwrap();
    assert_eq!(g.notes, "Press seams open.");
    s.undo().unwrap();
    assert!(
        s.project()
            .garments()
            .values()
            .next()
            .unwrap()
            .notes
            .is_empty()
    );
    let long = "x".repeat(20_001);
    let e = run(&mut s, json!({"type": "garment.set_notes", "notes": long})).unwrap_err();
    assert_eq!(e.message().code, "invalid.range");
}

#[test]
fn the_graded_shirt_has_a_tech_pack() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Shirt.r3d");
    let mut s = graded_shirt();
    ok(
        &mut s,
        json!({"type": "garment.set_notes", "notes": "Topstitch the collar 6 mm from the edge."}),
    );
    r3d_core::project_dir::create(&dir, s.project()).unwrap();
    let mut ws = Workspace::open(&dir).unwrap();
    // A picture from the project folder (a 2 x 2 PNG made by hand).
    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, 2, 2);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[200; 12]).unwrap();
    }
    std::fs::create_dir_all(dir.join("renders")).unwrap();
    std::fs::write(dir.join("renders/front.png"), &png).unwrap();
    let out = ws
        .run_step(
            &Step::from_value(json!({"type": "export.techpack",
                "images": [{"path": "renders/front.png", "caption": "Front"}],
                "labels": {"views": "Vues", "decimal": ","}}))
            .unwrap(),
        )
        .unwrap();
    assert_eq!(out["path"], "exports/Shirt-techpack.pdf");
    assert_eq!(out["images"], 1);
    // The fabric of one shirt in the sample size, priced.
    let fabric = &out["fabric"][0];
    let yardage = fabric["yardage_m"].as_f64().unwrap();
    assert!(yardage > 0.8 && yardage < 2.5, "{yardage}");
    assert!((fabric["cost"].as_f64().unwrap() - yardage * 12.5).abs() < 0.01);
    let pdf = std::fs::read(dir.join("exports/Shirt-techpack.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let text = String::from_utf8_lossy(&pdf);
    for word in [
        "Vues",
        "Bill of materials",
        "Size chart",
        "Topstitch the collar",
        // The shirt's buttons in the bill of materials.
        "Button",
        "11 mm",
    ] {
        assert!(text.contains(word), "{word}");
    }
    // Several pages: views, materials, measurements, the pattern, notes.
    let pages = text.matches("/Type /Page").count() - text.matches("/Type /Pages").count();
    assert!(pages >= 4, "{pages} pages");
    // A picture that is not a PNG is refused with a clear code.
    std::fs::write(dir.join("renders/bad.png"), b"nope").unwrap();
    let e = ws
        .run_step(
            &Step::from_value(json!({"type": "export.techpack",
                "images": [{"path": "renders/bad.png"}]}))
            .unwrap(),
        )
        .unwrap_err();
    assert_eq!(e.message().code, "image_unreadable");
}
