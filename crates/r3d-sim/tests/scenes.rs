#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! The reference scenes (docs/physics.md §8) on the CPU backend, which is
//! also checked for determinism.

mod reference;

use glam::{Vec2, Vec3};
use r3d_sim::{CpuSolver, SimScene, Solver};
use reference::*;

fn cpu(scene: SimScene) -> Box<dyn Solver> {
    Box::new(CpuSolver::new(scene))
}

#[test]
fn hanging_cloth_sags_and_settles() {
    reference::hanging_cloth_sags_and_settles(&cpu);
}

#[test]
fn cloth_lands_on_the_ground() {
    reference::cloth_lands_on_the_ground(&cpu);
}

#[test]
fn cloth_drapes_over_a_sphere_without_going_through() {
    reference::cloth_drapes_over_a_sphere_without_going_through(&cpu);
}

#[test]
fn a_moving_body_carries_the_cloth() {
    reference::a_moving_body_carries_the_cloth(&cpu);
}

#[test]
fn two_panels_sewn_into_a_tube_close_their_seams() {
    reference::two_panels_sewn_into_a_tube_close_their_seams(&cpu);
}

#[test]
fn buttons_hold_two_bands_together() {
    reference::buttons_hold_two_bands_together(&cpu);
}

#[test]
fn jersey_stretches_more_along_the_weft() {
    reference::jersey_stretches_more_along_the_weft(&cpu);
}

#[test]
fn stacked_sheets_do_not_pass_through_each_other() {
    reference::stacked_sheets_do_not_pass_through_each_other(&cpu);
}

#[test]
fn grabbed_cloth_follows_the_hand_and_falls_when_released() {
    reference::grabbed_cloth_follows_the_hand_and_falls_when_released(&cpu);
}

#[test]
fn simulation_is_deterministic() {
    let make = || {
        let mut scene = sheet(0.3, 0.3, 11, Vec2::Y, |r| Vec3::new(r.x, 0.4, r.y));
        scene.ground = true;
        scene
    };
    let a = run(&cpu, make(), 0.5);
    let b = run(&cpu, make(), 0.5);
    assert_eq!(a.positions(), b.positions());
}

#[test]
fn results_do_not_depend_on_the_thread_count() {
    let make = || {
        let mut scene = sheet(0.4, 0.4, 25, Vec2::Y, |r| Vec3::new(r.x, 0.6, r.y));
        scene.collider = Some(sphere(Vec3::new(0.2, 0.4, 0.2), 0.15));
        scene.ground = true;
        scene
    };
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| run(&cpu, make(), 0.5));
    let many = run(&cpu, make(), 0.5);
    assert_eq!(one.positions(), many.positions());
}

#[test]
fn an_elastic_gathers_the_edge_it_is_sewn_on() {
    reference::an_elastic_gathers_the_edge_it_is_sewn_on(&cpu);
}
