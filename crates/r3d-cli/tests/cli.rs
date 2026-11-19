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
fn avatar_import_copies_detects_and_checks() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Scan.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    // A box 170 cm tall, 40 wide, 25 deep, in centimetres.
    let mut obj = String::new();
    for i in 0..8 {
        let (x, y, z) = (
            if i & 1 == 0 { -20 } else { 20 },
            if i & 2 == 0 { 0 } else { 170 },
            if i & 4 == 0 { -12 } else { 13 },
        );
        obj += &format!(
            "v {x} {y} {z}
"
        );
    }
    obj += "f 1 3 4 2
f 5 6 8 7
f 1 2 6 5
f 3 7 8 4
f 1 5 7 3
f 2 4 8 6
";
    let file = tmp.path().join("box.obj");
    std::fs::write(&file, obj).unwrap();
    let out = r3d()
        .args(["avatar", "import"])
        .arg(&dir)
        .arg(&file)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = json_of(&out);
    assert_eq!(out["check"]["scale"], 0.01);
    assert_eq!(out["check"]["height_m"], 1.7);
    assert!(dir.join("avatars/box.obj").is_file());
    let out = r3d()
        .args(["avatar", "check"])
        .arg(&dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let check = json_of(&out);
    assert_eq!(check["ready"], true);
    assert_eq!(check["file"], "avatars/box.obj");
    // Not a model: refused, nothing copied.
    let bad = tmp.path().join("body.stl");
    std::fs::write(&bad, "solid").unwrap();
    r3d()
        .args(["avatar", "import"])
        .arg(&dir)
        .arg(&bad)
        .assert()
        .failure();
    assert!(!dir.join("avatars/body.stl").exists());
}

#[test]
fn texture_puts_a_print_on_the_garment() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Shirt.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    // A built-in print, turned.
    let out = r3d()
        .arg("texture")
        .arg(&dir)
        .args(["stripes", "--width-mm", "25", "--angle", "-30"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let print = &json_of(&out)["garments"][0]["print"];
    assert_eq!(print["source"], json!({"builtin": "stripes"}));
    assert_eq!(print["width_mm"], 25.0);
    assert_eq!(print["angle_deg"], -30.0);
    // An image: copied into textures/.
    let jpg = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../r3d-io/tests/data/two.jpg");
    let out = r3d()
        .arg("texture")
        .arg(&dir)
        .arg(&jpg)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let print = &json_of(&out)["garments"][0]["print"];
    assert_eq!(print["source"], json!({"image": "textures/two.jpg"}));
    assert!(dir.join("textures/two.jpg").is_file());
    // Not an image: refused, nothing copied.
    let bad = tmp.path().join("notes.png");
    std::fs::write(&bad, "hello").unwrap();
    let out = r3d().arg("texture").arg(&dir).arg(&bad).assert().failure();
    let err = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(err.contains("texture.format"), "{err}");
    assert!(!dir.join("textures/notes.png").exists());
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

    // Game-ready: no avatar, so no skin; two LODs as asked.
    let out = r3d()
        .args(["export"])
        .arg(&dir)
        .args(["--format", "game", "--preset", "godot", "--no-skin"])
        .args(["--lods", "1,0.4", "--normal-map", "256", "--shell"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["path"], "exports/Skirt-godot.glb");
    assert_eq!(v["lods"].as_array().unwrap().len(), 2);
    assert_eq!(v["skinned"], false);
    assert!(dir.join("exports/Skirt-godot.glb").is_file());
    let out = r3d()
        .args(["export"])
        .arg(&dir)
        .args(["--format", "game"])
        .assert()
        .failure();
    let err = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(err.contains("export.no_skeleton"), "{err}");
}

#[test]
fn marker_lays_out_and_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Bag.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .write_stdin(
            r#"[{"type": "panel.create_rect", "width": 400, "height": 600},
                {"type": "panel.create_rect", "width": 400, "height": 600},
                {"type": "garment.set_fabric_details", "price_per_m": 10}]"#,
        )
        .assert()
        .success();
    // Figures only: two 40 x 60 cm pieces, grain along the length, side
    // by side across a 140 cm fabric.
    let out = r3d()
        .arg("marker")
        .arg(&dir)
        .args(["--width-cm", "140"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["length_mm"], 600.0);
    assert_eq!(v["cost"], 6.0);
    assert!(!dir.join("exports").join("Bag-marker.svg").exists());
    let out = r3d()
        .arg("marker")
        .arg(&dir)
        .args(["--width-cm", "140", "--format", "pdf"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(json_of(&out)["path"], "exports/Bag-marker.pdf");
    assert!(dir.join("exports/Bag-marker.pdf").is_file());
    // Too narrow for a piece: a user error.
    r3d()
        .arg("marker")
        .arg(&dir)
        .args(["--width-cm", "35"])
        .assert()
        .code(1);
}

#[test]
fn techpack_writes_a_pdf() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Bag.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .write_stdin(
            r#"[{"type": "panel.create_rect", "width": 400, "height": 600},
                {"type": "garment.set_fabric_details", "price_per_m": 10},
                {"type": "garment.set_notes", "notes": "Double-stitch the handles."}]"#,
        )
        .assert()
        .success();
    let out = r3d()
        .arg("techpack")
        .arg(&dir)
        .args(["--views", "none", "--paper", "letter"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["path"], "exports/Bag-techpack.pdf");
    assert_eq!(v["fabric"][0]["cost"], 6.0);
    let pdf = std::fs::read(dir.join("exports/Bag-techpack.pdf")).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("Double-stitch the handles."));
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

#[test]
fn run_resolves_references_to_earlier_results() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("P.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    let script = r#"[
        {"type": "panel.create_rect", "width": 100, "height": 200},
        {"type": "panel.mirror", "panel": "$0.created[0]", "name": "Copy"},
        {"type": "panel.get", "panel": "$1.created[0]"},
        {"type": "panel.rename", "panel": "$9.created[0]", "name": "x"}
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
    assert_eq!(err["error"]["code"], "bad_reference");
    assert_eq!(err["error"]["args"]["step"], "3");
    assert_eq!(err["error"]["args"]["reference"], "$9.created[0]");
}

/// A project with a body and the sewn skirt template.
fn skirt_project(dir: &std::path::Path) {
    r3d().arg("new").arg(dir).assert().success();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    for script in ["demos/m4/body.json", "demos/m4/skirt.json"] {
        r3d()
            .arg("run")
            .arg(dir)
            .arg(format!("{root}/{script}"))
            .assert()
            .success();
    }
}

#[test]
fn simulate_caches_a_drape_that_render_uses() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Skirt.r3d");
    skirt_project(&dir);

    let out = r3d()
        .args(["simulate", "--frames", "60"])
        .arg(&dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report = json_of(&out);
    assert_eq!(report["pieces"], 3);
    assert!(
        report["max_seam_gap_mm"].as_f64().unwrap() < 5.0,
        "{report}"
    );
    assert!(dir.join(".r3d-cache/drape.bin").is_file());

    let png = tmp.path().join("drape.png");
    let render = r3d()
        .args(["render", "--view", "3d", "--size", "320x240", "--out"])
        .arg(&png)
        .arg(&dir)
        .output()
        .unwrap();
    // Machines without any GPU adapter cannot render; everything else must.
    if render.status.success() {
        let info = json_of(&render.stdout);
        assert_eq!(info["drape"], "simulated");
        assert!(std::fs::read(&png).unwrap().starts_with(b"\x89PNG"));
    } else {
        assert_eq!(json_of(&render.stderr)["error"]["code"], "gpu");
    }

    // How it fits: the numbers, and the map in colour.
    let fit = r3d().arg("fit").arg(&dir).assert().success();
    let report = json_of(&fit.get_output().stdout);
    assert_eq!(report["drape"], "simulated");
    assert_eq!(report["pieces"].as_array().unwrap().len(), 3);
    assert_eq!(report["ease"][1]["at"], "waist");
    let render = r3d()
        .args([
            "render",
            "--view",
            "3d",
            "--fit-map",
            "gap",
            "--size",
            "64x48",
        ])
        .arg("--out")
        .arg(&png)
        .arg(&dir)
        .output()
        .unwrap();
    if render.status.success() {
        assert_eq!(json_of(&render.stdout)["fit_map"]["map"], "gap");
    }
    r3d()
        .args(["render", "--view", "board", "--fit-map", "strain", "--out"])
        .arg(&png)
        .arg(&dir)
        .assert()
        .failure();

    // Any edit makes the cached drape stale: renders fall back to the start.
    r3d()
        .arg("run")
        .arg(&dir)
        .arg("-")
        .write_stdin(r#"[{"type": "garment.set_fabric", "fabric": "silk"}]"#)
        .assert()
        .success();
    let render = r3d()
        .args(["render", "--view", "3d", "--size", "64x64", "--out"])
        .arg(&png)
        .arg(&dir)
        .output()
        .unwrap();
    if render.status.success() {
        assert_eq!(json_of(&render.stdout)["drape"], "start");
    }

    // A photo, quick and small.
    let photo = r3d()
        .args([
            "render",
            "--view",
            "photo",
            "--size",
            "48x36",
            "--quality",
            "quick",
        ])
        .args([
            "--samples",
            "2",
            "--lighting",
            "dramatic",
            "--camera",
            "side",
            "--out",
        ])
        .arg(&png)
        .arg(&dir)
        .output()
        .unwrap();
    if photo.status.success() {
        let info = json_of(&photo.stdout);
        assert_eq!(info["view"], "photo");
        assert_eq!(info["samples"], 2);
        assert_eq!(info["lighting"], "dramatic");
        assert!(std::fs::read(&png).unwrap().starts_with(b"\x89PNG"));
    } else {
        assert_eq!(json_of(&photo.stderr)["error"]["code"], "gpu");
    }
}

#[test]
fn simulate_without_placed_panels_is_a_user_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Empty.r3d");
    r3d().arg("new").arg(&dir).assert().success();
    let out = r3d()
        .arg("simulate")
        .arg(&dir)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    assert_eq!(json_of(&out)["error"]["code"], "nothing_to_simulate");
}

#[test]
fn a_dxf_export_imports_into_another_project() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("A.r3d");
    let b = tmp.path().join("B.r3d");
    r3d().arg("new").arg(&a).assert().success();
    r3d().arg("new").arg(&b).assert().success();
    r3d()
        .arg("run")
        .arg(&a)
        .arg("-")
        .write_stdin(r#"[{"type": "template.apply", "template": "straight_skirt"}]"#)
        .assert()
        .success();
    let out = r3d()
        .args(["export", "--format", "dxf"])
        .arg(&a)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let dxf = a.join(json_of(&out)["path"].as_str().unwrap());
    let out = r3d()
        .arg("import")
        .arg(&b)
        .arg(&dxf)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["scale"], "file");
    let pieces = v["panels"].as_array().unwrap().len();
    assert!(pieces >= 2, "{v}");
    // Copied into the project, and saved.
    assert!(b.join("imports").join(dxf.file_name().unwrap()).is_file());
    let out = r3d()
        .arg("query")
        .arg(&b)
        .arg(json!({"type": "panel.list"}).to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(json_of(&out).as_array().unwrap().len(), pieces);

    // Inches given on the command line.
    let out = r3d()
        .arg("import")
        .arg(&b)
        .arg(&dxf)
        .args(["--units", "in"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(json_of(&out)["scale"], "given");
    assert_eq!(json_of(&out)["mm_per_unit"], 25.4);
}
