//! The user's pattern library (docs/specs/32): a garment saved from one
//! project is listed, added to another, renamed, shared as a zip and
//! deleted, through the `library.*` actions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{Step, Workspace, WorkspaceError};
use r3d_core::Localizable;
use serde_json::{Value, json};

fn step(ws: &mut Workspace, v: Value) -> Result<Value, WorkspaceError> {
    ws.run_step(&Step::from_value(v).unwrap())
}

fn run(ws: &mut Workspace, v: Value) -> Value {
    step(ws, v).unwrap()
}

fn code(e: WorkspaceError) -> &'static str {
    e.message().code
}

fn project(dir: &tempfile::TempDir, name: &str, library: &std::path::Path) -> Workspace {
    let mut ws = Workspace::create(&dir.path().join(format!("{name}.r3d")), name).unwrap();
    ws.set_library(Some(library.to_path_buf()));
    ws
}

fn names(ws: &mut Workspace) -> Vec<String> {
    let list = run(ws, json!({"type": "library.list"}));
    list["patterns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_skirt_saved_from_one_project_is_added_to_another() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library");
    let mut a = project(&dir, "A", &library);
    run(
        &mut a,
        json!({"type": "avatar.set", "body": {"masculinity": 0, "height_cm": 168,
            "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
    );
    run(
        &mut a,
        json!({"type": "template.apply", "template": "t_shirt"}),
    );
    let out = run(
        &mut a,
        json!({"type": "template.apply", "template": "a_line_skirt"}),
    );
    let skirt = out["created"][0].as_str().unwrap().to_owned();
    // Its print, an image of the project.
    std::fs::create_dir_all(a.dir().join("textures")).unwrap();
    std::fs::write(a.dir().join("textures/cloth.png"), b"not decoded here").unwrap();
    run(
        &mut a,
        json!({"type": "garment.set_print", "garment": skirt,
            "print": {"image": "textures/cloth.png"}}),
    );

    let saved = run(
        &mut a,
        json!({"type": "library.save", "garment": skirt, "name": "Flared skirt",
            "kind": "dresses_and_skirts"}),
    );
    assert_eq!(saved["kind"], "dresses_and_skirts");
    assert_eq!(saved["template"], "a_line_skirt");
    let again = step(
        &mut a,
        json!({"type": "library.save", "garment": skirt, "name": "flared SKIRT"}),
    );
    assert_eq!(code(again.unwrap_err()), "library.exists");
    run(
        &mut a,
        json!({"type": "library.save", "garment": skirt, "name": "Flared skirt",
            "kind": "dresses_and_skirts", "replace": true}),
    );

    // Listed with the built-in patterns.
    let list = run(&mut a, json!({"type": "library.list"}));
    assert!(
        list["builtin"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["template"] == "a_line_skirt" && b["kind"] == "dresses_and_skirts")
    );
    assert_eq!(names(&mut a), ["Flared skirt"]);

    // Another project, another body.
    let mut b = project(&dir, "B", &library);
    run(
        &mut b,
        json!({"type": "avatar.set", "body": {"masculinity": 0, "height_cm": 175,
            "bust_cm": 100, "waist_cm": 84, "hips_cm": 108}}),
    );
    run(
        &mut b,
        json!({"type": "template.apply", "template": "shirt"}),
    );
    let before = b.session().project().clone();
    let out = run(
        &mut b,
        json!({"type": "library.add", "name": "Flared skirt"}),
    );
    assert!(out["created"][0].as_str().unwrap().starts_with("garment:"));
    let p = b.session().project();
    assert_eq!(p.garments().len(), 2);
    let g = &p.garments()[&p.newest_garment().unwrap()];
    assert_eq!(g.name, "A-line skirt");
    // The print came along into the project.
    assert!(b.dir().join("textures/cloth.png").is_file());
    let summary = run(&mut b, json!({"type": "project.summary"}));
    assert_eq!(
        summary["garments"][1]["print"]["source"]["image"],
        "textures/cloth.png"
    );
    // One undo step.
    run(&mut b, json!({"type": "history.undo"}));
    assert_eq!(b.session().project(), &before);

    // Renamed, shared and deleted.
    run(
        &mut b,
        json!({"type": "library.rename", "name": "Flared skirt", "to": "Skirt"}),
    );
    assert_eq!(names(&mut b), ["Skirt"]);
    let out = run(&mut b, json!({"type": "library.export", "name": "Skirt"}));
    assert_eq!(out["path"], "exports/Skirt.zip");
    let imported = run(
        &mut b,
        json!({"type": "library.import", "path": "exports/Skirt.zip"}),
    );
    assert_eq!(imported["name"], "Skirt 2");
    let refused = step(&mut b, json!({"type": "library.delete", "name": "Skirt"}));
    assert_eq!(code(refused.unwrap_err()), "confirmation_required");
    run(
        &mut b,
        json!({"type": "library.delete", "name": "Skirt", "confirm": true}),
    );
    assert_eq!(names(&mut b), ["Skirt 2"]);
    let missing = step(&mut b, json!({"type": "library.add", "name": "Skirt"}));
    assert_eq!(code(missing.unwrap_err()), "library.missing");
}

#[test]
fn without_a_library_folder_library_steps_say_so() {
    let dir = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&dir.path().join("P.r3d"), "P").unwrap();
    let err = step(&mut ws, json!({"type": "library.list"})).unwrap_err();
    assert_eq!(code(err), "library.none");
}
