#![allow(clippy::unwrap_used, clippy::panic)] // test code
//! The diagnostic report of a draped T-shirt (spec 34).

use r3d_commands::drape::Latest;
use r3d_commands::{Step, Workspace};
use serde_json::{Value, json};

fn run(ws: &mut Workspace, step: Value) -> Result<Value, String> {
    ws.run_step(&Step::from_value(step).unwrap())
        .map_err(|e| e.to_string())
}

fn faces(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap()
}

#[test]
fn a_draped_t_shirt_is_diagnosed() {
    let dir = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&dir.path().join("p.r3d"), "p").unwrap();
    run(
        &mut ws,
        json!({"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
            "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
    )
    .unwrap();
    run(
        &mut ws,
        json!({"type": "template.apply", "template": "t_shirt"}),
    )
    .unwrap();
    let start = run(&mut ws, json!({"type": "drape.diagnose"})).unwrap();
    assert_eq!(start["drape"], "start");
    run(&mut ws, json!({"type": "simulate", "frames": 150})).unwrap();
    let report = run(&mut ws, json!({"type": "drape.diagnose", "limit": 5})).unwrap();
    assert_eq!(report["drape"], "simulated");
    assert_eq!(report["body_signed"], true);
    assert!((report["replay"]["time_s"].as_f64().unwrap() - 2.5).abs() < 1e-3);
    assert_eq!(report["replay"]["fingerprint"].as_str().unwrap().len(), 16);
    let counts = &report["counts"];
    assert_eq!(counts["non_finite"], 0);
    assert!(counts["inside_body"].as_u64().unwrap() * 100 < counts["particles"].as_u64().unwrap());

    // Every seam, widest first, closed.
    let seams = faces(&report["seams"]);
    let listed = run(&mut ws, json!({"type": "seam.list"})).unwrap();
    assert_eq!(seams.len(), faces(&listed).len().min(5));
    let gaps: Vec<f64> = seams
        .iter()
        .map(|s| s["max_gap_mm"].as_f64().unwrap())
        .collect();
    assert!(gaps.windows(2).all(|w| w[0] >= w[1]), "{gaps:?}");
    assert!(gaps[0] < 10.0, "{gaps:?}");

    // The worst faces, worst first.
    let strained = faces(&report["worst"]["strained"]);
    assert_eq!(strained.len(), 5);
    let strain: Vec<f64> = strained
        .iter()
        .map(|f| f["strain_pct"].as_f64().unwrap())
        .collect();
    assert!(strain.windows(2).all(|w| w[0] >= w[1]), "{strain:?}");
    for f in faces(&report["worst"]["crumpled"]) {
        assert!(f["area_ratio"].as_f64().unwrap() < 1.0, "{f}");
    }
    for f in faces(&report["worst"]["inside_body"]) {
        assert!(f["body_mm"].as_f64().unwrap() < -2.0, "{f}");
    }

    // The worst face, found again by a disc on its pattern and a ball on
    // the drape.
    let top = &strained[0];
    let regions = run(
        &mut ws,
        json!({"type": "drape.diagnose", "regions": [
            {"kind": "pattern", "panel": top["panel"], "at_mm": top["at_mm"], "radius_mm": 1},
            {"kind": "drape", "point_m": top["point_m"], "radius_mm": 1},
        ]}),
    )
    .unwrap();
    for region in faces(&regions["regions"]) {
        let listed = faces(&region["listed"]);
        assert!(!listed.is_empty(), "{region}");
        assert_eq!(listed[0]["triangle"], top["triangle"], "{region}");
        assert_eq!(region["max_strain_pct"], top["strain_pct"]);
    }

    // Around a seam, the piece sewn on is the face's own neighbourhood,
    // not a contact.
    let latest = Latest::load(ws.dir(), ws.session().project()).unwrap();
    let [a, b] = latest.drape.scene.seams[latest.drape.scene.seams.len() / 2];
    let mid = (latest.positions[a as usize] + latest.positions[b as usize]) / 2.0;
    let around = run(
        &mut ws,
        json!({"type": "drape.diagnose", "limit": 200, "regions": [
            {"kind": "drape", "point_m": [mid.x, mid.y, mid.z], "radius_mm": 15},
        ]}),
    )
    .unwrap();
    let listed = faces(&around["regions"][0]["listed"]);
    let on_seam: Vec<&Value> = listed
        .iter()
        .filter(|f| !f["seam_gap_mm"].is_null())
        .collect();
    assert!(!on_seam.is_empty());
    for f in on_seam {
        if let Some(d) = f["contact"]["distance_mm"].as_f64() {
            assert!(d > 2.0, "{f}");
        }
    }

    // Refusals.
    for bad in [
        json!({"type": "drape.diagnose", "limit": 0}),
        json!({"type": "drape.diagnose", "regions": [
            {"kind": "drape", "point_m": [0, 1, 0], "radius_mm": 0}]}),
        json!({"type": "drape.diagnose", "regions": [
            {"kind": "pattern", "panel": "panel:999", "at_mm": [0, 0], "radius_mm": 5}]}),
    ] {
        assert!(run(&mut ws, bad.clone()).is_err(), "{bad}");
    }
}
