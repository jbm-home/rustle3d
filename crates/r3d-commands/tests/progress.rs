//! Long steps report their progress and stop when asked, changing nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use r3d_commands::{Progress, Session, Status, Step};
use r3d_core::{Localizable, Project};
use serde_json::json;

fn graded() -> Session {
    let mut s = Session::new(Project::new("t"));
    let chart = json!({"type": "grading.use_chart", "chart": "women", "from": "36", "to": "44", "base": "40"});
    s.run_step(&Step::from_value(chart).unwrap()).unwrap();
    s
}

fn shirt() -> Step {
    Step::from_value(json!({"type": "template.apply", "template": "t_shirt"})).unwrap()
}

#[test]
fn a_graded_template_reports_each_size() {
    let mut s = graded();
    let seen: Arc<Mutex<Vec<Status>>> = Arc::default();
    let log = seen.clone();
    let progress = Progress::new(
        move |status| log.lock().unwrap().push(status.clone()),
        Arc::default(),
    );
    s.run_step_with(&shirt(), &progress).unwrap();
    let seen = seen.lock().unwrap();
    // A start, then each of the 5 sizes (36 to 44), in any order.
    assert_eq!(seen.len(), 6);
    assert!(
        seen.iter()
            .all(|st| st.task == "template.sizes" && st.total == 5)
    );
    let mut done: Vec<u32> = seen.iter().map(|st| st.done).collect();
    done.sort_unstable();
    assert_eq!(done, [0, 1, 2, 3, 4, 5]);
    let mut sizes: Vec<&str> = seen.iter().map(|st| st.item.as_str()).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, ["", "36", "38", "40", "42", "44"]);
}

#[test]
fn a_cancelled_step_changes_nothing() {
    let mut s = graded();
    let before = s.project().clone();
    let undo_steps = s.history().len();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    // Stop as soon as the drafting starts.
    let progress = Progress::new(move |_| stop.store(true, Ordering::Relaxed), cancel);
    let err = s.run_step_with(&shirt(), &progress).unwrap_err();
    assert_eq!(err.message().code, "cancelled");
    assert_eq!(s.project(), &before);
    assert_eq!(s.history().len(), undo_steps);
    // Without a cancel, the same step goes through.
    s.run_step(&shirt()).unwrap();
}
