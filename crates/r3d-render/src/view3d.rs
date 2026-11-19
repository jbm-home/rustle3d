//! The 3D view: backdrop, ground and surfaces (avatar, cloth).

use bytemuck::{Pod, Zeroable};

use crate::camera::OrbitCamera;
use crate::gpu::Gpu;
use crate::mesh::Mesh;
use crate::print::{PrintCache, Printed};
use crate::targets::{Attachments, DEPTH_FORMAT};
use r3d_core::Fabric;

use crate::theme::{Rgba, Theme, hex, parse_hex};
use crate::{MSAA_SAMPLES, Target};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    sky_top: [f32; 4],
    sky_bottom: [f32; 4],
    ground: [f32; 4],
}

/// How a surface looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub base: Rgba,
    /// Colour of the grazing-angle sheen of fibres.
    pub sheen: Rgba,
    /// Strength of the sheen (0 = matte).
    pub sheen_amount: f32,
}

impl Material {
    /// A fabric's usual look, or `color` (`#RRGGBB`) when given. The sheen
    /// is the fibres' colour at grazing angles: strong for silk, a soft halo
    /// for wool, faint for denim.
    pub fn fabric(fabric: Fabric, color: Option<&str>) -> Self {
        let look = fabric.look();
        let sheen_amount = look.sheen;
        let base = color.and_then(parse_hex).unwrap_or_else(|| hex(look.rgb));
        let lighter = |c: f32| c + (1.0 - c) * 0.6;
        Self {
            base,
            sheen: [lighter(base[0]), lighter(base[1]), lighter(base[2]), 1.0],
            sheen_amount,
        }
    }

    /// The same material with a print drawn over it: the print's colours
    /// replace the base (left white, so a tint still shows), the sheen
    /// takes the print's average colour `mean` (sRGB `0xRRGGBB`).
    pub fn printed(self, mean: u32) -> Self {
        let mean = hex(mean);
        let lighter = |c: f32| c + (1.0 - c) * 0.6;
        Self {
            base: [1.0; 4],
            sheen: [lighter(mean[0]), lighter(mean[1]), lighter(mean[2]), 1.0],
            ..self
        }
    }

    /// The same material, tinted toward `tint` (selection highlight).
    pub fn tinted(self, tint: Rgba, amount: f32) -> Self {
        let mix = |a: Rgba| -> Rgba { std::array::from_fn(|i| a[i] + (tint[i] - a[i]) * amount) };
        Self {
            base: mix(self.base),
            sheen: mix(self.sheen),
            ..self
        }
    }

    /// The avatar: a matte dress form.
    pub fn mannequin(theme: &Theme) -> Self {
        Self {
            base: theme.avatar,
            sheen: theme.avatar,
            sheen_amount: 0.2,
        }
    }
}

/// One mesh to draw with its material, and its print if it has one (the
/// mesh then carries texture coordinates).
#[derive(Debug, Clone, Copy)]
pub struct Draw<'a> {
    pub mesh: &'a Mesh,
    pub material: Material,
    pub print: Option<Printed<'a>>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    base: [f32; 4],
    sheen: [f32; 4],
    params: [f32; 4],
}

/// Materials live in one buffer, one 256-byte slot per draw (dynamic
/// offsets must be aligned to this).
const MATERIAL_STRIDE: u64 = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
    /// The mesh's own colour (sRGB), used when the material says so.
    color: [f32; 4],
    /// Texture coordinates on the print.
    uv: [f32; 2],
}

/// A GPU buffer that grows when needed.
struct Growable {
    buffer: wgpu::Buffer,
    capacity: u64,
}

impl Growable {
    fn write(slot: &mut Option<Growable>, gpu: &Gpu, usage: wgpu::BufferUsages, bytes: &[u8]) {
        let needed = bytes.len() as u64;
        if slot.as_ref().is_none_or(|g| g.capacity < needed) {
            let capacity = needed.next_power_of_two().max(256);
            *slot = Some(Growable {
                buffer: gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("mesh"),
                    size: capacity,
                    usage: usage | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                capacity,
            });
        }
        if let Some(g) = slot {
            gpu.queue.write_buffer(&g.buffer, 0, bytes);
        }
    }
}

/// Renders the 3D view into a texture of the format given at creation.
pub struct ViewRenderer3d {
    format: wgpu::TextureFormat,
    backdrop: wgpu::RenderPipeline,
    ground: wgpu::RenderPipeline,
    surface: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    material_layout: wgpu::BindGroupLayout,
    prints: PrintCache,
    /// Material buffer and its bind group, grown to the number of draws.
    materials: Option<(Growable, wgpu::BindGroup)>,
    vertices: Option<Growable>,
    indices: Option<Growable>,
    attachments: Option<Attachments>,
}

impl std::fmt::Debug for ViewRenderer3d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewRenderer3d")
            .field("format", &self.format)
            .finish()
    }
}

fn depth(write: bool, compare: wgpu::CompareFunction) -> Option<wgpu::DepthStencilState> {
    Some(wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    })
}

impl ViewRenderer3d {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/view3d.wgsl"));
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d globals"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d material"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<MaterialUniform>() as u64
                    ),
                },
                count: None,
            }],
        });
        // The backdrop and the ground only need the globals; surfaces also
        // take a material.
        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d scene"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let prints = PrintCache::new(gpu, wgpu::ShaderStages::FRAGMENT);
        let surface_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d surface"),
            bind_group_layouts: &[Some(&layout), Some(&material_layout), Some(prints.layout())],
            immediate_size: 0,
        });
        let pipeline = |label,
                        layout: &wgpu::PipelineLayout,
                        vs,
                        fs,
                        buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
                        depth_state,
                        blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: depth_state,
                multisample: wgpu::MultisampleState {
                    count: MSAA_SAMPLES,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2
            ],
        };
        Self {
            format,
            backdrop: pipeline(
                "backdrop",
                &scene_layout,
                "vs_backdrop",
                "fs_backdrop",
                &[],
                depth(false, wgpu::CompareFunction::Always),
                None,
            ),
            ground: pipeline(
                "ground",
                &scene_layout,
                "vs_ground",
                "fs_ground",
                &[],
                depth(false, wgpu::CompareFunction::Less),
                Some(wgpu::BlendState::ALPHA_BLENDING),
            ),
            surface: pipeline(
                "surface",
                &surface_layout,
                "vs_surface",
                "fs_surface",
                &[Some(vertex_layout)],
                depth(true, wgpu::CompareFunction::Less),
                None,
            ),
            globals,
            bind_group,
            material_layout,
            prints,
            materials: None,
            vertices: None,
            indices: None,
            attachments: None,
        }
    }

    /// Draw the scene (backdrop, ground, then `draws`) seen by `camera`.
    pub fn render(
        &mut self,
        gpu: &Gpu,
        target: Target<'_>,
        camera: &OrbitCamera,
        draws: &[Draw<'_>],
        theme: &Theme,
    ) {
        let Target {
            view: target,
            width,
            height,
        } = target;
        let eye = camera.eye();
        let globals = Globals {
            view_proj: camera
                .view_proj(width as f32 / height.max(1) as f32)
                .to_cols_array_2d(),
            eye: [eye.x, eye.y, eye.z, 1.0],
            sky_top: theme.sky_top,
            sky_bottom: theme.sky_bottom,
            ground: theme.ground,
        };
        gpu.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        // Pack every mesh into one vertex and one index buffer.
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut ranges = Vec::with_capacity(draws.len());
        let mut material_bytes = vec![0u8; draws.len() * MATERIAL_STRIDE as usize];
        let mut prints = Vec::with_capacity(draws.len());
        for (i, d) in draws.iter().enumerate() {
            let first_index = indices.len() as u32;
            let base_vertex = vertices.len() as i32;
            let painted = d.mesh.colors.len() == d.mesh.positions.len();
            let print = d
                .print
                .filter(|_| !painted && d.mesh.uvs.len() == d.mesh.positions.len());
            if let Some(p) = print {
                self.prints.prepare(gpu, p);
            }
            prints.push(print.map(|p| p.key));
            vertices.extend(
                d.mesh
                    .positions
                    .iter()
                    .zip(&d.mesh.normals)
                    .enumerate()
                    .map(|(k, (p, n))| Vertex {
                        pos: p.to_array(),
                        normal: n.to_array(),
                        color: d
                            .mesh
                            .colors
                            .get(k)
                            .copied()
                            .filter(|_| painted)
                            .unwrap_or_default(),
                        uv: d.mesh.uvs.get(k).copied().unwrap_or_default(),
                    }),
            );
            indices.extend_from_slice(&d.mesh.indices);
            ranges.push((first_index..indices.len() as u32, base_vertex));
            let uniform = MaterialUniform {
                base: d.material.base,
                sheen: d.material.sheen,
                params: [
                    d.material.sheen_amount,
                    if painted { 1.0 } else { 0.0 },
                    if print.is_some() { 1.0 } else { 0.0 },
                    0.0,
                ],
            };
            let at = i * MATERIAL_STRIDE as usize;
            material_bytes[at..at + std::mem::size_of::<MaterialUniform>()]
                .copy_from_slice(bytemuck::bytes_of(&uniform));
        }
        if !indices.is_empty() {
            Growable::write(
                &mut self.vertices,
                gpu,
                wgpu::BufferUsages::VERTEX,
                bytemuck::cast_slice(&vertices),
            );
            Growable::write(
                &mut self.indices,
                gpu,
                wgpu::BufferUsages::INDEX,
                bytemuck::cast_slice(&indices),
            );
            self.write_materials(gpu, &material_bytes);
        }

        let att = Attachments::get(
            &mut self.attachments,
            &gpu.device,
            self.format,
            width,
            height,
        );
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("3d") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &att.msaa,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &att.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_pipeline(&self.backdrop);
            pass.draw(0..3, 0..1);
            pass.set_pipeline(&self.ground);
            pass.draw(0..6, 0..1);
            if let (false, Some(v), Some(i), Some((_, materials))) = (
                indices.is_empty(),
                &self.vertices,
                &self.indices,
                &self.materials,
            ) {
                pass.set_pipeline(&self.surface);
                pass.set_vertex_buffer(0, v.buffer.slice(..));
                pass.set_index_buffer(i.buffer.slice(..), wgpu::IndexFormat::Uint32);
                for (n, (range, base_vertex)) in ranges.into_iter().enumerate() {
                    pass.set_bind_group(1, materials, &[(n as u64 * MATERIAL_STRIDE) as u32]);
                    let print = prints.get(n).copied().flatten();
                    pass.set_bind_group(2, self.prints.get(print), &[]);
                    pass.draw_indexed(range, base_vertex, 0..1);
                }
            }
        }
        gpu.queue.submit([encoder.finish()]);
        let used: Vec<u64> = prints.into_iter().flatten().collect();
        self.prints.retain(&used);
    }

    fn write_materials(&mut self, gpu: &Gpu, bytes: &[u8]) {
        let grows = self
            .materials
            .as_ref()
            .is_none_or(|(g, _)| g.capacity < bytes.len() as u64);
        if grows {
            let mut slot = None;
            Growable::write(&mut slot, gpu, wgpu::BufferUsages::UNIFORM, bytes);
            if let Some(buffer) = slot {
                let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("3d materials"),
                    layout: &self.material_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer.buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(
                                std::mem::size_of::<MaterialUniform>() as u64
                            ),
                        }),
                    }],
                });
                self.materials = Some((buffer, bind_group));
            }
        } else if let Some((g, _)) = &self.materials {
            gpu.queue.write_buffer(&g.buffer, 0, bytes);
        }
    }
}
