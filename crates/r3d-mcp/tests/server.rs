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
fn the_tech_pack_without_views() {
    let (tmp, s) = server();
    call(
        &s,
        "panel_create_rect",
        json!({"width": 400, "height": 600}),
    );
    let (err, out) = call(
        &s,
        "export_techpack",
        json!({"views": "none", "paper": "letter"}),
    );
    assert!(!err, "{out}");
    assert_eq!(out["images"], 0);
    assert_eq!(out["path"], "exports/P-techpack.pdf");
    assert!(tmp.path().join("P.r3d/exports/P-techpack.pdf").is_file());
    let (err, body) = call(&s, "export_techpack", json!({"views": "sketch"}));
    assert!(err);
    assert_eq!(body["error"]["args"]["name"], "views");
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

    // Lit by a built-in environment, turned.
    let photo = serde_json::to_value(s.call(
        "render_photo",
        Some(Map::from_iter([
            ("width".into(), json!(48)),
            ("height".into(), json!(48)),
            ("samples".into(), json!(1)),
            ("environment".into(), json!("neon")),
            ("turn_deg".into(), json!(90)),
            ("brightness".into(), json!(1.5)),
        ])),
    ))
    .unwrap();
    assert_eq!(photo["content"][0]["type"], "image", "{photo}");
    let info: Value = serde_json::from_str(photo["content"][1]["text"].as_str().unwrap()).unwrap();
    assert_eq!(info["environment"], "neon");
    assert_eq!(info["turn_deg"], 90.0);
    assert_eq!(info["brightness"], 1.5);
    assert!(info.get("lighting").is_none());
    for (args, name) in [
        (json!({ "environment": "sunset" }), "environment"),
        (
            json!({ "environment": "neon", "brightness": 50 }),
            "brightness",
        ),
    ] {
        let (err, body) = call(&s, "render_photo", args);
        assert!(err);
        assert_eq!(body["error"]["code"], "malformed");
        assert_eq!(body["error"]["args"]["name"], name);
    }
    let (err, body) = call(
        &s,
        "render_photo",
        json!({ "environment": "environments/no.hdr" }),
    );
    assert!(err);
    assert_eq!(body["error"]["code"], "read_failed", "{body}");

    // The project's photo scene fills in what the arguments leave out.
    let (err, body) = call(
        &s,
        "photo_set_scene",
        json!({ "scene": {
            "light": { "kind": "environment", "name": "pav" },
            "turn_deg": 30, "brightness": 2,
            "camera": { "target_mm": [0, 900, 0], "yaw_deg": 45, "pitch_deg": 5,
                "distance_mm": 3000, "focal_mm": 50 },
            "size": [40, 30] } }),
    );
    assert!(!err, "{body}");
    let photo_info = |args: Value| {
        let Value::Object(args) = args else {
            unreachable!()
        };
        let photo = serde_json::to_value(s.call("render_photo", Some(args))).unwrap();
        assert_eq!(photo["content"][0]["type"], "image", "{photo}");
        serde_json::from_str::<Value>(photo["content"][1]["text"].as_str().unwrap()).unwrap()
    };
    let info = photo_info(json!({ "samples": 1 }));
    assert_eq!(info["environment"], "pav");
    assert_eq!(info["camera"], "scene");
    assert_eq!(info["size"], json!([40, 30]));
    assert_eq!(
        (info["turn_deg"].clone(), info["brightness"].clone()),
        (json!(30.0), json!(2.0))
    );
    let info = photo_info(
        json!({ "samples": 1, "lighting": "dramatic", "camera": "front", "brightness": 1, "width": 32 }),
    );
    assert_eq!(info["lighting"], "dramatic");
    assert_eq!(info["camera"], "front");
    assert_eq!(info["turn_deg"], 30.0);
    assert_eq!(info["size"], json!([32, 768]));

    // The tech pack opens with the front and back of the drape.
    let (err, out) = call(
        &s,
        "export_techpack",
        json!({ "labels": {"front": "Devant"} }),
    );
    assert!(!err, "{out}");
    assert_eq!(out["images"], 2);
    let pdf = std::fs::read(tmp.path().join("P.r3d").join(out["path"].as_str().unwrap())).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("Devant"));

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

    // The fit: a map in colour, and the numbers.
    let fit = serde_json::to_value(s.call(
        "render_3d",
        Some(Map::from_iter([
            ("width".into(), json!(64)),
            ("height".into(), json!(48)),
            ("fit_map".into(), json!("strain")),
        ])),
    ))
    .unwrap();
    assert_eq!(fit["content"][0]["type"], "image", "{fit}");
    let info: Value = serde_json::from_str(fit["content"][1]["text"].as_str().unwrap()).unwrap();
    assert_eq!(info["fit_map"]["map"], "strain");
    assert_eq!(info["fit_map"]["unit"], "%");
    let (err, body) = call(&s, "render_3d", json!({ "fit_map": "stress" }));
    assert!(err);
    assert_eq!(body["error"]["args"]["name"], "fit_map");
    let (err, report) = call(&s, "fit_report", json!({}));
    assert!(!err, "{report}");
    assert_eq!(report["pieces"].as_array().unwrap().len(), 3);
    let (err, tape) = call(&s, "fit_tape", json!({ "height_cm": 95 }));
    assert!(!err, "{tape}");
    assert!(tape["body_cm"].as_f64().is_some(), "{tape}");
}
