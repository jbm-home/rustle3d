//! Texture baking for game exports: tangent frames, and a detailed
//! mesh's shading or colours written into an image through the texture
//! coordinates it shares with a simpler mesh.
//!
//! The simplified levels of detail keep the full mesh's texture
//! coordinates (`decimate` keeps vertices), so a point of the image is
//! the same point of the cloth on both meshes: the full mesh's normal at
//! a texel is read there and written in the simple mesh's tangent frame
//! (appearance-preserving simplification, Cohen, Olano & Manocha,
//! SIGGRAPH 1998). Tangents follow Lengyel's per-triangle derivatives of
//! position along the texture axes, orthogonalised against the normal
//! (Mikkelsen, "Simulation of Wrinkled Surfaces Revisited", 2008, for
//! the frame a renderer rebuilds: bitangent = (normal × tangent) · w).
//!
//! Conventions are glTF's: texture coordinate `v` grows down the image
//! (row 0 at `v` = 0) and a normal map's green channel points up the
//! image, towards −`v`.

use glam::{Vec2, Vec3};

/// A mesh drawn into a texture: corners with their texture coordinates
/// (in [0, 1]²) and their attributes.
#[derive(Debug, Clone, Copy)]
pub struct Sheet<'a> {
    pub positions: &'a [Vec3],
    /// Unit vertex normals.
    pub normals: &'a [Vec3],
    pub uvs: &'a [Vec2],
    pub triangles: &'a [[u32; 3]],
}

/// A flat normal (0, 0, 1) in a normal map.
pub const FLAT: [u8; 4] = [128, 128, 255, 255];

/// Per vertex, the tangent (unit, along +`u`, perpendicular to the
/// normal) and in `w` the sign that makes `(normal × tangent) · w` point
/// towards −`v`.
pub fn tangents(mesh: &Sheet<'_>) -> Vec<[f32; 4]> {
    let n = mesh.positions.len();
    let mut along_u = vec![Vec3::ZERO; n];
    let mut along_v = vec![Vec3::ZERO; n];
    for t in mesh.triangles {
        let (Some(p), Some(uv)) = (corners(mesh.positions, *t), corners(mesh.uvs, *t)) else {
            continue;
        };
        let (e1, e2) = (p[1] - p[0], p[2] - p[0]);
        let (d1, d2) = (uv[1] - uv[0], uv[2] - uv[0]);
        let det = d1.perp_dot(d2);
        if det.abs() < 1e-20 {
            continue;
        }
        let su = (e1 * d2.y - e2 * d1.y) / det;
        let sv = (e2 * d1.x - e1 * d2.x) / det;
        // Weighted by the triangle's area (|det| in texture space times
        // the stretch), as the sums of unnormalised derivatives are.
        let w = det.abs();
        for v in t {
            if let Some(a) = along_u.get_mut(*v as usize) {
                *a += su * w;
            }
            if let Some(a) = along_v.get_mut(*v as usize) {
                *a += sv * w;
            }
        }
    }
    (0..n)
        .map(|i| {
            let normal = mesh.normals.get(i).copied().unwrap_or(Vec3::Z);
            let su = along_u.get(i).copied().unwrap_or_default();
            let sv = along_v.get(i).copied().unwrap_or_default();
            let t = (su - normal * normal.dot(su))
                .try_normalize()
                .unwrap_or_else(|| normal.any_orthonormal_vector());
            let w = if normal.cross(t).dot(-sv) < 0.0 {
                -1.0
            } else {
                1.0
            };
            [t.x, t.y, t.z, w]
        })
        .collect()
}

fn corners<T: Copy>(values: &[T], t: [u32; 3]) -> Option<[T; 3]> {
    Some([
        *values.get(t[0] as usize)?,
        *values.get(t[1] as usize)?,
        *values.get(t[2] as usize)?,
    ])
}

/// Visit the texels of a `size` × `size` image whose centres lie in a
/// triangle of `mesh`: texel index, triangle corners, barycentric
/// weights. A texel on an edge shared by two triangles is visited for
/// both (the later wins), so none falls between them.
fn raster(
    uvs: &[Vec2],
    triangles: &[[u32; 3]],
    size: u32,
    mut visit: impl FnMut(usize, [u32; 3], Vec3),
) {
    let s = size as f32;
    for t in triangles {
        let Some(c) = corners(uvs, *t) else {
            continue;
        };
        let c = c.map(|p| p * s);
        let area = (c[1] - c[0]).perp_dot(c[2] - c[0]);
        if area.abs() < 1e-12 {
            continue;
        }
        let lo = c[0].min(c[1]).min(c[2]).floor().max(Vec2::ZERO);
        let hi = c[0].max(c[1]).max(c[2]).ceil().min(Vec2::splat(s));
        let eps = -1e-4 * area.abs();
        for y in lo.y as u32..hi.y as u32 {
            for x in lo.x as u32..hi.x as u32 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let w0 = (c[1] - p).perp_dot(c[2] - p) / area;
                let w1 = (c[2] - p).perp_dot(c[0] - p) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 * area.abs() >= eps && w1 * area.abs() >= eps && w2 * area.abs() >= eps {
                    visit((y * size + x) as usize, *t, Vec3::new(w0, w1, w2));
                }
            }
        }
    }
}

fn blend(values: [Vec3; 3], w: Vec3) -> Vec3 {
    values[0] * w.x + values[1] * w.y + values[2] * w.z
}

/// The normal map that gives `low` the shading of `high` (both drawn in
/// the same texture space), `size` × `size` RGBA, with `low`'s
/// `tangents` (as [`tangents`] makes them). Texels outside the meshes
/// are padded from their neighbours over `padding` texels, then flat.
pub fn bake_normals(
    high: &Sheet<'_>,
    low: &Sheet<'_>,
    low_tangents: &[[f32; 4]],
    size: u32,
    padding: u32,
) -> Vec<u8> {
    let texels = (size as usize) * (size as usize);
    let mut detail: Vec<Option<Vec3>> = vec![None; texels];
    raster(high.uvs, high.triangles, size, |i, t, w| {
        if let (Some(n), Some(slot)) = (corners(high.normals, t), detail.get_mut(i)) {
            *slot = blend(n, w).try_normalize();
        }
    });
    let mut rgba = vec![0u8; texels * 4];
    let mut covered = vec![false; texels];
    raster(low.uvs, low.triangles, size, |i, t, w| {
        let (Some(n), Some(tg)) = (corners(low.normals, t), corners(low_tangents, t)) else {
            return;
        };
        let Some(normal) = blend(n, w).try_normalize() else {
            return;
        };
        let xyz = tg.map(|a| Vec3::new(a[0], a[1], a[2]));
        let sign = if tg[0][3] * w.x + tg[1][3] * w.y + tg[2][3] * w.z < 0.0 {
            -1.0
        } else {
            1.0
        };
        let tangent = (blend(xyz, w) - normal * normal.dot(blend(xyz, w)))
            .try_normalize()
            .unwrap_or_else(|| normal.any_orthonormal_vector());
        let bitangent = normal.cross(tangent) * sign;
        let target = detail.get(i).copied().flatten().unwrap_or(normal);
        let local = Vec3::new(
            target.dot(tangent),
            target.dot(bitangent),
            target.dot(normal),
        )
        .try_normalize()
        .unwrap_or(Vec3::Z);
        let byte = |c: f32| ((c * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
        if let Some(px) = rgba.get_mut(i * 4..i * 4 + 4) {
            px.copy_from_slice(&[byte(local.x), byte(local.y), byte(local.z), 255]);
        }
        if let Some(c) = covered.get_mut(i) {
            *c = true;
        }
    });
    pad(&mut rgba, &mut covered, size, padding, FLAT);
    rgba
}

/// The image of `mesh`'s colours: at each texel inside it, `colour` of
/// the point's other coordinates `attribute` (one per vertex, e.g. a
/// print's repeating coordinates), `size` × `size` RGBA, padded as in
/// [`bake_normals`] and `fill` beyond.
pub fn bake_colour(
    uvs: &[Vec2],
    triangles: &[[u32; 3]],
    attribute: &[Vec2],
    size: u32,
    padding: u32,
    fill: [u8; 4],
    colour: impl Fn(Vec2) -> [u8; 4],
) -> Vec<u8> {
    let texels = (size as usize) * (size as usize);
    let mut rgba = vec![0u8; texels * 4];
    let mut covered = vec![false; texels];
    raster(uvs, triangles, size, |i, t, w| {
        let Some(a) = corners(attribute, t) else {
            return;
        };
        let at = a[0] * w.x + a[1] * w.y + a[2] * w.z;
        if let Some(px) = rgba.get_mut(i * 4..i * 4 + 4) {
            px.copy_from_slice(&colour(at));
        }
        if let Some(c) = covered.get_mut(i) {
            *c = true;
        }
    });
    pad(&mut rgba, &mut covered, size, padding, fill);
    rgba
}

/// Grow the covered texels outwards `passes` times (each new texel the
/// average of its covered neighbours, so mipmaps and filtering near a
/// piece's edge do not pick up the background), then `fill` the rest.
fn pad(rgba: &mut [u8], covered: &mut [bool], size: u32, passes: u32, fill: [u8; 4]) {
    let s = size as i64;
    for _ in 0..passes {
        let mut grown = Vec::new();
        for y in 0..s {
            for x in 0..s {
                let i = (y * s + x) as usize;
                if covered.get(i).copied().unwrap_or(true) {
                    continue;
                }
                let mut sum = [0u32; 4];
                let mut count = 0;
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= s || ny >= s {
                        continue;
                    }
                    let j = (ny * s + nx) as usize;
                    if covered.get(j).copied().unwrap_or(false)
                        && let Some(px) = rgba.get(j * 4..j * 4 + 4)
                    {
                        for (acc, c) in sum.iter_mut().zip(px) {
                            *acc += u32::from(*c);
                        }
                        count += 1;
                    }
                }
                if count > 0 {
                    grown.push((i, sum.map(|c| ((c + count / 2) / count) as u8)));
                }
            }
        }
        if grown.is_empty() {
            break;
        }
        for (i, px) in grown {
            if let Some(dst) = rgba.get_mut(i * 4..i * 4 + 4) {
                dst.copy_from_slice(&px);
            }
            if let Some(c) = covered.get_mut(i) {
                *c = true;
            }
        }
    }
    for (px, c) in rgba.as_chunks_mut::<4>().0.iter_mut().zip(covered.iter()) {
        if !c {
            *px = fill;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// A `k` × `k` grid over the texture square, its height `h(x, y)`
    /// with `y` down the image; smooth normals.
    fn sheet(
        k: u32,
        h: impl Fn(f32, f32) -> f32,
    ) -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec2>, Vec<[u32; 3]>) {
        let mut p = Vec::new();
        let mut uv = Vec::new();
        for j in 0..=k {
            for i in 0..=k {
                let (x, y) = (i as f32 / k as f32, j as f32 / k as f32);
                // The image's up (−v) is +y in 3D: z towards the viewer.
                p.push(Vec3::new(x, -y, h(x, y)));
                uv.push(Vec2::new(x, y));
            }
        }
        let id = |i: u32, j: u32| j * (k + 1) + i;
        let mut t = Vec::new();
        for j in 0..k {
            for i in 0..k {
                // Counter-clockwise seen from +z.
                t.push([id(i, j), id(i, j + 1), id(i + 1, j + 1)]);
                t.push([id(i, j), id(i + 1, j + 1), id(i + 1, j)]);
            }
        }
        let mut n = vec![Vec3::ZERO; p.len()];
        for tri in &t {
            let c = tri.map(|v| p[v as usize]);
            let f = (c[1] - c[0]).cross(c[2] - c[0]);
            for v in tri {
                n[*v as usize] += f;
            }
        }
        let n = n.iter().map(|v| v.normalize()).collect();
        (p, n, uv, t)
    }

    #[test]
    fn tangents_point_along_u_and_the_bitangent_up_the_image() {
        let (p, n, uv, t) = sheet(4, |_, _| 0.0);
        let mesh = Sheet {
            positions: &p,
            normals: &n,
            uvs: &uv,
            triangles: &t,
        };
        for (tg, normal) in tangents(&mesh).iter().zip(&n) {
            assert!((Vec3::new(tg[0], tg[1], tg[2]) - Vec3::X).length() < 1e-5);
            assert!((normal.cross(Vec3::X) * tg[3] - Vec3::Y).length() < 1e-5);
        }
    }

    #[test]
    fn a_flat_sheet_bakes_flat_and_a_wrinkled_one_does_not() {
        let (p, n, uv, t) = sheet(32, |_, _| 0.0);
        let low_t = vec![
            [0, (32 + 1) * 32, (32 + 1) * 33 - 1],
            [0, (32 + 1) * 33 - 1, 32],
        ];
        let flat = Sheet {
            positions: &p,
            normals: &n,
            uvs: &uv,
            triangles: &t,
        };
        let low_n = vec![Vec3::Z; p.len()];
        let low = Sheet {
            normals: &low_n,
            triangles: &low_t,
            ..flat
        };
        let tg = tangents(&low);
        let map = bake_normals(&flat, &low, &tg, 32, 2);
        assert!(map.chunks(4).all(|px| px == FLAT), "{:?}", &map[..8]);

        // Folds running across the image (along u): their slopes tilt
        // the normals up and down the image (green), not sideways.
        let (wp, wn, ..) = sheet(32, |_, y| 0.05 * (y * 25.0).sin());
        let wrinkled = Sheet {
            positions: &wp,
            normals: &wn,
            ..flat
        };
        let map = bake_normals(&wrinkled, &low, &tg, 32, 2);
        let red = map.chunks(4).map(|px| px[0].abs_diff(128)).max().unwrap();
        let green = map.chunks(4).map(|px| px[1].abs_diff(128)).max().unwrap();
        assert!(red <= 2 && green > 40, "{red} {green}");
        // Where the height grows down the image (y), the surface faces up
        // the image: green above the middle.
        let at = |x: usize, y: usize| map[(y * 32 + x) * 4 + 1];
        assert!(at(16, 0) > 128 + 20, "{}", at(16, 0));
        assert_eq!(map, bake_normals(&wrinkled, &low, &tg, 32, 2));
    }

    #[test]
    fn colours_follow_the_attribute_and_pad_outwards() {
        let uv = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.5, 0.0),
            Vec2::new(0.0, 0.5),
        ];
        let attr = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::ZERO];
        let img = bake_colour(&uv, &[[0, 2, 1]], &attr, 16, 1, [0, 0, 0, 255], |a| {
            [(a.x * 255.0) as u8, 0, 0, 255]
        });
        let px = |x: usize, y: usize| &img[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4];
        assert!(px(6, 0)[0] > 150, "{:?}", px(6, 0));
        assert!(px(0, 0)[0] < 30);
        assert_eq!(px(15, 15), [0, 0, 0, 255]);
        // One texel beyond the diagonal edge is padded.
        assert!(px(4, 4)[0] > 0 && px(4, 4)[3] == 255);
    }
}
