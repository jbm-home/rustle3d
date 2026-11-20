#![allow(clippy::unwrap_used)] // test code
//! Fit maps and the tape measure on a draped T-shirt (spec 24).

use r3d_commands::drape::Latest;
use r3d_commands::fit::{FitMap, Probe, girth, span};
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

    // Clicked on the chest: still level, the same girth as by height.
    let chest_y = chest["height_cm"].as_f64().unwrap() as f32 / 100.0;
    let level = girth(
        &latest.drape,
        &latest.positions,
        body,
        Probe::Height(chest_y),
    );
    let level_loop = level.garment.as_ref().unwrap();
    let front = level_loop
        .points
        .iter()
        .copied()
        .max_by(|a, b| a.z.total_cmp(&b.z))
        .unwrap();
    let clicked = girth(&latest.drape, &latest.positions, body, Probe::Near(front));
    eprintln!("{} {}", level.to_json("level"), clicked.to_json("clicked"));
    let round = clicked.garment.as_ref().unwrap();
    assert!(round.closed);
    assert!(round.plane.tilt() < 10.0, "{}", clicked.to_json("chest"));
    assert!(
        (round.length() - level_loop.length()).abs() < level_loop.length() * 0.03,
        "{}",
        clicked.to_json("chest")
    );
    // All round the chest, front, sides and back: level.
    for p in level_loop
        .points
        .iter()
        .step_by(level_loop.points.len().max(12) / 12)
    {
        let g = girth(&latest.drape, &latest.positions, body, Probe::Near(*p));
        let tilt = g.garment.as_ref().map_or(0.0, |l| l.plane.tilt());
        assert!(tilt < 10.0, "{}", g.to_json("chest"));
    }

    // Between the sleeve's middle and its hem: round the arm, square to the
    // slanting sleeve, the arm inside.
    let sleeve_points = &latest.positions[piece.particles.clone()];
    let middle = (sleeve_points.iter().copied().sum::<glam::Vec3>() / sleeve_points.len() as f32
        + lowest)
        / 2.0;
    let on_sleeve = sleeve_points
        .iter()
        .copied()
        .min_by(|a, b| a.distance(middle).total_cmp(&b.distance(middle)))
        .unwrap();
    let arm = girth(
        &latest.drape,
        &latest.positions,
        body,
        Probe::Near(on_sleeve),
    );
    eprintln!("{}", arm.to_json("mid-sleeve"));
    let sleeve_loop = arm.garment.as_ref().unwrap();
    assert!(
        sleeve_loop.closed && sleeve_loop.length() < 0.6,
        "{}",
        arm.to_json("arm")
    );
    assert!(sleeve_loop.plane.tilt() > 15.0, "{}", arm.to_json("arm"));
    assert!(
        arm.ease().is_some_and(|e| e > 0.0),
        "{}",
        arm.to_json("arm")
    );

    // From the chest down the front, 20 cm: along the cloth, by tape,
    // straight.
    let below = latest
        .positions
        .iter()
        .copied()
        .filter(|p| (p.y - (front.y - 0.2)).abs() < 0.02 && p.x.abs() < 0.03 && p.z > 0.0)
        .max_by(|a, b| a.z.total_cmp(&b.z))
        .unwrap();
    let length = span(&latest.drape, &latest.positions, body, front, below);
    let reading = length.to_json();
    eprintln!("{reading}");
    assert_eq!(reading["joined"], true, "{reading}");
    let (along, tape) = (length.along().unwrap(), length.tape.unwrap());
    assert!(
        along + 1e-4 >= tape && tape + 1e-4 >= length.straight,
        "{reading}"
    );
    assert!(along < length.straight * 1.5, "{reading}");
    let action = run(
        &mut ws,
        json!({"type": "fit.length",
            "from": [front.x, front.y, front.z], "to": [below.x, below.y, below.z]}),
    );
    assert_eq!(action["joined"], true, "{action}");
    assert_eq!(action["drape"], "simulated");

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
