# 05 — Fabric materials

Provenance: maintainer's functional brief (2026-11-14); fabric model and parameters from `docs/physics.md` (Strain Based Dynamics, quadratic bending, Kawabata-style properties); general textile knowledge for fabric weights.

## User story
As a designer, I pick a fabric from a short list and see the garment behave
differently. If needed, I can fine-tune it with understandable sliders.

## Behavior
- Built-in presets: **Cotton, Denim, Jersey, Silk, Wool** with our own
  documented parameters (density, thickness, warp/weft stretch, shear,
  stretch limit, warp/weft bending, friction, damping).
- A material is assigned per garment (spec 17: a project holds several,
  each in its own fabric). Later: a project material library (built-in
  presets copied in on first use, plus custom ones).
- Custom materials: duplicate a preset and edit it. Parameters are validated
  (positive, within physical ranges).
- Visual properties: base color, and a print over it (built-in or the
  user's image, M21, spec 25).

## UX
- The fabric picker shows swatch cards with a short description ("Soft knit,
  stretchy — T-shirts").
- The default view has three plain sliders (Weight, Stiffness, Stretch). They
  map to the physical parameters on log scales. "Advanced" shows every raw
  parameter with units.
- Changing a material while simulating updates the drape live.

## Acceptance criteria
- Each preset hung in the anisotropy scene (physics §8.5) gives distinct,
  plausible results (denim sags least, silk drapes most).
- Material edits are undoable commands, and presets themselves are read-only.
- Material parameters round-trip through the project JSON.
