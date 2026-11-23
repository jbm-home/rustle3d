#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Imported bodies (spec 27): the built-in body written to files in other
//! units, axes and facings comes back as the same body; it is checked,
//! measured, given arrangement points, and a T-shirt drapes on it.

use glam::Vec3;
use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Step, Workspace};
use r3d_core::Body;
use r3d_sim::{CpuSolver, Solver};
use serde_json::{Value, json};

fn women_m() -> Body {
    Body {
        masculinity: 0.0,
        height_cm: 168.0,
        bust_cm: 90.0,
        waist_cm: 73.0,
        hips_cm: 98.0,
    }
}

/// The built-in body (m, Y up, facing +Z) and its triangles.
fn builtin() -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let form = Form::of_body(&women_m(), &Default::default()).unwrap();
    (form.positions.clone(), form.triangles.clone())
}

/// An OBJ of `positions` mapped by `to_file`.
fn obj(positions: &[Vec3], triangles: &[[u32; 3]], to_file: impl Fn(Vec3) -> Vec3) -> String {
    let mut s = String::new();
    for p in positions {
        let q = to_file(*p);
        s += &format!("v {} {} {}\n", q.x, q.y, q.z);
    }
    for t in triangles {
        s += &format!("f {} {} {}\n", t[0] + 1, t[1] + 1, t[2] + 1);
    }
    s
}

fn glb(positions: &[Vec3], triangles: &[[u32; 3]], to_file: impl Fn(Vec3) -> Vec3) -> Vec<u8> {
    use r3d_io::scene3d::{Mesh3d, Scene3d};
    r3d_io::gltf::to_glb(&Scene3d {
        generator: "test".into(),
        name: "body".into(),
        pieces: Vec::new(),
        avatar: Some(Mesh3d {
            name: "Avatar".into(),
            positions: positions.iter().map(|p| to_file(*p).to_array()).collect(),
            uvs: Vec::new(),
            indices: triangles.iter().flatten().copied().collect(),
            material: 0,
        }),
        materials: Vec::new(),
    })
}

fn run(ws: &mut Workspace, v: Value) -> Value {
    ws.run_step(&Step::from_value(v).unwrap()).unwrap()
}

fn workspace() -> (tempfile::TempDir, Workspace) {
    let tmp = tempfile::tempdir().unwrap();
    let ws = Workspace::create(&tmp.path().join("Scan.r3d"), "Scan").unwrap();
    (tmp, ws)
}

fn write(ws: &Workspace, name: &str, bytes: &[u8]) -> String {
    let rel = format!("avatars/{name}");
    std::fs::create_dir_all(ws.dir().join("avatars")).unwrap();
    std::fs::write(ws.dir().join(&rel), bytes).unwrap();
    rel
}

fn codes(check: &Value) -> Vec<String> {
    check["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_body_in_other_units_axes_and_facings_comes_back_the_same() {
    let (positions, triangles) = builtin();
    let (_tmp, mut ws) = workspace();
    // Centimetres, Z up, facing -Y (as many modelling tools write it).
    let cm_z = obj(&positions, &triangles, |p| {
        Vec3::new(p.x, -p.z, p.y) * 100.0
    });
    let file = write(&ws, "cm-z.obj", cm_z.as_bytes());
    let out = run(&mut ws, json!({"type": "avatar.import", "path": file}));
    let check = &out["check"];
    eprintln!("{}", serde_json::to_string_pretty(check).unwrap());
    assert_eq!(check["ready"], true);
    assert_eq!(check["scale"], 0.01);
    assert_eq!(check["up"], "z");
    assert_eq!(check["turn_deg"], 0.0);
    let warnings: Vec<&Value> = check["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["severity"] != "info")
        .collect();
    assert!(warnings.is_empty(), "{warnings:?}");
    // Every point inside the body.
    for p in check["points"].as_array().unwrap() {
        assert!(p["distance_mm"].as_f64().unwrap() < 0.0, "{p}");
    }
    // Measured like the built-in body measures itself.
    let achieved = r3d_commands::queries::fitted(&women_m(), &Default::default())
        .unwrap()
        .achieved;
    let body: Body = serde_json::from_value(out["body"].clone()).unwrap();
    for (got, want) in [
        (body.height_cm, achieved.height_cm),
        (body.bust_cm, achieved.bust_cm),
        (body.waist_cm, achieved.waist_cm),
        (body.hips_cm, achieved.hips_cm),
    ] {
        assert!((got - f64::from(want)).abs() < 2.0, "{got} vs {want}");
    }
    assert!(body.masculinity < 0.3, "{}", body.masculinity);
    let first = Form::of_project(ws.session().project()).unwrap().unwrap();

    // Metres, Y up, facing -Z, in GLB: the same body.
    let file = write(
        &ws,
        "turned.glb",
        &glb(&positions, &triangles, |p| Vec3::new(-p.x, p.y, -p.z)),
    );
    let out = run(&mut ws, json!({"type": "avatar.import", "path": file}));
    assert_eq!(out["check"]["scale"], 1.0);
    assert_eq!(out["check"]["up"], "y");
    assert_eq!(out["check"]["turn_deg"], 180.0);
    let second = Form::of_project(ws.session().project()).unwrap().unwrap();
    let apart = first
        .positions
        .iter()
        .zip(&second.positions)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0f32, f32::max);
    assert!(apart < 0.001, "{apart} m apart");

    // Corrected by hand, then back to the detected values.
    run(&mut ws, json!({"type": "avatar.orient", "turn_deg": 90}));
    let check = run(&mut ws, json!({"type": "avatar.check"}));
    assert_eq!(check["turn_deg"], 90.0);
    run(&mut ws, json!({"type": "history.undo"}));
    // Motions are refused on an imported body.
    let err = ws
        .run_step(
            &Step::from_value(json!({"type": "simulate", "frames": 1, "motion": "walk"})).unwrap(),
        )
        .unwrap_err();
    assert_eq!(
        r3d_core::Localizable::message(&err).code,
        "motion.imported_body"
    );
}

#[test]
fn holes_and_wrong_points_are_reported_and_a_shirt_drapes() {
    let (positions, mut triangles) = builtin();
    // Open the soles: drop the triangles under the ankles.
    triangles.retain(|t| t.iter().any(|v| positions[*v as usize].y > 0.015));
    let (_tmp, mut ws) = workspace();
    let file = write(
        &ws,
        "open.obj",
        obj(&positions, &triangles, |p| p).as_bytes(),
    );
    let out = run(&mut ws, json!({"type": "avatar.import", "path": file}));
    let found = codes(&out["check"]);
    assert!(found.contains(&"mesh.open".to_owned()), "{found:?}");
    assert!(found.contains(&"motion.unavailable".to_owned()));

    // A point moved out of the body is flagged.
    run(
        &mut ws,
        json!({"type": "avatar.set_point", "point": "left_elbow", "at_mm": [600, 1000, 0]}),
    );
    let check = run(&mut ws, json!({"type": "avatar.check"}));
    assert!(codes(&check).contains(&"points.outside".to_owned()));
    run(&mut ws, json!({"type": "avatar.guess_points"}));
    let check = run(&mut ws, json!({"type": "avatar.check"}));
    assert!(!codes(&check).contains(&"points.outside".to_owned()));

    // A T-shirt sized from the measurements taken drapes on it.
    run(
        &mut ws,
        json!({"type": "template.apply", "template": "t_shirt"}),
    );
    let project = ws.session().project().clone();
    let form = Form::of_project(&project).unwrap().unwrap();
    let drape = build(&project, Some(&form), DEFAULT_SPACING_MM).unwrap();
    let collider = drape.scene.collider.clone().unwrap();
    let mut solver = CpuSolver::new(drape.scene);
    for _ in 0..180 {
        solver.step(1.0 / 60.0).unwrap();
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let deepest = p
        .iter()
        .filter_map(|q| collider.sample(*q).map(|c| -c.distance))
        .fold(0.0f32, f32::max);
    assert!(deepest < 0.003, "{deepest} m inside the body");
    let stats = solver.stats();
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    // Still on the shoulders.
    let high = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    assert!(high > 1.3, "{high}");

    // Back to the built-in body, keeping the measurements.
    let body = ws.session().project().avatar().unwrap().body;
    run(&mut ws, json!({"type": "avatar.remove_mesh"}));
    let avatar = ws.session().project().avatar().unwrap();
    assert!(avatar.mesh.is_none());
    assert_eq!(avatar.body, body);
}

#[test]
fn a_missing_file_is_reported_after_reopening() {
    let (positions, triangles) = builtin();
    let (_tmp, mut ws) = workspace();
    // A body no other test registers (scaled), so it is not in memory.
    let file = write(
        &ws,
        "gone.obj",
        obj(&positions, &triangles, |p| p * 1000.0).as_bytes(),
    );
    run(
        &mut ws,
        json!({"type": "avatar.import", "path": file.clone()}),
    );
    assert_eq!(
        ws.session()
            .project()
            .avatar()
            .unwrap()
            .mesh
            .as_ref()
            .unwrap()
            .scale,
        0.001
    );
    ws.save().unwrap();
    let dir = ws.dir().to_path_buf();
    // Reopened: read again from the folder.
    let ws = Workspace::open(&dir).unwrap();
    let check = ws
        .session()
        .query(&serde_json::from_value(json!({"type": "avatar.check"})).unwrap())
        .unwrap();
    assert_eq!(check["ready"], true);
    // The file changed by hand.
    std::fs::write(dir.join(&file), "v 0 0 0\n").unwrap();
    let mesh = ws
        .session()
        .project()
        .avatar()
        .unwrap()
        .mesh
        .clone()
        .unwrap();
    let changed = r3d_core::imported::fingerprint(b"v 0 0 0\n");
    let other = r3d_core::AvatarMesh {
        hash: changed,
        ..mesh
    };
    let report = r3d_commands::bodies::check(&other);
    assert!(!report.ready);
    assert_eq!(report.items[0].code, "file.unavailable");
}
