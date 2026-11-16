#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

mod common;

use r3d_commands::{CommandError, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, CommandError> {
    Step::from_value(v).and_then(|step| s.run_step(&step))
}

fn skirt() -> (Session, common::Piece) {
    let mut s = Session::new(Project::new("t"));
    let front = common::a_line_skirt(&mut s);
    (s, front)
}

fn max_mismatch(s: &mut Session) -> f64 {
    let seams = run(s, json!({"type": "seam.list"})).unwrap();
    seams
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["mismatch_mm"].as_f64().unwrap())
        .fold(0.0, f64::max)
}

/// (x, y) of a point, mm.
fn pos(s: &Session, id: &Value) -> (f64, f64) {
    let p = s
        .project()
        .point(id.as_str().unwrap().parse().unwrap())
        .unwrap()
        .pos;
    (p.x, p.y)
}

#[test]
fn taking_in_a_side_seam_keeps_it_matched() {
    let (mut s, front) = skirt();
    // The front's right side, sewn to the back left.
    let edge: r3d_core::EdgeId = front.edges[1].as_str().unwrap().parse().unwrap();
    let side_seam = json!(s.project().seam_of(edge).unwrap().to_string());
    let (hem, waist) = (front.points[1].clone(), front.points[2].clone());
    let (hem0, waist0) = (pos(&s, &hem), pos(&s, &waist));

    run(
        &mut s,
        json!({"type": "seam.take_in", "seam": side_seam, "mm": 20}),
    )
    .unwrap();
    assert!(max_mismatch(&mut s) < 0.5);
    // The front's right side moved inward (left) at both ends.
    assert!(pos(&s, &hem).0 < hem0.0 - 5.0 && pos(&s, &waist).0 < waist0.0 - 5.0);

    s.undo().unwrap();
    assert_eq!(pos(&s, &hem), hem0);

    // Only at the waist: the hem stays. The back is a linked mirror, so the
    // left side seam is taken in too (its waist point moves right), and
    // every seam stays matched even though the change is tapered.
    let left_waist = front.points[3].clone();
    let left0 = pos(&s, &left_waist);
    run(
        &mut s,
        json!({"type": "seam.take_in", "seam": side_seam, "mm": 60, "at": waist}),
    )
    .unwrap();
    assert_eq!(pos(&s, &hem), hem0);
    assert!(pos(&s, &waist).0 < waist0.0 - 20.0);
    assert!(pos(&s, &left_waist).0 > left0.0 + 20.0);
    assert!(max_mismatch(&mut s) < 0.5, "{}", max_mismatch(&mut s));

    let err = run(
        &mut s,
        json!({"type": "seam.take_in", "seam": side_seam, "mm": 5, "at": front.points[0]}),
    )
    .unwrap_err();
    assert_eq!(err.message().code, "invalid.not_seam_end");
}

#[test]
fn every_seam_and_end_stays_matched() {
    for mm in [-30.0, 30.0] {
        let (s0, _) = skirt();
        let seams: Vec<_> = s0.project().seams().keys().copied().collect();
        for seam in seams {
            let ends = {
                let p = s0.project();
                let e = p.edge(p.seams()[&seam].a.edge).unwrap();
                [None, Some(e.start), Some(e.end)]
            };
            for at in ends {
                let (mut s, _) = skirt();
                let mut step = json!({"type": "seam.take_in", "seam": seam.to_string()});
                if let Some(at) = at {
                    step["at"] = json!(at.to_string());
                }
                // In small steps, as a drag in 3D applies it.
                let mut ok = true;
                for _ in 0..10 {
                    let mut part = step.clone();
                    part["mm"] = json!(mm / 10.0);
                    ok &= run(&mut s, part).is_ok();
                }
                if ok {
                    let m = max_mismatch(&mut s);
                    assert!(m < 0.5, "{seam} at {at:?} by {mm}: mismatch {m}");
                }
            }
        }
    }
}
