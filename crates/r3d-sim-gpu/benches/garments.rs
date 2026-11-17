//! Cost of a frame on the reference garments (docs/physics.md §7), on the
//! GPU and on the CPU: the T-shirt and the dress, on the standard women's
//! M body, after 1 s of draping. Run with `cargo bench -p r3d-sim-gpu`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // bench code

use criterion::{Criterion, criterion_group, criterion_main};
use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Session, Step};
use r3d_core::Project;
use r3d_sim::{CpuSolver, SimScene, Solver};
use r3d_sim_gpu::{GpuContext, GpuSolver};
use serde_json::json;

fn garment(template: &str) -> SimScene {
    let mut s = Session::new(Project::new("Untitled"));
    for step in [
        json!({"type": "avatar.set", "preset": "Women M", "body": {"masculinity": 0,
            "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}}),
        json!({"type": "template.apply", "template": template}),
    ] {
        s.run_step(&Step::from_value(step).unwrap()).unwrap();
    }
    let form = Form::of_project(s.project()).unwrap().unwrap();
    build(s.project(), Some(&form), DEFAULT_SPACING_MM)
        .unwrap()
        .scene
}

fn context() -> Option<GpuContext> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    GpuContext::new(&device, &queue).ok()
}

fn frame(c: &mut Criterion) {
    let gpu = context();
    let mut group = c.benchmark_group("frame");
    group.sample_size(30);
    for template in ["t_shirt", "dress"] {
        let scene = garment(template);
        let mut solvers: Vec<(&str, Box<dyn Solver>)> =
            vec![("cpu", Box::new(CpuSolver::new(scene.clone())))];
        if let Some(gpu) = &gpu {
            solvers.push(("gpu", Box::new(GpuSolver::new(gpu, scene.clone()).unwrap())));
        }
        for (backend, mut solver) in solvers {
            for _ in 0..60 {
                solver.step(1.0 / 60.0).unwrap();
            }
            group.bench_function(format!("{template} ({backend})"), |b| {
                b.iter(|| solver.step(1.0 / 60.0).unwrap())
            });
        }
    }
    group.finish();
}

criterion_group!(benches, frame);
criterion_main!(benches);
