//! Reading a 3D model as one triangle mesh: an imported body (spec 27).
//!
//! - **OBJ** (Wavefront): `v` and `f` lines; polygons split into fans;
//!   negative (relative) indices; everything else ignored.
//! - **glTF 2.0** (Khronos), as `.glb` or as `.gltf` with its buffers
//!   embedded as `data:` URIs: every triangle primitive (lists, strips,
//!   fans) of the default scene, with its nodes' transforms. Buffers in
//!   separate files and compressed geometry (Draco, meshopt) are refused
//!   with a message saying so.
//!
//! Coordinates are returned as written (file units, file axes); the
//! caller decides the scale and the up axis.

use glam::{DMat4, DQuat, DVec3};
use r3d_core::{Localizable, Message};
use serde_json::Value;

/// Most triangles a model may have.
pub const MAX_TRIANGLES: usize = 2_000_000;

/// The formats a body can be imported from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    Obj,
    /// glTF as JSON (buffers embedded).
    Gltf,
    /// glTF binary.
    Glb,
}

impl ModelFormat {
    /// The format of a file name, by its extension.
    pub fn of_name(name: &str) -> Option<Self> {
        let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
        match ext.as_str() {
            "obj" => Some(Self::Obj),
            "gltf" => Some(Self::Gltf),
            "glb" => Some(Self::Glb),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Obj => "OBJ",
            Self::Gltf | Self::Glb => "glTF",
        }
    }
}

/// A model read as one mesh.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Model {
    /// In file units and axes.
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    /// The file has a skeleton (glTF skins); not used yet.
    pub skeleton: bool,
}

/// Why a model cannot be read.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ModelError {
    #[error("this {format} file cannot be read: {detail}")]
    Malformed {
        format: &'static str,
        detail: String,
    },
    #[error("the glTF file keeps its data in separate files; save it as a single .glb")]
    ExternalBuffers,
    #[error("the glTF file is compressed with {extension}; save it without compression")]
    Compressed { extension: String },
    #[error("the file holds no triangles")]
    Empty,
    #[error("the model has {triangles} triangles, more than {limit}; simplify it first")]
    TooLarge { triangles: usize, limit: usize },
}

impl Localizable for ModelError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Malformed { format, detail } => Message::new("model.malformed", text)
                .arg("format", format)
                .arg("detail", detail),
            Self::ExternalBuffers => Message::new("model.external_buffers", text),
            Self::Compressed { extension } => {
                Message::new("model.compressed", text).arg("extension", extension)
            }
            Self::Empty => Message::new("model.empty", text),
            Self::TooLarge { triangles, limit } => Message::new("model.too_large", text)
                .arg("triangles", triangles)
                .arg("limit", limit),
        }
    }
}

/// Read a model.
pub fn read(bytes: &[u8], format: ModelFormat) -> Result<Model, ModelError> {
    let model = match format {
        ModelFormat::Obj => read_obj(bytes)?,
        ModelFormat::Gltf => {
            let json: Value = serde_json::from_slice(bytes).map_err(|e| bad(format, e))?;
            read_gltf(&json, None)?
        }
        ModelFormat::Glb => {
            let (json, bin) = glb_chunks(bytes)?;
            read_gltf(&json, bin)?
        }
    };
    if model.triangles.is_empty() {
        return Err(ModelError::Empty);
    }
    Ok(model)
}

fn bad(format: ModelFormat, detail: impl ToString) -> ModelError {
    ModelError::Malformed {
        format: format.name(),
        detail: detail.to_string(),
    }
}

fn too_large(triangles: usize) -> Result<(), ModelError> {
    if triangles > MAX_TRIANGLES {
        Err(ModelError::TooLarge {
            triangles,
            limit: MAX_TRIANGLES,
        })
    } else {
        Ok(())
    }
}

fn read_obj(bytes: &[u8]) -> Result<Model, ModelError> {
    let obj = |d: String| bad(ModelFormat::Obj, d);
    let text = String::from_utf8_lossy(bytes);
    let mut model = Model::default();
    for (n, line) in text.lines().enumerate() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("v") => {
                let xyz: Vec<f32> = words.take(3).map_while(|w| w.parse().ok()).collect();
                match xyz[..] {
                    [x, y, z] if x.is_finite() && y.is_finite() && z.is_finite() => {
                        model.positions.push([x, y, z]);
                    }
                    _ => return Err(obj(format!("line {}: a vertex needs 3 numbers", n + 1))),
                }
            }
            Some("f") => {
                let count = model.positions.len() as i64;
                let corners = words
                    .map(|w| {
                        let i: i64 = w
                            .split('/')
                            .next()
                            .and_then(|i| i.parse().ok())
                            .ok_or_else(|| obj(format!("line {}: bad face index", n + 1)))?;
                        let index = if i < 0 { count + i } else { i - 1 };
                        u32::try_from(index)
                            .ok()
                            .filter(|i| i64::from(*i) < count)
                            .ok_or_else(|| {
                                obj(format!("line {}: face index {i} out of range", n + 1))
                            })
                    })
                    .collect::<Result<Vec<u32>, _>>()?;
                if let Some((first, rest)) = corners.split_first() {
                    for pair in rest.windows(2) {
                        if let [b, c] = pair {
                            model.triangles.push([*first, *b, *c]);
                        }
                    }
                }
                too_large(model.triangles.len())?;
            }
            _ => {}
        }
    }
    Ok(model)
}

/// The JSON and binary chunks of a GLB file.
fn glb_chunks(bytes: &[u8]) -> Result<(Value, Option<&[u8]>), ModelError> {
    let glb = |d: &str| bad(ModelFormat::Glb, d);
    let word = |at: usize| -> Option<u32> {
        let b = bytes.get(at..at + 4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if bytes.get(0..4) != Some(b"glTF") {
        return Err(glb("not a GLB file"));
    }
    if word(4) != Some(2) {
        return Err(glb("only glTF 2.0 is supported"));
    }
    let (mut json, mut bin) = (None, None);
    let mut at = 12;
    while let (Some(len), Some(kind)) = (word(at), word(at + 4)) {
        let start = at + 8;
        let chunk = bytes
            .get(start..start + len as usize)
            .ok_or_else(|| glb("a chunk runs past the end of the file"))?;
        match kind {
            0x4E4F_534A => json = Some(chunk),
            0x004E_4942 => bin = Some(chunk),
            _ => {}
        }
        at = start + len as usize;
    }
    let json = json.ok_or_else(|| glb("no JSON chunk"))?;
    let value = serde_json::from_slice(json).map_err(|e| bad(ModelFormat::Glb, e))?;
    Ok((value, bin))
}

/// Decode standard base64 (whitespace ignored, padding optional).
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// A glTF document with its buffers.
struct Gltf<'a> {
    json: &'a Value,
    buffers: Vec<std::borrow::Cow<'a, [u8]>>,
}

fn usize_at(v: &Value, key: &str) -> Option<usize> {
    v.get(key)?.as_u64().map(|n| n as usize)
}

impl Gltf<'_> {
    fn list(&self, key: &str) -> &[Value] {
        self.json
            .get(key)
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }

    /// The bytes of accessor `index`'s elements: for each, its start in
    /// the returned slice; the element size in bytes.
    fn accessor(
        &self,
        index: usize,
        want: &str,
        components: &[u64],
    ) -> Result<(Vec<usize>, &[u8], u64), ModelError> {
        let gltf = |d: String| bad(ModelFormat::Gltf, d);
        let a = self
            .list("accessors")
            .get(index)
            .ok_or_else(|| gltf(format!("accessor {index} is missing")))?;
        if a.get("sparse").is_some() {
            return Err(gltf("sparse accessors are not supported".into()));
        }
        let kind = a.get("type").and_then(Value::as_str).unwrap_or_default();
        let component = a.get("componentType").and_then(Value::as_u64).unwrap_or(0);
        if kind != want || !components.contains(&component) {
            return Err(gltf(format!(
                "accessor {index} is {kind} of type {component}, expected {want}"
            )));
        }
        let size = match component {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            _ => 4,
        } * match want {
            "VEC3" => 3,
            _ => 1,
        };
        let count = usize_at(a, "count").unwrap_or(0);
        let view_index = usize_at(a, "bufferView")
            .ok_or_else(|| gltf(format!("accessor {index} has no data")))?;
        let view = self
            .list("bufferViews")
            .get(view_index)
            .ok_or_else(|| gltf(format!("buffer view {view_index} is missing")))?;
        let buffer = self
            .buffers
            .get(usize_at(view, "buffer").unwrap_or(0))
            .ok_or_else(|| gltf(format!("buffer view {view_index} has no buffer")))?;
        let start = usize_at(view, "byteOffset").unwrap_or(0);
        let length = usize_at(view, "byteLength").unwrap_or(0);
        let data = buffer
            .get(start..start.saturating_add(length))
            .ok_or_else(|| gltf(format!("buffer view {view_index} runs past its buffer")))?;
        let stride = usize_at(view, "byteStride").unwrap_or(size).max(size);
        let offset = usize_at(a, "byteOffset").unwrap_or(0);
        let starts: Vec<usize> = (0..count).map(|i| offset + i * stride).collect();
        if starts.last().is_some_and(|s| s + size > data.len()) {
            return Err(gltf(format!("accessor {index} runs past its buffer view")));
        }
        Ok((starts, data, component))
    }

    fn positions(&self, index: usize) -> Result<Vec<DVec3>, ModelError> {
        let (starts, data, _) = self.accessor(index, "VEC3", &[5126])?;
        Ok(starts
            .iter()
            .filter_map(|s| {
                let f = |k: usize| -> Option<f64> {
                    let b = data.get(s + 4 * k..s + 4 * k + 4)?;
                    Some(f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
                };
                Some(DVec3::new(f(0)?, f(1)?, f(2)?))
            })
            .collect())
    }

    fn indices(&self, index: usize) -> Result<Vec<u32>, ModelError> {
        let (starts, data, component) = self.accessor(index, "SCALAR", &[5121, 5123, 5125])?;
        Ok(starts
            .iter()
            .filter_map(|s| match component {
                5121 => data.get(*s).map(|b| u32::from(*b)),
                5123 => data
                    .get(*s..s + 2)
                    .map(|b| u32::from(u16::from_le_bytes([b[0], b[1]]))),
                _ => data
                    .get(*s..s + 4)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            })
            .collect())
    }

    /// Add mesh `index`'s triangle primitives, moved by `to_world`.
    fn add_mesh(&self, index: usize, to_world: DMat4, out: &mut Model) -> Result<(), ModelError> {
        let gltf = |d: String| bad(ModelFormat::Gltf, d);
        let mesh = self
            .list("meshes")
            .get(index)
            .ok_or_else(|| gltf(format!("mesh {index} is missing")))?;
        let primitives = mesh
            .get("primitives")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        for p in primitives {
            let mode = p.get("mode").and_then(Value::as_u64).unwrap_or(4);
            if !(4..=6).contains(&mode) {
                continue; // points and lines
            }
            let Some(position) = p.get("attributes").and_then(|a| usize_at(a, "POSITION")) else {
                continue;
            };
            let positions = self.positions(position)?;
            let indices = match usize_at(p, "indices") {
                Some(i) => self.indices(i)?,
                None => (0..positions.len() as u32).collect(),
            };
            let base = out.positions.len() as u32;
            let n = positions.len() as u32;
            out.positions.extend(
                positions
                    .iter()
                    .map(|v| to_world.transform_point3(*v).as_vec3().to_array()),
            );
            let corner = |k: usize| indices.get(k).copied().filter(|i| *i < n);
            let triangles: Vec<[usize; 3]> = match mode {
                4 => (0..indices.len() / 3)
                    .map(|t| [3 * t, 3 * t + 1, 3 * t + 2])
                    .collect(),
                // Strips alternate their winding; fans share the first.
                5 => (0..indices.len().saturating_sub(2))
                    .map(|t| {
                        if t % 2 == 0 {
                            [t, t + 1, t + 2]
                        } else {
                            [t + 1, t, t + 2]
                        }
                    })
                    .collect(),
                _ => (1..indices.len().saturating_sub(1))
                    .map(|t| [0, t, t + 1])
                    .collect(),
            };
            // A mirroring transform turns the triangles inside out.
            let mirrored = to_world.determinant() < 0.0;
            for [a, b, c] in triangles {
                if let (Some(a), Some(b), Some(c)) = (corner(a), corner(b), corner(c)) {
                    out.triangles.push(if mirrored {
                        [base + a, base + c, base + b]
                    } else {
                        [base + a, base + b, base + c]
                    });
                }
            }
            too_large(out.triangles.len())?;
        }
        Ok(())
    }

    /// Node `index` and its children, under `parent`.
    fn add_node(
        &self,
        index: usize,
        parent: DMat4,
        depth: usize,
        out: &mut Model,
    ) -> Result<(), ModelError> {
        if depth > 64 {
            return Err(bad(
                ModelFormat::Gltf,
                "the node hierarchy is too deep or loops",
            ));
        }
        let node = self
            .list("nodes")
            .get(index)
            .ok_or_else(|| bad(ModelFormat::Gltf, format!("node {index} is missing")))?;
        let numbers = |key: &str| -> Option<Vec<f64>> {
            node.get(key)?
                .as_array()?
                .iter()
                .map(Value::as_f64)
                .collect()
        };
        let local = match numbers("matrix") {
            Some(m) if m.len() == 16 => {
                let mut cols = [0.0; 16];
                cols.copy_from_slice(&m);
                DMat4::from_cols_array(&cols)
            }
            _ => {
                let t = numbers("translation")
                    .filter(|v| v.len() == 3)
                    .map_or(DVec3::ZERO, |v| DVec3::new(v[0], v[1], v[2]));
                let r = numbers("rotation")
                    .filter(|v| v.len() == 4)
                    .map_or(DQuat::IDENTITY, |v| {
                        DQuat::from_xyzw(v[0], v[1], v[2], v[3]).normalize()
                    });
                let s = numbers("scale")
                    .filter(|v| v.len() == 3)
                    .map_or(DVec3::ONE, |v| DVec3::new(v[0], v[1], v[2]));
                DMat4::from_scale_rotation_translation(s, r, t)
            }
        };
        let world = parent * local;
        // A skinned mesh is drawn in its skeleton's space, not its node's.
        if let Some(mesh) = usize_at(node, "mesh") {
            let skinned = node.get("skin").is_some();
            self.add_mesh(mesh, if skinned { DMat4::IDENTITY } else { world }, out)?;
        }
        for child in node
            .get("children")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
        {
            if let Some(c) = child.as_u64() {
                self.add_node(c as usize, world, depth + 1, out)?;
            }
        }
        Ok(())
    }
}

fn read_gltf(json: &Value, bin: Option<&[u8]>) -> Result<Model, ModelError> {
    let gltf = |d: &str| bad(ModelFormat::Gltf, d);
    let version = json
        .get("asset")
        .and_then(|a| a.get("version"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !version.starts_with('2') {
        return Err(gltf("only glTF 2.0 is supported"));
    }
    for ext in json
        .get("extensionsRequired")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(Value::as_str)
    {
        if ext.contains("draco") || ext.contains("meshopt") {
            return Err(ModelError::Compressed {
                extension: ext.to_owned(),
            });
        }
    }
    let mut buffers = Vec::new();
    for (i, b) in json
        .get("buffers")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
    {
        let data = match b.get("uri").and_then(Value::as_str) {
            None if i == 0 => bin
                .map(std::borrow::Cow::Borrowed)
                .ok_or_else(|| gltf("the binary buffer is missing"))?,
            None => return Err(gltf("a buffer has no data")),
            Some(uri) if uri.starts_with("data:") => {
                let (_, payload) = uri
                    .split_once(";base64,")
                    .ok_or_else(|| gltf("a data URI is not base64"))?;
                std::borrow::Cow::Owned(base64(payload).ok_or_else(|| gltf("bad base64 data"))?)
            }
            Some(_) => return Err(ModelError::ExternalBuffers),
        };
        buffers.push(data);
    }
    let doc = Gltf { json, buffers };
    let mut model = Model {
        skeleton: !doc.list("skins").is_empty(),
        ..Model::default()
    };
    let scene = usize_at(json, "scene").unwrap_or(0);
    match doc.list("scenes").get(scene) {
        Some(s) => {
            for node in s
                .get("nodes")
                .and_then(Value::as_array)
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .filter_map(Value::as_u64)
            {
                doc.add_node(node as usize, DMat4::IDENTITY, 0, &mut model)?;
            }
        }
        // No scene: every mesh as it is.
        None => {
            for i in 0..doc.list("meshes").len() {
                doc.add_mesh(i, DMat4::IDENTITY, &mut model)?;
            }
        }
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::scene3d::{Mesh3d, Scene3d};

    fn tetra() -> Mesh3d {
        Mesh3d {
            name: "Avatar".into(),
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            uvs: Vec::new(),
            indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
            material: 0,
        }
    }

    #[test]
    fn our_own_glb_and_obj_read_back() {
        let scene = Scene3d {
            generator: "test".into(),
            name: "t".into(),
            pieces: Vec::new(),
            avatar: Some(tetra()),
            materials: Vec::new(),
        };
        let glb = crate::gltf::to_glb(&scene);
        let m = read(&glb, ModelFormat::Glb).unwrap();
        assert_eq!(m.positions.len(), 4);
        assert_eq!(
            m.triangles,
            vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]
        );
        assert!(!m.skeleton);
        let obj = crate::obj::to_obj(&scene, "t").obj;
        let m = read(obj.as_bytes(), ModelFormat::Obj).unwrap();
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.triangles.len(), 4);
    }

    #[test]
    fn obj_polygons_negative_indices_and_errors() {
        let m = read(
            b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf -4/1/1 -3/2/2 -2 -1\n",
            ModelFormat::Obj,
        )
        .unwrap();
        assert_eq!(m.triangles, vec![[0, 1, 2], [0, 2, 3]]);
        assert!(matches!(
            read(b"v 0 0 0\nf 1 2 3\n", ModelFormat::Obj),
            Err(ModelError::Malformed { .. })
        ));
        assert_eq!(read(b"v 0 0 0\n", ModelFormat::Obj), Err(ModelError::Empty));
    }

    #[test]
    fn gltf_with_embedded_buffers_nodes_and_refusals() {
        // One triangle in a data URI, under a node scaled by 2 and moved.
        let mut bytes = Vec::new();
        for v in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut b64 = String::new();
        for chunk in bytes.chunks(3) {
            let n =
                chunk.iter().fold(0u32, |a, b| a << 8 | u32::from(*b)) << (8 * (3 - chunk.len()));
            for k in 0..=chunk.len() {
                b64.push(ALPHABET[(n >> (18 - 6 * k) & 63) as usize] as char);
            }
        }
        let doc = |uri: &str, extra: &str| {
            format!(
                r#"{{"asset":{{"version":"2.0"}},{extra}"scene":0,"scenes":[{{"nodes":[0]}}],
                "nodes":[{{"children":[1],"translation":[0,1,0]}},{{"mesh":0,"scale":[2,2,2]}}],
                "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}}}}]}}],
                "accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}}],
                "bufferViews":[{{"buffer":0,"byteLength":36}}],
                "buffers":[{{"byteLength":36,"uri":"{uri}"}}]}}"#
            )
        };
        let m = read(
            doc(&format!("data:application/octet-stream;base64,{b64}"), "").as_bytes(),
            ModelFormat::Gltf,
        )
        .unwrap();
        assert_eq!(
            m.positions,
            vec![[0.0, 1.0, 0.0], [2.0, 1.0, 0.0], [0.0, 3.0, 0.0]]
        );
        assert_eq!(m.triangles, vec![[0, 1, 2]]);
        assert_eq!(
            read(doc("body.bin", "").as_bytes(), ModelFormat::Gltf),
            Err(ModelError::ExternalBuffers)
        );
        assert!(matches!(
            read(
                doc(
                    "x.bin",
                    r#""extensionsRequired":["KHR_draco_mesh_compression"],"#
                )
                .as_bytes(),
                ModelFormat::Gltf
            ),
            Err(ModelError::Compressed { .. })
        ));
        assert!(matches!(
            read(b"glTF", ModelFormat::Glb),
            Err(ModelError::Malformed { .. })
        ));
    }
}
