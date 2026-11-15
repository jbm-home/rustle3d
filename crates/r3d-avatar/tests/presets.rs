#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_avatar::fit::fit;
use r3d_avatar::model::BodyModel;
use r3d_avatar::presets::{BodyPreset, PresetLibrary, builtin};

#[test]
fn every_builtin_size_can_be_fitted() {
    let model = BodyModel::builtin().unwrap();
    for preset in builtin() {
        let a = fit(&model, &preset.body).achieved;
        let b = preset.body;
        let worst = [
            (a.bust_cm - b.bust_cm).abs(),
            (a.waist_cm - b.waist_cm).abs(),
            (a.hips_cm - b.hips_cm).abs(),
        ]
        .into_iter()
        .fold(0.0, f32::max);
        eprintln!("{}: off by at most {worst:.2} cm", preset.name);
        // Within 1.5 cm for common sizes; the XL sizes reach the edge of the
        // morph targets' range (moderately extrapolated) and land within 3 cm.
        // The UI always shows the measurements actually achieved.
        assert!(worst < 3.0, "{} off by {worst} cm: {a:?}", preset.name);
    }
}

#[test]
fn library_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("sub/body-presets.json");
    assert!(PresetLibrary::load(&path).unwrap().presets.is_empty());
    let mut lib = PresetLibrary::default();
    let mut p = BodyPreset {
        name: "Me".into(),
        body: builtin()[1].body,
    };
    lib.upsert(p.clone());
    p.body.waist_cm = 70.0;
    lib.upsert(p.clone());
    assert_eq!(lib.presets.len(), 1);
    lib.save(&path).unwrap();
    assert_eq!(PresetLibrary::load(&path).unwrap(), lib);
    assert!(lib.remove("Me"));
    assert!(!lib.remove("Me"));
    std::fs::write(&path, "nope").unwrap();
    assert!(PresetLibrary::load(&path).is_err());
}
