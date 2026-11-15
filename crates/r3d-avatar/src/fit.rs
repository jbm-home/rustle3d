//! Fitting the body to measurements.
//!
//! Stature is matched exactly by scaling; its proportions come from the
//! stature targets. Bust, waist and hips are matched by a regularised
//! Gauss–Newton search over the body weight and the three girth targets,
//! with a numerical Jacobian (our own method).

use glam::Vec3;
use r3d_core::Body;

use crate::measure::{Measurements, measure};
use crate::model::{BodyModel, BodyShape, MAX_EXTRAPOLATION};

/// A fitted body.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    pub shape: BodyShape,
    /// Uniform scale applied after shaping (exact stature).
    pub scale: f32,
    /// What the fitted body actually measures.
    pub achieved: Measurements,
}

const ITERATIONS: usize = 12;
const STEP: f32 = 0.05;
/// Pull towards the neutral body when several shapes fit equally well.
const REGULARISATION: f32 = 0.002;

fn with_params(base: BodyShape, u: [f32; 4]) -> BodyShape {
    BodyShape {
        weight: u[0],
        girths: [u[1], u[2], u[3]],
        ..base
    }
}

/// Measurements of a shape after scaling it to `height_cm`; also returns
/// the scale.
fn scaled(model: &BodyModel, shape: &BodyShape, height_cm: f32) -> (Measurements, f32) {
    let m = measure(
        &model.positions(shape),
        model.triangles(),
        model.landmarks(),
    );
    let k = if m.height_cm > 0.0 {
        height_cm / m.height_cm
    } else {
        1.0
    };
    (
        Measurements {
            height_cm: m.height_cm * k,
            bust_cm: m.bust_cm * k,
            waist_cm: m.waist_cm * k,
            hips_cm: m.hips_cm * k,
        },
        k,
    )
}

fn residuals(m: &Measurements, spec: &Body) -> [f32; 3] {
    let rel = |got: f32, want: f64| (got - want as f32) / (want as f32).max(1.0);
    [
        rel(m.bust_cm, spec.bust_cm),
        rel(m.waist_cm, spec.waist_cm),
        rel(m.hips_cm, spec.hips_cm),
    ]
}

/// Solve the 4×4 system `a x = b` (Gaussian elimination, partial pivoting).
fn solve4(mut a: [[f32; 4]; 4], mut b: [f32; 4]) -> Option<[f32; 4]> {
    for col in 0..4 {
        let pivot = (col..4).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..4 {
            let f = a[row][col] / a[col][col];
            for k in col..4 {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = [0.0; 4];
    for row in (0..4).rev() {
        let s: f32 = (row + 1..4).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    Some(x)
}

/// Find the shape that best matches `spec`.
pub fn fit(model: &BodyModel, spec: &Body) -> Fit {
    let height_cm = spec.height_cm as f32;
    // Stature proportions: interpolate between the short and tall targets.
    let neutral = BodyShape {
        masculinity: (spec.masculinity as f32).clamp(0.0, 1.0),
        ..BodyShape::default()
    };
    let height_of = |stature: f32| {
        let p = model.positions(&BodyShape { stature, ..neutral });
        p.iter().map(|v| v.y).fold(0.0, f32::max) * 100.0
    };
    let (short, mid, tall) = (height_of(-1.0), height_of(0.0), height_of(1.0));
    let stature = if height_cm >= mid {
        ((height_cm - mid) / (tall - mid).max(1e-3)).clamp(0.0, 1.0)
    } else {
        -((mid - height_cm) / (mid - short).max(1e-3)).clamp(0.0, 1.0)
    };
    let base = BodyShape { stature, ..neutral };

    let mut u = [0.0f32; 4];
    for _ in 0..ITERATIONS {
        let (m, _) = scaled(model, &with_params(base, u), height_cm);
        let r = residuals(&m, spec);
        if r.iter().all(|e| e.abs() < 0.002) {
            break;
        }
        // Numerical Jacobian, one column per parameter.
        let mut jac = [[0.0f32; 4]; 3];
        for j in 0..4 {
            let mut v = u;
            v[j] = (v[j] + STEP).clamp(-1.0, MAX_EXTRAPOLATION);
            let h = v[j] - u[j];
            if h.abs() < 1e-6 {
                continue;
            }
            let (mj, _) = scaled(model, &with_params(base, v), height_cm);
            let rj = residuals(&mj, spec);
            for i in 0..3 {
                jac[i][j] = (rj[i] - r[i]) / h;
            }
        }
        // (JᵀJ + λI) δ = −Jᵀr − λu
        let mut a = [[0.0f32; 4]; 4];
        let mut b = [0.0f32; 4];
        for j in 0..4 {
            for k in 0..4 {
                a[j][k] = (0..3).map(|i| jac[i][j] * jac[i][k]).sum();
            }
            a[j][j] += REGULARISATION;
            b[j] = -(0..3).map(|i| jac[i][j] * r[i]).sum::<f32>() - REGULARISATION * u[j];
        }
        let Some(delta) = solve4(a, b) else { break };
        for j in 0..4 {
            u[j] = (u[j] + delta[j]).clamp(-1.0, MAX_EXTRAPOLATION);
        }
    }
    let shape = with_params(base, u);
    let (achieved, scale) = scaled(model, &shape, height_cm);
    Fit {
        shape,
        scale,
        achieved,
    }
}

/// Positions of a fitted body (metres, feet on the ground).
pub fn fitted_positions(model: &BodyModel, fit: &Fit) -> Vec<Vec3> {
    model
        .positions(&fit.shape)
        .into_iter()
        .map(|p| p * fit.scale)
        .collect()
}
