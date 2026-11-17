#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! The reference scenes (docs/physics.md §8) on the GPU backend: the same
//! checks as on the CPU. Machines without any GPU adapter skip them.

#[path = "../../r3d-sim/tests/reference/mod.rs"]
mod reference;

mod gpu;

use r3d_sim::{SimScene, Solver};
use r3d_sim_gpu::GpuSolver;

fn on_gpu(check: fn(reference::Make)) {
    let Some(gpu) = gpu::context() else {
        return;
    };
    let make =
        |scene: SimScene| -> Box<dyn Solver> { Box::new(GpuSolver::new(gpu, scene).unwrap()) };
    check(&make);
}

#[test]
fn hanging_cloth_sags_and_settles() {
    on_gpu(reference::hanging_cloth_sags_and_settles);
}

#[test]
fn cloth_lands_on_the_ground() {
    on_gpu(reference::cloth_lands_on_the_ground);
}

#[test]
fn cloth_drapes_over_a_sphere_without_going_through() {
    on_gpu(reference::cloth_drapes_over_a_sphere_without_going_through);
}

#[test]
fn two_panels_sewn_into_a_tube_close_their_seams() {
    on_gpu(reference::two_panels_sewn_into_a_tube_close_their_seams);
}

#[test]
fn jersey_stretches_more_along_the_weft() {
    on_gpu(reference::jersey_stretches_more_along_the_weft);
}

#[test]
fn stacked_sheets_do_not_pass_through_each_other() {
    on_gpu(reference::stacked_sheets_do_not_pass_through_each_other);
}

#[test]
fn grabbed_cloth_follows_the_hand_and_falls_when_released() {
    on_gpu(reference::grabbed_cloth_follows_the_hand_and_falls_when_released);
}

#[test]
fn simulation_is_deterministic() {
    let Some(gpu) = gpu::context() else {
        return;
    };
    let run = || {
        let mut scene = reference::sheet(0.4, 0.4, 25, glam::Vec2::Y, |r| {
            glam::Vec3::new(r.x, 0.6, r.y)
        });
        scene.collider = Some(reference::sphere(glam::Vec3::new(0.2, 0.4, 0.2), 0.15));
        scene.ground = true;
        let make =
            |scene: SimScene| -> Box<dyn Solver> { Box::new(GpuSolver::new(gpu, scene).unwrap()) };
        reference::run(&make, scene, 0.5).positions().to_vec()
    };
    assert_eq!(run(), run());
}
