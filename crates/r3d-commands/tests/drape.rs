#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! End to end: a sewn skirt on the avatar drapes and stays on the hips.

use r3d_avatar::fit::{fit, fitted_positions};
use r3d_commands::drape::{BodyMesh, DEFAULT_SPACING_MM, build};
use r3d_commands::queries::body_model;
use r3d_commands::{Script, Session};
use r3d_core::Project;
use r3d_sim::Solver;

#[test]
fn demo_skirt_closes_and_stays_on_the_body() {
    let mut s = Session::new(Project::new("t"));
    let script = Script::parse(include_str!("../../../demos/m4/skirt.json")).unwrap();
    let results = script.run(|step| s.run_step(step)).unwrap();
    let seams = results.last().unwrap();
    assert_eq!(seams.as_array().unwrap().len(), 3);

    let project = s.project();
    let body = project.avatar().unwrap().body;
    let model = body_model().unwrap();
    let positions = fitted_positions(model, &fit(model, &body));
    let drape = build(
        project,
        Some(BodyMesh {
            positions: &positions,
            triangles: model.triangles(),
        }),
        DEFAULT_SPACING_MM,
    )
    .unwrap();
    assert!(!drape.scene.seams.is_empty());
    // All pieces share the same waistline and hem: sewn points pair at the
    // same pattern height (a reversed seam would twist the skirt).
    let rest = |i: u32| drape.scene.particles[i as usize].rest.y;
    let twist = drape
        .scene
        .seams
        .iter()
        .map(|[a, b]| (rest(*a) - rest(*b)).abs())
        .fold(0.0, f32::max);
    assert!(twist < 0.002, "twisted seam: {twist} m");
    let n = drape.scene.particles.len();
    eprintln!(
        "{n} particles, {} triangles, {} seam pairs",
        drape.scene.triangles.len(),
        drape.scene.seams.len()
    );

    let mut solver = Solver::new(drape.scene);
    let start = std::time::Instant::now();
    for _ in 0..120 {
        solver.step(1.0 / 60.0);
    }
    let per_frame = start.elapsed() / 120;
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    let stats = solver.stats();
    let low = p.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let high = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    eprintln!("{per_frame:?}/frame, stats {stats:?}, y {low:.3}..{high:.3}");
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    // Placed at the waist (1.10 m) and fitted to it, it hangs above the
    // hips (widest at 0.82 m). It still creeps down a few centimetres: the
    // membrane is not fully converged at this step count (docs/physics.md,
    // known issues).
    assert!(high > 0.95, "skirt slid down: top at {high}");
    assert!(low > 0.2, "skirt collapsed: hem at {low}");
}
