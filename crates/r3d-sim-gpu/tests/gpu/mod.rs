//! One GPU device and solver context for the whole test process, never
//! dropped: devices made and dropped on worker threads can hang some
//! Windows drivers.

use std::sync::OnceLock;

use r3d_sim_gpu::GpuContext;

static CONTEXT: OnceLock<Option<GpuContext>> = OnceLock::new();

/// The test context, or `None` on machines without any GPU adapter.
pub fn context() -> Option<&'static GpuContext> {
    CONTEXT
        .get_or_init(|| {
            let instance =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    ..Default::default()
                }))
                .ok()?;
            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                    .ok()?;
            GpuContext::new(&device, &queue).ok()
        })
        .as_ref()
}
