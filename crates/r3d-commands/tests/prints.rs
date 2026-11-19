#![allow(clippy::unwrap_used)] // test code
//! Prints on a striped shirt (spec 25): texture coordinates follow every
//! piece's grainline at the print's real size; the exports carry it.

use glam::DVec2;
use r3d_commands::drape::Latest;
use r3d_commands::prints::{Prints, piece_uvs};
use r3d_commands::{Step, Workspace};
use serde_json::{Value, json};

fn run(ws: &mut Workspace, step: Value) -> Value {
    ws.run_step(&Step::from_value(step).unwrap()).unwrap()
}

fn shirt() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&dir.path().join("p.r3d"), "p").unwrap();
    run(
        &mut ws,
        json!({"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
            "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
    );
    run(
        &mut ws,
        json!({"type": "template.apply", "template": "shirt"}),
    );
    (dir, ws)
}

#[test]
fn stripes_follow_every_pieces_grainline() {
    let (_dir, mut ws) = shirt();
    run(
        &mut ws,
        json!({"type": "garment.set_print", "print": {"builtin": "stripes"},
            "width_mm": 20, "height_mm": 50}),
    );
    // One piece's print turned a quarter: its stripes then run across.
    let project = ws.session().project().clone();
    let turned = *project.panels().keys().next().unwrap();
    run(
        &mut ws,
        json!({"type": "panel.set_print_placement", "panel": turned.to_string(),
            "angle_deg": 90}),
    );
    let project = ws.session().project();
    let latest = Latest::load(ws.dir(), project).unwrap();
    assert!(!latest.drape.pieces.is_empty());
    for piece in &latest.drape.pieces {
        let panel = project.panel(piece.panel).unwrap();
        let grain = (panel.grain_angle_deg + panel.print.angle_deg).to_radians();
        let up = DVec2::from_angle(grain);
        let uvs = piece_uvs(project, &latest.drape, piece, 1.0);
        let rest = &latest.drape.scene.particles[piece.particles.clone()];
        assert_eq!(uvs.len(), rest.len());
        // On every triangle, v falls along the grain (the print's up) at
        // one repeat per 50 mm, and u does not change along it.
        for t in piece.indices.chunks(3) {
            let p = |i: u32| {
                let r = rest[i as usize].rest;
                DVec2::new(f64::from(r.x), f64::from(r.y)) * 1000.0
            };
            let uv = |i: u32| {
                let w = uvs[i as usize];
                DVec2::new(f64::from(w[0]), f64::from(w[1]))
            };
            let (e1, e2) = (p(t[1]) - p(t[0]), p(t[2]) - p(t[0]));
            let (d1, d2) = (uv(t[1]) - uv(t[0]), uv(t[2]) - uv(t[0]));
            let det = e1.perp_dot(e2);
            if det.abs() < 1.0 {
                continue;
            }
            // Gradients of u and v on the pattern (per mm).
            let grad = |a: f64, b: f64| DVec2::new(a * e2.y - b * e1.y, b * e1.x - a * e2.x) / det;
            let gu = grad(d1.x, d2.x);
            let gv = grad(d1.y, d2.y);
            assert!(
                gu.dot(up).abs() < 1e-4,
                "{}: u changes along the grain",
                panel.name
            );
            let along = -gv.dot(up) * 50.0;
            assert!((along - 1.0).abs() < 1e-3, "{}: {along}", panel.name);
            assert!((gu.length() * 20.0 - 1.0).abs() < 1e-3, "{}", panel.name);
        }
    }
}

#[test]
fn prints_are_checked() {
    let (_dir, mut ws) = shirt();
    for (step, code) in [
        (json!({"print": {"builtin": "plaid"}}), "malformed"),
        (json!({"print": {"image": "/etc/a.png"}}), "invalid.format"),
        (json!({"print": {"image": "../a.png"}}), "invalid.format"),
        (json!({"print": {"image": "a.gif"}}), "invalid.format"),
        (
            json!({"print": {"builtin": "dots"}, "width_mm": 0.5}),
            "invalid.range",
        ),
        (
            json!({"print": {"builtin": "dots"}, "height_mm": 9000}),
            "invalid.range",
        ),
    ] {
        let mut step = step;
        step["type"] = json!("garment.set_print");
        let err = Step::from_value(step.clone())
            .map_err(|e| e.to_string())
            .and_then(|s| {
                ws.run_step(&s).map_err(|e| {
                    use r3d_core::Localizable;
                    e.message().code.to_string()
                })
            });
        let err = err.unwrap_err();
        assert!(err.contains(code) || code == "malformed", "{step}: {err}");
    }
    // Defaults: the built-in's usual repeat; the angle kept in a half turn.
    run(
        &mut ws,
        json!({"type": "garment.set_print", "print": {"builtin": "gingham"}, "angle_deg": 270}),
    );
    let summary = run(&mut ws, json!({"type": "project.summary"}));
    let print = &summary["garments"][0]["print"];
    assert_eq!(print["source"], json!({"builtin": "gingham"}));
    assert_eq!(print["width_mm"], 12.0);
    assert_eq!(print["angle_deg"], -90.0);
}

#[test]
fn exports_carry_the_print_and_a_missing_image_falls_back() {
    let (dir, mut ws) = shirt();
    // An image the user copied in: 4 × 2 pixels, so twice as wide as high.
    let texture = r3d_core::Texture {
        width: 4,
        height: 2,
        rgba: [200u8, 30, 30, 255].repeat(8),
    };
    std::fs::create_dir_all(ws.dir().join("textures")).unwrap();
    std::fs::write(
        ws.dir().join("textures/red.png"),
        r3d_io::texture::encode_png(&texture),
    )
    .unwrap();
    run(
        &mut ws,
        json!({"type": "garment.set_print", "print": {"image": "textures/red.png"},
            "width_mm": 200}),
    );
    let project = ws.session().project().clone();
    let mut prints = Prints::default();
    let all = prints.all(&project, Some(ws.dir()));
    let image = all.values().next().unwrap();
    assert_eq!(image.aspect, 0.5);
    assert_eq!(image.texture.mean_rgb(), 0xC81E1E);

    let glb = run(&mut ws, json!({"type": "export.gltf"}));
    let bytes = std::fs::read(ws.dir().join(glb["path"].as_str().unwrap())).unwrap();
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let doc: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
    assert_eq!(doc["images"][0]["mimeType"], "image/png");
    assert_eq!(
        doc["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["index"],
        0
    );

    let obj = run(
        &mut ws,
        json!({"type": "export.obj", "path": "exports/my shirt.obj"}),
    );
    assert_eq!(obj["mtl"], "exports/my_shirt.mtl");
    assert_eq!(obj["textures"], json!(["exports/my_shirt_1.png"]));
    let mtl = std::fs::read_to_string(ws.dir().join("exports/my_shirt.mtl")).unwrap();
    assert!(mtl.contains("map_Kd my_shirt_1.png"), "{mtl}");
    assert!(ws.dir().join("exports/my_shirt_1.png").exists());

    // The file gone: the plain colour, no error.
    std::fs::remove_file(ws.dir().join("textures/red.png")).unwrap();
    assert!(prints.all(&project, Some(ws.dir())).is_empty());
    run(&mut ws, json!({"type": "export.gltf"}));
    drop(dir);
}
