# 14 — Photorealistic rendering

Provenance: maintainer's functional description (2026-11-14); public literature on path tracing and cloth appearance (papers cited in `docs/rendering.md`; Khronos `KHR_materials_sheen` and the Khronos PBR Neutral tone mapper, open specifications).

## User story
As a designer, I produce a photorealistic image of my garment on the avatar
to show a client or publish online, without leaving Rustle3D.

## Behavior
- A "Photo" action produces an image at a chosen size (the 3D view's, or
  a preset size up to 3840 × 2160), seen from the current view in the app
  or from a camera angle headless (front, three-quarter, side, back;
  framing the garment or the whole body).
- Three studio setups, or an environment (an HDRI, spec 35): **Studio** (key, fill and
  rim soft boxes, light grey seamless backdrop), **Soft** (large soft
  boxes, white backdrop), **Dramatic** (one hard key and a rim, dark
  backdrop). The soft boxes keep their place relative to the camera.
- A progressive path tracer refines the image in the 3D view and in the
  background. It can be stopped at any time, and the best image so far is
  kept; it can be saved as PNG.
- Fabric appearance comes from the material (`Fabric::look`, shared with
  the 3D view and the glTF export): base colour (the garment's colour if
  set), sheen (colour, roughness), specular roughness and strength, and
  translucency of a single layer. The avatar is a satin-finish dress form.
- Available headless: MCP `render_photo` (`quality`, `lighting`,
  `camera`, `frame`, and advanced `samples`, `bounces`, `denoise`) and
  `r3d render --view photo`, so agents can show results. The text result
  gives the samples, bounces, lighting and the time taken.

## UX
- One "Photo" button over the 3D view opens the photo scene (spec 35):
  a panel beside the 3D view with the lighting, the lens, the size,
  three quality presets (Quick 16, Good 128, Best 512 samples per
  pixel) and the advanced settings (samples, bounces, denoising) folded
  away, while the 3D view shows the photo as it will be.
- While the photo refines, a bar at the bottom of the 3D view shows the
  progress (samples, seconds) with Stop, Save image… and Close; the view
  takes no input until it is closed.

## Acceptance criteria
- The same scene renders identically (within noise) on macOS, Windows and
  Linux: the sampling is deterministic (a hash of the pixel and the sample
  index), and the same image comes out twice on one machine (test).
- A 1920×1080 "Good" render of the T-shirt template finishes in under one
  minute on the reference machine. Measured on a GPU-less Linux machine
  (software Vulkan, 16 CPU threads): 480 × 360 Good in 15 s; to measure on
  the reference machine.
- Cancelling never blocks the UI: the render thread stops after its
  current batch of samples (each aims at 40 ms of GPU time).

## Not yet
- Weave detail (normal maps); cloth thickness at the edges; hardware ray
  queries (wgpu's are an unsafe, experimental feature).
