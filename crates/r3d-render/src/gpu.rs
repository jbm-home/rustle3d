//! GPU device and queue.

/// Errors from GPU setup or readback.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no suitable GPU adapter: {0}")]
    Adapter(String),
    #[error("cannot create the GPU device: {0}")]
    Device(String),
    #[error("GPU readback failed: {0}")]
    Readback(String),
    #[error("cannot encode PNG: {0}")]
    Png(String),
}

/// A device and its queue, shared by every renderer.
#[derive(Debug, Clone)]
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// A device without any window (CLI, MCP, tests).
    pub fn headless() -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            ..Default::default()
        }))
        .map_err(|e| RenderError::Adapter(e.to_string()))?;
        Self::from_adapter(&adapter)
    }

    /// A device on a given adapter (e.g. the one the UI texture lives on).
    pub fn from_adapter(adapter: &wgpu::Adapter) -> Result<Self, RenderError> {
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rustle3d"),
            ..Default::default()
        }))
        .map_err(|e| RenderError::Device(e.to_string()))?;
        Ok(Self { device, queue })
    }
}
