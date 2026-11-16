//! A draped garment as triangle meshes, ready to write as glTF or OBJ:
//! metres, Y up, right-handed (the glTF convention, as in the simulation).

/// A triangle mesh: one pattern piece, or the avatar.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh3d {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    /// Texture coordinates, one per position, or none (the avatar). For a
    /// piece they are its pattern position: 1 unit = 1 m of fabric, `v`
    /// pointing down the pattern, so a texture repeats at a fixed size on
    /// every piece and follows the grain.
    pub uvs: Vec<[f32; 2]>,
    /// Triangle corners, counter-clockwise seen from outside.
    pub indices: Vec<u32>,
    /// Index into [`Scene3d::materials`].
    pub material: usize,
}

/// A surface's look.
#[derive(Clone, Debug, PartialEq)]
pub struct Material3d {
    pub name: String,
    /// sRGB colour, `0xRRGGBB`.
    pub rgb: u32,
    /// Sheen strength as the app renders it (0 = matte, 1.6 = silk).
    pub sheen: f32,
    /// Microfacet roughness, 0–1.
    pub roughness: f32,
    /// Cloth is a single layer seen from both sides.
    pub double_sided: bool,
}

/// What a 3D export holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene3d {
    /// Written in the file as the generating tool.
    pub generator: String,
    pub name: String,
    pub pieces: Vec<Mesh3d>,
    pub avatar: Option<Mesh3d>,
    pub materials: Vec<Material3d>,
}

impl Mesh3d {
    /// Smooth vertex normals: the sum of the adjacent triangles' area
    /// normals, normalised.
    pub fn normals(&self) -> Vec<[f32; 3]> {
        let mut n = vec![[0.0f32; 3]; self.positions.len()];
        for t in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| i as usize);
            let (Some(pa), Some(pb), Some(pc)) = (
                self.positions.get(a),
                self.positions.get(b),
                self.positions.get(c),
            ) else {
                continue;
            };
            let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
            let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
            let cross = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            for i in [a, b, c] {
                if let Some(acc) = n.get_mut(i) {
                    for k in 0..3 {
                        acc[k] += cross[k];
                    }
                }
            }
        }
        n.iter()
            .map(|v| {
                let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                if len > 0.0 {
                    [v[0] / len, v[1] / len, v[2] / len]
                } else {
                    [0.0, 1.0, 0.0]
                }
            })
            .collect()
    }
}

/// An sRGB `0xRRGGBB` colour in linear RGB (glTF colour factors are
/// linear).
pub fn linear_rgb(rgb: u32) -> [f32; 3] {
    let channel = |shift: u32| {
        let c = ((rgb >> shift) & 0xff) as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(16), channel(8), channel(0)]
}
