//! The 2D pattern board: grid, filled panels, outlines, notches, internal
//! lines and selection, in board millimetres (y up).

use bytemuck::{Pod, Zeroable};
use glam::camera::rh::proj::directx;
use r3d_core::{EdgeId, PanelId, Project};
use r3d_geom::{Aabb, DVec2, polygon};
use wgpu::util::DeviceExt;

use crate::gpu::Gpu;
use crate::print::{PrintCache, Printed};
use crate::targets::Attachments;
use crate::theme::{PIECE_COLORS, Rgba, SEAM_COLORS, Theme, WARNING};
use crate::{MSAA_SAMPLES, Target};

const NOTCH_MM: f64 = 6.0;
/// Zippers on the board: a brass tone, readable on both themes.
const ZIPPER: Rgba = crate::theme::hex(0xB8862B);
/// Elastic: a muted teal zigzag, apart from seams and trims.
const ELASTIC: Rgba = crate::theme::hex(0x2F8F83);
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
    /// Flattened holes.
    pub holes: Vec<Vec<DVec2>>,
    /// The fill: points and triangles over them.
    pub fill: Vec<DVec2>,
    pub triangles: Vec<[u32; 3]>,
    pub internal_lines: Vec<Vec<DVec2>>,
    /// Notch position and outward normal.
    pub notches: Vec<(DVec2, DVec2)>,
    /// The panel's outline in the other sizes of the project's range (the
    /// graded nest), and whether each is the size shown in 3D.
    pub sizes: Vec<(Vec<DVec2>, bool)>,
    /// Its garment's print (index into [`BoardScene::prints`]) and where
    /// each fill point falls on it.
    pub print: Option<(usize, Vec<[f32; 2]>)>,
}

/// A garment's print for the board: its pixels, a key that changes with
/// them, and the image's height over width.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardPrint {
    pub garment: r3d_core::GarmentId,
    pub key: u64,
    pub texture: std::sync::Arc<r3d_core::Texture>,
    pub aspect: f64,
}

/// Everything the board draws.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoardScene {
    pub panels: Vec<BoardPanel>,
    pub selected: Vec<PanelId>,
    /// Every contour and hole edge, for picking.
    pub edges: Vec<BoardEdge>,
    /// Every seam side, to draw.
    pub seam_sides: Vec<BoardSeamSide>,
    /// Edges picked (one, or two to constrain together).
    pub selected_edges: Vec<EdgeId>,
    /// Construction lines: each the line through two points, drawn across
    /// the board.
    pub guides: Vec<[DVec2; 2]>,
    /// Symmetric pieces' axes, between the drawn half's two ends.
    pub axes: Vec<[DVec2; 2]>,
    /// The garments' prints the pieces are filled with.
    pub prints: Vec<BoardPrint>,
    pub buttons: Vec<BoardButton>,
    pub zippers: Vec<BoardZipper>,
    /// Where each elastic runs (mm).
    pub elastics: Vec<Vec<DVec2>>,
    pub topstitches: Vec<BoardTopstitch>,
}

/// A topstitch on the board: its rows (mm), its stitch length (mm), its
/// thread's colour (`0xRRGGBB`) and width (mm).
#[derive(Debug, Clone, PartialEq)]
pub struct BoardTopstitch {
    pub rows: Vec<Vec<DVec2>>,
    pub stitch_mm: f64,
    pub rgb: u32,
    pub width_mm: f64,
}

/// A button and its buttonhole on the board (mm).
#[derive(Debug, Clone, PartialEq)]
pub struct BoardButton {
    pub id: r3d_core::ButtonId,
    pub at: DVec2,
    pub diameter: f64,
    /// The buttonhole: where the button sits, then the slit's other end.
    pub slit: [DVec2; 2],
    pub closed: bool,
}

/// A zipper on the board: its two sides from the bottom stop (mm), and
/// the share closed from there (0–1).
#[derive(Debug, Clone, PartialEq)]
pub struct BoardZipper {
    pub id: r3d_core::ZipperId,
    pub sides: [Vec<DVec2>; 2],
    pub closed: f64,
}

/// A contour edge on the board.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardEdge {
    pub id: EdgeId,
    pub panel: PanelId,
    /// From its start to its end (mm).
    pub points: Vec<DVec2>,
}

/// One side of a seam on the board.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardSeamSide {
    /// Index of the seam (both sides share one colour).
    pub index: usize,
    /// Where it runs, in the direction the seam walks it (mm).
    pub points: Vec<DVec2>,
    /// The two sides differ in length by more than [`SEAM_TOLERANCE_MM`]
    /// beyond the seam's ease.
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
                let holes = project.hole_outlines(*id);
                let (fill, triangles) = r3d_geom::mesh::fill(&outline, &holes)?;
                Some(BoardPanel {
                    id: *id,
                    color: color_of(*id, panel),
                    holes,
                    fill,
                    triangles,
                    outline,
                    internal_lines,
                    notches,
                    sizes: Vec::new(),
                    print: None,
                })
            })
            .collect();
        let edges = project
            .panels()
            .iter()
            .flat_map(|(id, panel)| {
                panel
                    .contour
                    .iter()
                    .chain(panel.holes.iter().flatten())
                    .map(move |e| (*id, *e))
            })
            .filter_map(|(panel, e)| {
                Some(BoardEdge {
                    id: e,
                    panel,
                    points: project.edge_polyline(e)?,
                })
            })
            .collect();
        let seam_sides = project
            .seams()
            .values()
            .enumerate()
            .flat_map(|(index, seam)| {
                let length = |s| project.side_length(s).unwrap_or_default();
                // A gathered seam's sides differ on purpose.
                let mismatch = !seam.gathered
                    && (length(&seam.a) - length(&seam.b) - seam.ease_mm).abs() > SEAM_TOLERANCE_MM;
                [&seam.a, &seam.b]
                    .into_iter()
                    .filter_map(move |side| {
                        Some(BoardSeamSide {
                            index,
                            points: project.side_polyline(side)?,
                            mismatch,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let pos = |p| project.point(p).map(|p| p.pos);
        let axes = project
            .panels()
            .values()
            .filter_map(|panel| {
                let sym = panel.symmetry.as_ref()?;
                Some([pos(sym.start)?, pos(sym.end)?])
            })
            .collect();
        let buttons = project
            .buttons()
            .iter()
            .map(|(id, b)| BoardButton {
                id: *id,
                at: b.at,
                diameter: b.diameter_mm,
                slit: b.slit(),
                closed: b.closed,
            })
            .collect();
        let zippers = project
            .zippers()
            .iter()
            .filter_map(|(id, z)| {
                Some(BoardZipper {
                    id: *id,
                    sides: [project.side_polyline(&z.a)?, project.side_polyline(&z.b)?],
                    closed: project.zipper_closed(z),
                })
            })
            .collect();
        let elastics = project
            .elastics()
            .filter_map(|(_, e)| project.side_polyline(&e.side))
            .collect();
        let topstitches = project
            .topstitches()
            .map(|(_, t)| BoardTopstitch {
                rows: project.topstitch_rows(t),
                stitch_mm: t.stitch_mm,
                rgb: project.thread_rgb(t),
                width_mm: t.thread.width_mm(),
            })
            .collect();
        Self {
            panels,
            selected: selected.to_vec(),
            edges,
            seam_sides,
            selected_edges: Vec::new(),
            guides: project.guides().values().map(|g| [g.a, g.b]).collect(),
            axes,
            prints: Vec::new(),
            buttons,
            zippers,
            elastics,
            topstitches,
        }
    }

    /// The button whose button or buttonhole is within `max_mm` of `p`.
    pub fn button_at(&self, p: DVec2, max_mm: f64) -> Option<r3d_core::ButtonId> {
        self.buttons
            .iter()
            .map(|b| {
                let [s0, s1] = b.slit;
                let d = (b.at.distance(p) - b.diameter / 2.0)
                    .max(0.0)
                    .min(r3d_geom::fit::distance_to_segment(p, s0, s1));
                (d, b.id)
            })
            .filter(|(d, _)| *d <= max_mm)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }

    /// The zipper with a side within `max_mm` of `p`.
    pub fn zipper_at(&self, p: DVec2, max_mm: f64) -> Option<r3d_core::ZipperId> {
        self.zippers
            .iter()
            .map(|z| {
                let d = z
                    .sides
                    .iter()
                    .flat_map(|s| s.windows(2))
                    .filter_map(|w| match w {
                        [a, b] => Some(r3d_geom::fit::distance_to_segment(p, *a, *b)),
                        _ => None,
                    })
                    .fold(f64::MAX, f64::min);
                (d, z.id)
            })
            .filter(|(d, _)| *d <= max_mm)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
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

    /// The same scene with `edges` highlighted.
    /// Fill each piece with its garment's print, if `prints` has it
    /// (spec 25): along its grainline, at the print's real size.
    pub fn with_prints(mut self, project: &Project, prints: Vec<BoardPrint>) -> Self {
        for panel in &mut self.panels {
            let Some(garment) = project.panel(panel.id).map(|p| p.garment) else {
                continue;
            };
            let Some(at) = prints.iter().position(|p| p.garment == garment) else {
                continue;
            };
            let Some(frame) = prints
                .get(at)
                .and_then(|p| project.print_frame(panel.id, p.aspect))
            else {
                continue;
            };
            let uvs = panel
                .fill
                .iter()
                .map(|p| {
                    let uv = frame.uv(*p);
                    [uv.x as f32, uv.y as f32]
                })
                .collect();
            panel.print = Some((at, uvs));
        }
        self.prints = prints;
        self
    }

    pub fn with_selected_edges(mut self, edges: &[EdgeId]) -> Self {
        self.selected_edges = edges.to_vec();
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

/// A vertex of a printed fill: the tint mixed over the print (its
/// amount in alpha) and where it falls on the print.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PrintVertex {
    pos: [f32; 2],
    tint: [f32; 4],
    uv: [f32; 2],
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

    /// Dashes of `dash` with gaps of `gap` from `a` to `b`.
    fn dashed(&mut self, a: DVec2, b: DVec2, dash: f64, gap: f64, width: f64, color: Rgba) {
        let length = a.distance(b);
        let period = dash + gap;
        if length <= 0.0 || period <= 0.0 {
            return;
        }
        let dir = (b - a) / length;
        // At most a few thousand dashes, whatever the zoom.
        let count = (length / period).ceil().min(4000.0) as usize;
        for k in 0..count {
            let s = k as f64 * period;
            self.segment(a + dir * s, a + dir * (s + dash).min(length), width, color);
        }
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
    /// Printed fills, drawn between the grid and the rest.
    printed: wgpu::RenderPipeline,
    prints: PrintCache,
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
        let prints = PrintCache::new(gpu, wgpu::ShaderStages::FRAGMENT);
        let printed_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("board prints"),
            bind_group_layouts: &[Some(&layout), Some(prints.layout())],
            immediate_size: 0,
        });
        let make =
            |label, layout: &wgpu::PipelineLayout, vs, fs, buffer: wgpu::VertexBufferLayout<'_>| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some(vs),
                        compilation_options: Default::default(),
                        buffers: &[Some(buffer)],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
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
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            };
        let pipeline = make(
            "board",
            &pipeline_layout,
            "vs_main",
            "fs_main",
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
            },
        );
        let printed = make(
            "board prints",
            &printed_layout,
            "vs_print",
            "fs_print",
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PrintVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2, 1 => Float32x4, 2 => Float32x2
                ],
            },
        );
        Self {
            format,
            pipeline,
            printed,
            prints,
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

        // Construction lines, dashed across the whole board, in a soft
        // tint of the selection colour.
        let guide = mix(theme.board_background, theme.selection, 0.55);
        for [a, b] in &scene.guides {
            let Some(dir) = (*b - *a).try_normalize() else {
                continue;
            };
            let reach = (hi - lo).length();
            let from = *a + dir * dir.dot(view.center - *a) - dir * reach;
            batch.dashed(
                from,
                from + dir * 2.0 * reach,
                8.0 * px,
                5.0 * px,
                px,
                guide,
            );
        }
        // The grid and guides lie under the printed fills, the rest over.
        let under = batch.vertices.len();
        // Printed fills, by print: (key, first vertex, count).
        let mut printed: Vec<PrintVertex> = Vec::new();
        let mut runs: Vec<(u64, u32, u32)> = Vec::new();

        for panel in &scene.panels {
            let selected = scene.selected.contains(&panel.id);
            let base = theme.pieces[panel.color % PIECE_COLORS];
            let fill = if selected {
                mix(base, theme.selection, 0.18)
            } else {
                base
            };
            let print = panel
                .print
                .as_ref()
                .and_then(|(at, uvs)| Some((scene.prints.get(*at)?, uvs)))
                .filter(|(_, uvs)| uvs.len() == panel.fill.len());
            match print {
                Some((print, uvs)) => {
                    self.prints.prepare(
                        gpu,
                        Printed {
                            key: print.key,
                            texture: &print.texture,
                        },
                    );
                    let first = printed.len() as u32;
                    let tint = if selected {
                        let [r, g, b, _] = theme.selection;
                        [r, g, b, 0.25]
                    } else {
                        [0.0; 4]
                    };
                    for t in &panel.triangles {
                        for i in t {
                            if let (Some(p), Some(uv)) =
                                (panel.fill.get(*i as usize), uvs.get(*i as usize))
                            {
                                printed.push(PrintVertex {
                                    pos: [p.x as f32, p.y as f32],
                                    tint,
                                    uv: *uv,
                                });
                            }
                        }
                    }
                    runs.push((print.key, first, printed.len() as u32 - first));
                }
                None => {
                    for t in &panel.triangles {
                        if let [Some(a), Some(b), Some(c)] = t.map(|i| panel.fill.get(i as usize)) {
                            batch.tri(*a, *b, *c, fill);
                        }
                    }
                }
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
            for hole in &panel.holes {
                batch.polyline(hole, true, width, line_color);
            }
            for (pos, outward) in &panel.notches {
                batch.segment(*pos, *pos - *outward * NOTCH_MM, width, line_color);
            }
        }
        // Seams: both sides in the same colour, over the outlines, with a
        // chevron at the middle pointing the way the seam walks the side
        // (matching sides point alike when the pieces lie side by side).
        // Sides of different lengths are red, with a dot.
        for seam in &scene.seam_sides {
            let color = if seam.mismatch {
                WARNING
            } else {
                SEAM_COLORS[seam.index % SEAM_COLORS.len()]
            };
            batch.polyline(&seam.points, false, 2.5 * px, color);
            if let Some((mid, dir)) = midpoint(&seam.points) {
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
        // Trims. Zippers: a band of teeth along each side, solid where it
        // is closed, dashed where it is open, a bar at the bottom stop.
        for z in &scene.zippers {
            for side in &z.sides {
                let total = r3d_core::seam::polyline_length(side);
                let cut = total * z.closed;
                let closed = r3d_core::seam::sub_polyline(side, 0.0, cut);
                let open = r3d_core::seam::sub_polyline(side, cut, total);
                if cut > 0.0 {
                    batch.polyline(&closed, false, 4.0 * px, ZIPPER);
                }
                for w in open.windows(2) {
                    if let [a, b] = w {
                        batch.dashed(*a, *b, 4.0 * px, 3.0 * px, 3.0 * px, ZIPPER);
                    }
                }
                if let [a, b, ..] = side.as_slice()
                    && let Some(dir) = (*b - *a).try_normalize()
                {
                    let across = DVec2::new(-dir.y, dir.x) * 5.0 * px;
                    batch.segment(*a - across, *a + across, 2.5 * px, ZIPPER);
                }
            }
        }
        // Elastics: a zigzag along their side, as a few pixels on screen.
        for line in &scene.elastics {
            let total = r3d_core::seam::polyline_length(line);
            let step = 4.0 * px;
            let n = (total / step).floor() as usize;
            let zigzag: Vec<DVec2> = (0..=n)
                .filter_map(|k| {
                    let s = (k as f64 * step).min(total);
                    let (p, dir) = r3d_core::seam::point_along(line, s);
                    let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                    Some(p + dir.try_normalize()?.perp() * (3.0 * px * side))
                })
                .collect();
            batch.polyline(&zigzag, false, 1.5 * px, ELASTIC);
        }
        // Topstitches: their stitches at their real length, a gap where
        // the thread goes in, in the thread's colour over a faint dark
        // edge (a white thread shows on a white piece).
        for t in &scene.topstitches {
            let color = crate::theme::hex(t.rgb);
            let mut edge = theme.panel_line;
            edge[3] *= 0.45;
            let width = t.width_mm.max(1.5 * px);
            let visible = (t.stitch_mm - 0.6).max(0.4 * t.stitch_mm);
            for row in &t.rows {
                let total = r3d_core::seam::polyline_length(row);
                let mut s = 0.0;
                while s + visible <= total + 1e-9 {
                    let (a, _) = r3d_core::seam::point_along(row, s);
                    let (b, _) = r3d_core::seam::point_along(row, s + visible);
                    batch.segment(a, b, width + 1.5 * px, edge);
                    batch.segment(a, b, width, color);
                    s += t.stitch_mm;
                }
            }
        }
        // Buttons at their real size (a ring and the thread's dot), and
        // buttonholes as slits with bar tacks; unbuttoned ones fainter.
        for b in &scene.buttons {
            let mut color = theme.panel_line;
            if !b.closed {
                color[3] *= 0.5;
            }
            let r = (b.diameter / 2.0).max(2.0 * px);
            let ring: Vec<DVec2> = (0..24)
                .map(|k| b.at + DVec2::from_angle(k as f64 / 24.0 * std::f64::consts::TAU) * r)
                .collect();
            batch.polyline(&ring, true, 1.5 * px, color);
            batch.dot(b.at, 2.0 * px, color);
            let [s0, s1] = b.slit;
            batch.segment(s0, s1, 2.0 * px, color);
            if let Some(dir) = (s1 - s0).try_normalize() {
                let across = DVec2::new(-dir.y, dir.x) * 3.0 * px;
                for end in [s0, s1] {
                    batch.segment(end - across, end + across, 2.0 * px, color);
                }
            }
        }
        // Symmetry axes: long dash, dot.
        for [a, b] in &scene.axes {
            batch.dashed(*a, *b, 12.0 * px, 3.0 * px, 1.5 * px, theme.panel_line);
        }
        for edge in scene
            .edges
            .iter()
            .filter(|e| scene.selected_edges.contains(&e.id))
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
        let print_buffer = (!printed.is_empty()).then(|| {
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("board prints"),
                    contents: bytemuck::cast_slice(&printed),
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
            pass.set_bind_group(0, &self.bind_group, &[]);
            let all = batch.vertices.len() as u32;
            let under = under as u32;
            if let Some(vb) = &vertex_buffer {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..under, 0..1);
            }
            if let Some(pb) = &print_buffer {
                pass.set_pipeline(&self.printed);
                pass.set_vertex_buffer(0, pb.slice(..));
                for (key, first, count) in &runs {
                    pass.set_bind_group(1, self.prints.get(Some(*key)), &[]);
                    pass.draw(*first..first + count, 0..1);
                }
            }
            if let Some(vb) = &vertex_buffer {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(under..all, 0..1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
        let used: Vec<u64> = runs.iter().map(|r| r.0).collect();
        self.prints.retain(&used);
    }
}
