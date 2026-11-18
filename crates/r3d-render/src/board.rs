//! The 2D pattern board: grid, filled panels, outlines, notches, internal
//! lines and selection, in board millimetres (y up).

use bytemuck::{Pod, Zeroable};
use glam::camera::rh::proj::directx;
use r3d_core::{EdgeId, PanelId, Project, SeamId};
use r3d_geom::{Aabb, DVec2, polygon};
use wgpu::util::DeviceExt;

use crate::gpu::Gpu;
use crate::targets::Attachments;
use crate::theme::{PIECE_COLORS, Rgba, SEAM_COLORS, Theme, WARNING};
use crate::{MSAA_SAMPLES, Target};

const NOTCH_MM: f64 = 6.0;
const MINOR_GRID_MM: f64 = 10.0;
const MAJOR_GRID_MM: f64 = 100.0;

/// What the board shows for one panel, extracted from the document.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardPanel {
    pub id: PanelId,
    /// Index into [`Theme::pieces`].
    pub color: usize,
    /// Flattened contour, counter-clockwise.
    pub outline: Vec<DVec2>,
    pub triangles: Vec<[usize; 3]>,
    pub internal_lines: Vec<Vec<DVec2>>,
    /// Notch position and outward normal.
    pub notches: Vec<(DVec2, DVec2)>,
    /// The panel's outline in the other sizes of the project's range (the
    /// graded nest), and whether each is the size shown in 3D.
    pub sizes: Vec<(Vec<DVec2>, bool)>,
}

/// Everything the board draws.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoardScene {
    pub panels: Vec<BoardPanel>,
    pub selected: Vec<PanelId>,
    /// Every contour edge, for picking and drawing seams.
    pub edges: Vec<BoardEdge>,
    pub selected_edge: Option<EdgeId>,
}

/// A contour edge on the board.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardEdge {
    pub id: EdgeId,
    pub panel: PanelId,
    /// From its start to its end (mm).
    pub points: Vec<DVec2>,
    /// Its seam, if sewn.
    pub seam: Option<BoardSeam>,
}

/// How an edge is sewn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardSeam {
    /// Index of the seam (both sides share one colour).
    pub index: usize,
    /// The seam walks this edge from its end to its start.
    pub reversed: bool,
    /// The two sides differ in length by more than [`SEAM_TOLERANCE_MM`].
    pub mismatch: bool,
}

/// Seam sides whose lengths differ by more than this are flagged (mm).
pub const SEAM_TOLERANCE_MM: f64 = 2.0;

impl BoardScene {
    pub fn from_project(project: &Project, selected: &[PanelId]) -> Self {
        let order: Vec<PanelId> = project.panels().keys().copied().collect();
        let color_of = |id: PanelId, panel: &r3d_core::Panel| {
            let owner = panel.mirror_of.as_ref().map_or(id, |l| l.source);
            order.iter().position(|p| *p == owner).unwrap_or_default() % PIECE_COLORS
        };
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
                    color: color_of(*id, panel),
                    triangles: polygon::triangulate(&outline),
                    outline,
                    internal_lines,
                    notches,
                    sizes: Vec::new(),
                })
            })
            .collect();
        let seams: Vec<SeamId> = project.seams().keys().copied().collect();
        let edges = project
            .panels()
            .iter()
            .flat_map(|(id, panel)| panel.contour.iter().map(move |e| (*id, *e)))
            .filter_map(|(panel, e)| {
                Some(BoardEdge {
                    id: e,
                    panel,
                    points: project.edge_polyline(e)?,
                    seam: project.seam_of(e).and_then(|s| {
                        let seam = project.seams().get(&s)?;
                        let side = if seam.a.edge == e { seam.a } else { seam.b };
                        let length = |x: EdgeId| project.edge_length(x).unwrap_or_default();
                        Some(BoardSeam {
                            index: seams.iter().position(|x| *x == s)?,
                            reversed: side.reversed,
                            // A gathered seam's sides differ on purpose.
                            mismatch: !seam.gathered
                                && (length(seam.a.edge) - length(seam.b.edge)).abs()
                                    > SEAM_TOLERANCE_MM,
                        })
                    }),
                })
            })
            .collect();
        Self {
            panels,
            selected: selected.to_vec(),
            edges,
            selected_edge: None,
        }
    }

    /// The same scene with the graded nest: each piece's outline in the
    /// other sizes of `project`'s range.
    pub fn with_sizes(mut self, project: &Project) -> Self {
        let Some(g) = project.grading() else {
            return self;
        };
        for k in (0..g.sizes.len()).filter(|k| *k != g.base) {
            let sized = project.at_size(k);
            for panel in &mut self.panels {
                if let Some(outline) = sized.panel_outline(panel.id) {
                    panel.sizes.push((outline, k == g.shown));
                }
            }
        }
        self
    }

    /// The same scene with `edge` highlighted.
    pub fn with_selected_edge(mut self, edge: Option<EdgeId>) -> Self {
        self.selected_edge = edge;
        self
    }

    /// The edge nearest to a board point, within `max_mm`.
    pub fn edge_at(&self, p: DVec2, max_mm: f64) -> Option<&BoardEdge> {
        self.edges
            .iter()
            .filter_map(|e| {
                let d = e
                    .points
                    .windows(2)
                    .map(|w| {
                        let ab = w[1] - w[0];
                        let t = if ab.length_squared() > 0.0 {
                            ((p - w[0]).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        p.distance(w[0] + ab * t)
                    })
                    .fold(f64::MAX, f64::min);
                (d <= max_mm).then_some((d, e))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, e)| e)
    }

    pub fn bounds(&self) -> Option<Aabb> {
        let all: Vec<DVec2> = self
            .panels
            .iter()
            .flat_map(|p| {
                p.outline
                    .iter()
                    .chain(p.sizes.iter().flat_map(|(o, _)| o.iter()))
                    .copied()
            })
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

    /// The pixel showing board point `p` (mm): the inverse of
    /// [`Self::to_board`].
    pub fn to_pixels(&self, p: DVec2, width: u32, height: u32) -> DVec2 {
        let d = (p - self.center) / self.mm_per_px;
        DVec2::new(width as f64 / 2.0 + d.x, height as f64 / 2.0 - d.y)
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
    /// A filled disc.
    fn dot(&mut self, at: DVec2, radius: f64, color: Rgba) {
        const SIDES: usize = 16;
        let corner = |k: usize| {
            let a = std::f64::consts::TAU * k as f64 / SIDES as f64;
            at + DVec2::new(a.cos(), a.sin()) * radius
        };
        for k in 0..SIDES {
            self.tri(at, corner(k), corner(k + 1), color);
        }
    }

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

/// The point halfway along a polyline and the unit direction there.
fn midpoint(points: &[DVec2]) -> Option<(DVec2, DVec2)> {
    let total: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total / 2.0;
    for w in points.windows(2) {
        let len = w[0].distance(w[1]);
        if len > 0.0 && left <= len {
            let dir = (w[1] - w[0]) / len;
            return Some((w[0] + dir * left, dir));
        }
        left -= len;
    }
    None
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
            let base = theme.pieces[panel.color % PIECE_COLORS];
            let fill = if selected {
                mix(base, theme.selection, 0.18)
            } else {
                base
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
            // The graded nest: the other sizes' outlines, the one shown in
            // 3D stronger.
            for (outline, shown) in &panel.sizes {
                let (color, width) = if *shown {
                    (theme.panel_line, 1.5 * px)
                } else {
                    (faint, px)
                };
                batch.polyline(outline, true, width, color);
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
        // Seams: both sides in the same colour, over the outlines, with a
        // chevron at the middle pointing the way the seam walks the edge
        // (matching sides point alike when the pieces lie side by side).
        // Sides of different lengths are red, with a dot.
        for edge in &scene.edges {
            let Some(seam) = edge.seam else { continue };
            let color = if seam.mismatch {
                WARNING
            } else {
                SEAM_COLORS[seam.index % SEAM_COLORS.len()]
            };
            batch.polyline(&edge.points, false, 2.5 * px, color);
            if let Some((mid, dir)) = midpoint(&edge.points) {
                let dir = if seam.reversed { -dir } else { dir };
                let side = DVec2::new(-dir.y, dir.x);
                let size = 7.0 * px;
                let tip = mid + dir * size * 0.5;
                for wing in [side, -side] {
                    batch.segment(tip, tip - dir * size + wing * size * 0.7, 2.0 * px, color);
                }
                if seam.mismatch {
                    batch.dot(mid + side * 9.0 * px, 4.5 * px, WARNING);
                }
            }
        }
        if let Some(edge) = scene
            .edges
            .iter()
            .find(|e| Some(e.id) == scene.selected_edge)
        {
            batch.polyline(&edge.points, false, 4.5 * px, theme.selection);
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
