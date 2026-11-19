//! Prints on a garment's fabric (spec 25): what the print is, where it
//! lies on each piece (texture coordinates along the grainline, at the
//! fabric's real scale) and the built-in prints, drawn by our own code.
//!
//! Images are decoded elsewhere (`r3d-io`); this module only holds the
//! pixels ([`Texture`]) every consumer shares: the 3D view, the photo
//! renderer, the board and the exports.

use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Largest repeat of a print, each way (mm).
pub const MAX_REPEAT_MM: f64 = 5000.0;
/// Smallest repeat of a print, each way (mm).
pub const MIN_REPEAT_MM: f64 = 1.0;
/// Repeat of the texture coordinates of a garment without a print (mm):
/// one unit per metre of fabric.
pub const PLAIN_REPEAT_MM: f64 = 1000.0;

/// A garment's print: what it is, how big one repeat is on the fabric and
/// how it is turned from the grainline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Print {
    /// A built-in print or an image in the project folder.
    pub source: PrintSource,
    /// Width of one repeat across the grain (mm).
    pub width_mm: f64,
    /// Height of one repeat along the grain (mm); none: the image's
    /// proportions (square for built-ins).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f64>,
    /// The print turned from the grainline (degrees, counter-clockwise);
    /// 0: the image's top along the grain.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub angle_deg: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// Where a print's pixels come from.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrintSource {
    /// One of the prints the app ships, in the garment's colour.
    Builtin(BuiltinPrint),
    /// A PNG or JPEG file, relative to the project folder.
    Image(String),
}

/// The prints the app ships, drawn by [`builtin`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BuiltinPrint {
    /// A stripe of the colour and one of the ground, along the grain.
    Stripes,
    /// Crossing bands of the colour, darker where they cross.
    Gingham,
    /// Dots of the colour on the ground, in staggered rows (two per
    /// repeat).
    Dots,
    /// A 2/1 twill weave: diagonal ribs of the colour and a lighter
    /// thread.
    Twill,
}

impl BuiltinPrint {
    pub const ALL: [BuiltinPrint; 4] = [
        BuiltinPrint::Stripes,
        BuiltinPrint::Gingham,
        BuiltinPrint::Dots,
        BuiltinPrint::Twill,
    ];

    /// The repeat it is usually woven or printed at (mm).
    pub fn default_repeat_mm(self) -> f64 {
        match self {
            BuiltinPrint::Stripes => 20.0,
            BuiltinPrint::Gingham => 12.0,
            BuiltinPrint::Dots => 16.0,
            BuiltinPrint::Twill => 4.0,
        }
    }

    /// Its stable name (as in the JSON).
    pub fn name(self) -> &'static str {
        match self {
            BuiltinPrint::Stripes => "stripes",
            BuiltinPrint::Gingham => "gingham",
            BuiltinPrint::Dots => "dots",
            BuiltinPrint::Twill => "twill",
        }
    }
}

impl Print {
    /// One repeat's width and height (mm); `aspect` is the image's
    /// height over its width, used when no height is given.
    pub fn repeat_mm(&self, aspect: f64) -> DVec2 {
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            1.0
        };
        DVec2::new(
            self.width_mm,
            self.height_mm.unwrap_or(self.width_mm * aspect),
        )
    }
}

/// How a piece shifts and turns its garment's print on itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrintPlacement {
    /// The print moved on the piece (mm, board axes).
    #[serde(default)]
    pub offset_mm: DVec2,
    /// The print turned on the piece (degrees, counter-clockwise).
    #[serde(default)]
    pub angle_deg: f64,
}

impl PrintPlacement {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// The map from a piece's pattern (mm, board axes) to texture coordinates
/// (in repeats, `v` down the image): the fabric's frame turned to the
/// piece's grainline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvFrame {
    pub origin: DVec2,
    /// `u` per mm: the print's right, divided by the repeat's width.
    pub u: DVec2,
    /// `v` per mm: the print's down, divided by the repeat's height.
    pub v: DVec2,
}

impl UvFrame {
    /// The frame turned `angle_deg` from the board's +X (the print's up
    /// along it), shifted by `origin`, for a repeat of `repeat` mm.
    pub fn new(angle_deg: f64, origin: DVec2, repeat: DVec2) -> Self {
        let up = DVec2::from_angle(angle_deg.to_radians());
        let right = DVec2::new(up.y, -up.x);
        let (w, h) = (repeat.x.max(1e-9), repeat.y.max(1e-9));
        Self {
            origin,
            u: right / w,
            v: -up / h,
        }
    }

    /// Texture coordinates of the pattern point `p` (mm).
    pub fn uv(&self, p: DVec2) -> DVec2 {
        let q = p - self.origin;
        DVec2::new(q.dot(self.u), q.dot(self.v))
    }
}

/// An image in sRGB, 8 bits per channel, rows from the top.
#[derive(Clone, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    /// `width × height × 4` bytes (alpha is kept but not used).
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl Texture {
    /// Height over width.
    pub fn aspect(&self) -> f64 {
        f64::from(self.height) / f64::from(self.width.max(1))
    }

    /// The pixel at (`x`, `y`), wrapped around the edges.
    pub fn pixel(&self, x: i64, y: i64) -> [u8; 4] {
        let (w, h) = (i64::from(self.width.max(1)), i64::from(self.height.max(1)));
        let i = (y.rem_euclid(h) * w + x.rem_euclid(w)) as usize * 4;
        match self.rgba.get(i..i + 4) {
            Some(p) => [p[0], p[1], p[2], p[3]],
            None => [0, 0, 0, 255],
        }
    }

    /// The average colour (sRGB `0xRRGGBB`), averaged in linear light.
    pub fn mean_rgb(&self) -> u32 {
        let mut sum = [0.0f64; 3];
        let mut n = 0.0;
        for p in self.rgba.as_chunks::<4>().0 {
            for c in 0..3 {
                sum[c] += f64::from(to_linear(p[c]));
            }
            n += 1.0;
        }
        if n == 0.0 {
            return 0x808080;
        }
        let [r, g, b] = sum.map(|s| u32::from(to_srgb((s / n) as f32)));
        r << 16 | g << 8 | b
    }

    /// Half the size (each side rounded down, at least 1), each pixel the
    /// average of the 2 × 2 it covers, in linear light: the next level of
    /// a mip-map pyramid (Williams 1983).
    pub fn halved(&self) -> Texture {
        let (w, h) = ((self.width / 2).max(1), (self.height / 2).max(1));
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (2 * x + dx).min(self.width.saturating_sub(1));
                    let sy = (2 * y + dy).min(self.height.saturating_sub(1));
                    let p = self.pixel(i64::from(sx), i64::from(sy));
                    for c in 0..3 {
                        acc[c] += to_linear(p[c]);
                    }
                    acc[3] += f32::from(p[3]);
                }
                rgba.extend([
                    to_srgb(acc[0] / 4.0),
                    to_srgb(acc[1] / 4.0),
                    to_srgb(acc[2] / 4.0),
                    (acc[3] / 4.0).round() as u8,
                ]);
            }
        }
        Texture {
            width: w,
            height: h,
            rgba,
        }
    }

    /// Scaled down by halving until both sides are at most `max` pixels.
    pub fn fitted(self, max: u32) -> Texture {
        let mut t = self;
        while t.width > max || t.height > max {
            t = t.halved();
        }
        t
    }
}

/// The off-white ground of the built-in prints (sRGB).
pub const GROUND_RGB: u32 = 0xF3F0E8;

/// Side of a built-in print's tile (pixels): one repeat.
pub const BUILTIN_SIZE: u32 = 240;

fn to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

fn linear_rgb(rgb: u32) -> [f32; 3] {
    [16, 8, 0].map(|s| to_linear(((rgb >> s) & 0xFF) as u8))
}

/// `a` mixed toward `b` by `t` (linear light).
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// One repeat of a built-in print in `rgb` (the garment's colour, sRGB
/// `0xRRGGBB`) on [`GROUND_RGB`]: [`BUILTIN_SIZE`] pixels square, `u`
/// across the columns, the grain up the rows.
pub fn builtin(print: BuiltinPrint, rgb: u32) -> Texture {
    let n = BUILTIN_SIZE;
    let colour = linear_rgb(rgb);
    let ground = linear_rgb(GROUND_RGB);
    // Samples per pixel side, for smooth dot edges.
    const SS: u32 = 4;
    let shade = |u: f32, v: f32| -> [f32; 3] {
        match print {
            BuiltinPrint::Stripes => {
                if u < 0.5 {
                    colour
                } else {
                    ground
                }
            }
            BuiltinPrint::Gingham => {
                let bands = f32::from(u8::from(u < 0.5)) + f32::from(u8::from(v < 0.5));
                // Where the bands cross, the colour is woven both ways.
                match bands as u32 {
                    0 => ground,
                    1 => mix(ground, colour, 0.5),
                    _ => colour,
                }
            }
            BuiltinPrint::Dots => {
                let r = 0.14;
                let near = [(0.25, 0.25), (0.75, 0.75)]
                    .iter()
                    .flat_map(|(cx, cy)| {
                        [-1.0f32, 0.0, 1.0].into_iter().flat_map(move |ox| {
                            [-1.0f32, 0.0, 1.0].map(move |oy| {
                                let (dx, dy) = (u - cx - ox, v - cy - oy);
                                dx * dx + dy * dy
                            })
                        })
                    })
                    .fold(f32::MAX, f32::min);
                if near < r * r { colour } else { ground }
            }
            BuiltinPrint::Twill => {
                // Six threads each way: two repeats of the 2/1 weave.
                let threads = 6.0;
                let (x, y) = (u * threads, v * threads);
                let (i, j) = (x.floor() as i32, y.floor() as i32);
                let (fx, fy) = (x.fract(), y.fract());
                // The warp (along the grain, columns) floats over two
                // weft threads and under one, stepping one per row.
                let warp_up = (i - j).rem_euclid(3) != 0;
                let light = mix(colour, ground, 0.55);
                let (thread, across) = if warp_up { (colour, fx) } else { (light, fy) };
                let round = 0.72 + 0.28 * (std::f32::consts::PI * across).sin();
                thread.map(|c| c * round)
            }
        }
    };
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let mut acc = [0.0f32; 3];
            for sy in 0..SS {
                for sx in 0..SS {
                    let u = (x as f32 + (sx as f32 + 0.5) / SS as f32) / n as f32;
                    let v = (y as f32 + (sy as f32 + 0.5) / SS as f32) / n as f32;
                    let c = shade(u, v);
                    for k in 0..3 {
                        acc[k] += c[k];
                    }
                }
            }
            let k = (SS * SS) as f32;
            rgba.extend([
                to_srgb(acc[0] / k),
                to_srgb(acc[1] / k),
                to_srgb(acc[2] / k),
                255,
            ]);
        }
    }
    Texture {
        width: n,
        height: n,
        rgba,
    }
}

/// Is `path` a print image's path the project can hold: relative, inside
/// the folder (no `..`, no root), ending in `.png`, `.jpg` or `.jpeg`?
pub fn image_path_ok(path: &str) -> bool {
    let p = std::path::Path::new(path);
    // Forward slashes only, so the project reads alike on every system.
    let inside = !path.is_empty()
        && !path.contains(['\\', ':'])
        && !path.starts_with('/')
        && p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    inside && matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_follows_the_grain() {
        // Grain straight up the board: u across (x), v down (−y).
        let f = UvFrame::new(90.0, DVec2::ZERO, DVec2::new(20.0, 40.0));
        let uv = f.uv(DVec2::new(10.0, 40.0));
        assert!((uv.x - 0.5).abs() < 1e-9 && (uv.y + 1.0).abs() < 1e-9);
        // Grain along +X: one repeat along it is one unit of v.
        let f = UvFrame::new(0.0, DVec2::new(5.0, 0.0), DVec2::splat(10.0));
        let uv = f.uv(DVec2::new(15.0, 0.0));
        assert!(uv.x.abs() < 1e-9 && (uv.y + 1.0).abs() < 1e-9);
    }

    #[test]
    fn builtins_are_one_repeat_in_the_colour() {
        for p in BuiltinPrint::ALL {
            let t = builtin(p, 0x1F4E9A);
            assert_eq!((t.width, t.height), (BUILTIN_SIZE, BUILTIN_SIZE));
            assert_eq!(t.rgba.len(), (BUILTIN_SIZE * BUILTIN_SIZE * 4) as usize);
        }
        let s = builtin(BuiltinPrint::Stripes, 0x1F4E9A);
        assert_eq!(s.pixel(10, 3), [0x1F, 0x4E, 0x9A, 255]);
        assert_eq!(s.pixel(200, 3), [0xF3, 0xF0, 0xE8, 255]);
        // Stripes run along the grain: every row is the same.
        assert_eq!(s.pixel(10, 3), s.pixel(10, 230));
    }

    #[test]
    fn halving_averages_in_linear_light() {
        let t = Texture {
            width: 2,
            height: 2,
            rgba: vec![
                0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
            ],
        };
        let h = t.halved();
        assert_eq!((h.width, h.height), (1, 1));
        assert_eq!(h.rgba[0], 188);
        let big = builtin(BuiltinPrint::Dots, 0).fitted(64);
        assert!(big.width <= 64 && big.height <= 64);
    }

    #[test]
    fn image_paths_stay_inside() {
        assert!(image_path_ok("textures/liberty.JPG"));
        assert!(image_path_ok("a.png"));
        for bad in [
            "",
            "/etc/a.png",
            "../a.png",
            "textures/../../a.png",
            "a.gif",
            "C:\\a.png",
        ] {
            assert!(!image_path_ok(bad), "{bad}");
        }
    }
}
