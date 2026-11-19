#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Several garments in one project, each in its own fabric and colour,
//! worn in layers (M19).

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Session, Step};
use r3d_core::{Fabric, GarmentId, Project};
use r3d_sim::{CpuSolver, Solver};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Result<Value, r3d_commands::CommandError> {
    s.run_step(&Step::from_value(v).unwrap())
}

/// Trousers, then a shirt, on the standard women's M body.
fn outfit() -> Session {
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": "trousers"}),
    )
    .unwrap();
    run(
        &mut s,
        json!({"type": "template.apply", "template": "shirt"}),
    )
    .unwrap();
    s
}

fn garment(s: &Session, name: &str) -> GarmentId {
    let p = s.project();
    *p.garments().iter().find(|(_, g)| g.name == name).unwrap().0
}

#[test]
fn a_second_template_is_a_garment_of_its_own_worn_over_the_first() {
    let s = outfit();
    let p = s.project();
    assert_eq!(p.name(), "Trousers");
    let (trousers, shirt) = (garment(&s, "Trousers"), garment(&s, "Shirt"));
    assert_eq!(p.garments()[&trousers].fabric, Fabric::Denim);
    assert_eq!(p.garments()[&shirt].fabric, Fabric::Cotton);
    assert_eq!(p.garments_by_layer(), vec![trousers, shirt]);
    let in_shirt = p.panels().values().filter(|pn| pn.garment == shirt).count();
    assert_eq!(in_shirt, 8);
    let summary = s
        .query(&serde_json::from_value(json!({"type": "project.summary"})).unwrap())
        .unwrap();
    assert_eq!(summary["garments"][1]["name"], "Shirt");
    assert_eq!(summary["garments"][1]["layer"], 1);
}

#[test]
fn layers_reorder_and_the_last_garment_stays() {
    let mut s = outfit();
    let (trousers, shirt) = (garment(&s, "Trousers"), garment(&s, "Shirt"));
    run(
        &mut s,
        json!({"type": "garment.set_layer", "garment": shirt, "layer": 0}),
    )
    .unwrap();
    assert_eq!(s.project().garments_by_layer(), vec![shirt, trousers]);
    let too_far = run(
        &mut s,
        json!({"type": "garment.set_layer", "garment": shirt, "layer": 2}),
    );
    assert!(too_far.is_err());
    run(
        &mut s,
        json!({"type": "garment.remove", "garment": shirt, "confirm": true}),
    )
    .unwrap();
    let p = s.project();
    assert_eq!(p.garments().len(), 1);
    assert_eq!(p.garments()[&trousers].layer, 0);
    assert_eq!(p.panels().len(), 4);
    let last = run(
        &mut s,
        json!({"type": "garment.remove", "garment": trousers, "confirm": true}),
    );
    assert!(last.is_err());
}

#[test]
fn a_piece_moves_with_its_mirror_copy() {
    let mut s = outfit();
    let shirt = garment(&s, "Shirt");
    let p = s.project();
    let (front, _) = p
        .panels()
        .iter()
        .find(|(_, pn)| pn.name == "Front left" && pn.garment != shirt)
        .unwrap();
    let front = *front;
    run(
        &mut s,
        json!({"type": "panel.set_garment", "panel": front, "garment": shirt}),
    )
    .unwrap();
    let p = s.project();
    let moved: Vec<&str> = p
        .panels()
        .values()
        .filter(|pn| pn.garment == shirt && pn.name.starts_with("Front"))
        .map(|pn| pn.name.as_str())
        .collect();
    // The trousers' front left and its mirror copy, besides the shirt's
    // two fronts.
    assert_eq!(moved.len(), 4, "{moved:?}");
}

#[test]
fn each_garment_drapes_in_its_own_fabric_and_starts_over_the_one_under_it() {
    let s = outfit();
    let p = s.project();
    let form = Form::of_project(p).unwrap().unwrap();
    let drape = build(p, Some(&form), DEFAULT_SPACING_MM).unwrap();
    assert_eq!(drape.scene.fabrics.len(), 2);
    let shirt = garment(&s, "Shirt");
    let fabric_of = |piece: &r3d_commands::drape::Piece| {
        let first = piece.indices[0] as usize + piece.particles.start;
        drape
            .scene
            .triangles
            .iter()
            .find(|t| t.vertices[0] as usize == first)
            .map(|t| t.fabric)
    };
    for piece in &drape.pieces {
        let expected = u16::from(p.panel(piece.panel).unwrap().garment == shirt);
        assert_eq!(fabric_of(piece), Some(expected));
    }
    // At the hips the shirt starts outside the trousers, all around.
    let radius = |garment: GarmentId, inner: bool| {
        drape
            .pieces
            .iter()
            .filter(|pc| p.panel(pc.panel).unwrap().garment == garment)
            .flat_map(|pc| drape.scene.particles[pc.particles.clone()].iter())
            .map(|q| q.position)
            .filter(|q| (q.y - 0.95).abs() < 0.02 && q.z > 0.05)
            .map(|q| q.x.hypot(q.z))
            .fold(if inner { 0.0 } else { f32::MAX }, |m, r| {
                if inner { m.max(r) } else { m.min(r) }
            })
    };
    let trousers_out = radius(garment(&s, "Trousers"), true);
    let shirt_in = radius(shirt, false);
    assert!(shirt_in > trousers_out, "{shirt_in} vs {trousers_out}");

    let mut solver = CpuSolver::new(drape.scene.clone());
    for _ in 0..120 {
        solver.step(1.0 / 60.0).unwrap();
    }
    assert!(solver.positions().iter().all(|v| v.is_finite()));
    let stats = solver.stats();
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
}

/// A T-shirt over trousers starts outside them at every height and angle
/// where both are: loose trousers start wider than the body plus a layer
/// gap at the front and back of the hips, and the T-shirt's hem started
/// inside them there (it then drapes tangled in them).
#[test]
fn a_t_shirt_starts_outside_the_trousers_all_around() {
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    for template in ["trousers", "t_shirt"] {
        run(
            &mut s,
            json!({"type": "template.apply", "template": template}),
        )
        .unwrap();
    }
    let p = s.project();
    let form = Form::of_project(p).unwrap().unwrap();
    let drape = build(p, Some(&form), DEFAULT_SPACING_MM).unwrap();
    let trousers = garment(&s, "Trousers");
    // Per 2 cm of height and 15° of angle, the trousers' farthest
    // particle and the T-shirt's nearest one from the body's axis.
    let cell = |q: glam::Vec3| {
        let angle = q.x.atan2(q.z).rem_euclid(std::f32::consts::TAU);
        ((q.y / 0.02) as i32, (angle / 15f32.to_radians()) as i32)
    };
    let mut out = std::collections::BTreeMap::new();
    let mut inn = std::collections::BTreeMap::new();
    for pc in &drape.pieces {
        let is_trousers = p.panel(pc.panel).unwrap().garment == trousers;
        for q in &drape.scene.particles[pc.particles.clone()] {
            let (c, r) = (cell(q.position), q.position.x.hypot(q.position.z));
            if r > 0.3 {
                continue; // sleeves
            }
            if is_trousers {
                let m = out.entry(c).or_insert(0.0f32);
                *m = m.max(r);
            } else {
                let m = inn.entry(c).or_insert(f32::MAX);
                *m = m.min(r);
            }
        }
    }
    let worst = inn
        .iter()
        .filter_map(|(c, r)| out.get(c).map(|t| (r - t, *c)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap();
    assert!(
        worst.0 > 0.02,
        "the T-shirt starts inside the trousers: {worst:?}"
    );
}

/// Two templates on the women's M body, `inner` worn under `outer`; the
/// scene to drape.
fn layered(inner: &str, outer: &str) -> r3d_sim::SimScene {
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    for template in [inner, outer] {
        run(
            &mut s,
            json!({"type": "template.apply", "template": template}),
        )
        .unwrap();
    }
    let p = s.project();
    let form = Form::of_project(p).unwrap().unwrap();
    build(p, Some(&form), DEFAULT_SPACING_MM).unwrap().scene
}

/// Drape `scene` for `frames` frames; the positions, after the usual
/// checks (finite, seams closed).
fn drape(scene: &r3d_sim::SimScene, frames: u32) -> Vec<glam::Vec3> {
    let mut solver = CpuSolver::new(scene.clone());
    for _ in 0..frames {
        solver.step(1.0 / 60.0).unwrap();
    }
    let x = solver.positions().to_vec();
    assert!(x.iter().all(|v| v.is_finite()));
    let stats = solver.stats();
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    x
}

/// A shirt worn under trousers drapes tucked into them: from the hips to
/// just under the waistband, no shirt shows through the trousers, front
/// or back (thin contacts between garments, docs/physics.md). Particle
/// repulsion kept layers 2 cm apart, and the trousers' waist snapped
/// through the shirt.
#[test]
fn a_shirt_under_trousers_is_tucked_in() {
    let scene = layered("shirt", "trousers");
    let x = drape(&scene, 150);
    // The shirt is garment 0, the trousers 1; their waistband's top.
    let top = scene
        .triangles
        .iter()
        .filter(|t| t.garment == 1)
        .flat_map(|t| t.vertices)
        .map(|v| x[v as usize])
        .filter(|p| p.x.abs() < 0.1)
        .map(|p| p.y)
        .fold(0.0f32, f32::max);
    let seen = common::seen_through(&scene, &x, 0, 1, 0.85, top - 0.03);
    assert!(
        seen.is_empty(),
        "shirt showing through the trousers below {top}: {:?}",
        seen.iter().map(|i| (i, x[*i])).collect::<Vec<_>>()
    );
}

/// A T-shirt worn over trousers hides them where it covers them: the
/// trousers showed through it in places.
#[test]
fn trousers_do_not_show_through_a_t_shirt_over_them() {
    let scene = layered("trousers", "t_shirt");
    let x = drape(&scene, 150);
    let hem = scene
        .triangles
        .iter()
        .filter(|t| t.garment == 1)
        .flat_map(|t| t.vertices)
        .map(|v| x[v as usize])
        .filter(|p| p.x.abs() < 0.1)
        .map(|p| p.y)
        .fold(f32::MAX, f32::min);
    let seen = common::seen_through(&scene, &x, 0, 1, hem + 0.03, 1.15);
    assert!(
        seen.is_empty(),
        "trousers showing through the T-shirt above {hem}: {seen:?}"
    );
}
