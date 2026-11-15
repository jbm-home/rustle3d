//! The body pack: a base mesh and named morph targets in one binary file.
//!
//! Layout (little endian):
//! ```text
//! "R3DBODY1"
//! u32 len, utf-8 provenance text
//! u32 vertex count V, V × [f32; 3] positions (metres, Y up, feet at y = 0)
//! u32 triangle count T, T × [u32; 3] indices
//! u32 target count, then per target:
//!   u32 len, utf-8 name, u32 count N, N × (u32 vertex, [f32; 3] offset)
//! ```

use glam::Vec3;

const MAGIC: &[u8; 8] = b"R3DBODY1";

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

/// A sparse per-vertex displacement.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub name: String,
    pub vertices: Vec<u32>,
    pub offsets: Vec<Vec3>,
}

/// Base mesh plus morph targets.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyPack {
    pub provenance: String,
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
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
        put_u32(&mut out, self.targets.len());
        for t in &self.targets {
            put_str(&mut out, &t.name);
            put_u32(&mut out, t.vertices.len());
            for (v, o) in t.vertices.iter().zip(&t.offsets) {
                out.extend_from_slice(&v.to_le_bytes());
                put_vec3(&mut out, *o);
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackError> {
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
            return Err(PackError::Invalid("triangle index out of range".into()));
        }
        let count = r.u32()? as usize;
        let mut targets = Vec::with_capacity(count);
        for _ in 0..count {
            let name = r.string()?;
            let m = r.u32()? as usize;
            let mut vertices = Vec::with_capacity(m);
            let mut offsets = Vec::with_capacity(m);
            for _ in 0..m {
                let v = r.u32()?;
                if v as usize >= n {
                    return Err(PackError::Invalid(format!("{name}: vertex out of range")));
                }
                vertices.push(v);
                offsets.push(r.vec3()?);
            }
            targets.push(Target {
                name,
                vertices,
                offsets,
            });
        }
        if r.at != bytes.len() {
            return Err(PackError::Invalid("trailing bytes".into()));
        }
        Ok(Self {
            provenance,
            positions,
            triangles,
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
            targets: vec![Target {
                name: "t".into(),
                vertices: vec![2],
                offsets: vec![Vec3::Z],
            }],
        };
        let bytes = pack.to_bytes();
        assert_eq!(BodyPack::from_bytes(&bytes), Ok(pack));
        assert_eq!(BodyPack::from_bytes(b"nope"), Err(PackError::Truncated));
        assert_eq!(BodyPack::from_bytes(b"XXXXXXXX"), Err(PackError::BadMagic));
        assert_eq!(
            BodyPack::from_bytes(&bytes[..bytes.len() - 1]),
            Err(PackError::Truncated)
        );
    }
}
