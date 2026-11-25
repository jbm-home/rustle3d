//! Fitting the body to measurements.
//!
//! Stature is matched exactly by scaling; its proportions come from the
//! stature targets. Bust, waist and hips are matched by a regularised
//! Gauss–Newton search over the body weight and the three girth targets,
//! with a numerical Jacobian. The other measurements the user typed that
//! the model can follow ([`FOLLOWED`]) are then matched one at a time by
//! a secant search on their targets, and the girths fitted again; twice
//! (our own method).

use glam::Vec3;
use r3d_core::{Body, Measure, Measures};

use crate::measure::{Measurements, measure};
use crate::model::{BodyModel, BodyShape, FOLLOWED, MAX_EXTRAPOLATION, Shaped};

/// A fitted body.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    pub shape: BodyShape,
    /// Uniform scale applied after shaping (exact stature).
    pub scale: f32,
    /// What the fitted body actually measures.
    pub achieved: Measurements,
}

const ITERATIONS: usize = 20;
const STEP: f32 = 0.05;
/// Pull towards the neutral body when several shapes fit equally well
/// (residuals are relative). At 0.002 it outweighed the residuals of
/// large sizes and left every body from about 44 up slimmer than asked
/// (a women's 50 with a waist of 94.6 cm for 98).
const REGULARISATION: f32 = 0.00001;

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
            waist_height_cm: m.waist_height_cm * k,
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
    fit_with(model, spec, [0.0; FOLLOWED.len()])
}

/// Rounds of following the typed measurements then fitting the girths
/// again (lengthening a leg shortens the rest once scaled to the height).
const ROUNDS: usize = 2;
const SECANT_STEPS: usize = 6;
/// A followed measurement this close (cm) is reached.
const FOLLOW_TOLERANCE_CM: f32 = 0.2;
/// The secant search's first step on a target's weight.
const FIRST_STEP: f32 = 0.3;

/// Find the shape that best matches `spec` and the measurements typed
/// besides (cm) that the body model can follow; the others it keeps as
/// it has them.
pub fn fit_typed(model: &BodyModel, spec: &Body, typed: &Measures) -> Fit {
    let goals: Vec<(usize, Measure, f32)> = FOLLOWED
        .iter()
        .enumerate()
        .filter_map(|(i, (m, _))| Some((i, *m, *typed.get(m)? as f32)))
        .collect();
    let mut followed = [0.0; FOLLOWED.len()];
    let mut f = fit_with(model, spec, followed);
    if goals.is_empty() {
        return f;
    }
    for _ in 0..ROUNDS {
        for &(i, m, want) in &goals {
            if let Some(slot) = followed.get_mut(i) {
                *slot = follow(model, &f, i, m, want);
            }
        }
        f = fit_with(model, spec, followed);
    }
    f
}

/// The weight of followed target `i` that gives `m` the value `want`
/// (cm), the rest of `f` as it is; the best found if none does.
fn follow(model: &BodyModel, f: &Fit, i: usize, m: Measure, want: f32) -> f32 {
    let height = f.achieved.height_cm / 100.0;
    let at = |u: f32| {
        let mut shape = f.shape;
        if let Some(slot) = shape.followed.get_mut(i) {
            *slot = u;
        }
        let mut shaped = model.shaped(&shape);
        let top = shaped.positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let k = if top > 0.0 { height / top } else { 1.0 };
        for p in shaped.positions.iter_mut().chain(&mut shaped.joints) {
            *p *= k;
        }
        crate::tape::take(model, &shaped, m)
    };
    let mut u0 = f.shape.followed.get(i).copied().unwrap_or_default();
    let Some(mut v0) = at(u0) else {
        return u0;
    };
    let mut best = (u0, (v0 - want).abs());
    let step = if v0 < want { FIRST_STEP } else { -FIRST_STEP };
    let mut u1 = (u0 + step).clamp(-1.0, MAX_EXTRAPOLATION);
    for _ in 0..SECANT_STEPS {
        if best.1 < FOLLOW_TOLERANCE_CM || u1 == u0 {
            break;
        }
        let Some(v1) = at(u1) else {
            break;
        };
        if (v1 - want).abs() < best.1 {
            best = (u1, (v1 - want).abs());
        }
        if (v1 - v0).abs() < 1e-4 {
            break;
        }
        let u2 = (u1 - (v1 - want) * (u1 - u0) / (v1 - v0)).clamp(-1.0, MAX_EXTRAPOLATION);
        (u0, v0, u1) = (u1, v1, u2);
    }
    best.0
}

/// [`fit`] with the followed targets at `followed`.
fn fit_with(model: &BodyModel, spec: &Body, followed: [f32; FOLLOWED.len()]) -> Fit {
    let height_cm = spec.height_cm as f32;
    // Stature proportions: interpolate between the short and tall targets.
    let neutral = BodyShape {
        masculinity: (spec.masculinity as f32).clamp(0.0, 1.0),
        age_years: spec.age_years.map(|a| a as f32),
        followed,
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

    // The objective (squared residuals plus the pull to neutral) at `u`,
    // and its residuals.
    let cost = |u: [f32; 4]| {
        let (m, _) = scaled(model, &with_params(base, u), height_cm);
        let r = residuals(&m, spec);
        let c = r.iter().map(|e| e * e).sum::<f32>()
            + REGULARISATION * u.iter().map(|v| v * v).sum::<f32>();
        (if c.is_finite() { c } else { f32::MAX }, r)
    };
    let mut u = [0.0f32; 4];
    let (mut c, mut r) = cost(u);
    for _ in 0..ITERATIONS {
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
        // Take a step only if it lowers the cost, halving it until it
        // does (backtracking). Taken whole, a step past the targets'
        // range (clamped there) threw the fit far off: a women's size 52
        // (waist 104 cm) came out with a waist of 69 cm. Far past the
        // targets' authored range the Gauss–Newton step can point the
        // wrong way: then a steepest-descent step (the Cauchy point of
        // the same model) is tried, as in a dogleg.
        let gauss_newton = solve4(a, b);
        let jb: [f32; 3] = std::array::from_fn(|i| (0..4).map(|j| jac[i][j] * b[j]).sum());
        let bb: f32 = b.iter().map(|v| v * v).sum();
        let curvature: f32 = jb.iter().map(|v| v * v).sum::<f32>() + REGULARISATION * bb;
        let descent = (curvature > 1e-12).then(|| b.map(|v| v * bb / curvature));
        let mut improved = false;
        for delta in [gauss_newton, descent].into_iter().flatten() {
            let mut scale = 1.0;
            for _ in 0..8 {
                let v: [f32; 4] = std::array::from_fn(|j| {
                    (u[j] + scale * delta[j]).clamp(-1.0, MAX_EXTRAPOLATION)
                });
                let (cv, rv) = cost(v);
                if cv < c {
                    (u, c, r) = (v, cv, rv);
                    improved = true;
                    break;
                }
                scale *= 0.5;
            }
            if improved {
                break;
            }
        }
        if !improved {
            break;
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
    fitted_body(model, fit).positions
}

/// Vertices and joints of a fitted body (metres, feet on the ground).
pub fn fitted_body(model: &BodyModel, fit: &Fit) -> Shaped {
    let mut shaped = model.shaped(&fit.shape);
    for p in shaped.positions.iter_mut().chain(&mut shaped.joints) {
        *p *= fit.scale;
    }
    shaped
}

/// The upper body of a fitted body as a shirt pattern reads it
/// (centimetres; heights above the floor, widths from the median plane).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpperBody {
    /// The shoulder joint's distance from the median plane.
    pub shoulder_joint_cm: f32,
    /// The top of the shoulder over the joint.
    pub shoulder_top_cm: f32,
    /// Where the arm parts from the torso (in the A pose).
    pub armpit_cm: f32,
    /// The base of the neck, and the neck's girth there.
    pub neck_base_cm: f32,
    pub neck_cm: f32,
    /// The bust of the same body at neutral weight and girths: the
    /// shoulders of the body model do not widen with its weight.
    pub neutral_bust_cm: f32,
}

/// The top of the shoulder over its joint (m): the highest vertex above
/// the joint, below the neck's base `neck_y` (a small child's head is
/// wide enough for its ear to stand over the shoulder joint).
pub(crate) fn shoulder_top(positions: &[Vec3], joint: Vec3, neck_y: f32) -> f32 {
    positions
        .iter()
        .filter(|v| (v.x - joint.x).abs() < 0.015 && (v.z - joint.z).abs() < 0.04 && v.y < neck_y)
        .map(|v| v.y)
        .fold(joint.y, f32::max)
}

/// A girth this much smaller than the one just above marks where the arm
/// parted from the torso (the section loop around the body stops taking
/// the arm in).
const ARMPIT_DROP: f32 = 0.15;

/// Measure the upper body of `fit` on its skeleton and skin.
pub fn upper_body(model: &BodyModel, fit: &Fit) -> Option<UpperBody> {
    let shaped = fitted_body(model, fit);
    let skeleton = crate::skeleton::Skeleton::new(model, &shaped);
    let head = |name: &str| {
        skeleton
            .bones
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.head)
    };
    let (joint, neck) = (head("upper_arm_l")?, head("neck")?);
    let (p, t) = (&shaped.positions, model.triangles());
    let shoulder_top = shoulder_top(p, joint, neck.y);
    // Down from the joint every 5 mm until the girth drops.
    let girth = |y: f32| crate::measure::girth_at(p, t, y, 0.0);
    let mut armpit = None;
    let mut above = girth(joint.y)?;
    for k in 1..=60 {
        let y = joint.y - 0.005 * k as f32;
        let g = girth(y)?;
        if g < (1.0 - ARMPIT_DROP) * above {
            armpit = Some(y);
            break;
        }
        above = g;
    }
    let neutral = BodyShape {
        weight: 0.0,
        girths: [0.0; 3],
        ..fit.shape
    };
    let (m, _) = scaled(model, &neutral, fit.achieved.height_cm);
    Some(UpperBody {
        shoulder_joint_cm: joint.x * 100.0,
        shoulder_top_cm: shoulder_top * 100.0,
        armpit_cm: armpit? * 100.0,
        neck_base_cm: neck.y * 100.0,
        neck_cm: girth(neck.y)?,
        neutral_bust_cm: m.bust_cm,
    })
}

/// Heights of a body's neutral shape as fractions of its stature: where
/// garments are placed from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Proportions {
    /// The base of the neck (the neck's joint).
    pub neck: f32,
    /// The shoulder joint.
    pub shoulder: f32,
    /// The crotch.
    pub crotch: f32,
}

/// The [`Proportions`] of the model's neutral body of `masculinity` at
/// `age_years` (`None`: an adult). A child's head is larger: its neck,
/// shoulders and crotch sit lower on its stature.
pub fn proportions(
    model: &BodyModel,
    masculinity: f32,
    age_years: Option<f32>,
) -> Option<Proportions> {
    let shaped = model.shaped(&BodyShape {
        masculinity,
        age_years,
        ..BodyShape::default()
    });
    let height = shaped.positions.iter().map(|p| p.y).fold(0.0, f32::max);
    if height <= 0.0 {
        return None;
    }
    let skeleton = crate::skeleton::Skeleton::new(model, &shaped);
    let joint = |name: &str| Some(skeleton.bones.get(skeleton.find(name)?)?.head.y / height);
    let crotch = crate::measure::heights(&shaped.positions, model.triangles()).crotch?;
    Some(Proportions {
        neck: joint("neck")?,
        shoulder: joint("upper_arm_l")?,
        crotch: crotch / height,
    })
}
