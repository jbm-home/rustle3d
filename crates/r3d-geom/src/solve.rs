//! Nonlinear equation systems solved by damped, minimum-norm Gauss–Newton
//! steps: the geometric constraint solver's numerical core.
//!
//! A [`System`] is `m` equations `r_i(x) = 0` over `n` variables, usually
//! with fewer equations than variables (a sketch is rarely fully
//! constrained). Each iteration takes the smallest weighted step that
//! cancels the linearised residual,
//!
//! ```text
//! dx = W⁻¹ Jᵀ (J W⁻¹ Jᵀ + λ I)⁻¹ (−r)
//! ```
//!
//! the minimum-norm Newton step for underdetermined systems (Ben-Israel,
//! "A Newton–Raphson method for the solution of systems of equations",
//! J. Math. Anal. Appl. 15, 1966), with Levenberg–Marquardt damping λ
//! (Levenberg 1944; Marquardt, SIAM J. Appl. Math. 11, 1963) that keeps
//! the step defined when equations are redundant and shortens it when the
//! residual does not drop. Moving the variables as little as possible is
//! what makes a sketch behave: editing one dimension moves what it must
//! and leaves the rest of the drawing alone (the variational geometry of
//! Light & Gossard, "Modification of geometric models through variational
//! geometry", Computer-Aided Design 14(4), 1982). Weights `W` make some
//! variables stiffer than others (the point the user just dragged).
//!
//! After solving, the Jacobian's rows are orthogonalised in order
//! (modified Gram–Schmidt, Golub & Van Loan, *Matrix Computations*, §5.2):
//! a row that is a combination of earlier ones is a *redundant* equation
//! (over-constrained), and `n − rank` is the number of degrees of freedom
//! left. The Jacobian is computed by central differences over the
//! variables each equation depends on.
//!
//! Never panics: a system that cannot be satisfied returns
//! `converged: false` with the best point found.

/// A system of equations `r_i(x) = 0`.
pub trait System {
    /// Number of variables `n`.
    fn vars(&self) -> usize;
    /// Number of equations `m`.
    fn equations(&self) -> usize;
    /// The residual of equation `i` at `x` (`x.len() == n`).
    fn residual(&self, i: usize, x: &[f64]) -> f64;
    /// The variables equation `i` depends on (the others are taken to have
    /// no effect on it).
    fn depends(&self, i: usize) -> &[usize];
}

/// The result of [`solve`].
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// The variables found (the starting point if nothing could be done).
    pub x: Vec<f64>,
    /// Every residual is within the tolerance.
    pub converged: bool,
    /// The largest absolute residual at `x`.
    pub residual: f64,
    /// Rank of the Jacobian at `x`.
    pub rank: usize,
    /// Equations that are combinations of earlier ones at `x`.
    pub redundant: Vec<usize>,
}

impl Solution {
    /// Degrees of freedom left: variables not fixed by the equations.
    pub fn freedom(&self) -> usize {
        self.x.len().saturating_sub(self.rank)
    }
}

/// Solver settings.
#[derive(Clone, Debug)]
pub struct Options {
    /// Largest acceptable absolute residual.
    pub tolerance: f64,
    /// Gauss–Newton iterations at most.
    pub max_iterations: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            tolerance: 1e-7,
            max_iterations: 100,
        }
    }
}

fn residuals(sys: &dyn System, x: &[f64]) -> Vec<f64> {
    (0..sys.equations()).map(|i| sys.residual(i, x)).collect()
}

fn norm2(r: &[f64]) -> f64 {
    r.iter().map(|v| v * v).sum()
}

fn max_abs(r: &[f64]) -> f64 {
    r.iter()
        .fold(0.0, |m, v| if v.abs() > m { v.abs() } else { m })
}

/// The Jacobian as sparse rows: `(variable, ∂r_i/∂x_j)`.
fn jacobian(sys: &dyn System, x: &[f64]) -> Vec<Vec<(usize, f64)>> {
    let mut probe = x.to_vec();
    (0..sys.equations())
        .map(|i| {
            sys.depends(i)
                .iter()
                .filter(|j| **j < x.len())
                .map(|&j| {
                    let x0 = probe.get(j).copied().unwrap_or_default();
                    let h = 1e-6 * x0.abs().max(1.0);
                    if let Some(v) = probe.get_mut(j) {
                        *v = x0 + h;
                    }
                    let plus = sys.residual(i, &probe);
                    if let Some(v) = probe.get_mut(j) {
                        *v = x0 - h;
                    }
                    let minus = sys.residual(i, &probe);
                    if let Some(v) = probe.get_mut(j) {
                        *v = x0;
                    }
                    (j, (plus - minus) / (2.0 * h))
                })
                .collect()
        })
        .collect()
}

/// Solve `a y = b` for a symmetric positive definite `a` (row-major,
/// `m × m`) by Cholesky factorisation; `None` if it is not positive
/// definite.
fn cholesky_solve(a: &[f64], b: &[f64]) -> Option<Vec<f64>> {
    let m = b.len();
    let mut l = vec![0.0; m * m];
    for i in 0..m {
        for j in 0..=i {
            let mut s = *a.get(i * m + j)?;
            for k in 0..j {
                s -= l.get(i * m + k)? * l.get(j * m + k)?;
            }
            if i == j {
                if s <= 0.0 || !s.is_finite() {
                    return None;
                }
                *l.get_mut(i * m + i)? = s.sqrt();
            } else {
                *l.get_mut(i * m + j)? = s / l.get(j * m + j)?;
            }
        }
    }
    // L z = b, then Lᵀ y = z.
    let mut y = b.to_vec();
    for i in 0..m {
        let mut s = *y.get(i)?;
        for k in 0..i {
            s -= l.get(i * m + k)? * y.get(k)?;
        }
        *y.get_mut(i)? = s / l.get(i * m + i)?;
    }
    for i in (0..m).rev() {
        let mut s = *y.get(i)?;
        for k in i + 1..m {
            s -= l.get(k * m + i)? * y.get(k)?;
        }
        *y.get_mut(i)? = s / l.get(i * m + i)?;
    }
    y.iter().all(|v| v.is_finite()).then_some(y)
}

/// The damped minimum-norm step for residuals `r` and Jacobian `jac`,
/// with inverse weights `inv_w`.
fn step(jac: &[Vec<(usize, f64)>], r: &[f64], inv_w: &[f64], lambda: f64) -> Option<Vec<f64>> {
    let m = r.len();
    let mut a = vec![0.0; m * m];
    for (i, ri) in jac.iter().enumerate() {
        for (k, rk) in jac.iter().enumerate().take(i + 1) {
            let mut s = 0.0;
            for (j, v) in ri {
                if let Some((_, w)) = rk.iter().find(|(jj, _)| jj == j) {
                    s += v * w * inv_w.get(*j).copied().unwrap_or(1.0);
                }
            }
            *a.get_mut(i * m + k)? = s;
            *a.get_mut(k * m + i)? = s;
        }
    }
    for i in 0..m {
        *a.get_mut(i * m + i)? += lambda;
    }
    let neg: Vec<f64> = r.iter().map(|v| -v).collect();
    let y = cholesky_solve(&a, &neg)?;
    let mut dx = vec![0.0; inv_w.len()];
    for (row, yi) in jac.iter().zip(&y) {
        for (j, v) in row {
            if let Some(d) = dx.get_mut(*j) {
                *d += v * yi * inv_w.get(*j).copied().unwrap_or(1.0);
            }
        }
    }
    Some(dx)
}

/// Rank of the Jacobian and its redundant rows, in order (modified
/// Gram–Schmidt on the rows).
fn rank(jac: &[Vec<(usize, f64)>], n: usize) -> (usize, Vec<usize>) {
    let mut basis: Vec<Vec<f64>> = Vec::new();
    let mut redundant = Vec::new();
    for (i, row) in jac.iter().enumerate() {
        let mut v = vec![0.0; n];
        for (j, d) in row {
            if let Some(slot) = v.get_mut(*j) {
                *slot += d;
            }
        }
        let original = norm2(&v).sqrt();
        for b in &basis {
            let dot: f64 = v.iter().zip(b).map(|(x, y)| x * y).sum();
            for (x, y) in v.iter_mut().zip(b) {
                *x -= dot * y;
            }
        }
        let left = norm2(&v).sqrt();
        if original == 0.0 || left <= 1e-7 * original {
            redundant.push(i);
        } else {
            basis.push(v.iter().map(|x| x / left).collect());
        }
    }
    (basis.len(), redundant)
}

/// Solve `sys` from `x0`, moving variable `j` as if it weighed
/// `weights[j]` (missing or non-positive weights count as 1).
pub fn solve(sys: &dyn System, x0: &[f64], weights: &[f64], opts: &Options) -> Solution {
    let n = x0.len();
    let inv_w: Vec<f64> = (0..n)
        .map(|j| match weights.get(j) {
            Some(w) if *w > 0.0 && w.is_finite() => 1.0 / w,
            _ => 1.0,
        })
        .collect();
    let mut x = x0.to_vec();
    let mut r = residuals(sys, &x);
    let mut lambda = 1e-9;
    for _ in 0..opts.max_iterations {
        if max_abs(&r) <= opts.tolerance || r.iter().any(|v| !v.is_finite()) {
            break;
        }
        let jac = jacobian(sys, &x);
        let scale = jac
            .iter()
            .map(|row| row.iter().map(|(_, v)| v * v).sum::<f64>())
            .fold(0.0, f64::max)
            .max(1e-12);
        let current = norm2(&r);
        let mut improved = false;
        for _ in 0..12 {
            let Some(dx) = step(&jac, &r, &inv_w, lambda * scale) else {
                lambda *= 10.0;
                continue;
            };
            let trial: Vec<f64> = x.iter().zip(&dx).map(|(a, b)| a + b).collect();
            let tr = residuals(sys, &trial);
            let t = norm2(&tr);
            if t.is_finite() && t < current {
                x = trial;
                r = tr;
                lambda = (lambda / 10.0).max(1e-12);
                improved = true;
                break;
            }
            lambda *= 10.0;
        }
        if !improved {
            break;
        }
    }
    let residual = max_abs(&r);
    let (rank, redundant) = rank(&jacobian(sys, &x), n);
    Solution {
        converged: residual <= opts.tolerance && residual.is_finite(),
        residual,
        x,
        rank,
        redundant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An equation: the variables it reads, and its residual.
    type Equation = (Vec<usize>, Box<dyn Fn(&[f64]) -> f64>);

    /// Equations given as closures over all variables.
    struct Eqs {
        n: usize,
        eqs: Vec<Equation>,
    }

    impl System for Eqs {
        fn vars(&self) -> usize {
            self.n
        }
        fn equations(&self) -> usize {
            self.eqs.len()
        }
        fn residual(&self, i: usize, x: &[f64]) -> f64 {
            self.eqs.get(i).map_or(0.0, |(_, f)| f(x))
        }
        fn depends(&self, i: usize) -> &[usize] {
            self.eqs.get(i).map_or(&[], |(d, _)| d)
        }
    }

    fn dist(x: &[f64], a: usize, b: usize) -> f64 {
        ((x[2 * b] - x[2 * a]).powi(2) + (x[2 * b + 1] - x[2 * a + 1]).powi(2)).sqrt()
    }

    #[test]
    fn a_distance_moves_both_points_least() {
        // Two points 10 apart, asked to be 20 apart: each moves 5.
        let sys = Eqs {
            n: 4,
            eqs: vec![(vec![0, 1, 2, 3], Box::new(|x| dist(x, 0, 1) - 20.0))],
        };
        let s = solve(&sys, &[0.0, 0.0, 10.0, 0.0], &[], &Options::default());
        assert!(s.converged);
        assert!((s.x[0] + 5.0).abs() < 1e-6 && (s.x[2] - 15.0).abs() < 1e-6);
        assert_eq!(s.rank, 1);
        assert_eq!(s.freedom(), 3);
    }

    #[test]
    fn weights_keep_heavy_variables_still() {
        let sys = Eqs {
            n: 4,
            eqs: vec![(vec![0, 1, 2, 3], Box::new(|x| dist(x, 0, 1) - 20.0))],
        };
        let w = [1e6, 1e6, 1.0, 1.0];
        let s = solve(&sys, &[0.0, 0.0, 10.0, 0.0], &w, &Options::default());
        assert!(s.converged);
        assert!(s.x[0].abs() < 1e-4);
        assert!((s.x[2] - 20.0).abs() < 1e-4);
    }

    #[test]
    fn redundant_equations_are_found_and_conflicts_do_not_converge() {
        // x0 = 1 twice: consistent but redundant.
        let twice = Eqs {
            n: 1,
            eqs: vec![
                (vec![0], Box::new(|x| x[0] - 1.0)),
                (vec![0], Box::new(|x| 2.0 * x[0] - 2.0)),
            ],
        };
        let s = solve(&twice, &[0.0], &[], &Options::default());
        assert!(s.converged);
        assert_eq!(s.redundant, vec![1]);
        assert_eq!(s.freedom(), 0);
        // x0 = 1 and x0 = 2: no solution, no panic.
        let conflict = Eqs {
            n: 1,
            eqs: vec![
                (vec![0], Box::new(|x| x[0] - 1.0)),
                (vec![0], Box::new(|x| x[0] - 2.0)),
            ],
        };
        let s = solve(&conflict, &[0.0], &[], &Options::default());
        assert!(!s.converged);
        assert!(s.x[0].is_finite());
    }

    #[test]
    fn a_triangle_with_three_sides_closes() {
        // Points 0 (fixed by weight), 1, 2: sides 30, 40, 50.
        let sys = Eqs {
            n: 6,
            eqs: vec![
                (vec![0, 1, 2, 3], Box::new(|x| dist(x, 0, 1) - 30.0)),
                (vec![2, 3, 4, 5], Box::new(|x| dist(x, 1, 2) - 40.0)),
                (vec![0, 1, 4, 5], Box::new(|x| dist(x, 0, 2) - 50.0)),
            ],
        };
        let s = solve(
            &sys,
            &[0.0, 0.0, 25.0, 3.0, 20.0, 35.0],
            &[],
            &Options::default(),
        );
        assert!(s.converged, "{s:?}");
        assert!((dist(&s.x, 0, 1) - 30.0).abs() < 1e-6);
        assert!((dist(&s.x, 1, 2) - 40.0).abs() < 1e-6);
        assert!((dist(&s.x, 0, 2) - 50.0).abs() < 1e-6);
        // Rigid in the plane: 3 rigid-body motions left.
        assert_eq!(s.freedom(), 3);
    }

    #[test]
    fn impossible_triangle_does_not_converge() {
        let sys = Eqs {
            n: 6,
            eqs: vec![
                (vec![0, 1, 2, 3], Box::new(|x| dist(x, 0, 1) - 10.0)),
                (vec![2, 3, 4, 5], Box::new(|x| dist(x, 1, 2) - 10.0)),
                (vec![0, 1, 4, 5], Box::new(|x| dist(x, 0, 2) - 50.0)),
            ],
        };
        let s = solve(
            &sys,
            &[0.0, 0.0, 10.0, 0.0, 5.0, 8.0],
            &[],
            &Options::default(),
        );
        assert!(!s.converged);
        assert!(s.x.iter().all(|v| v.is_finite()));
    }
}
