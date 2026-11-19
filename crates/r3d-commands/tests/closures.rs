#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Trims in the drape (spec 26): a skirt with a centre-back zipper drapes
//! closed, opens and closes again while it hangs, without starting over.

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, Drape, Form, build, carry_over};
use r3d_commands::{Session, Step};
use r3d_core::{EdgeId, Project};
use r3d_sim::{CpuSolver, Solver};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Value {
    s.run_step(&Step::from_value(v).unwrap()).unwrap()
}

/// The A-line skirt with its centre back zipped over the top 20 cm
/// instead of sewn.
fn zipped_skirt() -> (Session, Value) {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    common::a_line_skirt(&mut s);
    let left = common::piece(&s, "Back left");
    let cb: EdgeId = left.edges[1].as_str().unwrap().parse().unwrap();
    let seam = s.project().seam_of(cb).unwrap();
    let twin = s.project().seams()[&seam].b.edges[0];
    let len = s.project().edge_length(cb).unwrap();
    let (cb, twin) = (json!(cb.to_string()), json!(twin.to_string()));
    run(
        &mut s,
        json!({"type": "seam.remove", "seam": seam.to_string()}),
    );
    // The left back's centre back runs up from the hem; its mirror image
    // runs down.
    run(
        &mut s,
        json!({"type": "seam.add",
            "a": {"edge": cb, "to_mm": len - 200.0},
            "b": {"edge": twin, "from_mm": 200.0, "reversed": true}}),
    );
    let zip = run(
        &mut s,
        json!({"type": "zipper.add",
            "a": {"edge": cb, "from_mm": len - 200.0},
            "b": {"edge": twin, "to_mm": 200.0, "reversed": true}}),
    );
    (s, zip["created"][0].clone())
}

/// Rebuild after an edit and carry on from `solver`'s state, as the app
/// does.
fn carry(s: &Session, form: &Form, old: &Drape, solver: &CpuSolver) -> (Drape, CpuSolver) {
    let mut new = build(s.project(), Some(form), DEFAULT_SPACING_MM).unwrap();
    carry_over(old, solver.positions(), &mut new);
    let mut next = CpuSolver::new(new.scene.clone());
    next.continue_at(solver.time());
    (new, next)
}

fn frames(solver: &mut CpuSolver, n: usize) {
    for _ in 0..n {
        solver.step(1.0 / 60.0).unwrap();
    }
    assert!(solver.positions().iter().all(|p| p.is_finite()));
}

#[test]
fn a_zipper_opens_and_closes_while_the_skirt_hangs() {
    let (mut s, zip) = zipped_skirt();
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let closed = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let pairs = closed.scene.closures.len();
    assert!(pairs >= 8, "{pairs} zipper pairs");
    let mut solver = CpuSolver::new(closed.scene.clone());
    frames(&mut solver, 90);
    let st = solver.stats();
    assert!(st.max_closure_gap < 0.002, "zipper open: {st:?}");
    assert!(st.max_seam_gap < 0.005, "{st:?}");

    // Opened all the way: only the bottom stop holds.
    run(
        &mut s,
        json!({"type": "zipper.set_open", "zipper": zip, "open_mm": 200}),
    );
    let (open, mut solver) = carry(&s, &form, &closed, &solver);
    assert_eq!(open.scene.closures.len(), 1);
    frames(&mut solver, 60);
    // The top of the zipper, its last pair while closed, has come apart.
    let [a, b] = closed.scene.closures.last().unwrap().pair;
    let gap = solver.positions()[a as usize].distance(solver.positions()[b as usize]);
    assert!(gap > 0.005, "the opened zipper stayed shut: {gap} m");

    // Closed again: back within a millimetre or two in a second.
    run(
        &mut s,
        json!({"type": "zipper.set_open", "zipper": zip, "open_mm": 0}),
    );
    let (again, mut solver) = carry(&s, &form, &open, &solver);
    assert_eq!(again.scene.closures.len(), pairs);
    frames(&mut solver, 60);
    let st = solver.stats();
    assert!(st.max_closure_gap < 0.002, "did not close again: {st:?}");
    assert!(st.max_speed < 3.0, "{st:?}");
}
