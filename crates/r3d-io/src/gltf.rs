//! glTF 2.0 binary (GLB) export (Khronos glTF 2.0 specification, with the
//! `KHR_materials_sheen` extension for the fabric's sheen).
//!
//! One node and mesh per pattern piece, named after it, under a "garment"
//! node; the avatar, if included, as its own node. Positions, normals and
//! texture coordinates (from the pattern along the grainline, spec 25) per
//! vertex; 32-bit indices. A garment's print is embedded as an image
//! (PNG or JPEG) with a repeating sampler, its base colour texture.

use serde_json::{Value, json};

use crate::scene3d::{Mesh3d, Scene3d, linear_rgb};

const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;
const LINEAR: u32 = 9729;
const LINEAR_MIPMAP_LINEAR: u32 = 9987;
const REPEAT: u32 = 10497;

/// The binary buffer and the JSON arrays that describe it.
#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Builder {
    /// Append `bytes` as a buffer view (4-byte aligned) for vertex data
    /// or indices (`target`) or an image (none); its index.
    fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let offset = self.bin.len();
        self.bin.extend_from_slice(bytes);
        let mut view = json!({
            "buffer": 0, "byteOffset": offset, "byteLength": bytes.len(),
        });
        if let Some(target) = target {
            view["target"] = json!(target);
        }
        self.views.push(view);
        self.views.len() - 1
    }

    /// Vectors of `N` floats as an accessor (with bounds when asked, as
    /// glTF requires for positions); its index.
    fn floats<const N: usize>(&mut self, data: &[[f32; N]], bounds: bool) -> usize {
        let bytes: Vec<u8> = data
            .iter()
            .flatten()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let view = self.view(&bytes, Some(ARRAY_BUFFER));
        let kind = match N {
            2 => "VEC2",
            3 => "VEC3",
            _ => "SCALAR",
        };
        let mut accessor = json!({
            "bufferView": view, "componentType": FLOAT, "count": data.len(), "type": kind,
        });
        if bounds && !data.is_empty() {
            let mut lo = [f32::MAX; N];
            let mut hi = [f32::MIN; N];
            for v in data {
                for k in 0..N {
                    lo[k] = lo[k].min(v[k]);
                    hi[k] = hi[k].max(v[k]);
                }
            }
            accessor["min"] = json!(lo.to_vec());
            accessor["max"] = json!(hi.to_vec());
        }
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }

    fn indices(&mut self, data: &[u32]) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|i| i.to_le_bytes()).collect();
        let view = self.view(&bytes, Some(ELEMENT_ARRAY_BUFFER));
        self.accessors.push(json!({
            "bufferView": view, "componentType": UNSIGNED_INT, "count": data.len(), "type": "SCALAR",
        }));
        self.accessors.len() - 1
    }

    /// A mesh with one primitive.
    fn mesh(&mut self, m: &Mesh3d) -> Value {
        let mut attributes = json!({
            "POSITION": self.floats(&m.positions, true),
            "NORMAL": self.floats(&m.normals(), false),
        });
        if m.uvs.len() == m.positions.len() && !m.uvs.is_empty() {
            attributes["TEXCOORD_0"] = json!(self.floats(&m.uvs, false));
        }
        json!({
            "name": m.name,
            "primitives": [{
                "attributes": attributes,
                "indices": self.indices(&m.indices),
                "material": m.material,
            }],
        })
    }
}

/// The scene as a GLB file.
pub fn to_glb(scene: &Scene3d) -> Vec<u8> {
    let mut b = Builder::default();
    let mut meshes = Vec::new();
    let mut nodes = vec![json!({ "name": scene.name, "children": [] })];
    for piece in &scene.pieces {
        meshes.push(b.mesh(piece));
        nodes.push(json!({ "name": piece.name, "mesh": meshes.len() - 1 }));
    }
    nodes[0]["children"] = json!((1..nodes.len()).collect::<Vec<_>>());
    let mut roots = vec![0];
    if let Some(avatar) = &scene.avatar {
        meshes.push(b.mesh(avatar));
        nodes.push(json!({ "name": avatar.name, "mesh": meshes.len() - 1 }));
        roots.push(nodes.len() - 1);
    }
    let mut images = Vec::new();
    let materials: Vec<Value> = scene
        .materials
        .iter()
        .map(|m| {
            let [r, g, bl] = linear_rgb(m.rgb);
            let mut material = json!({
                "name": m.name,
                "pbrMetallicRoughness": {
                    "baseColorFactor": [r, g, bl, 1.0],
                    "metallicFactor": 0.0,
                    "roughnessFactor": m.roughness,
                },
                "doubleSided": m.double_sided,
            });
            if let Some(image) = &m.texture {
                let view = b.view(&image.bytes, None);
                images.push(json!({ "bufferView": view, "mimeType": image.mime }));
                // One texture per image, all through the one sampler.
                let pbr = &mut material["pbrMetallicRoughness"];
                pbr["baseColorFactor"] = json!([1.0, 1.0, 1.0, 1.0]);
                pbr["baseColorTexture"] = json!({ "index": images.len() - 1 });
            }
            if m.sheen > 0.0 {
                // The sheen takes the fibres' lighter tint; the app's
                // strongest sheen (silk, 1.6) is full strength.
                let strength = (m.sheen / 1.6).min(1.0);
                let tint = |c: f32| (c + (1.0 - c) * 0.6) * strength;
                material["extensions"] = json!({
                    "KHR_materials_sheen": {
                        "sheenColorFactor": [tint(r), tint(g), tint(bl)],
                        "sheenRoughnessFactor": m.sheen_roughness,
                    },
                });
            }
            material
        })
        .collect();
    let uses_sheen = scene.materials.iter().any(|m| m.sheen > 0.0);
    while !b.bin.len().is_multiple_of(4) {
        b.bin.push(0);
    }
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": scene.generator },
        "scene": 0,
        "scenes": [{ "name": scene.name, "nodes": roots }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": b.accessors,
        "bufferViews": b.views,
        "buffers": [{ "byteLength": b.bin.len() }],
    });
    if uses_sheen {
        doc["extensionsUsed"] = json!(["KHR_materials_sheen"]);
    }
    if !images.is_empty() {
        doc["textures"] = json!(
            (0..images.len())
                .map(|i| json!({ "sampler": 0, "source": i }))
                .collect::<Vec<_>>()
        );
        doc["images"] = json!(images);
        doc["samplers"] = json!([{
            "magFilter": LINEAR, "minFilter": LINEAR_MIPMAP_LINEAR, "wrapS": REPEAT, "wrapT": REPEAT,
        }]);
    }
    let mut json = doc.to_string().into_bytes();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len() + 8 + b.bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out.extend_from_slice(&(b.bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&b.bin);
    out
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::scene3d::{Image3d, Material3d};

    pub(crate) fn sample() -> Scene3d {
        let square = |name: &str, y: f32| Mesh3d {
            name: name.into(),
            positions: vec![
                [0.0, y, 0.0],
                [1.0, y, 0.0],
                [1.0, y + 1.0, 0.0],
                [0.0, y + 1.0, 0.0],
            ],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, -1.0], [0.0, -1.0]],
            indices: vec![0, 1, 2, 0, 2, 3],
            material: 0,
        };
        Scene3d {
            generator: "test".into(),
            name: "Skirt".into(),
            pieces: vec![square("Front", 0.0), square("Back", 2.0)],
            avatar: Some(Mesh3d {
                uvs: Vec::new(),
                material: 1,
                ..square("Avatar", 4.0)
            }),
            materials: vec![
                Material3d {
                    name: "Cotton".into(),
                    rgb: 0xE4DCCD,
                    sheen: 0.6,
                    sheen_roughness: 0.6,
                    roughness: 0.85,
                    double_sided: true,
                    texture: Some(Image3d {
                        bytes: crate::texture::encode_png(&r3d_core::print::builtin(
                            r3d_core::BuiltinPrint::Stripes,
                            0x204080,
                        )),
                        mime: "image/png",
                    }),
                },
                Material3d {
                    name: "Avatar".into(),
                    rgb: 0xC8C8C8,
                    sheen: 0.0,
                    sheen_roughness: 0.0,
                    roughness: 0.7,
                    double_sided: false,
                    texture: None,
                },
            ],
        }
    }

    #[test]
    fn a_glb_holds_its_chunks_and_every_piece() {
        let glb = to_glb(&sample());
        assert_eq!(&glb[0..4], b"glTF");
        let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
        assert_eq!(word(4), 2);
        assert_eq!(word(8), glb.len());
        let json_len = word(12);
        assert_eq!(&glb[16..20], b"JSON");
        let doc: Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
        let bin_at = 20 + json_len;
        assert_eq!(&glb[bin_at + 4..bin_at + 8], b"BIN\0");
        assert_eq!(
            word(bin_at),
            doc["buffers"][0]["byteLength"].as_u64().unwrap() as usize
        );
        assert_eq!(word(bin_at) % 4, 0);

        // Two pieces under the garment node, the avatar on its own.
        assert_eq!(doc["nodes"][0]["children"], json!([1, 2]));
        assert_eq!(doc["scenes"][0]["nodes"], json!([0, 3]));
        assert_eq!(doc["nodes"][1]["name"], "Front");
        // Positions have bounds; the avatar has no texture coordinates.
        let front = &doc["meshes"][0]["primitives"][0];
        let pos = &doc["accessors"][front["attributes"]["POSITION"].as_u64().unwrap() as usize];
        assert_eq!(pos["min"], json!([0.0, 0.0, 0.0]));
        assert!(front["attributes"]["TEXCOORD_0"].is_number());
        assert!(doc["meshes"][2]["primitives"][0]["attributes"]["TEXCOORD_0"].is_null());
        assert_eq!(doc["extensionsUsed"], json!(["KHR_materials_sheen"]));
        assert!(doc["materials"][1]["extensions"].is_null());
        // The print is embedded, through a repeating sampler.
        let pbr = &doc["materials"][0]["pbrMetallicRoughness"];
        assert_eq!(pbr["baseColorTexture"]["index"], 0);
        assert_eq!(pbr["baseColorFactor"], json!([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(doc["textures"][0]["source"], 0);
        assert_eq!(doc["images"][0]["mimeType"], "image/png");
        assert_eq!(doc["samplers"][0]["wrapS"], REPEAT);
        let image_view =
            &doc["bufferViews"][doc["images"][0]["bufferView"].as_u64().unwrap() as usize];
        assert!(image_view["target"].is_null());
        let at = bin_at + 8 + image_view["byteOffset"].as_u64().unwrap() as usize;
        assert_eq!(&glb[at..at + 4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn colours_are_written_linear() {
        let [r, ..] = linear_rgb(0xFF0000);
        assert!((r - 1.0).abs() < 1e-6);
        let [g, ..] = linear_rgb(0x800000);
        assert!((g - 0.2158).abs() < 1e-3);
    }
}
