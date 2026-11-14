//! The 3D view: backdrop, ground and cloth.

use bytemuck::{Pod, Zeroable};

use crate::camera::OrbitCamera;
use crate::gpu::Gpu;
use crate::mesh::ClothMesh;
use crate::targets::{Attachments, DEPTH_FORMAT};
use crate::theme::Theme;
use crate::{MSAA_SAMPLES, Target};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    cloth: [f32; 4],
    sheen: [f32; 4],
    sky_top: [f32; 4],
    sky_bottom: [f32; 4],
    ground: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
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
    cloth: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label,
                        vs,
                        fs,
                        buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
                        depth_state,
                        blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
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
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        };
        Self {
            format,
            backdrop: pipeline(
                "backdrop",
                "vs_backdrop",
                "fs_backdrop",
                &[],
                depth(false, wgpu::CompareFunction::Always),
                None,
            ),
            ground: pipeline(
                "ground",
                "vs_ground",
                "fs_ground",
                &[],
                depth(false, wgpu::CompareFunction::Less),
                Some(wgpu::BlendState::ALPHA_BLENDING),
            ),
            cloth: pipeline(
                "cloth",
                "vs_cloth",
                "fs_cloth",
                &[Some(vertex_layout)],
                depth(true, wgpu::CompareFunction::Less),
                None,
            ),
            globals,
            bind_group,
            vertices: None,
            indices: None,
            attachments: None,
        }
    }

    /// Draw `mesh` seen by `camera` into `target`.
    pub fn render(
        &mut self,
        gpu: &Gpu,
        target: Target<'_>,
        camera: &OrbitCamera,
        mesh: &ClothMesh,
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
            cloth: theme.cloth,
            sheen: theme.cloth_sheen,
            sky_top: theme.sky_top,
            sky_bottom: theme.sky_bottom,
            ground: theme.ground,
        };
        gpu.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
        let vertices: Vec<Vertex> = mesh
            .positions
            .iter()
            .zip(&mesh.normals)
            .map(|(p, n)| Vertex {
                pos: p.to_array(),
                normal: n.to_array(),
            })
            .collect();
        let index_count = mesh.indices.len() as u32;
        if index_count > 0 {
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
                bytemuck::cast_slice(&mesh.indices),
            );
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
            if let (true, Some(v), Some(i)) = (index_count > 0, &self.vertices, &self.indices) {
                pass.set_pipeline(&self.cloth);
                pass.set_vertex_buffer(0, v.buffer.slice(..));
                pass.set_index_buffer(i.buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
    }
}
