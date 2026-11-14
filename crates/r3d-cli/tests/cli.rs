#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use assert_cmd::Command;
use serde_json::{Value, json};

fn r3d() -> Command {
    Command::cargo_bin("r3d").unwrap()
}

fn json_of(out: &[u8]) -> Value {
    serde_json::from_slice(out).unwrap()
}

#[test]
fn new_run_query_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Skirt.r3d");

    let out = r3d()
        .arg("new")
        .arg(&dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(json_of(&out)["name"], "Skirt");
    assert!(dir.join("project.json").is_file());

    let script = r#"[
        {"type": "panel.create_rect", "name": "Front", "width": 400, "height": 600},
        {"type": "panel.rename", "panel": "panel:2", "name": "Front skirt"},
        {"type": "project.summary"}
    ]"#;
    let out = r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .write_stdin(script)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["saved"], true);
    assert_eq!(
        v["results"][2]["result"]["panels"][0]["name"],
        "Front skirt"
    );

    let out = r3d()
        .arg("query")
        .arg(&dir)
        .arg(r#"{"type": "panel.list"}"#)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(json_of(&out)[0]["area_mm2"], 240000.0);
}

#[test]
fn failing_script_saves_nothing_and_reports_structured_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("P.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    let before = std::fs::read_to_string(dir.join("project.json")).unwrap();

    let script = r#"[
        {"type": "panel.create_rect", "width": 10, "height": 10},
        {"type": "panel.create_rect", "width": -5, "height": 10}
    ]"#;
    let out = r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .write_stdin(script)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let err = json_of(&out);
    assert_eq!(err["error"]["code"], "invalid.not_positive");
    assert_eq!(err["error"]["args"]["step"], "1");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).unwrap(),
        before
    );
}

#[test]
fn dry_run_does_not_save() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("P.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    let before = std::fs::read_to_string(dir.join("project.json")).unwrap();
    r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .arg("--dry-run")
        .write_stdin(r#"[{"type": "panel.create_rect", "width": 10, "height": 10}]"#)
        .assert()
        .success();
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).unwrap(),
        before
    );
}

#[test]
fn new_refuses_existing_project() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("P.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    let out = r3d()
        .arg("new")
        .arg(&dir)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    assert_eq!(json_of(&out)["error"]["code"], "file.already_exists");
}

#[test]
fn schema_lists_and_describes_operations() {
    let out = r3d()
        .arg("schema")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert!(
        v["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "panel.create_rect")
    );
    let out = r3d()
        .args(["schema", "panel.create_rect"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(json_of(&out)["properties"]["width"].is_object());
    r3d().args(["schema", "nope"]).assert().code(1);
}

#[test]
fn export_writes_into_the_project() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Skirt.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .write_stdin(r#"[{"type": "panel.create_rect", "width": 400, "height": 600}]"#)
        .assert()
        .success();
    let out = r3d()
        .args(["export"])
        .arg(&dir)
        .args(["--format", "pdf", "--paper", "a4"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["path"], "exports/Skirt.pdf");
    assert!(v["pages"].as_u64().unwrap() >= 4);
    assert!(dir.join("exports/Skirt.pdf").is_file());
}

#[test]
fn dry_run_refuses_actions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("P.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    let out = r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .arg("--dry-run")
        .write_stdin(r#"[{"type": "export.svg"}]"#)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    assert_eq!(json_of(&out)["error"]["code"], "needs_project");
    assert!(!dir.join("exports/P.svg").exists());
}

#[test]
fn mcp_over_stdio_lists_and_calls_tools() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Agent.r3d");
    let messages = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
            "name": "panel_create_rect", "arguments": {"width": 300, "height": 500}}}),
    ];
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
    let out = r3d()
        .arg("mcp")
        .arg("--project")
        .arg(&dir)
        .write_stdin(input)
        .timeout(std::time::Duration::from_secs(20))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let replies: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let by_id = |id: i64| replies.iter().find(|r| r["id"] == id).unwrap();
    assert_eq!(by_id(1)["result"]["serverInfo"]["name"], "rustle3d");
    let tools = by_id(2)["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "panel_create_rect"));
    let call = &by_id(3)["result"];
    assert_ne!(call["isError"], true, "{call}");
    // The project folder was created and the change saved.
    let saved = std::fs::read_to_string(dir.join("project.json")).unwrap();
    assert!(saved.contains("panel:2"));
}
