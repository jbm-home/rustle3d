//! Garments' prints on the GPU (spec 25): textures uploaded once with
//! their mip-map pyramid (Williams 1983, built on the CPU in linear light
//! by [`r3d_core::Texture::halved`]) and kept while they are drawn.

use std::collections::HashMap;

use r3d_core::Texture;

use crate::gpu::Gpu;

/// A print to draw: its pixels and a key that changes whenever they do.
#[derive(Debug, Clone, Copy)]
pub struct Printed<'a> {
    pub key: u64,
    pub texture: &'a Texture,
}

/// The repeating, trilinear sampler prints are read with.
pub(crate) fn sampler(gpu: &Gpu) -> wgpu::Sampler {
    gpu.device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("print"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: 8,
        ..Default::default()
    })
}

/// `texture` on the GPU with every level of its pyramid, as stored
/// (sRGB-encoded bytes in a `Rgba8Unorm` texture; shaders decode them,
/// as they do the materials' colours).
pub(crate) fn upload(gpu: &Gpu, texture: &Texture) -> wgpu::TextureView {
    let mut levels = vec![texture.clone()];
    while let Some(last) = levels.last() {
        if last.width <= 1 && last.height <= 1 {
            break;
        }
        let next = last.halved();
        levels.push(next);
    }
    let size = wgpu::Extent3d {
        width: texture.width.max(1),
        height: texture.height.max(1),
        depth_or_array_layers: 1,
    };
    let gpu_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("print"),
        size,
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, t) in levels.iter().enumerate() {
        if t.rgba.len() != (t.width * t.height * 4) as usize {
            continue;
        }
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &t.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * t.width),
                rows_per_image: Some(t.height),
            },
            wgpu::Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
        );
    }
    gpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Bind groups of uploaded prints (texture + sampler), by key; a white
/// one for surfaces without a print. Prints not drawn in a frame are
/// dropped at the next [`PrintCache::retain`].
pub(crate) struct PrintCache {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    plain: wgpu::BindGroup,
    groups: HashMap<u64, wgpu::BindGroup>,
}

impl PrintCache {
    pub(crate) fn new(gpu: &Gpu, visibility: wgpu::ShaderStages) -> Self {
        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("print"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sampler = sampler(gpu);
        let white = Texture {
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        };
        let plain = Self::group(gpu, &layout, &sampler, &upload(gpu, &white));
        Self {
            layout,
            sampler,
            plain,
            groups: HashMap::new(),
        }
    }

    fn group(
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("print"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Upload `print` unless it already is.
    pub(crate) fn prepare(&mut self, gpu: &Gpu, print: Printed<'_>) {
        if !self.groups.contains_key(&print.key) {
            let view = upload(gpu, print.texture);
            let group = Self::group(gpu, &self.layout, &self.sampler, &view);
            self.groups.insert(print.key, group);
        }
    }

    /// The bind group of `print` (prepared before), or the white one.
    pub(crate) fn get(&self, print: Option<u64>) -> &wgpu::BindGroup {
        print
            .and_then(|k| self.groups.get(&k))
            .unwrap_or(&self.plain)
    }

    /// Drop the prints whose keys are not in `used`.
    pub(crate) fn retain(&mut self, used: &[u64]) {
        self.groups.retain(|k, _| used.contains(k));
    }
}
