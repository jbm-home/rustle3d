#![allow(clippy::unwrap_used)] // test code
//! The avatar moves and the garment follows (spec 22).

mod common;

use glam::Vec3;
use r3d_avatar::motion::Motion;
use r3d_commands::drape::{Animation, MOTION_START_S, STOP_S, load_cache, simulate};
use r3d_commands::{Progress, Session, Step, Workspace};
use r3d_core::Project;
use r3d_sim::motion::{Posed, touch};
use serde_json::json;

fn dressed(template: &str) -> Session {
    let mut s = Session::new(Project::new("t"));
    common::women_m(&mut s);
    let step = json!({"type": "template.apply", "template": template});
    s.run_step(&Step::from_value(step).unwrap()).unwrap();
    s
}

#[test]
fn choosing_motions_never_makes_the_body_jump() {
    // Chosen before the garment has closed: it starts at 1.5 s.
    let walk = Animation::switch(None, Some(Motion::Walk), 0.2).unwrap();
    assert_eq!(walk.start, MOTION_START_S);
    // Chosen again: nothing changes.
    assert_eq!(
        Animation::switch(Some(walk), Some(Motion::Walk), 3.0),
        Some(walk)
    );
    // Not begun yet: replaced at once.
    let turn = Animation::switch(Some(walk), Some(Motion::Turn), 1.0).unwrap();
    assert_eq!((turn.motion, turn.stop), (Motion::Turn, None));
    // Walking: back to rest first, then turning.
    let next = Animation::switch(Some(walk), Some(Motion::Turn), 5.0).unwrap();
    assert_eq!(
        (next.motion, next.stop, next.then),
        (Motion::Walk, Some(5.0), Some(Motion::Turn))
    );
    assert_eq!(next.chosen(), Some(Motion::Turn));
    let later = next.settled(5.0 + STOP_S).unwrap();
    assert_eq!(
        (later.motion, later.start, later.stop),
        (Motion::Turn, 5.0 + STOP_S, None)
    );
    // Standing still: back to rest, then nothing.
    let still = Animation::switch(Some(walk), None, 5.0).unwrap();
    assert_eq!(still.chosen(), None);
    assert!(still.settled(5.5).is_some());
    assert!(still.settled(5.0 + STOP_S).is_none());
    // Changing one's mind while coming back to rest keeps the way back.
    let again = Animation::switch(Some(still), Some(Motion::RaiseArms), 5.5).unwrap();
    assert_eq!(
        (again.stop, again.then),
        (Some(5.0), Some(Motion::RaiseArms))
    );
    // A drape started again starts the chosen motion again.
    assert_eq!(
        next.restarted(),
        Some(Animation::starting(Motion::Turn, 0.0))
    );
    assert_eq!(still.restarted(), None);
}

/// Drape `template` while the avatar does `motion` for `frames` frames:
/// the cloth stays out of the moving body, the seams closed.
fn follows(template: &str, motion: Motion, frames: u32) -> r3d_commands::drape::Simulated {
    let s = dressed(template);
    let out = simulate(s.project(), frames, Some(motion), &Progress::default()).unwrap();
    assert!(out.positions.iter().all(|p| p.is_finite()), "{template}");
    let scene = &out.drape.scene;
    let (body, moving) = (
        scene.collider.as_ref().unwrap(),
        scene.motion.as_ref().unwrap(),
    );
    let t = frames as f32 / 60.0;
    let posed = Posed::new(moving.at(t - 1.0 / 1800.0), moving.at(t));
    let inside = out
        .positions
        .iter()
        .filter_map(|p| touch(body, Some(&posed), *p))
        .map(|(c, _)| -c.distance)
        .fold(0.0f32, f32::max);
    assert!(
        inside < 0.003,
        "{template} {motion:?}: {inside} m inside the body"
    );
    assert!(
        out.stats.max_seam_gap < 0.01,
        "{template} {motion:?}: {:?}",
        out.stats
    );
    out
}

/// Centre of the particles of the panel called `name`.
fn centre(s: &r3d_commands::drape::Simulated, project: &Project, name: &str) -> Vec3 {
    let piece = s
        .drape
        .pieces
        .iter()
        .find(|p| project.panel(p.panel).unwrap().name == name)
        .unwrap();
    let points = &s.positions[piece.particles.clone()];
    points.iter().sum::<Vec3>() / points.len() as f32
}

#[test]
fn a_dress_walks_with_the_avatar() {
    // 1.5 s of draping, then a second of easing in and a step cycle.
    let out = follows("dress", Motion::Walk, 220);
    let top = out.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(top > 1.2, "the dress slid down: {top}");
}

#[test]
fn a_skirt_turns_with_the_avatar() {
    // Half round, in the pause (1.5 + 2.5 s), about the vertical between
    // the feet: the front piece, 11 cm in front of it at rest, is now
    // behind it.
    let s = dressed("a_line_skirt");
    let out = follows("a_line_skirt", Motion::Turn, 240);
    let front = centre(&out, s.project(), "Front");
    assert!(front.z < -0.02, "the skirt did not turn: front at {front}");
}

#[test]
fn a_t_shirt_rides_up_when_the_arms_go_up() {
    let still = simulate(
        dressed("t_shirt").project(),
        200,
        None,
        &Progress::default(),
    )
    .unwrap();
    // Arms up from 1.5 + 1.5 s, held until 1.5 + 2 s.
    let raised = follows("t_shirt", Motion::RaiseArms, 200);
    let hem = |s: &r3d_commands::drape::Simulated| {
        s.positions.iter().map(|p| p.y).fold(f32::MAX, f32::min)
    };
    assert!(
        hem(&raised) > hem(&still) + 0.02,
        "the hem stayed at {} (still: {})",
        hem(&raised),
        hem(&still)
    );
}

#[test]
fn a_drape_with_a_motion_is_cached_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.r3d");
    let mut ws = Workspace::create(&path, "p").unwrap();
    for step in [
        json!({"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
            "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
        json!({"type": "template.apply", "template": "a_line_skirt"}),
    ] {
        ws.run_step(&Step::from_value(step).unwrap()).unwrap();
    }
    let step = json!({"type": "simulate", "frames": 120, "motion": "walk"});
    let out = ws.run_step(&Step::from_value(step).unwrap()).unwrap();
    assert_eq!(out["motion"], "walk");
    assert_eq!(out["motion_s"], 0.5);
    let particles = simulate(ws.session().project(), 1, None, &Progress::default())
        .unwrap()
        .positions
        .len();
    let cached = load_cache(&path, ws.session().project(), particles).unwrap();
    assert_eq!(
        cached.animation,
        Some(Animation::starting(Motion::Walk, 0.0))
    );
    assert!((cached.time - 2.0).abs() < 1e-6);
    // Deterministic: the same drape again gives the same cloth.
    let again = simulate(
        ws.session().project(),
        120,
        Some(Motion::Walk),
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(again.positions, cached.positions);
    // An unknown motion is refused.
    let bad = json!({"type": "simulate", "frames": 10, "motion": "dance"});
    assert!(Step::from_value(bad).is_err());
}
