# 14 — Photorealistic rendering

Provenance: maintainer's functional description (2026-11-14); public literature on path tracing and cloth appearance (microfacet sheen models; Khronos `KHR_materials_sheen`).

## User story
As a designer, I produce a photorealistic image of my garment on the avatar
to show a client or publish online, without leaving Rustle3D.

## Behavior
- A "Render" action produces an image at a chosen size with a camera
  preset or the current view, and a studio HDRI or a simple light setup.
- A progressive path tracer refines the image in the 3D view and in the
  background. It can be stopped at any time, and the best image so far is
  kept.
- Fabric appearance comes from the material: base colour or print, sheen
  (colour, roughness), roughness, and optional normal detail.
- Available headless (`render_photo` for CLI and MCP), so agents can show
  results.

## UX
- One "Render" button with three presets (Quick, Good, Best) and a
  progress indicator. Advanced settings (samples, bounces) are folded away.

## Acceptance criteria
- The same scene renders identically (within noise) on macOS, Windows and
  Linux.
- A 1920×1080 "Good" render of the T-shirt template finishes in under one
  minute on the reference machine.
- Cancelling never blocks the UI.
