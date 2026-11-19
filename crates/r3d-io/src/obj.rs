//! Wavefront OBJ export: one object per pattern piece (and the avatar),
//! with texture coordinates (from the pattern along the grainline, spec
//! 25) and smooth normals, and its MTL material library: one material per
//! garment with its colour (`Kd`) and, when it has a print, the image
//! beside it (`map_Kd`).

use std::fmt::Write;

use crate::scene3d::{Scene3d, linear_rgb};

/// The files of an OBJ export, to write side by side.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjFiles {
    /// The geometry (`<stem>.obj`).
    pub obj: String,
    /// The materials (`<stem>.mtl`).
    pub mtl: String,
    /// The print images the materials name, and their file names.
    pub textures: Vec<(String, Vec<u8>)>,
}

/// A name OBJ and MTL statements can hold (no spaces).
fn token(name: &str) -> String {
    name.replace(char::is_whitespace, "_")
}

/// The scene as an OBJ file named `<stem>.obj`, with its `<stem>.mtl`
/// and the images (`<stem>_<n>.png|jpg`).
pub fn to_obj(scene: &Scene3d, stem: &str) -> ObjFiles {
    let stem = token(stem);
    let mut out = format!(
        "# {}\n# {}: metres, Y up\nmtllib {stem}.mtl\n",
        scene.generator, scene.name
    );
    // Material names must be unique in the library.
    let names: Vec<String> = scene
        .materials
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{}_{i}", token(&m.name)))
        .collect();
    // OBJ indices are 1-based and count across the whole file.
    let (mut v0, mut vt0) = (1usize, 1usize);
    for mesh in scene.pieces.iter().chain(scene.avatar.as_ref()) {
        let has_uv = mesh.uvs.len() == mesh.positions.len() && !mesh.uvs.is_empty();
        // Writing to a String cannot fail.
        let _ = writeln!(out, "o {}", token(&mesh.name));
        if let Some(name) = names.get(mesh.material) {
            let _ = writeln!(out, "usemtl {name}");
        }
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
    let mut mtl = format!("# {}\n", scene.generator);
    let mut textures = Vec::new();
    for (m, name) in scene.materials.iter().zip(&names) {
        let [r, g, b] = linear_rgb(m.rgb);
        let _ = writeln!(mtl, "\nnewmtl {name}\nKd {r} {g} {b}\nKs 0 0 0\nillum 1");
        if let Some(image) = &m.texture {
            let file = format!("{stem}_{}.{}", textures.len() + 1, image.extension());
            let _ = writeln!(mtl, "map_Kd {file}");
            textures.push((file, image.bytes.clone()));
        }
    }
    ObjFiles {
        obj: out,
        mtl,
        textures,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::gltf::tests::sample;

    #[test]
    fn objects_count_their_indices_across_the_file() {
        let files = to_obj(&sample(), "My skirt");
        let obj = &files.obj;
        assert_eq!(obj.lines().filter(|l| l.starts_with("o ")).count(), 3);
        assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), 12);
        assert_eq!(obj.lines().filter(|l| l.starts_with("vt ")).count(), 8);
        // The second piece starts at vertex 5; the avatar has no UVs.
        assert!(obj.contains("f 5/5/5 6/6/6 7/7/7"));
        assert!(obj.contains("f 9//9 10//10 11//11"));
        assert!(obj.contains("mtllib My_skirt.mtl\n"));
        assert!(obj.contains("o Front\nusemtl Cotton_0\n"));
    }

    #[test]
    fn the_library_names_the_print() {
        let files = to_obj(&sample(), "skirt");
        assert!(files.mtl.contains("newmtl Cotton_0\n"));
        assert!(files.mtl.contains("map_Kd skirt_1.png\n"));
        assert!(files.mtl.contains("newmtl Avatar_1\n"));
        assert_eq!(files.textures.len(), 1);
        assert_eq!(files.textures[0].0, "skirt_1.png");
    }
}
