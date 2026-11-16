#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{Session, Step, Workspace};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn step(v: Value) -> Step {
    Step::from_value(v).unwrap()
}

#[test]
fn exports_write_inside_the_project() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&tmp.path().join("Skirt.r3d"), "My skirt").unwrap();
    ws.run_step(&step(
        json!({"type": "panel.create_rect", "width": 400, "height": 600}),
    ))
    .unwrap();

    let out = ws.run_step(&step(json!({"type": "export.svg"}))).unwrap();
    assert_eq!(out["path"], "exports/My skirt.svg");
    assert!(ws.dir().join("exports/My skirt.svg").is_file());

    let out = ws
        .run_step(&step(
            json!({"type": "export.pdf", "path": "print/tiles.pdf", "paper": "a4"}),
        ))
        .unwrap();
    assert!(out["pages"].as_u64().unwrap() >= 4);
    assert!(ws.dir().join("print/tiles.pdf").is_file());

    let err = ws
        .run_step(&step(json!({"type": "export.svg", "path": "../evil.svg"})))
        .unwrap_err();
    assert_eq!(err.message().code, "outside_project");
    assert!(!tmp.path().join("evil.svg").exists());
}

#[test]
fn actions_need_a_project_folder() {
    let mut s = Session::new(Project::new("t"));
    let err = s
        .run_step(&step(json!({"type": "export.svg"})))
        .unwrap_err();
    assert_eq!(err.message().code, "needs_project");
}

#[test]
fn first_save_after_migration_keeps_a_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Old.r3d");
    std::fs::create_dir_all(&dir).unwrap();
    let v1 = include_str!("../../r3d-core/tests/golden/v1/sample.json");
    std::fs::write(dir.join("project.json"), v1).unwrap();

    let mut ws = Workspace::open(&dir).unwrap();
    ws.save().unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("project.v1.json.bak")).unwrap(),
        v1
    );
    let saved = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let current = format!("\"schema_version\": {}", r3d_core::file::SCHEMA_VERSION);
    assert!(saved.contains(&current));
    // A second open is current: no new backup needed.
    assert!(Workspace::open(&dir).is_ok());
}

#[test]
fn the_draped_garment_exports_as_glb_and_obj() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&tmp.path().join("Skirt.r3d"), "Skirt").unwrap();
    ws.run_step(&step(json!({"type": "avatar.set", "preset": "Women M",
        "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}})))
        .unwrap();
    ws.run_step(&step(
        json!({"type": "template.apply", "template": "a_line_skirt"}),
    ))
    .unwrap();

    // Before any simulation, the starting positions are exported.
    let out = ws
        .run_step(&step(json!({"type": "export.gltf", "avatar": true})))
        .unwrap();
    assert_eq!(out["drape"], "start");
    assert_eq!(out["pieces"], 3);
    let glb = std::fs::read(ws.dir().join(out["path"].as_str().unwrap())).unwrap();
    assert_eq!(&glb[0..4], b"glTF");
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let doc: Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    let names: Vec<&str> = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["name"].as_str())
        .collect();
    assert!(
        names.contains(&"Front") && names.contains(&"Avatar"),
        "{names:?}"
    );

    // After a simulation, the drape.
    ws.run_step(&step(json!({"type": "simulate", "frames": 10})))
        .unwrap();
    let out = ws
        .run_step(&step(json!({"type": "export.obj", "path": "skirt.obj"})))
        .unwrap();
    assert_eq!(out["drape"], "simulated");
    let obj = std::fs::read_to_string(ws.dir().join("skirt.obj")).unwrap();
    assert_eq!(obj.lines().filter(|l| l.starts_with("o ")).count(), 3);
    assert!(obj.lines().any(|l| l.starts_with("vt ")));
}
