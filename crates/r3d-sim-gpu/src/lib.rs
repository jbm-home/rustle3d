//! The GPU backend of the cloth solver: XPBD with small steps on wgpu
//! compute (docs/physics.md §6), behind `r3d_sim::Solver`.
//!
//! The model is the CPU backend's ([`r3d_sim::Model`]); only the order of
//! the Gauss–Seidel sweep differs. Each kind of constraint is split into
//! colours, sets of constraints that share no particle (greedy graph
//! colouring), and a colour is one parallel dispatch. Contacts with the
//! body, the edges held out of it and self-collision are computed per
//! particle by gathering (Jacobi, averaged), so no two invocations ever
//! write the same particle. Self-collision candidates come from a spatial
//! hash rebuilt on the GPU once per frame (Teschner et al. 2003) and are
//! kept sorted, so a run is deterministic on a given GPU. Contacts between
//! garments (`r3d_sim::layers`) gather their candidates from the same
//! hash, rebuilt within the frame whenever the cloth has moved enough (a
//! flag the hash kernels read), and push a triangle's corners through
//! integer atomics, whose sums do not depend on their order.
//!
//! A frame is one command buffer (about thirty dispatches per substep);
//! positions and velocities are read back at its end. A moving body's
//! parts (`r3d_sim::motion`) are uploaded with each frame, three rigid
//! motions per bone and substep. (Kernels may bind at most 8 storage
//! buffers: the field's nodes' vertices and the skin weights follow its
//! values in one buffer, and the bones' boxes lead the body's motions in
//! another.)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use r3d_geom::Rigid;
use r3d_sim::collide::{BAND, BRICK};
use r3d_sim::layers::{BORDER, LAYER_SKIN, OPEN_SEAM, OVERHANG, RECOVER, RECOVERY_SPEED, TURNED};
use r3d_sim::model::Model;
use r3d_sim::motion::{BodyMotion, ITERATIONS, ON_SKIN};
use r3d_sim::selfcollide::MARGIN;
use r3d_sim::solver::{
    CLOSING_SPEED, CLOSURE_COMPLIANCE, GRAB_COMPLIANCE, NEAR, SEWING_TIME, SKIN, SUBSTEPS,
    StepError, StepStats, grabbed, sewing, sewing_ramp,
};
use r3d_sim::{SimScene, Solver};

/// Self-collision candidates kept per particle; past this the highest
/// indices are dropped (a particle has about five on another layer).
const MAX_NEIGHBOURS: u32 = 32;
/// Candidate triangles of other garments kept per particle (sorted; past
/// this the highest indices are dropped).
const MAX_TOUCHES: u32 = 24;
/// Unit of the integer sums of the pushes between garments (m): 0.1 µm,
/// up to ±214 m.
const FIXED: f32 = 1e-7;
/// The average push of contacts between garments on a particle is scaled
/// by this (successive over-relaxation of averaged constraints, Macklin et
/// al. 2014 §4.2: from 1 to 2).
const LAYER_RELAXATION: f32 = 1.5;
/// Marks a triangle of the cloth's own in `GpuLayerTri::bridged`.
const NONE: u32 = u32::MAX;
/// Contacts between garments gather their candidates again, if the cloth
/// has moved enough, every this many substeps once the garment is sewn
/// (every substep before, when the seams close at up to 20 m/s): a check
/// is six dispatches, and checked every substep they made the frame of a
/// shirt and trousers 6 ms longer (RTX 2080).
const REGATHER_EVERY: u32 = 5;
const WORKGROUP: u32 = 64;
/// Uniform slots are this far apart (the largest dynamic offset alignment
/// wgpu allows a device to require).
const SLOT: u64 = 256;

/// Why the GPU backend cannot run.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GpuSimError {
    #[error("the GPU refused the cloth solver: {0}")]
    Setup(String),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct StepUniform {
    gravity_h: [f32; 4],
    params: [f32; 4],
    grab_target: [f32; 4],
    /// x: how far the sewing ramp has gone (0–1).
    sewing: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct RangeUniform {
    start: u32,
    count: u32,
    _pad: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    sdf_origin: [f32; 4],
    sdf_dims: [u32; 4],
    contact: [f32; 4],
    counts: [u32; 4],
    /// Where the field's nodes' vertices start in its values, bones of a
    /// moving body (0: it stands still), where the skin weights start.
    body: [u32; 4],
    /// x: how far the corners of a triangle of another garment a
    /// particle may touch can be (m); 0: a single garment.
    layers: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuLayerTri {
    /// Corners; garment | free edges << 16.
    v: [u32; 4],
    /// The cloth triangle a seam's bridge continues ([`NONE`]: cloth).
    bridged: u32,
    _pad: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuMembrane {
    v: [u32; 4],
    coef: [f32; 4],
    alpha: [f32; 4],
}

/// An elastic segment (r3d-sim `ElasticLink`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuElastic {
    v: [u32; 2],
    /// Length on the pattern, relaxed length (m), 1 / stiffness (1/N).
    full: f32,
    rest: f32,
    inv_stiffness: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuHinge {
    v: [u32; 4],
    alpha: [f32; 4],
}

/// Storage bindings of group 1 (see `xpbd.wgsl`).
mod bind {
    pub const X: u32 = 0;
    pub const PREV: u32 = 1;
    pub const VEL: u32 = 2;
    pub const PDATA: u32 = 3;
    pub const MEMBRANES: u32 = 4;
    pub const HINGES: u32 = 5;
    pub const SEAMS: u32 = 6;
    pub const NEARBY: u32 = 7;
    pub const DELTA: u32 = 8;
    pub const EDGE_ROWS: u32 = 9;
    pub const SEWN: u32 = 10;
    pub const EXCLUDED: u32 = 11;
    pub const SDF_SLOTS: u32 = 12;
    pub const SDF_VALUES: u32 = 13;
    pub const GRAB: u32 = 14;
    pub const CELL_COUNT: u32 = 15;
    pub const CELL_START: u32 = 16;
    pub const SORTED: u32 = 17;
    pub const NEIGHBOURS: u32 = 18;
    pub const NEIGHBOUR_COUNT: u32 = 19;
    pub const CONTACTS: u32 = 20;
    pub const BODY: u32 = 21;
    pub const CLOSURES: u32 = 22;
    pub const GARMENT: u32 = 23;
    pub const INCIDENT: u32 = 24;
    pub const TRIANGLES: u32 = 25;
    pub const TOUCHES: u32 = 26;
    pub const LAYER_DELTA: u32 = 27;
    pub const GATHERED: u32 = 28;
    pub const ELASTICS: u32 = 29;
    /// Bindings the kernels only read.
    pub const READ_ONLY: [u32; 16] = [
        PDATA, MEMBRANES, HINGES, SEAMS, EDGE_ROWS, SEWN, EXCLUDED, SDF_SLOTS, SDF_VALUES, GRAB,
        BODY, CLOSURES, GARMENT, INCIDENT, TRIANGLES, ELASTICS,
    ];
}

/// The kernels (entry points of `xpbd.wgsl`) and the group-1 bindings
/// each uses; a kernel's index here is its id.
mod kernel {
    use super::bind::*;

    pub const PREDICT: usize = 0;
    pub const MEMBRANE: usize = 1;
    pub const HINGE: usize = 2;
    pub const SEAM: usize = 3;
    pub const PULL: usize = 4;
    pub const COLLIDE: usize = 5;
    pub const EDGES: usize = 6;
    pub const APPLY: usize = 7;
    pub const VELOCITY: usize = 8;
    pub const HASH_CLEAR: usize = 9;
    pub const HASH_COUNT: usize = 10;
    pub const HASH_SCAN: usize = 11;
    pub const HASH_SCATTER: usize = 12;
    pub const FIND_NEIGHBOURS: usize = 13;
    pub const SELF_COUNT: usize = 14;
    pub const SELF_GATHER: usize = 15;
    pub const SELF_APPLY: usize = 16;
    pub const CLOSURE: usize = 17;
    pub const LAYER_MARK: usize = 18;
    pub const LAYER_RESET: usize = 19;
    pub const LAYER_MOVED: usize = 20;
    pub const LAYER_FIND: usize = 21;
    pub const LAYER_SOLVE: usize = 22;
    pub const LAYER_TURNED: usize = 23;
    pub const LAYER_AVERAGE: usize = 24;
    pub const ELASTIC: usize = 25;

    pub const ALL: [(&str, &[u32]); 26] = [
        ("predict", &[X, PREV, VEL, PDATA]),
        ("membrane", &[X, PDATA, MEMBRANES]),
        ("hinge", &[X, PDATA, HINGES]),
        ("seam", &[X, PDATA, SEAMS]),
        ("pull", &[X, PDATA, GRAB]),
        (
            "collide",
            &[X, PREV, PDATA, NEARBY, SDF_SLOTS, SDF_VALUES, BODY],
        ),
        (
            "edges",
            &[
                X, PDATA, NEARBY, DELTA, EDGE_ROWS, SDF_SLOTS, SDF_VALUES, BODY,
            ],
        ),
        ("apply", &[X, DELTA]),
        ("velocity", &[X, PREV, VEL, PDATA]),
        ("hash_clear", &[CELL_COUNT, TOUCHES]),
        ("hash_count", &[X, CELL_COUNT, TOUCHES]),
        ("hash_scan", &[CELL_COUNT, CELL_START, TOUCHES]),
        ("hash_scatter", &[X, CELL_COUNT, SORTED, TOUCHES]),
        (
            "find_neighbours",
            &[
                X,
                PDATA,
                EXCLUDED,
                CELL_START,
                SORTED,
                NEIGHBOURS,
                NEIGHBOUR_COUNT,
                GARMENT,
            ],
        ),
        (
            "self_count",
            &[X, PDATA, NEIGHBOURS, NEIGHBOUR_COUNT, CONTACTS],
        ),
        (
            "self_gather",
            &[X, PDATA, DELTA, SEWN, NEIGHBOURS, NEIGHBOUR_COUNT, CONTACTS],
        ),
        ("self_apply", &[X, DELTA, SEWN]),
        ("closure", &[X, PREV, PDATA, CLOSURES]),
        ("layer_mark", &[TOUCHES]),
        ("layer_reset", &[TOUCHES]),
        ("layer_moved", &[X, TOUCHES, GATHERED]),
        (
            "layer_find",
            &[
                X, PDATA, CELL_START, SORTED, INCIDENT, TRIANGLES, TOUCHES, GATHERED,
            ],
        ),
        (
            "layer_solve",
            &[
                X,
                PREV,
                PDATA,
                NEARBY,
                SEWN,
                TRIANGLES,
                TOUCHES,
                LAYER_DELTA,
            ],
        ),
        (
            "layer_turned",
            &[X, TRIANGLES, TOUCHES, SDF_SLOTS, SDF_VALUES, BODY],
        ),
        ("layer_average", &[X, LAYER_DELTA, DELTA, SEWN]),
        ("elastic", &[X, PDATA, ELASTICS]),
    ];
}

/// The solver's kernels compiled for a device: made once and shared by
/// every solver on it (compiling them is most of the cost of a solver,
/// ~25 ms, and the app rebuilds its solver on pattern edits).
#[derive(Debug, Clone)]
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    uniform_layout: wgpu::BindGroupLayout,
    /// Per kernel id: the layout of its group 1, and its pipeline.
    kernels: Vec<(wgpu::BindGroupLayout, wgpu::ComputePipeline)>,
    /// The last body uploaded, reused while it is the same: the app
    /// rebuilds its solver on pattern edits, and a body's distance field
    /// is ~7 MB.
    body: Arc<Mutex<Option<BodyBuffers>>>,
}

/// A body's distance field on the GPU, and what it was made from.
#[derive(Debug)]
struct BodyBuffers {
    slots: Vec<u32>,
    field: Vec<u32>,
    slots_buffer: wgpu::Buffer,
    field_buffer: wgpu::Buffer,
}

/// A storage buffer holding `bytes` (at least 16: a binding cannot be
/// empty).
fn storage(
    device: &wgpu::Device,
    label: &str,
    bytes: &[u8],
    extra: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let mut contents = bytes.to_vec();
    contents.resize(contents.len().max(16), 0);
    wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &contents,
            usage: wgpu::BufferUsages::STORAGE | extra,
        },
    )
}

impl GpuContext {
    /// Compile the kernels on `device`. Fails, without side effects on
    /// the device, if the GPU refuses any of it.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, GpuSimError> {
        scoped(device, || Self::compile(device, queue))
    }

    fn compile(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let source = format!(
            "const SKIN: f32 = {SKIN:?};\nconst NEAR: f32 = {NEAR:?};\nconst BAND: f32 = {BAND:?};\n\
             const BRICK: u32 = {BRICK}u;\nconst MAX_NEIGHBOURS: u32 = {MAX_NEIGHBOURS}u;\n\
             const ITERATIONS: u32 = {ITERATIONS}u;\nconst ON_SKIN: f32 = {ON_SKIN:?};\n\
             const CLOSURE_COMPLIANCE: f32 = {CLOSURE_COMPLIANCE:?};\n\
             const CLOSING_SPEED: f32 = {CLOSING_SPEED:?};\n\
             const MAX_TOUCHES: u32 = {MAX_TOUCHES}u;\nconst LAYER_SKIN: f32 = {LAYER_SKIN:?};\n\
             const OVERHANG: f32 = {OVERHANG:?};\nconst RECOVER: f32 = {RECOVER:?};\n\
             const RECOVERY_SPEED: f32 = {RECOVERY_SPEED:?};\n\
             const OPEN_SEAM: f32 = {OPEN_SEAM:?};\nconst BORDER: u32 = {BORDER}u;\n\
             const MARGIN: f32 = {MARGIN:?};\nconst FIXED: f32 = {FIXED:?};\n\
             const LAYER_RELAXATION: f32 = {LAYER_RELAXATION:?};\n\
             const TURNED: f32 = {TURNED:?};\n{}",
            include_str!("shaders/xpbd.wgsl")
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloth xpbd"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let uniform_entry = |binding: u32, dynamic: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        };
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloth uniforms"),
            entries: &[
                uniform_entry(0, true),
                uniform_entry(1, true),
                uniform_entry(2, false),
            ],
        });
        let kernels = kernel::ALL
            .iter()
            .map(|(entry, bindings)| {
                let entries: Vec<wgpu::BindGroupLayoutEntry> = bindings
                    .iter()
                    .map(|b| wgpu::BindGroupLayoutEntry {
                        binding: *b,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: bind::READ_ONLY.contains(b),
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    })
                    .collect();
                let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(entry),
                    entries: &entries,
                });
                let pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some(entry),
                        bind_group_layouts: &[Some(&uniform_layout), Some(&layout)],
                        immediate_size: 0,
                    });
                let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                });
                (layout, pipeline)
            })
            .collect();
        Self {
            device: device.clone(),
            queue: queue.clone(),
            uniform_layout,
            kernels,
            body: Arc::new(Mutex::new(None)),
        }
    }

    /// The buffers of a body's field: its `slots`, and the words of
    /// `field` (see [`field_words`]). The last ones if they hold the same
    /// field, new ones otherwise.
    fn body_buffers(&self, slots: &[u32], field: Vec<u32>) -> (wgpu::Buffer, wgpu::Buffer) {
        let make = |field: Vec<u32>| BodyBuffers {
            slots: slots.to_vec(),
            slots_buffer: storage(
                &self.device,
                "body slots",
                bytemuck::cast_slice(slots),
                wgpu::BufferUsages::empty(),
            ),
            field_buffer: storage(
                &self.device,
                "body field",
                bytemuck::cast_slice(&field),
                wgpu::BufferUsages::empty(),
            ),
            field,
        };
        let Ok(mut cached) = self.body.lock() else {
            let b = make(field);
            return (b.slots_buffer, b.field_buffer);
        };
        let b = match cached.take() {
            Some(b) if b.slots == slots && b.field == field => b,
            _ => make(field),
        };
        let out = (b.slots_buffer.clone(), b.field_buffer.clone());
        *cached = Some(b);
        out
    }
}

/// Run `f` and fail if the device reports a validation or out-of-memory
/// error meanwhile (on this thread).
fn scoped<T>(device: &wgpu::Device, f: impl FnOnce() -> T) -> Result<T, GpuSimError> {
    let scopes = [
        device.push_error_scope(wgpu::ErrorFilter::Validation),
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory),
    ];
    let out = f();
    // Pop in reverse order of pushing.
    for scope in scopes.into_iter().rev() {
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(GpuSimError::Setup(e.to_string()));
        }
    }
    Ok(out)
}

/// One dispatch of a kernel: its id, the uniform slot of its range and how
/// many workgroups.
#[derive(Debug, Clone, Copy)]
struct Dispatch {
    kernel: usize,
    range: u32,
    groups: u32,
}

/// The GPU cloth solver.
pub struct GpuSolver {
    context: GpuContext,
    /// Per kernel id: its group 1 on this solver's buffers.
    groups: Vec<wgpu::BindGroup>,
    uniforms: wgpu::BindGroup,
    step_buffer: wgpu::Buffer,
    scene_buffer: wgpu::Buffer,
    grab_buffer: wgpu::Buffer,
    /// Per part, its box (rest pose: low, high); then per substep and
    /// part, the part's motion at the substep's end, its inverse and its
    /// motion at the start (each a quaternion and a translation).
    body_buffer: wgpu::Buffer,
    motion: Option<BodyMotion>,
    x_buffer: wgpu::Buffer,
    vel_buffer: wgpu::Buffer,
    readback: wgpu::Buffer,
    /// Once per frame, before the substeps.
    frame_passes: Vec<Dispatch>,
    /// Every substep.
    substep_passes: Vec<Dispatch>,
    /// Before each substep but the first while sewing, then every
    /// [`REGATHER_EVERY`]th.
    regather_passes: Vec<Dispatch>,
    /// The grab's dispatch, inserted after self-collision while held.
    pull: Dispatch,
    /// Where `pull` goes in `substep_passes`.
    pull_at: usize,
    scene: SceneUniform,
    inv_mass: Vec<f32>,
    seams: Vec<[u32; 2]>,
    closures: Vec<[u32; 2]>,
    has_seams: bool,
    /// Positions and velocities read back after the last step.
    x: Vec<Vec3>,
    v: Vec<Vec3>,
    grab: Option<Vec3>,
    time: f32,
    stats: StepStats,
}

impl std::fmt::Debug for GpuSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuSolver")
            .field("particles", &self.x.len())
            .field("dispatches_per_substep", &self.substep_passes.len())
            .field("time", &self.time)
            .finish()
    }
}

/// Greedy colouring of constraints over `n` particles: no two constraints
/// of a colour share a particle. Returns the constraints' order (by colour,
/// then index) and each colour's `(start, count)` in that order.
fn colour<const N: usize>(items: &[[u32; N]], n: usize) -> (Vec<usize>, Vec<(u32, u32)>) {
    let mut used: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut colours: Vec<Vec<usize>> = Vec::new();
    for (k, item) in items.iter().enumerate() {
        let mut c = 0u32;
        while item
            .iter()
            .any(|v| used.get(*v as usize).is_some_and(|u| u.contains(&c)))
        {
            c += 1;
        }
        for v in item {
            if let Some(u) = used.get_mut(*v as usize) {
                u.push(c);
            }
        }
        let c = c as usize;
        if colours.len() <= c {
            colours.resize_with(c + 1, Vec::new);
        }
        colours[c].push(k);
    }
    let mut order = Vec::with_capacity(items.len());
    let mut ranges = Vec::with_capacity(colours.len());
    for list in colours {
        ranges.push((order.len() as u32, list.len() as u32));
        order.extend(list);
    }
    (order, ranges)
}

/// Rows as one array: entries `0..=n` are where each row starts (absolute
/// indices), the rows follow.
fn csr<'a>(rows: impl ExactSizeIterator<Item = &'a [u32]>) -> Vec<u32> {
    let n = rows.len();
    let mut out = vec![0u32; n + 1];
    let mut items = Vec::new();
    for (i, row) in rows.enumerate() {
        out[i] = (n + 1 + items.len()) as u32;
        items.extend_from_slice(row);
    }
    out[n] = (n + 1 + items.len()) as u32;
    out.extend(items);
    out
}

/// The first `size` bytes of `buffer` (a slot of a dynamic uniform).
fn sized(buffer: &wgpu::Buffer, size: u64) -> wgpu::BindingResource<'_> {
    wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        buffer,
        offset: 0,
        size: wgpu::BufferSize::new(size),
    })
}

fn groups(count: u32) -> u32 {
    count.div_ceil(WORKGROUP)
}

/// Bytes per bone and substep in the body buffer: three rigid motions of
/// two `vec4f` each.
const BODY_BYTES: u64 = 3 * 2 * 16;

/// A body's field as the kernels read it, in 32-bit words: its values,
/// then per value its node's vertex, then per vertex its skin weights
/// (four bone indices in one word, then four weights).
fn field_words(grid: &r3d_sim::collide::Grid<'_>) -> Vec<u32> {
    let mut words: Vec<u32> = grid.values.iter().map(|v| v.to_bits()).collect();
    words.extend_from_slice(grid.nodes);
    for s in grid.skin {
        words.push(u32::from_le_bytes(s.bones));
        words.extend(s.weights.map(f32::to_bits));
    }
    words
}

/// The body buffer's rows for a frame from `time` in substeps of `h`:
/// per substep and part, its motion at the substep's end (quaternion,
/// translation), the inverse of that, and its motion at the start.
fn body_rows(motion: &BodyMotion, time: f32, h: f32, parts: usize) -> Vec<[f32; 4]> {
    let rigid = |r: &Rigid| {
        let t = r.translation;
        [r.rotation.to_array(), [t.x, t.y, t.z, 0.0]]
    };
    let mut rows = Vec::with_capacity(SUBSTEPS as usize * parts * 6);
    let mut before = motion.at(time);
    for s in 1..=SUBSTEPS {
        let now = motion.at(time + s as f32 * h);
        for k in 0..parts {
            let (Some(n), Some(b)) = (now.get(k), before.get(k)) else {
                rows.extend([[0.0, 0.0, 0.0, 1.0], [0.0; 4]].repeat(3));
                continue;
            };
            rows.extend(rigid(n));
            rows.extend(rigid(&n.inverse()));
            rows.extend(rigid(b));
        }
        before = now;
    }
    rows
}

fn vec4s(v: &[Vec3]) -> Vec<[f32; 4]> {
    v.iter().map(|p| [p.x, p.y, p.z, 0.0]).collect()
}

impl GpuSolver {
    pub fn new(context: &GpuContext, scene: SimScene) -> Result<Self, GpuSimError> {
        Self::from_model(context, Model::new(scene))
    }

    /// Upload `model`. Fails, without side effects on the device, if the
    /// GPU refuses it.
    pub fn from_model(context: &GpuContext, model: Model) -> Result<Self, GpuSimError> {
        scoped(&context.device, || Self::build(context, model))?
    }

    fn build(context: &GpuContext, model: Model) -> Result<Self, GpuSimError> {
        let (device, queue) = (&context.device, &context.queue);
        let n = model.positions.len();
        let n32 = u32::try_from(n).map_err(|_| GpuSimError::Setup("too many particles".into()))?;

        // Constraints, reordered by colour.
        let (membrane_order, membrane_ranges) =
            colour(&model.membranes.iter().map(|m| m.v).collect::<Vec<_>>(), n);
        let membranes: Vec<GpuMembrane> = membrane_order
            .iter()
            .filter_map(|k| model.membranes.get(*k))
            .map(|m| GpuMembrane {
                v: [m.v[0], m.v[1], m.v[2], 0],
                coef: [m.cu.x, m.cu.y, m.cv.x, m.cv.y],
                alpha: [
                    m.alpha_u,
                    m.alpha_v,
                    m.alpha_s,
                    if m.buckles { 1.0 } else { 0.0 },
                ],
            })
            .collect();
        let (hinge_order, hinge_ranges) =
            colour(&model.hinges.iter().map(|h| h.v).collect::<Vec<_>>(), n);
        let hinges: Vec<GpuHinge> = hinge_order
            .iter()
            .filter_map(|k| model.hinges.get(*k))
            .map(|h| GpuHinge {
                v: h.v,
                alpha: [h.alpha, 0.0, 0.0, 0.0],
            })
            .collect();
        let (seam_order, seam_ranges) = colour(&model.seams, n);
        let seams: Vec<[u32; 2]> = seam_order
            .iter()
            .filter_map(|k| model.seams.get(*k))
            .copied()
            .collect();
        let (closure_order, closure_ranges) = colour(&model.closures, n);
        let closures: Vec<[u32; 2]> = closure_order
            .iter()
            .filter_map(|k| model.closures.get(*k))
            .copied()
            .collect();
        let (elastic_order, elastic_ranges) =
            colour(&model.elastics.iter().map(|e| e.v).collect::<Vec<_>>(), n);
        let elastics: Vec<GpuElastic> = elastic_order
            .iter()
            .filter_map(|k| model.elastics.get(*k))
            .map(|e| GpuElastic {
                v: e.v,
                full: e.full,
                rest: e.rest,
                inv_stiffness: 1.0 / e.stiffness,
                _pad: [0.0; 3],
            })
            .collect();

        // Per-particle rows.
        let mut adjacent: Vec<Vec<u32>> = vec![Vec::new(); n];
        for [a, b] in &model.edges {
            adjacent[*a as usize].push(*b);
            adjacent[*b as usize].push(*a);
        }
        let edge_csr = csr(adjacent.iter().map(Vec::as_slice));
        let contact = &model.self_contact;
        let sewn_csr = csr(contact.sewn.iter().map(Vec::as_slice));
        let excluded_csr = csr(contact.excluded.iter().map(Vec::as_slice));
        let pdata: Vec<[f32; 4]> = (0..n)
            .map(|i| {
                [
                    model.inv_mass[i],
                    model.thickness[i],
                    model.friction[i],
                    model.damping[i],
                ]
            })
            .collect();

        // The body's field, with its skin if it moves.
        let (sdf_origin, sdf_dims, sdf_slots, field, values) = match &model.collider {
            Some(mesh) => {
                let g = mesh.grid();
                let d = g.dims.map(|d| d as u32);
                (
                    [g.origin.x, g.origin.y, g.origin.z, g.voxel],
                    [d[0], d[1], d[2], 1],
                    g.slots,
                    field_words(&g),
                    g.values.len() as u32,
                )
            }
            None => ([0.0, 0.0, 0.0, 1.0], [0; 4], &[][..], Vec::new(), 0),
        };
        let part_boxes: Vec<[f32; 4]> = match (&model.collider, &model.motion) {
            (Some(mesh), Some(_)) => mesh
                .part_boxes()
                .iter()
                .flat_map(|(lo, hi)| [[lo.x, lo.y, lo.z, 0.0], [hi.x, hi.y, hi.z, 0.0]])
                .collect(),
            _ => Vec::new(),
        };
        let parts = match &model.motion {
            Some(m) => (part_boxes.len() / 2).min(m.parts()) as u32,
            None => 0,
        };
        let self_collision = contact.radius > 0.0 && n > 1;
        let reach = 2.0 * contact.radius + MARGIN;
        let table = (2 * n).next_power_of_two().clamp(256, 1 << 20) as u32;
        // Contacts between garments: their triangles (the cloth's, then
        // the seams' bridges) and each particle's.
        let layers = &model.layers;
        let layered = layers.reach > 0.0 && n > 1;
        let layer_tris: Vec<GpuLayerTri> = layers
            .triangles
            .iter()
            .enumerate()
            .map(|(k, t)| GpuLayerTri {
                v: [
                    t[0],
                    t[1],
                    t[2],
                    u32::from(layers.triangle_garment.get(k).copied().unwrap_or(0))
                        | u32::from(layers.free.get(k).copied().unwrap_or(0)) << 16,
                ],
                bridged: k
                    .checked_sub(layers.cloth)
                    .and_then(|b| layers.bridged.get(b).copied())
                    .unwrap_or(NONE),
                _pad: [0; 3],
            })
            .collect();
        let incident_csr = csr(layers.incident.iter().map(Vec::as_slice));
        let garment_of: Vec<u32> = layers.garment.iter().map(|g| u32::from(*g)).collect();
        let rows = u64::from(MAX_TOUCHES + 1) * n as u64;
        let scene = SceneUniform {
            sdf_origin,
            sdf_dims,
            contact: [
                contact.radius,
                // The hash's cells hold both kinds of candidates.
                if layered {
                    reach.max(layers.reach)
                } else {
                    reach
                },
                if model.ground { 1.0 } else { 0.0 },
                reach,
            ],
            counts: [n32, table, 0, 0],
            body: [values, parts, 2 * values, 0],
            layers: [
                if layered { layers.reach } else { 0.0 },
                layers.cloth as f32,
                0.0,
                0.0,
            ],
        };

        // Buffers.
        use wgpu::BufferUsages as U;
        let storage = |label: &str, bytes: &[u8], extra: U| storage(device, label, bytes, extra);
        let (sdf_slots_buffer, sdf_values_buffer) = context.body_buffers(sdf_slots, field);
        let body_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("body motion"),
            size: (16 * part_boxes.len() as u64
                + BODY_BYTES * u64::from(SUBSTEPS) * u64::from(parts))
            .max(16),
            usage: U::STORAGE | U::COPY_DST,
            mapped_at_creation: false,
        });
        if parts > 0 {
            queue.write_buffer(&body_buffer, 0, bytemuck::cast_slice(&part_boxes));
        }
        let zeroed = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.max(16),
                usage: U::STORAGE,
                mapped_at_creation: false,
            })
        };
        let start = vec4s(&model.positions);
        let x_buffer = storage("cloth x", bytemuck::cast_slice(&start), U::COPY_SRC);
        let prev_buffer = storage("cloth prev", bytemuck::cast_slice(&start), U::empty());
        let vel_buffer = storage(
            "cloth v",
            bytemuck::cast_slice(&vec![[0f32; 4]; n]),
            U::COPY_SRC,
        );
        let buffers: HashMap<u32, wgpu::Buffer> = [
            (
                bind::PDATA,
                storage("cloth pdata", bytemuck::cast_slice(&pdata), U::empty()),
            ),
            (
                bind::MEMBRANES,
                storage(
                    "cloth membranes",
                    bytemuck::cast_slice(&membranes),
                    U::empty(),
                ),
            ),
            (
                bind::HINGES,
                storage("cloth hinges", bytemuck::cast_slice(&hinges), U::empty()),
            ),
            (
                bind::SEAMS,
                storage("cloth seams", bytemuck::cast_slice(&seams), U::empty()),
            ),
            (
                bind::CLOSURES,
                storage(
                    "cloth closures",
                    bytemuck::cast_slice(&closures),
                    U::empty(),
                ),
            ),
            (
                bind::ELASTICS,
                storage(
                    "cloth elastics",
                    bytemuck::cast_slice(&elastics),
                    U::empty(),
                ),
            ),
            (bind::NEARBY, zeroed("cloth near", 4 * n as u64)),
            (bind::DELTA, zeroed("cloth delta", 16 * n as u64)),
            (
                bind::EDGE_ROWS,
                storage("cloth edges", bytemuck::cast_slice(&edge_csr), U::empty()),
            ),
            (
                bind::SEWN,
                storage("cloth sewn", bytemuck::cast_slice(&sewn_csr), U::empty()),
            ),
            (
                bind::EXCLUDED,
                storage(
                    "cloth excluded",
                    bytemuck::cast_slice(&excluded_csr),
                    U::empty(),
                ),
            ),
            (bind::SDF_SLOTS, sdf_slots_buffer),
            (bind::SDF_VALUES, sdf_values_buffer),
            (
                bind::CELL_COUNT,
                zeroed("hash counts", 4 * u64::from(table)),
            ),
            (
                bind::CELL_START,
                zeroed("hash starts", 4 * (u64::from(table) + 1)),
            ),
            (bind::SORTED, zeroed("hash sorted", 4 * n as u64)),
            (
                bind::NEIGHBOURS,
                zeroed("cloth neighbours", 4 * u64::from(MAX_NEIGHBOURS) * n as u64),
            ),
            (
                bind::NEIGHBOUR_COUNT,
                zeroed("cloth neighbour counts", 4 * n as u64),
            ),
            (bind::CONTACTS, zeroed("cloth contacts", 4 * n as u64)),
            (
                bind::GARMENT,
                storage(
                    "cloth garments",
                    bytemuck::cast_slice(&garment_of),
                    U::empty(),
                ),
            ),
            (
                bind::INCIDENT,
                storage(
                    "layer incident",
                    bytemuck::cast_slice(&incident_csr),
                    U::empty(),
                ),
            ),
            (
                bind::TRIANGLES,
                storage(
                    "layer triangles",
                    bytemuck::cast_slice(&layer_tris),
                    U::empty(),
                ),
            ),
            // The rows, then both flags raised (the first frame gathers),
            // then whether each cloth triangle is turned over.
            (bind::TOUCHES, {
                let mut words = vec![0u32; rows as usize];
                words.extend([1, 1]);
                words.extend(std::iter::repeat_n(0, layers.cloth));
                storage("layer touches", bytemuck::cast_slice(&words), U::empty())
            }),
            (bind::LAYER_DELTA, zeroed("layer pushes", 16 * n as u64)),
            (
                bind::GATHERED,
                storage("layer gathered", bytemuck::cast_slice(&start), U::empty()),
            ),
        ]
        .into_iter()
        .collect();
        let grab_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloth grab"),
            size: (16 * n as u64).max(16),
            usage: U::STORAGE | U::COPY_DST,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloth readback"),
            size: (32 * n as u64).max(16),
            usage: U::MAP_READ | U::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: U::UNIFORM | U::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let step_buffer = uniform("cloth steps", SLOT * u64::from(SUBSTEPS));
        let scene_buffer = uniform("cloth scene", std::mem::size_of::<SceneUniform>() as u64);
        queue.write_buffer(&scene_buffer, 0, bytemuck::bytes_of(&scene));

        // Ranges: slot 0 is unused (kernels over all particles), then the
        // colours of each kind.
        let mut ranges = vec![(0u32, n32)];
        // (uniform slot, constraint count) of each colour.
        let mut slots = |list: &[(u32, u32)]| -> Vec<(u32, u32)> {
            let first = ranges.len() as u32;
            ranges.extend_from_slice(list);
            (first..).zip(list.iter().map(|r| r.1)).collect()
        };
        let membrane_slots = slots(&membrane_ranges);
        let hinge_slots = slots(&hinge_ranges);
        let seam_slots = slots(&seam_ranges);
        let closure_slots = slots(&closure_ranges);
        let elastic_slots = slots(&elastic_ranges);
        let mut range_bytes = vec![0u8; ranges.len() * SLOT as usize];
        for (k, (start, count)) in ranges.iter().enumerate() {
            let r = RangeUniform {
                start: *start,
                count: *count,
                _pad: [0; 2],
            };
            let at = k * SLOT as usize;
            range_bytes[at..at + 16].copy_from_slice(bytemuck::bytes_of(&r));
        }
        let range_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("cloth ranges"),
                contents: &range_bytes,
                usage: U::UNIFORM,
            },
        );

        // Bindings.
        let uniforms = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloth uniforms"),
            layout: &context.uniform_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sized(&step_buffer, std::mem::size_of::<StepUniform>() as u64),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sized(&range_buffer, std::mem::size_of::<RangeUniform>() as u64),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scene_buffer.as_entire_binding(),
                },
            ],
        });
        let buffer_of = |b: u32| match b {
            bind::X => Some(&x_buffer),
            bind::PREV => Some(&prev_buffer),
            bind::VEL => Some(&vel_buffer),
            bind::GRAB => Some(&grab_buffer),
            bind::BODY => Some(&body_buffer),
            other => buffers.get(&other),
        };
        let mut bind_groups = Vec::with_capacity(kernel::ALL.len());
        for ((entry, bindings), (layout, _)) in kernel::ALL.iter().zip(&context.kernels) {
            let mut entries = Vec::with_capacity(bindings.len());
            for b in *bindings {
                let buffer = buffer_of(*b)
                    .ok_or_else(|| GpuSimError::Setup(format!("no buffer for binding {b}")))?;
                entries.push(wgpu::BindGroupEntry {
                    binding: *b,
                    resource: buffer.as_entire_binding(),
                });
            }
            bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(entry),
                layout,
                entries: &entries,
            }));
        }

        use kernel::*;
        let all = |kernel| Dispatch {
            kernel,
            range: 0,
            groups: groups(n32),
        };
        let one = |kernel| Dispatch {
            kernel,
            range: 0,
            groups: 1,
        };
        // Over the cloth's triangles.
        let turned = Dispatch {
            kernel: LAYER_TURNED,
            range: 0,
            groups: groups(u32::try_from(layers.cloth).unwrap_or(u32::MAX)),
        };
        let hash = [
            Dispatch {
                kernel: HASH_CLEAR,
                range: 0,
                groups: groups(table),
            },
            all(HASH_COUNT),
            one(HASH_SCAN),
            all(HASH_SCATTER),
        ];
        let mut frame_passes = Vec::new();
        let mut substep_passes = vec![all(PREDICT)];
        // Gather the candidates again if the cloth has moved.
        let mut regather_passes = Vec::new();
        if layered {
            regather_passes.push(all(LAYER_MOVED));
            regather_passes.extend(hash);
            regather_passes.push(turned);
            regather_passes.push(all(LAYER_FIND));
            regather_passes.push(one(LAYER_RESET));
        }
        let coloured = |kernel, slots: &[(u32, u32)]| {
            slots
                .iter()
                .map(move |(range, count)| Dispatch {
                    kernel,
                    range: *range,
                    groups: groups(*count),
                })
                .collect::<Vec<_>>()
        };
        substep_passes.extend(coloured(MEMBRANE, &membrane_slots));
        substep_passes.extend(coloured(HINGE, &hinge_slots));
        substep_passes.extend(coloured(SEAM, &seam_slots));
        substep_passes.extend(coloured(CLOSURE, &closure_slots));
        substep_passes.extend(coloured(ELASTIC, &elastic_slots));
        if layered {
            frame_passes.push(all(LAYER_MOVED));
        }
        if self_collision || layered {
            frame_passes.push(one(LAYER_MARK));
            frame_passes.extend(hash);
        }
        if self_collision {
            frame_passes.push(all(FIND_NEIGHBOURS));
            substep_passes.push(all(SELF_COUNT));
            substep_passes.push(all(SELF_GATHER));
            substep_passes.push(all(SELF_APPLY));
        }
        if layered {
            frame_passes.push(turned);
            frame_passes.push(all(LAYER_FIND));
            substep_passes.push(all(LAYER_SOLVE));
            substep_passes.push(all(LAYER_AVERAGE));
            substep_passes.push(all(SELF_APPLY));
        }
        if self_collision || layered {
            frame_passes.push(one(LAYER_RESET));
        }
        let pull_at = substep_passes.len();
        substep_passes.push(all(COLLIDE));
        if model.collider.is_some() {
            substep_passes.push(all(EDGES));
            substep_passes.push(all(APPLY));
        }
        substep_passes.push(all(VELOCITY));

        Ok(Self {
            context: context.clone(),
            groups: bind_groups,
            uniforms,
            step_buffer,
            scene_buffer,
            grab_buffer,
            body_buffer,
            motion: model.motion.filter(|_| parts > 0),
            x_buffer,
            vel_buffer,
            readback,
            frame_passes,
            substep_passes,
            regather_passes,
            pull: Dispatch {
                kernel: PULL,
                range: 0,
                groups: 0,
            },
            pull_at,
            scene,
            inv_mass: model.inv_mass,
            seams: model.seams,
            closures: model.closures,
            has_seams: !seams.is_empty(),
            x: model.positions,
            v: vec![Vec3::ZERO; n],
            grab: None,
            time: 0.0,
            stats: StepStats::default(),
        })
    }

    fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("cloth frame"),
            timestamp_writes: None,
        });
        let mut run = |d: &Dispatch, substep: u32| {
            if d.groups == 0 {
                return;
            }
            let (Some((_, pipeline)), Some(group)) = (
                self.context.kernels.get(d.kernel),
                self.groups.get(d.kernel),
            ) else {
                return;
            };
            pass.set_pipeline(pipeline);
            let offsets = [
                (u64::from(substep) * SLOT) as u32,
                (u64::from(d.range) * SLOT) as u32,
            ];
            pass.set_bind_group(0, &self.uniforms, &offsets);
            pass.set_bind_group(1, group, &[]);
            pass.dispatch_workgroups(d.groups, 1, 1);
        };
        for d in &self.frame_passes {
            run(d, 0);
        }
        let held = self.grab.is_some() && self.pull.groups > 0;
        let every = if self.time < SEWING_TIME {
            1
        } else {
            REGATHER_EVERY
        };
        for s in 0..SUBSTEPS {
            if s > 0 && s % every == 0 {
                for d in &self.regather_passes {
                    run(d, s);
                }
            }
            for (k, d) in self.substep_passes.iter().enumerate() {
                if k == self.pull_at && held {
                    run(&self.pull, s);
                }
                run(d, s);
            }
        }
    }

    /// Copy positions and velocities to the readback buffer, run the
    /// frame and read them back.
    fn run_frame(&mut self, encoder: wgpu::CommandEncoder) -> Result<(), StepError> {
        let mut encoder = encoder;
        let n = self.x.len() as u64;
        encoder.copy_buffer_to_buffer(&self.x_buffer, 0, &self.readback, 0, 16 * n);
        encoder.copy_buffer_to_buffer(&self.vel_buffer, 0, &self.readback, 16 * n, 16 * n);
        self.context.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        let slice = self.readback.slice(..32 * n);
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.context
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| StepError::Device(e.to_string()))?;
        rx.recv()
            .map_err(|e| StepError::Device(e.to_string()))?
            .map_err(|e| StepError::Device(e.to_string()))?;
        {
            let view = slice
                .get_mapped_range()
                .map_err(|e| StepError::Device(e.to_string()))?;
            let all: &[[f32; 4]] =
                bytemuck::try_cast_slice(&view).map_err(|e| StepError::Device(e.to_string()))?;
            let (x, v) = all.split_at(all.len() / 2);
            let three = |p: &[f32; 4]| Vec3::new(p[0], p[1], p[2]);
            self.x.clear();
            self.x.extend(x.iter().map(three));
            self.v.clear();
            self.v.extend(v.iter().map(three));
        }
        self.readback.unmap();
        Ok(())
    }
}

impl Solver for GpuSolver {
    fn backend(&self) -> &'static str {
        "gpu"
    }

    fn step(&mut self, frame_dt: f32) -> Result<(), StepError> {
        if self.x.is_empty() {
            self.time += frame_dt;
            return Ok(());
        }
        let h = frame_dt / SUBSTEPS as f32;
        let mut bytes = vec![0u8; (SLOT * u64::from(SUBSTEPS)) as usize];
        let target = self.grab.unwrap_or_default();
        for s in 0..SUBSTEPS as usize {
            let (gravity, seam_alpha) = sewing(self.time + s as f32 * h, self.has_seams);
            let u = StepUniform {
                gravity_h: [gravity.x, gravity.y, gravity.z, h],
                params: [
                    seam_alpha / (h * h),
                    GRAB_COMPLIANCE / (h * h),
                    1.0 / (h * h),
                    s as f32,
                ],
                grab_target: [target.x, target.y, target.z, 0.0],
                sewing: [sewing_ramp(self.time + s as f32 * h), 0.0, 0.0, 0.0],
            };
            let at = s * SLOT as usize;
            bytes[at..at + std::mem::size_of::<StepUniform>()]
                .copy_from_slice(bytemuck::bytes_of(&u));
        }
        self.context
            .queue
            .write_buffer(&self.step_buffer, 0, &bytes);
        if let Some(motion) = &self.motion {
            let parts = self.scene.body[1] as usize;
            let body = body_rows(motion, self.time, h, parts);
            // After the parts' boxes.
            let at = 2 * 16 * parts as u64;
            self.context
                .queue
                .write_buffer(&self.body_buffer, at, bytemuck::cast_slice(&body));
        }
        let scope = self
            .context
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("cloth frame"),
                });
        self.encode(&mut encoder);
        let result = self.run_frame(encoder);
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(StepError::Device(e.to_string()));
        }
        result?;
        self.time += frame_dt;
        self.stats = StepStats::measure(&self.x, &self.v, &self.seams, &self.closures);
        Ok(())
    }

    fn positions(&self) -> &[Vec3] {
        &self.x
    }

    fn time(&self) -> f32 {
        self.time
    }

    fn stats(&self) -> StepStats {
        self.stats
    }

    fn continue_at(&mut self, t: f32) {
        self.time = t.max(0.0);
    }

    fn grab(&mut self, index: usize) {
        let Some((target, particles)) = grabbed(&self.x, &self.inv_mass, index) else {
            return;
        };
        let rows: Vec<[f32; 4]> = particles
            .iter()
            // Indices are exact in an f32 below 2²⁴.
            .filter(|(i, _)| *i < 1 << 24)
            .map(|(i, o)| [o.x, o.y, o.z, *i as f32])
            .collect();
        if !rows.is_empty() {
            self.context
                .queue
                .write_buffer(&self.grab_buffer, 0, bytemuck::cast_slice(&rows));
        }
        let count = rows.len() as u32;
        self.scene.counts[2] = count;
        self.context
            .queue
            .write_buffer(&self.scene_buffer, 0, bytemuck::bytes_of(&self.scene));
        self.pull.groups = groups(count);
        self.grab = Some(target);
    }

    fn move_grab(&mut self, target: Vec3) {
        if let Some(g) = &mut self.grab {
            *g = target;
        }
    }

    fn release(&mut self) {
        self.grab = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_share_no_particle() {
        // A strip of triangles: each shares vertices with its neighbours.
        let tris: Vec<[u32; 3]> = (0..20).map(|i| [i, i + 1, i + 2]).collect();
        let (order, ranges) = colour(&tris, 22);
        assert_eq!(order.len(), tris.len());
        assert_eq!(ranges.len(), 3);
        for (start, count) in ranges {
            let mut seen = std::collections::HashSet::new();
            for k in &order[start as usize..(start + count) as usize] {
                for v in tris[*k] {
                    assert!(seen.insert(v), "particle {v} twice in a colour");
                }
            }
        }
    }

    #[test]
    fn rows_are_found_by_their_offsets() {
        let rows: Vec<Vec<u32>> = vec![vec![4, 5], vec![], vec![7]];
        let c = csr(rows.iter().map(Vec::as_slice));
        let row = |i: usize| &c[c[i] as usize..c[i + 1] as usize];
        assert_eq!(row(0), &[4, 5]);
        assert!(row(1).is_empty());
        assert_eq!(row(2), &[7]);
    }
}
