//! The body pack: a base mesh, named morph targets and a skeleton with its
//! skin weights, in one binary file.
//!
//! Layout (little endian):
//! ```text
//! "R3DBODY2"
//! u32 len, utf-8 provenance text
//! u32 vertex count V, V × [f32; 3] positions (metres, Y up, feet at y = 0)
//! u32 triangle count T, T × [u32; 3] indices
//! u32 joint count J, J × [f32; 3] joint positions
//! u32 bone count B, per bone:
//!   u32 len, utf-8 name, u32 parent (u32::MAX: none), u32 head joint, u32 tail joint
//! V × ([u8; 4] bones, [f32; 4] weights): each vertex's skin weights
//! u32 target count, then per target:
//!   u32 len, utf-8 name, u32 count N, N × (u32 vertex, [f32; 3] offset),
//!   u32 count M, M × (u32 joint, [f32; 3] offset)
//! ```
//!
//! Bones come parents first, at most [`MAX_BONES`] of them.

use glam::Vec3;

const MAGIC: &[u8; 8] = b"R3DBODY2";
/// Most bones a pack may have: the cloth solver tells body parts apart
/// with one bit per bone in a `u32`.
pub const MAX_BONES: usize = 32;
const NO_PARENT: u32 = u32::MAX;

/// Errors reading a pack.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PackError {
    #[error("not a body pack")]
    BadMagic,
    #[error("truncated body pack")]
    Truncated,
    #[error("invalid body pack: {0}")]
    Invalid(String),
}

/// A sparse displacement of vertices and joints.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Target {
    pub name: String,
    pub vertices: Vec<u32>,
    pub offsets: Vec<Vec3>,
    pub joints: Vec<u32>,
    pub joint_offsets: Vec<Vec3>,
}

/// A bone of the skeleton: it turns about its head joint; its tail joint
/// gives its direction.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    pub name: String,
    /// Index of the parent bone, always lower than the bone's own.
    pub parent: Option<u32>,
    pub head: u32,
    pub tail: u32,
}

/// How a vertex follows the bones: up to four of them, weights summing
/// to 1 (unused slots have weight 0).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Influences {
    pub bones: [u8; 4],
    pub weights: [f32; 4],
}

/// Base mesh, skeleton and morph targets.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyPack {
    pub provenance: String,
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    pub joints: Vec<Vec3>,
    pub bones: Vec<Bone>,
    /// One per vertex.
    pub weights: Vec<Influences>,
    pub targets: Vec<Target>,
}

impl BodyPack {
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets.iter().find(|t| t.name == name)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        put_str(&mut out, &self.provenance);
        put_u32(&mut out, self.positions.len());
        for p in &self.positions {
            put_vec3(&mut out, *p);
        }
        put_u32(&mut out, self.triangles.len());
        for t in &self.triangles {
            for i in t {
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
        put_u32(&mut out, self.joints.len());
        for j in &self.joints {
            put_vec3(&mut out, *j);
        }
        put_u32(&mut out, self.bones.len());
        for b in &self.bones {
            put_str(&mut out, &b.name);
            for i in [b.parent.unwrap_or(NO_PARENT), b.head, b.tail] {
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
        for w in &self.weights {
            out.extend_from_slice(&w.bones);
            for x in w.weights {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        put_u32(&mut out, self.targets.len());
        for t in &self.targets {
            put_str(&mut out, &t.name);
            put_u32(&mut out, t.vertices.len());
            for (v, o) in t.vertices.iter().zip(&t.offsets) {
                out.extend_from_slice(&v.to_le_bytes());
                put_vec3(&mut out, *o);
            }
            put_u32(&mut out, t.joints.len());
            for (j, o) in t.joints.iter().zip(&t.joint_offsets) {
                out.extend_from_slice(&j.to_le_bytes());
                put_vec3(&mut out, *o);
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackError> {
        let invalid = |s: &str| PackError::Invalid(s.to_owned());
        let mut r = Reader { bytes, at: 0 };
        if r.take(8)? != MAGIC {
            return Err(PackError::BadMagic);
        }
        let provenance = r.string()?;
        let n = r.u32()? as usize;
        let positions = (0..n).map(|_| r.vec3()).collect::<Result<Vec<_>, _>>()?;
        let t = r.u32()? as usize;
        let triangles = (0..t)
            .map(|_| Ok([r.u32()?, r.u32()?, r.u32()?]))
            .collect::<Result<Vec<_>, PackError>>()?;
        if triangles.iter().flatten().any(|i| *i as usize >= n) {
            return Err(invalid("triangle index out of range"));
        }
        let j = r.u32()? as usize;
        let joints = (0..j).map(|_| r.vec3()).collect::<Result<Vec<_>, _>>()?;
        let b = r.u32()? as usize;
        if b > MAX_BONES {
            return Err(invalid("too many bones"));
        }
        let mut bones = Vec::with_capacity(b);
        for i in 0..b {
            let name = r.string()?;
            let (parent, head, tail) = (r.u32()?, r.u32()?, r.u32()?);
            let parent = (parent != NO_PARENT).then_some(parent);
            if parent.is_some_and(|p| p as usize >= i) {
                return Err(PackError::Invalid(format!(
                    "{name}: parent after its child"
                )));
            }
            if head as usize >= j || tail as usize >= j {
                return Err(PackError::Invalid(format!("{name}: joint out of range")));
            }
            bones.push(Bone {
                name,
                parent,
                head,
                tail,
            });
        }
        let mut weights = Vec::with_capacity(n);
        for _ in 0..n {
            let ids = r.take(4)?;
            let w = Influences {
                bones: [ids[0], ids[1], ids[2], ids[3]],
                weights: [r.f32()?, r.f32()?, r.f32()?, r.f32()?],
            };
            let mut used = w.bones.iter().zip(w.weights).filter(|(_, x)| *x != 0.0);
            if used.any(|(i, _)| *i as usize >= b) || w.weights.iter().any(|x| !x.is_finite()) {
                return Err(invalid("skin weight out of range"));
            }
            weights.push(w);
        }
        let count = r.u32()? as usize;
        let mut targets = Vec::with_capacity(count);
        for _ in 0..count {
            let name = r.string()?;
            let (vertices, offsets) = r.sparse(n, &name)?;
            let (joints, joint_offsets) = r.sparse(j, &name)?;
            targets.push(Target {
                name,
                vertices,
                offsets,
                joints,
                joint_offsets,
            });
        }
        if r.at != bytes.len() {
            return Err(invalid("trailing bytes"));
        }
        Ok(Self {
            provenance,
            positions,
            triangles,
            joints,
            bones,
            weights,
            targets,
        })
    }
}

fn put_u32(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&(n as u32).to_le_bytes());
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    put_u32(out, s.len());
    out.extend_from_slice(s.as_bytes());
}

fn put_vec3(out: &mut Vec<u8>, v: Vec3) {
    for c in v.to_array() {
        out.extend_from_slice(&c.to_le_bytes());
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], PackError> {
        let end = self.at.checked_add(n).ok_or(PackError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(PackError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, PackError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f32(&mut self) -> Result<f32, PackError> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn vec3(&mut self) -> Result<Vec3, PackError> {
        Ok(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }

    fn string(&mut self) -> Result<String, PackError> {
        let n = self.u32()? as usize;
        String::from_utf8(self.take(n)?.to_vec())
            .map_err(|_| PackError::Invalid("non-utf8 text".into()))
    }

    /// A count, then (index below `limit`, offset) pairs.
    fn sparse(&mut self, limit: usize, name: &str) -> Result<(Vec<u32>, Vec<Vec3>), PackError> {
        let m = self.u32()? as usize;
        let mut indices = Vec::with_capacity(m.min(limit));
        let mut offsets = Vec::with_capacity(m.min(limit));
        for _ in 0..m {
            let i = self.u32()?;
            if i as usize >= limit {
                return Err(PackError::Invalid(format!("{name}: index out of range")));
            }
            indices.push(i);
            offsets.push(self.vec3()?);
        }
        Ok((indices, offsets))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_validation() {
        let pack = BodyPack {
            provenance: "test".into(),
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            triangles: vec![[0, 1, 2]],
            joints: vec![Vec3::ZERO, Vec3::Y],
            bones: vec![Bone {
                name: "root".into(),
                parent: None,
                head: 0,
                tail: 1,
            }],
            weights: vec![
                Influences {
                    bones: [0; 4],
                    weights: [1.0, 0.0, 0.0, 0.0],
                };
                3
            ],
            targets: vec![Target {
                name: "t".into(),
                vertices: vec![2],
                offsets: vec![Vec3::Z],
                joints: vec![1],
                joint_offsets: vec![Vec3::X],
            }],
        };
        let bytes = pack.to_bytes();
        assert_eq!(BodyPack::from_bytes(&bytes), Ok(pack.clone()));
        assert_eq!(BodyPack::from_bytes(b"nope"), Err(PackError::Truncated));
        assert_eq!(BodyPack::from_bytes(b"XXXXXXXX"), Err(PackError::BadMagic));
        assert_eq!(
            BodyPack::from_bytes(&bytes[..bytes.len() - 1]),
            Err(PackError::Truncated)
        );
        let mut orphan = pack;
        orphan.bones[0].parent = Some(0);
        assert!(matches!(
            BodyPack::from_bytes(&orphan.to_bytes()),
            Err(PackError::Invalid(_))
        ));
    }
}
