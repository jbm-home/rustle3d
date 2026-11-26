#![allow(clippy::unwrap_used)] // test code
//! The project's photo scene (spec 35): saved with the project, checked.

use r3d_commands::{Step, Workspace};
use serde_json::{Value, json};

fn scene() -> Value {
    json!({
        "light": {"kind": "studio"},
        "camera": {"target_mm": [0, 900, 0], "yaw_deg": 20, "pitch_deg": 5,
            "distance_mm": 3000, "focal_mm": 50}
    })
}

fn code(ws: &mut Workspace, scene: Value) -> Result<(), String> {
    let step = Step::from_value(json!({"type": "photo.set_scene", "scene": scene}))
        .map_err(|e| e.to_string())?;
    ws.run_step(&step).map(|_| ()).map_err(|e| {
        use r3d_core::Localizable;
        e.message().code.to_string()
    })
}

#[test]
fn the_scene_is_saved_with_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.r3d");
    let mut ws = Workspace::create(&path, "p").unwrap();
    code(&mut ws, scene()).unwrap();
    ws.save().unwrap();
    let ws = Workspace::open(&path).unwrap();
    let photo = ws.session().project().photo().unwrap();
    assert_eq!(photo.light, r3d_core::PhotoLight::Studio);
    // Omitted: no turn, the usual brightness, the view's size.
    assert_eq!(
        (photo.turn_deg, photo.brightness, photo.size),
        (0.0, 1.0, None)
    );
    assert_eq!(photo.camera.focal_mm, 50.0);
}

#[test]
fn scenes_are_checked() {
    let dir = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&dir.path().join("p.r3d"), "p").unwrap();
    let with = |key: &str, value: Value| {
        let mut s = scene();
        s[key] = value;
        s
    };
    let camera = |key: &str, value: Value| {
        let mut s = scene();
        s["camera"][key] = value;
        s
    };
    for (scene, expected) in [
        (
            with("light", json!({"kind": "environment", "name": "nope"})),
            "invalid.format",
        ),
        (
            with("light", json!({"kind": "environment", "name": "../a.hdr"})),
            "invalid.format",
        ),
        (with("brightness", json!(30)), "invalid.range"),
        (with("place_size_m", json!(0.5)), "invalid.range"),
        (with("size", json!([8, 100])), "invalid.range"),
        (with("size", json!([4096, 4096])), "invalid.range"),
        (camera("focal_mm", json!(5)), "invalid.range"),
        (camera("pitch_deg", json!(89)), "invalid.range"),
        (camera("distance_mm", json!(10)), "invalid.range"),
    ] {
        assert_eq!(
            code(&mut ws, scene.clone()),
            Err(expected.into()),
            "{scene}"
        );
    }
    assert!(ws.session().project().photo().is_none());
    // A project file and a built-in name are both fine.
    let file = json!({"kind": "environment", "name": "environments/loft.hdr"});
    code(&mut ws, with("light", file)).unwrap();
    code(
        &mut ws,
        with("light", json!({"kind": "environment", "name": "pav"})),
    )
    .unwrap();
}
