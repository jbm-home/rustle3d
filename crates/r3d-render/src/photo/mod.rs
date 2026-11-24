//! Photorealistic rendering: a progressive path tracer in wgpu compute
//! shaders, its denoiser and its tone mapping. See `docs/rendering.md`.
//!
//! - Light transport: unidirectional path tracing with next event
//!   estimation on the studio's soft boxes, combined with BSDF sampling by
//!   multiple importance sampling (Veach and Guibas 1995, "Optimally
//!   Combining Sampling Techniques for Monte Carlo Rendering"), and Russian
//!   roulette after a few bounces.
//! - Geometry: Phong-tessellated triangles ([`tessellate`]) in a bounding
//!   volume hierarchy built on the CPU ([`bvh`]) and walked in the shader.
//!   Hardware ray queries are still an unsafe, experimental wgpu feature,
//!   so they wait.
//! - Fabric: a two-sided layered BSDF — Lambertian reflection and diffuse
//!   transmission (translucency of a thin layer), a GGX specular lobe
//!   (Walter et al. 2007, "Microfacet Models for Refraction through Rough
//!   Surfaces", sampled with Heitz 2018, "Sampling the GGX Distribution of
//!   Visible Normals"), and a sheen on top ([`sheen`]).
//! - Shading normals: shadow rays leave from the surface the vertex
//!   normals describe (Hanika 2021, "Hacking the Shadow Terminator", Ray
//!   Tracing Gems II), so coarse cloth has no jagged terminator.
//! - Prints (spec 25): the cloth's albedo is read from its print at the
//!   hit's texture coordinates (bilinear, repeating, sRGB decoded), from
//!   one storage buffer holding every print's texels.
//! - Denoising: an edge-avoiding à-trous wavelet filter (Dammertz et al.
//!   2010) guided by normals, depth and the luminance variance, as in the
//!   spatial part of SVGF (Schied et al. 2017), on the light reaching the
//!   surface (colour divided by albedo), so the fabric's colour stays sharp.
//! - Tone mapping: Khronos PBR Neutral (2024), made for product images:
//!   colours under neutral light come out as specified.

mod bvh;
mod sheen;
mod studio;
mod tessellate;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::camera::OrbitCamera;
use crate::gpu::{Gpu, RenderError};
use crate::mesh::Mesh;
use crate::offscreen;
use crate::print::Printed;
use r3d_core::Fabric;

/// How long to refine: samples per pixel and light bounces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quality {
    /// A first look in seconds.
    Quick,
    /// Clean enough to show.
    #[default]
    Good,
    /// For print and close-ups.
    Best,
}

/// The studio setup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lighting {
    /// A key, a fill and a rim light on a light grey backdrop.
    #[default]
    Studio,
    /// Large soft boxes on a white backdrop, few shadows.
    Soft,
    /// One hard key and a rim on a dark backdrop.
    Dramatic,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Quick, Quality::Good, Quality::Best];

    pub const fn name(self) -> &'static str {
        match self {
            Quality::Quick => "quick",
            Quality::Good => "good",
            Quality::Best => "best",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|q| q.name() == name)
    }

    /// Samples per pixel.
    pub const fn samples(self) -> u32 {
        match self {
            Quality::Quick => 16,
            Quality::Good => 128,
            Quality::Best => 512,
        }
    }

    /// Light bounces after the first hit.
    pub const fn bounces(self) -> u32 {
        match self {
            Quality::Quick => 4,
            Quality::Good => 6,
            Quality::Best => 10,
        }
    }
}

impl Lighting {
    pub const ALL: [Lighting; 3] = [Lighting::Studio, Lighting::Soft, Lighting::Dramatic];

    pub const fn name(self) -> &'static str {
        match self {
            Lighting::Studio => "studio",
            Lighting::Soft => "soft",
            Lighting::Dramatic => "dramatic",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.name() == name)
    }
}

/// Most samples and bounces a render may ask for.
pub const MAX_SAMPLES: u32 = 16384;
pub const MAX_BOUNCES: u32 = 32;
/// Most pixels in a photo (3840 × 2160 fits): each per-pixel buffer
/// stays within the GPU's guaranteed 128 MiB binding.
pub const MAX_PIXELS: u64 = 1 << 23;
/// Longest side of a photo.
pub const MAX_SIDE: u32 = 4096;

/// What to render and how long.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub samples: u32,
    pub bounces: u32,
    pub lighting: Lighting,
    pub denoise: bool,
}

impl Settings {
    pub fn new(quality: Quality, lighting: Lighting) -> Self {
        Self {
            samples: quality.samples(),
            bounces: quality.bounces(),
            lighting,
            denoise: true,
        }
    }
}

/// How a surface reflects light (linear RGB, values in 0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub albedo: [f32; 3],
    /// Colour of the sheen (black: none).
    pub sheen: [f32; 3],
    pub sheen_roughness: f32,
    pub roughness: f32,
    pub specular: f32,
    pub translucency: f32,
}

/// sRGB channel (0..1) to linear.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_rgb(rgb: u32) -> [f32; 3] {
    [16, 8, 0].map(|s| linear(((rgb >> s) & 0xFF) as f32 / 255.0))
}

impl Surface {
    /// A fabric's look in `color` (`#RRGGBB`), or its usual colour. The
    /// sheen takes the fibres' lighter tint, its strength scaled so silk's
    /// is full (as in the glTF export).
    pub fn fabric(fabric: Fabric, color: Option<&str>) -> Self {
        let look = fabric.look();
        let rgb = color
            .and_then(|c| c.strip_prefix('#'))
            .filter(|d| d.len() == 6)
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .unwrap_or(look.rgb);
        let albedo = linear_rgb(rgb);
        let strength = (look.sheen / 1.6).min(1.0);
        Self {
            albedo,
            sheen: albedo.map(|c| (c + (1.0 - c) * 0.6) * strength),
            sheen_roughness: look.sheen_roughness,
            roughness: look.roughness,
            specular: look.specular,
            translucency: look.translucency,
        }
    }

    /// A trim or a thread: a plain matte surface of `base` (sRGB, as the
    /// 3D view's materials).
    pub fn plain(base: crate::theme::Rgba) -> Self {
        Self {
            albedo: [base[0], base[1], base[2]].map(linear),
            sheen: [0.0; 3],
            sheen_roughness: 0.5,
            roughness: 0.6,
            specular: 0.3,
            translucency: 0.0,
        }
    }

    /// The avatar: a dress form with a satin finish.
    pub fn mannequin() -> Self {
        Self {
            albedo: linear_rgb(0xCFCBC5),
            sheen: [0.0; 3],
            sheen_roughness: 0.5,
            roughness: 0.45,
            specular: 0.5,
            translucency: 0.0,
        }
    }

    fn backdrop(albedo: Vec3) -> Self {
        Self {
            albedo: albedo.to_array(),
            sheen: [0.0; 3],
            sheen_roughness: 0.5,
            roughness: 1.0,
            specular: 0.0,
            translucency: 0.0,
        }
    }
}

/// Cuts per triangle edge for the cloth and the avatar (see
/// [`tessellate`]): the cloth's mesh is coarse, the avatar's finer.
const CLOTH_LEVEL: u32 = 3;
const BODY_LEVEL: u32 = 2;

/// One piece of cloth in a photo: its mesh (with texture coordinates
/// when it has a print), its surface and its print.
#[derive(Clone, Copy, Debug)]
pub struct Cloth<'a> {
    pub mesh: &'a Mesh,
    pub surface: Surface,
    pub print: Option<Printed<'a>>,
}

/// What a photo shows: surfaces as triangles, before the studio is added.
#[derive(Clone, Debug)]
pub struct PhotoScene {
    triangles: Vec<tessellate::Corners>,
    /// Surface index of each triangle (into `surfaces`).
    surface_of: Vec<u32>,
    surfaces: Vec<Surface>,
    /// The print of each surface that has one (by surface index), and
    /// the prints' pixels.
    print_of: Vec<(u32, usize)>,
    prints: Vec<(u64, r3d_core::Texture)>,
}

impl Default for PhotoScene {
    /// An empty studio.
    fn default() -> Self {
        Self::new(&[], &[], None)
    }
}

impl PhotoScene {
    /// The garment's pieces with their surfaces, its trims and stitches
    /// (`extras`, drawn as they are), on the avatar's `body`. The coarse
    /// simulation meshes are cut into curved triangles; meshes without
    /// normals get them computed.
    pub fn new(cloth: &[Cloth<'_>], extras: &[(&Mesh, Surface)], body: Option<&Mesh>) -> Self {
        let mut scene = Self {
            triangles: Vec::new(),
            surface_of: Vec::new(),
            // Slot 0 is the backdrop, set per lighting.
            surfaces: vec![Surface::backdrop(Vec3::ONE)],
            print_of: Vec::new(),
            prints: Vec::new(),
        };
        if let Some(body) = body {
            scene.add(&tessellate::closed(body), Surface::mannequin(), BODY_LEVEL);
        }
        for c in cloth {
            let printed = c
                .print
                .filter(|_| c.mesh.uvs.len() == c.mesh.positions.len());
            if let Some(p) = printed {
                let at = match scene.prints.iter().position(|(k, _)| *k == p.key) {
                    Some(at) => at,
                    None => {
                        scene.prints.push((p.key, p.texture.clone()));
                        scene.prints.len() - 1
                    }
                };
                scene.print_of.push((scene.surfaces.len() as u32, at));
            }
            scene.add(c.mesh, c.surface, CLOTH_LEVEL);
        }
        for (mesh, surface) in extras {
            scene.add(mesh, *surface, 0);
        }
        scene
    }

    fn add(&mut self, mesh: &Mesh, surface: Surface, level: u32) {
        let with_normals;
        let mesh = if mesh.normals.len() == mesh.positions.len() {
            mesh
        } else {
            let mut m = mesh.clone();
            m.recompute_normals();
            with_normals = m;
            &with_normals
        };
        let index = self.surfaces.len() as u32;
        self.surfaces.push(surface);
        let tris = tessellate::tessellate(mesh, level);
        self.surface_of
            .extend(std::iter::repeat_n(index, tris.len()));
        self.triangles.extend(tris);
    }

    /// Number of triangles (without the backdrop).
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Corners of the box around everything in the scene.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut points = self.triangles.iter().flat_map(|(p, _, _)| p.iter());
        let first = *points.next()?;
        Some(points.fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p))))
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuTriangle {
    /// Corner positions; `p[0].w` holds the surface index.
    p: [[f32; 4]; 3],
    n: [[f32; 4]; 3],
    /// Texture coordinates of the corners: u0 v0 u1 v1, then u2 v2.
    uv: [[f32; 4]; 2],
}

/// Every print's texels in one buffer of `u32` (RGBA8, sRGB): first the
/// number of prints, then each one's offset, width and height, then the
/// texels.
fn print_texels(prints: &[(u64, r3d_core::Texture)]) -> Vec<u32> {
    let mut out = vec![prints.len() as u32];
    let header = 1 + 3 * prints.len();
    let mut offset = header;
    for (_, t) in prints {
        out.extend([offset as u32, t.width, t.height]);
        offset += (t.width * t.height) as usize;
    }
    for (_, t) in prints {
        out.extend(
            t.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| u32::from_le_bytes(*p)),
        );
    }
    out
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuSurface {
    /// Albedo and translucency.
    base: [f32; 4],
    /// Sheen colour and sheen roughness.
    sheen: [f32; 4],
    /// Specular roughness, specular strength.
    spec: [f32; 4],
}

impl From<&Surface> for GpuSurface {
    fn from(s: &Surface) -> Self {
        let [r, g, b] = s.albedo;
        let [sr, sg, sb] = s.sheen;
        Self {
            base: [r, g, b, s.translucency.clamp(0.0, 1.0)],
            sheen: [sr, sg, sb, s.sheen_roughness.clamp(0.0, 1.0)],
            spec: [
                s.roughness.clamp(0.0, 1.0),
                s.specular.clamp(0.0, 1.0),
                0.0,
                0.0,
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct GpuLight {
    /// Corner, and area in `w`.
    corner: [f32; 4],
    edge_u: [f32; 4],
    edge_v: [f32; 4],
    /// Radiance, and the probability of picking this light in `w`.
    radiance: [f32; 4],
}

const MAX_LIGHTS: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TraceParams {
    inv_view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    size: [u32; 2],
    sample_index: u32,
    bounces: u32,
    light_count: u32,
    clamp: f32,
    _pad: [u32; 2],
    lights: [GpuLight; MAX_LIGHTS],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FilterParams {
    size: [u32; 2],
    samples: u32,
    step: u32,
    exposure: f32,
    _pad: [u32; 3],
}

/// Filter passes (steps of 1, 2, 4, 8, 16 pixels).
const FILTER_PASSES: u32 = 5;
/// Uniform slots are 256 bytes apart (dynamic offset alignment).
const SLOT: u64 = 256;
/// Brightest contribution of an indirect path, against fireflies.
const CLAMP: f32 = 10.0;
/// Workgroup side in the compute shaders.
const GROUP: u32 = 8;

/// The format of the finished image.
pub const PHOTO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A photo being refined: add samples, then resolve into [`Self::image`].
pub struct PhotoRenderer {
    width: u32,
    height: u32,
    settings: Settings,
    done: u32,
    params: TraceParams,
    params_buffer: wgpu::Buffer,
    filter_buffer: wgpu::Buffer,
    trace: wgpu::ComputePipeline,
    trace_group: wgpu::BindGroup,
    prepare: wgpu::ComputePipeline,
    atrous: wgpu::ComputePipeline,
    /// Bind groups reading the ping buffer then the pong buffer.
    filter_groups: [wgpu::BindGroup; 2],
    present: wgpu::RenderPipeline,
    present_groups: [wgpu::BindGroup; 2],
    image: wgpu::Texture,
}

impl std::fmt::Debug for PhotoRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhotoRenderer")
            .field("size", &(self.width, self.height))
            .field("settings", &self.settings)
            .field("done", &self.done)
            .finish()
    }
}

fn storage(gpu: &Gpu, label: &str, bytes: &[u8], extra: wgpu::BufferUsages) -> wgpu::Buffer {
    // Bindings must hold at least one element of the shader's array.
    let size = (bytes.len() as u64).max(256).next_multiple_of(4);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | extra,
        mapped_at_creation: false,
    });
    if !bytes.is_empty() {
        gpu.queue.write_buffer(&buffer, 0, bytes);
    }
    buffer
}

fn layout_entry(
    binding: u32,
    ty: wgpu::BufferBindingType,
    dynamic: bool,
) -> wgpu::BindGroupLayoutEntry {
    // Writable storage is for compute shaders only.
    let visibility = if ty == WRITE {
        wgpu::ShaderStages::COMPUTE
    } else {
        wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT
    };
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: dynamic,
            min_binding_size: None,
        },
        count: None,
    }
}

const READ: wgpu::BufferBindingType = wgpu::BufferBindingType::Storage { read_only: true };
const WRITE: wgpu::BufferBindingType = wgpu::BufferBindingType::Storage { read_only: false };
const UNIFORM: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;

fn group(
    gpu: &Gpu,
    layout: &wgpu::BindGroupLayout,
    resources: &[wgpu::BindingResource<'_>],
) -> wgpu::BindGroup {
    let entries: Vec<wgpu::BindGroupEntry<'_>> = resources
        .iter()
        .enumerate()
        .map(|(i, r)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: r.clone(),
        })
        .collect();
    gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("photo"),
        layout,
        entries: &entries,
    })
}

fn slot(buffer: &wgpu::Buffer, size: usize) -> wgpu::BindingResource<'_> {
    wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        buffer,
        offset: 0,
        size: wgpu::BufferSize::new(size as u64),
    })
}

impl PhotoRenderer {
    /// Start a photo of `scene` seen by `camera`, `width × height` pixels
    /// (at most [`MAX_SIDE`] a side and [`MAX_PIXELS`] in all). Adds the
    /// studio and builds the hierarchy, then waits for [`Self::sample`]
    /// calls.
    pub fn new(
        gpu: &Gpu,
        scene: &PhotoScene,
        camera: &OrbitCamera,
        settings: Settings,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        if width == 0
            || height == 0
            || width > MAX_SIDE
            || height > MAX_SIDE
            || u64::from(width) * u64::from(height) > MAX_PIXELS
        {
            return Err(RenderError::TooLarge { width, height });
        }
        let settings = Settings {
            samples: settings.samples.clamp(1, MAX_SAMPLES),
            bounces: settings.bounces.min(MAX_BOUNCES),
            ..settings
        };
        let eye = camera.eye();
        let (lo, hi) = scene
            .bounds()
            .unwrap_or((Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 1.8, 0.3)));
        let centre = (lo + hi) * 0.5;
        let studio = studio::Studio::new(settings.lighting, centre, eye);

        // The backdrop wraps the camera and the subject.
        let reach = Vec3::new(eye.x, 0.0, eye.z)
            .length()
            .max(Vec3::new(lo.x, 0.0, lo.z).length())
            .max(Vec3::new(hi.x, 0.0, hi.z).length());
        let radius = (reach + 2.5).max(6.0);
        let height_m = (eye.y.max(hi.y) + 3.0).max(5.0);
        let mut triangles = scene.triangles.clone();
        let mut surface_of = scene.surface_of.clone();
        let backdrop = studio::backdrop(radius, height_m);
        surface_of.extend(std::iter::repeat_n(0, backdrop.len()));
        triangles.extend(backdrop.into_iter().map(|(p, n)| (p, n, [[0.0; 2]; 3])));
        let mut surfaces = scene.surfaces.clone();
        if let Some(b) = surfaces.first_mut() {
            *b = Surface::backdrop(studio.backdrop);
        }

        let positions: Vec<[Vec3; 3]> = triangles.iter().map(|(p, _, _)| *p).collect();
        let tree = bvh::Bvh::build(&positions);
        let gpu_triangles: Vec<GpuTriangle> = tree
            .order
            .iter()
            .filter_map(|&t| {
                let (p, n, uv) = triangles.get(t as usize)?;
                let surface = *surface_of.get(t as usize)?;
                let mut p = p.map(|v| v.extend(0.0).to_array());
                p[0][3] = surface as f32;
                Some(GpuTriangle {
                    p,
                    n: n.map(|v| v.extend(0.0).to_array()),
                    uv: [
                        [uv[0][0], uv[0][1], uv[1][0], uv[1][1]],
                        [uv[2][0], uv[2][1], 0.0, 0.0],
                    ],
                })
            })
            .collect();
        let mut gpu_surfaces: Vec<GpuSurface> = surfaces.iter().map(GpuSurface::from).collect();
        // A printed surface reads its print (index + 1; 0: none).
        for (surface, print) in &scene.print_of {
            if let Some(s) = gpu_surfaces.get_mut(*surface as usize) {
                s.spec[2] = (*print + 1) as f32;
            }
        }

        let total_power: f32 = studio.lights.iter().map(studio::AreaLight::power).sum();
        let mut lights = [GpuLight::default(); MAX_LIGHTS];
        for (slot, l) in lights.iter_mut().zip(&studio.lights) {
            *slot = GpuLight {
                corner: l.corner.extend(l.area()).to_array(),
                edge_u: l.edge_u.extend(0.0).to_array(),
                edge_v: l.edge_v.extend(0.0).to_array(),
                radiance: l
                    .radiance
                    .extend(l.power() / total_power.max(1e-9))
                    .to_array(),
            };
        }
        let aspect = width as f32 / height as f32;
        let params = TraceParams {
            inv_view_proj: camera.view_proj(aspect).inverse().to_cols_array_2d(),
            eye: eye.extend(1.0).to_array(),
            sky_top: studio.sky_top.extend(0.0).to_array(),
            sky_horizon: studio.sky_horizon.extend(0.0).to_array(),
            size: [width, height],
            sample_index: 0,
            bounces: settings.bounces,
            light_count: studio.lights.len().min(MAX_LIGHTS) as u32,
            clamp: CLAMP,
            _pad: [0; 2],
            lights,
        };

        let device = &gpu.device;
        let none = wgpu::BufferUsages::empty();
        let params_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("photo params"),
            size: std::mem::size_of::<TraceParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let nodes = storage(gpu, "bvh", bytemuck::cast_slice(&tree.nodes), none);
        let tris = storage(gpu, "triangles", bytemuck::cast_slice(&gpu_triangles), none);
        let surfaces = storage(gpu, "surfaces", bytemuck::cast_slice(&gpu_surfaces), none);
        // The sheen's table and the prints share a buffer (8 storage
        // buffers per stage at most): the table first.
        let table = sheen::albedo_table();
        let mut lut: Vec<u32> = table.iter().map(|v| v.to_bits()).collect();
        lut.extend(print_texels(&scene.prints));
        let lut = storage(gpu, "sheen and prints", bytemuck::cast_slice(&lut), none);
        let pixels = u64::from(width) * u64::from(height);
        let make = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                // One vec4<f32> per pixel.
                size: pixels * 16,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        // Sums over the samples: colour and luminance²; albedo and depth
        // at the first hit; its normal and whether something was hit.
        let color = make("photo colour");
        let albedo = make("photo albedo");
        let normal = make("photo normal");
        let ping = make("filter ping");
        let pong = make("filter pong");
        let filter_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter params"),
            size: SLOT * u64::from(FILTER_PASSES + 2),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let trace_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("photo trace"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "const SHEEN_SIZE: u32 = {}u;\nconst MIN_SHEEN_ALPHA: f32 = {};\n{}",
                    sheen::SIZE,
                    sheen::MIN_ALPHA,
                    include_str!("../shaders/photo_trace.wgsl")
                )
                .into(),
            ),
        });
        let trace_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("photo trace"),
            entries: &[
                layout_entry(0, UNIFORM, false),
                layout_entry(1, READ, false),
                layout_entry(2, READ, false),
                layout_entry(3, READ, false),
                layout_entry(4, READ, false),
                layout_entry(5, WRITE, false),
                layout_entry(6, WRITE, false),
                layout_entry(7, WRITE, false),
            ],
        });
        let compute =
            |label, module: &wgpu::ShaderModule, entry, layout: &wgpu::BindGroupLayout| {
                let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(label),
                    bind_group_layouts: &[Some(layout)],
                    immediate_size: 0,
                });
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pl),
                    module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
            };
        let trace = compute("photo trace", &trace_shader, "trace", &trace_layout);
        let trace_group = group(
            gpu,
            &trace_layout,
            &[
                params_buffer.as_entire_binding(),
                nodes.as_entire_binding(),
                tris.as_entire_binding(),
                surfaces.as_entire_binding(),
                lut.as_entire_binding(),
                color.as_entire_binding(),
                albedo.as_entire_binding(),
                normal.as_entire_binding(),
            ],
        );

        let filter_shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/photo_filter.wgsl"));
        let filter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("photo filter"),
            entries: &[
                layout_entry(0, UNIFORM, true),
                layout_entry(1, READ, false),
                layout_entry(2, READ, false),
                layout_entry(3, READ, false),
                layout_entry(4, WRITE, false),
                layout_entry(5, WRITE, false),
            ],
        });
        let prepare = compute("photo prepare", &filter_shader, "prepare", &filter_layout);
        let atrous = compute("photo atrous", &filter_shader, "atrous", &filter_layout);
        let filter_size = std::mem::size_of::<FilterParams>();
        // (src, dst): the prepare pass writes ping; passes alternate.
        let filter_groups = [(&ping, &pong), (&pong, &ping)].map(|(src, dst)| {
            group(
                gpu,
                &filter_layout,
                &[
                    slot(&filter_buffer, filter_size),
                    color.as_entire_binding(),
                    albedo.as_entire_binding(),
                    normal.as_entire_binding(),
                    src.as_entire_binding(),
                    dst.as_entire_binding(),
                ],
            )
        });

        let present_shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/photo_present.wgsl"));
        let present_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("photo present"),
            entries: &[
                layout_entry(0, UNIFORM, true),
                layout_entry(1, READ, false),
                layout_entry(2, READ, false),
            ],
        });
        let present_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("photo present"),
            bind_group_layouts: &[Some(&present_layout)],
            immediate_size: 0,
        });
        let present = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("photo present"),
            layout: Some(&present_pl),
            vertex: wgpu::VertexState {
                module: &present_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &present_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: PHOTO_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // After an odd number of filter passes the result is in pong.
        let filtered = if FILTER_PASSES % 2 == 1 { &pong } else { &ping };
        let present_groups = [filtered, &ping].map(|src| {
            group(
                gpu,
                &present_layout,
                &[
                    slot(&filter_buffer, filter_size),
                    albedo.as_entire_binding(),
                    src.as_entire_binding(),
                ],
            )
        });
        let image = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("photo"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PHOTO_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Ok(Self {
            width,
            height,
            settings,
            done: 0,
            params,
            params_buffer,
            filter_buffer,
            trace,
            trace_group,
            prepare,
            atrous,
            filter_groups,
            present,
            present_groups,
            image,
        })
    }

    /// Samples per pixel taken so far.
    pub fn samples_done(&self) -> u32 {
        self.done
    }

    pub fn is_done(&self) -> bool {
        self.done >= self.settings.samples
    }

    /// Queue up to `count` more samples per pixel (fewer near the end).
    /// Does not wait for the GPU.
    pub fn sample(&mut self, gpu: &Gpu, count: u32) {
        let count = count.min(self.settings.samples.saturating_sub(self.done));
        for _ in 0..count {
            self.params.sample_index = self.done;
            gpu.queue
                .write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&self.params));
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("photo trace"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("photo trace"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.trace);
                pass.set_bind_group(0, &self.trace_group, &[]);
                pass.dispatch_workgroups(
                    self.width.div_ceil(GROUP),
                    self.height.div_ceil(GROUP),
                    1,
                );
            }
            gpu.queue.submit([encoder.finish()]);
            self.done += 1;
        }
    }

    /// Filter and tone-map the samples so far into [`Self::image`].
    /// Does not wait for the GPU.
    pub fn resolve(&self, gpu: &Gpu) {
        let base = FilterParams {
            size: [self.width, self.height],
            samples: self.done.max(1),
            step: 1,
            exposure: 1.0,
            _pad: [0; 3],
        };
        let mut bytes = vec![0u8; (SLOT * u64::from(FILTER_PASSES + 2)) as usize];
        for i in 0..=FILTER_PASSES as usize {
            let p = FilterParams {
                step: if i == 0 { 0 } else { 1 << (i - 1) },
                ..base
            };
            let at = i * SLOT as usize;
            bytes[at..at + std::mem::size_of::<FilterParams>()]
                .copy_from_slice(bytemuck::bytes_of(&p));
        }
        gpu.queue.write_buffer(&self.filter_buffer, 0, &bytes);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("photo resolve"),
            });
        let groups = (self.width.div_ceil(GROUP), self.height.div_ceil(GROUP));
        let filtering = self.settings.denoise && self.done > 0;
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("photo filter"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.prepare);
            // The prepare pass writes the ping buffer (its `dst` in the
            // second group).
            pass.set_bind_group(0, &self.filter_groups[1], &[0]);
            pass.dispatch_workgroups(groups.0, groups.1, 1);
            if filtering {
                pass.set_pipeline(&self.atrous);
                for i in 0..FILTER_PASSES as usize {
                    let offset = ((i + 1) as u64 * SLOT) as u32;
                    pass.set_bind_group(0, &self.filter_groups[i % 2], &[offset]);
                    pass.dispatch_workgroups(groups.0, groups.1, 1);
                }
            }
        }
        {
            let view = self
                .image
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("photo present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.present);
            let src = usize::from(!filtering);
            pass.set_bind_group(0, &self.present_groups[src], &[0]);
            pass.draw(0..3, 0..1);
        }
        gpu.queue.submit([encoder.finish()]);
    }

    /// The finished image ([`PHOTO_FORMAT`], as big as the photo).
    pub fn image(&self) -> &wgpu::Texture {
        &self.image
    }

    /// The image as PNG bytes (waits for the GPU).
    pub fn png(&self, gpu: &Gpu) -> Result<Vec<u8>, RenderError> {
        let pixels = offscreen::read_pixels(gpu, &self.image)?;
        offscreen::encode_png(self.width, self.height, &pixels)
    }
}

/// Render a whole photo and return it as PNG bytes.
pub fn render_png(
    gpu: &Gpu,
    scene: &PhotoScene,
    camera: &OrbitCamera,
    settings: Settings,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, RenderError> {
    let mut photo = PhotoRenderer::new(gpu, scene, camera, settings, width, height)?;
    while !photo.is_done() {
        photo.sample(gpu, 8);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| RenderError::Readback(e.to_string()))?;
    }
    photo.resolve(gpu);
    photo.png(gpu)
}
