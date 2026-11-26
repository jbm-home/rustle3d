#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Imported bodies (spec 27): the built-in body written to files in other
//! units, axes and facings comes back as the same body; it is checked,
//! measured, given arrangement points, and a T-shirt drapes on it.

use glam::Vec3;
use r3d_avatar::motion::Motion;
use r3d_commands::drape::{Animation, DEFAULT_SPACING_MM, Form, Simulated, build, simulate};
use r3d_commands::{Progress, Step, Workspace};
use r3d_core::Body;
use r3d_sim::motion::{Posed, touch};
use r3d_sim::{CpuSolver, Solver};
use serde_json::{Value, json};

fn women_m() -> Body {
    Body {
        masculinity: 0.0,
        height_cm: 168.0,
        bust_cm: 90.0,
        waist_cm: 73.0,
        hips_cm: 98.0,
        age_years: None,
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

/// A binary little-endian PLY, as written.
fn ply(positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<u8> {
    let mut b = format!(
        "ply\nformat binary_little_endian 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n",
        positions.len(),
        triangles.len()
    )
    .into_bytes();
    for p in positions {
        for v in p.to_array() {
            b.extend(v.to_le_bytes());
        }
    }
    for t in triangles {
        b.push(3);
        for i in t {
            b.extend((*i as i32).to_le_bytes());
        }
    }
    b
}

/// A binary STL: each triangle with its own corners.
fn stl(positions: &[Vec3], triangles: &[[u32; 3]], to_file: impl Fn(Vec3) -> Vec3) -> Vec<u8> {
    let mut b = vec![0u8; 80];
    b.extend((triangles.len() as u32).to_le_bytes());
    for t in triangles {
        b.extend([0u8; 12]);
        for i in t {
            for v in to_file(positions[*i as usize]).to_array() {
                b.extend(v.to_le_bytes());
            }
        }
        b.extend([0u8; 2]);
    }
    b
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

    // A scan in binary PLY (metres, Y up): the same body, vertex for vertex.
    let file = write(&ws, "scan.ply", &ply(&positions, &triangles));
    let out = run(&mut ws, json!({"type": "avatar.import", "path": file}));
    assert_eq!(out["check"]["ready"], true);
    let ply_form = Form::of_project(ws.session().project()).unwrap().unwrap();
    let apart = first
        .positions
        .iter()
        .zip(&ply_form.positions)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0f32, f32::max);
    assert!(apart < 0.001, "{apart} m apart");

    // In binary STL (millimetres, Z up): loose triangles welded back into
    // one closed surface, so no warning, and the same extent.
    let mm_z = |p: Vec3| Vec3::new(p.x, -p.z, p.y) * 1000.0;
    let file = write(&ws, "scan.stl", &stl(&positions, &triangles, mm_z));
    let out = run(&mut ws, json!({"type": "avatar.import", "path": file}));
    let check = &out["check"];
    assert_eq!(check["ready"], true);
    assert_eq!(check["scale"], 0.001);
    assert_eq!(check["up"], "z");
    let warnings: Vec<&Value> = check["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["severity"] != "info")
        .collect();
    assert!(warnings.is_empty(), "{warnings:?}");
    let stl_form = Form::of_project(ws.session().project()).unwrap().unwrap();
    let extent = |f: &Form| {
        f.positions
            .iter()
            .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            })
    };
    let ((lo_a, hi_a), (lo_b, hi_b)) = (extent(&first), extent(&stl_form));
    assert!(lo_a.distance(lo_b) < 0.001 && hi_a.distance(hi_b) < 0.001);
    assert_eq!(stl_form.triangles.len(), first.triangles.len());

    // Corrected by hand, then back to the detected values.
    run(&mut ws, json!({"type": "avatar.orient", "turn_deg": 90}));
    let check = run(&mut ws, json!({"type": "avatar.check"}));
    assert_eq!(check["turn_deg"], 90.0);
    run(&mut ws, json!({"type": "history.undo"}));
    // It moves, with a skeleton placed by its points.
    let check = run(&mut ws, json!({"type": "avatar.check"}));
    let rig = check["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["code"] == "motion.rig")
        .unwrap();
    assert_eq!(rig["args"]["source"], "points");
    let form = imported_form(&ws);
    assert_ne!(walking(&form, 3.3), form.positions);
}

/// The skin of `form` `t` s into a drape where it walks.
fn walking(form: &Form, t: f32) -> Vec<Vec3> {
    let walk = Animation::starting(Motion::Walk, 0.0);
    form.posed(Some(&walk), t)
}

/// Mean and largest distances between two skins.
fn apart(a: &[Vec3], b: &[Vec3]) -> (f32, f32) {
    assert_eq!(a.len(), b.len());
    let d: Vec<f32> = a.iter().zip(b).map(|(a, b)| a.distance(*b)).collect();
    (
        d.iter().sum::<f32>() / d.len() as f32,
        d.iter().copied().fold(0.0, f32::max),
    )
}

/// The imported body of `ws`'s project, as the drape uses it.
fn imported_form(ws: &Workspace) -> Form {
    Form::of_project(ws.session().project()).unwrap().unwrap()
}

/// Where the import puts a body's points: centred on its vertical axis,
/// the hips' front and back centred, the feet on the floor (spec 27).
fn import_shift(positions: &[Vec3]) -> Vec3 {
    let (lo, hi) = positions
        .iter()
        .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
            (lo.min(*p), hi.max(*p))
        });
    let height = hi.y - lo.y;
    let (zlo, zhi) = positions
        .iter()
        .filter(|p| (0.47..=0.56).contains(&((p.y - lo.y) / height)))
        .fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.z), b.max(p.z)));
    Vec3::new((lo.x + hi.x) / 2.0, lo.y, (zlo + zhi) / 2.0)
}

#[test]
fn a_body_with_a_skeleton_of_its_own_walks_with_it() {
    use r3d_io::game::{GameMesh, GameScene, Joint, to_glb};
    let built_in = Form::of_body(&women_m(), &Default::default()).unwrap();
    let rig = built_in.rig().unwrap();
    // A game character: our bones under another rig's names, its
    // shoulder twist bones unknown to us.
    let name = |ours: &str| -> String {
        let trunk = [
            ("pelvis", "Hips"),
            ("spine1", "Spine"),
            ("spine2", "Spine1"),
            ("spine3", "Spine2"),
            ("chest", "Spine3"),
            ("upper_chest", "Spine4"),
            ("neck", "Neck"),
            ("head", "Head"),
        ];
        if let Some((_, theirs)) = trunk.iter().find(|(o, _)| *o == ours) {
            return format!("rig:{theirs}");
        }
        let (stem, letter) = ours.rsplit_once('_').unwrap();
        let side = if letter == "l" { "Left" } else { "Right" };
        let part = match stem {
            "clavicle" => "Shoulder",
            "shoulder" => "ShoulderTwist",
            "upper_arm" => "Arm",
            "forearm" => "ForeArm",
            "hand" => "Hand",
            "thigh" => "UpLeg",
            "shin" => "Leg",
            _ => "Foot",
        };
        format!("rig:{side}{part}")
    };
    let bones = &rig.skeleton.bones;
    let joints: Vec<Joint> = bones
        .iter()
        .map(|b| {
            let parent = b.parent.map_or(Vec3::ZERO, |p| bones[p].head);
            Joint {
                name: name(&b.name),
                parent: b.parent,
                translation: (b.head - parent).to_array(),
                rotation: [0.0, 0.0, 0.0, 1.0],
                inverse_bind: glam::Mat4::from_translation(-b.head).to_cols_array(),
            }
        })
        .collect();
    let scene = GameScene {
        generator: "test".into(),
        name: "character".into(),
        meshes: vec![GameMesh {
            name: "Body".into(),
            positions: built_in.positions.iter().map(|p| p.to_array()).collect(),
            normals: vec![[0.0, 1.0, 0.0]; built_in.positions.len()],
            joints: rig.weights.iter().map(|w| w.bones).collect(),
            weights: rig.weights.iter().map(|w| w.weights).collect(),
            indices: built_in.triangles.iter().flatten().copied().collect(),
            ..GameMesh::default()
        }],
        materials: Vec::new(),
        images: Vec::new(),
        joints,
        clips: Vec::new(),
        extras: Value::Null,
    };
    let (_tmp, mut ws) = workspace();
    let file = write(&ws, "character.glb", &to_glb(&scene));
    let out = run(&mut ws, json!({"type": "avatar.import", "path": file}));
    assert_eq!(out["check"]["skeleton"], true);
    let rig_item = out["check"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["code"] == "motion.rig")
        .cloned()
        .unwrap();
    assert_eq!(rig_item["args"]["source"], "file");

    // It walks as the built-in body does: each point moves as much.
    let form = imported_form(&ws);
    let t = 3.3;
    let moved = |f: &Form| -> Vec<Vec3> {
        walking(f, t)
            .iter()
            .zip(&f.positions)
            .map(|(a, b)| *a - *b)
            .collect()
    };
    let (mean, most) = apart(&moved(&form), &moved(&built_in));
    assert!(mean < 0.004 && most < 0.03, "{mean} {most}");
    // Standing, it is as imported.
    assert_eq!(form.posed(None, t), form.positions);
}

/// The built-in body with its arms raised to a T, as a scan or a game
/// character often stands, imported from an OBJ, its arms' points moved
/// onto the raised arms as the user does (where the guess finds them on
/// arms down, raised with the arms). Also where the import moved it.
fn t_posed() -> (tempfile::TempDir, Workspace, Vec3) {
    use r3d_avatar::skeleton::{Pose, skin};
    let built_in = Form::of_body(&women_m(), &Default::default()).unwrap();
    let rig = built_in.rig().unwrap();
    let s = rig.skeleton;
    let mut t_pose = Pose::rest(s.bones.len());
    for (bone, angle) in [("upper_arm_l", 0.9f32), ("upper_arm_r", -0.9)] {
        t_pose.rotations[s.find(bone).unwrap()] = glam::Quat::from_rotation_z(angle);
    }
    let to_t = s.transforms(&t_pose);
    let posed = skin(&built_in.positions, rig.weights, &to_t);
    let (tmp, mut ws) = workspace();
    let file = write(
        &ws,
        "t.obj",
        obj(&posed, &built_in.triangles, |p| p).as_bytes(),
    );
    run(&mut ws, json!({"type": "avatar.import", "path": file}));
    let shift = import_shift(&posed);
    let guessed = r3d_commands::drape::Parts::find(&built_in.positions);
    for ((side, letter), arm) in [("left", "l"), ("right", "r")]
        .into_iter()
        .zip(guessed.arms)
    {
        let raised = to_t[s.find(&format!("upper_arm_{letter}")).unwrap()];
        for (point, at) in ["shoulder", "elbow", "wrist"].into_iter().zip(arm) {
            let at = (raised.apply(at) - shift) * 1000.0;
            run(
                &mut ws,
                json!({"type": "avatar.set_point", "point": format!("{side}_{point}"), "at_mm": at.to_array()}),
            );
        }
    }
    (tmp, ws, shift)
}

/// `template` draped on the imported body of `ws` as it does `motion`
/// for `frames`: finite, its seams closed, nothing more than 3 mm inside
/// the moving body.
fn drapes_moving(ws: &mut Workspace, template: &str, motion: Motion, frames: u32) -> Simulated {
    run(ws, json!({"type": "template.apply", "template": template}));
    let out = simulate(
        ws.session().project(),
        frames,
        Some(motion.clone()),
        &Progress::default(),
    )
    .unwrap();
    assert!(out.positions.iter().all(|p| p.is_finite()), "{template}");
    let scene = &out.drape.scene;
    let (body, moving) = (
        scene.collider.as_ref().unwrap(),
        scene.motion.as_ref().unwrap(),
    );
    let t = frames as f32 / 60.0;
    let posed = Posed::new(moving.at(t - 1.0 / 1800.0), moving.at(t));
    let inside = out
        .positions
        .iter()
        .filter_map(|p| touch(body, Some(&posed), *p))
        .map(|(c, _)| -c.distance)
        .fold(0.0f32, f32::max);
    assert!(inside < 0.003, "{template} {motion:?}: {inside} m inside");
    assert!(
        out.stats.max_seam_gap < 0.01,
        "{template} {motion:?}: {:?}",
        out.stats
    );
    out
}

#[test]
fn an_imported_body_walks_in_its_dress() {
    let (positions, triangles) = builtin();
    let (_tmp, mut ws) = workspace();
    let file = write(
        &ws,
        "body.obj",
        obj(&positions, &triangles, |p| p).as_bytes(),
    );
    run(&mut ws, json!({"type": "avatar.import", "path": file}));
    // 1.5 s of draping, then a second of easing in and a step cycle.
    let out = drapes_moving(&mut ws, "dress", Motion::Walk, 220);
    let top = out.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(top > 1.2, "the dress slid down: {top}");
}

#[test]
fn an_imported_body_raises_its_arms_in_a_t_shirt() {
    let (positions, triangles) = builtin();
    let (_tmp, mut ws) = workspace();
    let file = write(
        &ws,
        "body.obj",
        obj(&positions, &triangles, |p| p).as_bytes(),
    );
    run(&mut ws, json!({"type": "avatar.import", "path": file}));
    // Arms up from 1.5 + 1.5 s, held until 1.5 + 2 s: the hem rides up.
    let raised = drapes_moving(&mut ws, "t_shirt", Motion::RaiseArms, 200);
    let still = simulate(ws.session().project(), 200, None, &Progress::default()).unwrap();
    let hem = |s: &Simulated| s.positions.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    assert!(
        hem(&raised) > hem(&still) + 0.02,
        "the hem stayed at {} (still: {})",
        hem(&raised),
        hem(&still)
    );
}

#[test]
fn a_t_posed_body_walks_with_its_arms_down() {
    let built_in = Form::of_body(&women_m(), &Default::default()).unwrap();
    let rig = built_in.rig().unwrap();
    let s = rig.skeleton;
    let (_tmp, ws, shift) = t_posed();
    let form = imported_form(&ws);
    // Standing, it keeps its T; walking, it walks as the built-in body,
    // arms down.
    assert_eq!(form.posed(None, 3.3), form.positions);
    let walked: Vec<Vec3> = walking(&form, 3.3).iter().map(|p| *p + shift).collect();
    let expected = walking(&built_in, 3.3);
    let (mean, _) = apart(&walked, &expected);
    assert!(mean < 0.02, "{mean}");
    let hand = s.find("hand_l").unwrap();
    let hands: Vec<f32> = walked
        .iter()
        .zip(rig.weights)
        .filter(|(_, w)| usize::from(w.bones[0]) == hand && w.weights[0] > 0.9)
        .map(|(p, _)| p.y)
        .collect();
    let highest = hands.iter().copied().fold(f32::MIN, f32::max);
    assert!(!hands.is_empty() && highest < 1.0, "{highest}");
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
    assert!(found.contains(&"motion.rig".to_owned()));

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
