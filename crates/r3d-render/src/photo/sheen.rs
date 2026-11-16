//! Directional albedo of the sheen lobe, tabulated for the shader.
//!
//! The sheen is the "Charlie" microfacet distribution (Estevez and Kulla
//! 2017, "Production Friendly Microfacet Sheen BRDF") with the visibility
//! term of Neubelt and Pettineo (2013, "Crafting a Next-Gen Material
//! Pipeline for The Order: 1886"), as in Khronos `KHR_materials_sheen`.
//! The layers under the sheen are dimmed by `1 - sheen · E(cos θ, α)`, the
//! light the sheen already reflected, so a sheen never adds energy.

/// Table size along each axis (cosine of the view angle, roughness).
pub(crate) const SIZE: usize = 32;
/// Smallest roughness used, in the table and the shader.
pub(crate) const MIN_ALPHA: f32 = 0.05;

fn charlie(alpha: f32, nh: f32) -> f32 {
    let inv = 1.0 / alpha;
    let sin2 = (1.0 - nh * nh).max(0.0);
    (2.0 + inv) * sin2.powf(inv * 0.5) / std::f32::consts::TAU
}

fn visibility(nl: f32, nv: f32) -> f32 {
    1.0 / (4.0 * (nl + nv - nl * nv)).max(1e-6)
}

/// `E(μ, α)` for μ = i / (SIZE - 1) (row-major by α then μ), with
/// α = max(roughness², MIN_ALPHA) for roughness = j / (SIZE - 1).
pub(crate) fn albedo_table() -> Vec<f32> {
    const STEPS: usize = 48;
    let mut table = Vec::with_capacity(SIZE * SIZE);
    for j in 0..SIZE {
        let r = j as f32 / (SIZE - 1) as f32;
        let alpha = (r * r).max(MIN_ALPHA);
        for i in 0..SIZE {
            let nv = (i as f32 / (SIZE - 1) as f32).max(1e-3);
            let wo = glam::Vec3::new((1.0 - nv * nv).sqrt(), 0.0, nv);
            let mut sum = 0.0;
            for a in 0..STEPS {
                let theta = (a as f32 + 0.5) / STEPS as f32 * std::f32::consts::FRAC_PI_2;
                let (st, ct) = theta.sin_cos();
                for b in 0..STEPS {
                    // Half the circle; the lobe is symmetric about the
                    // plane holding the view direction.
                    let phi = (b as f32 + 0.5) / STEPS as f32 * std::f32::consts::PI;
                    let (sp, cp) = phi.sin_cos();
                    let wi = glam::Vec3::new(st * cp, st * sp, ct);
                    let h = (wi + wo).normalize_or(glam::Vec3::Z);
                    let f = charlie(alpha, h.z) * visibility(ct, nv);
                    // dω = sin θ dθ dφ, and twice the half circle.
                    sum += 2.0 * f * ct * st;
                }
            }
            let d_area = (std::f32::consts::FRAC_PI_2 / STEPS as f32)
                * (std::f32::consts::PI / STEPS as f32);
            table.push((sum * d_area).min(1.0));
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheen_reflects_less_than_it_receives() {
        let t = albedo_table();
        assert_eq!(t.len(), SIZE * SIZE);
        assert!(t.iter().all(|e| (0.0..=1.0).contains(e)));
        // Rougher sheens spread more light; grazing views see more sheen.
        let at = |mu: usize, r: usize| t[r * SIZE + mu];
        assert!(at(SIZE - 1, SIZE - 1) > at(SIZE - 1, 4));
        assert!(at(2, SIZE / 2) > at(SIZE - 1, SIZE / 2));
    }
}
