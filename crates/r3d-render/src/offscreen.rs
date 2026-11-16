//! Offscreen rendering to PNG (CLI, MCP, tests).

use crate::gpu::{Gpu, RenderError};

/// Texture format for offscreen renders; create renderers with it.
pub const PNG_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Render with `draw` into a `width × height` texture and encode it as PNG.
pub fn render_png(
    gpu: &Gpu,
    width: u32,
    height: u32,
    draw: impl FnOnce(&wgpu::TextureView),
) -> Result<Vec<u8>, RenderError> {
    let (width, height) = (width.max(1), height.max(1));
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: PNG_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    draw(&texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let pixels = read_pixels(gpu, &texture)?;
    encode_png(width, height, &pixels)
}

/// Encode RGBA pixels (row after row, no padding) as PNG.
pub fn encode_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, RenderError> {
    let mut png_bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| RenderError::Png(e.to_string()))?;
    writer
        .write_image_data(pixels)
        .map_err(|e| RenderError::Png(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| RenderError::Png(e.to_string()))?;
    Ok(png_bytes)
}

/// The pixels of a 4-bytes-per-pixel `texture` (made with `COPY_SRC`),
/// row after row with no padding. Waits for the GPU.
pub fn read_pixels(gpu: &Gpu, texture: &wgpu::Texture) -> Result<Vec<u8>, RenderError> {
    let (width, height) = (texture.width(), texture.height());
    let row = width * 4;
    let padded =
        row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("readback"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);

    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| RenderError::Readback(e.to_string()))?;
    rx.recv()
        .map_err(|e| RenderError::Readback(e.to_string()))?
        .map_err(|e| RenderError::Readback(e.to_string()))?;

    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|e| RenderError::Readback(e.to_string()))?;
    let mut pixels = Vec::with_capacity((row * height) as usize);
    for r in 0..height as usize {
        let start = r * padded as usize;
        pixels.extend_from_slice(mapped.get(start..start + row as usize).unwrap_or_default());
    }
    drop(mapped);
    buffer.unmap();
    Ok(pixels)
}
