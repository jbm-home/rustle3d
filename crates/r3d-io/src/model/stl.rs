//! STL (stereolithography), binary or ASCII: a list of triangles, each
//! with its own three corners. Corners written with the same coordinates
//! are welded into one vertex, so the body's check sees a connected
//! surface rather than loose triangles. Facet normals, the header and
//! the attribute bytes are ignored.

use super::{Model, ModelError, ModelFormat, bad, too_large};
use std::collections::HashMap;

/// Read an STL file, binary or ASCII.
pub(super) fn read(bytes: &[u8]) -> Result<Model, ModelError> {
    let corners = match binary_count(bytes) {
        Some(count) => binary(bytes, count)?,
        None => ascii(bytes)?,
    };
    Ok(weld(&corners))
}

/// The triangle count of a binary file whose size matches it. A binary
/// file may start with `solid` like an ASCII one; its size tells them
/// apart.
fn binary_count(bytes: &[u8]) -> Option<usize> {
    let count = u32::from_le_bytes(bytes.get(80..84)?.try_into().ok()?) as usize;
    let size = count.checked_mul(50)?.checked_add(84)?;
    (size == bytes.len()).then_some(count)
}

fn binary(bytes: &[u8], count: usize) -> Result<Vec<[f32; 3]>, ModelError> {
    too_large(count)?;
    let stl = |d: String| bad(ModelFormat::Stl, d);
    let mut corners = Vec::with_capacity(count * 3);
    let (facets, _) = bytes.get(84..).unwrap_or_default().as_chunks::<50>();
    for (n, facet) in facets.iter().enumerate() {
        // 12 bytes of normal, then three corners of three floats.
        let (floats, _) = facet.get(12..48).unwrap_or_default().as_chunks::<4>();
        for corner in floats.chunks(3) {
            let mut xyz = [0.0f32; 3];
            for (v, b) in xyz.iter_mut().zip(corner) {
                *v = f32::from_le_bytes(*b);
            }
            if !xyz.iter().all(|v| v.is_finite()) {
                return Err(stl(format!("triangle {}: a corner is not a number", n + 1)));
            }
            corners.push(xyz);
        }
    }
    Ok(corners)
}

fn ascii(bytes: &[u8]) -> Result<Vec<[f32; 3]>, ModelError> {
    let stl = |d: String| bad(ModelFormat::Stl, d);
    let text = String::from_utf8_lossy(bytes);
    if !text.trim_start().starts_with("solid") {
        return Err(stl("neither a binary nor an ASCII STL file".into()));
    }
    let mut corners = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let mut words = line.split_whitespace();
        if words.next() != Some("vertex") {
            continue;
        }
        let xyz: Vec<f32> = words.take(3).map_while(|w| w.parse().ok()).collect();
        match xyz[..] {
            [x, y, z] if x.is_finite() && y.is_finite() && z.is_finite() => {
                corners.push([x, y, z]);
            }
            _ => return Err(stl(format!("line {}: a vertex needs 3 numbers", n + 1))),
        }
        too_large(corners.len() / 3)?;
    }
    if corners.len() % 3 != 0 {
        return Err(stl("a facet does not have three vertices".into()));
    }
    Ok(corners)
}

/// One vertex per distinct position (`-0.0` and `0.0` alike), triangles
/// in the file's order.
fn weld(corners: &[[f32; 3]]) -> Model {
    let mut model = Model::default();
    let mut index: HashMap<[u32; 3], u32> = HashMap::new();
    let mut ids = corners.iter().map(|p| {
        let key = p.map(|v| (v + 0.0).to_bits());
        *index.entry(key).or_insert_with(|| {
            model.positions.push(*p);
            (model.positions.len() - 1) as u32
        })
    });
    while let (Some(a), Some(b), Some(c)) = (ids.next(), ids.next(), ids.next()) {
        model.triangles.push([a, b, c]);
    }
    model
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::super::{ModelError, ModelFormat, read};

    const SQUARE: &str = "solid square
  facet normal 0 0 1
    outer loop
      vertex 0 0 0
      vertex 1 0 0
      vertex 1 1 0
    endloop
  endfacet
  facet normal 0 0 1
    outer loop
      vertex 0 0 -0
      vertex 1 1 0
      vertex 0 1 0
    endloop
  endfacet
endsolid square
";

    fn binary(triangles: &[[[f32; 3]; 3]]) -> Vec<u8> {
        // A header starting with "solid", as some exporters write.
        let mut b = b"solid binary".to_vec();
        b.resize(80, 0);
        b.extend((triangles.len() as u32).to_le_bytes());
        for t in triangles {
            b.extend([0u8; 12]);
            for v in t.iter().flatten() {
                b.extend(v.to_le_bytes());
            }
            b.extend([0u8; 2]);
        }
        b
    }

    #[test]
    fn ascii_and_binary_weld_shared_corners() {
        let m = read(SQUARE.as_bytes(), ModelFormat::Stl).unwrap();
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.triangles, vec![[0, 1, 2], [0, 2, 3]]);
        let tris = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        ];
        let m = read(&binary(&tris), ModelFormat::Stl).unwrap();
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn broken_files_are_refused() {
        assert!(matches!(
            read(b"not an stl", ModelFormat::Stl),
            Err(ModelError::Malformed { .. })
        ));
        assert!(matches!(
            read(b"solid x\nvertex 0 0\nendsolid\n", ModelFormat::Stl),
            Err(ModelError::Malformed { .. })
        ));
        assert!(matches!(
            read(b"solid x\nvertex 0 0 0\nendsolid\n", ModelFormat::Stl),
            Err(ModelError::Malformed { .. })
        ));
        assert_eq!(
            read(b"solid empty\nendsolid empty\n", ModelFormat::Stl),
            Err(ModelError::Empty)
        );
        let nan = [[[f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]];
        assert!(matches!(
            read(&binary(&nan), ModelFormat::Stl),
            Err(ModelError::Malformed { .. })
        ));
    }
}
