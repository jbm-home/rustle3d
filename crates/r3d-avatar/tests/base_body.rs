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
    use r3d_avatar::fit::fit;
    use r3d_core::Body;
    let model = BodyModel::builtin().unwrap();
    let specs = [
        Body {
            masculinity: 0.0,
            height_cm: 168.0,
            bust_cm: 92.0,
            waist_cm: 74.0,
            hips_cm: 100.0,
            age_years: None,
        },
        Body {
            masculinity: 0.0,
            height_cm: 160.0,
            bust_cm: 84.0,
            waist_cm: 66.0,
            hips_cm: 92.0,
            age_years: None,
        },
        Body {
            masculinity: 1.0,
            height_cm: 180.0,
            bust_cm: 100.0,
            waist_cm: 86.0,
            hips_cm: 100.0,
            age_years: None,
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
        let off = |got: f32, want: f64| (got as f64 - want).abs();
        assert!(off(a.height_cm, spec.height_cm) < 0.5);
        assert!(off(a.bust_cm, spec.bust_cm) < 1.5, "bust");
        assert!(off(a.waist_cm, spec.waist_cm) < 1.5, "waist");
        assert!(off(a.hips_cm, spec.hips_cm) < 1.5, "hips");
    }
}

#[test]
fn a_childs_head_is_larger_and_its_body_fits_its_chart() {
    use r3d_avatar::fit::{fit, proportions};
    use r3d_avatar::presets::Children;
    let model = BodyModel::builtin().unwrap();
    for m in [0.0, 1.0] {
        let adult = proportions(&model, m, None).unwrap();
        let mut last = proportions(&model, m, Some(1.0)).unwrap();
        // Neck, shoulders and crotch rise on the stature as the child
        // grows, up to the adult's.
        for age in [3.0, 6.0, 11.0, 25.0] {
            let p = proportions(&model, m, Some(age)).unwrap();
            assert!(
                p.neck >= last.neck && p.crotch >= last.crotch,
                "{age}: {p:?}"
            );
            last = p;
        }
        assert!((last.neck - adult.neck).abs() < 1e-4);
    }
    // Every girl's size is reached within half a centimetre.
    for (name, body) in Children::Girls.sizes() {
        let f = fit(&model, &body);
        let a = f.achieved;
        for (got, want) in [
            (a.height_cm, body.height_cm),
            (a.bust_cm, body.bust_cm),
            (a.waist_cm, body.waist_cm),
            (a.hips_cm, body.hips_cm),
        ] {
            assert!((f64::from(got) - want).abs() < 0.5, "{name}: {a:?}");
        }
    }
}
