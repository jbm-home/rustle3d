//! The 2D pattern board: grid, filled panels, outlines, notches, internal
//! lines and selection, in board millimetres (y up).

use bytemuck::{Pod, Zeroable};
use glam::camera::rh::proj::directx;
use r3d_core::{PanelId, Project};
use r3d_geom::{Aabb, DVec2, polygon};
use wgpu::util::DeviceExt;

use crate::gpu::Gpu;
use crate::targets::Attachments;
use crate::theme::{Rgba, Theme};
use crate::{MSAA_SAMPLES, Target};

const NOTCH_MM: f64 = 6.0;
const MINOR_GRID_MM: f64 = 10.0;
const MAJOR_GRID_MM: f64 = 100.0;

/// What the board shows for one panel, extracted from the document.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardPanel {
    pub id: PanelId,
    /// Flattened contour, counter-clockwise.
    pub outline: Vec<DVec2>,
    pub triangles: Vec<[usize; 3]>,
    pub internal_lines: Vec<Vec<DVec2>>,
    /// Notch position and outward normal.
    pub notches: Vec<(DVec2, DVec2)>,
}

/// Everything the board draws.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoardScene {
    pub panels: Vec<BoardPanel>,
    pub selected: Vec<PanelId>,
}

impl BoardScene {
    pub fn from_project(project: &Project, selected: &[PanelId]) -> Self {
        let panels = project
            .panels()
            .iter()
            .filter_map(|(id, panel)| {
                let outline = project.panel_outline(*id)?;
                let notches = project
                    .notches()
                    .iter()
                    .filter(|(_, n)| panel.contour.contains(&n.edge))
                    .filter_map(|(nid, _)| project.notch_placement(*nid))
                    .map(|at| (at.pos, DVec2::new(at.tangent.y, -at.tangent.x)))
                    .collect();
                let internal_lines = project
                    .internal_lines()
                    .iter()
                    .filter(|(_, l)| l.panel == *id)
                    .filter_map(|(lid, _)| project.internal_line_polyline(*lid))
                    .collect();
                Some(BoardPanel {
                    id: *id,
                    triangles: polygon::triangulate(&outline),
                    outline,
                    internal_lines,
                    notches,
                })
            })
            .collect();
        Self {
            panels,
            selected: selected.to_vec(),
        }
    }

    pub fn bounds(&self) -> Option<Aabb> {
        let all: Vec<DVec2> = self
            .panels
            .iter()
            .flat_map(|p| p.outline.iter().copied())
            .collect();
        polygon::bounding_box(&all)
    }

    /// The topmost panel under a board point.
    pub fn panel_at(&self, p: DVec2) -> Option<PanelId> {
        self.panels
            .iter()
            .rev()
            .find(|panel| polygon::contains(&panel.outline, p))
            .map(|panel| panel.id)
    }
}

/// Which part of the board is visible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardView {
    /// Board point at the centre of the target (mm).
    pub center: DVec2,
    /// Zoom: millimetres per pixel.
    pub mm_per_px: f64,
}

impl BoardView {
    /// Fit `bounds` in a `width × height` target with a margin.
    pub fn fit(bounds: Option<Aabb>, width: u32, height: u32) -> Self {
        let Some(b) = bounds else {
            return Self {
                center: DVec2::ZERO,
                mm_per_px: 1.0,
            };
        };
        let margin_px = 48.0;
        let size = (b.max - b.min).max(DVec2::splat(1.0));
        let avail = DVec2::new(
            (width as f64 - 2.0 * margin_px).max(1.0),
            (height as f64 - 2.0 * margin_px).max(1.0),
        );
        Self {
            center: (b.min + b.max) * 0.5,
            mm_per_px: (size.x / avail.x).max(size.y / avail.y),
        }
    }

    /// Board point under a pixel (origin top-left, y down).
    pub fn to_board(&self, x: f64, y: f64, width: u32, height: u32) -> DVec2 {
        self.center + DVec2::new(x - width as f64 / 2.0, height as f64 / 2.0 - y) * self.mm_per_px
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

/// Triangles accumulated for one frame.
#[derive(Default)]
struct Batch {
    vertices: Vec<Vertex>,
}

impl Batch {
    fn tri(&mut self, a: DVec2, b: DVec2, c: DVec2, color: Rgba) {
        for p in [a, b, c] {
            self.vertices.push(Vertex {
                pos: [p.x as f32, p.y as f32],
                color,
            });
        }
    }

    /// A straight stroke of `width` mm with square caps.
    fn segment(&mut self, a: DVec2, b: DVec2, width: f64, color: Rgba) {
        let Some(d) = (b - a).try_normalize() else {
            return;
        };
        let h = width / 2.0;
        let (n, e) = (d.perp() * h, d * h);
        let (p0, p1, p2, p3) = (a - e + n, a - e - n, b + e - n, b + e + n);
        self.tri(p0, p1, p2, color);
        self.tri(p0, p2, p3, color);
    }

    fn polyline(&mut self, pts: &[DVec2], closed: bool, width: f64, color: Rgba) {
        for w in pts.windows(2) {
            self.segment(w[0], w[1], width, color);
        }
        if let (true, Some(first), Some(last)) = (closed, pts.first(), pts.last()) {
            self.segment(*last, *first, width, color);
        }
    }
}

fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// Renders the board into a texture of the format given at creation.
pub struct BoardRenderer {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    attachments: Option<Attachments>,
}

impl std::fmt::Debug for BoardRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoardRenderer")
            .field("format", &self.format)
            .finish()
    }
}

impl BoardRenderer {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/board.wgsl"));
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("board uniforms"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("board"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("board"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("board"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("board"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLES,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            format,
            pipeline,
            uniforms,
            bind_group,
            attachments: None,
        }
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        target: Target<'_>,
        scene: &BoardScene,
        view: &BoardView,
        theme: &Theme,
    ) {
        let Target {
            view: target,
            width,
            height,
        } = target;
        let px = view.mm_per_px;
        let half = DVec2::new(width as f64, height as f64) * 0.5 * px;
        let (lo, hi) = (view.center - half, view.center + half);
        let mut batch = Batch::default();

        // Grid: minor lines only when they are far enough apart to read.
        for (step, color) in [
            (MINOR_GRID_MM, theme.grid_minor),
            (MAJOR_GRID_MM, theme.grid_major),
        ] {
            if step / px < 6.0 {
                continue;
            }
            let mut x = (lo.x / step).floor() * step;
            while x <= hi.x {
                batch.segment(DVec2::new(x, lo.y), DVec2::new(x, hi.y), px, color);
                x += step;
            }
            let mut y = (lo.y / step).floor() * step;
            while y <= hi.y {
                batch.segment(DVec2::new(lo.x, y), DVec2::new(hi.x, y), px, color);
                y += step;
            }
        }

        for panel in &scene.panels {
            let selected = scene.selected.contains(&panel.id);
            let fill = if selected {
                mix(theme.panel_fill, theme.selection, 0.10)
            } else {
                theme.panel_fill
            };
            for t in &panel.triangles {
                batch.tri(
                    panel.outline[t[0]],
                    panel.outline[t[1]],
                    panel.outline[t[2]],
                    fill,
                );
            }
            let mut faint = theme.panel_line;
            faint[3] *= 0.45;
            for line in &panel.internal_lines {
                batch.polyline(line, false, px, faint);
            }
            let (line_color, width) = if selected {
                (theme.selection, 2.5 * px)
            } else {
                (theme.panel_line, 1.5 * px)
            };
            batch.polyline(&panel.outline, true, width, line_color);
            for (pos, outward) in &panel.notches {
                batch.segment(*pos, *pos - *outward * NOTCH_MM, width, line_color);
            }
        }

        let transform = directx::orthographic(
            lo.x as f32,
            hi.x as f32,
            lo.y as f32,
            hi.y as f32,
            -1.0,
            1.0,
        );
        gpu.queue.write_buffer(
            &self.uniforms,
            0,
            bytemuck::cast_slice(&transform.to_cols_array()),
        );
        let vertex_buffer = (!batch.vertices.is_empty()).then(|| {
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("board vertices"),
                    contents: bytemuck::cast_slice(&batch.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });

        let att = Attachments::get(
            &mut self.attachments,
            &gpu.device,
            self.format,
            width,
            height,
        );
        let bg = theme.board_background;
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("board"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("board"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &att.msaa,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0] as f64,
                            g: bg[1] as f64,
                            b: bg[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(vb) = &vertex_buffer {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..batch.vertices.len() as u32, 0..1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
    }
}
