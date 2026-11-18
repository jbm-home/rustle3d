#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::{Action, Command, Query, Workspace};
use r3d_mcp::Server;
use serde_json::{Map, Value, json};

fn server() -> (tempfile::TempDir, Server) {
    let tmp = tempfile::tempdir().unwrap();
    let ws = Workspace::create(&tmp.path().join("P.r3d"), "P").unwrap();
    (tmp, Server::new(ws))
}

/// Call a tool; returns (is_error, parsed JSON body).
fn call(s: &Server, name: &str, args: Value) -> (bool, Value) {
    let args: Option<Map<String, Value>> = match args {
        Value::Object(m) => Some(m),
        _ => None,
    };
    let result = serde_json::to_value(s.call(name, args)).unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();
    (
        result["isError"] == true,
        serde_json::from_str(text).unwrap(),
    )
}

#[test]
fn tools_mirror_the_registries() {
    let (_tmp, s) = server();
    let names = s.tool_names();
    let expected =
        Command::registry().len() + Query::registry().len() + Action::registry().len() + 8;
    assert_eq!(names.len(), expected);
    for n in &names {
        assert!(
            n.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "{n}"
        );
    }
    assert!(names.contains(&"panel_create_rect".to_owned()));
    assert!(names.contains(&"export_pdf".to_owned()));
    assert!(names.contains(&"render_photo".to_owned()));
    assert!(names.contains(&"history_go_to".to_owned()));
}

#[test]
fn edits_are_saved_and_undoable() {
    let (tmp, s) = server();
    let (err, out) = call(
        &s,
        "panel_create_rect",
        json!({"width": 400, "height": 600}),
    );
    assert!(!err, "{out}");
    assert_eq!(out["created"][0], "panel:2");
    let saved = std::fs::read_to_string(tmp.path().join("P.r3d/project.json")).unwrap();
    assert!(saved.contains("panel:2"));

    let (_, summary) = call(&s, "project_summary", json!({}));
    assert_eq!(summary["counts"]["panels"], 1);

    let (err, _) = call(&s, "history_undo", Value::Null);
    assert!(!err);
    let saved = std::fs::read_to_string(tmp.path().join("P.r3d/project.json")).unwrap();
    assert!(!saved.contains("panel:2"));
}

#[test]
fn errors_are_readable_results() {
    let (_tmp, s) = server();
    call(&s, "panel_create_rect", json!({"width": 10, "height": 10}));
    let (err, body) = call(&s, "panel_delete", json!({"panel": "panel:2"}));
    assert!(err);
    assert_eq!(body["error"]["code"], "confirmation_required");
    let (err, body) = call(&s, "nope", json!({}));
    assert!(err);
    assert_eq!(body["error"]["code"], "unknown_type");
    let (err, body) = call(&s, "export_svg", json!({"path": "/etc/x.svg"}));
    assert!(err);
    assert_eq!(body["error"]["code"], "outside_project");
}

#[test]
fn batch_and_export() {
    let (tmp, s) = server();
    let (err, out) = call(
        &s,
        "batch",
        json!({"commands": [
            {"type": "panel.create_rect", "name": "Front", "width": 400, "height": 600},
            {"type": "panel.set_seam_allowance", "panel": "panel:2", "mm": 10}
        ]}),
    );
    assert!(!err, "{out}");
    let (err, out) = call(&s, "export_pdf", json!({"paper": "a4"}));
    assert!(!err, "{out}");
    assert!(
        tmp.path()
            .join("P.r3d")
            .join(out["path"].as_str().unwrap())
            .is_file()
    );
}

#[test]
fn agents_simulate_and_look_at_the_garment() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&tmp.path().join("P.r3d"), "P").unwrap();
    let body = r#"[{"type": "avatar.set", "body": {"masculinity": 0, "height_cm": 168,
        "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}]"#;
    let skirt = r#"[{"type": "template.apply", "template": "a_line_skirt"}]"#;
    for script in [body, skirt] {
        let script = r3d_commands::Script::parse(script).unwrap();
        script.run(|step| ws.run_step(step)).unwrap();
    }
    let s = Server::new(ws);

    let (err, report) = call(&s, "simulate", json!({ "frames": 30 }));
    assert!(!err, "{report}");
    assert_eq!(report["pieces"], 3);

    let result = serde_json::to_value(s.call(
        "render_3d",
        Some(Map::from_iter([
            ("width".into(), json!(320)),
            ("height".into(), json!(240)),
        ])),
    ))
    .unwrap();
    if result["isError"] == true {
        // Machines without any GPU adapter cannot render.
        let text = result["content"][0]["text"].as_str().unwrap();
        let body: Value = serde_json::from_str(text).unwrap();
        assert_eq!(body["error"]["code"], "gpu");
        return;
    }
    assert_eq!(result["content"][0]["type"], "image");
    assert_eq!(result["content"][0]["mimeType"], "image/png");
    assert!(
        result["content"][0]["data"]
            .as_str()
            .unwrap()
            .starts_with("iVBORw0KGgo")
    );
    let info: Value = serde_json::from_str(result["content"][1]["text"].as_str().unwrap()).unwrap();
    assert_eq!(info["drape"], "simulated");

    let photo = serde_json::to_value(s.call(
        "render_photo",
        Some(Map::from_iter([
            ("width".into(), json!(64)),
            ("height".into(), json!(48)),
            ("quality".into(), json!("quick")),
            ("lighting".into(), json!("soft")),
            ("camera".into(), json!("back")),
            ("samples".into(), json!(2)),
        ])),
    ))
    .unwrap();
    assert_eq!(photo["content"][0]["type"], "image", "{photo}");
    let info: Value = serde_json::from_str(photo["content"][1]["text"].as_str().unwrap()).unwrap();
    assert_eq!(info["view"], "photo");
    assert_eq!(info["samples"], 2);
    assert_eq!(info["lighting"], "soft");
    assert_eq!(info["drape"], "simulated");

    let (err, body) = call(&s, "render_board", json!({ "width": 8 }));
    assert!(err);
    assert_eq!(body["error"]["code"], "invalid.range");
    let (err, body) = call(&s, "render_photo", json!({ "quality": "superb" }));
    assert!(err);
    assert_eq!(body["error"]["code"], "malformed");
    assert_eq!(body["error"]["args"]["name"], "quality");
    let (err, body) = call(&s, "render_photo", json!({ "width": 4096, "height": 4096 }));
    assert!(err);
    assert_eq!(body["error"]["code"], "invalid.range");
    let (err, body) = call(&s, "render_3d", json!({ "quality": "good" }));
    assert!(err);
    assert_eq!(body["error"]["args"]["name"], "quality");
}
