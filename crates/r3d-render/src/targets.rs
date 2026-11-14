//! Multisampled colour and depth attachments, recreated when the size
//! changes.

use crate::MSAA_SAMPLES;

pub(crate) struct Attachments {
    size: (u32, u32),
    pub msaa: wgpu::TextureView,
    pub depth: wgpu::TextureView,
}

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

impl Attachments {
    /// The attachments for `width × height`, created on first use and when
    /// the size changes.
    pub fn get<'a>(
        slot: &'a mut Option<Attachments>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> &'a Attachments {
        let size = (width.max(1), height.max(1));
        if slot.as_ref().is_some_and(|a| a.size != size) {
            *slot = None;
        }
        slot.get_or_insert_with(|| {
            let make = |format, label| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d {
                            width: size.0,
                            height: size.1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: MSAA_SAMPLES,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            };
            Attachments {
                size,
                msaa: make(format, "msaa colour"),
                depth: make(DEPTH_FORMAT, "depth"),
            }
        })
    }
}
