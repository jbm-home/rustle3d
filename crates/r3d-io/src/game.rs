//! The game-ready glTF 2.0 binary (GLB) export (spec 15): meshes with
//! tangents and skin weights, materials with a base-colour and a normal
//! map, the avatar's skeleton as joint nodes in its rest pose, and its
//! motions as animations (Khronos glTF 2.0: skins, animations,
//! `normalTexture`, `TANGENT`; `KHR_materials_sheen`).
//!
//! Skinned mesh nodes are scene roots without a transform, as glTF
//! expects (their joints place them). Everything is in metres, Y up.

use serde_json::{Value, json};

use crate::gltf::{Builder, LINEAR, LINEAR_MIPMAP_LINEAR, glb};
use crate::scene3d::{Image3d, linear_rgb};

const CLAMP_TO_EDGE: u32 = 33071;

/// A mesh ready for an engine: one primitive.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameMesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    /// Unit normals, one per position.
    pub normals: Vec<[f32; 3]>,
    /// Unit tangents and handedness, one per position, or none.
    pub tangents: Vec<[f32; 4]>,
    /// Texture coordinates in the atlas, one per position, or none.
    pub uvs: Vec<[f32; 2]>,
    /// Up to four joints (indices into [`GameScene::joints`]) per
    /// position and their weights (summing to 1; unused slots joint 0,
    /// weight 0), or none for a mesh that is not skinned.
    pub joints: Vec<[u8; 4]>,
    pub weights: Vec<[f32; 4]>,
    /// Triangle corners, counter-clockwise seen from outside.
    pub indices: Vec<u32>,
    /// Index into [`GameScene::materials`].
    pub material: usize,
}

/// A material: the fabric's look, maps by index into
/// [`GameScene::images`].
#[derive(Clone, Debug, PartialEq)]
pub struct GameMaterial {
    pub name: String,
    /// sRGB `0xRRGGBB`, the base colour without a base-colour map.
    pub rgb: u32,
    /// Sheen strength as the app renders it (0 = matte, 1.6 = silk).
    pub sheen: f32,
    pub sheen_roughness: f32,
    pub roughness: f32,
    pub double_sided: bool,
    pub base_colour: Option<usize>,
    pub normal_map: Option<usize>,
}

/// An image embedded in the file, by name.
#[derive(Clone, Debug, PartialEq)]
pub struct GameImage {
    pub name: String,
    pub image: Image3d,
}

/// A joint of the skeleton, in its rest pose.
#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub name: String,
    /// Always lower than the joint's own index.
    pub parent: Option<usize>,
    /// Rest position relative to the parent (m); the rest pose has no
    /// rotation.
    pub translation: [f32; 3],
    /// The inverse of the joint's rest transform in the scene (column
    /// major).
    pub inverse_bind: [f32; 16],
}

/// What an animation channel moves.
#[derive(Clone, Debug, PartialEq)]
pub enum Track {
    /// Local rotations (unit quaternions `x, y, z, w`), one per time.
    Rotation(Vec<[f32; 4]>),
    /// Local translations (m), one per time.
    Translation(Vec<[f32; 3]>),
}

/// An animation: joints' local transforms sampled at the same times,
/// interpolated linearly.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub name: String,
    /// Increasing, from 0 (s).
    pub times: Vec<f32>,
    /// (joint, track).
    pub channels: Vec<(usize, Track)>,
}

/// What a game export holds.
#[derive(Clone, Debug, PartialEq)]
pub struct GameScene {
    pub generator: String,
    pub name: String,
    pub meshes: Vec<GameMesh>,
    pub materials: Vec<GameMaterial>,
    pub images: Vec<GameImage>,
    /// Empty: no skeleton, nothing skinned.
    pub joints: Vec<Joint>,
    pub clips: Vec<Clip>,
    /// Written as the asset's `extras` (the preset and options used).
    pub extras: Value,
}

fn material_json(m: &GameMaterial) -> Value {
    let [r, g, b] = linear_rgb(m.rgb);
    let mut out = json!({
        "name": m.name,
        "pbrMetallicRoughness": {
            "baseColorFactor": [r, g, b, 1.0],
            "metallicFactor": 0.0,
            "roughnessFactor": m.roughness,
        },
        "doubleSided": m.double_sided,
    });
    if let Some(i) = m.base_colour {
        let pbr = &mut out["pbrMetallicRoughness"];
        pbr["baseColorFactor"] = json!([1.0, 1.0, 1.0, 1.0]);
        pbr["baseColorTexture"] = json!({ "index": i });
    }
    if let Some(i) = m.normal_map {
        out["normalTexture"] = json!({ "index": i });
    }
    if m.sheen > 0.0 {
        // As the plain export: the fibres' lighter tint, silk at full
        // strength.
        let strength = (m.sheen / 1.6).min(1.0);
        let tint = |c: f32| (c + (1.0 - c) * 0.6) * strength;
        out["extensions"] = json!({
            "KHR_materials_sheen": {
                "sheenColorFactor": [tint(r), tint(g), tint(b)],
                "sheenRoughnessFactor": m.sheen_roughness,
            },
        });
    }
    out
}

/// The scene as a GLB file.
pub fn to_glb(scene: &GameScene) -> Vec<u8> {
    let mut b = Builder::default();
    let skinned = !scene.joints.is_empty();
    // Joints first, then one node per mesh.
    let mut nodes: Vec<Value> = scene
        .joints
        .iter()
        .map(|j| json!({ "name": j.name, "translation": j.translation }))
        .collect();
    for (i, j) in scene.joints.iter().enumerate() {
        if let Some(node) = j.parent.and_then(|p| nodes.get_mut(p)) {
            match node["children"].as_array_mut() {
                Some(children) => children.push(json!(i)),
                None => node["children"] = json!([i]),
            }
        }
    }
    let mut roots: Vec<usize> = (0..scene.joints.len())
        .filter(|i| scene.joints.get(*i).is_some_and(|j| j.parent.is_none()))
        .collect();
    let mut meshes = Vec::new();
    for m in &scene.meshes {
        let mut attributes = json!({
            "POSITION": b.floats(&m.positions, true),
            "NORMAL": b.floats(&m.normals, false),
        });
        if m.tangents.len() == m.positions.len() && !m.tangents.is_empty() {
            attributes["TANGENT"] = json!(b.floats(&m.tangents, false));
        }
        if m.uvs.len() == m.positions.len() && !m.uvs.is_empty() {
            attributes["TEXCOORD_0"] = json!(b.floats(&m.uvs, false));
        }
        let skin = skinned
            && m.joints.len() == m.positions.len()
            && m.weights.len() == m.positions.len()
            && !m.joints.is_empty();
        if skin {
            attributes["JOINTS_0"] = json!(b.bytes4(&m.joints));
            attributes["WEIGHTS_0"] = json!(b.floats(&m.weights, false));
        }
        meshes.push(json!({
            "name": m.name,
            "primitives": [{
                "attributes": attributes,
                "indices": b.indices(&m.indices),
                "material": m.material,
            }],
        }));
        let mut node = json!({ "name": m.name, "mesh": meshes.len() - 1 });
        if skin {
            node["skin"] = json!(0);
        }
        nodes.push(node);
        roots.push(nodes.len() - 1);
    }
    let materials: Vec<Value> = scene.materials.iter().map(material_json).collect();
    let images: Vec<Value> = scene
        .images
        .iter()
        .map(|i| {
            let view = b.view(&i.image.bytes, None);
            json!({ "name": i.name, "bufferView": view, "mimeType": i.image.mime })
        })
        .collect();
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": scene.generator, "extras": scene.extras },
        "scene": 0,
        "scenes": [{ "name": scene.name, "nodes": roots }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
    });
    if skinned {
        let ibm: Vec<[f32; 16]> = scene.joints.iter().map(|j| j.inverse_bind).collect();
        let accessor = b.floats_in(&ibm, false, None);
        let mut skin = json!({
            "name": "Armature",
            "inverseBindMatrices": accessor,
            "joints": (0..scene.joints.len()).collect::<Vec<_>>(),
        });
        let tops: Vec<usize> = (0..scene.joints.len())
            .filter(|i| scene.joints.get(*i).is_some_and(|j| j.parent.is_none()))
            .collect();
        if let [top] = tops.as_slice() {
            skin["skeleton"] = json!(top);
        }
        doc["skins"] = json!([skin]);
        let animations: Vec<Value> = scene
            .clips
            .iter()
            .filter(|c| c.times.len() >= 2)
            .map(|c| {
                let times: Vec<[f32; 1]> = c.times.iter().map(|t| [*t]).collect();
                let input = b.floats_in(&times, true, None);
                let mut samplers = Vec::new();
                let mut channels = Vec::new();
                for (joint, track) in &c.channels {
                    let (output, path) = match track {
                        Track::Rotation(q) => (b.floats_in(q, false, None), "rotation"),
                        Track::Translation(t) => (b.floats_in(t, false, None), "translation"),
                    };
                    samplers.push(json!({
                        "input": input, "output": output, "interpolation": "LINEAR",
                    }));
                    channels.push(json!({
                        "sampler": samplers.len() - 1,
                        "target": { "node": joint, "path": path },
                    }));
                }
                json!({ "name": c.name, "samplers": samplers, "channels": channels })
            })
            .collect();
        if !animations.is_empty() {
            doc["animations"] = json!(animations);
        }
    }
    if !scene.images.is_empty() {
        doc["textures"] = json!(
            (0..scene.images.len())
                .map(|i| json!({ "sampler": 0, "source": i }))
                .collect::<Vec<_>>()
        );
        doc["images"] = json!(images);
        // The atlas is not a repeating print: clamp at its edges.
        doc["samplers"] = json!([{
            "magFilter": LINEAR, "minFilter": LINEAR_MIPMAP_LINEAR,
            "wrapS": CLAMP_TO_EDGE, "wrapT": CLAMP_TO_EDGE,
        }]);
    }
    if scene.materials.iter().any(|m| m.sheen > 0.0) {
        doc["extensionsUsed"] = json!(["KHR_materials_sheen"]);
    }
    while !b.bin.len().is_multiple_of(4) {
        b.bin.push(0);
    }
    doc["accessors"] = json!(b.accessors);
    doc["bufferViews"] = json!(b.views);
    doc["buffers"] = json!([{ "byteLength": b.bin.len() }]);
    glb(&doc, &b.bin)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn json_of(glb: &[u8]) -> Value {
        let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&glb[20..20 + len]).unwrap()
    }

    #[test]
    fn a_skinned_scene_has_its_skeleton_skin_and_animation() {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let scene = GameScene {
            generator: "test".into(),
            name: "Shirt".into(),
            meshes: vec![GameMesh {
                name: "Shirt_LOD0".into(),
                positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                normals: vec![[0.0, 0.0, 1.0]; 3],
                tangents: vec![[1.0, 0.0, 0.0, 1.0]; 3],
                uvs: vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0]],
                joints: vec![[0, 0, 0, 0], [1, 0, 0, 0], [0, 1, 0, 0]],
                weights: vec![
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [0.5, 0.5, 0.0, 0.0],
                ],
                indices: vec![0, 1, 2],
                material: 0,
            }],
            materials: vec![GameMaterial {
                name: "Shirt".into(),
                rgb: 0x336699,
                sheen: 0.0,
                sheen_roughness: 0.5,
                roughness: 0.8,
                double_sided: true,
                base_colour: None,
                normal_map: Some(0),
            }],
            images: vec![GameImage {
                name: "Shirt_normal".into(),
                image: Image3d {
                    bytes: vec![0x89, b'P', b'N', b'G'],
                    mime: "image/png",
                },
            }],
            joints: vec![
                Joint {
                    name: "pelvis".into(),
                    parent: None,
                    translation: [0.0, 1.0, 0.0],
                    inverse_bind: identity,
                },
                Joint {
                    name: "spine".into(),
                    parent: Some(0),
                    translation: [0.0, 0.2, 0.0],
                    inverse_bind: identity,
                },
            ],
            clips: vec![Clip {
                name: "walk".into(),
                times: vec![0.0, 0.5, 1.0],
                channels: vec![
                    (1, Track::Rotation(vec![[0.0, 0.0, 0.0, 1.0]; 3])),
                    (0, Track::Translation(vec![[0.0, 1.0, 0.0]; 3])),
                ],
            }],
            extras: json!({ "preset": "generic" }),
        };
        let doc = json_of(&to_glb(&scene));
        assert_eq!(doc["nodes"][0]["children"], json!([1]));
        assert_eq!(doc["nodes"][2]["skin"], 0);
        assert_eq!(doc["scenes"][0]["nodes"], json!([0, 2]));
        assert_eq!(doc["skins"][0]["joints"], json!([0, 1]));
        assert_eq!(doc["skins"][0]["skeleton"], 0);
        let ibm =
            &doc["accessors"][doc["skins"][0]["inverseBindMatrices"].as_u64().unwrap() as usize];
        assert_eq!(ibm["type"], "MAT4");
        let attrs = &doc["meshes"][0]["primitives"][0]["attributes"];
        for key in ["TANGENT", "JOINTS_0", "WEIGHTS_0", "TEXCOORD_0"] {
            assert!(attrs[key].is_number(), "{key}");
        }
        assert_eq!(doc["materials"][0]["normalTexture"]["index"], 0);
        let walk = &doc["animations"][0];
        assert_eq!(walk["channels"][1]["target"]["path"], "translation");
        let input = &doc["accessors"][walk["samplers"][0]["input"].as_u64().unwrap() as usize];
        assert_eq!(input["max"], json!([1.0]));
        assert_eq!(doc["asset"]["extras"]["preset"], "generic");
        assert_eq!(doc["samplers"][0]["wrapS"], CLAMP_TO_EDGE);
    }
}
