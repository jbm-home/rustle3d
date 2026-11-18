#![allow(clippy::unwrap_used)] // test code
//! Fit maps and the tape measure on a draped T-shirt (spec 24).

use r3d_commands::drape::Latest;
use r3d_commands::fit::{FitMap, Probe, girth};
use r3d_commands::{Step, Workspace};
use serde_json::{Value, json};

fn run(ws: &mut Workspace, step: Value) -> Value {
    ws.run_step(&Step::from_value(step).unwrap()).unwrap()
}

#[test]
fn a_draped_t_shirt_reads_its_fit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.r3d");
    let mut ws = Workspace::create(&path, "p").unwrap();
    run(
        &mut ws,
        json!({"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
            "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
    );
    run(
        &mut ws,
        json!({"type": "template.apply", "template": "t_shirt"}),
    );
    // Before any drape: the starting positions, said so.
    let start = run(&mut ws, json!({"type": "fit.report"}));
    assert_eq!(start["drape"], "start");
    run(&mut ws, json!({"type": "simulate", "frames": 150}));
    let report = run(&mut ws, json!({"type": "fit.report", "heights_cm": [100]}));
    eprintln!("{report:#}");
    assert_eq!(report["drape"], "simulated");
    let pieces = report["pieces"].as_array().unwrap();
    assert_eq!(
        pieces.len(),
        ws.session().project().panels().len(),
        "every piece"
    );
    for p in pieces {
        let max = p["max_strain_pct"].as_f64().unwrap();
        let mean = p["mean_strain_pct"].as_f64().unwrap();
        assert!((0.0..50.0).contains(&max) && mean <= max, "{p}");
        assert!(p["mean_gap_mm"].as_f64().unwrap() >= 0.0);
    }
    // Chest, waist, hips and the height asked for.
    let ease = report["ease"].as_array().unwrap();
    let names: Vec<&str> = ease.iter().map(|e| e["at"].as_str().unwrap()).collect();
    assert_eq!(names, ["chest", "waist", "hips", "100 cm"]);
    let chest = &ease[0];
    assert_eq!(chest["closed"], true, "{chest}");
    let (garment, body) = (
        chest["garment_cm"].as_f64().unwrap(),
        chest["body_cm"].as_f64().unwrap(),
    );
    // A T-shirt is looser than the 90 cm bust, but not by a metre.
    assert!((85.0..95.0).contains(&body), "{chest}");
    assert!(garment > body && garment < body + 40.0, "{chest}");
    assert!(chest["ease_cm"].as_f64().unwrap() > 0.0);

    // The tape at a point of the drape: on the sleeve, around the arm.
    let latest = Latest::load(&path, ws.session().project()).unwrap();
    let body = latest
        .body
        .as_deref()
        .zip(latest.form.as_ref().map(|f| &f.triangles[..]));
    let sleeve = ws
        .session()
        .project()
        .panels()
        .iter()
        .find(|(_, p)| p.name.contains("Sleeve"))
        .map(|(id, _)| *id)
        .unwrap();
    let piece = latest
        .drape
        .pieces
        .iter()
        .find(|p| p.panel == sleeve)
        .unwrap();
    // Near the sleeve's lowest point: around the arm, not the body.
    let lowest = latest.positions[piece.particles.clone()]
        .iter()
        .copied()
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap();
    let at = lowest + glam::Vec3::Y * 0.01;
    let g = girth(&latest.drape, &latest.positions, body, Probe::Near(at));
    let tape = g.to_json("sleeve");
    eprintln!("{tape}");
    let around = g.garment.as_ref().unwrap();
    assert!(around.length() < 0.6, "{tape}");
    // The arm inside it, if the cut goes round both.
    assert!(g.body.is_none_or(|b| b < 0.5), "{tape}");
    assert!(g.gap.unwrap() < 0.1, "{tape}");

    // Maps: finite, one value per particle.
    for map in FitMap::ALL {
        let v = map.values(&latest.drape, &latest.positions, body);
        assert_eq!(v.len(), latest.positions.len());
        assert!(v.iter().all(|x| x.is_finite() && *x >= 0.0), "{map:?}");
    }

    // The tape by height, and refusals.
    let tape = run(&mut ws, json!({"type": "fit.tape", "height_cm": 120}));
    assert_eq!(tape["drape"], "simulated");
    assert!(tape["garment_cm"].as_f64().is_some(), "{tape}");
    let none = Step::from_value(json!({"type": "fit.tape"})).unwrap();
    assert!(ws.run_step(&none).is_err());
    let high = Step::from_value(json!({"type": "fit.tape", "height_cm": 900})).unwrap();
    assert!(ws.run_step(&high).is_err());
}
