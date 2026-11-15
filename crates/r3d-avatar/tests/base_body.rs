#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_avatar::measure::measure;
use r3d_avatar::model::{BodyModel, BodyShape};

#[test]
fn builtin_body_measures_like_an_adult() {
    let model = BodyModel::builtin().unwrap();
    for (label, m) in [("female", 0.0), ("male", 1.0)] {
        let shape = BodyShape {
            masculinity: m,
            ..Default::default()
        };
        let pos = model.positions(&shape);
        let me = measure(&pos, model.triangles(), model.landmarks());
        eprintln!("{label}: {me:?}");
        assert!(
            (150.0..195.0).contains(&me.height_cm),
            "{label} height {me:?}"
        );
        assert!((70.0..120.0).contains(&me.bust_cm), "{label} bust {me:?}");
        assert!((55.0..110.0).contains(&me.waist_cm), "{label} waist {me:?}");
        assert!((75.0..125.0).contains(&me.hips_cm), "{label} hips {me:?}");
        assert!(me.waist_cm < me.hips_cm && me.waist_cm < me.bust_cm);
    }
}

#[test]
fn fitting_reaches_typical_measurements() {
    use r3d_avatar::fit::{BodySpec, fit};
    let model = BodyModel::builtin().unwrap();
    let specs = [
        BodySpec {
            masculinity: 0.0,
            height_cm: 168.0,
            bust_cm: 92.0,
            waist_cm: 74.0,
            hips_cm: 100.0,
        },
        BodySpec {
            masculinity: 0.0,
            height_cm: 160.0,
            bust_cm: 84.0,
            waist_cm: 66.0,
            hips_cm: 92.0,
        },
        BodySpec {
            masculinity: 1.0,
            height_cm: 180.0,
            bust_cm: 100.0,
            waist_cm: 86.0,
            hips_cm: 100.0,
        },
    ];
    for spec in specs {
        let start = std::time::Instant::now();
        let f = fit(&model, &spec);
        let a = f.achieved;
        eprintln!(
            "{spec:?}\n -> {a:?} in {:?} ({:?})",
            start.elapsed(),
            f.shape
        );
        assert!((a.height_cm - spec.height_cm).abs() < 0.5);
        assert!((a.bust_cm - spec.bust_cm).abs() < 1.5, "bust");
        assert!((a.waist_cm - spec.waist_cm).abs() < 1.5, "waist");
        assert!((a.hips_cm - spec.hips_cm).abs() < 1.5, "hips");
    }
}
