//! Environment lighting (spec 35): a latitude-longitude picture of the
//! light all around lights the photo instead of the studio.
//!
//! - Directions map to the picture as `u = ½ + atan2(x, −z) / 2π`,
//!   `v = acos(y) / π` (the picture's centre looks along −z, its top is
//!   straight up), in the environment's own frame, turned about the
//!   vertical to world space.
//! - Importance sampling (Pharr, Jakob and Humphreys, *Physically Based
//!   Rendering*, 3rd ed., §13.6.7 and §14.2.4): a piecewise-constant 2D
//!   distribution over a grid of at most 1024 × 512 cells, proportional
//!   to each cell's mean luminance times sin θ; a marginal CDF over the
//!   rows, a conditional CDF in each row. The density of a direction is
//!   `p(u, v) / (2π² sin θ)`.
//! - The picture's lower half is its ground: camera rays that reach the
//!   floor see it as from a point [`CAMERA_HEIGHT`] above the subject
//!   (a projection on the ground plane), so the subject stands on it.
//! - The GPU reads the texels as Ward's RGBE (one `u32` each) and the
//!   tables as `f32` bits, from the buffer shared with the sheen table
//!   and the prints.

use std::f32::consts::PI;

/// Widest picture sent to the GPU; wider ones are halved.
pub const MAX_WIDTH: u32 = 4096;
/// Mean luminance over the sphere every environment is scaled to.
const MEAN_LUMINANCE: f32 = 0.8;
/// Height above the floor the picture is taken from (m): its ground is
/// projected on the floor as seen from there, above the subject.
pub(crate) const CAMERA_HEIGHT: f32 = 1.6;
/// Largest importance-sampling grid.
const GRID_WIDTH: u32 = 1024;
const GRID_HEIGHT: u32 = 512;

fn luminance(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The light around a photo: linear RGB texels, top row first. How it
/// is turned and how bright it is are the photo's settings.
#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    width: u32,
    height: u32,
    rgb: Vec<[f32; 3]>,
    /// The factor that brings the mean luminance over the sphere to
    /// [`MEAN_LUMINANCE`].
    exposure: f32,
}

impl Environment {
    /// A `width × height` picture whose texel `i` (row by row, top row
    /// first) is `texel(i)`. Pictures wider than [`MAX_WIDTH`] are
    /// halved, averaging 2 × 2 texels, until they fit. `None` when the
    /// picture is empty.
    pub fn new(width: u32, height: u32, texel: impl Fn(usize) -> [f32; 3]) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let mut k = 1u32;
        while width.div_ceil(k) > MAX_WIDTH {
            k *= 2;
        }
        let (w, h) = ((width / k).max(1), (height / k).max(1));
        let mut rgb = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0.0f32; 3];
                let mut n = 0.0f32;
                for sy in y * k..((y + 1) * k).min(height) {
                    for sx in x * k..((x + 1) * k).min(width) {
                        let t = texel((sy * width + sx) as usize);
                        let finite = t.map(|c| if c.is_finite() { c.max(0.0) } else { 0.0 });
                        (0..3).for_each(|c| sum[c] += finite[c]);
                        n += 1.0;
                    }
                }
                rgb.push(sum.map(|c| c / n.max(1.0)));
            }
        }
        let mut env = Self {
            width: w,
            height: h,
            rgb,
            exposure: 1.0,
        };
        env.exposure = env.mean_exposure();
        Some(env)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Polar angle of row `y`'s centre.
    fn theta(&self, y: u32) -> f32 {
        PI * (y as f32 + 0.5) / self.height as f32
    }

    /// The factor on the texels that brings the mean luminance over the
    /// sphere to [`MEAN_LUMINANCE`] (before the photo's brightness).
    pub(crate) fn exposure(&self) -> f32 {
        self.exposure
    }

    fn mean_exposure(&self) -> f32 {
        let (mut sum, mut weight) = (0.0f64, 0.0f64);
        for (y, row) in self.rgb.chunks(self.width as usize).enumerate() {
            let s = f64::from(self.theta(y as u32).sin());
            sum += s * row.iter().map(|c| f64::from(luminance(*c))).sum::<f64>();
            weight += s * row.len() as f64;
        }
        let mean = if weight > 0.0 { sum / weight } else { 0.0 };
        if mean > 1e-9 {
            MEAN_LUMINANCE / mean as f32
        } else {
            1.0
        }
    }

    /// Luminance falling on an upward floor from the whole environment,
    /// unscaled: ∫ L(ω) max(cos θ, 0) dω.
    pub(crate) fn floor_irradiance(&self) -> f32 {
        let texel_angle = (2.0 * PI / self.width as f32) * (PI / self.height as f32);
        let mut sum = 0.0f64;
        for (y, row) in self.rgb.chunks(self.width as usize).enumerate() {
            let theta = self.theta(y as u32);
            let cos = theta.cos();
            if cos <= 0.0 {
                continue;
            }
            let row_sum: f64 = row.iter().map(|c| f64::from(luminance(*c))).sum();
            sum += row_sum * f64::from(cos * theta.sin() * texel_angle);
        }
        sum as f32
    }

    /// The texels as RGBE, one `u32` each.
    pub(crate) fn rgbe(&self) -> Vec<u32> {
        self.rgb.iter().map(|c| rgbe(*c)).collect()
    }

    /// The sampling grid's size and its tables, as `f32`: the marginal
    /// CDF (rows + 1 values), each row's conditional CDF (columns + 1
    /// values), then each cell's density `p(u, v)`.
    pub(crate) fn sampling(&self) -> (u32, u32, Vec<f32>) {
        let (nu, nv) = (self.width.min(GRID_WIDTH), self.height.min(GRID_HEIGHT));
        let mut f = Vec::with_capacity((nu * nv) as usize);
        for j in 0..nv {
            let (y0, y1) = (
                j * self.height / nv,
                ((j + 1) * self.height / nv).max(j * self.height / nv + 1),
            );
            let sin = (PI * (j as f32 + 0.5) / nv as f32).sin();
            for i in 0..nu {
                let (x0, x1) = (
                    i * self.width / nu,
                    ((i + 1) * self.width / nu).max(i * self.width / nu + 1),
                );
                let mut sum = 0.0f32;
                for y in y0..y1.min(self.height) {
                    for x in x0..x1.min(self.width) {
                        sum += self
                            .rgb
                            .get((y * self.width + x) as usize)
                            .map_or(0.0, |c| luminance(*c));
                    }
                }
                let count = ((y1 - y0) * (x1 - x0)) as f32;
                f.push(sum / count * sin);
            }
        }
        // A black picture is sampled uniformly over the sphere.
        if f.iter().all(|v| *v <= 0.0) {
            for j in 0..nv {
                let sin = (PI * (j as f32 + 0.5) / nv as f32).sin();
                for i in 0..nu {
                    if let Some(v) = f.get_mut((j * nu + i) as usize) {
                        *v = sin;
                    }
                }
            }
        }
        let integral = f.iter().map(|v| f64::from(*v)).sum::<f64>() / f64::from(nu * nv);
        let rows: Vec<f64> = f
            .chunks(nu as usize)
            .map(|r| r.iter().map(|v| f64::from(*v)).sum())
            .collect();
        let total: f64 = rows.iter().sum();
        let mut out = Vec::with_capacity((nv + 1 + nv * (nu + 1) + nu * nv) as usize);
        out.extend(cdf(&rows, total));
        for (row, sum) in f.chunks(nu as usize).zip(&rows) {
            let values: Vec<f64> = row.iter().map(|v| f64::from(*v)).collect();
            out.extend(cdf(&values, *sum));
        }
        out.extend(f.iter().map(|v| (f64::from(*v) / integral) as f32));
        (nu, nv, out)
    }
}

/// A cumulative distribution from 0 to 1 over `values` (uniform when
/// they sum to nothing).
fn cdf(values: &[f64], total: f64) -> Vec<f32> {
    let n = values.len() as f64;
    let mut out = Vec::with_capacity(values.len() + 1);
    out.push(0.0);
    let mut acc = 0.0;
    for (k, v) in values.iter().enumerate() {
        acc += v;
        let c = if total > 0.0 {
            acc / total
        } else {
            (k + 1) as f64 / n
        };
        out.push(c as f32);
    }
    if let Some(last) = out.last_mut() {
        *last = 1.0;
    }
    out
}

/// Ward's shared-exponent encoding of a linear colour (Ward 1991,
/// "Real Pixels"): bytes red, green, blue, exponent + 128, low first.
fn rgbe(c: [f32; 3]) -> u32 {
    let v = c[0].max(c[1]).max(c[2]);
    if v.is_nan() || v <= 1e-32 || !v.is_finite() {
        return 0;
    }
    let mut e = v.log2().floor() as i32 + 1;
    // v / 2^e in [0.5, 1).
    if v / 2f32.powi(e) >= 1.0 {
        e += 1;
    } else if v / 2f32.powi(e) < 0.5 {
        e -= 1;
    }
    if e + 128 <= 0 {
        return 0;
    }
    let e = e.min(127);
    let factor = 256.0 / 2f32.powi(e);
    let [r, g, b] = c.map(|x| (x.max(0.0) * factor).floor().min(255.0) as u32);
    r | g << 8 | b << 16 | ((e + 128) as u32) << 24
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform(w: u32, h: u32, c: [f32; 3]) -> Environment {
        Environment::new(w, h, |_| c).unwrap()
    }

    /// The decoder in the shader (Ward's).
    fn decode(t: u32) -> [f32; 3] {
        let e = t >> 24;
        if e == 0 {
            return [0.0; 3];
        }
        let s = 2f32.powi(e as i32 - 136);
        [0, 8, 16].map(|k| (((t >> k) & 0xFF) as f32 + 0.5) * s)
    }

    #[test]
    fn rgbe_round_trips_within_a_percent() {
        for c in [[1.0, 0.5, 0.25], [3185.8, 2000.0, 10.0], [0.01, 0.02, 0.0]] {
            let back = decode(rgbe(c));
            let max = c.iter().fold(0.0f32, |a, b| a.max(*b));
            for k in 0..3 {
                assert!((back[k] - c[k]).abs() <= max * 0.01, "{c:?} -> {back:?}");
            }
        }
        assert_eq!(rgbe([0.0; 3]), 0);
        assert_eq!(rgbe([f32::NAN, 1.0, 1.0]), rgbe([0.0, 1.0, 1.0]));
    }

    #[test]
    fn wide_pictures_are_halved_by_averaging() {
        let env = Environment::new(MAX_WIDTH * 2, 4, |i| [(i % 2) as f32; 3]).unwrap();
        assert_eq!((env.width(), env.height()), (MAX_WIDTH, 2));
        assert!(env.rgb.iter().all(|c| (c[0] - 0.5).abs() < 1e-6));
        assert!(Environment::new(0, 4, |_| [1.0; 3]).is_none());
    }

    #[test]
    fn exposure_brings_the_mean_to_the_target() {
        let env = uniform(64, 32, [5.0; 3]);
        assert!((env.exposure() * 5.0 - MEAN_LUMINANCE).abs() < 1e-4);
        assert_eq!(uniform(4, 2, [0.0; 3]).exposure(), 1.0);
    }

    #[test]
    fn a_uniform_sky_lights_the_floor_with_pi() {
        let env = uniform(256, 128, [1.0; 3]);
        assert!((env.floor_irradiance() - PI).abs() < 0.01);
    }

    /// Density of a direction in the environment's frame, as the shader
    /// computes it.
    fn pdf(nu: u32, nv: u32, tables: &[f32], d: [f32; 3]) -> f32 {
        let u = 0.5 + d[0].atan2(-d[2]) / (2.0 * PI);
        let v = d[1].clamp(-1.0, 1.0).acos() / PI;
        let sin = (1.0 - d[1] * d[1]).max(0.0).sqrt();
        let i = ((u * nu as f32) as u32).min(nu - 1);
        let j = ((v * nv as f32) as u32).min(nv - 1);
        let at = (nv + 1 + nv * (nu + 1) + j * nu + i) as usize;
        tables[at] / (2.0 * PI * PI * sin)
    }

    #[test]
    fn a_uniform_sky_has_density_one_over_four_pi() {
        let env = uniform(64, 32, [2.0; 3]);
        let (nu, nv, t) = env.sampling();
        for d in [[0.0, 0.0, -1.0], [0.6, 0.8, 0.0], [0.0, -0.6, 0.8]] {
            assert!((pdf(nu, nv, &t, d) - 1.0 / (4.0 * PI)).abs() < 2e-3);
        }
    }

    #[test]
    fn densities_integrate_to_one() {
        // A sky with a bright sun and a dark ground.
        let (w, h) = (128u32, 64u32);
        let env = Environment::new(w, h, |i| {
            let (x, y) = (i as u32 % w, i as u32 / w);
            if (x, y) == (40, 20) {
                [500.0; 3]
            } else if y > h / 2 {
                [0.05; 3]
            } else {
                [1.0, 1.2, 1.5]
            }
        })
        .unwrap();
        let (nu, nv, t) = env.sampling();
        // ∫ p dω over a fine grid of directions.
        let (n_theta, n_phi) = (400, 800);
        let mut sum = 0.0f64;
        for a in 0..n_theta {
            let theta = PI * (a as f32 + 0.5) / n_theta as f32;
            for b in 0..n_phi {
                let phi = 2.0 * PI * (b as f32 + 0.5) / n_phi as f32 - PI;
                let d = [
                    theta.sin() * phi.sin(),
                    theta.cos(),
                    -theta.sin() * phi.cos(),
                ];
                let dw = theta.sin() * (PI / n_theta as f32) * (2.0 * PI / n_phi as f32);
                sum += f64::from(pdf(nu, nv, &t, d) * dw);
            }
        }
        assert!((sum - 1.0).abs() < 0.01, "{sum}");
        // CDFs rise from 0 to 1.
        let marginal = &t[..=nv as usize];
        assert_eq!((marginal[0], marginal[nv as usize]), (0.0, 1.0));
        assert!(marginal.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn a_black_picture_is_sampled_uniformly() {
        let env = uniform(16, 8, [0.0; 3]);
        let (nu, nv, t) = env.sampling();
        assert!((pdf(nu, nv, &t, [0.0, 0.0, -1.0]) - 1.0 / (4.0 * PI)).abs() < 0.01);
        assert!(t.iter().all(|v| v.is_finite()));
    }
}
