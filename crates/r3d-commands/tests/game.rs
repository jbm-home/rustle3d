#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! The game-ready export (spec 15): a draped T-shirt with LODs, baked
//! normal maps, a print atlas, a shell, and skinning to the avatar's
//! skeleton with its motions.

use glam::Vec3;
use r3d_avatar::motion::Motion;
use r3d_avatar::pack::Influences;
use r3d_avatar::skeleton::skin;
use r3d_commands::drape::{Animation, Form, MOTION_START_S};
use r3d_commands::{Step, Workspace};
use r3d_core::{Body, Localizable};
use serde_json::{Value, json};

fn run(ws: &mut Workspace, step: Value) -> Value {
    ws.run_step(&Step::from_value(step).unwrap()).unwrap()
}

fn women_m() -> Value {
    json!({"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98})
}

fn t_shirt() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    let mut ws = Workspace::create(&dir.path().join("p.r3d"), "Tee").unwrap();
    run(
        &mut ws,
        json!({"type": "avatar.set", "preset": "Women M", "body": women_m()}),
    );
    run(
        &mut ws,
        json!({"type": "template.apply", "template": "t_shirt"}),
    );
    run(&mut ws, json!({"type": "simulate", "frames": 40}));
    (dir, ws)
}

/// The JSON chunk and the binary chunk of a GLB.
fn chunks(glb: &[u8]) -> (Value, Vec<u8>) {
    let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
    let len = word(12);
    let doc = serde_json::from_slice(&glb[20..20 + len]).unwrap();
    let bin = glb[20 + len + 8..].to_vec();
    (doc, bin)
}

/// An accessor's components as f32 (floats) or u8 (bytes, as f32).
fn read(doc: &Value, bin: &[u8], accessor: &Value) -> Vec<f32> {
    let a = &doc["accessors"][accessor.as_u64().unwrap() as usize];
    let view = &doc["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
    let offset = view["byteOffset"].as_u64().unwrap() as usize;
    let len = view["byteLength"].as_u64().unwrap() as usize;
    let bytes = &bin[offset..offset + len];
    match a["componentType"].as_u64().unwrap() {
        5126 => bytes
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect(),
        5121 => bytes.iter().map(|b| f32::from(*b)).collect(),
        _ => bytes
            .chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()) as f32)
            .collect(),
    }
}

#[test]
fn a_draped_t_shirt_exports_with_lods_maps_and_skin() {
    let (_dir, mut ws) = t_shirt();
    run(
        &mut ws,
        json!({"type": "garment.set_print", "print": {"builtin": "stripes"}, "width_mm": 20}),
    );
    run(&mut ws, json!({"type": "simulate", "frames": 40}));
    let out = run(
        &mut ws,
        json!({"type": "export.game", "path": "a.glb", "normal_map": 256}),
    );
    assert_eq!(out["drape"], "simulated");
    let lods = out["lods"].as_array().unwrap();
    assert_eq!(lods.len(), 3);
    let full = lods[0]["triangles"].as_f64().unwrap();
    assert!(
        lods[1]["triangles"].as_f64().unwrap() <= 0.55 * full,
        "{out}"
    );
    assert!(
        lods[2]["triangles"].as_f64().unwrap() <= 0.3 * full,
        "{out}"
    );
    assert_eq!(
        out["animations"],
        json!(["walk", "turn", "raise_arms", "idle"])
    );

    let glb = std::fs::read(ws.dir().join("a.glb")).unwrap();
    // Deterministic.
    run(
        &mut ws,
        json!({"type": "export.game", "path": "b.glb", "normal_map": 256}),
    );
    assert_eq!(glb, std::fs::read(ws.dir().join("b.glb")).unwrap());

    let (doc, bin) = chunks(&glb);
    let joints = doc["skins"][0]["joints"].as_array().unwrap().len();
    assert!(joints >= 24, "{joints}");
    let names: Vec<&str> = doc["meshes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["T-shirt_LOD0", "T-shirt_LOD1", "T-shirt_LOD2"],
        "{names:?}"
    );
    // LOD0 has no normal map, the others do; all share the print atlas.
    let materials = doc["materials"].as_array().unwrap();
    assert!(materials[0]["normalTexture"].is_null());
    assert!(materials[1]["normalTexture"]["index"].is_number());
    assert_eq!(
        materials[0]["pbrMetallicRoughness"]["baseColorTexture"],
        materials[2]["pbrMetallicRoughness"]["baseColorTexture"]
    );
    assert_eq!(doc["images"].as_array().unwrap().len(), 3);

    // The LOD0 vertices' weights sum to 1 on bones of the skeleton, and
    // the sleeves' ends follow the arms.
    let attrs = &doc["meshes"][0]["primitives"][0]["attributes"];
    let weights = read(&doc, &bin, &attrs["WEIGHTS_0"]);
    let bones = read(&doc, &bin, &attrs["JOINTS_0"]);
    let positions = read(&doc, &bin, &attrs["POSITION"]);
    for w in weights.chunks(4) {
        assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-5, "{w:?}");
    }
    assert!(bones.iter().all(|b| (*b as usize) < joints));
    let widest = (0..positions.len() / 3)
        .max_by(|a, b| positions[a * 3].abs().total_cmp(&positions[b * 3].abs()))
        .unwrap();
    let heaviest = (0..4)
        .max_by(|a, b| weights[widest * 4 + a].total_cmp(&weights[widest * 4 + b]))
        .unwrap();
    let joint = doc["skins"][0]["joints"][bones[widest * 4 + heaviest] as usize]
        .as_u64()
        .unwrap();
    let name = doc["nodes"][joint as usize]["name"].as_str().unwrap();
    assert!(name.contains("arm"), "{name}");
    // Tangents are unit with a sign.
    let tangents = read(&doc, &bin, &attrs["TANGENT"]);
    for t in tangents.chunks(4) {
        let len = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-3 && t[3].abs() == 1.0, "{t:?}");
    }
    let walk = &doc["animations"][0];
    assert_eq!(walk["name"], "walk");
    assert!(walk["channels"].as_array().unwrap().len() >= 24);

    // Skinned with the weights exported, the garment follows the walking
    // body: it stays about as far from the skin as at rest.
    assert_eq!(joints, 24, "one root: joints are the bones");
    let form = Form::of_body(&women()).unwrap();
    let walk = Animation::starting(Motion::Walk, 0.0);
    let time = MOTION_START_S + 1.4;
    let transforms = form.transforms(&walk, time).unwrap();
    let body = form.posed(Some(&walk), time);
    let rest: Vec<Vec3> = positions
        .chunks(3)
        .map(|p| Vec3::new(p[0], p[1], p[2]))
        .collect();
    let influences: Vec<Influences> = bones
        .chunks(4)
        .zip(weights.chunks(4))
        .map(|(b, w)| Influences {
            bones: [b[0] as u8, b[1] as u8, b[2] as u8, b[3] as u8],
            weights: [w[0], w[1], w[2], w[3]],
        })
        .collect();
    let moved = skin(&rest, &influences, &transforms);
    let gap = |p: &Vec3, surface: &[Vec3]| {
        surface
            .iter()
            .map(|q| q.distance(*p))
            .fold(f32::INFINITY, f32::min)
    };
    let mut growth: Vec<f32> = rest
        .iter()
        .zip(&moved)
        .step_by(7)
        .map(|(r, m)| (gap(m, &body) - gap(r, &form.positions)).abs())
        .collect();
    growth.sort_by(f32::total_cmp);
    let p90 = growth[growth.len() * 9 / 10];
    assert!(p90 < 0.02, "{p90}");
}

fn women() -> Body {
    Body {
        masculinity: 0.0,
        height_cm: 168.0,
        bust_cm: 90.0,
        waist_cm: 73.0,
        hips_cm: 98.0,
    }
}

#[test]
fn a_shell_unreal_export_without_skin() {
    let (_dir, mut ws) = t_shirt();
    let out = run(
        &mut ws,
        json!({"type": "export.game", "preset": "unreal", "sides": "shell",
            "thickness_mm": 2, "skin": false, "normal_map": 0, "avatar": true}),
    );
    assert_eq!(out["path"], "exports/T-shirt-unreal.glb");
    assert_eq!(out["lods"].as_array().unwrap().len(), 4);
    let (doc, _) = chunks(&std::fs::read(ws.dir().join("exports/T-shirt-unreal.glb")).unwrap());
    assert!(doc["skins"].is_null() && doc["animations"].is_null());
    assert!(doc["images"].is_null());
    assert_eq!(doc["meshes"][0]["name"], "SK_T-shirt_LOD0");
    assert_eq!(doc["materials"][0]["name"], "M_T-shirt");
    assert_eq!(doc["materials"][0]["doubleSided"], false);
    assert_eq!(
        doc["meshes"].as_array().unwrap().last().unwrap()["name"],
        "Avatar"
    );
    // The shell doubles the surface and adds a rim.
    let count = |m: usize| {
        let i = doc["meshes"][m]["primitives"][0]["indices"]
            .as_u64()
            .unwrap() as usize;
        doc["accessors"][i]["count"].as_u64().unwrap() / 3
    };
    let full = out["lods"][0]["triangles"].as_u64().unwrap();
    assert!(count(0) > 2 * full, "{} {full}", count(0));
}

#[test]
fn bad_options_and_bodies_without_a_skeleton_are_refused() {
    let (_dir, mut ws) = t_shirt();
    let err = ws
        .run_step(&Step::from_value(json!({"type": "export.game", "lods": [1, 0.6, 0.7]})).unwrap())
        .unwrap_err();
    assert_eq!(err.message().code, "invalid.lods");

    // The built-in body imported as a file has no skeleton.
    let form = Form::of_body(&women()).unwrap();
    let mut obj = String::new();
    for p in &form.positions {
        obj += &format!("v {} {} {}\n", p.x, p.y, p.z);
    }
    for t in &form.triangles {
        obj += &format!("f {} {} {}\n", t[0] + 1, t[1] + 1, t[2] + 1);
    }
    std::fs::create_dir_all(ws.dir().join("avatars")).unwrap();
    std::fs::write(ws.dir().join("avatars/body.obj"), obj).unwrap();
    run(
        &mut ws,
        json!({"type": "avatar.import", "path": "avatars/body.obj"}),
    );
    let err = ws
        .run_step(&Step::from_value(json!({"type": "export.game"})).unwrap())
        .unwrap_err();
    assert_eq!(err.message().code, "export.no_skeleton");
    run(
        &mut ws,
        json!({"type": "export.game", "skin": false, "normal_map": 0}),
    );
}
