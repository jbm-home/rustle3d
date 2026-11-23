#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_avatar::fit::{fit, fitted_body};
use r3d_avatar::model::BodyModel;
use r3d_avatar::presets::builtin;
use r3d_avatar::tape::tape;
use r3d_core::Measure;

/// The estimates of `r3d_core::body` were fitted to what the body model
/// measures: on every preset they stay within this (cm).
const AGREEMENT_CM: f64 = 2.5;

#[test]
fn estimates_are_what_the_avatar_measures() {
    let model = BodyModel::builtin().unwrap();
    for preset in builtin() {
        let f = fit(&model, &preset.body);
        let taped = tape(&model, &fitted_body(&model, &f)).unwrap();
        // The estimate from what the fitted body reaches.
        let a = f.achieved;
        let reached = r3d_core::Body {
            masculinity: preset.body.masculinity,
            height_cm: a.height_cm.into(),
            bust_cm: a.bust_cm.into(),
            waist_cm: a.waist_cm.into(),
            hips_cm: a.hips_cm.into(),
        };
        for m in Measure::ALL {
            let (got, want) = (f64::from(taped.get(m)), m.estimate(&reached));
            assert!(
                (got - want).abs() <= AGREEMENT_CM,
                "{}: {} measures {got:.1} cm, estimated {want:.1}",
                preset.name,
                m.name()
            );
        }
    }
}

#[test]
fn a_womans_measurements_are_plausible() {
    let model = BodyModel::builtin().unwrap();
    let women_m = builtin().into_iter().find(|p| p.name == "Women M").unwrap();
    let f = fit(&model, &women_m.body);
    let t = tape(&model, &fitted_body(&model, &f)).unwrap();
    // Sensible for a 168 cm woman, as the body model is built.
    let within = |m: Measure, lo: f32, hi: f32| {
        let v = t.get(m);
        assert!((lo..=hi).contains(&v), "{}: {v:.1}", m.name());
    };
    within(Measure::Neck, 32.0, 40.0);
    within(Measure::ShoulderWidth, 33.0, 40.0);
    within(Measure::InsideLeg, 72.0, 84.0);
    within(Measure::WaistHeight, 100.0, 112.0);
    within(Measure::CrotchDepth, 24.0, 32.0);
    // Down the body: waist above the hips above the crotch.
    assert!(t.get(Measure::WaistHeight) > t.get(Measure::HipHeight));
    assert!(t.get(Measure::HipHeight) > t.get(Measure::InsideLeg));
}

#[test]
fn the_body_follows_typed_measurements() {
    let model = BodyModel::builtin().unwrap();
    let body = builtin()
        .into_iter()
        .find(|p| p.name == "Women M")
        .unwrap()
        .body;
    let typed: r3d_core::Measures = [
        (Measure::ArmLength, Measure::ArmLength.estimate(&body) + 5.0),
        (Measure::Neck, Measure::Neck.estimate(&body) + 3.0),
        (Measure::InsideLeg, Measure::InsideLeg.estimate(&body) - 4.0),
    ]
    .into_iter()
    .collect();
    let start = std::time::Instant::now();
    let f = r3d_avatar::fit::fit_typed(&model, &body, &typed);
    eprintln!("fitted in {:.1} s", start.elapsed().as_secs_f32());
    let t = tape(&model, &fitted_body(&model, &f)).unwrap();
    for (m, want) in &typed {
        let got = f64::from(t.get(*m));
        eprintln!("{}: {got:.1} for {want:.1}", m.name());
        assert!(
            (got - want).abs() < 1.0,
            "{}: {got:.1} for {want:.1}",
            m.name()
        );
    }
    // The four stay.
    let a = f.achieved;
    assert!((f64::from(a.height_cm) - body.height_cm).abs() < 0.5);
    assert!((f64::from(a.waist_cm) - body.waist_cm).abs() < 1.0);
}
