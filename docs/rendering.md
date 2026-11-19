# Rendering

Status: living document, last updated 2026-11-19 (M24, fit maps).

How `r3d-render` draws, and where each algorithm comes from. Read it
before touching `crates/r3d-render/src/photo` or its shaders. Every
algorithm comes from a published paper or an open specification; the
implementing code cites it too.

## 1. Three tiers, one renderer

| Tier | Code | Use |
|---|---|---|
| Board | `board.rs`, `board.wgsl` | The 2D pattern (app, `render_board`). |
| Interactive 3D | `view3d.rs`, `view3d.wgsl` | The 3D view and `render_3d`: raster, studio-like lights, a sheen term, MSAA. |
| Photo | `photo/`, `photo_*.wgsl` | Path-traced studio images: the app's Photo button, `render_photo`, `r3d render --view photo`. |

All three render offscreen too (PNG for the CLI and MCP) with the same
code as the app. A fabric's look comes from `r3d_core::Fabric::look`,
shared by the three tiers and the glTF export.

**Fit maps** (M24, spec 24) are drawn by the interactive tier: a mesh may
carry one colour per vertex (`Mesh::colors`), used instead of the
material's base colour, with the same lighting so folds stay readable
(no sheen over them). The colours come from `fitmap::color`, the viridis
scale (van der Walt & Smith 2015, CC0; its lightness rises steadily, so
it reads in grey and for colour-blind viewers: Nuñez et al. 2018), nine
stops linear in between; 0 is loose (dark violet), 1 tight (yellow).
What a level means (strain 0–20 %, gap 50–0 mm) is
`r3d_commands::fit::FitMap`. Headless renders paint the scale as a bar
at the right edge (`fitmap::paint_legend`); the app draws its own legend
with the same stops.

**Prints** (M21, spec 25): a garment's print is a texture over the
cloth, at texture coordinates computed from the pattern
(`r3d_core::UvFrame`: the piece's position in mm projected on the
print's frame turned to its grainline, in repeats, so every piece shows
the print at its real size along its own grain). The interactive tier
samples it (`Mesh::uvs`, `Draw::print`; `print.rs` uploads each print
once with a mip-map pyramid built on the CPU in linear light, Williams
1983, read through a repeating trilinear sampler with 8× anisotropy)
and multiplies the material's base by it; the base is white for a
printed material and the sheen takes the print's average colour
(`Material::printed`). The board fills each piece with its print the
same way. Built-in prints (stripes, gingham, dots, twill) are drawn by
`r3d_core::print::builtin` in the garment's colour; images are decoded
by `r3d_io::texture` (PNG; JPEG with `zune-jpeg`) and scaled down by
halving to at most 2048 pixels a side.

## 2. The photo renderer

`PhotoRenderer` takes a `PhotoScene` (the garment's pieces with their
`Surface`, the avatar), a camera (`OrbitCamera`) and `Settings`
(samples, bounces, lighting, denoising). It adds the studio, builds the
hierarchy and uploads everything; then each `sample` call adds one sample
per pixel per dispatch, and `resolve` denoises and tone-maps into an
`Rgba8Unorm` image. The app's render thread (`r3d-ffi/src/photo.rs`)
alternates batches of samples (about 40 ms of GPU time each, so the
views' frames queued behind them stay smooth) and resolves a few times a
second.

### Geometry
- **Phong tessellation** (Boubekeur and Alexa 2008, "Phong
  Tessellation"): the simulation mesh (25 mm spacing) is cut into 9
  curved triangles per triangle (the avatar into 4), lifted onto the
  surface the vertex normals describe, so silhouettes and folds are
  round. Border edges stay straight, so pieces sewn along them still
  meet. Holes of closed meshes (the avatar's eye sockets) are capped.
- **Bounding volume hierarchy** built on the CPU with the surface area
  heuristic over 16 centroid bins per axis (Wald 2007, "On fast
  Construction of SAH-based Bounding Volume Hierarchies"), at most 4
  triangles per leaf, depth ≤ 48; nodes laid out depth first. The shader
  walks it with a 64-entry stack, nearer child first; triangles are
  intersected with Möller and Trumbore (1997).
- Hardware ray queries are not used: in wgpu 30 they are an experimental
  feature enabled through an `unsafe` call, which the project denies.

### Light transport
- Unidirectional path tracing with **next event estimation** on the
  studio's rectangular soft boxes and **multiple importance sampling**
  with the power heuristic (Veach and Guibas 1995, "Optimally Combining
  Sampling Techniques for Monte Carlo Rendering"). Lights are picked in
  proportion to their power and sampled uniformly over their area.
- Russian roulette after three bounces; indirect contributions are
  clamped (a brightest channel of 10) against fireflies, a small bias.
- The camera does not see the soft boxes (they stand out of frame in a
  real studio); reflections do.
- Random numbers: a PCG hash of the pixel and the sample index
  (Jarzynski and Olano 2020, "Hash Functions for GPU Rendering"), so a
  render is deterministic.
- Shadow rays leave from the surface the vertex normals describe
  (Hanika 2021, "Hacking the Shadow Terminator", Ray Tracing Gems II), so
  coarse triangles cast no faceted terminator.

### Materials
A two-sided layered BSDF, evaluated in the shading frame (normals turned
toward the viewer; orthonormal basis of Duff et al. 2017):
- **Sheen** on top: the "Charlie" distribution (Estevez and Kulla 2017,
  "Production Friendly Microfacet Sheen BRDF") with the visibility term
  of Neubelt and Pettineo 2013, as in Khronos `KHR_materials_sheen`. The
  layers below are dimmed by `1 − max(sheen) · E(μ, α)`, the sheen's
  directional albedo, tabulated at start-up (32 × 32) by numerical
  integration.
- **Specular**: GGX microfacets (Walter et al. 2007, "Microfacet Models
  for Refraction through Rough Surfaces") with the separable Smith
  shadowing and Schlick's Fresnel (F0 = 0.04, scaled by the fabric's
  specular strength), sampled from the distribution of visible normals
  (Heitz 2018, "Sampling the GGX Distribution of Visible Normals").
- **Diffuse reflection** and **diffuse transmission** (Lambertian), split
  by the fabric's translucency: thin cotton and silk glow against the
  light, denim does not.
- Lobes are picked by their expected contribution; the density used for
  MIS is the mixture's.

| Fabric | Sheen | Sheen roughness | Roughness | Specular | Translucency |
|---|---|---|---|---|---|
| Cotton | 0.6 | 0.6 | 0.85 | 0.25 | 0.2 |
| Denim | 0.3 | 0.7 | 0.9 | 0.2 | 0.03 |
| Jersey | 0.5 | 0.5 | 0.8 | 0.25 | 0.12 |
| Silk | 1.6 | 0.3 | 0.3 | 0.9 | 0.25 |
| Wool | 0.9 | 0.9 | 0.95 | 0.1 | 0.05 |

Sheen strength is on the 3D view's scale (1.6 = full); the photo and the
glTF export use `sheen / 1.6` times the fibres' lighter tint as the sheen
colour.

### Studio
- A seamless backdrop: a floor curving up into a round wall (a
  cyclorama), sized to hold the camera; a dim sky above it.
- Soft boxes placed from where the camera looks (they turn with it, as a
  photographer moves them around the model), aimed at the garment's
  centre. Studio: a large key high on the left, a tall fill low on the
  right, a rim behind. Soft: three large boxes, white backdrop. Dramatic:
  one small hard key from the side and a rim, dark backdrop.

### Denoising and display
- Per pixel the path tracer sums the colour, the luminance squared, and
  at the first hit the albedo, depth and normal.
- The **edge-avoiding à-trous wavelet filter** (Dammertz et al. 2010) runs
  five passes (taps 1, 2, 4, 8, 16 pixels apart, a 5 × 5 B3-spline
  kernel) on the light reaching the surface (colour divided by albedo, so
  the fabric's colour and edges stay sharp), stopping at edges of normal,
  depth and luminance; the luminance tolerance follows the variance of
  the mean, blurred over 3 × 3 pixels, as in the spatial part of SVGF
  (Schied et al. 2017). More samples, less blur.
- **Khronos PBR Neutral** tone mapping (2024), made for product images
  (colours under neutral light come out as specified), then the sRGB
  transfer function.

### Limits
- Accumulation buffers are one `vec4<f32>` per pixel each, within the
  128 MiB binding every GPU guarantees: at most 8.4 megapixels (3840 ×
  2160) and 4096 pixels a side.
- Prints colour the cloth's albedo only (no weave normal or roughness
  maps); the cloth is a single surface, printed alike on both sides.

### Prints
A printed surface (`Cloth::print`) carries texture coordinates on its
triangles (interpolated flat through the Phong tessellation) and the
index of its print. At a hit the albedo is the print's colour there:
bilinear, repeating, decoded from sRGB, read from one storage buffer
that holds the sheen table and every print's texels (one `u32` per
texel; WebGPU guarantees eight storage buffers per stage, all in use).
No mip-maps: the samples a pixel takes average the print as they
accumulate. The denoiser is guided by this albedo, so the print stays
sharp after filtering. The sheen keeps the tint of the print's average
colour.

## 3. Tests
`crates/r3d-render/tests/photo.rs` (skipped on machines without any GPU
adapter; a software Vulkan driver such as lavapipe is enough): a photo in each lighting, the same photo twice is
identical, more samples and the denoiser smooth the image at the same
mean brightness, progressive sample counting, oversized photos refused.
Unit tests cover the hierarchy (every triangle in one leaf; walking it
finds the brute-force closest hit), tessellation, hole capping, the
studio and the sheen table. Set `R3D_RENDER_OUT` to keep the images.
