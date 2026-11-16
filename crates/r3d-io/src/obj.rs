//! Wavefront OBJ export: one object per pattern piece (and the avatar),
//! with texture coordinates (the pattern position, 1 unit = 1 m) and
//! smooth normals. Geometry only; colours are in the glTF export.

use std::fmt::Write;

use crate::scene3d::Scene3d;

/// The scene as OBJ text.
pub fn to_obj(scene: &Scene3d) -> String {
    let mut out = format!("# {}\n# {}: metres, Y up\n", scene.generator, scene.name);
    // OBJ indices are 1-based and count across the whole file.
    let (mut v0, mut vt0) = (1usize, 1usize);
    for mesh in scene.pieces.iter().chain(scene.avatar.as_ref()) {
        let has_uv = mesh.uvs.len() == mesh.positions.len() && !mesh.uvs.is_empty();
        // Writing to a String cannot fail.
        let _ = writeln!(out, "o {}", mesh.name.replace(char::is_whitespace, "_"));
        for p in &mesh.positions {
            let _ = writeln!(out, "v {} {} {}", p[0], p[1], p[2]);
        }
        for n in mesh.normals() {
            let _ = writeln!(out, "vn {} {} {}", n[0], n[1], n[2]);
        }
        if has_uv {
            for t in &mesh.uvs {
                // OBJ's v points up the texture.
                let _ = writeln!(out, "vt {} {}", t[0], -t[1]);
            }
        }
        for t in mesh.indices.as_chunks::<3>().0 {
            let corner = |i: u32| {
                let v = v0 + i as usize;
                if has_uv {
                    format!("{v}/{}/{v}", vt0 + i as usize)
                } else {
                    format!("{v}//{v}")
                }
            };
            let _ = writeln!(out, "f {} {} {}", corner(t[0]), corner(t[1]), corner(t[2]));
        }
        // Normals are written one per position, so they share its index.
        v0 += mesh.positions.len();
        if has_uv {
            vt0 += mesh.uvs.len();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::gltf::tests::sample;

    #[test]
    fn objects_count_their_indices_across_the_file() {
        let obj = to_obj(&sample());
        assert_eq!(obj.lines().filter(|l| l.starts_with("o ")).count(), 3);
        assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), 12);
        assert_eq!(obj.lines().filter(|l| l.starts_with("vt ")).count(), 8);
        // The second piece starts at vertex 5; the avatar has no UVs.
        assert!(obj.contains("f 5/5/5 6/6/6 7/7/7"));
        assert!(obj.contains("f 9//9 10//10 11//11"));
    }
}
